//! LLVM-based native code generation backend.
//!
//! This module lowers the typed Titrate AST to LLVM IR using the `inkwell`
//! crate, then emits a native object file.
//!
//! Phase 1 supports:
//! - All primitive literals (int, float, bool, char, string)
//! - `let`/`var`/`const` declarations for all primitive types
//! - Assignment to primitive and string variables
//! - String concatenation with `+`
//! - `io::println(...)` calls with any primitive argument
//! - Arithmetic, comparison, logical, and bitwise operators
//! - Control flow: if/else, while, do-while, for, switch, ternary
//! - Function declarations, calls, and recursion
//!
//! String values are represented at the LLVM level as a struct `{ i64, i8* }`
//! where the `i8*` points to a UTF-8 byte buffer of exactly `len` bytes. The
//! runtime helpers `titrate_println`, `titrate_string_concat`, and
//! `titrate_free` (provided by the `titrate_native` crate) operate on this
//! representation. Primitive values use the natural LLVM representation
//! (i32 for int, f64 for double, i1 for bool, etc.).

pub mod enum_codegen;
pub mod linker;
pub mod native_bridge;
pub mod ownership;
pub mod target_wrappers;
pub mod tuple_codegen;
pub mod types;
pub mod vtable;

use std::collections::HashMap;
use std::path::Path;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{FunctionValue, IntValue, PointerValue};

mod strings;
mod infer;
mod numeric;
mod assign;
mod binary;
mod binary_cmp;
mod calls_helpers;
mod calls_member;
mod calls;
mod calls_static;
mod arrays;
mod mono;
mod objects;
mod classes;
mod stmts;
mod flow;
mod closures;
mod program;
mod binary_num;

/// LLVM metadata kind id for `llvm.loop`. This is the well-known id LLVM
/// reserves for loop-vectorization metadata attached to branch instructions.
const LLVM_LOOP_METADATA_KIND: u32 = 6;

use crate::ast::{ClassDecl, Expr, FnDecl, Program, Stmt, Type};
use super::llvm::enum_codegen::EnumInfo;
use super::llvm::vtable::{ClassInfo, InterfaceInfo};

/// String value tracked during codegen: the byte length and the pointer to
/// the underlying UTF-8 buffer.
#[derive(Clone, Copy)]
struct StringValue<'ctx> {
    len: IntValue<'ctx>,
    ptr: PointerValue<'ctx>,
}

/// A method's resolved implementation for transparency analysis: the
/// owning class, its body, and its (name, type) parameters.
type MethodBody = (String, crate::ast::Block, Vec<(String, Type)>);

/// A local variable's storage info: the alloca pointer and the LLVM type
/// stored at that pointer.
#[derive(Clone)]
struct LocalVar<'ctx> {
    ptr: PointerValue<'ctx>,
    ty: BasicTypeEnum<'ctx>,
    /// Original Titrate type name for native call resolution (e.g., "ArrayList", "HashMap").
    titrate_type: Option<String>,
    /// Full Titrate type including generic parameters (e.g., ArrayList<int>).
    full_type: Option<Type>,
    /// True when `ptr` aliases storage owned elsewhere (a caller's slot or
    /// another variable's slot). Container mutations through the slot are
    /// shared; whole-value rebinding detaches to a private copy.
    shared: bool,
    /// Present when this local was initialized by a closure literal in the
    /// current scope: the declared signature, used to type indirect
    /// invocations through the local. Cleared on rebinding.
    closure_sig: Option<ClosureSig>,
}

/// A closure literal's declared signature, used to type indirect
/// invocations through the local holding the closure value.
#[derive(Clone)]
struct ClosureSig {
    /// Declared parameter types (container params pass as slot pointers).
    params: Vec<Type>,
    /// Declared return type.
    ret: Type,
    /// Index of the parameter a passthrough body returns (`return <param>`),
    /// if any. Lets callers alias that argument's slot (VM reference
    /// semantics) instead of snapshotting the result.
    returns_param: Option<usize>,
}

/// True when a Titrate type still mentions a generic type parameter
/// (single uppercase letter by convention, possibly nested). Such types
/// have no concrete LLVM calling convention, so indirect calls through
/// them stay honestly unsupported until monomorphization exists.
fn fn_sig_has_type_param(ty: &Type) -> bool {
    match ty {
        Type::Named { name, params } => {
            (name.len() == 1
                && name.chars().next().is_some_and(|c| c.is_ascii_uppercase()))
                || params.iter().any(fn_sig_has_type_param)
        }
        Type::Ref(inner) | Type::MutRef(inner) => fn_sig_has_type_param(inner),
        Type::Tuple(elements) => elements.iter().any(fn_sig_has_type_param),
    }
}

