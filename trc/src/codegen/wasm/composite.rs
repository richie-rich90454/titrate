//! Lowering for composite values: collections, tuples, enums, Result and
//! the operations on them. All composite values are host object handles.

use std::collections::HashMap;

use super::dtypes::DType;
use super::{CtrlLabel, LoopCtx, WasmBackend, WTy};
use crate::ast::{Expr, ForStmt, Stmt};
use wasm_encoder::{BlockType, Instruction::*};

fn setter_for(wty: WTy) -> &'static str {
    match wty {
        WTy::F64 => "field_set_f64",
        WTy::Bool => "field_set_bool",
        WTy::Str => "field_set_str",
        _ => "field_set_i64",
    }
}

#[derive(Clone)]
pub(crate) struct ClosureMeta {
    pub(crate) slot: u32,
    pub(crate) captures: Vec<(String, DType)>,
}

struct ClosureParts {
    params: Vec<(String, crate::ast::Type)>,
    return_type: crate::ast::Type,
    body: Vec<Stmt>,
    expr: Option<Box<Expr>>,
    captured: Vec<String>,
}

impl<'a> WasmBackend<'a> {
    /// Compile an expression that constructs a collection instance.
    pub(crate) fn compile_collection_new(&mut self, class: &str) -> Option<Result<WTy, String>> {
        match class {
            "ArrayList" => {
                let r = self.call_host("list_new");
                Some(r.map(|_| WTy::Handle))
            }
            "HashMap" => {
                let r = self.call_host("map_new");
                Some(r.map(|_| WTy::Handle))
            }
            _ => None,
        }
    }

