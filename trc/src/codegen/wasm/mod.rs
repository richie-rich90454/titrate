//! WebAssembly code-generation backend.
//!
//! Lowers the typed Titrate AST to a standalone `.wasm` module. Arithmetic,
//! control flow and function calls are compiled to native WebAssembly
//! instructions; strings and a small set of runtime facilities (printing,
//! numeric formatting, parsing, string algorithms, math) are provided as
//! `env` imports by the host. See `docs/guide/wasm-backend.md` for the exact
//! runtime contract and the list of unsupported constructs.
//!
//! [`compile_tir_to_wasm`] is the first backend entry point that consumes the
//! shared TIR directly instead of lowering the AST again. It reads interned
//! names, field slots, method slots and resolved call targets from a
//! [`crate::tir::TirProgram`] and rejects constructs it cannot model with an
//! explicit error.

pub mod runtime;
mod closures;
mod composite;
mod containers;
mod dispatch;
mod dtypes;
mod json;
mod lower;
mod mono;
mod numeric;
mod tir_ext;
mod tir_lower;

pub use tir_ext::{
    compile_tir_extended_to_wasm, lower_tir_extended_to_wasm, ExtendedShape,
};
pub use tir_lower::{
    compile_tir_to_wasm, lower_tir_to_wasm, CallKind, CallShape, FieldAccessShape, FunctionShape,
    TirWasmShape, WasmValueKind,
};
pub use closures::{CaptureField, CapturePlan};
pub use containers::{
    host_alias, iteration_plan as container_iteration_plan, plan as container_method_plan,
    plan_native_typed, ContainerKind, IterationPlan, MethodPlan, NativeRewrite, NativeRewriteKind,
};
pub use dispatch::{find_slot as find_vtable_slot, resolve as resolve_dispatch, resolve_named, DispatchPlan};
pub use numeric::{
    emit_derived as emit_derived_numeric, emit_native as emit_native_numeric, plan as numeric_plan,
    DerivedNumericOp, NativeNumericOp, NumericPlan,
};

use std::collections::HashMap;

use wasm_encoder::{Ieee64, ValType};

use dtypes::DType;

use crate::ast::{
    Block, ClassDecl, Declaration, Expr, FieldDecl, FnDecl, Literal, MethodDecl, Operator, Param,
    Program, Stmt, Type, UnOp,
};

use runtime::{import_index, IMPORT_COUNT};

/// The WebAssembly representation chosen for each Titrate type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WTy {
    /// All signed/unsigned integer types (WASM i64).
    I64,
    /// All floating types (WASM f64).
    F64,
    /// bool (WASM i32, 0 or 1).
    Bool,
    /// char (WASM i32, Unicode scalar value).
    Char,
    /// string (WASM i32, pointer to a length-prefixed UTF-8 buffer).
    Str,
    /// A host object handle (WASM i32).
    Handle,
    /// No value.
    Void,
    /// An unsupported / dynamically typed value.
    Unknown,
}

impl WTy {
    fn val_type(self) -> Option<ValType> {
        match self {
            WTy::I64 => Some(ValType::I64),
            WTy::F64 => Some(ValType::F64),
            WTy::Bool | WTy::Char | WTy::Str | WTy::Handle => Some(ValType::I32),
            WTy::Void | WTy::Unknown => None,
        }
    }

}

fn ty_of(t: &Type) -> WTy {
    match t {
        Type::Tuple(_) | Type::Ref(_) | Type::MutRef(_) => WTy::Handle,
        Type::Named { name, .. } => match name.as_str() {
            "bool" => WTy::Bool,
            "char" => WTy::Char,
            "string" => WTy::Str,
            "float" | "double" | "half" | "quad" | "f64" => WTy::F64,
            "void" => WTy::Void,
            "byte" | "short" | "int" | "long" | "vast" | "uvast" | "size" | "u8" | "u16"
            | "u32" | "u64" => WTy::I64,
            // Enums, interfaces, collections and user classes are all
            // represented by host object handles at runtime.
            _ => WTy::Handle,
        },
    }
}

/// A virtual-dispatch signature: method name plus the physical WASM parameter
/// and result types (the leading `this` handle is implicit). Two overloads
/// with the same name but different signatures get distinct dispatch tables.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct MethodKey {
    name: String,
    params: Vec<WTy>,
    ret: WTy,
}

impl MethodKey {
    fn of_decl(name: &str, params: &[Param], ret: Option<&Type>) -> Self {
        MethodKey {
            name: name.to_string(),
            params: params.iter().map(|p| ty_of(&p.typ)).collect(),
            ret: ret.map(ty_of).unwrap_or(WTy::Void),
        }
    }

    /// Name of the generated function implementing this signature for a class.
    fn fname(&self, owner: &str) -> String {
        format!("{}::{}__{}", owner, self.name, self.suffix())
    }

    /// Name of the host-object field holding this signature's dispatch slot.
    fn vtbl_field(&self) -> String {
        format!("__vtbl_{}__{}", self.name, self.suffix())
    }

    fn suffix(&self) -> String {
        let mut s: String = self.params.iter().map(|t| t.code()).collect();
        s.push('_');
        s.push(self.ret.code());
        s
    }
}

impl WTy {
    fn code(self) -> char {
        match self {
            WTy::I64 => 'i',
            WTy::F64 => 'f',
            WTy::Bool => 'b',
            WTy::Char => 'c',
            WTy::Str => 's',
            WTy::Handle => 'o',
            WTy::Void => 'v',
            WTy::Unknown => 'u',
        }
    }
}

