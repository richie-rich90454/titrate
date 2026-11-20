//! WebAssembly code-generation backend.
//!
//! Lowers the typed Titrate AST to a standalone `.wasm` module. Arithmetic,
//! control flow and function calls are compiled to native WebAssembly
//! instructions; strings and a small set of runtime facilities (printing,
//! numeric formatting, parsing, string algorithms, math) are provided as
//! `env` imports by the host. See `docs/guide/wasm-backend.md` for the exact
//! runtime contract and the list of unsupported constructs.

pub mod runtime;
mod lower;

use std::collections::HashMap;

use wasm_encoder::{Ieee64, ValType};

use crate::ast::{
    Block, ClassDecl, Declaration, Expr, FieldDecl, FnDecl, Literal, MethodDecl, Operator, Param,
    Program, Stmt, Type, UnOp,
};

use runtime::{import_index, IMPORT_COUNT};

/// The WebAssembly representation chosen for each Titrate type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WTy {
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
    match t.name() {
        "bool" => WTy::Bool,
        "char" => WTy::Char,
        "string" => WTy::Str,
        "float" | "double" | "half" | "quad" | "f64" => WTy::F64,
        "void" => WTy::Void,
        "byte" | "short" | "int" | "long" | "vast" | "uvast" | "size" | "u8" | "u16"
        | "u32" | "u64" => WTy::I64,
        _ => WTy::Unknown,
    }
}

#[derive(Clone)]
struct FuncInfo {
    params: Vec<WTy>,
    ret: WTy,
    func_idx: u32,
    /// True for generated constructors: the WASM result is an object handle.
    #[allow(dead_code)]
    handle_ret: bool,
}

/// Saved local-variable state when entering a nested block.
type Scope = (HashMap<String, (u32, WTy)>, HashMap<String, Type>);

/// A generated function (class constructor or method) owned by the backend.
#[derive(Clone)]
struct GenFn {
    name: String,
    params: Vec<Param>,
    return_type: Option<Type>,
    body: Block,
    handle_ret: bool,
    this_class: Option<String>,
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
    local_types: HashMap<String, Type>,
    current_class: Option<String>,

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
            local_types: HashMap::new(),
            current_class: None,
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
            Expr::Call(c, args, _) => {
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
            | Expr::RegionAlloc(_, x, _) => self.collect_literals_in_expr(x),
            Expr::Closure {
                expr: Some(x), ..
            } => self.collect_literals_in_expr(x),
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
                            func_idx,
                            handle_ret: false,
                        },
                    );
                    self.func_order.push(f);
                }
                Declaration::Class(_) => {}
                Declaration::VarDecl(d) | Declaration::ConstDecl(d) => {
                    let ty = d
                        .typ
                        .as_ref()
                        .map(ty_of)
                        .or_else(|| d.init.as_ref().map(|e| self.infer(e)))
                        .unwrap_or(WTy::I64);
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
                }
                _ => {}
            }
        }
        // Class functions are appended after all user functions so function
        // indices stay in declaration order.
        for decl in &self.program.declarations {
            if let Declaration::Class(c) = decl {
                self.collect_class(c)?;
            }
        }
        Ok(())
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
            self.add_generated(&fname, ctor.params.clone(), None, ctor.body.clone(), true, Some(c.name.clone()));
        }
        for m in &methods {
            let fname = format!("{}::{}", c.name, m.name);
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
        });
    }

    fn field_type(&self, class: &str, field: &str) -> Option<Type> {
        self.classes
            .get(class)
            .and_then(|ci| ci.fields.iter().find(|f| f.name == field))
            .map(|f| f.typ.clone())
    }

    fn class_of_expr(&self, e: &Expr) -> Option<String> {
        match e {
            Expr::This(_) => self.current_class.clone(),
            Expr::Identifier(n, _) => self.local_types.get(n).map(|t| t.name().to_string()),
            Expr::MemberAccess(o, f, _) => {
                let base = self.class_of_expr(o)?;
                self.field_type(&base, f).map(|t| t.name().to_string())
            }
            _ => None,
        }
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

        self.local_count = 0;
        self.local_map.clear();
        self.local_types.clear();
        for (i, p) in params.iter().enumerate() {
            let has_this = params.first().is_some_and(|p| p.name == "this");
            let ty = if this_class.is_some() && has_this && i == 0 {
                WTy::Handle
            } else {
                ty_of(&p.typ)
            };
            self.local_map.insert(p.name.clone(), (i as u32, ty));
            self.local_types.insert(p.name.clone(), p.typ.clone());
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
        self.scopes
            .push((self.local_map.clone(), self.local_types.clone()));
    }

    fn pop_scope(&mut self) {
        if let Some(saved) = self.scopes.pop() {
            self.local_map = saved.0;
            self.local_types = saved.1;
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
                Literal::Null => WTy::Str,
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
            Expr::Call(callee, _, _) => self.infer_call(callee),
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
            _ => WTy::Unknown,
        }
    }
}

/// Compile a typed Titrate program into a WebAssembly module.
pub fn compile_to_wasm(program: &Program) -> Result<Vec<u8>, String> {
    let mut backend = WasmBackend::new(program);
    backend.compile()
}