/// Declared signature of one interface method, used to shape indirect calls
/// through interface dispatch exactly like the implementations expect.
#[derive(Clone)]
struct IfaceMethodSig {
    /// Declared parameter types.
    params: Vec<Type>,
    /// Declared return type.
    ret: Option<Type>,
}

/// Loop context for break/continue codegen.
struct LoopContext<'ctx> {
    continue_block: inkwell::basic_block::BasicBlock<'ctx>,
    break_block: inkwell::basic_block::BasicBlock<'ctx>,
}

/// The LLVM backend. Owns the inkwell `Context`, `Module`, and `Builder`.
pub struct LlvmBackend<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    /// Local variables in the current function scope: name -> storage.
    locals: HashMap<String, LocalVar<'ctx>>,
    /// Counter for generating unique global string names.
    string_counter: usize,
    /// Stack of loop contexts for break/continue.
    loop_stack: Vec<LoopContext<'ctx>>,
    /// Map from function name to LLVM function value (for non-generic functions).
    functions: HashMap<String, FunctionValue<'ctx>>,
    /// Map from function name to its declaration (for late compilation / recursion).
    function_decls: HashMap<String, FnDecl>,
    /// Generic function declarations by name (each a list of overloads),
    /// instantiated on demand with concrete type arguments.
    generic_fn_decls: HashMap<String, Vec<FnDecl>>,
    /// Completed monomorphizations (mangled class/function names), guarding
    /// against infinite instantiation recursion.
    mono_cache: std::collections::HashSet<String>,
    /// In-progress monomorphizations (struct registered, bodies compiling).
    /// Re-entrant uses resolve by name; genuine cycles still hit mono_depth.
    mono_active: std::collections::HashSet<String>,
    /// Current monomorphization nesting depth (second bound on recursion).
    mono_depth: u32,
    /// Ownership / region / cleanup state.
    ownership: ownership::OwnershipContext<'ctx>,
    /// Counter for generating unique closure function names.
    #[allow(dead_code)]
    closure_counter: usize,
    /// Declared signature of each interface method: (interface, method) ->
    /// signature. Used to shape indirect calls through interface dispatch
    /// (container slot sharing, fat-pointer wrapping, return type).
    iface_method_sigs: HashMap<(String, String), IfaceMethodSig>,
    /// Declared return type of the function/method/closure currently being
    /// compiled. Used to wrap object values returned where an interface is
    /// declared (fat pointer) instead of mis-casting pointer to struct.
    current_ret_ty: Option<Type>,
    /// Stack of catch blocks for throw/try-catch codegen. Each entry is
    /// the basic block that a `throw` should branch to, plus the alloca
    /// where the thrown error value should be stored.
    #[allow(dead_code)]
    catch_stack: Vec<CatchContext<'ctx>>,
    /// Class info map: class name -> compiled class info (struct layout, vtable, etc.).
    #[allow(dead_code)]
    class_infos: HashMap<String, ClassInfo<'ctx>>,
    /// Class method return types: class name -> method name -> return type.
    /// Used by `infer_expr_type` so that `obj.method()` calls on user classes
    /// infer the correct result type (e.g. a double-returning method must not
    /// be treated as int).
    class_method_returns: HashMap<String, HashMap<String, Type>>,
    /// Class declarations by name, used to compute inherited field layouts.
    class_decls: HashMap<String, ClassDecl>,
    /// Type ID assignment: class name -> unique type_id.
    #[allow(dead_code)]
    class_type_ids: HashMap<String, u32>,
    /// Next available type_id.
    #[allow(dead_code)]
    next_type_id: u32,
    /// Current 'this' pointer in method codegen (None if not in a method).
    #[allow(dead_code)]
    current_this: Option<PointerValue<'ctx>>,
    /// Current class name when compiling a method (None if not in a method).
    current_class_name: Option<String>,
    /// Interface info map: interface name -> compiled interface info.
    #[allow(dead_code)]
    interface_infos: HashMap<String, InterfaceInfo<'ctx>>,
    /// Interface vtable globals: (interface_name, class_name) -> vtable global.
    #[allow(dead_code)]
    interface_vtables: HashMap<(String, String), inkwell::values::GlobalValue<'ctx>>,
    /// Enum info map: enum name -> compiled enum info.
    #[allow(dead_code)]
    enum_infos: HashMap<String, EnumInfo<'ctx>>,
    /// Field type name cache: "ClassName_fieldName" -> Titrate type name string.
    /// Used by infer_expr_type to resolve field types (e.g., "Graph__adj" -> "HashMap").
    field_titrate_type_names: HashMap<String, String>,
    /// Whether release-mode optimizations (inline hints, fast-cc, memset,
    /// vectorization metadata) should be emitted. Set from `compile_program`.
    release_mode: bool,
}