#[derive(Clone)]
struct FuncInfo {
    params: Vec<WTy>,
    ret: WTy,
    ret_dtype: DType,
    func_idx: u32,
    /// True for generated constructors: the WASM result is an object handle.
    #[allow(dead_code)]
    handle_ret: bool,
}

/// Saved local-variable state when entering a nested block.
type Scope = (
    HashMap<String, (u32, WTy)>,
    HashMap<String, Type>,
    HashMap<String, DType>,
);

/// A generated function (class constructor or method) owned by the backend.
#[derive(Clone)]
struct GenFn {
    name: String,
    params: Vec<Param>,
    return_type: Option<Type>,
    body: Block,
    handle_ret: bool,
    this_class: Option<String>,
    captures: Vec<(String, DType)>,
}

/// The fields and methods of one class, needed to lower member access.
struct ClassInfo {
    fields: Vec<FieldDecl>,
    methods: Vec<MethodDecl>,
    ctor: Option<MethodDecl>,
}

#[derive(Clone, Copy)]
enum CtrlLabel {
    Block,
    LoopHead,
}

#[derive(Clone, Copy)]
struct LoopCtx {
    exit_label: u32,
    head_label: u32,
}

/// The lowering engine. One instance drives the whole module; per-function
/// mutable state is reset by `begin_function`.
struct WasmBackend<'a> {
    program: &'a Program,

    // String literal pool.
    literals: Vec<Vec<u8>>,
    literal_offsets: Vec<i32>,
    heap_start: i32,

    // Function table.
    functions: HashMap<String, FuncInfo>,
    func_order: Vec<&'a FnDecl>,
    gen_funcs: Vec<GenFn>,
    classes: HashMap<String, ClassInfo>,
    /// All methods of every class, including inherited ones. Each name maps to
    /// every overload declared for it, in declaration order.
    all_class_methods: HashMap<String, HashMap<String, Vec<crate::ast::MethodDecl>>>,
    class_parents: HashMap<String, String>,
    /// Interface name -> implementing class names.
    iface_impls: HashMap<String, Vec<String>>,
    /// Interface name -> method signatures (including default method bodies).
    interface_methods: HashMap<String, HashMap<String, Vec<crate::ast::MethodSig>>>,
    enums: HashMap<String, HashMap<String, Vec<crate::ast::Type>>>,
    variant_enum: HashMap<String, String>,
    local_types: HashMap<String, Type>,
    /// Display indices for borrows, mirroring the VM's Value::Ref heap slots.
    ref_counter: u32,
    ref_indices: HashMap<String, u32>,
    local_dtypes: HashMap<String, DType>,
    global_dtypes: HashMap<String, DType>,
    current_class: Option<String>,
    current_error_label: Option<u32>,

    // Globals (heap pointer is global 0).
    global_defs: Vec<(ValType, u64)>,
    globals: HashMap<String, (u32, WTy)>,

    // Current function state.
    instrs: Vec<wasm_encoder::Instruction<'static>>,
    locals: Vec<(u32, ValType)>,
    local_count: u32,
    local_map: HashMap<String, (u32, WTy)>,
    scopes: Vec<Scope>,
    labels: Vec<CtrlLabel>,
    loops: Vec<LoopCtx>,
    current_ret: WTy,
    current_ret_dtype: DType,
    /// Closure-signature key -> (table index, WASM type index).
    closure_tables: HashMap<String, (u32, u32)>,
    /// Table index -> element function indices.
    table_elems: HashMap<u32, Vec<u32>>,
    next_table: u32,
    /// Captured variables of the closure currently being compiled.
    closure_captures: HashMap<String, DType>,
    env_local: Option<u32>,
    closure_counter: u32,
    /// Dispatch table index per method signature.
    method_tables: HashMap<MethodKey, u32>,
    /// (class, signature) -> slot inside that signature's dispatch table.
    method_slots: HashMap<(String, MethodKey), u32>,
    /// Method signature -> WASM type index shared by all implementations.
    method_type_idx: HashMap<MethodKey, u32>,
    /// Closure literal metadata, keyed by source span, filled in by a pre-pass.
    closure_meta: HashMap<(u32, u32), composite::ClosureMeta>,
}

impl<'a> WasmBackend<'a> {
    fn new(program: &'a Program) -> Self {
        WasmBackend {
            program,
            literals: Vec::new(),
            literal_offsets: Vec::new(),
            heap_start: 0,
            functions: HashMap::new(),
            func_order: Vec::new(),
            gen_funcs: Vec::new(),
            classes: HashMap::new(),
            all_class_methods: HashMap::new(),
            class_parents: HashMap::new(),
            iface_impls: HashMap::new(),
            interface_methods: HashMap::new(),
            enums: HashMap::new(),
            variant_enum: HashMap::new(),
            local_types: HashMap::new(),
            ref_counter: 0,
            ref_indices: HashMap::new(),
            local_dtypes: HashMap::new(),
            global_dtypes: HashMap::new(),
            current_class: None,
            current_error_label: None,
            global_defs: Vec::new(),
            globals: HashMap::new(),
            instrs: Vec::new(),
            locals: Vec::new(),
            local_count: 0,
            local_map: HashMap::new(),
            scopes: Vec::new(),
            labels: Vec::new(),
            loops: Vec::new(),
            current_ret: WTy::Void,
            current_ret_dtype: DType::Void,
            closure_tables: HashMap::new(),
            table_elems: HashMap::new(),
            next_table: 0,
            closure_captures: HashMap::new(),
            env_local: None,
            closure_counter: 0,
            method_tables: HashMap::new(),
            method_slots: HashMap::new(),
            method_type_idx: HashMap::new(),
            closure_meta: HashMap::new(),
        }
    }