    /// Compile a call whose callee is a member access on a composite value.
    /// Returns `None` when the receiver is not a known composite.
    pub(crate) fn compile_composite_call(
        &mut self,
        obj: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Option<Result<WTy, String>> {
        let dt = self.dtype_of(obj);
        let result = match &dt {
            DType::List(elem) => self.compile_list_call(obj, method, args, elem),
            DType::Map(key, val) => self.compile_map_call(obj, method, args, key, val),
            DType::ResultT(ok, _) => self.compile_result_call(obj, method, args, ok),
            DType::Enum(_) if method == "toString" => {
                let r = self.compile_expr(obj).and_then(|_| self.call_host("enum_display"));
                r.map(|_| WTy::Str)
            }
            DType::Tuple(_) if method == "toString" => {
                let r = self.compile_expr(obj).and_then(|_| self.call_host("tuple_display"));
                r.map(|_| WTy::Str)
            }
            _ => return None,
        };
        Some(result)
    }

    fn compile_list_call(
        &mut self,
        obj: &Expr,
        method: &str,
        args: &[Expr],
        elem: &DType,
    ) -> Result<WTy, String> {
        let suffix = elem.suffix();
        match method {
            "add" => {
                self.compile_expr(obj)?;
                if args.is_empty() {
                    return Err("wasm: ArrayList.add expects a value".to_string());
                }
                self.push_boxed_value(&args[0])?;
                self.call_host("list_add")?;
                Ok(WTy::Void)
            }
            "set" => {
                self.compile_expr(obj)?;
                self.compile_index_expr(&args[0])?;
                self.push_boxed_value(&args[1])?;
                self.call_host("list_set")?;
                Ok(WTy::Void)
            }
            "get" | "remove" | "pop" => {
                self.compile_expr(obj)?;
                self.compile_index_expr(&args[0])?;
                self.call_host(&format!("list_get_{}", suffix))?;
                Ok(elem.wty())
            }
            "size" | "length" => {
                self.compile_expr(obj)?;
                self.call_host("list_size")?;
                Ok(WTy::I64)
            }
            "contains" => {
                self.compile_expr(obj)?;
                self.push_boxed_value(&args[0])?;
                self.call_host("list_contains")?;
                Ok(WTy::Bool)
            }
            "indexOf" | "lastIndexOf" => {
                self.compile_expr(obj)?;
                self.push_boxed_value(&args[0])?;
                self.call_host("list_index_of")?;
                Ok(WTy::I64)
            }
            "toString" => {
                self.compile_expr(obj)?;
                self.call_host("list_display")?;
                Ok(WTy::Str)
            }
            "clear" => {
                self.compile_expr(obj)?;
                self.call_host("list_clear")?;
                Ok(WTy::Void)
            }
            "isEmpty" => {
                self.compile_expr(obj)?;
                self.call_host("list_is_empty")?;
                Ok(WTy::Bool)
            }
            _ => Err(format!("wasm: unsupported ArrayList method '{}'", method)),
        }
    }

    fn compile_map_call(
        &mut self,
        obj: &Expr,
        method: &str,
        args: &[Expr],
        key: &DType,
        val: &DType,
    ) -> Result<WTy, String> {
        let ks = if key == &DType::I64 { "i64" } else { "str" };
        match method {
            "put" => {
                self.compile_expr(obj)?;
                self.compile_map_key(&args[0], key)?;
                self.push_boxed_value(&args[1])?;
                self.call_host(&format!("map_put_{}", ks))?;
                Ok(WTy::Void)
            }
            "get" | "remove" | "pop" => {
                self.compile_expr(obj)?;
                self.compile_map_key(&args[0], key)?;
                self.call_host(&format!(
                    "map_get_{}_{}",
                    ks,
                    val.suffix()
                ))?;
                Ok(val.wty())
            }
            "size" => {
                self.compile_expr(obj)?;
                self.call_host("map_size")?;
                Ok(WTy::I64)
            }
            "containsKey" => {
                self.compile_expr(obj)?;
                self.compile_map_key(&args[0], key)?;
                self.call_host(&format!("map_contains_{}", ks))?;
                Ok(WTy::Bool)
            }
            "toString" => {
                self.compile_expr(obj)?;
                self.call_host("map_display")?;
                Ok(WTy::Str)
            }
            "keys" | "values" => {
                self.compile_expr(obj)?;
                self.call_host(if method == "keys" { "map_keys" } else { "map_values" })?;
                Ok(WTy::Handle)
            }
            _ => Err(format!("wasm: unsupported HashMap method '{}'", method)),
        }
    }

    fn compile_result_call(
        &mut self,
        obj: &Expr,
        method: &str,
        args: &[Expr],
        ok: &DType,
    ) -> Result<WTy, String> {
        let _ = args;
        match method {
            "unwrap" => {
                self.compile_expr(obj)?;
                self.call_host(&format!("result_unwrap_{}", ok.suffix()))?;
                Ok(ok.wty())
            }
            "isOk" => {
                self.compile_expr(obj)?;
                self.call_host("result_is_ok")?;
                Ok(WTy::Bool)
            }
            "isErr" => {
                self.compile_expr(obj)?;
                self.call_host("result_is_ok")?;
                self.push(I32Eqz);
                Ok(WTy::Bool)
            }
            _ => Err(format!("wasm: unsupported Result method '{}'", method)),
        }
    }

    fn compile_index_expr(&mut self, e: &Expr) -> Result<(), String> {
        let t = self.compile_expr(e)?;
        self.coerce(t, WTy::I64)
    }

    fn compile_map_key(&mut self, e: &Expr, key: &DType) -> Result<(), String> {
        let t = self.compile_expr(e)?;
        if key == &DType::I64 {
            self.coerce(t, WTy::I64)
        } else {
            self.coerce(t, WTy::Str)
        }
    }

    /// Compile `Ok(expr)` / `Err(expr)` and enum-variant constructor calls.
    pub(crate) fn compile_tagged_call(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Option<Result<WTy, String>> {
        if name == "Ok" || name == "Err" {
            return Some((|| {
                self.i32c(if name == "Ok" { 1 } else { 0 });
                if args.is_empty() {
                    self.i32c(0);
                    self.i64c(0);
                    self.f64c(0.0);
                    self.i32c(0);
                } else {
                    self.push_boxed_value(&args[0])?;
                }
                self.call_host("result_new")?;
                Ok(WTy::Handle)
            })());
        }
        if let Some(ename) = self.variant_enum.get(name).cloned() {
            return Some((|| {
                let noff = self.intern_string(&ename);
                let voff = self.intern_string(name);
                self.i32c(noff);
                self.i32c(voff);
                self.i64c(args.len() as i64);
                self.call_host("enum_new")?;
                let h = self.add_local(WTy::Handle);
                self.push(LocalSet(h));
                for (i, a) in args.iter().enumerate() {
                    self.push(LocalGet(h));
                    self.i64c(i as i64);
                    self.push_boxed_value(a)?;
                    self.call_host("enum_set")?;
                }
                self.push(LocalGet(h));
                Ok(WTy::Handle)
            })());
        }
        None
    }

    /// Compile tuple construction.
    pub(crate) fn compile_tuple(&mut self, elems: &[Expr]) -> Result<WTy, String> {
        self.i64c(elems.len() as i64);
        self.call_host("tuple_new")?;
        let h = self.add_local(WTy::Handle);
        self.push(LocalSet(h));
        for (i, e) in elems.iter().enumerate() {
            self.push(LocalGet(h));
            self.i64c(i as i64);
            self.push_boxed_value(e)?;
            self.call_host("tuple_set")?;
        }
        self.push(LocalGet(h));
        Ok(WTy::Handle)
    }

    /// Compile indexing into a list, map or tuple.
    pub(crate) fn compile_index(&mut self, obj: &Expr, idx: &Expr) -> Result<WTy, String> {
        let dt = self.dtype_of(obj);
        let suffix = match &dt {
            DType::List(e) => e.suffix(),
            DType::Map(_, v) => v.suffix(),
            DType::Tuple(elems) => elems.first().map(|x| x.suffix()).unwrap_or("i64"),
            _ => return Err("wasm: index requires a collection or tuple".to_string()),
        };
        self.compile_expr(obj)?;
        self.compile_index_expr(idx)?;
        let host = match dt {
            DType::List(_) => format!("list_get_{}", suffix),
            DType::Tuple(_) => format!("tuple_get_{}", suffix),
            DType::Map(_, _) => {
                let ks = "i64";
                format!("map_get_{}_{}", ks, suffix)
            }
            _ => unreachable!(),
        };
        self.call_host(&host)?;
        Ok(match dt {
            DType::List(e) => e.wty(),
            DType::Map(_, v) => v.wty(),
            DType::Tuple(elems) => elems.first().map(|x| x.wty()).unwrap_or(WTy::I64),
            _ => WTy::I64,
        })
    }

    /// Compile the logical-type check `value is Type`.
    pub(crate) fn compile_is(&mut self, expr: &Expr, typ: &crate::ast::Type) -> Result<WTy, String> {
        let dt = self.dtype_of(expr);
        let name = typ.name().to_string();
        let is_iface = self.iface_impls.contains_key(&name);
        self.compile_expr(expr)?;
        let h = self.add_local(WTy::Handle);
        self.push(LocalSet(h));
        if is_iface {
            let off = self.intern_string(&format!("__iface_{}", name));
            self.push(LocalGet(h));
            self.i32c(off);
            self.call_host("field_get_bool")?;
        } else {
            self.push(LocalGet(h));
            self.call_host("obj_class")?;
            let off = self.intern_string(&name);
            self.i32c(off);
            self.call_host("str_eq")?;
        }
        let _ = dt;
        Ok(WTy::Bool)
    }

    /// Store dispatch slots on a freshly constructed object handle, which is
    /// expected on top of the stack. Leaves the handle on the stack.
    pub(crate) fn set_vtbl_fields(&mut self, class: &str) -> Result<(), String> {
        let methods: Vec<String> = self
            .all_class_methods
            .get(class)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        if methods.is_empty() && self.iface_impls.values().all(|v| !v.iter().any(|c| c == class)) {
            return Ok(());
        }
        let h = self.add_local(WTy::Handle);
        self.push(LocalSet(h));
        for method in methods {
            if let Some(slot) = self
                .method_slots
                .get(&(class.to_string(), method.clone()))
                .copied()
            {
                let off = self.intern_string(&format!("__vtbl_{}", method));
                self.push(LocalGet(h));
                self.i32c(off);
                self.i64c(slot as i64);
                self.call_host("field_set_i64")?;
            }
        }
        let ifaces: Vec<String> = self.iface_impls.keys().cloned().collect();
        for iface in ifaces {
            if self.iface_impls[&iface].iter().any(|c| c == class) {
                let off = self.intern_string(&format!("__iface_{}", iface));
                self.push(LocalGet(h));
                self.i32c(off);
                self.i32c(1);
                self.call_host("field_set_bool")?;
            }
        }
        self.push(LocalGet(h));
        Ok(())
    }

    fn closure_sig(key: &[WTy], ret: WTy) -> String {
        format!("{:?}|{:?}", key, ret)
    }

    /// Pre-pass: register every closure literal so generated functions and
    /// dispatch tables are known before the function/element sections are
    /// emitted.
    pub(crate) fn collect_closures(&mut self) {
        use crate::ast::{ClassMember, Declaration};
        let decls = self.program.declarations.clone();
        let mut types = HashMap::new();
        for d in &decls {
            match d {
                Declaration::Function(f) => {
                    types.clear();
                    for p in &f.params {
                        types.insert(p.name.clone(), self.dtype_of_type(&p.typ));
                    }
                    self.scan_block(&f.body, &mut types);
                }
                Declaration::Class(c) => {
                    for m in &c.members {
                        if let ClassMember::Method(md) | ClassMember::Constructor(md) = m {
                            let mut t = HashMap::new();
                            for p in &md.params {
                                t.insert(p.name.clone(), self.dtype_of_type(&p.typ));
                            }
                            self.scan_block(&md.body, &mut t);
                        }
                    }
                }
                Declaration::Interface(i) => {
                    for m in &i.methods {
                        if let Some(b) = &m.body {
                            self.scan_block(b, &mut HashMap::new());
                        }
                    }
                }
                Declaration::VarDecl(v) | Declaration::ConstDecl(v) => {
                    if let Some(e) = &v.init {
                        self.scan_expr(e, &mut types);
                    }
                }
                _ => {}
            }
        }
    }

    fn scan_block(&mut self, block: &[Stmt], types: &mut HashMap<String, DType>) {
        for s in block {
            self.scan_stmt(s, types);
        }
    }

    fn scan_stmt(&mut self, s: &Stmt, types: &mut HashMap<String, DType>) {
        use crate::ast::Stmt;
        match s {
            Stmt::Expr(e) | Stmt::Throw(e, _) => self.scan_expr(e, types),
            Stmt::VarDecl(v) | Stmt::ConstDecl(v) => {
                if let Some(e) = &v.init {
                    self.scan_expr(e, types);
                }
                let dt = v
                    .typ
                    .as_ref()
                    .map(|t| self.dtype_of_type(t))
                    .or_else(|| v.init.as_ref().map(|e| self.dtype_of(e)))
                    .unwrap_or(DType::I64);
                types.insert(v.name.clone(), dt);
            }
            Stmt::Block(b) => self.scan_block(b, types),
            Stmt::If(i) => {
                self.scan_expr(&i.condition, types);
                self.scan_block(&i.then_branch, types);
                if let Some(e) = &i.else_branch {
                    self.scan_block(e, types);
                }
            }
            Stmt::While(w) => {
                self.scan_expr(&w.condition, types);
                self.scan_block(&w.body, types);
            }
            Stmt::DoWhile(d) => {
                self.scan_expr(&d.condition, types);
                self.scan_block(&d.body, types);
            }
            Stmt::For(f) => {
                self.scan_expr(&f.iterable, types);
                self.scan_block(&f.body, types);
            }
            Stmt::CFor(c) => {
                if let Some(i) = &c.init {
                    self.scan_stmt(i, types);
                }
                if let Some(x) = &c.condition {
                    self.scan_expr(x, types);
                }
                if let Some(x) = &c.increment {
                    self.scan_expr(x, types);
                }
                self.scan_block(&c.body, types);
            }
            Stmt::Return(Some(e)) => self.scan_expr(e, types),
            Stmt::Switch(sw) => {
                self.scan_expr(&sw.expr, types);
                for c in &sw.cases {
                    self.scan_block(&c.body, types);
                }
                if let Some(d) = &sw.default {
                    self.scan_block(d, types);
                }
            }
            Stmt::TryCatch {
                try_block, catch_block, ..
            } => {
                self.scan_block(try_block, types);
                self.scan_block(catch_block, types);
            }
            Stmt::TupleDestructure { expr, .. } => self.scan_expr(expr, types),
            _ => {}
        }
    }

    fn scan_expr(&mut self, e: &Expr, types: &mut HashMap<String, DType>) {
        use crate::ast::Literal;
        match e {
            Expr::Literal(Literal::String(_), _) => {}
            Expr::Binary(a, _, b, _) => {
                self.scan_expr(a, types);
                self.scan_expr(b, types);
            }
            Expr::Unary(_, a, _) | Expr::Cast(a, _, _) | Expr::Is(a, _, _) => self.scan_expr(a, types),
            Expr::Call(c, args, _) => {
                self.scan_expr(c, types);
                for a in args {
                    self.scan_expr(a, types);
                }
            }
            Expr::StaticCall { args, .. } | Expr::New(_, args, _) => {
                for a in args {
                    self.scan_expr(a, types);
                }
            }
            Expr::Assign(a, b, _) => {
                self.scan_expr(a, types);
                self.scan_expr(b, types);
            }
            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                self.scan_expr(condition, types);
                self.scan_expr(then_expr, types);
                self.scan_expr(else_expr, types);
            }
            Expr::Index(o, i, _) => {
                self.scan_expr(o, types);
                self.scan_expr(i, types);
            }
            Expr::MemberAccess(o, _, _) => self.scan_expr(o, types),
            Expr::Tuple(xs, _) => {
                for x in xs {
                    self.scan_expr(x, types);
                }
            }
            Expr::ErrorPropagation(x, _) => self.scan_expr(x, types),
            Expr::Closure {
                params,
                return_type,
                body,
                expr,
                captured_vars,
                span,
            } => {
                for x in body {
                    self.scan_stmt(x, types);
                }
                if let Some(x) = expr {
                    self.scan_expr(x, types);
                }
                let parts = ClosureParts {
                    params: params.clone(),
                    return_type: return_type.clone(),
                    body: body.clone(),
                    expr: expr.clone(),
                    captured: captured_vars.clone(),
                };
                self.register_closure(&parts, types, *span);
            }
            _ => {}
        }
    }

    fn register_closure(
        &mut self,
        parts: &ClosureParts,
        types: &HashMap<String, DType>,
        span: crate::ast::Span,
    ) {
        use super::{GenFn, Param};
        let ClosureParts { params, return_type, body, expr, captured } = parts;
        let mut ptypes: Vec<WTy> = Vec::new();
        let mut plist: Vec<Param> = Vec::new();
        plist.push(Param {
            name: "__env".to_string(),
            typ: crate::ast::Type::simple("Handle"),
        });
        for (n, t) in params {
            ptypes.push(self.dtype_of_type(t).wty());
            plist.push(Param { name: n.clone(), typ: t.clone() });
        }
        let ret = self.dtype_of_type(return_type).wty();
        let key = Self::closure_sig(&ptypes, ret);
        let (table, type_idx) = if let Some(&v) = self.closure_tables.get(&key) {
            v
        } else {
            let t = self.next_table;
            self.next_table += 1;
            self.table_elems.insert(t, Vec::new());
            let v = (t, 0u32);
            self.closure_tables.insert(key.clone(), v);
            v
        };
        let mut fn_body: Vec<Stmt> = body.to_vec();
        if let Some(e) = expr {
            fn_body.push(Stmt::Return(Some((**e).clone())));
        }
        let name = format!("$closure_{}", self.closure_counter);
        self.closure_counter += 1;
        let captures: Vec<(String, DType)> = captured
            .iter()
            .map(|n| (n.clone(), types.get(n).cloned().unwrap_or(DType::Unknown)))
            .collect();
        let func_idx = super::IMPORT_COUNT + self.func_order.len() as u32 + self.gen_funcs.len() as u32;
        self.functions.insert(
            name.clone(),
            super::FuncInfo {
                params: {
                    let mut v = vec![WTy::Handle];
                    v.extend(ptypes.iter().copied());
                    v
                },
                ret,
                ret_dtype: self.dtype_of_type(return_type),
                func_idx,
                handle_ret: false,
            },
        );
        self.gen_funcs.push(GenFn {
            name,
            params: plist,
            return_type: Some(return_type.clone()),
            body: fn_body,
            handle_ret: false,
            this_class: None,
            captures: captures.clone(),
        });
        let elems = self.table_elems.get_mut(&table).unwrap();
        let slot = elems.len() as u32;
        elems.push(func_idx);
        if type_idx == 0 {
            self.closure_tables.insert(key, (table, func_idx));
        }
        self.closure_meta.insert(
            (span.line, span.column),
            ClosureMeta { slot, captures },
        );
    }

    /// Emit the host object creation for a previously registered closure.
    pub(crate) fn compile_closure(
        &mut self,
        span: crate::ast::Span,
        captured: &[String],
    ) -> Result<WTy, String> {
        let meta = self
            .closure_meta
            .get(&(span.line, span.column))
            .cloned()
            .ok_or_else(|| "wasm: closure was not registered".to_string())?;
        let _ = captured;
        self.i64c(meta.slot as i64);
        self.call_host("closure_new")?;
        let cl = self.add_local(WTy::Handle);
        self.push(LocalSet(cl));
        for (n, _) in &meta.captures {
            let (idx, wty) = self
                .lookup_local(n)
                .ok_or_else(|| format!("wasm: cannot capture unknown variable '{}'", n))?;
            let off = self.intern_string(&format!("__cap_{}", n));
            self.push(LocalGet(cl));
            self.i32c(off);
            self.push(LocalGet(idx));
            self.call_host(setter_for(wty))?;
        }
        self.push(LocalGet(cl));
        Ok(WTy::Handle)
    }

    /// Indirect call through a closure value.
    pub(crate) fn compile_closure_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        ptypes: &[DType],
        ret: &DType,
    ) -> Result<WTy, String> {
        use wasm_encoder::Instruction::CallIndirect;
        let wtypes: Vec<WTy> = ptypes.iter().map(|d| d.wty()).collect();
        let key = Self::closure_sig(&wtypes, ret.wty());
        let (table, type_idx) = *self
            .closure_tables
            .get(&key)
            .ok_or_else(|| format!("wasm: unknown closure signature '{}'", key))?;
        self.compile_expr(callee)?;
        let cl = self.add_local(WTy::Handle);
        self.push(LocalSet(cl));
        self.push(LocalGet(cl));
        if args.len() != ptypes.len() {
            return Err(format!(
                "wasm: closure expects {} args, got {}",
                ptypes.len(),
                args.len()
            ));
        }
        for (a, p) in args.iter().zip(ptypes) {
            let from = self.compile_expr(a)?;
            self.coerce(from, p.wty())?;
        }
        let code_off = self.intern_string("__code");
        self.push(LocalGet(cl));
        self.i32c(code_off);
        self.call_host("field_get_i64")?;
        self.push(I32WrapI64);
        self.push(CallIndirect {
            type_index: type_idx,
            table_index: table,
        });
        Ok(ret.wty())
    }