/// Catch context for try/catch codegen.
#[allow(dead_code)]
struct CatchContext<'ctx> {
    /// Block to branch to when an exception is thrown.
    catch_block: inkwell::basic_block::BasicBlock<'ctx>,
    /// Alloca (i8*) where the thrown error pointer is stored.
    error_alloca: PointerValue<'ctx>,
}


    /// True when a statement is (or contains, at any depth) a `return`.
    /// Closure literals are separate scopes: their returns never escape,
    /// so their bodies are skipped.
    fn stmt_contains_return(stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Return(_) => true,
            Stmt::Break | Stmt::Continue => false,
            Stmt::Block(b) => b.iter().any(stmt_contains_return),
            Stmt::Expr(e) | Stmt::Throw(e, _) => expr_contains_return(e),
            Stmt::If(i) => {
                i.then_branch.iter().any(stmt_contains_return)
                    || i.else_branch
                        .as_ref()
                        .map(|b| b.iter().any(stmt_contains_return))
                        .unwrap_or(false)
            }
            Stmt::While(w) => w.body.iter().any(stmt_contains_return),
            Stmt::DoWhile(d) => d.body.iter().any(stmt_contains_return),
            Stmt::WhileLet(w) => {
                expr_contains_return(&w.expr) || w.body.iter().any(stmt_contains_return)
            }
            Stmt::For(f) => {
                expr_contains_return(&f.iterable) || f.body.iter().any(stmt_contains_return)
            }
            Stmt::CFor(c) => {
                c.init
                    .as_ref()
                    .map(|s| stmt_contains_return(s))
                    .unwrap_or(false)
                    || c.condition
                        .as_ref()
                        .map(expr_contains_return)
                        .unwrap_or(false)
                    || c.increment
                        .as_ref()
                        .map(expr_contains_return)
                        .unwrap_or(false)
                    || c.body.iter().any(stmt_contains_return)
            }
            Stmt::Switch(s) => {
                s.cases.iter().any(|c| c.body.iter().any(stmt_contains_return))
                    || s.default
                        .as_ref()
                        .map(|b| b.iter().any(stmt_contains_return))
                        .unwrap_or(false)
            }
            Stmt::With(w) => {
                expr_contains_return(&w.resource_expr)
                    || w.body.iter().any(stmt_contains_return)
            }
            Stmt::VarDecl(d) | Stmt::ConstDecl(d) => d
                .init
                .as_ref()
                .map(expr_contains_return)
                .unwrap_or(false),
            Stmt::TupleDestructure { expr, .. } => expr_contains_return(expr),
            Stmt::TryCatch { try_block, catch_block, .. } => {
                try_block.iter().any(stmt_contains_return)
                    || catch_block.iter().any(stmt_contains_return)
            }
        }
    }

    /// True when an expression contains a `return` escaping to the enclosing
    /// function: only `unsafe` blocks hold statements inline; closures are
    /// separate scopes and never count.
    fn expr_contains_return(expr: &Expr) -> bool {
        match expr {
            Expr::Closure { .. } => false,
            Expr::UnsafeBlock(b, _) => b.iter().any(stmt_contains_return),
            Expr::Literal(_, _)
            | Expr::Identifier(_, _)
            | Expr::This(_)
            | Expr::Super(_)
            | Expr::Unit(_) => false,
            Expr::Binary(l, _, r, _)
            | Expr::Assign(l, r, _)
            | Expr::Range(l, r, _)
            | Expr::RangeInclusive(l, r, _) => {
                expr_contains_return(l) || expr_contains_return(r)
            }
            Expr::Unary(_, e, _)
            | Expr::OwnedDeref(e, _)
            | Expr::RegionAlloc(_, e, _)
            | Expr::RefExpr(e, _, _)
            | Expr::ErrorPropagation(e, _)
            | Expr::Cast(e, _, _)
            | Expr::Is(e, _, _) => expr_contains_return(e),
            Expr::Call(callee, args, _) => {
                expr_contains_return(callee) || args.iter().any(expr_contains_return)
            }
            Expr::MemberAccess(o, _, _) | Expr::Index(o, _, _) => expr_contains_return(o),
            Expr::New(_, args, _) => args.iter().any(expr_contains_return),
            Expr::StaticCall { args, .. } => args.iter().any(expr_contains_return),
            Expr::Ternary { condition, then_expr, else_expr, .. } => {
                expr_contains_return(condition)
                    || expr_contains_return(then_expr)
                    || expr_contains_return(else_expr)
            }
            Expr::Tuple(elements, _) => elements.iter().any(expr_contains_return),
        }
    }

    /// True when `stmts` binds `name` at any depth (shadowing hazard for
    /// transparency analysis). Closure literals are separate scopes and are
    /// skipped, as are their parameters.
    fn body_binds_name(stmts: &[Stmt], name: &str) -> bool {
        stmts.iter().any(|stmt| match stmt {
            Stmt::VarDecl(d) | Stmt::ConstDecl(d) => {
                d.name == name
                    || d.init
                        .as_ref()
                        .map(|e| expr_binds_name(e, name))
                        .unwrap_or(false)
            }
            Stmt::TupleDestructure { names, expr, .. } => {
                names.iter().any(|n| n == name) || expr_binds_name(expr, name)
            }
            Stmt::Block(b) => body_binds_name(b, name),
            Stmt::Expr(e) | Stmt::Throw(e, _) => expr_binds_name(e, name),
            Stmt::If(i) => {
                body_binds_name(&i.then_branch, name)
                    || i.else_branch
                        .as_ref()
                        .map(|b| body_binds_name(b, name))
                        .unwrap_or(false)
            }
            Stmt::While(w) => body_binds_name(&w.body, name),
            Stmt::DoWhile(d) => body_binds_name(&d.body, name),
            Stmt::WhileLet(w) => w.var_name == name || body_binds_name(&w.body, name),
            Stmt::For(f) => f.var == name || body_binds_name(&f.body, name),
            Stmt::CFor(c) => {
                c.init
                    .as_ref()
                    .map(|s| body_binds_name(std::slice::from_ref(s.as_ref()), name))
                    .unwrap_or(false)
                    || body_binds_name(&c.body, name)
            }
            Stmt::Switch(s) => {
                s.cases.iter().any(|c| body_binds_name(&c.body, name))
                    || s.default
                        .as_ref()
                        .map(|b| body_binds_name(b, name))
                        .unwrap_or(false)
            }
            Stmt::With(w) => {
                w.var_name.as_ref().map(|n| n == name).unwrap_or(false)
                    || body_binds_name(&w.body, name)
            }
            Stmt::TryCatch { try_block, catch_var, catch_block, .. } => {
                catch_var == name
                    || body_binds_name(try_block, name)
                    || body_binds_name(catch_block, name)
            }
            Stmt::Return(_) | Stmt::Break | Stmt::Continue => false,
        })
    }

    /// True when an expression contains a nested binding of `name`
    /// (closures skipped: separate scope).
    fn expr_binds_name(expr: &Expr, name: &str) -> bool {
        match expr {
            Expr::Closure { .. } => false,
            Expr::UnsafeBlock(b, _) => body_binds_name(b, name),
            Expr::Literal(_, _)
            | Expr::Identifier(_, _)
            | Expr::This(_)
            | Expr::Super(_)
            | Expr::Unit(_) => false,
            Expr::Binary(l, _, r, _)
            | Expr::Assign(l, r, _)
            | Expr::Range(l, r, _)
            | Expr::RangeInclusive(l, r, _) => {
                expr_binds_name(l, name) || expr_binds_name(r, name)
            }
            Expr::Unary(_, e, _)
            | Expr::OwnedDeref(e, _)
            | Expr::RegionAlloc(_, e, _)
            | Expr::RefExpr(e, _, _)
            | Expr::ErrorPropagation(e, _)
            | Expr::Cast(e, _, _)
            | Expr::Is(e, _, _) => expr_binds_name(e, name),
            Expr::Call(callee, args, _) => {
                expr_binds_name(callee, name) || args.iter().any(|a| expr_binds_name(a, name))
            }
            Expr::MemberAccess(o, _, _) | Expr::Index(o, _, _) => expr_binds_name(o, name),
            Expr::New(_, args, _) => args.iter().any(|a| expr_binds_name(a, name)),
            Expr::StaticCall { args, .. } => args.iter().any(|a| expr_binds_name(a, name)),
            Expr::Ternary { condition, then_expr, else_expr, .. } => {
                expr_binds_name(condition, name)
                    || expr_binds_name(then_expr, name)
                    || expr_binds_name(else_expr, name)
            }
            Expr::Tuple(elements, _) => elements.iter().any(|e| expr_binds_name(e, name)),
        }
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
            loop_stack: Vec::new(),
            functions: HashMap::new(),
            function_decls: HashMap::new(),
            generic_fn_decls: HashMap::new(),
            mono_cache: std::collections::HashSet::new(),
            mono_active: std::collections::HashSet::new(),
            mono_depth: 0,
            ownership: ownership::OwnershipContext::new(),
            closure_counter: 0,
            iface_method_sigs: HashMap::new(),
            current_ret_ty: None,
            catch_stack: Vec::new(),
            class_infos: HashMap::new(),
            class_method_returns: HashMap::new(),
            class_decls: HashMap::new(),
            class_type_ids: HashMap::new(),
            next_type_id: 0,
            current_this: None,
            current_class_name: None,
            interface_infos: HashMap::new(),
            interface_vtables: HashMap::new(),
            enum_infos: HashMap::new(),
            field_titrate_type_names: HashMap::new(),
            release_mode: false,
        }
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
    root_dir: &std::path::Path,
) -> Result<(), String> {
    let context = Context::create();
    let mut backend = LlvmBackend::new(&context, "titrate_main");
    backend.compile_program(program, object_path, release, root_dir)
}

