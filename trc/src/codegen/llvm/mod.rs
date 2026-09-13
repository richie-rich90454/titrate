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
use std::path::{Path, PathBuf};

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::targets::{CodeModel, FileType, RelocMode, Target, TargetMachine};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::OptimizationLevel;

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
mod binary_num;

/// LLVM metadata kind id for `llvm.loop`. This is the well-known id LLVM
/// reserves for loop-vectorization metadata attached to branch instructions.
const LLVM_LOOP_METADATA_KIND: u32 = 6;

use crate::ast::ClassMember;
use crate::ast::{
    ClassDecl, Declaration, Expr, FnDecl, InterfaceDecl, Literal, Program, Stmt, Type,
};

use super::llvm::enum_codegen::{compile_enum_decl, EnumInfo};
use super::llvm::types as llvm_types;
use super::llvm::vtable::{
    build_class_struct_type, build_interface_fat_ptr_type, create_interface_vtable,
    create_vtable_global, emit_as_cast, emit_field_access,
    emit_interface_fat_ptr, emit_interface_is_check, emit_is_check,
    emit_new_allocation, ClassInfo, InterfaceInfo,
};

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

    /// Compile a `new ClassName(args)` expression.
    fn compile_new(
        &mut self,
        type_name: &crate::ast::Type,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let class_name = type_name.name();

        // Handle built-in container types via native bridge.
        if class_name == "ArrayList" || class_name == "HashMap" {
            let native_name = format!("{}_new", class_name);
            let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
            let mut arg_tys: Vec<crate::ast::Type> = Vec::new();
            for arg in args {
                arg_vals.push(self.compile_expr(arg)?);
                arg_tys.push(self.infer_expr_type(arg));
            }
            // Call the native function and ensure the result is a {i64, ptr} struct.
            let result = self.compile_native_call(&native_name, &arg_vals, &arg_tys)?;
            // If the native bridge returned a non-struct (e.g., i32 due to incorrect type inference),
            // bitcast it to the expected {i64, ptr} container type.
            if !result.is_struct_value() {
                let string_ty = llvm_types::string_type(self.context).into_struct_type();
                let i64_val = if result.is_int_value() {
                    self.builder
                        .build_int_z_extend(
                            result.into_int_value(),
                            self.context.i64_type(),
                            "new.pad",
                        )
                        .map_err(|e| format!("build_int_z_extend failed: {:?}", e))?
                } else {
                    self.context.i64_type().const_int(0, false)
                };
                let sv = self
                    .builder
                    .build_insert_value(
                        self.builder
                            .build_insert_value(string_ty.const_zero(), i64_val, 0, "new.len")
                            .map_err(|e| format!("insert failed: {:?}", e))?,
                        self.context.ptr_type(AddressSpace::default()).const_null(),
                        1,
                        "new.ptr",
                    )
                    .map_err(|e| format!("insert failed: {:?}", e))?;
                match sv {
                    inkwell::values::AggregateValueEnum::StructValue(s) => {
                        Ok(BasicValueEnum::StructValue(s))
                    }
                    _ => Err("expected struct".into()),
                }
            } else {
                Ok(result)
            }
        } else {
            // Monomorphize generic instantiations (e.g. Box<int>) so the
            // rest of this path works with the specialized class.
            let effective = self.resolve_mono_class(type_name)?;
            let class_name = effective.as_str();
            // Try class_infos first.
            if let Some(class_info) = self.class_infos.get(class_name).cloned() {
                let ctor = class_info.constructor;
                let arg_vals = match ctor {
                    Some(ctor_fn) => {
                        let ctor_params = self.method_decl_params(class_name, "init");
                        self.build_this_call_args(ctor_fn, ctor_params.as_deref(), args)?
                    }
                    None => {
                        let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                        for arg in args {
                            arg_vals.push(self.compile_expr(arg)?);
                        }
                        arg_vals
                    }
                };
                return emit_new_allocation(
                    self.context,
                    &self.builder,
                    &self.module,
                    &class_info,
                    ctor,
                    &arg_vals,
                );
            }
            // Fallback for imported classes: try ClassName_new as a native function.
            let native_name = format!("{}_new", class_name);
            if native_bridge::is_native_function(&native_name) {
                let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                let mut arg_tys: Vec<crate::ast::Type> = Vec::new();
                for arg in args {
                    arg_vals.push(self.compile_expr(arg)?);
                    arg_tys.push(self.infer_expr_type(arg));
                }
                return self.compile_native_call(&native_name, &arg_vals, &arg_tys);
            }
            // Unknown classes are rejected by the analyzer, so reaching here
            // means an internal error. Report it honestly instead of
            // emitting a null pointer that would fail far from the cause.
            Err(format!(
                "codegen: unknown class in new expression: {}",
                class_name
            ))
        }
    }

    /// Compile a member access expression: obj.field
    fn compile_member_access(
        &mut self,
        object: &Expr,
        field: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let obj_val = self.compile_expr(object)?;
        if obj_val.is_pointer_value() {
            let obj_ptr = obj_val.into_pointer_value();
            let obj_type = self.infer_expr_type(object);
            let effective = self.resolve_mono_class(&obj_type)?;
            let class_name = effective.as_str();
            if let Some(class_info) = self.class_infos.get(class_name).cloned() {
                return emit_field_access(self.context, &self.builder, &class_info, obj_ptr, field);
            }
            // Unknown classes are rejected by the analyzer, so reaching here
            // means an internal error. Report it honestly instead of
            // emitting a null pointer that would fail far from the cause.
            return Err(format!(
                "codegen: member access on unknown class: {}",
                class_name
            ));
        }
        // If the value is a null pointer (from unknown type), return null.
        if obj_val.is_int_value() {
            let ptr_ty = self.context.ptr_type(AddressSpace::default());
            return Ok(ptr_ty.const_null().into());
        }
        Err(format!(
            "codegen: member access on non-pointer value: {:?}",
            object
        ))
    }

    /// Compile a `this` expression.
    fn compile_this(&self, _span: &crate::ast::Span) -> Result<BasicValueEnum<'ctx>, String> {
        match self.current_this {
            Some(this_ptr) => Ok(this_ptr.into()),
            None => Err("codegen: 'this' used outside of a method".to_string()),
        }
    }

    /// Compile an `is` type check expression.
    fn compile_is(
        &mut self,
        obj: &Expr,
        ty: &crate::ast::Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let type_name = ty.name();
        // Check if it's an interface.
        if let Some(iface_info) = self.interface_infos.get(type_name).cloned() {
            let obj_val = self.compile_expr(obj)?;
            if !obj_val.is_struct_value() {
                return Err(format!(
                    "codegen: 'is' check on non-struct (interface) value: {:?}",
                    obj
                ));
            }
            // For interface `is` check, we need an interface vtable. Look up
            // the first available class vtable for this interface.
            let fat_ptr_type = iface_info.fat_ptr_type;
            let i1_ty = self.context.bool_type();
            // Find any implementing class vtable for this interface.
            for ((iface_name, _class_name), vt_global) in &self.interface_vtables {
                if iface_name == type_name {
                    let expected_vt = vt_global.as_pointer_value();
                    let result = emit_interface_is_check(
                        self.context,
                        &self.builder,
                        fat_ptr_type,
                        obj_val,
                        expected_vt,
                    )?;
                    return Ok(result);
                }
            }
            return Ok(i1_ty.const_int(0, false).into());
        }
        // Check if it's a class.
        let class_name = type_name;
        let class_info =
            self.class_infos.get(class_name).cloned().ok_or_else(|| {
                format!("codegen: type '{}' not found for 'is' check", class_name)
            })?;
        let obj_val = self.compile_expr(obj)?;
        if !obj_val.is_pointer_value() {
            return Err(format!(
                "codegen: 'is' check on non-pointer value: {:?}",
                obj
            ));
        }
        let obj_ptr = obj_val.into_pointer_value();
        match &class_info.vtable_global {
            Some(vtable_global) => emit_is_check(
                self.context,
                &self.builder,
                obj_ptr,
                vtable_global.as_pointer_value(),
            ),
            None => {
                // Class has no vtable (no methods); return false.
                let i1_ty = self.context.bool_type();
                Ok(i1_ty.const_int(0, false).into())
            }
        }
    }

    /// Compile an `as` cast expression.
    fn compile_cast(
        &mut self,
        obj: &Expr,
        ty: &crate::ast::Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let type_name = ty.name();
        // Check if casting to an interface.
        if let Some(iface_info) = self.interface_infos.get(type_name).cloned() {
            let obj_val = self.compile_expr(obj)?;
            if !obj_val.is_pointer_value() {
                return Err(format!(
                    "codegen: 'as' cast to interface on non-pointer value: {:?}",
                    obj
                ));
            }
            let obj_ptr = obj_val.into_pointer_value();
            // Look up the object's type to find the interface vtable.
            let obj_type = self.infer_expr_type(obj);
            let class_name = obj_type.name();
            let vt_key = (type_name.to_string(), class_name.to_string());
            if let Some(vt_global) = self.interface_vtables.get(&vt_key) {
                return emit_interface_fat_ptr(
                    self.context,
                    &self.builder,
                    iface_info.fat_ptr_type,
                    obj_ptr,
                    vt_global.as_pointer_value(),
                );
            }
            return Err(format!(
                "codegen: no interface vtable for '{}' as '{}'",
                class_name, type_name
            ));
        }
        // Numeric cast (int -> float, float -> int, int -> int, float -> float).
        // This handles expressions like `(i / 4) as double` where the operand
        // is a primitive numeric value, not a pointer.
        if llvm_types::is_numeric(ty) {
            let obj_val = self.compile_expr(obj)?;
            // If the operand is a pointer, fall through to the class-cast path
            // below (a numeric `as` cast on a pointer would be a type error
            // elsewhere, but we keep the existing pointer-handling behavior).
            if !obj_val.is_pointer_value() {
                let target_ty = llvm_types::llvm_type(self.context, ty)?;
                return self.cast_value_to_type(obj_val, target_ty);
            }
        }
        // Simple class cast: identity.
        let obj_val = self.compile_expr(obj)?;
        if obj_val.is_pointer_value() {
            let obj_ptr = obj_val.into_pointer_value();
            return emit_as_cast(self.context, &self.builder, obj_ptr);
        }
        // int -> int (different widths) or same-width identity cast.
        // This handles cases like `(code as char)` where both source and target
        // are integer types but the target type wasn't recognized as numeric above.
        if obj_val.is_int_value() {
            let target_ty = llvm_types::llvm_type(self.context, ty)?;
            if target_ty.is_int_type() {
                return self.cast_value_to_type(obj_val, target_ty);
            }
        }
        // float -> int or int -> float for non-numeric target types.
        if obj_val.is_float_value() {
            let target_ty = llvm_types::llvm_type(self.context, ty)?;
            if target_ty.is_int_type() {
                return self.cast_value_to_type(obj_val, target_ty);
            }
        }
        if obj_val.is_int_value() {
            let target_ty = llvm_types::llvm_type(self.context, ty)?;
            if target_ty.is_float_type() {
                return self.cast_value_to_type(obj_val, target_ty);
            }
        }
        // As a last resort, if both types are the same, just return the value.
        Ok(obj_val)
    }

    /// Build the LLVM struct type for a class declaration, recursively
    /// embedding parent fields. Used when the parent class is declared after
    /// the child and its ClassInfo is not yet available.
    fn build_class_struct_type_for(&self, class_decl: &ClassDecl) -> Result<inkwell::types::StructType<'ctx>, String> {
        let mut field_types: HashMap<String, BasicTypeEnum<'ctx>> = HashMap::new();
        let mut field_decls: Vec<crate::ast::FieldDecl> = Vec::new();
        for member in &class_decl.members {
            if let ClassMember::Field(field) = member {
                let ft = llvm_types::llvm_type(self.context, &field.typ)?;
                field_types.insert(field.name.clone(), ft);
                field_decls.push(field.clone());
            }
        }
        let parent_struct_type = if let Some(parent) = &class_decl.parent {
            let parent_name = parent.name();
            if let Some(pi) = self.class_infos.get(parent_name) {
                Some(pi.struct_type)
            } else if let Some(pd) = self.class_decls.get(parent_name).cloned() {
                Some(self.build_class_struct_type_for(&pd)?)
            } else {
                None
            }
        } else {
            None
        };
        Ok(build_class_struct_type(
            self.context,
            &class_decl.name,
            &field_decls,
            &field_types,
            parent_struct_type,
        ))
    }

    /// Compile a class declaration: build struct type, compile methods, create vtable.
    fn compile_class_decl(&mut self, class_decl: &ClassDecl) -> Result<(), String> {
        // A concrete class extending a generic instantiation (e.g.
        // `class Child extends Base<int>`) links to the specialization:
        // instantiate the parent first and rewrite the link to the
        // mangled name so struct layout, super calls, and vtable
        // chains all resolve through one compiled parent.
        let normalized;
        let class_decl = match &class_decl.parent {
            Some(p) if !p.params().is_empty() => {
                let mangled = self.instantiate_llvm_class(p.name(), p.params())?;
                let mut owned = class_decl.clone();
                owned.parent = Some(Type::generic(&mangled, vec![]));
                normalized = owned;
                &normalized
            }
            _ => class_decl,
        };
        if class_decl.parent.as_ref().is_some_and(|p| p.params().is_empty())
            && self
                .class_decls
                .get(&class_decl.name)
                .is_some_and(|d| d.parent != class_decl.parent)
        {
            self.class_decls
                .insert(class_decl.name.clone(), class_decl.clone());
        }
        let class_name = class_decl.name.clone();
        let _type_id = self.next_type_id;
        self.next_type_id += 1;

        // Collect field types.
        let mut field_types: HashMap<String, BasicTypeEnum<'ctx>> = HashMap::new();
        let mut field_decls: Vec<crate::ast::FieldDecl> = Vec::new();
        let mut method_names: Vec<String> = Vec::new();
        let mut constructor: Option<FunctionValue> = None;

        for member in &class_decl.members {
            match member {
                ClassMember::Field(field) => {
                    let ft = llvm_types::llvm_type(self.context, &field.typ)?;
                    field_types.insert(field.name.clone(), ft);
                    field_decls.push(field.clone());
                    // Cache the Titrate type name for this field so that
                    // infer_expr_type can resolve field types (e.g., this._adj -> HashMap).
                    self.field_titrate_type_names.insert(
                        format!("{}_{}", class_name, field.name),
                        field.typ.name().to_string(),
                    );
                }
                ClassMember::Method(method) => {
                    method_names.push(method.name.clone());
                }
                ClassMember::Constructor(method) => {
                    method_names.push(method.name.clone());
                    // We'll compile the constructor and set it later.
                }
            }
        }

        // Build the struct type, embedding the parent's fields (after the
        // vtable pointer) so the parent's constructor and inherited field
        // access operate on the same offsets.
        let parent_struct_type = if let Some(parent) = &class_decl.parent {
            let parent_name = parent.name();
            if let Some(pi) = self.class_infos.get(parent_name) {
                Some(pi.struct_type)
            } else if let Some(pd) = self.class_decls.get(parent_name).cloned() {
                Some(self.build_class_struct_type_for(&pd)?)
            } else {
                None
            }
        } else {
            None
        };
        let struct_type = build_class_struct_type(
            self.context,
            &class_name,
            &field_decls,
            &field_types,
            parent_struct_type,
        );

        // Collect the FULL field list (inherited fields first, then own
        // fields) so `field_index` resolves inherited fields and the GEP
        // offsets match the struct layout.
        let mut fields: Vec<(String, BasicTypeEnum<'ctx>)> = Vec::new();
        {
            let mut chain: Vec<String> = Vec::new();
            let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut cur = class_decl
                .parent
                .as_ref()
                .map(|t| t.name().to_string());
            while let Some(pname) = cur {
                if visited.contains(&pname) {
                    break;
                }
                visited.insert(pname.clone());
                chain.push(pname.clone());
                cur = self
                    .class_decls
                    .get(&pname)
                    .and_then(|cd| cd.parent.as_ref().map(|t| t.name().to_string()));
            }
            for pname in chain.iter().rev() {
                if let Some(pc) = self.class_decls.get(pname) {
                    for m in &pc.members {
                        if let ClassMember::Field(f) = m {
                            let ft = llvm_types::llvm_type(self.context, &f.typ)?;
                            fields.push((f.name.clone(), ft));
                        }
                    }
                }
            }
        }
        for f in &field_decls {
            let ft = field_types
                .get(&f.name)
                .copied()
                .unwrap_or_else(|| self.context.ptr_type(AddressSpace::default()).into());
            fields.push((f.name.clone(), ft));
        }

        // Insert a skeleton class_info before compiling methods so that
        // this.field stores during method compilation can find the class.
        {
            let skeleton = ClassInfo {
                name: class_name.clone(),
                struct_type,
                fields: fields.clone(),
                method_names: method_names.clone(),
                constructor: None,
                vtable_global: None,
                parent: class_decl.parent.as_ref().map(|t| t.name().to_string()),
                ifaces: class_decl
                    .ifaces
                    .iter()
                    .map(|t| t.name().to_string())
                    .collect(),
            };
            self.class_infos.insert(class_name.clone(), skeleton);
        }

        // Compile methods and collect their LLVM function values.
        let mut method_functions: HashMap<String, FunctionValue<'ctx>> = HashMap::new();
        for member in &class_decl.members {
            match member {
                ClassMember::Method(method) => {
                    let fn_val = self.compile_class_method(&class_name, method, &struct_type)?;
                    method_functions.insert(method.name.clone(), fn_val);
                }
                ClassMember::Constructor(method) => {
                    let fn_val = self.compile_class_method(&class_name, method, &struct_type)?;
                    method_functions.insert(method.name.clone(), fn_val);
                    constructor = Some(fn_val);
                }
                _ => {}
            }
        }

        // Create the vtable. Order is prefix-stable: root-most ancestor
        // methods first, then this class's own new methods, so a virtual
        // call indexed through any static ancestor type lands on the
        // right slot. Overrides keep the ancestor's slot: `method_functions`
        // already holds this class's implementations, and the loops below
        // only fill names the class itself does not define.
        let mut all_method_names: Vec<String> = Vec::new();
        let mut all_method_functions: HashMap<String, FunctionValue<'ctx>> = method_functions;
        let own_names: std::collections::HashSet<String> =
            method_names.iter().cloned().collect();
        let mut chain: Vec<String> = Vec::new();
        {
            let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut cur = class_decl.parent.as_ref().map(|t| t.name().to_string());
            while let Some(pname) = cur {
                if visited.contains(&pname) {
                    break;
                }
                visited.insert(pname.clone());
                chain.push(pname.clone());
                cur = self
                    .class_decls
                    .get(&pname)
                    .and_then(|cd| cd.parent.as_ref().map(|t| t.name().to_string()));
            }
        }
        for pname in chain.iter().rev() {
            // Prefer the parent's own compiled order (already
            // prefix-stable); fall back to its declaration order.
            let parent_order: Vec<String> = if let Some(pi) = self.class_infos.get(pname) {
                pi.method_names.clone()
            } else if let Some(pd) = self.class_decls.get(pname) {
                pd.members
                    .iter()
                    .filter_map(|m| match m {
                        ClassMember::Method(mm) => Some(mm.name.clone()),
                        _ => None,
                    })
                    .collect()
            } else {
                Vec::new()
            };
            for mname in parent_order {
                // The slot always comes from the ancestor; only the
                // implementation is skipped when this class overrides it
                // (its own function value already won in the map).
                if !all_method_names.contains(&mname) {
                    all_method_names.push(mname.clone());
                }
                if !own_names.contains(&mname)
                    && !all_method_functions.contains_key(&mname)
                {
                    let fn_name = format!("{}_{}", pname, mname);
                    if let Some(f) = self.module.get_function(&fn_name) {
                        all_method_functions.insert(mname.clone(), f);
                    }
                }
            }
            // Declaration-order sweep for members the compiled order may
            // predate (keeps the set complete).
            if let Some(pd) = self.class_decls.get(pname) {
                for m in &pd.members {
                    if let ClassMember::Method(mm) = m {
                        if !all_method_names.contains(&mm.name) {
                            all_method_names.push(mm.name.clone());
                        }
                        if !own_names.contains(&mm.name)
                            && !all_method_functions.contains_key(&mm.name)
                        {
                            let fn_name = format!("{}_{}", pname, mm.name);
                            if let Some(f) = self.module.get_function(&fn_name) {
                                all_method_functions.insert(mm.name.clone(), f);
                            }
                        }
                    }
                }
            }
        }
        for n in &method_names {
            if !all_method_names.contains(n) {
                all_method_names.push(n.clone());
            }
        }
        let vtable_global = create_vtable_global(
            self.context,
            &self.module,
            &class_name,
            &all_method_names,
            &all_method_functions,
        );

        // Update class_info with the final vtable_global and constructor.
        let final_class_info = ClassInfo {
            name: class_name.clone(),
            struct_type,
            fields,
            method_names: all_method_names,
            constructor,
            vtable_global,
            parent: class_decl.parent.as_ref().map(|t| t.name().to_string()),
            ifaces: class_decl
                .ifaces
                .iter()
                .map(|t| t.name().to_string())
                .collect(),
        };

        self.class_infos.insert(class_name, final_class_info);
        Ok(())
    }

    /// Compile a class method (or constructor).
    fn compile_class_method(
        &mut self,
        class_name: &str,
        method: &crate::ast::MethodDecl,
        _struct_type: &inkwell::types::StructType<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let mut param_types: Vec<inkwell::types::BasicMetadataTypeEnum> = Vec::new();
        // First parameter is always `this` (i8*); container params pass by slot pointer.
        param_types.push(i8_ptr.into());
        for p in &method.params {
            let ty = self.user_param_llvm_type(&p.typ)?;
            param_types.push(ty.into());
        }
        let return_type = self.sig_ret_llvm_type(method.return_type.as_ref())?;
        let fn_type = match return_type {
            Some(ret) => ret.fn_type(&param_types, false),
            None => self.context.void_type().fn_type(&param_types, false),
        };

        let fn_name = format!("{}_{}", class_name, method.name);
        let fn_val = self.module.add_function(&fn_name, fn_type, None);
        self.functions.insert(fn_name.clone(), fn_val);

        // Create entry block.
        let entry = self.context.append_basic_block(fn_val, "entry");
        self.builder.position_at_end(entry);

        // Save locals and current_this.
        let saved_locals = std::mem::take(&mut self.locals);
        let saved_this = self.current_this;
        let saved_class_name = self.current_class_name.clone();
        let saved_ret_ty = self.current_ret_ty.clone();
        self.current_ret_ty = method.return_type.clone();

        // Set up `this` pointer.
        let this_param = fn_val
            .get_nth_param(0)
            .ok_or_else(|| format!("missing this param for {}", fn_name))?;
        let this_ptr = this_param.into_pointer_value();

        // Cast this to the struct pointer type for field access via GEP.
        let struct_ptr_type = self.context.ptr_type(AddressSpace::default());
        let struct_ptr = if this_ptr.get_type() != struct_ptr_type {
            self.builder
                .build_bit_cast(this_ptr, struct_ptr_type, "this.cast")
                .map_err(|e| format!("build_bit_cast this failed: {:?}", e))?
                .into_pointer_value()
        } else {
            this_ptr
        };

        self.current_this = Some(struct_ptr);
        self.current_class_name = Some(class_name.to_string());

        // Bind method parameters (skip `this`); container params alias the caller.
        for (i, p) in method.params.iter().enumerate() {
            let param_val = fn_val
                .get_nth_param((i + 1) as u32)
                .ok_or_else(|| format!("missing param {} for {}", i, fn_name))?;
            self.bind_param(&p.name, &p.typ, param_val)?;
        }

        // Compile method body.
        for s in &method.body {
            self.compile_stmt(s)?;
        }

        // Add a default return if the current block has no terminator.
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            match &method.return_type {
                Some(t) if !llvm_types::is_void(t) => {
                    let ty = self.sig_ret_llvm_type(Some(t))?.ok_or_else(|| {
                        format!("codegen: void return for '{}'", fn_name)
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
                            self.current_this = saved_this;
                            self.current_class_name = saved_class_name;
                            self.current_ret_ty = saved_ret_ty;
                            return Ok(fn_val);
                        }
                    };
                    self.builder
                        .build_return(Some(&zero))
                        .map_err(|e| format!("build_return zero failed: {:?}", e))?;
                }
                _ => {
                    self.builder
                        .build_return(None)
                        .map_err(|e| format!("build_return void failed: {:?}", e))?;
                }
            }
        }

        // Restore locals and this.
        self.locals = saved_locals;
        self.current_this = saved_this;
        self.current_class_name = saved_class_name;
        self.current_ret_ty = saved_ret_ty;

        Ok(fn_val)
    }

    /// Compile a per-class clone of an interface default method so calls on
    /// `this` inside the body resolve against the implementing class (the
    /// shared default only sees the interface). Clones are memoized by
    /// `Class_method` name; a default that calls itself resolves to the
    /// in-progress clone, matching recursive-method behavior.
    fn ensure_default_clone(
        &mut self,
        class_name: &str,
        method_sig: &crate::ast::MethodSig,
        body: &[Stmt],
    ) -> Result<FunctionValue<'ctx>, String> {
        let fn_name = format!("{}_{}", class_name, method_sig.name);
        if let Some(existing) = self.module.get_function(&fn_name) {
            return Ok(existing);
        }
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let mut param_types: Vec<inkwell::types::BasicMetadataTypeEnum> = Vec::new();
        param_types.push(i8_ptr.into());
        for p in &method_sig.params {
            let ty = self.user_param_llvm_type(&p.typ)?;
            param_types.push(ty.into());
        }
        let return_type = self.sig_ret_llvm_type(method_sig.return_type.as_ref())?;
        let fn_type = match return_type {
            Some(ret) => ret.fn_type(&param_types, false),
            None => self.context.void_type().fn_type(&param_types, false),
        };
        let fn_val = self.module.add_function(&fn_name, fn_type, None);
        self.functions.insert(fn_name.clone(), fn_val);

        let entry = self.context.append_basic_block(fn_val, "entry");
        self.builder.position_at_end(entry);
        let saved_locals = std::mem::take(&mut self.locals);
        let saved_this = self.current_this;
        let saved_class_name = self.current_class_name.clone();
        let saved_ret_ty = self.current_ret_ty.clone();
        self.current_ret_ty = method_sig.return_type.clone();

        let this_param = fn_val
            .get_nth_param(0)
            .ok_or_else(|| format!("missing this param for {}", fn_name))?;
        self.current_this = Some(this_param.into_pointer_value());
        self.current_class_name = Some(class_name.to_string());

        for (i, p) in method_sig.params.iter().enumerate() {
            let param_val = fn_val
                .get_nth_param((i + 1) as u32)
                .ok_or_else(|| format!("missing param {} for {}", i, fn_name))?;
            self.bind_param(&p.name, &p.typ, param_val)?;
        }

        for s in body {
            self.compile_stmt(s)?;
        }

        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            match &method_sig.return_type {
                Some(t) if !llvm_types::is_void(t) => {
                    let ty = self.sig_ret_llvm_type(Some(t))?.ok_or_else(|| {
                        format!("codegen: void return for '{}'", fn_name)
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
                            self.current_this = saved_this;
                            self.current_class_name = saved_class_name;
                            self.current_ret_ty = saved_ret_ty;
                            return Ok(fn_val);
                        }
                    };
                    self.builder
                        .build_return(Some(&zero))
                        .map_err(|e| format!("build_return zero failed: {:?}", e))?;
                }
                _ => {
                    self.builder
                        .build_return(None)
                        .map_err(|e| format!("build_return void failed: {:?}", e))?;
                }
            }
        }

        self.locals = saved_locals;
        self.current_this = saved_this;
        self.current_class_name = saved_class_name;
        self.current_ret_ty = saved_ret_ty;

        Ok(fn_val)
    }

    /// Compile an interface declaration: register interface info, compile
    /// default methods, and build the fat pointer type.
    fn compile_interface_decl(&mut self, iface_decl: &InterfaceDecl) -> Result<(), String> {
        let iface_name = iface_decl.name.clone();
        let fat_ptr_type = build_interface_fat_ptr_type(self.context);

        // Collect method names.
        let mut method_names: Vec<String> = Vec::new();
        let mut default_methods: HashMap<String, FunctionValue<'ctx>> = HashMap::new();

        for method_sig in &iface_decl.methods {
            method_names.push(method_sig.name.clone());
            self.iface_method_sigs.insert(
                (iface_name.clone(), method_sig.name.clone()),
                IfaceMethodSig {
                    params: method_sig.params.iter().map(|p| p.typ.clone()).collect(),
                    ret: method_sig.return_type.clone(),
                },
            );

            // Compile default method body if present.
            if method_sig.body.is_some() {
                let i8_ptr = self.context.ptr_type(AddressSpace::default());
                let mut param_types: Vec<inkwell::types::BasicMetadataTypeEnum> = Vec::new();
                param_types.push(i8_ptr.into());
                for p in &method_sig.params {
                    let ty = self.user_param_llvm_type(&p.typ)?;
                    param_types.push(ty.into());
                }
                let return_type =
                    self.sig_ret_llvm_type(method_sig.return_type.as_ref())?;
                let fn_type = match return_type {
                    Some(ret) => ret.fn_type(&param_types, false),
                    None => self.context.void_type().fn_type(&param_types, false),
                };

                let fn_name = format!("{}_default_{}", iface_name, method_sig.name);
                let fn_val = self.module.add_function(&fn_name, fn_type, None);
                self.functions.insert(fn_name.clone(), fn_val);

                // The body is intentionally not compiled here: without an
                // implementing class, calls on `this` cannot resolve. Each
                // implementing class gets a compiled per-class clone (see
                // ensure_default_clone) that carries the real code, so this
                // shared declaration is only a vtable fallback entry.
                default_methods.insert(method_sig.name.clone(), fn_val);
            }
        }

        let iface_info = InterfaceInfo {
            name: iface_name.clone(),
            fat_ptr_type,
            method_names: method_names.clone(),
            default_methods,
            vtable_type: None,
        };

        self.interface_infos.insert(iface_name, iface_info);
        Ok(())
    }

    /// Compile a single statement.
    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Expr(expr) => {
                self.compile_expr(expr)?;
                Ok(())
            }
            Stmt::VarDecl(decl) | Stmt::ConstDecl(decl) => {
                self.compile_var_decl(decl)?;
                Ok(())
            }
            Stmt::Return(expr) => {
                self.compile_return(expr)?;
                Ok(())
            }
            Stmt::If(if_stmt) => {
                self.compile_if(if_stmt)?;
                Ok(())
            }
            Stmt::While(while_stmt) => {
                self.compile_while(while_stmt)?;
                Ok(())
            }
            Stmt::DoWhile(do_while_stmt) => {
                self.compile_do_while(do_while_stmt)?;
                Ok(())
            }
            Stmt::For(for_stmt) => {
                self.compile_for(for_stmt)?;
                Ok(())
            }
            Stmt::CFor(cfor_stmt) => {
                self.compile_c_for(cfor_stmt)?;
                Ok(())
            }
            Stmt::Break => {
                let ctx = self
                    .loop_stack
                    .last()
                    .ok_or("codegen: break outside of loop")?;
                self.builder
                    .build_unconditional_branch(ctx.break_block)
                    .map_err(|e| format!("build_br break failed: {:?}", e))?;
                Ok(())
            }
            Stmt::Continue => {
                let ctx = self
                    .loop_stack
                    .last()
                    .ok_or("codegen: continue outside of loop")?;
                self.builder
                    .build_unconditional_branch(ctx.continue_block)
                    .map_err(|e| format!("build_br continue failed: {:?}", e))?;
                Ok(())
            }
            Stmt::Block(block) => {
                self.ownership.enter_scope();
                for s in block {
                    self.compile_stmt(s)?;
                }
                self.emit_scope_cleanup()?;
                Ok(())
            }
            Stmt::Switch(switch_stmt) => {
                self.compile_switch(switch_stmt)?;
                Ok(())
            }
            Stmt::Throw(expr, _) => {
                self.compile_throw(expr)?;
                Ok(())
            }
            Stmt::TryCatch {
                try_block,
                catch_var,
                catch_var_type,
                catch_block,
                ..
            } => {
                self.compile_try_catch(try_block, catch_var, catch_var_type.as_ref(), catch_block)?;
                Ok(())
            }
            Stmt::With(with_stmt) => {
                self.compile_with(with_stmt)?;
                Ok(())
            }
            Stmt::TupleDestructure { names, expr, .. } => {
                self.compile_tuple_destructure(names, expr)?;
                Ok(())
            }
            _ => Err(format!("codegen: unsupported statement: {:?}", stmt)),
        }
    }

    /// Compile a return statement.
    fn compile_return(&mut self, expr: &Option<Expr>) -> Result<(), String> {
        match expr {
            None => {
                // Check the actual LLVM function return type to generate the right ret.
                let current_fn = self.builder.get_insert_block().and_then(|b| b.get_parent());
                if let Some(fn_val) = current_fn {
                    if fn_val.get_type().get_return_type().is_none() {
                        // True void function.
                        self.builder
                            .build_return(None)
                            .map_err(|e| format!("build_return void failed: {:?}", e))?;
                    } else {
                        // Function returns a value but we have no expression.
                        // Return default zero.
                        let ret_ty = fn_val.get_type().get_return_type().unwrap();
                        let zero: BasicValueEnum<'ctx> = match ret_ty {
                            BasicTypeEnum::IntType(it) => it.const_int(0, false).into(),
                            BasicTypeEnum::FloatType(ft) => ft.const_float(0.0).into(),
                            BasicTypeEnum::PointerType(pt) => pt.const_null().into(),
                            BasicTypeEnum::StructType(_) => {
                                self.builder.build_return(None).map_err(|e| {
                                    format!("build_return struct default failed: {:?}", e)
                                })?;
                                return Ok(());
                            }
                            _ => {
                                self.builder
                                    .build_return(None)
                                    .map_err(|e| format!("build_return default failed: {:?}", e))?;
                                return Ok(());
                            }
                        };
                        self.builder
                            .build_return(Some(&zero))
                            .map_err(|e| format!("build_return zero failed: {:?}", e))?;
                    }
                } else {
                    self.builder
                        .build_return(None)
                        .map_err(|e| format!("build_return void failed: {:?}", e))?;
                }
            }
            Some(e) => {
                // Infer the static type first: wrapping an object returned
                // where an interface is declared needs the class name, but
                // the value must be compiled exactly once.
                let expr_ty = self.infer_expr_type(e);
                let mut v = self.compile_expr(e)?;
                // An object returned where an interface is declared becomes a
                // fat pointer; values already of that interface pass through.
                if let Some(rt) = self.current_ret_ty.clone() {
                    if self.is_interface_ty(&rt) && v.is_pointer_value() {
                        if expr_ty.name() == rt.name() {
                            return Err(format!(
                                "codegen: interface '{}' value degraded to a raw pointer",
                                rt.name()
                            ));
                        }
                        v = self.wrap_object_ptr_in_interface(
                            v.into_pointer_value(),
                            rt.name(),
                            expr_ty.name(),
                        )?;
                    }
                }
                // If the current function returns void, discard the value and return void.
                let current_fn = self.builder.get_insert_block().and_then(|b| b.get_parent());
                if let Some(fn_val) = current_fn {
                    if fn_val.get_type().get_return_type().is_none() {
                        self.builder
                            .build_return(None)
                            .map_err(|e| format!("build_return void failed: {:?}", e))?;
                        return Ok(());
                    }
                    // Cast the return value to match the function's declared return type.
                    let ret_ty = fn_val.get_type().get_return_type().unwrap();
                    let v = self
                        .cast_value_to_type(v, ret_ty)
                        .map_err(|e| format!("return type cast failed: {:?}", e))?;
                    self.builder
                        .build_return(Some(&v))
                        .map_err(|e| format!("build_return failed: {:?}", e))?;
                } else {
                    self.builder
                        .build_return(Some(&v))
                        .map_err(|e| format!("build_return failed: {:?}", e))?;
                }
            }
        }
        Ok(())
    }

    /// Compile a `let`/`var`/`const` declaration.
    fn compile_var_decl(&mut self, decl: &crate::ast::VarDecl) -> Result<(), String> {
        let init = decl
            .init
            .as_ref()
            .ok_or_else(|| format!("codegen: variable '{}' has no initializer", decl.name))?;

        // Determine the declared type (if any).
        let declared_ty = decl.typ.as_ref();

        // String variable.
        let is_string = declared_ty.map(llvm_types::is_string).unwrap_or(false);
        if is_string {
            let sv = self.compile_string_expr(init)?;
            let string_ty = llvm_types::string_type(self.context).into_struct_type();
            let alloca = self
                .builder
                .build_alloca(string_ty, &decl.name)
                .map_err(|e| format!("build_alloca '{}' failed: {:?}", decl.name, e))?;
            let len_ptr = self
                .builder
                .build_struct_gep(string_ty, alloca, 0, &format!("{}.len", decl.name))
                .map_err(|e| format!("build_struct_gep 0 failed: {:?}", e))?;
            let ptr_ptr = self
                .builder
                .build_struct_gep(string_ty, alloca, 1, &format!("{}.ptr", decl.name))
                .map_err(|e| format!("build_struct_gep 1 failed: {:?}", e))?;
            self.builder
                .build_store(len_ptr, sv.len)
                .map_err(|e| format!("build_store len failed: {:?}", e))?;
            self.builder
                .build_store(ptr_ptr, sv.ptr)
                .map_err(|e| format!("build_store ptr failed: {:?}", e))?;
            self.locals.insert(
                decl.name.clone(),
                LocalVar {
                    ptr: alloca,
                    ty: string_ty.into(),
                    full_type: declared_ty.cloned(),
                    titrate_type: Some("string".to_string()),
                    shared: false,
                closure_sig: None,
                },
            );
            return Ok(());
        }

        // Primitive (or inferred) variable.
        //
        // Reference binding: when the value is a shared container and the
        // initializer names an existing slot (`let y = nums`, `let y = obj.items`),
        // alias that slot so mutations stay visible (VM reference semantics).
        // Whole-value rebinding later detaches to a private copy.
        // The analyzer pre-fills unresolvable `let` types as `unknown`;
        // treat those as uninferred and fall back to codegen inference.
        let effective_declared: Option<&Type> = match declared_ty {
            Some(t) if t.name() == "unknown" => None,
            other => other,
        };
        let alias_ty: Option<Type> = match effective_declared {
            Some(t) if Self::is_shared_container_ty(t) => Some(t.clone()),
            Some(_) => None,
            None => {
                let inferred = self.infer_expr_type(init);
                if Self::is_shared_container_ty(&inferred) {
                    Some(inferred)
                } else {
                    None
                }
            }
        };
        if let Some(container_ty) = alias_ty {
            let slot = self.container_slot_ptr(init)?;
            let struct_ty: BasicTypeEnum<'ctx> =
                native_bridge::titrate_array_type(self.context).into();
            // Prefer a real declared type, but never the analyzer's
            // `unknown` placeholder: element-aware accessors (get/for-in)
            // need the true container type.
            let full_ty = match effective_declared {
                Some(t) => Some(t.clone()),
                None => Some(container_ty.clone()),
            };
            self.locals.insert(
                decl.name.clone(),
                LocalVar {
                    ptr: slot,
                    ty: struct_ty,
                    full_type: full_ty,
                    titrate_type: Some(container_ty.name().to_string()),
                    shared: true,
                    closure_sig: None,
                },
            );
            return Ok(());
        }
        // A closure literal always materializes as the {fn_ptr, capture}
        // struct, regardless of the declared `fn` type (which lowers to an
        // opaque pointer and cannot hold the struct).
        let is_closure_init = matches!(init, Expr::Closure { .. });
        let ty = if is_closure_init {
            let i8_ptr = self.context.ptr_type(AddressSpace::default());
            self.context
                .struct_type(&[i8_ptr.into(), i8_ptr.into()], false)
                .into()
        } else {
            match declared_ty {
                Some(t) => {
                    // Interface bindings hold the fat pointer so method
                    // calls dispatch (mirrors interface params).
                    if self.is_interface_ty(t) {
                        build_interface_fat_ptr_type(self.context).into()
                    } else {
                        // For unknown class types (e.g., JsonValue, Regex), fall back to opaque pointer.
                        llvm_types::llvm_type(self.context, t).unwrap_or_else(|_| {
                            self.context.ptr_type(AddressSpace::default()).into()
                        })
                    }
                }
                None => {
                    // Infer from the initializer.
                    let v = self.compile_expr(init)?;
                    v.get_type()
                }
            }
        };

        let alloca = self
            .builder
            .build_alloca(ty, &decl.name)
            .map_err(|e| format!("build_alloca '{}' failed: {:?}", decl.name, e))?;

        // Compile the initializer with a type hint for literals.
        let init_val = match init {
            Expr::Literal(lit, _) => self.compile_literal(lit, declared_ty)?,
            _ => self.compile_expr(init)?,
        };

        // Interface bindings wrap a raw object pointer with its vtable.
        // A value that is already a fat pointer passes through untouched.
        let init_val = match declared_ty {
            Some(t) if self.is_interface_ty(t) && init_val.is_pointer_value() => {
                let obj_ptr = init_val.into_pointer_value();
                let class_name = self.infer_expr_type(init).name().to_string();
                self.wrap_object_ptr_in_interface(obj_ptr, t.name(), &class_name)?
            }
            _ => self.cast_value_to_type(init_val, ty)?,
        };

        self.builder
            .build_store(alloca, init_val)
            .map_err(|e| format!("build_store '{}' failed: {:?}", decl.name, e))?;

        // Infer titrate_type: use declared type if present, otherwise try to infer from the init expression.
        let inferred_type = declared_ty.map(|t| t.name().to_string()).or_else(|| {
            // When no explicit type, infer from `new ClassName(...)` expressions.
            if let Expr::New(type_name, _, _) = init {
                Some(type_name.name().to_string())
            } else {
                None
            }
        });
        // A closure literal initializer records its declared signature so
        // direct invocations through this local can be typed exactly.
        let closure_sig = match init {
            Expr::Closure { params, return_type, body, .. } => Some(ClosureSig {
                params: params.iter().map(|(_, ty)| ty.clone()).collect(),
                ret: return_type.clone(),
                returns_param: self.return_alias_param(
                    body,
                    params,
                    &mut std::collections::HashSet::new(),
                ),
            }),
            _ => None,
        };
        // Keep generic arguments for `new Class<T>(...)` initializers so
        // later uses resolve the specialization (e.g. Box<int>.get()).
        let full_type = declared_ty.cloned().or_else(|| match init {
            Expr::New(type_name, _, _) => Some(type_name.clone()),
            _ => None,
        });
        self.locals.insert(
            decl.name.clone(),
            LocalVar {
                ptr: alloca,
                ty,
                full_type,
                titrate_type: inferred_type,
                shared: false,
                closure_sig,
            },
        );
        Ok(())
    }

    /// Compile an if/else statement.
    fn compile_if(&mut self, if_stmt: &crate::ast::IfStmt) -> Result<(), String> {
        let cond_val = self.compile_expr(&if_stmt.condition)?;
        let cond = if cond_val.is_int_value() {
            let int_val = cond_val.into_int_value();
            if int_val.get_type() == self.context.bool_type() {
                int_val
            } else {
                // Coerce non-bool integer to bool (non-zero = true)
                let zero = int_val.get_type().const_int(0, false);
                self.builder
                    .build_int_compare(inkwell::IntPredicate::NE, int_val, zero, "cond.bool")
                    .map_err(|e| format!("build_int_compare bool coercion failed: {:?}", e))?
            }
        } else if cond_val.is_struct_value() {
            // Coerce struct to bool: check if the data pointer is non-null.
            if let BasicValueEnum::StructValue(sv) = cond_val {
                let st = sv.get_type();
                if st.count_fields() == 2 {
                    if let Some(BasicTypeEnum::PointerType(_)) = st.get_field_type_at_index(1) {
                        let ptr_val = self
                            .builder
                            .build_extract_value(sv, 1, "cond.ptr")
                            .map_err(|e| format!("extract failed: {:?}", e))?
                            .into_pointer_value();
                        let null_ptr = self.context.ptr_type(AddressSpace::default()).const_null();
                        self.builder
                            .build_int_compare(
                                inkwell::IntPredicate::NE,
                                ptr_val,
                                null_ptr,
                                "cond.bool",
                            )
                            .map_err(|e| format!("icmp failed: {:?}", e))?
                    } else {
                        return Err("codegen: unsupported struct condition type".into());
                    }
                } else {
                    return Err("codegen: unsupported struct condition type".into());
                }
            } else {
                unreachable!()
            }
        } else {
            return Err(format!(
                "codegen: if condition must be a bool, got {:?}",
                cond_val.get_type()
            ));
        };
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for if")?;
        let then_block = self
            .context
            .insert_basic_block_after(current_block, "if.then");
        let end_block = self.context.insert_basic_block_after(then_block, "if.end");

        let else_block = if if_stmt.else_branch.is_some() {
            Some(self.context.insert_basic_block_after(then_block, "if.else"))
        } else {
            None
        };

        if let Some(eb) = else_block {
            self.builder
                .build_conditional_branch(cond, then_block, eb)
                .map_err(|e| format!("build_cond_br if failed: {:?}", e))?;
        } else {
            self.builder
                .build_conditional_branch(cond, then_block, end_block)
                .map_err(|e| format!("build_cond_br if failed: {:?}", e))?;
        }

        // Then block.
        self.builder.position_at_end(then_block);
        for s in &if_stmt.then_branch {
            self.compile_stmt(s)?;
        }
        // Only add the branch to end if the current block doesn't already
        // have a terminator (e.g. from a return statement).
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(end_block)
                .map_err(|e| format!("build_br if.end failed: {:?}", e))?;
        }

        // Else block.
        if let (Some(eb), Some(else_branch)) = (else_block, &if_stmt.else_branch) {
            self.builder.position_at_end(eb);
            for s in else_branch {
                self.compile_stmt(s)?;
            }
            if self
                .builder
                .get_insert_block()
                .and_then(|b| b.get_terminator())
                .is_none()
            {
                self.builder
                    .build_unconditional_branch(end_block)
                    .map_err(|e| format!("build_br if.end 2 failed: {:?}", e))?;
            }
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile a while loop.
    fn compile_while(&mut self, while_stmt: &crate::ast::WhileStmt) -> Result<(), String> {
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for while")?;
        let cond_block = self
            .context
            .insert_basic_block_after(current_block, "while.cond");
        let body_block = self
            .context
            .insert_basic_block_after(cond_block, "while.body");
        let end_block = self
            .context
            .insert_basic_block_after(body_block, "while.end");

        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br while.cond failed: {:?}", e))?;

        // Condition block.
        self.builder.position_at_end(cond_block);
        let cond_val = self.compile_expr(&while_stmt.condition)?;
        let cond = if cond_val.is_int_value() {
            let int_val = cond_val.into_int_value();
            if int_val.get_type() == self.context.bool_type() {
                int_val
            } else {
                let zero = int_val.get_type().const_int(0, false);
                self.builder
                    .build_int_compare(inkwell::IntPredicate::NE, int_val, zero, "cond.bool")
                    .map_err(|e| format!("build_int_compare bool coercion failed: {:?}", e))?
            }
        } else if cond_val.is_struct_value() {
            if let BasicValueEnum::StructValue(sv) = cond_val {
                let st = sv.get_type();
                if st.count_fields() == 2 {
                    if let Some(BasicTypeEnum::PointerType(_)) = st.get_field_type_at_index(1) {
                        let ptr_val = self
                            .builder
                            .build_extract_value(sv, 1, "cond.ptr")
                            .map_err(|e| format!("extract failed: {:?}", e))?
                            .into_pointer_value();
                        let null_ptr = self.context.ptr_type(AddressSpace::default()).const_null();
                        self.builder
                            .build_int_compare(
                                inkwell::IntPredicate::NE,
                                ptr_val,
                                null_ptr,
                                "cond.bool",
                            )
                            .map_err(|e| format!("icmp failed: {:?}", e))?
                    } else {
                        return Err("codegen: unsupported struct condition type".into());
                    }
                } else {
                    return Err("codegen: unsupported struct condition type".into());
                }
            } else {
                unreachable!()
            }
        } else {
            return Err(format!(
                "codegen: while condition must be a bool, got {:?}",
                cond_val.get_type()
            ));
        };
        self.builder
            .build_conditional_branch(cond, body_block, end_block)
            .map_err(|e| format!("build_cond_br while failed: {:?}", e))?;

        // Body block.
        self.builder.position_at_end(body_block);
        self.loop_stack.push(LoopContext {
            continue_block: cond_block,
            break_block: end_block,
        });
        for s in &while_stmt.body {
            self.compile_stmt(s)?;
        }
        self.loop_stack.pop();
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(cond_block)
                .map_err(|e| format!("build_br while.cond 2 failed: {:?}", e))?;
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile a do-while loop.
    fn compile_do_while(&mut self, do_while_stmt: &crate::ast::DoWhileStmt) -> Result<(), String> {
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for do-while")?;
        let body_block = self
            .context
            .insert_basic_block_after(current_block, "do.body");
        let cond_block = self.context.insert_basic_block_after(body_block, "do.cond");
        let end_block = self.context.insert_basic_block_after(cond_block, "do.end");

        self.builder
            .build_unconditional_branch(body_block)
            .map_err(|e| format!("build_br do.body failed: {:?}", e))?;

        // Body block.
        self.builder.position_at_end(body_block);
        self.loop_stack.push(LoopContext {
            continue_block: cond_block,
            break_block: end_block,
        });
        for s in &do_while_stmt.body {
            self.compile_stmt(s)?;
        }
        self.loop_stack.pop();
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(cond_block)
                .map_err(|e| format!("build_br do.cond failed: {:?}", e))?;
        }

        // Condition block.
        self.builder.position_at_end(cond_block);
        let cond_val = self.compile_expr(&do_while_stmt.condition)?;
        let cond = if cond_val.is_int_value() {
            let int_val = cond_val.into_int_value();
            if int_val.get_type() == self.context.bool_type() {
                int_val
            } else {
                let zero = int_val.get_type().const_int(0, false);
                self.builder
                    .build_int_compare(inkwell::IntPredicate::NE, int_val, zero, "cond.bool")
                    .map_err(|e| format!("build_int_compare bool coercion failed: {:?}", e))?
            }
        } else if cond_val.is_struct_value() {
            if let BasicValueEnum::StructValue(sv) = cond_val {
                let st = sv.get_type();
                if st.count_fields() == 2 {
                    if let Some(BasicTypeEnum::PointerType(_)) = st.get_field_type_at_index(1) {
                        let ptr_val = self
                            .builder
                            .build_extract_value(sv, 1, "cond.ptr")
                            .map_err(|e| format!("extract failed: {:?}", e))?
                            .into_pointer_value();
                        let null_ptr = self.context.ptr_type(AddressSpace::default()).const_null();
                        self.builder
                            .build_int_compare(
                                inkwell::IntPredicate::NE,
                                ptr_val,
                                null_ptr,
                                "cond.bool",
                            )
                            .map_err(|e| format!("icmp failed: {:?}", e))?
                    } else {
                        return Err("codegen: unsupported struct condition type".into());
                    }
                } else {
                    return Err("codegen: unsupported struct condition type".into());
                }
            } else {
                unreachable!()
            }
        } else {
            return Err(format!(
                "codegen: do-while condition must be a bool, got {:?}",
                cond_val.get_type()
            ));
        };
        self.builder
            .build_conditional_branch(cond, body_block, end_block)
            .map_err(|e| format!("build_cond_br do-while failed: {:?}", e))?;

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile a for-in loop. Supports Range iteration
    /// (`for (i in start..end)`, `for (i in start..=end)`) and collection
    /// iteration over `ArrayList`/`array` values (`for (item in list)`).
    fn compile_for(&mut self, for_stmt: &crate::ast::ForStmt) -> Result<(), String> {
        // Range iteration.
        let (start_expr, end_expr, inclusive) = match &for_stmt.iterable {
            Expr::Range(s, e, _) => (s.as_ref(), e.as_ref(), false),
            Expr::RangeInclusive(s, e, _) => (s.as_ref(), e.as_ref(), true),
            other => {
                return self.compile_for_collection(for_stmt, other);
            }
        };

        let i64_ty = self.context.i64_type();

        // Compile start and end values.
        let start_val = self.compile_expr(start_expr)?;
        let end_val = self.compile_expr(end_expr)?;

        // Extend to i64 if needed.
        let start_i64 = if start_val.is_int_value() && start_val.get_type() != i64_ty.into() {
            self.builder
                .build_int_s_extend(start_val.into_int_value(), i64_ty, "for.start.ext")
                .map_err(|e| format!("build_int_s_extend start failed: {:?}", e))?
        } else {
            start_val.into_int_value()
        };
        let end_i64 = if end_val.is_int_value() && end_val.get_type() != i64_ty.into() {
            self.builder
                .build_int_s_extend(end_val.into_int_value(), i64_ty, "for.end.ext")
                .map_err(|e| format!("build_int_s_extend end failed: {:?}", e))?
        } else {
            end_val.into_int_value()
        };

        // Allocate the loop counter.
        let counter_alloca = self
            .builder
            .build_alloca(i64_ty, "for.i")
            .map_err(|e| format!("build_alloca for.i failed: {:?}", e))?;
        self.builder
            .build_store(counter_alloca, start_i64)
            .map_err(|e| format!("build_store for.i failed: {:?}", e))?;

        // Allocate the loop variable (visible to the body).
        let loop_var_ty = start_val.get_type();
        let loop_var_alloca = self
            .builder
            .build_alloca(loop_var_ty, &for_stmt.var)
            .map_err(|e| format!("build_alloca '{}' failed: {:?}", for_stmt.var, e))?;

        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for for")?;
        let cond_block = self
            .context
            .insert_basic_block_after(current_block, "for.cond");
        let body_block = self
            .context
            .insert_basic_block_after(cond_block, "for.body");
        let inc_block = self.context.insert_basic_block_after(body_block, "for.inc");
        let end_block = self.context.insert_basic_block_after(inc_block, "for.end");

        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br for.cond failed: {:?}", e))?;

        // Condition block: while counter < end (or <= for inclusive).
        self.builder.position_at_end(cond_block);
        let counter_val = self
            .builder
            .build_load(i64_ty, counter_alloca, "for.i.val")
            .map_err(|e| format!("build_load for.i failed: {:?}", e))?
            .into_int_value();
        let pred = if inclusive {
            inkwell::IntPredicate::SLE
        } else {
            inkwell::IntPredicate::SLT
        };
        let cmp = self
            .builder
            .build_int_compare(pred, counter_val, end_i64, "for.cmp")
            .map_err(|e| format!("build_int_compare for failed: {:?}", e))?;
        self.builder
            .build_conditional_branch(cmp, body_block, end_block)
            .map_err(|e| format!("build_cond_br for failed: {:?}", e))?;

        // Body block: store the counter (truncated to loop var type) in the
        // loop variable, then run the body.
        self.builder.position_at_end(body_block);
        let counter_for_body = if loop_var_ty != i64_ty.into() {
            self.builder
                .build_int_truncate(counter_val, loop_var_ty.into_int_type(), "for.i.trunc")
                .map_err(|e| format!("build_int_truncate for failed: {:?}", e))?
        } else {
            counter_val
        };
        self.builder
            .build_store(loop_var_alloca, counter_for_body)
            .map_err(|e| format!("build_store loop var failed: {:?}", e))?;
        // Register the loop variable in locals.
        let prev = self.locals.insert(
            for_stmt.var.clone(),
            LocalVar {
                ptr: loop_var_alloca,
                ty: loop_var_ty,
                full_type: None,
                titrate_type: None,
                shared: false,
                closure_sig: None,
            },
        );
        self.loop_stack.push(LoopContext {
            continue_block: inc_block,
            break_block: end_block,
        });
        for s in &for_stmt.body {
            self.compile_stmt(s)?;
        }
        self.loop_stack.pop();
        // Restore the previous binding (if any).
        if let Some(p) = prev {
            self.locals.insert(for_stmt.var.clone(), p);
        } else {
            self.locals.remove(&for_stmt.var);
        }
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(inc_block)
                .map_err(|e| format!("build_br for.inc failed: {:?}", e))?;
        }

        // Increment block: counter += 1.
        self.builder.position_at_end(inc_block);
        let counter_val = self
            .builder
            .build_load(i64_ty, counter_alloca, "for.i.inc")
            .map_err(|e| format!("build_load for.i.inc failed: {:?}", e))?
            .into_int_value();
        let one = i64_ty.const_int(1, false);
        let next = self
            .builder
            .build_int_add(counter_val, one, "for.i.next")
            .map_err(|e| format!("build_int_add for.inc failed: {:?}", e))?;
        self.builder
            .build_store(counter_alloca, next)
            .map_err(|e| format!("build_store for.i.next failed: {:?}", e))?;
        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br for.cond 2 failed: {:?}", e))?;

        // In release mode, attach vectorization hints to the loop latch
        // (the back-edge branch in the increment block). This tells LLVM's
        // loop vectorizer that this loop is a candidate for SIMD.
        if self.release_mode {
            // Metadata attachment is best-effort: if it fails we still
            // produce correct (just unannotated) code.
            let _ = self.add_vectorize_metadata(inc_block);
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile `for (item in collection)` over an `ArrayList`/`array` value.
    /// Iterates by index, reading each element with the element-aware
    /// `titrate_array_get_*` accessor so numeric elements are not corrupted.
    fn compile_for_collection(
        &mut self,
        for_stmt: &crate::ast::ForStmt,
        iterable: &Expr,
    ) -> Result<(), String> {
        let arr_val = self.compile_expr(iterable)?;
        if !arr_val.is_struct_value() {
            return Err(format!(
                "codegen: for-in iterable must be an ArrayList/array, got {:?}",
                iterable
            ));
        }
        let sv = arr_val.into_struct_value();
        let len_val = self
            .builder
            .build_extract_value(sv, 0, "for.len")
            .map_err(|e| format!("extract for-in len failed: {:?}", e))?
            .into_int_value();

        let i64_ty = self.context.i64_type();
        let elem_name = self
            .array_element_type_name(iterable)
            .unwrap_or_else(|| "string".to_string());
        let elem_ty = llvm_types::llvm_type(self.context, &Type::simple(&elem_name))
            .map_err(|e| format!("for-in element type: {}", e))?;
        let accessor = match elem_name.as_str() {
            "int" | "u32" | "byte" | "short" | "u8" | "u16" => "titrate_array_get_int",
            "long" | "u64" | "size" => "titrate_array_get_long",
            "double" | "float" | "half" | "quad" => "titrate_array_get_double",
            _ => "titrate_array_get_string",
        };

        // Allocate the loop counter (i64) and the loop variable.
        let counter_alloca = self
            .builder
            .build_alloca(i64_ty, "for.i")
            .map_err(|e| format!("build_alloca for.i failed: {:?}", e))?;
        self.builder
            .build_store(counter_alloca, i64_ty.const_int(0, false))
            .map_err(|e| format!("store for.i failed: {:?}", e))?;
        let loop_var_alloca = self
            .builder
            .build_alloca(elem_ty, &for_stmt.var)
            .map_err(|e| format!("build_alloca '{}' failed: {:?}", for_stmt.var, e))?;

        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for for-in")?;
        let cond_block = self
            .context
            .insert_basic_block_after(current_block, "for.cond");
        let body_block = self
            .context
            .insert_basic_block_after(cond_block, "for.body");
        let inc_block = self.context.insert_basic_block_after(body_block, "for.inc");
        let end_block = self.context.insert_basic_block_after(inc_block, "for.end");

        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br for.cond failed: {:?}", e))?;

        self.builder.position_at_end(cond_block);
        let counter_val = self
            .builder
            .build_load(i64_ty, counter_alloca, "for.i.val")
            .map_err(|e| format!("load for.i failed: {:?}", e))?
            .into_int_value();
        let cmp = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::SLT,
                counter_val,
                len_val,
                "for.cmp",
            )
            .map_err(|e| format!("for-in cmp failed: {:?}", e))?;
        self.builder
            .build_conditional_branch(cmp, body_block, end_block)
            .map_err(|e| format!("for-in cond br failed: {:?}", e))?;

        self.builder.position_at_end(body_block);
        // Load the current element via the element-aware accessor.
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let arr_ty = self
            .context
            .struct_type(&[i64_ty.into(), i8_ptr.into()], false);
        let arr_slot = self
            .builder
            .build_alloca(arr_ty, "for.arr.slot")
            .map_err(|e| format!("build_alloca for.arr.slot failed: {:?}", e))?;
        self.builder
            .build_store(arr_slot, sv)
            .map_err(|e| format!("store for.arr.slot failed: {:?}", e))?;
        let is_string_elem = accessor == "titrate_array_get_string";
        let ret_ty: BasicTypeEnum<'ctx> = if accessor == "titrate_array_get_double" {
            self.context.f64_type().into()
        } else if is_string_elem {
            llvm_types::string_type(self.context)
        } else {
            i64_ty.into()
        };
        // String elements come back through an out-pointer (struct returns
        // disagree across the FFI); scalars return directly as before.
        let fn_type = if is_string_elem {
            self.context.void_type().fn_type(
                &[i8_ptr.into(), i64_ty.into(), i8_ptr.into()],
                false,
            )
        } else {
            ret_ty.fn_type(&[i8_ptr.into(), i64_ty.into()], false)
        };
        let fn_val = if let Some(f) = self.module.get_function(accessor) {
            f
        } else {
            self.module.add_function(accessor, fn_type, Some(Linkage::External))
        };
        let elem_raw = if is_string_elem {
            let out_slot = self
                .builder
                .build_alloca(
                    llvm_types::string_type(self.context).into_struct_type(),
                    "for.elem.out",
                )
                .map_err(|e| format!("build_alloca for.elem.out failed: {:?}", e))?;
            self.builder
                .build_call(
                    fn_val,
                    &[arr_slot.into(), counter_val.into(), out_slot.into()],
                    "for.elem",
                )
                .map_err(|e| format!("for-in array_get failed: {:?}", e))?;
            self.builder
                .build_load(
                    llvm_types::string_type(self.context).into_struct_type(),
                    out_slot,
                    "for.elem.val",
                )
                .map_err(|e| format!("load for.elem.val failed: {:?}", e))?
        } else {
            let elem_call = self
                .builder
                .build_call(
                    fn_val,
                    &[arr_slot.into(), counter_val.into()],
                    "for.elem",
                )
                .map_err(|e| format!("for-in array_get failed: {:?}", e))?;
            match elem_call.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(v) => v,
                _ => return Err("for-in array_get did not return a value".to_string()),
            }
        };
        // Truncate i64 results to the element type when narrower (e.g. int).
        let elem_val = if elem_raw.get_type() != elem_ty {
            if elem_raw.is_int_value() && elem_ty.is_int_type() {
                let src_w = elem_raw.get_type().into_int_type().get_bit_width();
                let dst_w = elem_ty.into_int_type().get_bit_width();
                if src_w > dst_w {
                    self.builder
                        .build_int_truncate(elem_raw.into_int_value(), elem_ty.into_int_type(), "for.elem.trunc")
                        .map_err(|e| format!("for-in truncate failed: {:?}", e))?
                        .into()
                } else {
                    elem_raw
                }
            } else {
                elem_raw
            }
        } else {
            elem_raw
        };
        self.builder
            .build_store(loop_var_alloca, elem_val)
            .map_err(|e| format!("store loop var failed: {:?}", e))?;

        let prev = self.locals.insert(
            for_stmt.var.clone(),
            LocalVar {
                ptr: loop_var_alloca,
                ty: elem_ty,
                full_type: None,
                titrate_type: Some(elem_name.clone()),
                shared: false,
                closure_sig: None,
            },
        );
        self.loop_stack.push(LoopContext {
            continue_block: inc_block,
            break_block: end_block,
        });
        for s in &for_stmt.body {
            self.compile_stmt(s)?;
        }
        self.loop_stack.pop();
        if let Some(p) = prev {
            self.locals.insert(for_stmt.var.clone(), p);
        } else {
            self.locals.remove(&for_stmt.var);
        }
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(inc_block)
                .map_err(|e| format!("build_br for.inc failed: {:?}", e))?;
        }

        self.builder.position_at_end(inc_block);
        let counter_val = self
            .builder
            .build_load(i64_ty, counter_alloca, "for.i.inc")
            .map_err(|e| format!("load for.i.inc failed: {:?}", e))?
            .into_int_value();
        let next = self
            .builder
            .build_int_add(counter_val, i64_ty.const_int(1, false), "for.i.next")
            .map_err(|e| format!("for.i.inc failed: {:?}", e))?;
        self.builder
            .build_store(counter_alloca, next)
            .map_err(|e| format!("store for.i.next failed: {:?}", e))?;
        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br for.cond 2 failed: {:?}", e))?;

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Emit an optimized array-iteration loop using pointer arithmetic.
    ///
    /// Instead of recomputing `array[i]` via GEP on each iteration (which
    /// requires a multiply + add for the index), this loads the base pointer
    /// once and increments it by the element size on each iteration. This is
    /// the canonical pattern LLVM's loop vectorizer recognizes.
    ///
    /// The emitted loop structure is:
    ///   ```text
    ///   ptr = base; end = base + count;
    ///   while (ptr != end) {
    ///       elem = load ptr;
    ///       ... body ...
    ///       ptr = ptr + 1;  // gep by element size
    ///   }
    ///   ```
    ///
    /// This is used as the foundation for `for (x in array)` loops once array
    /// iteration is fully supported by the codegen.
    #[allow(dead_code)]
    fn emit_array_loop(
        &mut self,
        base_ptr: PointerValue<'ctx>,
        count: IntValue<'ctx>,
        elem_ty: BasicTypeEnum<'ctx>,
        var_name: &str,
        body: impl FnOnce(&mut Self, BasicValueEnum<'ctx>) -> Result<(), String>,
    ) -> Result<(), String> {
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());

        // Compute the end pointer: end = base + count (GEP by element).
        let end_ptr = unsafe {
            self.builder
                .build_in_bounds_gep(elem_ty, base_ptr, &[count], "arr.end.ptr")
        }
        .map_err(|e| format!("build_gep arr.end.ptr failed: {:?}", e))?;

        // Allocate the loop pointer (current position).
        let cur_ptr_alloca = self
            .builder
            .build_alloca(i8_ptr_ty, "arr.cur.ptr")
            .map_err(|e| format!("build_alloca arr.cur.ptr failed: {:?}", e))?;
        // Store the base pointer into it.
        let base_as_i8 = self
            .builder
            .build_bit_cast(base_ptr, i8_ptr_ty, "arr.base.i8")
            .map_err(|e| format!("build_bit_cast base failed: {:?}", e))?;
        self.builder
            .build_store(cur_ptr_alloca, base_as_i8)
            .map_err(|e| format!("build_store cur.ptr failed: {:?}", e))?;

        // Allocate the loop variable (visible to the body).
        let loop_var_alloca = self
            .builder
            .build_alloca(elem_ty, var_name)
            .map_err(|e| format!("build_alloca '{}' failed: {:?}", var_name, e))?;

        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for array loop")?;
        let cond_block = self
            .context
            .insert_basic_block_after(current_block, "arr.cond");
        let body_block = self
            .context
            .insert_basic_block_after(cond_block, "arr.body");
        let inc_block = self.context.insert_basic_block_after(body_block, "arr.inc");
        let end_block = self.context.insert_basic_block_after(inc_block, "arr.end");

        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br arr.cond failed: {:?}", e))?;

        // Condition block: while cur_ptr != end_ptr.
        self.builder.position_at_end(cond_block);
        let cur_ptr_val = self
            .builder
            .build_load(i8_ptr_ty, cur_ptr_alloca, "arr.cur.val")
            .map_err(|e| format!("build_load arr.cur failed: {:?}", e))?
            .into_pointer_value();
        let end_as_i8 = self
            .builder
            .build_bit_cast(end_ptr, i8_ptr_ty, "arr.end.i8")
            .map_err(|e| format!("build_bit_cast end failed: {:?}", e))?
            .into_pointer_value();
        // Convert pointers to integers for comparison.
        let intptr_ty = if cfg!(target_pointer_width = "64") {
            self.context.i64_type()
        } else {
            self.context.i32_type()
        };
        let cur_int = self
            .builder
            .build_ptr_to_int(cur_ptr_val, intptr_ty, "arr.cur.int")
            .map_err(|e| format!("build_ptr_to_int cur failed: {:?}", e))?;
        let end_int = self
            .builder
            .build_ptr_to_int(end_as_i8, intptr_ty, "arr.end.int")
            .map_err(|e| format!("build_ptr_to_int end failed: {:?}", e))?;
        let cmp = self
            .builder
            .build_int_compare(inkwell::IntPredicate::EQ, cur_int, end_int, "arr.cmp")
            .map_err(|e| format!("build_int_compare arr failed: {:?}", e))?;
        // If equal, we're done; otherwise enter body.
        self.builder
            .build_conditional_branch(cmp, end_block, body_block)
            .map_err(|e| format!("build_cond_br arr failed: {:?}", e))?;

        // Body block: load the current element and run the body.
        self.builder.position_at_end(body_block);
        let cur_for_elem = self
            .builder
            .build_load(i8_ptr_ty, cur_ptr_alloca, "arr.body.ptr")
            .map_err(|e| format!("build_load arr.body.ptr failed: {:?}", e))?
            .into_pointer_value();
        // Cast back to the element pointer type.
        let elem_ptr = self
            .builder
            .build_bit_cast(cur_for_elem, i8_ptr_ty, "arr.elem.ptr")
            .map_err(|e| format!("build_bit_cast elem.ptr failed: {:?}", e))?
            .into_pointer_value();
        let elem_val = self
            .builder
            .build_load(elem_ty, elem_ptr, "arr.elem")
            .map_err(|e| format!("build_load arr.elem failed: {:?}", e))?;
        self.builder
            .build_store(loop_var_alloca, elem_val)
            .map_err(|e| format!("build_store arr.elem failed: {:?}", e))?;

        // Run the body closure.
        body(self, elem_val)?;

        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(inc_block)
                .map_err(|e| format!("build_br arr.inc failed: {:?}", e))?;
        }

        // Increment block: advance the pointer by one element.
        self.builder.position_at_end(inc_block);
        let cur_ptr_val = self
            .builder
            .build_load(i8_ptr_ty, cur_ptr_alloca, "arr.inc.ptr")
            .map_err(|e| format!("build_load arr.inc.ptr failed: {:?}", e))?
            .into_pointer_value();
        // Advance by the element size in bytes.
        let elem_size = elem_ty
            .size_of()
            .ok_or_else(|| "codegen: cannot compute element size".to_string())?;
        let next_ptr = unsafe {
            self.builder.build_in_bounds_gep(
                self.context.i8_type(),
                cur_ptr_val,
                &[elem_size],
                "arr.next.ptr",
            )
        }
        .map_err(|e| format!("build_gep arr.next.ptr failed: {:?}", e))?;
        self.builder
            .build_store(cur_ptr_alloca, next_ptr)
            .map_err(|e| format!("build_store arr.next failed: {:?}", e))?;
        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br arr.cond 2 failed: {:?}", e))?;

        // In release mode, attach vectorization hints.
        if self.release_mode {
            let _ = self.add_vectorize_metadata(inc_block);
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile a C-style for loop.
    fn compile_c_for(&mut self, cfor_stmt: &crate::ast::CForStmt) -> Result<(), String> {
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for c-for")?;

        // Init statement (in the current block).
        if let Some(init) = &cfor_stmt.init {
            self.compile_stmt(init)?;
        }

        let cond_block = self.context.insert_basic_block_after(
            self.builder.get_insert_block().unwrap_or(current_block),
            "cfor.cond",
        );
        let body_block = self
            .context
            .insert_basic_block_after(cond_block, "cfor.body");
        let inc_block = self
            .context
            .insert_basic_block_after(body_block, "cfor.inc");
        let end_block = self.context.insert_basic_block_after(inc_block, "cfor.end");

        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br cfor.cond failed: {:?}", e))?;

        // Condition block.
        self.builder.position_at_end(cond_block);
        if let Some(cond) = &cfor_stmt.condition {
            let cond_val = self.compile_expr(cond)?.into_int_value();
            self.builder
                .build_conditional_branch(cond_val, body_block, end_block)
                .map_err(|e| format!("build_cond_br cfor failed: {:?}", e))?;
        } else {
            self.builder
                .build_unconditional_branch(body_block)
                .map_err(|e| format!("build_br cfor.body failed: {:?}", e))?;
        }

        // Body block.
        self.builder.position_at_end(body_block);
        self.loop_stack.push(LoopContext {
            continue_block: inc_block,
            break_block: end_block,
        });
        for s in &cfor_stmt.body {
            self.compile_stmt(s)?;
        }
        self.loop_stack.pop();
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(inc_block)
                .map_err(|e| format!("build_br cfor.inc failed: {:?}", e))?;
        }

        // Increment block.
        self.builder.position_at_end(inc_block);
        if let Some(incr) = &cfor_stmt.increment {
            self.compile_expr(incr)?;
        }
        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| format!("build_br cfor.cond 2 failed: {:?}", e))?;

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile a switch/case statement. For literal patterns, uses LLVM's
    /// `switch` instruction. For wildcard `_`, uses the default case.
    /// For enum and `Result` discriminators (tagged `{ i32, ptr }` values),
    /// branches on the variant tag and binds the payload fields into locals.
    fn compile_switch(&mut self, switch_stmt: &crate::ast::SwitchStmt) -> Result<(), String> {
        use crate::ast::Pattern;
        // Resolve the full switch type (with generic params) so Result/enum
        // pattern bindings get the correct element types.
        let switch_ty = if let Expr::Identifier(name, _) = &switch_stmt.expr {
            self.locals
                .get(name)
                .and_then(|v| v.full_type.clone())
                .unwrap_or_else(|| self.infer_expr_type(&switch_stmt.expr))
        } else {
            self.infer_expr_type(&switch_stmt.expr)
        };
        let type_name = switch_ty.name();
        let val = self.compile_expr(&switch_stmt.expr)?;
        let i32_ty = self.context.i32_type();

        let enum_info = self.enum_infos.get(type_name).cloned();
        let is_result = type_name == "Result";

        // Compute the i32 discriminator:
        // - int: used directly
        // - Result { i32 tag, ptr payload }: extract tag
        // - enum pointer -> { i32 tag, ptr payload }: load struct, extract tag
        let disc: IntValue<'ctx> = if is_result {
            let sv = val
                .into_struct_value();
            self.builder
                .build_extract_value(sv, 0, "switch.tag")
                .map_err(|e| format!("build_extract_value switch tag failed: {:?}", e))?
                .into_int_value()
        } else if enum_info.is_some() {
            let ptr = val.into_pointer_value();
            let enum_ty = enum_info.as_ref().unwrap().struct_type;
            let loaded = self
                .builder
                .build_load(enum_ty, ptr, "switch.enum")
                .map_err(|e| format!("build_load switch enum failed: {:?}", e))?
                .into_struct_value();
            self.builder
                .build_extract_value(loaded, 0, "switch.tag")
                .map_err(|e| format!("build_extract_value enum tag failed: {:?}", e))?
                .into_int_value()
        } else {
            val.into_int_value()
        };

        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for switch")?;
        let end_block = self
            .context
            .insert_basic_block_after(current_block, "switch.end");

        // Create basic blocks for each case.
        let mut case_blocks: Vec<(inkwell::basic_block::BasicBlock<'ctx>, &crate::ast::Case)> =
            Vec::new();
        let mut prev_block = current_block;
        for case in &switch_stmt.cases {
            let cb = self
                .context
                .insert_basic_block_after(prev_block, "switch.case");
            case_blocks.push((cb, case));
            prev_block = cb;
        }
        let default_block = if switch_stmt.default.is_some() {
            let db = self
                .context
                .insert_basic_block_after(prev_block, "switch.default");
            Some(db)
        } else {
            None
        };

        // Build the switch instruction: collect (value, block) pairs.
        let default_target = default_block.unwrap_or(end_block);
        let mut cases_vec: Vec<(IntValue<'ctx>, inkwell::basic_block::BasicBlock<'ctx>)> =
            Vec::new();
        for (cb, case) in &case_blocks {
            match &case.pattern {
                Pattern::Literal(Literal::Int(v)) => {
                    cases_vec.push((i32_ty.const_int(*v as u64, false), *cb));
                }
                Pattern::Constructor { name, .. } => {
                    let tag: Option<u64> = if is_result {
                        match name.as_str() {
                            "Ok" => Some(0),
                            "Err" => Some(1),
                            _ => None,
                        }
                    } else if let Some(enum_info) = &enum_info {
                        enum_codegen::variant_tag(&enum_info.variant_names, name).map(|t| t as u64)
                    } else {
                        None
                    };
                    if let Some(tag) = tag {
                        cases_vec.push((i32_ty.const_int(tag, false), *cb));
                    }
                }
                _ => {}
            }
        }
        self.builder
            .build_switch(disc, default_target, &cases_vec)
            .map_err(|e| format!("build_switch failed: {:?}", e))?;

        // Compile each case body.
        for (cb, case) in case_blocks {
            self.builder.position_at_end(cb);
            // Bind constructor pattern payload fields into locals.
            if let Pattern::Constructor { name, bindings } = &case.pattern {
                self.bind_switch_pattern(&switch_ty, &enum_info, is_result, name, bindings, val)?;
            }
            for s in &case.body {
                self.compile_stmt(s)?;
            }
            if self
                .builder
                .get_insert_block()
                .and_then(|b| b.get_terminator())
                .is_none()
            {
                self.builder
                    .build_unconditional_branch(end_block)
                    .map_err(|e| format!("build_br switch.end failed: {:?}", e))?;
            }
        }

        // Default case.
        if let Some(db) = default_block {
            if let Some(default_body) = &switch_stmt.default {
                self.builder.position_at_end(db);
                for s in default_body {
                    self.compile_stmt(s)?;
                }
                if self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_terminator())
                    .is_none()
                {
                    self.builder
                        .build_unconditional_branch(end_block)
                        .map_err(|e| format!("build_br switch.end 2 failed: {:?}", e))?;
                }
            }
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Bind the payload fields of a matched `Constructor` pattern into locals.
    fn bind_switch_pattern(
        &mut self,
        switch_ty: &Type,
        enum_info: &Option<enum_codegen::EnumInfo<'ctx>>,
        is_result: bool,
        name: &str,
        bindings: &[String],
        subject: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let i8_ptr = self.context.ptr_type(AddressSpace::default());

        // Extract the payload pointer (field 1 of the { i32, ptr } tag/value).
        let payload_ptr: PointerValue<'ctx> = if is_result {
            let sv = subject.into_struct_value();
            let p = self
                .builder
                .build_extract_value(sv, 1, "switch.payload")
                .map_err(|e| format!("extract switch payload failed: {:?}", e))?;
            p.into_pointer_value()
        } else {
            let enum_info = enum_info
                .as_ref()
                .ok_or_else(|| format!("enum info not found for '{}'", switch_ty.name()))?;
            let ptr = subject.into_pointer_value();
            let loaded = self
                .builder
                .build_load(enum_info.struct_type, ptr, "switch.enum.payload")
                .map_err(|e| format!("load enum payload failed: {:?}", e))?
                .into_struct_value();
            let p = self
                .builder
                .build_extract_value(loaded, 1, "switch.payload")
                .map_err(|e| format!("extract enum payload failed: {:?}", e))?;
            p.into_pointer_value()
        };

        // Resolve each binding's LLVM type and source slot.
        for (i, binding) in bindings.iter().enumerate() {
            if binding == "_" {
                continue;
            }
            if is_result {
                // Result payload: a heap pointer to a single value of T or E.
                let params = switch_ty.params();
                let ty = if name == "Ok" {
                    params.first()
                } else {
                    params.get(1)
                };
                let binding_ty = match ty {
                    Some(t) => llvm_types::llvm_type(self.context, t)
                        .map_err(|e| format!("switch binding type: {}", e))?,
                    None => self.context.i64_type().into(),
                };
                let typed_ptr = self
                    .builder
                    .build_bit_cast(payload_ptr, i8_ptr, "switch.payload.cast")
                    .map_err(|e| format!("bit_cast payload failed: {:?}", e))?
                    .into_pointer_value();
                let field_val = self
                    .builder
                    .build_load(binding_ty, typed_ptr, "switch.field.val")
                    .map_err(|e| format!("load switch field failed: {:?}", e))?;
                self.bind_switch_local(binding, binding_ty, field_val)?;
            } else {
                let enum_info = enum_info
                    .as_ref()
                    .ok_or_else(|| format!("enum info not found for '{}'", switch_ty.name()))?;
                let variant_idx = enum_codegen::variant_tag(&enum_info.variant_names, name)
                    .ok_or_else(|| format!("variant '{}' not found in enum '{}'", name, switch_ty.name()))?
                    as usize;
                let fields = &enum_info.variant_fields[variant_idx];
                if fields.is_empty() {
                    // Simple variant (no fields): bind a dummy zero.
                    let binding_ty: BasicTypeEnum<'ctx> = self.context.i64_type().into();
                    let zero: BasicValueEnum<'ctx> = binding_ty.into_int_type().const_int(0, false).into();
                    self.bind_switch_local(binding, binding_ty, zero)?;
                } else {
                    let (_, binding_ty) = fields
                        .get(i)
                        .cloned()
                        .ok_or_else(|| format!("variant '{}' has no field {}", name, i))?;
                    let payload_struct_ty = enum_info.variant_payload_types[variant_idx]
                        .ok_or_else(|| format!("variant '{}' has no payload", name))?;
                    let payload_typed = self
                        .builder
                        .build_bit_cast(payload_ptr, i8_ptr, "switch.payload.cast")
                        .map_err(|e| format!("bit_cast payload failed: {:?}", e))?
                        .into_pointer_value();
                    let gep = self
                        .builder
                        .build_struct_gep(payload_struct_ty, payload_typed, i as u32, "switch.field")
                        .map_err(|e| format!("struct_gep switch field failed: {:?}", e))?;
                    let field_val = self
                        .builder
                        .build_load(binding_ty, gep, "switch.field.val")
                        .map_err(|e| format!("load switch field failed: {:?}", e))?;
                    self.bind_switch_local(binding, binding_ty, field_val)?;
                }
            }
        }
        Ok(())
    }

    /// Create a local for a switch pattern binding and store the value.
    fn bind_switch_local(
        &mut self,
        name: &str,
        ty: BasicTypeEnum<'ctx>,
        value: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let alloca = self
            .builder
            .build_alloca(ty, name)
            .map_err(|e| format!("build_alloca switch binding failed: {:?}", e))?;
        self.builder
            .build_store(alloca, value)
            .map_err(|e| format!("store switch binding failed: {:?}", e))?;
        self.locals.insert(
            name.to_string(),
            LocalVar {
                ptr: alloca,
                ty,
                full_type: None,
                titrate_type: None,
                shared: false,
                closure_sig: None,
            },
        );
        Ok(())
    }

    /// Return the name of the enum that declares a variant, if any.
    fn enum_name_for_variant(&self, variant: &str) -> Option<String> {
        self.enum_infos
            .iter()
            .find(|(_, info)| info.variant_names.iter().any(|v| v == variant))
            .map(|(name, _)| name.clone())
    }

    /// Compile an `Owned<T>` dereference expression (`*expr`).
    ///
    /// The operand must evaluate to an `Owned<T>` value, which we represent
    /// as an `i8*` heap pointer stored in an alloca. The result is the
    /// loaded inner value.
    ///
    /// For Phase 1, we look up the operand's alloca (if it's an identifier)
    /// and use `ownership::load_owned_value` to load the inner value. The
    /// inner type is inferred from the variable's declared type.
    fn compile_owned_deref(&mut self, inner: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        // Get the pointer alloca for the Owned<T> value.
        let (ptr_alloca, inner_ty) = match inner {
            Expr::Identifier(name, _) => {
                let var = self.locals.get(name).ok_or_else(|| {
                    format!("codegen: unknown variable '{}' for owned deref", name)
                })?;
                // The variable's LLVM type is `i8*` (an opaque pointer).
                // We don't track the inner type separately, so we default
                // to i64 for the loaded value. This is sufficient for
                // Phase 1 micro-tests.
                let inner_ty = self.context.i64_type().into();
                (var.ptr, inner_ty)
            }
            _ => {
                // For non-identifier operands, evaluate the expression to
                // get a pointer value, store it in a temporary alloca, and
                // load from it.
                let v = self.compile_expr(inner)?;
                let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
                let alloca = self
                    .builder
                    .build_alloca(i8_ptr_ty, "deref.tmp")
                    .map_err(|e| format!("build_alloca deref.tmp failed: {:?}", e))?;
                self.builder
                    .build_store(alloca, v)
                    .map_err(|e| format!("build_store deref.tmp failed: {:?}", e))?;
                (alloca, self.context.i64_type().into())
            }
        };
        ownership::load_owned_value(self.context, &self.builder, inner_ty, ptr_alloca, "deref")
    }

    /// Compile a borrow expression (`&expr` or `&mut expr`).
    ///
    /// For an identifier, this returns the address of the existing alloca.
    /// For other expressions, the value is evaluated and stored in a fresh
    /// alloca, whose address is returned.
    fn compile_ref_expr(
        &mut self,
        inner: &Expr,
        _ref_kind: &crate::ast::RefKind,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
        match inner {
            Expr::Identifier(name, _) => {
                let var = self
                    .locals
                    .get(name)
                    .ok_or_else(|| format!("codegen: unknown variable '{}' for borrow", name))?;
                // Cast the alloca pointer to i8* (opaque pointer in LLVM 15+).
                let ptr = if var.ptr.get_type() == i8_ptr_ty {
                    var.ptr
                } else {
                    self.builder
                        .build_bit_cast(var.ptr, i8_ptr_ty, "ref.cast")
                        .map_err(|e| format!("build_bit_cast ref failed: {:?}", e))?
                        .into_pointer_value()
                };
                Ok(ptr.into())
            }
            _ => {
                // Evaluate the expression and store it in a temporary alloca.
                let v = self.compile_expr(inner)?;
                let ty = v.get_type();
                let alloca = self
                    .builder
                    .build_alloca(ty, "ref.tmp")
                    .map_err(|e| format!("build_alloca ref.tmp failed: {:?}", e))?;
                self.builder
                    .build_store(alloca, v)
                    .map_err(|e| format!("build_store ref.tmp failed: {:?}", e))?;
                let ptr = if alloca.get_type() == i8_ptr_ty {
                    alloca
                } else {
                    self.builder
                        .build_bit_cast(alloca, i8_ptr_ty, "ref.tmp.cast")
                        .map_err(|e| format!("build_bit_cast ref.tmp failed: {:?}", e))?
                        .into_pointer_value()
                };
                Ok(ptr.into())
            }
        }
    }

    /// Compile a `region.alloc<T>(expr)` expression.
    ///
    /// For Phase 1, this is treated like an `Owned<T>` allocation: we
    /// allocate heap memory, store the initial value, and return the
    /// pointer. Region lifetime markers are not emitted here (they are
    /// emitted by `compile_unsafe_block` for `region` blocks).
    fn compile_region_alloc(
        &mut self,
        ty: &Type,
        init: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let inner_ty = llvm_types::llvm_type(self.context, ty)?;
        let init_val = self.compile_expr(init)?;
        let init_val = self.cast_value_to_type(init_val, inner_ty)?;
        let (ptr_alloca, _drop_flag) = ownership::alloc_owned(
            self.context,
            &self.builder,
            &self.module,
            init_val,
            inner_ty,
            "region.alloc",
        )?;
        // Return the raw i8* pointer.
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
        let raw = self
            .builder
            .build_load(i8_ptr_ty, ptr_alloca, "region.ptr")
            .map_err(|e| format!("build_load region.ptr failed: {:?}", e))?;
        Ok(raw)
    }

    /// Compile an `unsafe { ... }` block.
    ///
    /// For Phase 1, this simply compiles the body statements in the current
    /// scope without any safety checks. The block evaluates to the last
    /// expression statement's value (or `0` if the block is empty).
    fn compile_unsafe_block(&mut self, block: &[Stmt]) -> Result<BasicValueEnum<'ctx>, String> {
        let i32_ty = self.context.i32_type();
        let mut last_val: BasicValueEnum<'ctx> = i32_ty.const_int(0, false).into();
        self.ownership.enter_scope();
        for s in block {
            // If the statement is an expression statement, capture its value.
            if let Stmt::Expr(e) = s {
                last_val = self.compile_expr(e)?;
            } else {
                self.compile_stmt(s)?;
            }
        }
        self.emit_scope_cleanup()?;
        Ok(last_val)
    }

    /// Compile a `with` statement.
    ///
    /// For Phase 1, this evaluates the resource expression, binds it to the
    /// optional variable, compiles the body, and that's it. No automatic
    /// `.close()` call is emitted (Phase 1 limitation).
    fn compile_with(&mut self, with_stmt: &crate::ast::WithStmt) -> Result<(), String> {
        // Evaluate the resource expression.
        let resource_val = self.compile_expr(&with_stmt.resource_expr)?;
        // Bind it to the variable if a name was given.
        if let Some(name) = &with_stmt.var_name {
            let ty = with_stmt
                .var_type
                .as_ref()
                .map(|t| llvm_types::llvm_type(self.context, t))
                .transpose()?
                .unwrap_or_else(|| resource_val.get_type());
            let alloca = self
                .builder
                .build_alloca(ty, name)
                .map_err(|e| format!("build_alloca with '{}' failed: {:?}", name, e))?;
            let val = self.cast_value_to_type(resource_val, ty)?;
            self.builder
                .build_store(alloca, val)
                .map_err(|e| format!("build_store with '{}' failed: {:?}", name, e))?;
            self.locals.insert(
                name.clone(),
                LocalVar {
                    ptr: alloca,
                    ty,
                    full_type: None,
                    titrate_type: None,
                    shared: false,
                closure_sig: None,
                },
            );
        }
        // Compile the body with scope-based cleanup tracking.
        self.ownership.enter_scope();
        for s in &with_stmt.body {
            self.compile_stmt(s)?;
        }
        self.emit_scope_cleanup()?;
        Ok(())
    }

    /// Emit cleanup actions for the current scope. This pops the current
    /// scope's cleanup actions from the ownership stack and emits them
    /// (only if the current basic block has no terminator).
    fn emit_scope_cleanup(&mut self) -> Result<(), String> {
        let actions = self.ownership.exit_scope();
        let has_terminator = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_some();
        if !has_terminator && !actions.is_empty() {
            ownership::emit_cleanup(self.context, &self.builder, &self.module, &actions)?;
        }
        Ok(())
    }

    /// Compile a tuple expression `(a, b, c)` into an anonymous struct.
    fn compile_tuple(&mut self, elements: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        let mut vals = Vec::with_capacity(elements.len());
        for e in elements {
            vals.push(self.compile_expr(e)?);
        }
        tuple_codegen::emit_tuple_construct(self.context, &self.builder, &self.module, &vals)
    }

    /// Compile tuple destructuring: `let (a, b) = tuple_expr`
    fn compile_tuple_destructure(&mut self, names: &[String], expr: &Expr) -> Result<(), String> {
        let tuple_val = self.compile_expr(expr)?;
        let struct_val = match tuple_val {
            BasicValueEnum::StructValue(sv) => sv,
            _ => return Err("tuple destructure: expected struct value".to_string()),
        };
        for (i, name) in names.iter().enumerate() {
            let field_val =
                tuple_codegen::emit_tuple_field_access(&self.builder, struct_val.into(), i as u32)?;
            let ty = field_val.get_type();
            let alloca = self
                .builder
                .build_alloca(ty, name)
                .map_err(|e| format!("build_alloca destructure '{}' failed: {:?}", name, e))?;
            self.builder
                .build_store(alloca, field_val)
                .map_err(|e| format!("build_store destructure '{}' failed: {:?}", name, e))?;
            self.locals.insert(
                name.clone(),
                LocalVar {
                    ptr: alloca,
                    ty,
                    full_type: None,
                    titrate_type: None,
                    shared: false,
                closure_sig: None,
                },
            );
        }
        Ok(())
    }

    /// Compile a closure expression into a `{ i8*, i8* }` struct.
    ///
    /// Phase 1: generates a trampoline function for each closure. The closure
    /// value is a 2-pointer struct:
    ///   - field 0: bitcast of the trampoline function to i8*
    ///   - field 1: capture data pointer (null for non-capturing closures)
    ///
    /// The trampoline signature is `fn(capture_data: i8*, params...) -> ret`.
    fn compile_closure(
        &mut self,
        params: &[(String, Type)],
        return_type: &Type,
        body: &[Stmt],
        expr: Option<&Expr>,
        captured_vars: &[String],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
        let void_ty = self.context.void_type();

        // Generate a unique name for the trampoline.
        let closure_id = self.closure_counter;
        self.closure_counter += 1;
        let tramp_name = format!("_closure_{}", closure_id);

        // Build the trampoline function type: fn(i8* capture_data, params...) -> ret
        // Container params pass as slot pointers (reference semantics, like
        // user functions); the trampoline binds them with bind_param.
        let mut param_tys: Vec<inkwell::types::BasicMetadataTypeEnum> = Vec::new();
        param_tys.push(i8_ptr_ty.into()); // capture_data
        for (_, ty) in params {
            let llvm_ty = self.user_param_llvm_type(ty)?;
            param_tys.push(llvm_ty.into());
        }
        let ret_ty = self.sig_ret_llvm_type(Some(return_type))?;
        let fn_type = match ret_ty {
            Some(rt) => rt.fn_type(&param_tys, false),
            None => void_ty.fn_type(&param_tys, false),
        };

        let tramp_fn = self
            .module
            .add_function(&tramp_name, fn_type, Some(Linkage::Internal));

        // Save current state (including the insert position: everything below
        // must be emitted into the trampoline, and the caller resumes in its
        // own block afterwards).
        let saved_block = self.builder.get_insert_block();
        let saved_ret_ty = self.current_ret_ty.clone();
        self.current_ret_ty = Some(return_type.clone());
        let saved_locals = self.locals.clone();
        self.locals.clear();

        // Create entry block.
        let entry = self.context.append_basic_block(tramp_fn, "entry");
        self.builder.position_at_end(entry);

        // Bind the capture_data parameter (i8*).
        let capture_data = tramp_fn
            .get_first_param()
            .ok_or("codegen: closure trampoline has no capture_data param")?;
        let capture_data_ptr = capture_data.into_pointer_value();

        // Bind closure parameters (skip capture_data, already handled).
        // Container params alias the caller's slot (reference semantics).
        let mut param_iter = tramp_fn.get_params().into_iter().skip(1);
        for (name, ty) in params {
            let param_val = param_iter
                .next()
                .ok_or_else(|| format!("missing param '{}'", name))?;
            self.bind_param(name, ty, param_val)?;
        }

        // If there are captured variables, bind them from the capture data.
        // Phase 1: captured vars are stored sequentially in the capture buffer.
        if !captured_vars.is_empty() {
            let i32_ty = self.context.i32_type();
            for (i, var_name) in captured_vars.iter().enumerate() {
                let offset = (i as u64) * 8;
                let offset_val = i32_ty.const_int(offset, false);
                let gep = unsafe {
                    self.builder.build_gep(
                        i8_ptr_ty,
                        capture_data_ptr,
                        &[offset_val],
                        &format!("capture.{}.gep", var_name),
                    )
                }
                .map_err(|e| format!("capture gep '{}': {:?}", var_name, e))?;
                let ptr_ptr_ty = self.context.ptr_type(AddressSpace::default());
                let slot_ptr_val = self
                    .builder
                    .build_bit_cast(gep, ptr_ptr_ty, &format!("capture.{}.ptr", var_name))
                    .map_err(|e| format!("capture bit_cast '{}': {:?}", var_name, e))?;
                let slot_ptr = slot_ptr_val.into_pointer_value();
                let captured_val = self
                    .builder
                    .build_load(i8_ptr_ty, slot_ptr, &format!("capture.{}.val", var_name))
                    .map_err(|e| format!("capture load '{}': {:?}", var_name, e))?;
                let alloca = self
                    .builder
                    .build_alloca(i8_ptr_ty, var_name)
                    .map_err(|e| format!("capture alloca '{}': {:?}", var_name, e))?;
                self.builder
                    .build_store(alloca, captured_val)
                    .map_err(|e| format!("capture store '{}': {:?}", var_name, e))?;
                self.locals.insert(
                    var_name.clone(),
                    LocalVar {
                        ptr: alloca,
                        ty: i8_ptr_ty.into(),
                        full_type: None,
                        titrate_type: None,
                        shared: false,
                closure_sig: None,
                    },
                );
            }
        }

        // Compile the closure body.
        for s in body {
            self.compile_stmt(s)?;
        }

        // If there's an expression body (arrow closure), compile and return it.
        if let Some(e) = expr {
            let val = self.compile_expr(e)?;
            if tramp_fn.get_type().get_return_type().is_some() {
                self.builder
                    .build_return(Some(&val))
                    .map_err(|e| format!("closure return build: {:?}", e))?;
            }
        }

        // If the function has no terminator, add one.
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_return(None)
                .map_err(|e| format!("closure ret void: {:?}", e))?;
        }

        // Restore caller state and resume emitting in the caller's block.
        // Without this, the closure struct and all following code would be
        // emitted into the trampoline (after its terminator), corrupting
        // both functions.
        self.locals = saved_locals;
        self.current_ret_ty = saved_ret_ty;
        if let Some(block) = saved_block {
            self.builder.position_at_end(block);
        }

        // Build the closure struct: { i8*, i8* } = { fn_ptr, capture_data }
        let closure_struct_ty = self
            .context
            .struct_type(&[i8_ptr_ty.into(), i8_ptr_ty.into()], false);

        let fn_ptr = tramp_fn.as_global_value().as_pointer_value();
        let fn_ptr_i8 = self
            .builder
            .build_bit_cast(fn_ptr, i8_ptr_ty, &format!("{}.cast", tramp_name))
            .map_err(|e| format!("fn_ptr bit_cast: {:?}", e))?;

        let capture_ptr = if captured_vars.is_empty() {
            i8_ptr_ty.const_null()
        } else {
            // Allocate the capture buffer and store captured values.
            // Phase 1: each captured variable is an i8* (8 bytes).
            let i64_ty = self.context.i64_type();
            let capture_size = i64_ty.const_int((captured_vars.len() * 8) as u64, false);
            let malloc_fn = self.get_function("titrate_malloc");
            let capture_buf = self
                .builder
                .build_call(
                    malloc_fn,
                    &[capture_size.into()],
                    &format!("{}.capture_buf", tramp_name),
                )
                .map_err(|e| format!("capture malloc: {:?}", e))?;
            let capture_buf = match capture_buf.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
                _ => return Err("capture malloc returned non-value".to_string()),
            };

            // Store each captured variable's value into the buffer.
            let i32_ty = self.context.i32_type();
            for (i, var_name) in captured_vars.iter().enumerate() {
                let offset = (i as u64) * 8;
                let offset_val = i32_ty.const_int(offset, false);
                let gep = unsafe {
                    self.builder.build_gep(
                        i8_ptr_ty,
                        capture_buf,
                        &[offset_val],
                        &format!("{}.capture{}.gep", tramp_name, i),
                    )
                }
                .map_err(|e| format!("capture store gep: {:?}", e))?;
                let ptr_ptr_ty = self.context.ptr_type(AddressSpace::default());
                let slot_val = self
                    .builder
                    .build_bit_cast(
                        gep,
                        ptr_ptr_ty,
                        &format!("{}.capture{}.slot", tramp_name, i),
                    )
                    .map_err(|e| format!("capture store bit_cast: {:?}", e))?;
                let slot = slot_val.into_pointer_value();

                if let Some(local) = self.locals.get(var_name) {
                    let val = self
                        .builder
                        .build_load(
                            local.ty,
                            local.ptr,
                            &format!("{}.capture{}.val", tramp_name, i),
                        )
                        .map_err(|e| format!("capture load: {:?}", e))?;
                    let val_ptr = self
                        .builder
                        .build_bit_cast(
                            val.into_pointer_value(),
                            i8_ptr_ty,
                            &format!("{}.capture{}.cast", tramp_name, i),
                        )
                        .map_err(|e| format!("capture cast: {:?}", e))?;
                    self.builder
                        .build_store(slot, val_ptr)
                        .map_err(|e| format!("capture store: {:?}", e))?;
                }
            }

            capture_buf
        };

        // Build the closure struct: { i8* fn_ptr, i8* capture_data }
        let undef = closure_struct_ty.const_zero();
        let result = self
            .builder
            .build_insert_value(undef, fn_ptr_i8, 0, &format!("{}.fn", tramp_name))
            .map_err(|e| format!("closure insert fn_ptr: {:?}", e))?;
        let result = self
            .builder
            .build_insert_value(result, capture_ptr, 1, &format!("{}.capture", tramp_name))
            .map_err(|e| format!("closure insert capture: {:?}", e))?;

        let result: BasicValueEnum<'ctx> = match result {
            inkwell::values::AggregateValueEnum::StructValue(sv) => sv.into(),
            _ => return Err("unexpected aggregate value".to_string()),
        };
        Ok(result)
    }
    /// Get the global exception pointer (`__titrate_exception`).
    fn get_exception_global(&self) -> PointerValue<'ctx> {
        self.module
            .get_global("__titrate_exception")
            .unwrap_or_else(|| panic!("__titrate_exception not declared"))
            .as_pointer_value()
    }

    /// Compile `ok(value)` or `err(value)` into a `Result<T, E>` struct.
    ///
    /// The Result struct is `{ i32, i8* }` where:
    ///   - field 0: tag (0 = Ok, 1 = Err)
    ///   - field 1: heap-allocated payload pointer
    fn compile_result_ctor(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!("codegen: `{}` expects exactly 1 argument", name));
        }
        let tag = if name == "ok" || name == "Ok" { 0u64 } else { 1u64 };
        let i32_ty = self.context.i32_type();
        let _i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
        let result_ty = llvm_types::result_type(self.context).into_struct_type();

        // Compile the argument.
        let arg_val = self.compile_expr(&args[0])?;
        let arg_ty = arg_val.get_type();
        let size = arg_ty
            .size_of()
            .ok_or_else(|| format!("cannot compute size of type {:?}", arg_ty))?;

        // Allocate heap memory for the payload.
        let malloc_fn = self.get_function("titrate_malloc");
        let payload_ptr = self
            .builder
            .build_call(malloc_fn, &[size.into()], &format!("{}.payload", name))
            .map_err(|e| format!("build_call titrate_malloc for {} failed: {:?}", name, e))?;
        let payload_ptr = match payload_ptr.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
            _ => {
                return Err(format!(
                    "titrate_malloc did not return a value for {}",
                    name
                ))
            }
        };

        // Store the value into the heap allocation.
        self.builder
            .build_store(payload_ptr, arg_val)
            .map_err(|e| format!("build_store {} payload failed: {:?}", name, e))?;

        // Build the Result struct: { i32 tag, i8* payload }.
        let tag_val = i32_ty.const_int(tag, false);
        let undef = result_ty.const_zero();
        let result = self
            .builder
            .build_insert_value(undef, tag_val, 0, &format!("{}.tag", name))
            .map_err(|e| format!("build_insert_value tag failed: {:?}", e))?;
        let result = self
            .builder
            .build_insert_value(result, payload_ptr, 1, &format!("{}.ptr", name))
            .map_err(|e| format!("build_insert_value payload failed: {:?}", e))?;

        let result: BasicValueEnum<'ctx> = match result {
            inkwell::values::AggregateValueEnum::StructValue(sv) => sv.into(),
            _ => return Err("unexpected aggregate value".to_string()),
        };
        Ok(result)
    }
    /// Compile an error propagation expression (`expr?`).
    ///
    /// The operand must evaluate to a `Result<T, E>` struct `{ i32, i8* }`.
    /// If the tag is 1 (Err), the payload is stored in `__titrate_exception`
    /// and control branches to the nearest catch block. If the tag is 0 (Ok),
    /// the payload is loaded from the heap and returned as the unwrapped value.
    fn compile_error_propagation(&mut self, inner: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        let i32_ty = self.context.i32_type();
        let _i8_ptr_ty = self.context.ptr_type(AddressSpace::default());
        let _result_ty = llvm_types::result_type(self.context).into_struct_type();

        // Compile the expression to get the Result struct.
        let result_val = self.compile_expr(inner)?;
        let result_struct = match result_val {
            BasicValueEnum::StructValue(sv) => sv,
            _ => return Err("codegen: `?` operand must be a Result struct".to_string()),
        };

        // Extract the tag (field 0).
        let tag = self
            .builder
            .build_extract_value(result_struct, 0, "q.tag")
            .map_err(|e| format!("build_extract_value tag failed: {:?}", e))?;
        let tag = tag.into_int_value();

        // Compare tag with 1 (Err).
        let one = i32_ty.const_int(1, false);
        let is_err = self
            .builder
            .build_int_compare(inkwell::IntPredicate::EQ, tag, one, "q.is_err")
            .map_err(|e| format!("build_int_compare is_err failed: {:?}", e))?;

        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for `?`")?;
        let ok_block = self.context.insert_basic_block_after(current_block, "q.ok");
        let err_block = self.context.insert_basic_block_after(ok_block, "q.err");
        let end_block = self.context.insert_basic_block_after(err_block, "q.end");

        self.builder
            .build_conditional_branch(is_err, err_block, ok_block)
            .map_err(|e| format!("build_cond_br ? failed: {:?}", e))?;

        // Ok block: extract payload, load value from heap, branch to end.
        self.builder.position_at_end(ok_block);
        let payload_ptr = self
            .builder
            .build_extract_value(result_struct, 1, "q.payload")
            .map_err(|e| format!("build_extract_value payload failed: {:?}", e))?;
        let payload_ptr = payload_ptr.into_pointer_value();
        let ok_val = self
            .builder
            .build_load(i32_ty, payload_ptr, "q.ok.val")
            .map_err(|e| format!("build_load ok value failed: {:?}", e))?;
        let ok_block_end = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block after ok")?;
        self.builder
            .build_unconditional_branch(end_block)
            .map_err(|e| format!("build_br ?.end failed: {:?}", e))?;

        // Err block: extract payload, store in __titrate_exception, branch to catch.
        self.builder.position_at_end(err_block);
        let err_payload = self
            .builder
            .build_extract_value(result_struct, 1, "q.err.payload")
            .map_err(|e| format!("build_extract_value err payload failed: {:?}", e))?;
        let err_payload = err_payload.into_pointer_value();
        let exception_global = self.get_exception_global();
        self.builder
            .build_store(exception_global, err_payload)
            .map_err(|e| format!("build_store exception failed: {:?}", e))?;

        // Branch to the nearest catch block, or unreachable if none.
        if let Some(ctx) = self.catch_stack.last() {
            self.builder
                .build_unconditional_branch(ctx.catch_block)
                .map_err(|e| format!("build_br catch failed: {:?}", e))?;
        } else {
            // No catch handler - emit an unreachable.
            self.builder
                .build_unreachable()
                .map_err(|e| format!("build_unreachable failed: {:?}", e))?;
        }

        // End block: phi the ok value.
        self.builder.position_at_end(end_block);
        let phi = self
            .builder
            .build_phi(i32_ty, "q.result")
            .map_err(|e| format!("build_phi ? failed: {:?}", e))?;
        phi.add_incoming(&[(&ok_val, ok_block_end)]);

        Ok(phi.as_basic_value())
    }

    /// Compile a `throw expr;` statement.
    ///
    /// The expression is evaluated and its value is stored in the global
    /// `__titrate_exception`. Control then branches to the nearest catch
    /// block. If no catch block is active, an unreachable is emitted
    /// (Phase 1 limitation).
    fn compile_throw(&mut self, expr: &Expr) -> Result<(), String> {
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());

        // Compile the thrown value.
        let v = self.compile_expr(expr)?;

        // Store the value in __titrate_exception. If the value is not i8*,
        // we store it as-is (Phase 1: assume string or pointer).
        let ptr = if v.is_pointer_value() {
            v.into_pointer_value()
        } else if v.is_int_value() {
            let iv = v.into_int_value();
            self.builder
                .build_int_to_ptr(iv, i8_ptr_ty, "throw.ptr")
                .map_err(|e| format!("build_int_to_ptr throw failed: {:?}", e))?
        } else {
            return Err(format!(
                "codegen: cannot throw value of type {:?}",
                v.get_type()
            ));
        };

        let exception_global = self.get_exception_global();
        self.builder
            .build_store(exception_global, ptr)
            .map_err(|e| format!("build_store throw exception failed: {:?}", e))?;

        // Branch to catch block or unreachable.
        if let Some(ctx) = self.catch_stack.last() {
            self.builder
                .build_unconditional_branch(ctx.catch_block)
                .map_err(|e| format!("build_br throw catch failed: {:?}", e))?;
        } else {
            self.builder
                .build_unreachable()
                .map_err(|e| format!("build_unreachable throw failed: {:?}", e))?;
        }

        Ok(())
    }

    /// Compile a `try { ... } catch (e: T) { ... }` statement.
    ///
    /// Sets up a catch block that receives the error value from
    /// `__titrate_exception`. The try body is compiled normally; if an
    /// exception is thrown (via `throw` or `?`), control jumps to the
    /// catch block. Otherwise, the catch block is skipped.
    fn compile_try_catch(
        &mut self,
        try_block: &[Stmt],
        catch_var: &str,
        catch_var_type: Option<&Type>,
        catch_block: &[Stmt],
    ) -> Result<(), String> {
        let i8_ptr_ty = self.context.ptr_type(AddressSpace::default());

        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("codegen: no insert block for try")?;
        let try_body_block = self
            .context
            .insert_basic_block_after(current_block, "try.body");
        let catch_handler_block = self
            .context
            .insert_basic_block_after(try_body_block, "try.catch");
        let end_block = self
            .context
            .insert_basic_block_after(catch_handler_block, "try.end");

        // Allocate a slot for the error value.
        let error_alloca = self
            .builder
            .build_alloca(i8_ptr_ty, "try.error")
            .map_err(|e| format!("build_alloca try.error failed: {:?}", e))?;

        // Push the catch context.
        self.catch_stack.push(CatchContext {
            catch_block: catch_handler_block,
            error_alloca,
        });

        // Branch to the try body.
        self.builder
            .build_unconditional_branch(try_body_block)
            .map_err(|e| format!("build_br try.body failed: {:?}", e))?;

        // Compile the try body.
        self.builder.position_at_end(try_body_block);
        for s in try_block {
            self.compile_stmt(s)?;
        }
        // Pop the catch context.
        self.catch_stack.pop();

        // If the try body completed normally (no throw), branch to end.
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(end_block)
                .map_err(|e| format!("build_br try.end failed: {:?}", e))?;
        }

        // Catch handler: load the error from __titrate_exception, bind it
        // to the catch variable, and compile the catch body.
        self.builder.position_at_end(catch_handler_block);
        let exception_global = self.get_exception_global();
        let error_val = self
            .builder
            .build_load(i8_ptr_ty, exception_global, "catch.err")
            .map_err(|e| format!("build_load exception failed: {:?}", e))?;

        // Determine the catch variable type. Default to i8* (pointer).
        let var_ty = match catch_var_type {
            Some(t) if llvm_types::is_string(t) => llvm_types::string_type(self.context),
            Some(t) => llvm_types::llvm_type(self.context, t)?,
            None => i8_ptr_ty.into(),
        };

        // Allocate the catch variable and store the error value.
        let var_alloca = self
            .builder
            .build_alloca(var_ty, catch_var)
            .map_err(|e| format!("build_alloca catch '{}' failed: {:?}", catch_var, e))?;

        // Store the error value into the catch variable.
        self.builder
            .build_store(var_alloca, error_val)
            .map_err(|e| format!("build_store catch error failed: {:?}", e))?;

        // Register the catch variable in locals.
        let prev = self.locals.insert(
            catch_var.to_string(),
            LocalVar {
                ptr: var_alloca,
                ty: var_ty,
                full_type: None,
                titrate_type: None,
                shared: false,
                closure_sig: None,
            },
        );

        // Compile the catch body.
        for s in catch_block {
            self.compile_stmt(s)?;
        }
        // Restore previous binding.
        if let Some(p) = prev {
            self.locals.insert(catch_var.to_string(), p);
        } else {
            self.locals.remove(catch_var);
        }

        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_unconditional_branch(end_block)
                .map_err(|e| format!("build_br try.end 2 failed: {:?}", e))?;
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }

    /// Compile a non-generic top-level function.
    fn compile_function(&mut self, fn_decl: &FnDecl) -> Result<FunctionValue<'ctx>, String> {
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
    fn compile_main(&mut self, fn_decl: &FnDecl) -> Result<FunctionValue<'ctx>, String> {
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
    fn resolve_module_file(
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
    fn load_module_declarations(
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
    fn write_object(&self, path: &Path, release: bool) -> Result<(), String> {
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
