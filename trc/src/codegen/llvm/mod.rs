//! LLVM-based native code generation backend.
//!
//! This module lowers the typed Titrate AST to LLVM IR using the `inkwell`
//! crate, then emits a native object file. Phase 0 supports only the subset
//! of the language needed for `examples/hello.tr`:
//!
//! - `public fn main(): void` with a straight-line body
//! - String literals
//! - `let`/`var` declarations with `string` type
//! - Assignment to `string` variables
//! - String concatenation with `+`
//! - `io::println(...)` calls
//!
//! String values are represented at the LLVM level as a pair `(i64 len, i8* ptr)`
//! where `ptr` points to a UTF-8 byte buffer of exactly `len` bytes. The
//! runtime helpers `titrate_println`, `titrate_string_concat`, and
//! `titrate_free` (provided by the `titrate_native` crate) operate on this
//! representation.

pub mod linker;
pub mod target_wrappers;
pub mod types;

use std::collections::HashMap;
use std::path::Path;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::values::{FunctionValue, PointerValue};
use inkwell::OptimizationLevel;

use crate::ast::{Declaration, Expr, FnDecl, Literal, Operator, Program, Stmt};

/// String value tracked during codegen: the byte length and the pointer to
/// the underlying UTF-8 buffer.
#[derive(Clone, Copy)]
struct StringValue<'ctx> {
    len: inkwell::values::IntValue<'ctx>,
    ptr: inkwell::values::PointerValue<'ctx>,
}

/// The LLVM backend. Owns the inkwell `Context`, `Module`, and `Builder`.
pub struct LlvmBackend<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    /// Stack of string variables: name -> (len alloca, ptr alloca).
    locals: HashMap<String, (PointerValue<'ctx>, PointerValue<'ctx>)>,
    /// Counter for generating unique global string names.
    string_counter: usize,
}