/// Compile a typed Titrate program to LLVM IR text (without writing an object file).
///
/// This is useful for testing and debugging: it runs the full codegen
/// pipeline (including module verification) but returns the IR as a string
/// instead of invoking the system target machine.
///
/// `program` is the typed AST produced by `analyzer::analyze`.
pub fn compile_to_ir_text(program: &Program, root_dir: &std::path::Path) -> Result<String, String> {
    let context = Context::create();
    let mut backend = LlvmBackend::new(&context, "titrate_main");
    backend.compile_program_to_ir_text(program, root_dir)
}

/// Compile a typed Titrate program and write the LLVM IR to a `.ll` file.
///
/// This runs the same codegen pipeline as [`compile`] (including module
/// verification) but writes the IR text to `ir_path` instead of an object
/// file. Useful for inspecting the generated IR. The IR is written via
/// inkwell's `Module::print_to_file`.
///
/// `program` is the typed AST produced by `analyzer::analyze`.
pub fn compile_ir(
    program: &Program,
    ir_path: &Path,
    root_dir: &std::path::Path,
) -> Result<(), String> {
    let context = Context::create();
    let mut backend = LlvmBackend::new(&context, "titrate_main");
    // Run the full codegen pipeline (declare natives, compile all decls,
    // find main, and verify the module). The returned IR string is not needed
    // here; the IR is written via inkwell's `print_to_file` below.
    backend.compile_program_to_ir_text(program, root_dir)?;
    backend
        .module
        .print_to_file(ir_path)
        .map_err(|e| format!("failed to write LLVM IR to {}: {}", ir_path.display(), e))?;
    Ok(())
}

