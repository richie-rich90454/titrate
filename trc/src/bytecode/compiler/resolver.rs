// Module resolution – resolves import paths to file paths and caches results

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ast;
use super::Compiler;

// ---------------------------------------------------------------------------
// ModuleResolver – resolves import paths to file paths and caches results
// ---------------------------------------------------------------------------

pub(in crate::bytecode) struct ModuleResolver {
    /// Cache: dotted module name → resolved file path.
    pub cache: HashMap<String, PathBuf>,
}

impl ModuleResolver {
    pub fn new() -> Self {
        ModuleResolver {
            cache: HashMap::new(),
        }
    }

    /// Resolve an import path like `["tt", "lang", "Integer"]` to a file path.
    /// Searches in `root_dir` and `root_dir/lib/`.
    pub fn resolve(
        &mut self,
        import_path: &[String],
        root_dir: &Path,
    ) -> Result<PathBuf, String> {
        let dotted = import_path.join(".");

        // Check cache first.
        if let Some(path) = self.cache.get(&dotted) {
            return Ok(path.clone());
        }

        // Convert import path segments to a relative file path: tt/lang/Integer.tr
        let mut relative = PathBuf::new();
        for seg in &import_path[..import_path.len().saturating_sub(1)] {
            relative.push(seg);
        }
        // The last segment is the file name.
        if let Some(last) = import_path.last() {
            relative.push(format!("{}.tr", last));
        }

        // Search directories: root_dir first, then root_dir/lib/
        let search_dirs = vec![root_dir.to_path_buf(), root_dir.join("lib")];

        for dir in &search_dirs {
            let candidate = dir.join(&relative);
            if candidate.exists() {
                self.cache.insert(dotted, candidate.clone());
                return Ok(candidate);
            }
        }

        Err(format!(
            "Cannot resolve module '{}' – searched in {}",
            dotted,
            search_dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>().join(", ")
        ))
    }

    /// Resolve a glob import like `import tt::io::*` to all .tr files in the
    /// directory corresponding to the import path.
    pub fn resolve_glob(
        &mut self,
        import_path: &[String],
        root_dir: &Path,
    ) -> Result<Vec<PathBuf>, String> {
        // The directory is the full import path converted to a path.
        let mut dir_relative = PathBuf::new();
        for seg in import_path {
            dir_relative.push(seg);
        }

        let search_dirs = vec![root_dir.to_path_buf(), root_dir.join("lib")];

        for dir in &search_dirs {
            let candidate = dir.join(&dir_relative);
            if candidate.is_dir() {
                let mut files = Vec::new();
                if let Ok(entries) = std::fs::read_dir(&candidate) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("tr") {
                            files.push(path);
                        }
                    }
                }
                if !files.is_empty() {
                    files.sort();
                    return Ok(files);
                }
            }
        }

        Err(format!(
            "Cannot resolve glob import '{}' – no .tr files found in {}",
            import_path.join("."),
            search_dirs.iter().map(|d| d.join(&dir_relative).display().to_string()).collect::<Vec<_>>().join(", ")
        ))
    }
}

// ---------------------------------------------------------------------------
// Module system methods on Compiler
// ---------------------------------------------------------------------------

impl Compiler {
    /// Process imports from a program: resolve paths and load modules.
    pub(super) fn process_imports(&mut self, program: &ast::Program, root_dir: &Path) -> Result<(), String> {
        for import in &program.imports {
            let path = &import.path;

            // Check for glob import.
            if import.glob {
                let files = self.resolver.resolve_glob(path, root_dir)?;
                for file in files {
                    // Compute the dotted name from the file path relative to root_dir.
                    let dotted = Self::path_to_dotted_name(&file, root_dir);
                    self.load_module(&file, &dotted)?;
                }
            } else {
                let file_path = self.resolver.resolve(path, root_dir)?;
                let dotted = path.join(".");
                self.load_module(&file_path, &dotted)?;
            }
        }
        Ok(())
    }