impl<'ctx> LlvmBackend<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        LlvmBackend {
            context,
            module,
            builder,
            locals: HashMap::new(),
            string_counter: 0,
        }
    }

    /// Declare the external C-ABI functions provided by `titrate_native`.
    fn declare_natives(&self) {
        let i64_type = self.context.i64_type();
        let i8_ptr = self.context.ptr_type(inkwell::AddressSpace::default());
        let void_type = self.context.void_type();

        // void titrate_println(i64 len, i8* ptr)
        let println_fn = void_type.fn_type(
            &[i64_type.into(), i8_ptr.into()],
            false,
        );
        self.module.add_function("titrate_println", println_fn, Some(Linkage::External));

        // i8* titrate_string_concat(i64 a_len, i8* a_ptr, i64 b_len, i8* b_ptr, i64* out_len)
        let concat_fn = i8_ptr.fn_type(
            &[
                i64_type.into(),
                i8_ptr.into(),
                i64_type.into(),
                i8_ptr.into(),
                i8_ptr.into(),
            ],
            false,
        );
        self.module.add_function(
            "titrate_string_concat",
            concat_fn,
            Some(Linkage::External),
        );

        // void titrate_free(i8* ptr)
        let free_fn = void_type.fn_type(&[i8_ptr.into()], false);
        self.module.add_function("titrate_free", free_fn, Some(Linkage::External));
    }

    /// Create a global byte array holding the string's UTF-8 bytes and return
    /// a `(len, ptr)` pair pointing at it.
    fn make_string_global(&mut self, s: &str) -> StringValue<'ctx> {
        let bytes = s.as_bytes();
        let len = bytes.len();
        let i8_type = self.context.i8_type();
        let i64_type = self.context.i64_type();

        let name = format!(".str.{}", self.string_counter);
        self.string_counter += 1;

        // Include a NUL terminator so the buffer is also a valid C string,
        // which is handy for debugging and for native interop.
        let mut const_bytes: Vec<inkwell::values::IntValue> =
            bytes.iter().map(|&b| i8_type.const_int(b as u64, false)).collect();
        const_bytes.push(i8_type.const_int(0, false));

        let arr_type = i8_type.array_type(const_bytes.len() as u32);
        let global = self.module.add_global(arr_type, None, &name);
        global.set_linkage(Linkage::Private);
        global.set_constant(true);
        global.set_initializer(&i8_type.const_array(&const_bytes));

        let len_val = i64_type.const_int(len as u64, false);
        let ptr_val = global.as_pointer_value();
        StringValue {
            len: len_val,
            ptr: ptr_val,
        }
    }

    /// Look up a native function by name.
    fn get_function(&self, name: &str) -> FunctionValue<'ctx> {
        self.module
            .get_function(name)
            .unwrap_or_else(|| panic!("function '{}' not declared", name))
    }

    /// Compile a string expression to a `StringValue`.
    fn compile_string_expr(&mut self, expr: &Expr) -> Result<StringValue<'ctx>, String> {
        match expr {
            Expr::Literal(Literal::String(s), _) => {
                Ok(self.make_string_global(s))
            }
            Expr::Identifier(name, _) => {
                let (len_ptr, ptr_ptr) = self.locals.get(name).ok_or_else(|| {
                    format!("codegen: unknown string variable '{}'", name)
                })?;
                let i64_type = self.context.i64_type();
                let i8_ptr = self.context.ptr_type(inkwell::AddressSpace::default());
                let len = self.builder.build_load(i64_type, *len_ptr, &format!("{}.len", name))
                    .map_err(|e| format!("build_load len failed: {:?}", e))?
                    .into_int_value();
                let ptr = self.builder.build_load(i8_ptr, *ptr_ptr, &format!("{}.ptr", name))
                    .map_err(|e| format!("build_load ptr failed: {:?}", e))?
                    .into_pointer_value();
                Ok(StringValue { len, ptr })
            }
            Expr::Binary(left, Operator::Add, right, _) => {
                let l = self.compile_string_expr(left)?;
                let r = self.compile_string_expr(right)?;
                self.build_string_concat(l, r)
            }
            _ => Err(format!(
                "codegen: unsupported string expression: {:?}",
                expr
            )),
        }
    }

    /// Emit a call to `titrate_string_concat` and return the resulting
    /// `(len, ptr)` pair.
    fn build_string_concat(
        &self,
        a: StringValue<'ctx>,
        b: StringValue<'ctx>,
    ) -> Result<StringValue<'ctx>, String> {
        let i64_type = self.context.i64_type();
        let i8_ptr = self.context.ptr_type(inkwell::AddressSpace::default());

        // Allocate space for the output length.
        let out_len_alloca = self.builder.build_alloca(i64_type, "concat.out_len")
            .map_err(|e| format!("build_alloca failed: {:?}", e))?;

        let concat_fn = self.get_function("titrate_string_concat");
        let call_value = self.builder
            .build_call(
                concat_fn,
                &[
                    a.len.into(),
                    a.ptr.into(),
                    b.len.into(),
                    b.ptr.into(),
                    out_len_alloca.into(),
                ],
                "concat.result",
            )
            .map_err(|e| format!("build_call concat failed: {:?}", e))?;

        let result_ptr = match call_value.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
            inkwell::values::ValueKind::Instruction(_) => {
                return Err("titrate_string_concat did not return a value".to_string());
            }
        };

        let out_len = self.builder.build_load(i64_type, out_len_alloca, "concat.len")
            .map_err(|e| format!("build_load out_len failed: {:?}", e))?
            .into_int_value();

        // Cast the i8* result to the right pointer type if needed.
        let ptr = if result_ptr.get_type() == i8_ptr {
            result_ptr
        } else {
            self.builder
                .build_bit_cast(result_ptr, i8_ptr, "concat.ptr.cast")
                .map_err(|e| format!("build_bit_cast failed: {:?}", e))?
                .into_pointer_value()
        };

        Ok(StringValue { len: out_len, ptr })
    }

    /// Emit a call to `titrate_println(len, ptr)`.
    fn build_println(&self, s: StringValue<'ctx>) -> Result<(), String> {
        let println_fn = self.get_function("titrate_println");
        self.builder
            .build_call(println_fn, &[s.len.into(), s.ptr.into()], "println")
            .map_err(|e| format!("build_call println failed: {:?}", e))?;
        Ok(())
    }

    /// Compile a single statement. Only the Phase 0 subset is supported.
    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Expr(expr) => {
                self.compile_expr_stmt(expr)?;
                Ok(())
            }
            Stmt::VarDecl(decl) | Stmt::ConstDecl(decl) => {
                self.compile_var_decl(decl)?;
                Ok(())
            }
            Stmt::Return(_) => {
                // main returns void; ignore explicit returns in Phase 0.
                Ok(())
            }
            _ => Err(format!(
                "codegen: unsupported statement (Phase 0): {:?}",
                stmt
            )),
        }
    }

    /// Compile an expression statement. Handles `io::println(...)` calls and
    /// assignments.
    fn compile_expr_stmt(&mut self, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::Call(callee, args, _) => {
                if let Expr::MemberAccess(namespace, method, _) = &**callee {
                    if let Expr::Identifier(ns, _) = &**namespace {
                        if ns == "io" && method == "println" {
                            if args.len() != 1 {
                                return Err(format!(
                                    "codegen: io::println expects 1 argument, got {}",
                                    args.len()
                                ));
                            }
                            let s = self.compile_string_expr(&args[0])?;
                            return self.build_println(s);
                        }
                    }
                }
                Err(format!("codegen: unsupported call target (Phase 0): {:?}", callee))
            }
            Expr::Assign(target, value, _) => {
                if let Expr::Identifier(name, _) = &**target {
                    let new_val = self.compile_string_expr(value)?;
                    let (len_ptr, ptr_ptr) = self.locals.get(name).ok_or_else(|| {
                        format!("codegen: assignment to unknown variable '{}'", name)
                    })?;
                    self.builder.build_store(*len_ptr, new_val.len)
                        .map_err(|e| format!("build_store len failed: {:?}", e))?;
                    self.builder.build_store(*ptr_ptr, new_val.ptr)
                        .map_err(|e| format!("build_store ptr failed: {:?}", e))?;
                    return Ok(());
                }
                Err(format!("codegen: unsupported assignment target (Phase 0): {:?}", target))
            }
            _ => Err(format!("codegen: unsupported expression statement (Phase 0): {:?}", expr)),
        }
    }

    /// Compile a `let`/`var`/`const` declaration with a string initializer.
    fn compile_var_decl(
        &mut self,
        decl: &crate::ast::VarDecl,
    ) -> Result<(), String> {
        let init = decl
            .init
            .as_ref()
            .ok_or_else(|| format!("codegen: variable '{}' has no initializer", decl.name))?;

        let val = self.compile_string_expr(init)?;

        let i64_type = self.context.i64_type();
        let i8_ptr = self.context.ptr_type(inkwell::AddressSpace::default());

        let len_alloca = self.builder.build_alloca(i64_type, &format!("{}.len", decl.name))
            .map_err(|e| format!("build_alloca len failed: {:?}", e))?;
        let ptr_alloca = self.builder.build_alloca(i8_ptr, &format!("{}.ptr", decl.name))
            .map_err(|e| format!("build_alloca ptr failed: {:?}", e))?;

        self.builder.build_store(len_alloca, val.len)
            .map_err(|e| format!("build_store len failed: {:?}", e))?;
        self.builder.build_store(ptr_alloca, val.ptr)
            .map_err(|e| format!("build_store ptr failed: {:?}", e))?;

        self.locals.insert(decl.name.clone(), (len_alloca, ptr_alloca));
        Ok(())
    }

    /// Compile the `main` function. The Titrate `main` returns `void`, but we
    /// emit a C-style `int main()` that returns 0.
    fn compile_main(&mut self, fn_decl: &FnDecl) -> Result<FunctionValue<'ctx>, String> {
        let i32_type = self.context.i32_type();
        let main_fn_type = i32_type.fn_type(&[], false);
        let main_fn = self
            .module
            .add_function("main", main_fn_type, None);
        let entry = self.context.append_basic_block(main_fn, "entry");
        self.builder.position_at_end(entry);

        // Reset locals for this function scope.
        self.locals.clear();

        for stmt in &fn_decl.body {
            self.compile_stmt(stmt)?;
        }

        // Return 0.
        let zero = i32_type.const_int(0, false);
        self.builder
            .build_return(Some(&zero))
            .map_err(|e| format!("build_return failed: {:?}", e))?;

        Ok(main_fn)
    }

    /// Compile the whole program: find `main`, emit IR, verify, and write the
    /// object file.
    pub fn compile_program(
        &mut self,
        program: &Program,
        object_path: &Path,
        release: bool,
    ) -> Result<(), String> {
        self.declare_natives();

        // Find the main function.
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
            return Err(format!("LLVM module verification failed:\n{}", err.to_string()));
        }

        // Emit the object file.
        self.write_object(object_path, release)?;

        Ok(())
    }

    /// Write the module to an object file using the native target.
    fn write_object(&self, path: &Path, release: bool) -> Result<(), String> {
        let init_config = InitializationConfig::default();
        Target::initialize_native(&init_config)
            .map_err(|e| format!("failed to initialize native target: {}", e))?;

        let triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&triple)
            .map_err(|e| format!("failed to get target: {}", e))?;

        let opt_level = if release {
            OptimizationLevel::Aggressive
        } else {
            OptimizationLevel::None
        };

        let target_machine = target
            .create_target_machine(
                &triple,
                TargetMachine::get_host_cpu_name().to_str().unwrap_or("generic"),
                TargetMachine::get_host_cpu_features().to_str().unwrap_or(""),
                opt_level,
                RelocMode::Default,
                CodeModel::Default,
            )
            .ok_or("failed to create target machine")?;

        target_machine
            .write_to_file(&self.module, FileType::Object, path)
            .map_err(|e| format!("failed to write object file: {}", e))?;

        Ok(())
    }
}

/// Compile a typed Titrate program to a native object file.
///
/// `program` is the typed AST produced by `analyzer::analyze`.
/// `object_path` is where the `.o` / `.obj` file will be written.
/// If `release` is true, LLVM optimizations are enabled.
pub fn compile(
    program: &Program,
    object_path: &Path,
    release: bool,
) -> Result<(), String> {
    let context = Context::create();
    let mut backend = LlvmBackend::new(&context, "titrate_main");
    backend.compile_program(program, object_path, release)
}
