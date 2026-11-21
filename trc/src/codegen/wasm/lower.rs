//! AST lowering: statements and expressions to WebAssembly instructions.

use super::dtypes::DType;
use super::json::JsonLit;
use super::runtime;
use super::{ty_of, CtrlLabel, FuncInfo, LoopCtx, WasmBackend, WTy};
use crate::ast::{Expr, Literal, Operator, Pattern, Stmt, UnOp};
use std::borrow::Cow;

use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, DataSection, ElementSection, Elements, EntityType,
    ExportSection, FuncType, Function, FunctionSection, GlobalSection, GlobalType, Ieee64,
    ImportSection, Instruction, MemorySection, MemoryType, Module, RefType, TableSection, TableType,
    TypeSection, ValType,
};

fn field_setter(ty: WTy) -> &'static str {
    match ty {
        WTy::F64 => "field_set_f64",
        WTy::Bool => "field_set_bool",
        WTy::Str => "field_set_str",
        _ => "field_set_i64",
    }
}

fn field_getter(ty: WTy) -> &'static str {
    match ty {
        WTy::F64 => "field_get_f64",
        WTy::Bool => "field_get_bool",
        WTy::Str => "field_get_str",
        _ => "field_get_i64",
    }
}

impl<'a> WasmBackend<'a> {
    pub(crate) fn compile(&mut self) -> Result<Vec<u8>, String> {
        // 1. Intern string literals so data offsets are known up front.
        for decl in &self.program.declarations {
            match decl {
                crate::ast::Declaration::Function(f) => self.collect_literals_in_block(&f.body),
                crate::ast::Declaration::VarDecl(d) | crate::ast::Declaration::ConstDecl(d) => {
                    if let Some(i) = &d.init {
                        self.collect_literals_in_expr(i);
                    }
                }
                crate::ast::Declaration::Class(c) => {
                    use crate::ast::ClassMember;
                    for m in &c.members {
                        match m {
                            ClassMember::Method(md) | ClassMember::Constructor(md) => {
                                self.collect_literals_in_block(&md.body)
                            }
                            ClassMember::Field(f) => {
                                if let Some(i) = &f.init {
                                    self.collect_literals_in_expr(i);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        // Literals live at low addresses; start the dynamic string heap well
        // above them so later interning (e.g. field names) can never overlap.
        self.heap_start = 65_536;

        // 2. Collect function signatures and top-level globals.
        self.collect_decls()?;
        // Closures generate extra functions; discover them before the type and
        // function sections are built.
        self.collect_closures();

        if !self.functions.contains_key("main") {
            return Err("wasm: no entry point: file defines no 'main' function".to_string());
        }

        // 3. Type section: imports first, then defined functions.
        let mut types = TypeSection::new();
        for (_, params, result) in runtime::IMPORTS {
            types
                .ty()
                .func_type(&FuncType::new(params.iter().copied(), *result));
        }
        for name in self
            .func_order
            .iter()
            .map(|f| f.name.clone())
            .chain(self.gen_funcs.iter().map(|g| g.name.clone()))
        {
            let info: &FuncInfo = &self.functions[&name];
            types.ty().func_type(&FuncType::new(
                info.params.iter().filter_map(|t| t.val_type()),
                info.ret.val_type(),
            ));
        }

        // 4. Import section.
        let mut imports = ImportSection::new();
        for (i, (name, _, _)) in runtime::IMPORTS.iter().enumerate() {
            imports.import("env", name, EntityType::Function(i as u32));
        }

        // 5. Function section (type index = import count + definition index).
        let mut funcsec = FunctionSection::new();
        for i in 0..(self.func_order.len() + self.gen_funcs.len()) {
            funcsec.function(runtime::IMPORT_COUNT + i as u32);
        }

        // 6. Memory and globals.
        let mut memsec = MemorySection::new();
        memsec.memory(MemoryType {
            minimum: 256,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        let mut globalsec = GlobalSection::new();
        globalsec.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &ConstExpr::i32_const(self.heap_start),
        );
        for (vt, init) in &self.global_defs {
            let expr = match vt {
                ValType::F64 => ConstExpr::f64_const(Ieee64::new(*init)),
                ValType::I64 => ConstExpr::i64_const(*init as i64),
                _ => ConstExpr::i32_const(*init as i32),
            };
            globalsec.global(
                GlobalType {
                    val_type: *vt,
                    mutable: true,
                    shared: false,
                },
                &expr,
            );
        }

        // 7. Exports.
        let mut exports = ExportSection::new();
        exports.export("memory", wasm_encoder::ExportKind::Memory, 0);
        exports.export("heap_ptr", wasm_encoder::ExportKind::Global, 0);
        let main_idx = self.functions["main"].func_idx;
        exports.export("main", wasm_encoder::ExportKind::Func, main_idx);

        // 8. Code section.
        let mut code = CodeSection::new();
        let order = self.func_order.to_vec();
        for f in order {
            self.begin_function(f);
            self.compile_block(&f.body)?;
            self.emit_current_function(&mut code)?;
        }
        let gens: Vec<_> = self.gen_funcs.clone();
        for g in &gens {
            self.begin_gen(g);
            if g.handle_ret {
                // `this` is parameter 0, allocated by the caller.
                let this_idx = self.local_map["this"].0;
                let init_fields: Vec<(String, WTy)> = g
                    .this_class
                    .as_ref()
                    .and_then(|cls| self.classes.get(cls))
                    .map(|info| {
                        info.fields
                            .iter()
                            .map(|f| (f.name.clone(), ty_of(&f.typ)))
                            .collect()
                    })
                    .unwrap_or_default();
                for (fname, fty) in init_fields {
                    if let Some(pos) = g.params.iter().position(|p| p.name == fname) {
                        self.push(Instruction::LocalGet(this_idx));
                        let off = self.intern_string(&fname);
                        self.i32c(off);
                        self.push(Instruction::LocalGet(pos as u32));
                        self.call_host(field_setter(fty))?;
                    }
                }
            }
            self.compile_block(&g.body)?;
            if g.handle_ret {
                // Return the freshly created handle.
                let this_idx = self.local_map["this"].0;
                self.push(Instruction::LocalGet(this_idx));
            }
            if !g.handle_ret && self.current_ret != WTy::Void {
                self.zero(self.current_ret);
            }
            self.push(Instruction::End);
            let mut func = Function::new(self.locals.iter().copied());
            for ins in &self.instrs {
                func.instruction(ins);
            }
            code.function(&func);
        }

        // 9. Tables and dispatch elements (closures and virtual methods).
        let mut tables = TableSection::new();
        let mut elements = ElementSection::new();
        let mut table_ids: Vec<u32> = self.table_elems.keys().copied().collect();
        table_ids.sort_unstable();
        for tid in &table_ids {
            let elems = &self.table_elems[tid];
            tables.table(TableType {
                element_type: RefType::FUNCREF,
                minimum: elems.len() as u64,
                maximum: None,
                table64: false,
                shared: false,
            });
        }
        for tid in &table_ids {
            let elems = self.table_elems[tid].clone();
            elements.active(
                Some(*tid),
                &ConstExpr::i32_const(0),
                Elements::Functions(Cow::Owned(elems)),
            );
        }

        // 10. Data section (string literals).
        let mut data = DataSection::new();
        for (bytes, offset) in self.literals.iter().zip(self.literal_offsets.iter()) {
            let mut payload = Vec::with_capacity(4 + bytes.len());
            payload.extend_from_slice(&(bytes.len() as i32).to_le_bytes());
            payload.extend_from_slice(bytes);
            data.active(0, &ConstExpr::i32_const(*offset), payload);
        }

        // 11. Assemble.
        let mut module = Module::new();
        module.section(&types);
        module.section(&imports);
        module.section(&funcsec);
        if !table_ids.is_empty() {
            module.section(&tables);
        }
        module.section(&memsec);
        module.section(&globalsec);
        module.section(&exports);
        if !table_ids.is_empty() {
            module.section(&elements);
        }
        module.section(&code);
        if !self.literals.is_empty() {
            module.section(&data);
        }
        Ok(module.finish())
    }

    fn emit_current_function(&mut self, code: &mut CodeSection) -> Result<(), String> {
        if self.current_ret != WTy::Void {
            self.zero(self.current_ret);
        }
        self.push(Instruction::End);
        let mut func = Function::new(self.locals.iter().copied());
        for ins in &self.instrs {
            func.instruction(ins);
        }
        code.function(&func);
        Ok(())
    }

    // ---- statements ---------------------------------------------------------

    pub(crate) fn compile_block(&mut self, block: &[crate::ast::Stmt]) -> Result<(), String> {
        for s in block {
            self.compile_stmt(s)?;
        }
        Ok(())
    }

    fn compile_stmt(&mut self, s: &Stmt) -> Result<(), String> {
        use Instruction::*;
        match s {
            Stmt::VarDecl(d) | Stmt::ConstDecl(d) => {
                let declared = d
                    .typ
                    .as_ref()
                    .map(|t| self.dtype_of_type(t))
                    .filter(|t| t.wty() != WTy::Unknown);
                let init_dt = d
                    .init
                    .as_ref()
                    .map(|e| self.dtype_of(e))
                    .filter(|t| t.wty() != WTy::Unknown);
                let dt = if matches!(init_dt, Some(DType::ClosureT(_, _))) {
                    init_dt.clone().unwrap()
                } else {
                    let preferred = match (&declared, &init_dt) {
                        (Some(DType::Obj(c)), Some(i))
                            if !self.classes.contains_key(c)
                                && !self.enums.contains_key(c)
                                && !self.interface_methods.contains_key(c)
                                && i.wty() != WTy::Unknown =>
                        {
                            Some(i.clone())
                        }
                        _ => declared.clone(),
                    };
                    preferred
                        .or(init_dt)
                        .filter(|t| t.wty() != WTy::Void)
                        .unwrap_or(DType::I64)
                };
                let ty = dt.wty();
                match &d.init {
                    Some(e) => {
                        let from = self.compile_expr(e)?;
                        self.coerce(from, ty)?;
                    }
                    None => self.zero(ty),
                }
                let idx = self.add_local(ty);
                self.local_map.insert(d.name.clone(), (idx, ty));
                self.local_dtypes.insert(d.name.clone(), dt);
                if let Some(t) = d.typ.as_ref() {
                    self.local_types.insert(d.name.clone(), t.clone());
                }
                self.push(LocalSet(idx));
                Ok(())
            }
            Stmt::Expr(e) => {
                let ty = self.compile_expr(e)?;
                if ty != WTy::Void && ty != WTy::Unknown {
                    self.push(Drop);
                }
                Ok(())
            }
            Stmt::Return(expr) => {
                let ret = self.current_ret;
                if let Some(e) = expr {
                    let from = self.compile_expr(e)?;
                    if ret == WTy::Void || ret == WTy::Unknown {
                        if from != WTy::Void {
                            self.push(Drop);
                        }
                    } else {
                        self.coerce(from, ret)?;
                    }
                }
                self.push(Return);
                Ok(())
            }
            Stmt::Block(b) => {
                self.push_scope();
                self.compile_block(b)?;
                self.pop_scope();
                Ok(())
            }
            Stmt::If(i) => {
                let cond_ty = self.compile_expr(&i.condition)?;
                self.coerce(cond_ty, WTy::Bool)?;
                self.labels.push(CtrlLabel::Block);
                self.push(If(BlockType::Empty));
                self.push_scope();
                self.compile_block(&i.then_branch)?;
                self.pop_scope();
                if let Some(else_block) = &i.else_branch {
                    self.push(Else);
                    self.push_scope();
                    self.compile_block(else_block)?;
                    self.pop_scope();
                }
                self.push(End);
                self.labels.pop();
                Ok(())
            }
            Stmt::While(w) => {
                let exit = self.labels.len() as u32;
                self.labels.push(CtrlLabel::Block);
                self.push(Block(BlockType::Empty));
                let head = self.labels.len() as u32;
                self.labels.push(CtrlLabel::LoopHead);
                self.push(Loop(BlockType::Empty));
                self.loops.push(LoopCtx { exit_label: exit, head_label: head });

                let ct = self.compile_expr(&w.condition)?;
                self.coerce(ct, WTy::Bool)?;
                self.push(I32Eqz);
                let d = self.label_depth(exit);
                self.push(BrIf(d));

                self.push_scope();
                self.compile_block(&w.body)?;
                self.pop_scope();
                self.br_to(head);

                self.push(End);
                self.push(End);
                self.labels.pop();
                self.labels.pop();
                self.loops.pop();
                Ok(())
            }
            Stmt::DoWhile(d) => {
                let exit = self.labels.len() as u32;
                self.labels.push(CtrlLabel::Block);
                self.push(Block(BlockType::Empty));
                let head = self.labels.len() as u32;
                self.labels.push(CtrlLabel::LoopHead);
                self.push(Loop(BlockType::Empty));
                self.loops.push(LoopCtx { exit_label: exit, head_label: head });

                self.push_scope();
                self.compile_block(&d.body)?;
                self.pop_scope();
                let ct = self.compile_expr(&d.condition)?;
                self.coerce(ct, WTy::Bool)?;
                let dh = self.label_depth(head);
                self.push(BrIf(dh));

                self.push(End);
                self.push(End);
                self.labels.pop();
                self.labels.pop();
                self.loops.pop();
                Ok(())
            }
            Stmt::CFor(c) => {
                if let Some(init) = &c.init {
                    self.compile_stmt(init)?;
                }
                let exit = self.labels.len() as u32;
                self.labels.push(CtrlLabel::Block);
                self.push(Block(BlockType::Empty));
                let head = self.labels.len() as u32;
                self.labels.push(CtrlLabel::LoopHead);
                self.push(Loop(BlockType::Empty));
                self.loops.push(LoopCtx { exit_label: exit, head_label: head });

                if let Some(cond) = &c.condition {
                    let ct = self.compile_expr(cond)?;
                    self.coerce(ct, WTy::Bool)?;
                    self.push(I32Eqz);
                    let d = self.label_depth(exit);
                    self.push(BrIf(d));
                }

                self.push_scope();
                self.compile_block(&c.body)?;
                self.pop_scope();
                if let Some(inc) = &c.increment {
                    let t = self.compile_expr(inc)?;
                    if t != WTy::Void && t != WTy::Unknown {
                        self.push(Drop);
                    }
                }
                self.br_to(head);

                self.push(End);
                self.push(End);
                self.labels.pop();
                self.labels.pop();
                self.loops.pop();
                Ok(())
            }
            Stmt::Break => {
                let ctx = *self.loops.last().ok_or("wasm: break outside loop")?;
                self.br_to(ctx.exit_label);
                Ok(())
            }
            Stmt::Continue => {
                let ctx = *self.loops.last().ok_or("wasm: continue outside loop")?;
                self.br_to(ctx.head_label);
                Ok(())
            }
            Stmt::WhileLet(w) => self.compile_while_let(w),
            Stmt::With(w) => self.compile_with(w),
            Stmt::Throw(e, _) => self.compile_throw(e),
            Stmt::TryCatch {
                try_block,
                catch_var,
                catch_var_type,
                catch_block,
                ..
            } => self.compile_try_catch(
                try_block,
                catch_var,
                catch_var_type.as_ref(),
                catch_block,
            ),
            Stmt::Switch(sw) => self.compile_switch(sw),
            Stmt::For(f) => self.compile_for_in(f),
            Stmt::TupleDestructure { names, expr, .. } => self.compile_tuple_destructure(names, expr),
        }
    }

    /// Lower `while let v = expr { body }`. `expr` is re-evaluated each pass;
    /// an Ok payload is bound to `v` and the body runs, an Err exits the loop.
    fn compile_while_let(&mut self, w: &crate::ast::WhileLetStmt) -> Result<(), String> {
        use Instruction::*;
        let ok_dt = match self.dtype_of(&w.expr) {
            DType::ResultT(ok, _) => *ok,
            _ => return Err("wasm: while let requires a Result-valued expression".to_string()),
        };

        let exit = self.labels.len() as u32;
        self.labels.push(CtrlLabel::Block);
        self.push(Block(BlockType::Empty));
        let head = self.labels.len() as u32;
        self.labels.push(CtrlLabel::LoopHead);
        self.push(Loop(BlockType::Empty));
        self.loops.push(LoopCtx { exit_label: exit, head_label: head });

        let from = self.compile_expr(&w.expr)?;
        if from != WTy::Handle {
            return Err("wasm: while let expression must produce a Result handle".to_string());
        }
        let subject = self.add_local(WTy::Handle);
        self.push(LocalSet(subject));
        self.push(LocalGet(subject));
        self.call_host("result_is_ok")?;
        self.push(I32Eqz);
        let d = self.label_depth(exit);
        self.push(BrIf(d));

        let wt = ok_dt.wty();
        self.push(LocalGet(subject));
        self.call_host(&format!("result_unwrap_{}", ok_dt.suffix()))?;
        let bound = self.add_local(if wt == WTy::Unknown { WTy::I64 } else { wt });
        self.push(LocalSet(bound));

        self.push_scope();
        self.local_map.insert(w.var_name.clone(), (bound, wt));
        self.local_dtypes.insert(w.var_name.clone(), ok_dt);
        self.compile_block(&w.body)?;
        self.pop_scope();
        self.br_to(head);
        self.push(End);
        self.push(End);
        self.labels.pop();
        self.labels.pop();
        self.loops.pop();
        Ok(())
    }

    /// Lower `with (resource) { body }` by binding the resource, compiling the
    /// body, then calling `close()` on the resource (matching the bytecode VM).
    fn compile_with(&mut self, w: &crate::ast::WithStmt) -> Result<(), String> {
        use Instruction::*;
        let class = match self.dtype_of(&w.resource_expr) {
            DType::Obj(c) => c,
            _ => return Err("wasm: with requires a class resource with a close() method".to_string()),
        };
        let name = w
            .var_name
            .clone()
            .unwrap_or_else(|| "__with_resource".to_string());
        let from = self.compile_expr(&w.resource_expr)?;
        if from != WTy::Handle {
            return Err("wasm: with resource must be an object handle".to_string());
        }
        let idx = self.add_local(WTy::Handle);
        self.push(LocalSet(idx));

        self.push_scope();
        self.local_map.insert(name.clone(), (idx, WTy::Handle));
        self.local_dtypes.insert(name.clone(), DType::Obj(class.clone()));
        if let Some(t) = &w.var_type {
            self.local_types.insert(name.clone(), t.clone());
        }
        self.compile_block(&w.body)?;

        let recv = crate::ast::Expr::Identifier(name, w.span);
        let rt = self.compile_method_call(&class, &recv, "close", &[])?;
        if rt != WTy::Void && rt != WTy::Unknown {
            self.push(Drop);
        }
        self.pop_scope();
        Ok(())
    }

    fn compile_switch(&mut self, sw: &crate::ast::SwitchStmt) -> Result<(), String> {
        use Instruction::*;
        let val_dt = self.dtype_of(&sw.expr);
        let value = val_dt.wty();
        if value != WTy::I64
            && value != WTy::Bool
            && value != WTy::Str
            && value != WTy::Char
            && !val_dt.is_obj()
        {
            return Err("wasm: switch requires a scalar, enum or Result value".to_string());
        }
        let compiled = self.compile_expr(&sw.expr)?;
        self.coerce(compiled, value)?;
        let slot = self.add_local(value);
        self.push(LocalSet(slot));

        // Outer block gives each matched case a common exit target.
        let exit = self.labels.len() as u32;
        self.labels.push(CtrlLabel::Block);
        self.push(Block(BlockType::Empty));

        let mut emitted_default = false;
        for case in &sw.cases {
            match &case.pattern {
                Pattern::Wildcard => {
                    self.push_scope();
                    self.compile_block(&case.body)?;
                    self.pop_scope();
                    emitted_default = true;
                }
                Pattern::Literal(lit) => {
                    self.push(LocalGet(slot));
                    match (value, lit) {
                        (WTy::Str, Literal::String(s)) => {
                            let off = self.intern_string(s);
                            self.i32c(off);
                            self.call_host("str_eq")?;
                        }
                        (_, Literal::Int(v)) => {
                            self.i64c(*v);
                            self.push(I64Eq);
                        }
                        (_, Literal::Bool(b)) => {
                            self.i64c(*b as i64);
                            self.push(I64Eq);
                        }
                        (_, Literal::Char(c)) => {
                            self.i64c(*c as i64);
                            self.push(I64Eq);
                        }
                        _ => {
                            return Err("wasm: switch pattern type does not match value".to_string())
                        }
                    }
                    self.emit_case_body(&case.body, exit)?;
                }
                Pattern::Constructor { name, bindings } => {
                    let bind_types = self.constructor_match(&val_dt, slot, name, bindings)?;
                    self.emit_case_body_with_bindings(
                        &case.body,
                        bindings,
                        &bind_types,
                        slot,
                        exit,
                        matches!(val_dt, DType::ResultT(_, _)),
                    )?;
                }
            }
        }

        if !emitted_default {
            if let Some(def) = &sw.default {
                self.push_scope();
                self.compile_block(def)?;
                self.pop_scope();
            }
        }

        self.push(End);
        self.labels.pop();
        Ok(())
    }

    /// Emit the boolean test and payload binds for a constructor pattern.
    fn constructor_match(
        &mut self,
        val_dt: &DType,
        slot: u32,
        name: &str,
        bindings: &[String],
    ) -> Result<Vec<DType>, String> {
        use Instruction::*;
        match val_dt {
            DType::ResultT(ok, _) => {
                let expected = match name {
                    "Ok" => 1,
                    "Err" => 0,
                    _ => return Err(format!("wasm: unknown Result variant '{}'", name)),
                };
                self.push(LocalGet(slot));
                self.call_host("result_is_ok")?;
                self.i32c(expected);
                self.push(I32Eq);
                let mut types = Vec::new();
                for _ in bindings {
                    types.push(if name == "Ok" {
                        (**ok).clone()
                    } else {
                        DType::Str
                    });
                }
                Ok(types)
            }
            DType::Enum(ename) => {
                let fields = self.enums.get(ename).and_then(|v| v.get(name)).cloned();
                if fields.is_none() && !self.enums.get(ename).is_some_and(|v| v.contains_key(name)) {
                    return Err(format!(
                        "wasm: unknown variant '{}' of enum '{}'",
                        name, ename
                    ));
                }
                self.push(LocalGet(slot));
                self.call_host("enum_variant")?;
                let off = self.intern_string(name);
                self.i32c(off);
                self.call_host("str_eq")?;
                Ok(fields
                    .unwrap_or_default()
                    .iter()
                    .map(|t| self.dtype_of_type(t))
                    .collect())
            }
            _ => Err("wasm: constructor pattern requires an enum or Result value".to_string()),
        }
    }

    fn emit_case_body(
        &mut self,
        body: &[crate::ast::Stmt],
        exit: u32,
    ) -> Result<(), String> {
        use Instruction::*;
        self.labels.push(CtrlLabel::Block);
        self.push(If(BlockType::Empty));
        self.push_scope();
        self.compile_block(body)?;
        self.pop_scope();
        self.br_to(exit);
        self.push(End);
        self.labels.pop();
        Ok(())
    }

    fn emit_case_body_with_bindings(
        &mut self,
        body: &[crate::ast::Stmt],
        bindings: &[String],
        bind_types: &[DType],
        slot: u32,
        exit: u32,
        is_result: bool,
    ) -> Result<(), String> {
        use Instruction::*;
        // The match condition sits on the stack; save it so the subject handle
        // can be captured in a local before the conditional is emitted.
        let cond = self.add_local(WTy::Bool);
        self.push(LocalSet(cond));
        let subject = self.add_local(WTy::Handle);
        self.push(LocalGet(slot));
        self.push(LocalSet(subject));
        self.labels.push(CtrlLabel::Block);
        self.push(LocalGet(cond));
        self.push(If(BlockType::Empty));
        self.push_scope();
        for (i, name) in bindings.iter().enumerate() {
            let dt = bind_types.get(i).cloned().unwrap_or(DType::Unknown);
            self.push(LocalGet(subject));
            if !is_result {
                self.i64c(i as i64);
            }
            let host = if is_result {
                format!("result_unwrap_{}", dt.suffix())
            } else {
                format!("enum_get_{}", dt.suffix())
            };
            self.call_host(&host)?;
            let ty = dt.wty();
            let idx = self.add_local(if ty == WTy::Unknown { WTy::I64 } else { ty });
            self.push(LocalSet(idx));
            self.local_map.insert(name.clone(), (idx, ty));
            self.local_dtypes.insert(name.clone(), dt);
        }
        self.compile_block(body)?;
        self.pop_scope();
        self.br_to(exit);
        self.push(End);
        self.labels.pop();
        Ok(())
    }

    // ---- expressions --------------------------------------------------------

    pub(crate) fn compile_expr(&mut self, e: &Expr) -> Result<WTy, String> {
        use Instruction::*;
        match e {
            Expr::Literal(l, _) => match l {
                Literal::Int(v) => {
                    self.i64c(*v);
                    Ok(WTy::I64)
                }
                Literal::Float(v) => {
                    self.f64c(*v);
                    Ok(WTy::F64)
                }
                Literal::Bool(b) => {
                    self.i32c(*b as i32);
                    Ok(WTy::Bool)
                }
                Literal::Char(c) => {
                    self.i32c(*c as i32);
                    Ok(WTy::Char)
                }
                Literal::String(s) => {
                    let off = self.intern_string(s);
                    self.i32c(off);
                    Ok(WTy::Str)
                }
                Literal::Null => {
                    self.i32c(0);
                    Ok(WTy::Str)
                }
            },
            Expr::Identifier(name, _) => {
                if let Some(env) = self.env_local {
                    if let Some(dt) = self.closure_captures.get(name).cloned() {
                        let off = self.intern_string(&format!("__cap_{}", name));
                        self.push(LocalGet(env));
                        self.i32c(off);
                        self.call_host(field_getter(dt.wty()))?;
                        return Ok(dt.wty());
                    }
                }
                if let Some((idx, ty)) = self.lookup_local(name) {
                    self.push(LocalGet(idx));
                    return Ok(ty);
                }
                if let Some((idx, ty)) = self.globals.get(name).copied() {
                    self.push(GlobalGet(idx));
                    return Ok(ty);
                }
                Err(format!("wasm: unknown variable '{}'", name))
            }
            Expr::Unit(_) => Ok(WTy::Void),
            Expr::This(_) | Expr::Super(_) => {
                let idx = self
                    .local_map
                    .get("this")
                    .map(|v| v.0)
                    .ok_or("wasm: 'this' used outside a method")?;
                self.push(LocalGet(idx));
                Ok(WTy::Handle)
            }
            Expr::New(typ, args, _) => {
                let class = typ.name().to_string();
                if let Some(result) = self.compile_collection_new(&class) {
                    return result;
                }
                let (ctor_params, func_idx) = {
                    let info = self
                        .classes
                        .get(&class)
                        .ok_or_else(|| format!("wasm: unknown class '{}'", class))?;
                    let ctor = info
                        .ctor
                        .as_ref()
                        .ok_or_else(|| format!("wasm: class '{}' has no constructor", class))?;
                    let fname = format!("{}::init", class);
                    let fi = self
                        .functions
                        .get(&fname)
                        .ok_or_else(|| format!("wasm: no constructor for '{}'", class))?;
                    (ctor.params.clone(), fi.func_idx)
                };
                let ptypes: Vec<WTy> = ctor_params.iter().map(|p| ty_of(&p.typ)).collect();
                if args.len() != ptypes.len() {
                    return Err(format!(
                        "wasm: constructor '{}' expects {} args, got {}",
                        class,
                        ptypes.len(),
                        args.len()
                    ));
                }
                let name_off = self.intern_string(&class);
                self.i32c(name_off);
                self.call_host("obj_named_new")?;
                for (arg, pt) in args.iter().zip(ptypes) {
                    let from = self.compile_expr(arg)?;
                    self.coerce(from, pt)?;
                }
                self.push(Call(func_idx));
                self.set_vtbl_fields(&class)?;
                Ok(WTy::Handle)
            }
            Expr::Unary(op, inner, _) => {
                let t = self.compile_expr(inner)?;
                match op {
                    UnOp::Neg => {
                        if t == WTy::F64 {
                            self.push(F64Neg);
                        } else {
                            self.coerce(t, WTy::I64)?;
                            // WASM has no integer negate; compute 0 - value.
                            let tmp = self.add_local(WTy::I64);
                            self.push(LocalSet(tmp));
                            self.i64c(0);
                            self.push(LocalGet(tmp));
                            self.push(I64Sub);
                        }
                        Ok(if t == WTy::F64 { WTy::F64 } else { WTy::I64 })
                    }
                    UnOp::BitNot => {
                        self.coerce(t, WTy::I64)?;
                        self.i64c(-1);
                        self.push(I64Xor);
                        Ok(WTy::I64)
                    }
                    UnOp::Not => {
                        if t == WTy::F64 {
                            self.f64c(0.0);
                            self.push(F64Ne);
                        } else if t == WTy::I64 {
                            self.i64c(0);
                            self.push(I64Ne);
                        } else if t == WTy::Bool {
                            // already 0/1
                        } else {
                            self.coerce(t, WTy::Bool)?;
                        }
                        self.push(I32Eqz);
                        Ok(WTy::Bool)
                    }
                }
            }
            Expr::Binary(l, op, r, _) => self.compile_binary(l, op, r),
            Expr::Assign(target, value, _) => {
                if let Expr::MemberAccess(obj, field, _) = target.as_ref() {
                    let class = self.class_of_expr(obj).ok_or_else(|| {
                        format!("wasm: cannot resolve field '.{}' for assignment", field)
                    })?;
                    let fty = self
                        .field_type(&class, field)
                        .map(|t| ty_of(&t))
                        .ok_or_else(|| {
                            format!("wasm: class '{}' has no field '{}'", class, field)
                        })?;
                    self.compile_expr(obj)?;
                    let off = self.intern_string(field);
                    self.i32c(off);
                    let from = self.compile_expr(value)?;
                    self.coerce(from, fty)?;
                    self.call_host(field_setter(fty))?;
                    return Ok(WTy::Void);
                }
                let name = match target.as_ref() {
                    Expr::Identifier(n, _) => n.clone(),
                    other => return Err(format!("wasm: unsupported assignment target: {:?}", other)),
                };
                if let Some(env) = self.env_local {
                    if let Some(dt) = self.closure_captures.get(&name).cloned() {
                        let off = self.intern_string(&format!("__cap_{}", name));
                        let from = self.compile_expr(value)?;
                        self.coerce(from, dt.wty())?;
                        let vt = self.add_local(dt.wty());
                        self.push(LocalSet(vt));
                        self.push(LocalGet(env));
                        self.i32c(off);
                        self.push(LocalGet(vt));
                        self.call_host(field_setter(dt.wty()))?;
                        return Ok(WTy::Void);
                    }
                }
                let ty = if let Some((_, t)) = self.lookup_local(&name) {
                    t
                } else if let Some((_, t)) = self.globals.get(&name) {
                    *t
                } else {
                    return Err(format!("wasm: unknown assignment target '{}'", name));
                };
                let from = self.compile_expr(value)?;
                self.coerce(from, ty)?;
                if let Some((idx, _)) = self.lookup_local(&name) {
                    self.push(LocalTee(idx));
                } else if let Some((idx, _)) = self.globals.get(&name) {
                    let idx = *idx;
                    self.push(GlobalSet(idx));
                    self.push(GlobalGet(idx));
                }
                Ok(ty)
            }
            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                let ct = self.compile_expr(condition)?;
                self.coerce(ct, WTy::Bool)?;
                let then_ty = self.infer(then_expr);
                let else_ty = self.infer(else_expr);
                let result = if then_ty != WTy::Unknown && then_ty != WTy::Void {
                    then_ty
                } else {
                    else_ty
                };
                let bt = result
                    .val_type()
                    .map(BlockType::Result)
                    .unwrap_or(BlockType::Empty);
                self.labels.push(CtrlLabel::Block);
                self.push(If(bt));
                let a = self.compile_expr(then_expr)?;
                if result != WTy::Void {
                    self.coerce(a, result)?;
                }
                self.push(Else);
                let b = self.compile_expr(else_expr)?;
                if result != WTy::Void {
                    self.coerce(b, result)?;
                }
                self.push(End);
                self.labels.pop();
                Ok(result)
            }
            Expr::Call(callee, args, _) => self.compile_call(callee, args),
            Expr::StaticCall {
                class_name,
                method,
                args,
                ..
            } => self.compile_named_call(class_name, method, args),
            Expr::Cast(inner, target, _) => {
                let from = self.compile_expr(inner)?;
                let to = ty_of(target);
                self.coerce(from, to)?;
                Ok(to)
            }
            Expr::MemberAccess(obj, field, _) => {
                let class = self.class_of_expr(obj).ok_or_else(|| {
                    format!("wasm: cannot resolve field '.{}' on expression", field)
                })?;
                let fty = self
                    .field_type(&class, field)
                    .map(|t| ty_of(&t))
                    .ok_or_else(|| format!("wasm: class '{}' has no field '{}'", class, field))?;
                self.compile_expr(obj)?;
                let off = self.intern_string(field);
                self.i32c(off);
                self.call_host(field_getter(fty))?;
                Ok(fty)
            }
            Expr::Closure {
                captured_vars,
                span,
                ..
            } => self.compile_closure(*span, captured_vars),
            Expr::ErrorPropagation(inner, _) => self.compile_error_propagation(inner),
            Expr::OwnedDeref(inner, _) | Expr::RegionAlloc(_, inner, _) | Expr::RefExpr(inner, _, _) => {
                self.compile_expr(inner)
            }
            Expr::UnsafeBlock(body, _) | Expr::RegionBlock(body, _) => {
                self.push_scope();
                self.compile_block(body)?;
                self.pop_scope();
                Ok(WTy::Void)
            }
            Expr::Index(obj, idx, _) => self.compile_index(obj, idx),
            Expr::Is(expr, typ, _) => self.compile_is(expr, typ),
            Expr::Tuple(elems, _) => self.compile_tuple(elems),
            other => Err(format!("wasm: unsupported expression: {:?}", other)),
        }
    }

    fn compile_binary(&mut self, l: &Expr, op: &Operator, r: &Expr) -> Result<WTy, String> {
        use Instruction::*;
        let ldt = self.dtype_of(l);
        let method = match op {
            Operator::Add => "operator+",
            Operator::Sub => "operator-",
            Operator::Mul => "operator*",
            Operator::Div => "operator/",
            Operator::Mod => "operator%",
            _ => "",
        };
        if !method.is_empty() {
            if let DType::Obj(class) = &ldt {
                if self
                    .all_class_methods
                    .get(class)
                    .is_some_and(|m| m.contains_key(method))
                {
                    return self.compile_method_call(class, l, method, std::slice::from_ref(r));
                }
            }
        }
        let lt = self.infer(l);
        let rt = self.infer(r);

        if *op == Operator::Add && (lt == WTy::Str || rt == WTy::Str) {
            let a = self.compile_expr(l)?;
            self.coerce_to_str(a)?;
            let b = self.compile_expr(r)?;
            self.coerce_to_str(b)?;
            self.call_host("concat")?;
            return Ok(WTy::Str);
        }

        match op {
            Operator::Eq | Operator::Ne | Operator::Lt | Operator::Gt | Operator::Le
            | Operator::Ge => {
                if lt == WTy::Str && rt == WTy::Str {
                    let a = self.compile_expr(l)?;
                    let _ = a;
                    let b = self.compile_expr(r)?;
                    let _ = b;
                    self.call_host("str_eq")?;
                    if *op == Operator::Ne {
                        self.push(I32Eqz);
                    }
                    return Ok(WTy::Bool);
                }
                if lt == WTy::F64 || rt == WTy::F64 {
                    let a = self.compile_expr(l)?;
                    self.coerce(a, WTy::F64)?;
                    let b = self.compile_expr(r)?;
                    self.coerce(b, WTy::F64)?;
                    self.push(match op {
                        Operator::Eq => F64Eq,
                        Operator::Ne => F64Ne,
                        Operator::Lt => F64Lt,
                        Operator::Gt => F64Gt,
                        Operator::Le => F64Le,
                        Operator::Ge => F64Ge,
                        _ => unreachable!(),
                    });
                } else {
                    let a = self.compile_expr(l)?;
                    self.coerce(a, WTy::I64)?;
                    let b = self.compile_expr(r)?;
                    self.coerce(b, WTy::I64)?;
                    self.push(match op {
                        Operator::Eq => I64Eq,
                        Operator::Ne => I64Ne,
                        Operator::Lt => I64LtS,
                        Operator::Gt => I64GtS,
                        Operator::Le => I64LeS,
                        Operator::Ge => I64GeS,
                        _ => unreachable!(),
                    });
                }
                Ok(WTy::Bool)
            }
            Operator::And | Operator::Or => {
                let a = self.compile_expr(l)?;
                self.coerce(a, WTy::Bool)?;
                self.labels.push(CtrlLabel::Block);
                self.push(If(BlockType::Result(ValType::I32)));
                if *op == Operator::And {
                    let b = self.compile_expr(r)?;
                    self.coerce(b, WTy::Bool)?;
                    self.push(Else);
                    self.i32c(0);
                } else {
                    self.i32c(1);
                    self.push(Else);
                    let b = self.compile_expr(r)?;
                    self.coerce(b, WTy::Bool)?;
                }
                self.push(End);
                self.labels.pop();
                Ok(WTy::Bool)
            }
            _ => {
                if lt == WTy::F64 || rt == WTy::F64 {
                    let a = self.compile_expr(l)?;
                    self.coerce(a, WTy::F64)?;
                    let b = self.compile_expr(r)?;
                    self.coerce(b, WTy::F64)?;
                    match op {
                        Operator::Add => self.push(F64Add),
                        Operator::Sub => self.push(F64Sub),
                        Operator::Mul => self.push(F64Mul),
                        Operator::Div => self.push(F64Div),
                        Operator::Mod => self.call_host("fmod")?,
                        _ => return Err(format!("wasm: unsupported float operator {:?}", op)),
                    }
                    Ok(WTy::F64)
                } else {
                    let a = self.compile_expr(l)?;
                    self.coerce(a, WTy::I64)?;
                    let b = self.compile_expr(r)?;
                    self.coerce(b, WTy::I64)?;
                    self.push(match op {
                        Operator::Add => I64Add,
                        Operator::Sub => I64Sub,
                        Operator::Mul => I64Mul,
                        Operator::Div => I64DivS,
                        Operator::Mod => I64RemS,
                        Operator::BitAnd => I64And,
                        Operator::BitOr => I64Or,
                        Operator::BitXor => I64Xor,
                        Operator::BitShl => I64Shl,
                        Operator::BitShr => I64ShrS,
                        Operator::BitUshr => I64ShrU,
                        _ => return Err(format!("wasm: unsupported integer operator {:?}", op)),
                    });
                    Ok(WTy::I64)
                }
            }
        }
    }

    fn coerce_to_str(&mut self, from: WTy) -> Result<(), String> {
        if from == WTy::Str {
            return Ok(());
        }
        match from {
            WTy::I64 => self.call_host("to_string_i64")?,
            WTy::F64 => self.call_host("to_string_f64")?,
            WTy::Bool => self.call_host("to_string_bool")?,
            WTy::Char => self.call_host("to_string_char")?,
            _ => return Err(format!("wasm: cannot stringify {:?}", from)),
        }
        Ok(())
    }

    fn compile_call(&mut self, callee: &Expr, args: &[Expr]) -> Result<WTy, String> {
        if let DType::ClosureT(ptypes, ret) = self.dtype_of(callee) {
            return self.compile_closure_call(callee, args, &ptypes, &ret);
        }
        match callee {
            Expr::Identifier(name, _) => {
                if let Some(result) = self.compile_tagged_call(name, args) {
                    return result;
                }
                if let Some(info) = self.functions.get(name) {
                    let info = info.clone();
                    self.compile_args(args, &info.params)?;
                    self.push(Instruction::Call(info.func_idx));
                    return Ok(info.ret);
                }
                match name.as_str() {
                    "println" | "print" | "eprintln" => {
                        self.compile_named_call("io", if name == "print" { "print" } else { "println" }, args)
                    }
                    "toString" => self.compile_named_call("", "toString", args),
                    "parseInt" => self.compile_named_call("Integer", "parseInt", args),
                    "parseDouble" => self.compile_named_call("Double", "parseDouble", args),
                    _ => Err(format!("wasm: unknown function '{}' (dtype {:?})", name, self.dtype_of(callee))),
                }
            }
            Expr::MemberAccess(obj, method, _) => {
                if let Some(result) = self.compile_composite_call(obj, method, args) {
                    return result;
                }
                if matches!(obj.as_ref(), Expr::Super(_)) {
                    let cls = self
                        .current_class
                        .clone()
                        .ok_or("wasm: super call outside method")?;
                    return self.compile_method_call(&cls, obj, method, args);
                }
                if let Some(class) = self.class_of_expr(obj) {
                    return self.compile_method_call(&class, obj, method, args);
                }
                if let Expr::Identifier(module, _) = obj.as_ref() {
                    self.compile_named_call(module, method, args)
                } else {
                    Err("wasm: indirect calls are not supported".to_string())
                }
            }
            _ => Err("wasm: indirect calls are not supported".to_string()),
        }
    }

    fn compile_method_call(
        &mut self,
        class: &str,
        obj: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Result<WTy, String> {
        // `super.method(...)` is a statically bound call to the parent.
        if matches!(obj, Expr::Super(_)) {
            let parent = self
                .class_parents
                .get(class)
                .cloned()
                .ok_or_else(|| format!("wasm: '{}' has no parent for super call", class))?;
            let md = if method == "init" {
                self.classes
                    .get(&parent)
                    .and_then(|c| c.ctor.clone())
                    .ok_or("wasm: parent has no constructor")?
            } else {
                self.all_class_methods
                    .get(&parent)
                    .and_then(|m| m.get(method))
                    .cloned()
                    .ok_or_else(|| format!("wasm: parent has no method '{}'", method))?
            };
            let ptypes: Vec<WTy> = md.params.iter().map(|p| ty_of(&p.typ)).collect();
            let fname = format!("{}::{}", parent, method);
            let func_idx = self
                .functions
                .get(&fname)
                .ok_or_else(|| format!("wasm: missing code for '{}'", fname))?
                .func_idx;
            self.push(Instruction::LocalGet(
                self.local_map
                    .get("this")
                    .ok_or("wasm: super call outside method")?
                    .0,
            ));
            if args.len() != ptypes.len() {
                return Err(format!("wasm: method '{}' expects {} args", method, ptypes.len()));
            }
            for (a, pt) in args.iter().zip(ptypes) {
                let from = self.compile_expr(a)?;
                self.coerce(from, pt)?;
            }
            self.push(Instruction::Call(func_idx));
            // Constructors return a fresh handle; `super.init(...)` ignores it.
            self.push(Instruction::Drop);
            return Ok(WTy::Void);
        }

        let (param_types, ret) = if let Some(md) = self
            .all_class_methods
            .get(class)
            .and_then(|m| m.get(method))
            .cloned()
        {
            (
                md.params.iter().map(|p| ty_of(&p.typ)).collect::<Vec<_>>(),
                md.return_type.as_ref().map(ty_of).unwrap_or(WTy::Void),
            )
        } else if let Some(sig) = self
            .interface_methods
            .get(class)
            .and_then(|m| m.get(method))
            .cloned()
        {
            (
                sig.params.iter().map(|p| ty_of(&p.typ)).collect::<Vec<_>>(),
                sig.return_type.as_ref().map(ty_of).unwrap_or(WTy::Void),
            )
        } else {
            return Err(format!("wasm: '{}' has no method '{}'", class, method));
        };

        if args.len() != param_types.len() {
            return Err(format!(
                "wasm: method '{}' expects {} args, got {}",
                method,
                param_types.len(),
                args.len()
            ));
        }
        self.compile_expr(obj)?;
        let obj_local = self.add_local(WTy::Handle);
        self.push(Instruction::LocalSet(obj_local));
        self.push(Instruction::LocalGet(obj_local));
        for (arg, pt) in args.iter().zip(&param_types) {
            let from = self.compile_expr(arg)?;
            self.coerce(from, *pt)?;
        }

        // When the receiver's concrete class is known, bind directly. This
        // avoids an indirect call for the common non-polymorphic case.
        let fname = format!("{}::{}", class, method);
        if !self.interface_methods.contains_key(class) {
            if let Some(fi) = self.functions.get(&fname) {
                let idx = fi.func_idx;
                self.push(Instruction::Call(idx));
                return Ok(ret);
            }
        }

        let table = *self
            .method_tables
            .get(method)
            .ok_or_else(|| format!("wasm: no dispatch table for '{}'", method))?;
        let type_idx = *self
            .method_type_idx
            .get(method)
            .ok_or_else(|| format!("wasm: no dispatch type for '{}'", method))?;
        let off = self.intern_string(&format!("__vtbl_{}", method));
        self.push(Instruction::LocalGet(obj_local));
        self.i32c(off);
        self.call_host("field_get_i64")?;
        self.push(Instruction::I32WrapI64);
        self.push(Instruction::CallIndirect {
            type_index: type_idx,
            table_index: table,
        });
        Ok(ret)
    }

    fn compile_args(&mut self, args: &[Expr], params: &[WTy]) -> Result<(), String> {
        if args.len() != params.len() {
            return Err(format!(
                "wasm: argument count mismatch: expected {}, got {}",
                params.len(),
                args.len()
            ));
        }
        for (arg, param) in args.iter().zip(params) {
            let from = self.compile_expr(arg)?;
            self.coerce(from, *param)?;
        }
        Ok(())
    }

    fn compile_named_call(
        &mut self,
        module: &str,
        method: &str,
        args: &[Expr],
    ) -> Result<WTy, String> {
        match (module, method) {
            (_, "println") => {
                if args.is_empty() {
                    return Err("wasm: println expects one argument".to_string());
                }
                let from = self.compile_expr(&args[0])?;
                match from {
                    WTy::Str => self.call_host("print")?,
                    WTy::I64 => self.call_host("print_i64")?,
                    WTy::F64 => self.call_host("print_f64")?,
                    WTy::Bool => self.call_host("print_bool")?,
                    WTy::Char => self.call_host("print_char")?,
                    _ => return Err("wasm: unsupported println argument type".to_string()),
                }
                Ok(WTy::Void)
            }
            (_, "print") => {
                if args.is_empty() {
                    return Err("wasm: print expects one argument".to_string());
                }
                let from = self.compile_expr(&args[0])?;
                self.coerce_to_str(from)?;
                self.call_host("print_raw")?;
                Ok(WTy::Void)
            }
            (_, "toString") => {
                if args.is_empty() {
                    return Err("wasm: toString expects one argument".to_string());
                }
                let from = self.compile_expr(&args[0])?;
                self.coerce_to_str(from)?;
                Ok(WTy::Str)
            }
            ("Integer" | "Long" | "Int", "parseInt" | "parseLong" | "parse") => {
                let from = self.compile_expr(&args[0])?;
                if from != WTy::Str {
                    self.coerce_to_str(from)?;
                }
                self.call_host("parse_int")?;
                Ok(WTy::I64)
            }
            ("Double" | "Float", "parseDouble" | "parseFloat" | "parse") => {
                let from = self.compile_expr(&args[0])?;
                if from != WTy::Str {
                    self.coerce_to_str(from)?;
                }
                self.call_host("parse_double")?;
                Ok(WTy::F64)
            }
            ("String", m) => self.compile_string_call(m, args),
            ("Math" | "MathAdvanced" | "MathTrig", m) => {
                self.compile_math_call(m, args)
            }
            ("Json", "parse") => self.compile_json_parse(args),
            ("Json", "stringify") => {
                if args.is_empty() {
                    return Err("wasm: Json.stringify expects one argument".to_string());
                }
                let from = self.compile_expr(&args[0])?;
                let host = match from {
                    WTy::I64 => "json_stringify_i64",
                    WTy::F64 => "json_stringify_f64",
                    WTy::Bool => "json_stringify_bool",
                    WTy::Str => "json_stringify_str",
                    WTy::Handle => "json_stringify_handle",
                    _ => return Err("wasm: unsupported Json.stringify argument".to_string()),
                };
                self.call_host(host)?;
                Ok(WTy::Str)
            }
            ("File", m @ ("readFile" | "read" | "write" | "writeFile" | "append" | "exists" | "delete")) => {
                let host = match m {
                    "readFile" | "read" => "file_read",
                    "write" | "writeFile" => "file_write",
                    "append" => "file_append",
                    "exists" => "file_exists",
                    _ => "file_delete",
                };
                for a in args {
                    let t = self.compile_expr(a)?;
                    self.coerce_to_str(t)?;
                }
                self.call_host(host)?;
                Ok(match m {
                    "readFile" | "read" => WTy::Str,
                    "exists" => WTy::Bool,
                    _ => WTy::Void,
                })
            }
            _ => Err(format!("wasm: unsupported call {}.{}", module, method)),
        }
    }

    fn compile_string_call(&mut self, method: &str, args: &[Expr]) -> Result<WTy, String> {
        use Instruction::*;
        match method {
            "length" | "size" | "count" => {
                self.compile_expr(&args[0])?;
                self.call_host("str_len")?;
                self.push(I64ExtendI32U);
                Ok(WTy::I64)
            }
            "charAt" => {
                self.compile_expr(&args[0])?;
                let idx_ty = self.compile_expr(&args[1])?;
                self.coerce(idx_ty, WTy::I64)?;
                self.push(I32WrapI64);
                self.call_host("str_char_at")?;
                Ok(WTy::Str)
            }
            "substring" => {
                self.compile_expr(&args[0])?;
                for a in &args[1..] {
                    let t = self.compile_expr(a)?;
                    self.coerce(t, WTy::I64)?;
                    self.push(I32WrapI64);
                }
                self.call_host("str_substring")?;
                Ok(WTy::Str)
            }
            "indexOf" | "lastIndexOf" => {
                self.compile_expr(&args[0])?;
                self.compile_expr(&args[1])?;
                self.call_host("str_index_of")?;
                self.push(I64ExtendI32U);
                Ok(WTy::I64)
            }
            "toUpperCase" | "toLowerCase" | "trim" | "trimStart" | "trimEnd" => {
                self.compile_expr(&args[0])?;
                let host = match method {
                    "toUpperCase" => "str_upper",
                    "toLowerCase" => "str_lower",
                    _ => "str_trim",
                };
                self.call_host(host)?;
                Ok(WTy::Str)
            }
            "startsWith" | "endsWith" => {
                self.compile_expr(&args[0])?;
                self.compile_expr(&args[1])?;
                self.call_host(if method == "startsWith" {
                    "str_starts_with"
                } else {
                    "str_ends_with"
                })?;
                Ok(WTy::Bool)
            }
            "fromCharCode" => {
                let t = self.compile_expr(&args[0])?;
                self.coerce(t, WTy::I64)?;
                self.push(I32WrapI64);
                self.call_host("to_string_char")?;
                Ok(WTy::Str)
            }
            "equals" => {
                self.compile_expr(&args[0])?;
                self.compile_expr(&args[1])?;
                self.call_host("str_eq")?;
                Ok(WTy::Bool)
            }
            _ => Err(format!("wasm: unsupported String method '{}'", method)),
        }
    }

    fn json_scalar_dtype(&self, values: &[JsonLit]) -> Result<DType, String> {
        let first = values
            .first()
            .map(|v| self.json_scalar_dtype_one(v))
            .transpose()?
            .unwrap_or(DType::Unknown);
        for v in values {
            if self.json_scalar_dtype_one(v)? != first {
                return Err("wasm: JSON collections must have homogeneous values".to_string());
            }
        }
        Ok(first)
    }

    fn json_scalar_dtype_one(&self, v: &JsonLit) -> Result<DType, String> {
        Ok(match v {
            JsonLit::Int(_) => DType::I64,
            JsonLit::Float(_) => DType::F64,
            JsonLit::Bool(_) => DType::Bool,
            JsonLit::Str(_) => DType::Str,
            _ => return Err("wasm: nested JSON values are unsupported".to_string()),
        })
    }

    fn json_scalar_expr(&self, v: &JsonLit, span: crate::ast::Span) -> Result<Expr, String> {
        Ok(match v {
            JsonLit::Int(n) => Expr::Literal(Literal::Int(*n), span),
            JsonLit::Float(f) => Expr::Literal(Literal::Float(*f), span),
            JsonLit::Bool(b) => Expr::Literal(Literal::Bool(*b), span),
            JsonLit::Str(s) => Expr::Literal(Literal::String(s.clone()), span),
            _ => return Err("wasm: nested JSON values are unsupported".to_string()),
        })
    }

    fn compile_json_parse(&mut self, args: &[Expr]) -> Result<WTy, String> {
        use Instruction::*;
        let (text, span) = match args.first() {
            Some(Expr::Literal(Literal::String(s), sp)) => (s.clone(), *sp),
            _ => return Err("wasm: Json.parse requires a string literal argument".to_string()),
        };
        let lit = super::json::parse(&text)
            .map_err(|e| format!("wasm: invalid JSON literal: {}", e))?;
        match lit {
            JsonLit::Int(v) => {
                self.i64c(v);
                Ok(WTy::I64)
            }
            JsonLit::Float(v) => {
                self.f64c(v);
                Ok(WTy::F64)
            }
            JsonLit::Bool(b) => {
                self.i32c(b as i32);
                Ok(WTy::Bool)
            }
            JsonLit::Str(s) => {
                let off = self.intern_string(&s);
                self.i32c(off);
                Ok(WTy::Str)
            }
            JsonLit::Null => Err("wasm: JSON null is unsupported".to_string()),
            JsonLit::Array(xs) => {
                let elem = self.json_scalar_dtype(&xs)?;
                let _ = elem;
                self.call_host("list_new")?;
                let h = self.add_local(WTy::Handle);
                self.push(LocalSet(h));
                for x in &xs {
                    self.push(LocalGet(h));
                    let e = self.json_scalar_expr(x, span)?;
                    self.push_boxed_value(&e)?;
                    self.call_host("list_add")?;
                }
                self.push(LocalGet(h));
                Ok(WTy::Handle)
            }
            JsonLit::Object(entries) => {
                let values: Vec<JsonLit> = entries.iter().map(|(_, v)| v.clone()).collect();
                self.json_scalar_dtype(&values)?;
                self.call_host("map_new")?;
                let h = self.add_local(WTy::Handle);
                self.push(LocalSet(h));
                for (k, v) in &entries {
                    self.push(LocalGet(h));
                    let off = self.intern_string(k);
                    self.i32c(off);
                    let e = self.json_scalar_expr(v, span)?;
                    self.push_boxed_value(&e)?;
                    self.call_host("map_put_str")?;
                }
                self.push(LocalGet(h));
                Ok(WTy::Handle)
            }
        }
    }

    fn compile_math_call(&mut self, method: &str, args: &[Expr]) -> Result<WTy, String> {
        use Instruction::*;
        let host = match method {
            "sin" => "math_sin",
            "cos" => "math_cos",
            "tan" => "math_tan",
            "asin" => "math_asin",
            "acos" => "math_acos",
            "atan" => "math_atan",
            "atan2" => "math_atan2",
            "exp" => "math_exp",
            "ln" | "log" => "math_ln",
            "log2" => "math_log2",
            "log10" => "math_log10",
            "pow" => "math_pow",
            "cbrt" => "math_cbrt",
            "hypot" => "math_hypot",
            "abs" | "fabs" => "math_abs",
            "floor" => "math_floor",
            "ceil" => "math_ceil",
            "round" => "math_round",
            "sqrt" => "math_sqrt",
            "min" => {
                for a in args {
                    let t = self.compile_expr(a)?;
                    self.coerce(t, WTy::F64)?;
                }
                self.push(F64Min);
                return Ok(WTy::F64);
            }
            "max" => {
                for a in args {
                    let t = self.compile_expr(a)?;
                    self.coerce(t, WTy::F64)?;
                }
                self.push(F64Max);
                return Ok(WTy::F64);
            }
            _ => return Err(format!("wasm: unsupported math function '{}'", method)),
        };
        for a in args {
            let t = self.compile_expr(a)?;
            self.coerce(t, WTy::F64)?;
        }
        self.call_host(host)?;
        Ok(WTy::F64)
    }
}