    fn push(&mut self, instr: wasm_encoder::Instruction<'static>) {
        self.instrs.push(instr);
    }

    fn i32c(&mut self, v: i32) {
        self.push(wasm_encoder::Instruction::I32Const(v));
    }

    fn i64c(&mut self, v: i64) {
        self.push(wasm_encoder::Instruction::I64Const(v));
    }

    fn f64c(&mut self, v: f64) {
        self.push(wasm_encoder::Instruction::F64Const(Ieee64::new(v.to_bits())));
    }

    fn call_host(&mut self, name: &str) -> Result<(), String> {
        let idx = import_index(name)
            .ok_or_else(|| format!("wasm: unknown host import '{}'", name))?;
        self.push(wasm_encoder::Instruction::Call(idx));
        Ok(())
    }

    /// Compile `e` and push the four-word encoding `(tag, i64, f64, ptr)` used
    /// by the generic host storage imports.
    fn push_boxed_value(&mut self, e: &Expr) -> Result<(), String> {
        let mut dt = self.dtype_of(e);
        let from = self.compile_expr(e)?;
        let mut wty = dt.wty();
        if wty == WTy::Void || wty == WTy::Unknown {
            // Untyped expressions default to the integer representation.
            dt = DType::I64;
            wty = WTy::I64;
        }
        self.coerce(from, wty)?;
        if dt == DType::Char {
            self.call_host("to_string_char")?;
            dt = DType::Str;
        }
        self.emit_boxed_from_top(&dt);
        Ok(())
    }

    /// Store a value already on the stack into a fresh local of `dt`'s type.
    fn box_top_to_local(&mut self, dt: &DType) -> u32 {
        let tmp = self.add_local(dt.wty());
        self.push(wasm_encoder::Instruction::LocalSet(tmp));
        tmp
    }

    /// Emit the four-word `(tag, i64, f64, ptr)` encoding from a local.
    fn emit_boxed_parts(&mut self, idx: u32, dt: &DType) {
        use wasm_encoder::Instruction::*;
        self.i32c(dt.tag());
        match dt {
            DType::F64 => {
                self.i64c(0);
                self.push(LocalGet(idx));
                self.i32c(0);
            }
            DType::I64 => {
                self.push(LocalGet(idx));
                self.f64c(0.0);
                self.i32c(0);
            }
            _ => {
                self.i64c(0);
                self.f64c(0.0);
                self.push(LocalGet(idx));
            }
        }
    }

    /// Emit the four-word `(tag, i64, f64, ptr)` encoding for a value already
    /// on the stack in `dt`'s physical representation.
    fn emit_boxed_from_top(&mut self, dt: &DType) {
        let idx = self.box_top_to_local(dt);
        self.emit_boxed_parts(idx, dt);
    }

    // ---- string literal pool ------------------------------------------------

    fn intern_string(&mut self, s: &str) -> i32 {
        if let Some(pos) = self.literals.iter().position(|b| b.as_slice() == s.as_bytes()) {
            return self.literal_offsets[pos];
        }
        let offset = 16 + self.literals.iter().map(|b| 4 + b.len()).sum::<usize>() as i32;
        self.literals.push(s.as_bytes().to_vec());
        self.literal_offsets.push(offset);
        offset
    }

