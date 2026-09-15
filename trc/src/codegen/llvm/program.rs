// Program assembly, modules, and object emission

use super::enum_codegen::{compile_enum_decl};
use super::types as llvm_types;
use super::vtable::{create_interface_vtable};
use super::target_wrappers;
use super::{LLVM_LOOP_METADATA_KIND, LlvmBackend};
use crate::ast::{ClassMember, Declaration, FnDecl, Program, Type};
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::targets::{CodeModel, FileType, RelocMode, Target, TargetMachine};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::OptimizationLevel;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

impl<'ctx> LlvmBackend<'ctx> {
    /// Compile a non-generic top-level function.
    pub(super) fn compile_function(&mut self, fn_decl: &FnDecl) -> Result<FunctionValue<'ctx>, String> {
        // Don't compile main here; it's handled separately.
        if fn_decl.name == "main" {
            return self.compile_main(fn_decl);
        }

        // Build the function type (container params pass by slot pointer).
        let mut param_types: Vec<inkwell::types::BasicMetadataTypeEnum> = Vec::new();
        for p in &fn_decl.params {
            let ty = self.user_param_llvm_type(&p.typ)?;
            param_types.push(ty.into());
        }
        let return_type =
            self.sig_ret_llvm_type(fn_decl.return_type.as_ref())?;
        let fn_type = match return_type {
            Some(ret) => ret.fn_type(&param_types, false),
            None => self.context.void_type().fn_type(&param_types, false),
        };

        let fn_val = if let Some(existing) = self.module.get_function(&fn_decl.name) {
            existing
        } else {
            self.module.add_function(&fn_decl.name, fn_type, None)
        };
        self.functions.insert(fn_decl.name.clone(), fn_val);

        // Apply release-mode optimization hints (alwaysinline + fastcc for
        // small internal functions). Titrate functions are private by
        // default; only `public fn main` is treated as external.
        let is_external = fn_decl.name == "main";
        self.apply_release_attrs(fn_val, fn_decl.body.len(), is_external);

        // Create entry block.
        let entry = self.context.append_basic_block(fn_val, "entry");
        self.builder.position_at_end(entry);

        // Save and clear locals.
        let saved_locals = std::mem::take(&mut self.locals);
        let saved_ret_ty = self.current_ret_ty.clone();
        self.current_ret_ty = fn_decl.return_type.clone();

        // Allocate space for parameters and store them (container params
        // alias the caller's slot).
        for (i, p) in fn_decl.params.iter().enumerate() {
            let param_val = fn_val
                .get_nth_param(i as u32)
                .ok_or_else(|| format!("missing param {} for {}", i, fn_decl.name))?;
            self.bind_param(&p.name, &p.typ, param_val)?;
        }

        // Compile body.
        for s in &fn_decl.body {
            self.compile_stmt(s)?;
        }