    /// Lower `throw expr` as "store error value, branch to nearest try exit".
    pub(crate) fn compile_throw(&mut self, e: &Expr) -> Result<(), String> {
        let label = self
            .current_error_label
            .ok_or_else(|| "wasm: throw outside try/catch".to_string())?;
        self.push_boxed_value(e)?;
        self.call_host("err_set")?;
        self.br_to(label);
        Ok(())
    }

    /// Lower try/catch with a host-side error cell.
    pub(crate) fn compile_try_catch(
        &mut self,
        try_block: &[Stmt],
        catch_var: &str,
        catch_var_type: Option<&crate::ast::Type>,
        catch_block: &[Stmt],
    ) -> Result<(), String> {
        use wasm_encoder::BlockType;
        self.call_host("err_clear")?;
        let after = self.labels.len() as u32;
        self.labels.push(CtrlLabel::Block);
        self.push(Block(BlockType::Empty));

        let try_label = self.labels.len() as u32;
        self.labels.push(CtrlLabel::Block);
        self.push(Block(BlockType::Empty));
        let saved = self.current_error_label;
        self.current_error_label = Some(try_label);
        self.compile_block(try_block)?;
        self.current_error_label = saved;
        self.push(End);
        self.labels.pop();

        self.call_host("err_flag")?;
        self.push(I32Eqz);
        let d = self.label_depth(after);
        self.push(BrIf(d));

        let cdt = catch_var_type
            .map(|t| self.dtype_of_type(t))
            .unwrap_or(DType::Str);
        self.call_host(&format!("err_get_{}", cdt.suffix()))?;
        let ty = cdt.wty();
        let idx = self.add_local(if ty == WTy::Unknown { WTy::I64 } else { ty });
        self.push(LocalSet(idx));
        self.push_scope();
        self.local_map.insert(catch_var.to_string(), (idx, ty));
        self.local_dtypes.insert(catch_var.to_string(), cdt);
        self.compile_block(catch_block)?;
        self.pop_scope();

        self.push(End);
        self.labels.pop();
        Ok(())
    }