    /// Convert a file path to a dotted module name by making it relative to
    /// the root directory and replacing separators with ".".
    /// Strips a leading "lib" component so that files found under
    /// `root_dir/lib/` get the same module name as if they were under
    /// `root_dir/` directly (e.g. `lib.tt.algo.Graph` → `tt.algo.Graph`).
    pub(super) fn path_to_dotted_name(file_path: &Path, root_dir: &Path) -> String {
        let relative = file_path
            .strip_prefix(root_dir)
            .unwrap_or(file_path)
            .with_extension("");
        let components: Vec<&str> = relative
            .iter()
            .filter_map(|c| c.to_str())
            .collect();

        let start = if components.first() == Some(&"lib") { 1 } else { 0 };
        components[start..].join(".")
    }

    /// Register imported symbols from a program's imports into the symbol table.
    /// Only public declarations from imported modules are registered.
    pub(super) fn register_imported_symbols(&mut self, program: &ast::Program) -> Result<(), String> {
        for import in &program.imports {
            let path = &import.path;

            // Determine which modules and which names to import.
            if import.glob {
                // Glob import: import all public symbols from the module.
                let dotted_dir = path.join(".");

                // Collect module indices that match the glob.
                let matching_indices: Vec<usize> = self.modules.iter().enumerate()
                    .filter(|(_, m)| m.program.is_some() && (m.name.starts_with(&dotted_dir) || m.name == dotted_dir))
                    .map(|(i, _)| i)
                    .collect();

                for idx in matching_indices {
                    self.register_public_symbols_from_module_index(idx);
                }
            } else {
                // Specific import: import tt::lang::Integer
                // The last segment is the symbol name; the rest is the module path.
                if path.len() < 2 {
                    // Single-segment import: treat as a module name.
                    let dotted = path.join(".");
                    if let Some(idx) = self.module_map.get(&dotted).copied() {
                        self.register_public_symbols_from_module_index(idx);
                    }
                } else {
                    // Multi-segment import: the last segment could be a specific
                    // symbol or the module name itself.
                    let symbol_name = path.last().unwrap().clone();
                    let module_path = &path[..path.len() - 1];
                    let dotted_module = module_path.join(".");

                    // Try to find the module.
                    if let Some(&idx) = self.module_map.get(&dotted_module) {
                        // Extract the program data to avoid borrow conflicts.
                        let prog_data = self.modules[idx].program.clone();
                        if let Some(ref prog) = prog_data {
                            // Register only the specific symbol.
                            for decl in &prog.declarations {
                                let (decl_name, is_public) = match decl {
                                    ast::Declaration::Function(f) => (&f.name, f.access == ast::Access::Public),
                                    ast::Declaration::Class(c) => (&c.name, true),
                                    ast::Declaration::Enum(e) => (&e.name, true),
                                    _ => continue,
                                };
                                if decl_name == &symbol_name && is_public {
                                    let mangled = format!("{}.{}", dotted_module, decl_name);
                                    match decl {
                                        ast::Declaration::Function(_) => {
                                            if let Some(&fn_idx) = self.function_map.get(&mangled) {
                                                self.symbol_table.insert(symbol_name.clone(), super::Symbol::Function(fn_idx));
                                            }
                                        }
                                        ast::Declaration::Class(_) => {
                                            if let Some(&class_idx) = self.class_map.get(&mangled) {
                                                self.symbol_table.insert(symbol_name.clone(), super::Symbol::Class(class_idx));
                                            }
                                        }
                                        ast::Declaration::Enum(_) => {
                                            if let Some(&enum_idx) = self.enum_map.get(&mangled) {
                                                self.symbol_table.insert(symbol_name.clone(), super::Symbol::Enum(enum_idx));
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    } else {
                        // Maybe the full path is the module name (e.g., import tt::lang::Integer
                        // where Integer.tr is a file in tt/lang/).
                        let dotted = path.join(".");
                        if let Some(&idx) = self.module_map.get(&dotted) {
                            self.register_public_symbols_from_module_index(idx);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Register all public symbols from a module (by index) into the symbol table.
    pub(super) fn register_public_symbols_from_module_index(&mut self, module_idx: usize) {
        let (prog_data, module_name) = {
            let m = &self.modules[module_idx];
            (m.program.clone(), m.name.clone())
        };
        if let Some(ref prog) = prog_data {
            for decl in &prog.declarations {
                let (decl_name, is_public) = match decl {
                    ast::Declaration::Function(f) => (&f.name, f.access == ast::Access::Public),
                    ast::Declaration::Class(c) => (&c.name, true),
                    ast::Declaration::Enum(e) => (&e.name, true),
                    _ => continue,
                };
                if !is_public {
                    continue;
                }
                let mangled = format!("{}.{}", module_name, decl_name);
                match decl {
                    ast::Declaration::Function(_) => {
                        if let Some(&fn_idx) = self.function_map.get(&mangled) {
                            self.symbol_table.insert(decl_name.clone(), super::Symbol::Function(fn_idx));
                        }
                    }
                    ast::Declaration::Class(_) => {
                        if let Some(&class_idx) = self.class_map.get(&mangled) {
                            self.symbol_table.insert(decl_name.clone(), super::Symbol::Class(class_idx));
                        }
                    }
                    ast::Declaration::Enum(_) => {
                        if let Some(&enum_idx) = self.enum_map.get(&mangled) {
                            self.symbol_table.insert(decl_name.clone(), super::Symbol::Enum(enum_idx));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Register declarations from a module with mangled names.
    pub(super) fn register_module_declarations(&mut self, program: &ast::Program, module_name: &str) -> Result<(), String> {
        for decl in &program.declarations {
            match decl {
                ast::Declaration::Function(fn_decl) => {
                    if !fn_decl.type_params.is_empty() {
                        // Generic function: store for later instantiation with mangled base name.
                        // Only public generics are exported.
                        if fn_decl.access == ast::Access::Public {
                            let mangled_base = format!("{}.{}", module_name, fn_decl.name);
                            let idx = self.generic_functions.len();
                            let mut mangled_fn = fn_decl.clone();
                            mangled_fn.name = mangled_base.clone();
                            self.generic_function_map.insert(mangled_base, idx);
                            self.generic_functions.push(mangled_fn);
                        }
                        continue;
                    }
                    // Register ALL functions (public and private) with mangled names.
                    // Private functions are needed for internal calls within the module.
                    let mangled = format!("{}.{}", module_name, fn_decl.name);
                    let idx = self.functions.len() as u16;
                    self.function_map.insert(mangled.clone(), idx);
                    self.functions.push(super::FunctionDef {
                        name: mangled,
                        arity: fn_decl.params.len(),
                        chunk: super::Chunk::new(),
                        is_method: false,
                        is_constructor: false,
                        local_count: 0,
                    });
                }
                ast::Declaration::Class(class_decl) => {
                    // Only register public classes.
                    if class_decl.name.starts_with('_') {
                        // Convention: classes starting with _ are private.
                        // But we use the access field instead.
                    }
                    // We register all classes from imported modules; visibility
                    // filtering happens at the symbol table level.
                    if !class_decl.type_params.is_empty() {
                        let mangled_base = format!("{}.{}", module_name, class_decl.name);
                        let idx = self.generic_classes.len();
                        let mut mangled_class = class_decl.clone();
                        mangled_class.name = mangled_base.clone();
                        self.generic_class_map.insert(mangled_base, idx);
                        self.generic_classes.push(mangled_class);
                        continue;
                    }
                    let mangled = format!("{}.{}", module_name, class_decl.name);
                    // Avoid duplicate registration.
                    if self.class_map.contains_key(&mangled) {
                        continue;
                    }
                    let class_idx = self.classes.len() as u16;
                    self.class_map.insert(mangled.clone(), class_idx);

                    let parent_idx = class_decl.parent.as_ref().and_then(|p| {
                        self.class_map.get(p.name()).copied()
                    });

                    let mut fields = Vec::new();
                    let mut field_inits = Vec::new();
                    let mut methods = std::collections::HashMap::new();
                    let mut constructor = None;

                    for member in &class_decl.members {
                        match member {
                            ast::ClassMember::Field(field_decl) => {
                                let has_init = field_decl.init.is_some();
                                if field_decl.init.is_some() {
                                    let init_chunk = super::Chunk::new();
                                    field_inits.push((field_decl.name.clone(), init_chunk));
                                }
                                fields.push(super::FieldDef {
                                    name: field_decl.name.clone(),
                                    has_init,
                                });
                            }
                            ast::ClassMember::Method(method_decl) => {
                                let method_mangled = format!("{}.{}", mangled, method_decl.name);
                                let fn_idx = self.functions.len() as u16;
                                self.functions.push(super::FunctionDef {
                                    name: method_mangled,
                                    arity: method_decl.params.len(),
                                    chunk: super::Chunk::new(),
                                    is_method: true,
                                    is_constructor: false,
                                    local_count: 0,
                                });
                                methods.insert(method_decl.name.clone(), fn_idx);
                            }
                            ast::ClassMember::Constructor(ctor_decl) => {
                                let ctor_mangled = format!("{}.<init>", mangled);
                                let fn_idx = self.functions.len() as u16;
                                self.functions.push(super::FunctionDef {
                                    name: ctor_mangled,
                                    arity: ctor_decl.params.len(),
                                    chunk: super::Chunk::new(),
                                    is_method: true,
                                    is_constructor: true,
                                    local_count: 0,
                                });
                                methods.insert("init".to_string(), fn_idx);
                                constructor = Some(fn_idx);
                            }
                        }
                    }

                    self.classes.push(super::ClassDef {
                        name: mangled,
                        parent: parent_idx,
                        fields,
                        methods,
                        constructor,
                        field_inits,
                    });
                }
                ast::Declaration::Enum(enum_decl) => {
                    // Only register public enums.
                    let mangled = format!("{}.{}", module_name, enum_decl.name);
                    if self.enum_map.contains_key(&mangled) {
                        continue;
                    }
                    let enum_idx = self.enums.len() as u16;
                    self.enum_map.insert(mangled.clone(), enum_idx);

                    let variants: Vec<super::VariantDef> = enum_decl
                        .variants
                        .iter()
                        .enumerate()
                        .map(|(i, v)| {
                            self.variant_map
                                .insert(v.name.clone(), (mangled.clone(), i));
                            super::VariantDef {
                                name: v.name.clone(),
                                field_count: v.fields.len(),
                            }
                        })
                        .collect();

                    self.enums.push(super::EnumDef {
                        name: mangled,
                        variants,
                    });
                }
                ast::Declaration::VarDecl(var_decl) => {
                    // Register module-level variables as globals with mangled names
                    // to avoid collisions between modules with same-named variables.
                    let mangled = format!("{}.{}", module_name, var_decl.name);
                    if !self.global_map.contains_key(&mangled) {
                        let idx = self.globals.len() as u16;
                        self.globals.push(mangled.clone());
                        self.global_map.insert(mangled, idx);
                    }
                }
                ast::Declaration::ConstDecl(const_decl) => {
                    // Register module-level constants as globals with mangled names.
                    let mangled = format!("{}.{}", module_name, const_decl.name);
                    if !self.global_map.contains_key(&mangled) {
                        let idx = self.globals.len() as u16;
                        self.globals.push(mangled.clone());
                        self.global_map.insert(mangled, idx);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Compile a module's program (second pass: function bodies and methods).
    ///
    /// `module_name` is the dotted module name (e.g. "tt.algo.Graph") used to
    /// construct exact mangled names for deterministic function/class lookup.
    pub(super) fn compile_module_program(&mut self, program: &ast::Program, module_name: &str) -> Result<(), String> {
        let saved_current_module = self.current_module.clone();
        self.current_module = module_name.to_string();
        for decl in &program.declarations {
            match decl {
                ast::Declaration::Function(fn_decl) => {
                    if !fn_decl.type_params.is_empty() {
                        continue;
                    }
                    // Compile ALL functions (public and private) in the module.
                    // Use the exact mangled name for deterministic lookup.
                    let mangled = format!("{}.{}", module_name, fn_decl.name);
                    if let Some(&fn_idx) = self.function_map.get(&mangled) {
                        let saved_function = self.current_function;
                        let saved_locals = std::mem::take(&mut self.locals);
                        let saved_local_count = self.local_count;
                        let saved_scope_depth = self.scope_depth;

                        self.current_function = fn_idx as usize;
                        self.locals.clear();
                        self.local_count = 0;
                        self.scope_depth = 0;

                        self.begin_scope();
                        for param in &fn_decl.params {
                            self.declare_local(&param.name)?;
                        }
                        self.compile_block(&fn_decl.body)?;
                        self.emit_opcode(super::OpCode::PUSH_VOID, 0);
                        self.emit_opcode(super::OpCode::RET, 0);
                        self.end_scope();

                        self.functions[fn_idx as usize].local_count = self.local_count;

                        self.current_function = saved_function;
                        self.locals = saved_locals;
                        self.local_count = saved_local_count;
                        self.scope_depth = saved_scope_depth;
                    }
                }
                ast::Declaration::Class(class_decl) => {
                    if class_decl.type_params.is_empty() {
                        // Use the exact mangled class name for deterministic lookup.
                        let mangled_class = format!("{}.{}", module_name, class_decl.name);
                        if let Some(&class_idx) = self.class_map.get(&mangled_class) {
                            let class_idx_usize = class_idx as usize;

                            // Set current_class so that constructor delegation
                            // (this(args)) and super() calls can resolve the
                            // correct class.
                            let saved_class = self.current_class;
                            self.current_class = Some(class_idx);

                            // Compile field initialisers.
                            for member in &class_decl.members {
                                if let ast::ClassMember::Field(field_decl) = member {
                                    if let Some(ref init_expr) = field_decl.init {
                                        let saved_fn = self.current_function;
                                        self.current_function = 0;
                                        self.compile_expr(init_expr)?;
                                        self.emit_opcode(super::OpCode::POP, 0);
                                        self.current_function = saved_fn;
                                    }
                                }
                            }

                            // Compile methods.
                            for member in &class_decl.members {
                                match member {
                                    ast::ClassMember::Method(method_decl) => {
                                        let method_fn_idx = self
                                            .classes
                                            .get(class_idx_usize)
                                            .and_then(|c| c.methods.get(&method_decl.name))
                                            .copied();
                                        if let Some(fn_idx) = method_fn_idx {
                                            self.compile_method_body(
                                                fn_idx as usize,
                                                &method_decl.params,
                                                &method_decl.body,
                                            )?;
                                        }
                                    }
                                    ast::ClassMember::Constructor(ctor_decl) => {
                                        // Find the constructor function entry by
                                        // matching arity.  The class may have
                                        // multiple constructors (overloaded by
                                        // arity).  Each was registered with name
                                        // "<mangled_class>.<init>" and the
                                        // corresponding arity during
                                        // register_module_declarations.
                                        let ctor_pattern = format!("{}.<init>", mangled_class);
                                        let ctor_arity = ctor_decl.params.len();
                                        let ctor_fn_idx = self
                                            .functions
                                            .iter()
                                            .enumerate()
                                            .find(|(_, f)| f.name == ctor_pattern && f.arity == ctor_arity)
                                            .map(|(i, _)| i as u16)
                                            .or_else(|| {
                                                self.classes
                                                    .get(class_idx_usize)
                                                    .and_then(|c| c.constructor)
                                            });
                                        if let Some(fn_idx) = ctor_fn_idx {
                                            self.compile_method_body(
                                                fn_idx as usize,
                                                &ctor_decl.params,
                                                &ctor_decl.body,
                                            )?;
                                        }
                                    }
                                    ast::ClassMember::Field(_) => {}
                                }
                            }

                            self.current_class = saved_class;
                        }
                    }
                }
                ast::Declaration::VarDecl(var_decl) => {
                    // Compile the initializer into function 0 (main chunk) using STORE_GLOBAL.
                    let mangled = format!("{}.{}", module_name, var_decl.name);
                    if let Some(global_idx) = self.global_map.get(&mangled).copied() {
                        let saved_fn = self.current_function;
                        let saved_locals = std::mem::take(&mut self.locals);
                        let saved_local_count = self.local_count;
                        let saved_scope_depth = self.scope_depth;
                        self.current_function = 0;
                        self.locals.clear();
                        self.local_count = 0;
                        self.scope_depth = 0;
                        let line = var_decl.span.line;
                        if let Some(ref init) = var_decl.init {
                            self.compile_expr(init)?;
                        } else {
                            self.emit_opcode(super::OpCode::PUSH_NULL, line);
                        }
                        self.emit_opcode(super::OpCode::STORE_GLOBAL, line);
                        self.emit_u16(global_idx, line);
                        self.current_function = saved_fn;
                        self.locals = saved_locals;
                        self.local_count = saved_local_count;
                        self.scope_depth = saved_scope_depth;
                    }
                }
                ast::Declaration::ConstDecl(const_decl) => {
                    // Compile the initializer into function 0 (main chunk) using STORE_GLOBAL.
                    let mangled = format!("{}.{}", module_name, const_decl.name);
                    if let Some(global_idx) = self.global_map.get(&mangled).copied() {
                        let saved_fn = self.current_function;
                        let saved_locals = std::mem::take(&mut self.locals);
                        let saved_local_count = self.local_count;
                        let saved_scope_depth = self.scope_depth;
                        self.current_function = 0;
                        self.locals.clear();
                        self.local_count = 0;
                        self.scope_depth = 0;
                        let line = const_decl.span.line;
                        if let Some(ref init) = const_decl.init {
                            self.compile_expr(init)?;
                        } else {
                            self.emit_opcode(super::OpCode::PUSH_NULL, line);
                        }
                        self.emit_opcode(super::OpCode::STORE_GLOBAL, line);
                        self.emit_u16(global_idx, line);
                        self.current_function = saved_fn;
                        self.locals = saved_locals;
                        self.local_count = saved_local_count;
                        self.scope_depth = saved_scope_depth;
                    }
                }
                _ => {}
            }
        }
        self.current_module = saved_current_module;
        Ok(())
    }

    /// Topological sort of modules based on import dependencies.
    /// Returns module indices in dependency order (dependencies first).
    pub(super) fn topological_sort(&self) -> Result<Vec<usize>, String> {
        let n = self.modules.len();
        let mut visited = vec![false; n];
        let mut in_stack = vec![false; n];
        let mut order = Vec::new();

        for i in 0..n {
            self.topo_visit(i, &mut visited, &mut in_stack, &mut order)?;
        }

        Ok(order)
    }

    fn topo_visit(
        &self,
        node: usize,
        visited: &mut Vec<bool>,
        in_stack: &mut Vec<bool>,
        order: &mut Vec<usize>,
    ) -> Result<(), String> {
        if in_stack[node] {
            // Circular import: break the cycle by skipping the back-edge.
            // This is safe because the compiler registers all declarations
            // in a first pass before compiling function bodies.
            return Ok(());
        }
        if visited[node] {
            return Ok(());
        }

        in_stack[node] = true;

        // Visit dependencies (imports).
        if let Some(ref prog) = self.modules[node].program {
            for import in &prog.imports {
                let path = &import.path;
                // Find the module this import refers to.
                if path.last().map(|s| s.as_str()) == Some("*") {
                    // Glob import: find all modules in the directory.
                    let dir_path = &path[..path.len() - 1];
                    let dotted_dir = dir_path.join(".");
                    for (idx, module) in self.modules.iter().enumerate() {
                        if module.name.starts_with(&dotted_dir) || module.name == dotted_dir {
                            self.topo_visit(idx, visited, in_stack, order)?;
                        }
                    }
                } else {
                    // Try the full path as a module name.
                    let dotted = path.join(".");
                    if let Some(&dep_idx) = self.module_map.get(&dotted) {
                        self.topo_visit(dep_idx, visited, in_stack, order)?;
                    } else {
                        // Try the parent path as a module.
                        if path.len() > 1 {
                            let parent_dotted = path[..path.len() - 1].join(".");
                            if let Some(&dep_idx) = self.module_map.get(&parent_dotted) {
                                self.topo_visit(dep_idx, visited, in_stack, order)?;
                            }
                        }
                    }
                }
            }
        }

        in_stack[node] = false;
        visited[node] = true;
        order.push(node);
        Ok(())
    }
}