        // Add a default return if the current block has no terminator.
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            // Always return void for void functions, regardless of what the body produced.
            let is_void_fn = fn_decl
                .return_type
                .as_ref()
                .map(llvm_types::is_void)
                .unwrap_or(true);
            if is_void_fn {
                self.builder
                    .build_return(None)
                    .map_err(|e| format!("build_return void failed: {:?}", e))?;
            } else {
                match &fn_decl.return_type {
                    Some(t) => {
                        let ty = self.sig_ret_llvm_type(Some(t))?.ok_or_else(|| {
                            format!("codegen: void return for '{}'", fn_decl.name)
                        })?;
                        let zero: BasicValueEnum<'ctx> = match ty {
                            BasicTypeEnum::IntType(it) => it.const_int(0, false).into(),
                            BasicTypeEnum::FloatType(ft) => ft.const_float(0.0).into(),
                            BasicTypeEnum::PointerType(pt) => pt.const_null().into(),
                            _ => {
                                self.builder
                                    .build_return(None)
                                    .map_err(|e| format!("build_return failed: {:?}", e))?;
                                self.locals = saved_locals;
                                self.current_ret_ty = saved_ret_ty;
                                return Ok(fn_val);
                            }
                        };
                        self.builder
                            .build_return(Some(&zero))
                            .map_err(|e| format!("build_return zero failed: {:?}", e))?;
                    }
                    None => {
                        self.builder
                            .build_return(None)
                            .map_err(|e| format!("build_return void failed: {:?}", e))?;
                    }
                }
            }
        }

        // Restore locals.
        self.locals = saved_locals;
        self.current_ret_ty = saved_ret_ty;

        Ok(fn_val)
    }

    /// Compile the `main` function. The Titrate `main` returns `void`, but we
    /// emit a C-style `int main()` that returns 0.
    pub(super) fn compile_main(&mut self, fn_decl: &FnDecl) -> Result<FunctionValue<'ctx>, String> {
        let i32_type = self.context.i32_type();
        let main_fn_type = i32_type.fn_type(&[], false);
        let main_fn = self.module.add_function("main", main_fn_type, None);
        self.functions.insert("main".to_string(), main_fn);
        let entry = self.context.append_basic_block(main_fn, "entry");
        self.builder.position_at_end(entry);

        // Reset locals for this function scope.
        let saved_locals = std::mem::take(&mut self.locals);
        let saved_ret_ty = self.current_ret_ty.clone();
        self.current_ret_ty = None;

        for stmt in &fn_decl.body {
            self.compile_stmt(stmt)?;
        }

        // Return 0.
        let zero = i32_type.const_int(0, false);
        self.builder
            .build_return(Some(&zero))
            .map_err(|e| format!("build_return failed: {:?}", e))?;

        self.locals = saved_locals;
        self.current_ret_ty = saved_ret_ty;

        Ok(main_fn)
    }

    /// Compile the whole program: find `main`, emit IR, verify, and write the
    /// object file.
    ///
    /// `root_dir` is used to resolve imports. Imported modules are parsed,
    /// analysed, and their declarations are merged into the program before
    /// codegen so that classes like `Regex` from `import tt::regex::Regex`
    /// are available.
    /// Resolve an import path like `["tt", "lang", "String"]` to a `.tr` file,
    /// searching `root_dir`, `root_dir/lib/`, and the `lib/` of every ancestor.
    /// Tries progressively shorter prefixes for nested-type imports.
    pub(super) fn resolve_module_file(
        &self,
        import_path: &[String],
        root_dir: &std::path::Path,
    ) -> Option<PathBuf> {
        let mut search_dirs = vec![root_dir.to_path_buf(), root_dir.join("lib")];
        let mut ancestor = root_dir.parent();
        while let Some(dir) = ancestor {
            let candidate = dir.join("lib");
            if candidate.is_dir() && !search_dirs.contains(&candidate) {
                search_dirs.push(candidate);
            }
            ancestor = dir.parent();
        }
        let min_segments = 1;
        let mut start = import_path.len();
        while start >= min_segments {
            let prefix = &import_path[..start];
            let mut relative = PathBuf::new();
            for seg in &prefix[..prefix.len().saturating_sub(1)] {
                relative.push(seg);
            }
            if let Some(last) = prefix.last() {
                relative.push(format!("{}.tr", last));
            }
            for dir in &search_dirs {
                let candidate = dir.join(&relative);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
            start -= 1;
        }
        None
    }

    /// Load every transitively imported module's declarations so classes,
    /// enums, interfaces, and functions from sibling modules (e.g.
    /// `import forcefield;` -> `forcefield.tr`) are available during codegen.
    pub(super) fn load_module_declarations(
        &self,
        program: &Program,
        root_dir: &std::path::Path,
    ) -> Result<Vec<Declaration>, String> {
        let mut all = program.declarations.clone();
        let mut loaded: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
        let mut queue: Vec<Vec<String>> = program
            .imports
            .iter()
            .filter(|i| !i.glob)
            .map(|i| i.path.clone())
            .collect();
        while let Some(import_path) = queue.pop() {
            let file = self
                .resolve_module_file(&import_path, root_dir)
                .ok_or_else(|| {
                    format!(
                        "codegen: cannot resolve module '{}'",
                        import_path.join(".")
                    )
                })?;
            if !loaded.insert(file.clone()) {
                continue;
            }
            let source = std::fs::read_to_string(&file)
                .map_err(|e| format!("codegen: cannot read module '{}': {}", file.display(), e))?;
            let tokens = crate::lexer::tokenize(&source)
                .map_err(|e| format!("codegen: lexer error in module '{}': {}", file.display(), e))?;
            let ast = crate::parser::parse(tokens)
                .map_err(|e| format!("codegen: parser error in module '{}': {}", file.display(), e))?;
            // The semantic analyzer does not resolve imported symbols, so fall
            // back to the raw AST when it rejects a module.
            let mod_ast = match crate::analyzer::analyze(&ast) {
                Ok(a) => a,
                Err(_) => ast,
            };
            all.extend(mod_ast.declarations);
            for inner in &mod_ast.imports {
                if !inner.glob && !queue.contains(&inner.path) {
                    queue.push(inner.path.clone());
                }
            }
        }
        Ok(all)
    }

    pub fn compile_program(
        &mut self,
        program: &Program,
        object_path: &Path,
        release: bool,
        root_dir: &std::path::Path,
    ) -> Result<(), String> {
        // Record the release flag so that compile_function / compile_for can
        // emit the appropriate optimization hints.
        self.release_mode = release;
        self.declare_natives();

        // Merge declarations from imported modules so classes from sibling
        // files (e.g. `import forcefield;` -> forcefield.tr) are compiled.
        let all_declarations = self.load_module_declarations(program, root_dir)?;

        // Register all function declarations first (for recursion and
        // forward references from class methods). Generic functions are
        // stashed separately for on-demand monomorphization.
        for decl in &all_declarations {
            if let Declaration::Function(f) = decl {
                if f.type_params.is_empty() {
                    self.function_decls.insert(f.name.clone(), f.clone());
                } else {
                    self.generic_fn_decls
                        .entry(f.name.clone())
                        .or_default()
                        .push(f.clone());
                }
            }
        }

        // First pass: compile all enum declarations.
        for decl in &all_declarations {
            if let Declaration::Enum(enum_decl) = decl {
                let enum_info = compile_enum_decl(self.context, &self.module, enum_decl)?;
                self.enum_infos.insert(enum_decl.name.clone(), enum_info);
            }
        }

        // Pass: compile all interface declarations.
        for decl in &all_declarations {
            if let Declaration::Interface(iface_decl) = decl {
                self.compile_interface_decl(iface_decl)?;
            }
        }

        // Record each class method's return type so `infer_expr_type` can
        // resolve `obj.method()` calls on user classes (a double-returning
        // method must not be treated as int).
        for decl in &all_declarations {
            if let Declaration::Class(class_decl) = decl {
                self.class_decls
                    .insert(class_decl.name.clone(), class_decl.clone());
                let mut methods = HashMap::new();
                for member in &class_decl.members {
                    match member {
                        ClassMember::Method(m) | ClassMember::Constructor(m) => {
                            let ret = m
                                .return_type
                                .clone()
                                .unwrap_or_else(|| Type::simple("void"));
                            methods.insert(m.name.clone(), ret);
                        }
                        _ => {}
                    }
                }
                self.class_method_returns
                    .insert(class_decl.name.clone(), methods);
            }
        }

        // Second pass: compile all class declarations (build struct types, vtables, methods).
        // Generic classes compile on demand via monomorphization instead.
        for decl in &all_declarations {
            if let Declaration::Class(class_decl) = decl {
                if class_decl.type_params.is_empty() {
                    self.compile_class_decl(class_decl)?;
                }
            }
        }

        // Third pass: build interface vtables for classes that implement interfaces.
        // Generic classes are skipped here as well; their vtables are built
        // when they are instantiated.
        for decl in &all_declarations {
            if let Declaration::Class(class_decl) = decl {
                if class_decl.type_params.is_empty() {
                let class_name = class_decl.name.clone();
                // Interface implementation for this class
                for iface_type in &class_decl.ifaces {
                    let iface_name = iface_type.name();
                    if let Some(iface_info) = self.interface_infos.get(iface_name).cloned() {
                        let mut class_methods = std::collections::HashMap::new();
                        for member in &class_decl.members {
                            match member {
                                ClassMember::Method(m) | ClassMember::Constructor(m) => {
                                    let fn_name = format!("{}_{}", class_name, m.name);
                                    if let Some(fn_val) = self.module.get_function(&fn_name) {
                                        class_methods.insert(m.name.clone(), fn_val);
                                    }
                                }
                                _ => {}
                            }
                        }
                        // Fill interface defaults the class does not
                        // override with per-class clones so `this` calls
                        // inside default bodies resolve to this class.
                        let iface_decl = all_declarations.iter().find_map(|d| match d {
                            Declaration::Interface(idecl) if idecl.name == iface_name => {
                                Some(idecl.clone())
                            }
                            _ => None,
                        });
                        if let Some(idecl) = iface_decl {
                            for sig in &idecl.methods {
                                let Some(ref body) = sig.body else {
                                    continue;
                                };
                                if class_methods.contains_key(&sig.name) {
                                    continue;
                                }
                                let clone_fn = self.ensure_default_clone(
                                    &class_name,
                                    sig,
                                    body,
                                )?;
                                class_methods.insert(sig.name.clone(), clone_fn);
                            }
                        }
                        let vt = create_interface_vtable(
                            self.context,
                            &self.module,
                            &class_name,
                            iface_name,
                            &iface_info.method_names,
                            &class_methods,
                            &iface_info.default_methods,
                        );
                        if let Some(vt_global) = vt {
                            self.interface_vtables
                                .insert((iface_name.to_string(), class_name.clone()), vt_global);
                        }
                    }
                }
                }
            }
        }

        // Compile all non-generic, non-main functions first.
        for decl in &all_declarations {
            if let Declaration::Function(f) = decl {
                if f.name != "main" && f.type_params.is_empty() {
                    self.compile_function(f)?;
                }
            }
        }

        // Find and compile main.
        let main_decl = program
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Function(f) if f.name == "main" => Some(f),
                _ => None,
            })
            .ok_or("codegen: no `main` function found")?;

        self.compile_main(main_decl)?;

        // Verify the module.
        if let Err(err) = self.module.verify() {
            return Err(format!(
                "LLVM module verification failed:\n{}",
                err.to_string()
            ));
        }

        // Emit the object file.
        self.write_object(object_path, release)?;

        Ok(())
    }
    /// Compile a typed program to LLVM IR text (without writing an object file).
    ///
    /// This runs the same codegen pipeline as [`compile_program`] but skips
    /// the target-machine / object-file step, returning the IR as a string
    /// instead. Useful for testing and debugging.
    pub fn compile_program_to_ir_text(
        &mut self,
        program: &Program,
        root_dir: &std::path::Path,
    ) -> Result<String, String> {
        self.declare_natives();

        let all_declarations = self.load_module_declarations(program, root_dir)?;

        // Register all function declarations first (for recursion and
        // forward references from class methods). Generic functions are
        // stashed separately for on-demand monomorphization.
        for decl in &all_declarations {
            if let Declaration::Function(f) = decl {
                if f.type_params.is_empty() {
                    self.function_decls.insert(f.name.clone(), f.clone());
                } else {
                    self.generic_fn_decls
                        .entry(f.name.clone())
                        .or_default()
                        .push(f.clone());
                }
            }
        }

        // First pass: compile all enum declarations.
        for decl in &all_declarations {
            if let Declaration::Enum(enum_decl) = decl {
                let enum_info = compile_enum_decl(self.context, &self.module, enum_decl)?;
                self.enum_infos.insert(enum_decl.name.clone(), enum_info);
            }
        }

        // Pass: compile all interface declarations.
        for decl in &all_declarations {
            if let Declaration::Interface(iface_decl) = decl {
                self.compile_interface_decl(iface_decl)?;
            }
        }

        // Record each class declaration and method return type before
        // compiling any class, so a derived class can resolve its parent's
        // methods/fields when building its vtable regardless of order.
        for decl in &all_declarations {
            if let Declaration::Class(class_decl) = decl {
                self.class_decls
                    .insert(class_decl.name.clone(), class_decl.clone());
                let mut methods = HashMap::new();
                for member in &class_decl.members {
                    match member {
                        ClassMember::Method(m) | ClassMember::Constructor(m) => {
                            let ret = m
                                .return_type
                                .clone()
                                .unwrap_or_else(|| Type::simple("void"));
                            methods.insert(m.name.clone(), ret);
                        }
                        _ => {}
                    }
                }
                self.class_method_returns
                    .insert(class_decl.name.clone(), methods);
            }
        }

        // Second pass: compile all class declarations. Generic classes
        // compile on demand via monomorphization instead.
        for decl in &all_declarations {
            if let Declaration::Class(class_decl) = decl {
                if class_decl.type_params.is_empty() {
                    self.compile_class_decl(class_decl)?;
                }
            }
        }

        // Compile all non-generic, non-main functions first.
        for decl in &all_declarations {
            if let Declaration::Function(f) = decl {
                if f.name != "main" && f.type_params.is_empty() {
                    self.compile_function(f)?;
                }
            }
        }

        // Find and compile main.
        let main_decl = all_declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Function(f) if f.name == "main" => Some(f),
                _ => None,
            })
            .ok_or("codegen: no `main` function found")?;

        self.compile_main(main_decl)?;

        // Verify the module.
        if let Err(err) = self.module.verify() {
            return Err(format!(
                "LLVM module verification failed:\n{}",
                err.to_string()
            ));
        }

        Ok(self.module.print_to_string().to_string())
    }

    /// Emit a call to the `llvm.memset.p0.i64` intrinsic to zero-initialize
    /// `size` bytes starting at `ptr`. This is used to zero out freshly
    /// allocated class instances and arrays instead of emitting
    /// element-by-element stores.
    ///
    /// The intrinsic is declared (lazily) as:
    ///   `void @llvm.memset.p0.i64(i8* dest, i8 val, i64 len, i1 is_volatile)`
    pub fn emit_memset_zero(
        &self,
        ptr: PointerValue<'ctx>,
        size: IntValue<'ctx>,
    ) -> Result<(), String> {
        let i8_ty = self.context.i8_type();
        let i64_ty = self.context.i64_type();
        let i1_ty = self.context.bool_type();
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let void_ty = self.context.void_type();

        let fn_ty = void_ty.fn_type(
            &[i8_ptr.into(), i8_ty.into(), i64_ty.into(), i1_ty.into()],
            false,
        );

        // Declare the intrinsic if it isn't already in the module.
        let memset_fn = match self.module.get_function("llvm.memset.p0.i64") {
            Some(f) => f,
            None => self.module.add_function("llvm.memset.p0.i64", fn_ty, None),
        };

        let zero_val = i8_ty.const_int(0, false);
        let is_volatile = i1_ty.const_int(0, false);
        self.builder
            .build_call(
                memset_fn,
                &[ptr.into(), zero_val.into(), size.into(), is_volatile.into()],
                "memset.zero",
            )
            .map_err(|e| format!("build_call memset failed: {:?}", e))?;
        Ok(())
    }

    /// Attach `!llvm.loop` metadata with vectorization hints to the terminator
    /// of `loop_block` (typically the latch/cond branch of a for/while loop).
    ///
    /// The emitted metadata looks like:
    ///   `!llvm.loop !N` where `!N = !{!N, !{!"llvm.loop.vectorize.enable", i32 1}}`
    ///
    /// This is a hint only; LLVM may still choose not to vectorize.
    pub fn add_vectorize_metadata(
        &self,
        loop_block: inkwell::basic_block::BasicBlock<'ctx>,
    ) -> Result<(), String> {
        let i32_ty = self.context.i32_type();

        // Build the metadata nodes:
        //   !{!"llvm.loop.vectorize.enable", i32 1}
        //   !{!"llvm.loop.interleave.count", i32 4}
        let enable_str = self.context.metadata_string("llvm.loop.vectorize.enable");
        let enable_val = i32_ty.const_int(1, false);
        let enable_node = self
            .context
            .metadata_node(&[enable_str.into(), enable_val.into()]);

        let width_str = self.context.metadata_string("llvm.loop.vectorize.width");
        let width_val = i32_ty.const_int(4, false);
        let width_node = self
            .context
            .metadata_node(&[width_str.into(), width_val.into()]);

        // The loop metadata node references itself (LLVM convention) plus the
        // option nodes. We create it with the option nodes; LLVM treats the
        // first operand as a self-reference when attached to a branch.
        let loop_node = self
            .context
            .metadata_node(&[enable_node.into(), width_node.into()]);

        let terminator = loop_block.get_terminator().ok_or_else(|| {
            "codegen: loop block has no terminator for vectorize metadata".to_string()
        })?;
        terminator
            .set_metadata(loop_node, LLVM_LOOP_METADATA_KIND)
            .map_err(|e| format!("set_metadata llvm.loop failed: {:?}", e))?;
        Ok(())
    }

    /// Apply release-mode optimization attributes to a freshly-created
    /// function value:
    ///   - `alwaysinline` on small functions (< 10 statements)
    ///
    /// The fast calling convention is deliberately NOT applied here: a
    /// `fastcc` + `alwaysinline` function whose return value flows through
    /// the native-call marshalling alloca makes LLVM's optimizer mis-compile
    /// the marshal into `store to poison` (undefined behavior), so the
    /// `--release` binary crashes. Keeping internal functions on the default
    /// convention and relying on `alwaysinline` avoids the bad codegen while
    /// retaining the inlining benefit.
    ///
    /// `statement_count` is the number of top-level statements in the
    /// function body. The caller is responsible for passing an accurate count.
    pub fn apply_release_attrs(
        &self,
        fn_val: FunctionValue<'ctx>,
        statement_count: usize,
        is_external: bool,
    ) {
        if !self.release_mode {
            return;
        }

        // Small internal functions get alwaysinline.
        if !is_external && statement_count < 10 {
            // alwaysinline is an enum attribute with kind id = 10 (LLVM 22).
            // We look it up by name to be robust across LLVM versions.
            let kind_id = Attribute::get_named_enum_kind_id("alwaysinline");
            let attr = self.context.create_enum_attribute(kind_id, 0);
            fn_val.add_attribute(AttributeLoc::Function, attr);
        }
    }

    /// Write the module to an object file using the native target.
    pub(super) fn write_object(&self, path: &Path, release: bool) -> Result<(), String> {
        // Initialize the X86 target directly via the individual
        // LLVMInitializeX86* functions. This avoids relying on the
        // #[no_mangle] LLVM_InitializeNativeTarget symbol (called by
        // inkwell's Target::initialize_native), which may not resolve
        // correctly when the inkwell feature version and the installed
        // LLVM version differ. The X86 target is always available because
        // the `target-x86` inkwell feature is enabled.
        target_wrappers::initialize_x86();

        let triple = TargetMachine::get_default_triple();
        let target =
            Target::from_triple(&triple).map_err(|e| format!("failed to get target: {}", e))?;

        // The target machine's own pipeline only emits code; for release builds
        // the module is optimized explicitly via `run_passes` below, so pass
        // `OptimizationLevel::None` here to avoid running the pipeline twice.
        let opt_level = OptimizationLevel::None;

        let target_machine = target
            .create_target_machine(
                &triple,
                TargetMachine::get_host_cpu_name()
                    .to_str()
                    .unwrap_or("generic"),
                TargetMachine::get_host_cpu_features()
                    .to_str()
                    .unwrap_or(""),
                opt_level,
                RelocMode::Default,
                CodeModel::Default,
            )
            .ok_or("failed to create target machine")?;

        // For release builds, run the LLVM optimization pipeline explicitly.
        // `write_to_file` only emits the object; without this, the generated
        // code is unoptimized and --release is no faster than a debug build.
        if release {
            // The module has no data layout of its own; LLVM's optimization
            // passes are target-sensitive, so adopt the target machine's data
            // layout and triple before optimizing. Without this, `default<O3>`
            // can emit code that crashes at runtime.
            let data_layout = target_machine.get_target_data().get_data_layout();
            self.module.set_data_layout(&data_layout);
            self.module.set_triple(&triple);

            let options = inkwell::passes::PassBuilderOptions::create();
            self.module
                .run_passes("default<O3>", &target_machine, options)
                .map_err(|e| format!("LLVM optimization failed: {}", e))?;
        }

        target_machine
            .write_to_file(&self.module, FileType::Object, path)
            .map_err(|e| format!("failed to write object file: {}", e))?;

        Ok(())
    }
}