    /// Lower the `?` operator: unwrap `Ok`, return the `Err` from this function.
    pub(crate) fn compile_error_propagation(&mut self, inner: &Expr) -> Result<WTy, String> {
        use wasm_encoder::BlockType;
        let dt = self.dtype_of(inner);
        let (ok, is_result) = match &dt {
            DType::ResultT(ok, _) => ((**ok).clone(), true),
            _ => (DType::Unknown, false),
        };
        if !is_result || self.current_ret != WTy::Handle {
            return Err("wasm: '?' requires a Result-valued expression in a Result-returning function".to_string());
        }
        self.compile_expr(inner)?;
        let h = self.add_local(WTy::Handle);
        self.push(LocalSet(h));
        self.push(LocalGet(h));
        self.call_host("result_is_ok")?;
        let bt = if ok == DType::Void {
            BlockType::Empty
        } else {
            BlockType::Result(ok.wty().val_type().unwrap())
        };
        self.labels.push(CtrlLabel::Block);
        self.push(If(bt));
        if ok != DType::Void {
            self.push(LocalGet(h));
            self.call_host(&format!("result_unwrap_{}", ok.suffix()))?;
        }
        self.push(Else);
        self.push(LocalGet(h));
        self.push(Return);
        self.push(End);
        self.labels.pop();
        Ok(ok.wty())
    }