/// Compile a typed Titrate program to a native object file AND write the
/// LLVM IR to a `.ll` file in a single codegen pass.
///
/// Equivalent to calling [`compile`] (object file) and [`compile_ir`] (IR
/// file) but only runs the codegen pipeline once: after the object file is
/// written, the already-populated module is dumped to `ir_path` via inkwell's
/// `Module::print_to_file`.
///
/// `program` is the typed AST produced by `analyzer::analyze`.
/// `object_path` is where the `.o` / `.obj` file will be written.
/// `ir_path` is where the `.ll` IR file will be written.
/// If `release` is true, LLVM optimizations are enabled.
pub fn compile_with_ir(
    program: &Program,
    object_path: &Path,
    ir_path: &Path,
    release: bool,
    root_dir: &std::path::Path,
) -> Result<(), String> {
    let context = Context::create();
    let mut backend = LlvmBackend::new(&context, "titrate_main");
    // 1. Lower to LLVM IR, verify, and write the object file.
    backend.compile_program(program, object_path, release, root_dir)?;
    // 2. The module is now fully populated; dump it to the .ll file.
    backend
        .module
        .print_to_file(ir_path)
        .map_err(|e| format!("failed to write LLVM IR to {}: {}", ir_path.display(), e))?;
    Ok(())
}


#[cfg(test)]
mod tests;