    fn collect_literals_in_expr(&mut self, e: &Expr) {
        match e {
            Expr::Literal(Literal::String(s), _) => {
                self.intern_string(s);
            }
            Expr::Binary(a, _, b, _) => {
                self.collect_literals_in_expr(a);
                self.collect_literals_in_expr(b);
            }
            Expr::Unary(_, a, _) | Expr::Cast(a, _, _) | Expr::Is(a, _, _) => {
                self.collect_literals_in_expr(a);
            }
            Expr::Call(c, args, ..) => {
                self.collect_literals_in_expr(c);
                for a in args {
                    self.collect_literals_in_expr(a);
                }
            }
            Expr::StaticCall { args, .. } | Expr::New(_, args, _) => {
                for a in args {
                    self.collect_literals_in_expr(a);
                }
            }
            Expr::Assign(a, b, _) | Expr::Range(a, b, _) | Expr::RangeInclusive(a, b, _) => {
                self.collect_literals_in_expr(a);
                self.collect_literals_in_expr(b);
            }
            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                self.collect_literals_in_expr(condition);
                self.collect_literals_in_expr(then_expr);
                self.collect_literals_in_expr(else_expr);
            }
            Expr::Index(o, i, _) => {
                self.collect_literals_in_expr(o);
                self.collect_literals_in_expr(i);
            }
            Expr::MemberAccess(o, _, _) => self.collect_literals_in_expr(o),
            Expr::Tuple(elems, _) => {
                for x in elems {
                    self.collect_literals_in_expr(x);
                }
            }
            Expr::ErrorPropagation(x, _)
            | Expr::OwnedDeref(x, _)
            | Expr::RegionAlloc(_, x, _)
            | Expr::RefExpr(x, _, _) => self.collect_literals_in_expr(x),
            Expr::UnsafeBlock(b, _) | Expr::RegionBlock(b, _) => self.collect_literals_in_block(b),
            Expr::Closure {
                body,
                expr,
                captured_vars,
                ..
            } => {
                self.collect_literals_in_block(body);
                if let Some(x) = expr {
                    self.collect_literals_in_expr(x);
                }
                for name in captured_vars {
                    self.intern_string(&format!("__cap_{}", name));
                }
            }
            _ => {}
        }
    }

    fn collect_literals_in_block(&mut self, block: &Block) {
        for stmt in block {
            self.collect_literals_in_stmt(stmt);
        }
    }

    fn collect_literals_in_stmt(&mut self, s: &Stmt) {
        match s {
            Stmt::Expr(e) | Stmt::Throw(e, _) => self.collect_literals_in_expr(e),
            Stmt::VarDecl(d) | Stmt::ConstDecl(d) => {
                if let Some(i) = &d.init {
                    self.collect_literals_in_expr(i);
                }
            }
            Stmt::Block(b) => self.collect_literals_in_block(b),
            Stmt::If(i) => {
                self.collect_literals_in_expr(&i.condition);
                self.collect_literals_in_block(&i.then_branch);
                if let Some(e) = &i.else_branch {
                    self.collect_literals_in_block(e);
                }
            }
            Stmt::While(w) => {
                self.collect_literals_in_expr(&w.condition);
                self.collect_literals_in_block(&w.body);
            }
            Stmt::DoWhile(d) => {
                self.collect_literals_in_expr(&d.condition);
                self.collect_literals_in_block(&d.body);
            }
            Stmt::For(f) => {
                self.collect_literals_in_expr(&f.iterable);
                self.collect_literals_in_block(&f.body);
            }
            Stmt::CFor(c) => {
                if let Some(i) = &c.init {
                    self.collect_literals_in_stmt(i);
                }
                if let Some(x) = &c.condition {
                    self.collect_literals_in_expr(x);
                }
                if let Some(x) = &c.increment {
                    self.collect_literals_in_expr(x);
                }
                self.collect_literals_in_block(&c.body);
            }
            Stmt::Return(Some(e)) => self.collect_literals_in_expr(e),
            Stmt::Switch(sw) => {
                self.collect_literals_in_expr(&sw.expr);
                for c in &sw.cases {
                    self.collect_literals_in_block(&c.body);
                }
                if let Some(d) = &sw.default {
                    self.collect_literals_in_block(d);
                }
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                ..
            } => {
                self.collect_literals_in_block(try_block);
                self.collect_literals_in_block(catch_block);
            }
            Stmt::TupleDestructure { expr, .. } => self.collect_literals_in_expr(expr),
            Stmt::WhileLet(w) => {
                self.collect_literals_in_expr(&w.expr);
                self.collect_literals_in_block(&w.body);
            }
            Stmt::With(w) => {
                self.collect_literals_in_expr(&w.resource_expr);
                self.collect_literals_in_block(&w.body);
            }
            _ => {}
        }
    }

    // ---- declarations -------------------------------------------------------

    fn collect_decls(&mut self) -> Result<(), String> {
        for decl in &self.program.declarations {
            match decl {
                Declaration::Function(f) => {
                    let params: Vec<WTy> = f.params.iter().map(|p| ty_of(&p.typ)).collect();
                    let ret = f
                        .return_type
                        .as_ref()
                        .map(ty_of)
                        .unwrap_or(WTy::Void);
                    let func_idx = IMPORT_COUNT + self.func_order.len() as u32;
                    self.functions.insert(
                        f.name.clone(),
                        FuncInfo {
                            params,
                            ret,
                            ret_dtype: f
                                .return_type
                                .as_ref()
                                .map(|t| self.dtype_of_type(t))
                                .unwrap_or(DType::Void),
                            func_idx,
                            handle_ret: false,
                        },
                    );
                    self.func_order.push(f);
                }
                Declaration::Class(_) => {}
                Declaration::Enum(e) => {
                    let mut variants = HashMap::new();
                    for v in &e.variants {
                        variants.insert(
                            v.name.clone(),
                            v.fields.iter().map(|p| p.typ.clone()).collect(),
                        );
                        self.variant_enum.insert(v.name.clone(), e.name.clone());
                    }
                    self.enums.insert(e.name.clone(), variants);
                }
                Declaration::Interface(i) => {
                    let mut sigs: HashMap<String, Vec<crate::ast::MethodSig>> = HashMap::new();
                    for m in &i.methods {
                        sigs.entry(m.name.clone()).or_default().push(m.clone());
                    }
                    self.interface_methods.insert(i.name.clone(), sigs);
                }
                Declaration::VarDecl(d) | Declaration::ConstDecl(d) => {
                    let dt = d
                        .typ
                        .as_ref()
                        .map(|t| self.dtype_of_type(t))
                        .or_else(|| d.init.as_ref().map(|e| self.dtype_of(e)))
                        .unwrap_or(DType::I64);
                    let ty = dt.wty();
                    if ty == WTy::Unknown || ty == WTy::Void {
                        continue;
                    }
                    let vt = ty.val_type().unwrap();
                    let init = d
                        .init
                        .as_ref()
                        .and_then(|e| self.const_expr_of(e, ty))
                        .unwrap_or(0);
                    let gidx = self.global_defs.len() as u32 + 1;
                    self.global_defs.push((vt, init));
                    self.globals.insert(d.name.clone(), (gidx, ty));
                    self.global_dtypes.insert(d.name.clone(), dt);
                }
            }
        }
        // Interface default bodies are generated only after every user
        // function is registered: `add_generated` derives function indices
        // from `func_order.len()`, which is incomplete during the loop above.
        for decl in &self.program.declarations {
            if let Declaration::Interface(i) = decl {
                for m in &i.methods {
                    if let Some(body) = &m.body {
                        let key = MethodKey::of_decl(&m.name, &m.params, m.return_type.as_ref());
                        let fname = key.fname(&i.name);
                        let mut ps = vec![Param {
                            name: "this".to_string(),
                            typ: Type::simple(&i.name),
                        }];
                        ps.extend(m.params.clone());
                        self.add_generated(
                            &fname,
                            ps,
                            m.return_type.clone(),
                            body.clone(),
                            false,
                            Some(i.name.clone()),
                        );
                    }
                }
            }
        }
        // Class functions are appended after all user functions so function
        // indices stay in declaration order.
        for decl in &self.program.declarations {
            if let Declaration::Class(c) = decl {
                self.collect_class(c)?;
            }
        }
        self.build_class_tables();
        Ok(())
    }

    /// Flatten inheritance and build the per-method dispatch tables.
    fn build_class_tables(&mut self) {
        for decl in &self.program.declarations {
            if let Declaration::Class(c) = decl {
                if let Some(p) = &c.parent {
                    self.class_parents.insert(c.name.clone(), p.name().to_string());
                }
                for i in &c.ifaces {
                    self.iface_impls
                        .entry(i.name().to_string())
                        .or_default()
                        .push(c.name.clone());
                }
            }
        }
        let classes: Vec<ClassDecl> = self
            .program
            .declarations
            .iter()
            .filter_map(|d| match d {
                Declaration::Class(c) => Some(c.clone()),
                _ => None,
            })
            .collect();
        for c in &classes {
            let mut methods: HashMap<String, Vec<crate::ast::MethodDecl>> = HashMap::new();
            if let Some(info) = self.classes.get(&c.name) {
                for m in &info.methods {
                    methods.entry(m.name.clone()).or_default().push(m.clone());
                }
            }
            let mut parent = c.parent.as_ref().map(|p| p.name().to_string());
            let mut guard = 0;
            while let Some(pname) = parent {
                guard += 1;
                if guard > 64 {
                    break;
                }
                if let Some(pc) = classes.iter().find(|x| x.name == pname) {
                    for m in &pc.members {
                        if let crate::ast::ClassMember::Method(md) = m {
                            methods
                                .entry(md.name.clone())
                                .or_insert_with(|| vec![md.clone()]);
                        }
                    }
                    parent = pc.parent.as_ref().map(|p| p.name().to_string());
                } else {
                    break;
                }
            }
            self.all_class_methods.insert(c.name.clone(), methods);
        }

        // Every distinct (name, signature) pair needs its own table. Include
        // interface method signatures so a default body can be reached even
        // when no concrete class redeclares the method.
        let mut keys: Vec<MethodKey> = Vec::new();
        for variants in self.all_class_methods.values() {
            for (name, mds) in variants {
                for md in mds {
                    keys.push(MethodKey::of_decl(name, &md.params, md.return_type.as_ref()));
                }
            }
        }
        for variants in self.interface_methods.values() {
            for (name, sigs) in variants {
                for sig in sigs {
                    keys.push(MethodKey::of_decl(name, &sig.params, sig.return_type.as_ref()));
                }
            }
        }
        keys.sort_by_key(|k| k.vtbl_field());
        keys.dedup();

        let mut cls: Vec<String> = self.classes.keys().cloned().collect();
        cls.sort();
        for key in keys {
            let mut elems: Vec<u32> = Vec::new();
            for c in &cls {
                if let Some(func_idx) = self.resolve_impl(c, &key) {
                    let slot = elems.len() as u32;
                    elems.push(func_idx);
                    self.method_slots.insert((c.clone(), key.clone()), slot);
                    // Defined functions have func_idx == type index: the type
                    // section mirrors the function section one-to-one.
                    self.method_type_idx
                        .entry(key.clone())
                        .or_insert(func_idx);
                }
            }
            if elems.is_empty() {
                continue;
            }
            let table = self.next_table;
            self.next_table += 1;
            self.table_elems.insert(table, elems);
            self.method_tables.insert(key, table);
        }
    }

    /// Find the generated function implementing `key` for `class`, walking the
    /// parent chain. Only the class's own declarations are inspected here;
    /// inherited overloads are resolved by walking to the declaring ancestor.
    fn find_class_impl(&self, class: &str, key: &MethodKey) -> Option<u32> {
        let mut current = Some(class.to_string());
        let mut guard = 0;
        while let Some(c) = current {
            guard += 1;
            if guard > 64 {
                break;
            }
            if let Some(info) = self.classes.get(&c) {
                for md in &info.methods {
                    if md.name == key.name
                        && MethodKey::of_decl(&md.name, &md.params, md.return_type.as_ref())
                            == *key
                    {
                        if let Some(fi) = self.functions.get(&key.fname(&c)) {
                            return Some(fi.func_idx);
                        }
                    }
                }
            }
            current = self.class_parents.get(&c).cloned();
        }
        None
    }

    /// Whether `class` or any ancestor declares any overload of `name`.
    fn declares_any(&self, class: &str, name: &str) -> bool {
        let mut current = Some(class.to_string());
        let mut guard = 0;
        while let Some(c) = current {
            guard += 1;
            if guard > 64 {
                break;
            }
            if self
                .classes
                .get(&c)
                .is_some_and(|info| info.methods.iter().any(|m| m.name == name))
            {
                return true;
            }
            current = self.class_parents.get(&c).cloned();
        }
        false
    }

    /// Resolve the implementation used by `class` for `key`. An explicit class
    /// method wins; an interface default is inherited only when no overload of
    /// the name is declared by the class or an ancestor (matching the VM).
    fn resolve_impl(&self, class: &str, key: &MethodKey) -> Option<u32> {
        if let Some(idx) = self.find_class_impl(class, key) {
            return Some(idx);
        }
        if self.declares_any(class, &key.name) {
            return None;
        }
        for (iface, impls) in &self.iface_impls {
            if !impls.iter().any(|c| c == class) {
                continue;
            }
            if let Some(sigs) = self.interface_methods.get(iface).and_then(|m| m.get(&key.name)) {
                for sig in sigs {
                    if sig.body.is_some()
                        && MethodKey::of_decl(&sig.name, &sig.params, sig.return_type.as_ref())
                            == *key
                    {
                        if let Some(fi) = self.functions.get(&key.fname(iface)) {
                            return Some(fi.func_idx);
                        }
                    }
                }
            }
        }
        None
    }

    fn collect_class(&mut self, c: &ClassDecl) -> Result<(), String> {
        use crate::ast::ClassMember;
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut ctor = None;
        for m in &c.members {
            match m {
                ClassMember::Field(f) => fields.push(f.clone()),
                ClassMember::Method(md) => methods.push(md.clone()),
                ClassMember::Constructor(md) => ctor = Some(md.clone()),
            }
        }

        if let Some(ctor) = &ctor {
            let fname = format!("{}::init", c.name);
            let mut ps = vec![Param {
                name: "this".to_string(),
                typ: Type::simple(&c.name),
            }];
            ps.extend(ctor.params.clone());
            self.add_generated(&fname, ps, None, ctor.body.clone(), true, Some(c.name.clone()));
        }
        for m in &methods {
            let key = MethodKey::of_decl(&m.name, &m.params, m.return_type.as_ref());
            let fname = key.fname(&c.name);
            let mut ps = vec![Param {
                name: "this".to_string(),
                typ: Type::simple(&c.name),
            }];
            ps.extend(m.params.clone());
            self.add_generated(&fname, ps, m.return_type.clone(), m.body.clone(), false, Some(c.name.clone()));
        }

        self.classes.insert(
            c.name.clone(),
            ClassInfo {
                fields,
                methods,
                ctor,
            },
        );
        Ok(())
    }

    /// Register a generated function, assigning it the next function index.
    fn add_generated(
        &mut self,
        name: &str,
        params: Vec<Param>,
        return_type: Option<Type>,
        body: Block,
        handle_ret: bool,
        this_class: Option<String>,
    ) {
        let func_idx = IMPORT_COUNT
            + self.func_order.len() as u32
            + self.gen_funcs.len() as u32;
        let wt_params: Vec<WTy> = params
            .iter()
            .map(|p| {
                if this_class.is_some() && p.name == "this" {
                    WTy::Handle
                } else {
                    ty_of(&p.typ)
                }
            })
            .collect();
        let wt_ret = if handle_ret {
            WTy::Handle
        } else {
            return_type.as_ref().map(ty_of).unwrap_or(WTy::Void)
        };
        self.functions.insert(
            name.to_string(),
            FuncInfo {
                params: wt_params,
                ret: wt_ret,
                ret_dtype: if handle_ret {
                    this_class
                        .as_ref()
                        .map(|c| DType::Obj(c.clone()))
                        .unwrap_or(DType::Unknown)
                } else {
                    return_type
                        .as_ref()
                        .map(|t| self.dtype_of_type(t))
                        .unwrap_or(DType::Void)
                },
                func_idx,
                handle_ret,
            },
        );
        self.gen_funcs.push(GenFn {
            name: name.to_string(),
            params,
            return_type,
            body,
            handle_ret,
            this_class,
            captures: Vec::new(),
        });
    }

    fn field_type(&self, class: &str, field: &str) -> Option<Type> {
        let mut current = Some(class.to_string());
        let mut guard = 0;
        while let Some(c) = current {
            guard += 1;
            if guard > 64 {
                break;
            }
            if let Some(t) = self
                .classes
                .get(&c)
                .and_then(|ci| ci.fields.iter().find(|f| f.name == field))
                .map(|f| f.typ.clone())
            {
                return Some(t);
            }
            current = self.class_parents.get(&c).cloned();
        }
        None
    }

    fn class_of_expr(&self, e: &Expr) -> Option<String> {
        let direct = match e {
            Expr::This(_) => self.current_class.clone(),
            Expr::Identifier(n, _) => self.local_types.get(n).map(|t| t.name().to_string()),
            Expr::MemberAccess(o, f, _) => self
                .class_of_expr(o)
                .and_then(|base| self.field_type(&base, f))
                .map(|t| t.name().to_string()),
            _ => None,
        };
        direct.or_else(|| match self.dtype_of(e) {
            DType::Obj(name) => Some(name),
            _ => None,
        })
    }

    fn const_expr_of(&self, e: &Expr, ty: WTy) -> Option<u64> {
        match (e, ty) {
            (Expr::Literal(Literal::Int(v), _), WTy::I64) => Some(*v as u64),
            (Expr::Literal(Literal::Int(v), _), _) => Some(*v as u64),
            (Expr::Literal(Literal::Bool(b), _), _) => Some(*b as u64),
            (Expr::Literal(Literal::Char(c), _), _) => Some(*c as u64),
            (Expr::Literal(Literal::Float(f), _), WTy::F64) => Some(f.to_bits()),
            (Expr::Literal(Literal::Float(f), _), _) => Some(*f as u64),
            _ => None,
        }
    }

    // ---- per-function state -------------------------------------------------

    fn begin_function(&mut self, f: &FnDecl) {
        self.begin_fn(
            &f.params,
            f.return_type.as_ref(),
            false,
            None,
        );
    }

    fn begin_gen(&mut self, g: &GenFn) {
        self.begin_fn(&g.params, g.return_type.as_ref(), g.handle_ret, g.this_class.clone());
        if !g.captures.is_empty() {
            self.env_local = Some(0);
            for (name, dt) in &g.captures {
                self.closure_captures.insert(name.clone(), dt.clone());
            }
            self.local_dtypes.insert(
                "__env".to_string(),
                DType::Obj("ClosureEnv".to_string()),
            );
        }
    }

    fn begin_fn(
        &mut self,
        params: &[Param],
        ret: Option<&Type>,
        handle_ret: bool,
        this_class: Option<String>,
    ) {
        self.instrs.clear();
        self.locals.clear();
        self.scopes.clear();
        self.labels.clear();
        self.loops.clear();
        self.current_class = this_class.clone();
        self.current_ret = if handle_ret {
            WTy::Handle
        } else {
            ret.map(ty_of).unwrap_or(WTy::Void)
        };
        self.current_ret_dtype = if handle_ret {
            this_class
                .as_ref()
                .map(|c| DType::Obj(c.clone()))
                .unwrap_or(DType::Unknown)
        } else {
            ret.map(|t| self.dtype_of_type(t)).unwrap_or(DType::Void)
        };
        self.current_error_label = None;
        self.env_local = None;
        self.closure_captures.clear();

        self.local_count = 0;
        self.local_map.clear();
        self.local_types.clear();
        self.local_dtypes.clear();
        for (i, p) in params.iter().enumerate() {
            let has_this = params.first().is_some_and(|p| p.name == "this");
            let is_handle = p.name == "__env" || (this_class.is_some() && has_this && i == 0);
            let ty = if is_handle {
                WTy::Handle
            } else {
                ty_of(&p.typ)
            };
            self.local_map.insert(p.name.clone(), (i as u32, ty));
            self.local_types.insert(p.name.clone(), p.typ.clone());
            let dt = if p.name == "__env" {
                DType::Obj("ClosureEnv".to_string())
            } else if this_class.is_some() && has_this && i == 0 {
                this_class
                    .as_ref()
                    .map(|c| DType::Obj(c.clone()))
                    .unwrap_or(DType::Unknown)
            } else {
                self.dtype_of_type(&p.typ)
            };
            self.local_dtypes.insert(p.name.clone(), dt);
            self.local_count += 1;
        }
    }

    fn add_local(&mut self, ty: WTy) -> u32 {
        let vt = ty.val_type().unwrap_or(ValType::I32);
        let idx = self.local_count;
        self.locals.push((1, vt));
        self.local_count += 1;
        idx
    }

    fn push_scope(&mut self) {
        self.scopes.push((
            self.local_map.clone(),
            self.local_types.clone(),
            self.local_dtypes.clone(),
        ));
    }

    fn pop_scope(&mut self) {
        if let Some(saved) = self.scopes.pop() {
            self.local_map = saved.0;
            self.local_types = saved.1;
            self.local_dtypes = saved.2;
        }
    }

    fn lookup_local(&self, name: &str) -> Option<(u32, WTy)> {
        self.local_map.get(name).copied()
    }

    // ---- control-flow label helpers ----------------------------------------

    fn label_depth(&self, label: u32) -> u32 {
        self.labels.len() as u32 - 1 - label
    }

    fn br_to(&mut self, label: u32) {
        let d = self.label_depth(label);
        self.push(wasm_encoder::Instruction::Br(d));
    }

    // ---- type conversions ---------------------------------------------------

    fn zero(&mut self, ty: WTy) {
        match ty {
            WTy::I64 => self.i64c(0),
            WTy::F64 => self.f64c(0.0),
            WTy::Bool | WTy::Char | WTy::Str | WTy::Handle => self.i32c(0),
            WTy::Void | WTy::Unknown => {}
        }
    }

    /// Convert the value on top of the stack from `from` to `to`.
    fn coerce(&mut self, from: WTy, to: WTy) -> Result<(), String> {
        use wasm_encoder::Instruction::*;
        if from == to {
            return Ok(());
        }
        match (from, to) {
            (WTy::I64, WTy::F64) => self.push(F64ConvertI64S),
            (WTy::F64, WTy::I64) => self.push(I64TruncSatF64S),
            (WTy::Bool | WTy::Char, WTy::I64) => self.push(I64ExtendI32U),
            (WTy::I64, WTy::Bool) => {
                self.i64c(0);
                self.push(I64Ne);
            }
            (WTy::I64, WTy::Char) => self.push(I32WrapI64),
            (WTy::F64, WTy::Bool) => {
                self.f64c(0.0);
                self.push(F64Ne);
            }
            (WTy::F64, WTy::Char) => {
                self.push(I64TruncSatF64S);
                self.push(I32WrapI64);
            }
            (WTy::Bool | WTy::Char, WTy::F64) => self.push(F64ConvertI32U),
            _ => return Err(format!("wasm: cannot convert {:?} to {:?}", from, to)),
        }
        Ok(())
    }

    // ---- expression inference ----------------------------------------------

    fn infer(&self, e: &Expr) -> WTy {
        match e {
            Expr::Literal(l, _) => match l {
                Literal::Int(_) => WTy::I64,
                Literal::Float(_) => WTy::F64,
                Literal::Bool(_) => WTy::Bool,
                Literal::Char(_) => WTy::Char,
                Literal::String(_) => WTy::Str,
                Literal::Null => WTy::Handle,
            },
            Expr::Identifier(n, _) => {
                if let Some((_, t)) = self.local_map.get(n) {
                    return *t;
                }
                if let Some((_, t)) = self.globals.get(n) {
                    return *t;
                }
                WTy::Unknown
            }
            Expr::Binary(l, op, r, _) => {
                let lt = self.infer(l);
                let rt = self.infer(r);
                match op {
                    Operator::Eq
                    | Operator::Ne
                    | Operator::Lt
                    | Operator::Gt
                    | Operator::Le
                    | Operator::Ge
                    | Operator::And
                    | Operator::Or => WTy::Bool,
                    Operator::Add if lt == WTy::Str || rt == WTy::Str => WTy::Str,
                    _ if lt == WTy::F64 || rt == WTy::F64 => WTy::F64,
                    _ => WTy::I64,
                }
            }
            Expr::Unary(UnOp::Not, _, _) => WTy::Bool,
            Expr::Unary(UnOp::Neg | UnOp::BitNot, x, _) => self.infer(x),
            Expr::Assign(l, _, _) => self.infer(l),
            Expr::Ternary { then_expr, .. } => self.infer(then_expr),
            Expr::Cast(_, t, _) => ty_of(t),
            Expr::Call(callee, ..) => self.infer_call(callee),
            Expr::StaticCall {
                class_name, method, ..
            } => self.infer_named(class_name, method),
            Expr::This(_) => WTy::Handle,
            Expr::MemberAccess(o, m, _) => {
                if let Expr::Identifier(module, _) = o.as_ref() {
                    let named = self.infer_named(module, m);
                    if named != WTy::Unknown {
                        return named;
                    }
                }
                if let Some(class) = self.class_of_expr(o) {
                    if let Some(t) = self.field_type(&class, m) {
                        return ty_of(&t);
                    }
                }
                WTy::Unknown
            }
            Expr::New(t, _, _) => {
                if self.classes.contains_key(t.name()) {
                    WTy::Handle
                } else {
                    ty_of(t)
                }
            }
            Expr::Unit(_) => WTy::Void,
            _ => WTy::Unknown,
        }
    }

    fn infer_call(&self, callee: &Expr) -> WTy {
        match callee {
            Expr::Identifier(n, _) => {
                if let Some(f) = self.functions.get(n) {
                    return f.ret;
                }
                match n.as_str() {
                    "println" | "print" => WTy::Void,
                    "toString" => WTy::Str,
                    "parseInt" => WTy::I64,
                    "parseDouble" => WTy::F64,
                    _ => WTy::Unknown,
                }
            }
            Expr::MemberAccess(o, m, _) => {
                if let Expr::Identifier(module, _) = o.as_ref() {
                    let named = self.infer_named(module, m);
                    if named != WTy::Unknown {
                        return named;
                    }
                }
                if let Some(class) = self.class_of_expr(o) {
                    if let Some(info) = self.classes.get(&class) {
                        if let Some(md) = info.methods.iter().find(|x| x.name == *m) {
                            return md.return_type.as_ref().map(ty_of).unwrap_or(WTy::Void);
                        }
                    }
                }
                WTy::Unknown
            }
            _ => WTy::Unknown,
        }
    }

    fn infer_named(&self, module: &str, method: &str) -> WTy {
        match (module, method) {
            (_, "toString") => WTy::Str,
            ("io", "println" | "print" | "eprintln") | ("console", "log") => WTy::Void,
            ("Integer" | "Long" | "Int", "parseInt" | "parseLong" | "parseOr") => WTy::I64,
            ("Double" | "Float", "parseDouble" | "parseFloat" | "parse") => WTy::F64,
            ("String", "length" | "indexOf" | "compareTo") => WTy::I64,
            ("String", "startsWith" | "endsWith" | "equals") => WTy::Bool,
            ("String", _) => WTy::Str,
            ("Math" | "MathAdvanced" | "MathTrig", _) => WTy::F64,
            ("File", "readFile" | "read") => WTy::Str,
            ("File", "exists") => WTy::Bool,
            ("File", _) => WTy::Void,
            ("Json", "stringify") => WTy::Str,
            ("Regex", "match" | "fullMatch") => WTy::Bool,
            ("Regex", "find" | "replace" | "findGroups" | "subN") => WTy::Str,
            ("Regex", "groupCount") => WTy::I64,
            ("Regex", "split") => WTy::Handle,
            ("Os", "name" | "arch" | "family" | "getcwd" | "getenv" | "urandom") => WTy::Str,
            ("Os", "cpuCount" | "getpid") => WTy::I64,
            ("Os", "chmod") => WTy::Handle,
            ("Os", "makedirs") => WTy::Void,
            _ => WTy::Unknown,
        }
    }
}

/// Compile a typed Titrate program into a WebAssembly module.
pub fn compile_to_wasm(program: &Program) -> Result<Vec<u8>, String> {
    let specialized = mono::monomorphize(program)?;
    let mut backend = WasmBackend::new(&specialized);
    backend.compile()
}