    /// Lower `for (item in collection)` to an indexed WASM loop.
    pub(crate) fn compile_for_in(&mut self, f: &ForStmt) -> Result<(), String> {
        let dt = self.dtype_of(&f.iterable);
        let (elem, source) = match &dt {
            DType::List(e) => ((**e).clone(), None),
            DType::Map(k, _) => ((**k).clone(), Some("map_keys")),
            DType::Str => (DType::Char, None),
            _ => return Err("wasm: for-in requires a collection or string".to_string()),
        };
        let ht = if source.is_some() { WTy::Handle } else { dt.wty() };
        self.compile_expr(&f.iterable)?;
        if source.is_some() {
            self.call_host("map_keys")?;
        }
        let h = self.add_local(ht);
        self.push(LocalSet(h));

        if dt == DType::Str {
            self.push(LocalGet(h));
            self.call_host("str_len")?;
            self.push(I64ExtendI32U);
        } else {
            self.push(LocalGet(h));
            self.call_host("list_size")?;
        }
        let n = self.add_local(WTy::I64);
        self.push(LocalSet(n));
        let i = self.add_local(WTy::I64);
        self.i64c(0);
        self.push(LocalSet(i));

        let exit = self.labels.len() as u32;
        self.labels.push(CtrlLabel::Block);
        self.push(Block(BlockType::Empty));
        let head = self.labels.len() as u32;
        self.labels.push(CtrlLabel::LoopHead);
        self.push(Loop(BlockType::Empty));
        self.loops.push(LoopCtx {
            exit_label: exit,
            head_label: head,
        });

        self.push(LocalGet(i));
        self.push(LocalGet(n));
        self.push(I64GeS);
        let d = self.label_depth(exit);
        self.push(BrIf(d));

        let elem_ty = elem.wty();
        let elem_idx = if dt == DType::Str { n } else { h };
        if dt == DType::Str {
            self.push(LocalGet(elem_idx));
            self.push(LocalGet(i));
            self.push(I32WrapI64);
            self.call_host("str_char_at")?;
        } else {
            self.push(LocalGet(elem_idx));
            self.push(LocalGet(i));
            self.call_host(&format!("list_get_{}", elem.suffix()))?;
        }
        let elem_local = self.add_local(elem_ty);
        self.push(LocalSet(elem_local));

        self.push_scope();
        self.local_map
            .insert(f.var.clone(), (elem_local, elem_ty));
        self.local_dtypes.insert(f.var.clone(), elem.clone());
        self.compile_block(&f.body)?;
        self.pop_scope();

        self.push(LocalGet(i));
        self.i64c(1);
        self.push(I64Add);
        self.push(LocalSet(i));
        self.br_to(head);
        self.push(End);
        self.push(End);
        self.labels.pop();
        self.labels.pop();
        self.loops.pop();
        Ok(())
    }

    /// Lower tuple destructuring `let (a, b) = expr`.
    pub(crate) fn compile_tuple_destructure(
        &mut self,
        names: &[String],
        expr: &Expr,
    ) -> Result<(), String> {
        let dt = self.dtype_of(expr);
        let element_types: Vec<DType> = match &dt {
            DType::Tuple(ts) => ts.clone(),
            _ => vec![DType::Unknown; names.len()],
        };
        self.compile_expr(expr)?;
        let h = self.add_local(WTy::Handle);
        self.push(LocalSet(h));
        for (i, name) in names.iter().enumerate() {
            let edt = element_types.get(i).cloned().unwrap_or(DType::Unknown);
            let suffix = edt.suffix();
            self.push(LocalGet(h));
            self.i64c(i as i64);
            self.call_host(&format!("tuple_get_{}", suffix))?;
            let ty = edt.wty();
            let idx = self.add_local(if ty == WTy::Unknown { WTy::I64 } else { ty });
            self.push(LocalSet(idx));
            self.local_map.insert(name.clone(), (idx, ty));
            self.local_types.insert(name.clone(), crate::ast::Type::simple(""));
            self.local_dtypes.insert(name.clone(), edt);
        }
        Ok(())
    }
}
