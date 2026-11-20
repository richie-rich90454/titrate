//! AST lowering: statements and expressions to WebAssembly instructions.

use super::runtime;
use super::{ty_of, CtrlLabel, FuncInfo, LoopCtx, WasmBackend, WTy};
use crate::ast::{Expr, Literal, Operator, Pattern, Stmt, Type, UnOp};
use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, DataSection, EntityType, ExportSection, FuncType, Function,
    FunctionSection, GlobalSection, GlobalType, Ieee64, ImportSection, Instruction, MemorySection,
    MemoryType, Module, TypeSection, ValType,
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
                self.call_host("obj_new")?;
                let this_idx = self.add_local(WTy::Handle);
                self.push(Instruction::LocalSet(this_idx));
                self.local_map.insert("this".to_string(), (this_idx, WTy::Handle));
                self.local_types
                    .insert("this".to_string(), Type::simple(g.this_class.as_deref().unwrap_or("")));
                // Initialize declared fields from constructor parameters.
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

        // 9. Data section (string literals).
        let mut data = DataSection::new();
        for (bytes, offset) in self.literals.iter().zip(self.literal_offsets.iter()) {
            let mut payload = Vec::with_capacity(4 + bytes.len());
            payload.extend_from_slice(&(bytes.len() as i32).to_le_bytes());
            payload.extend_from_slice(bytes);
            data.active(0, &ConstExpr::i32_const(*offset), payload);
        }

        // 10. Assemble.
        let mut module = Module::new();
        module.section(&types);
        module.section(&imports);
        module.section(&funcsec);
        module.section(&memsec);
        module.section(&globalsec);
        module.section(&exports);
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

    fn compile_block(&mut self, block: &[crate::ast::Stmt]) -> Result<(), String> {
        for s in block {
            self.compile_stmt(s)?;
        }
        Ok(())
    }

    fn compile_stmt(&mut self, s: &Stmt) -> Result<(), String> {
        use Instruction::*;
        match s {
            Stmt::VarDecl(d) | Stmt::ConstDecl(d) => {
                let declared = d.typ.as_ref().map(ty_of).filter(|t| *t != WTy::Unknown);
                let init_ty = d
                    .init
                    .as_ref()
                    .map(|e| self.infer(e))
                    .filter(|t| *t != WTy::Unknown);
                let ty = declared
                    .or(init_ty)
                    .filter(|t| *t != WTy::Void)
                    .unwrap_or(WTy::I64);
                match &d.init {
                    Some(e) => {
                        let from = self.compile_expr(e)?;
                        self.coerce(from, ty)?;
                    }
                    None => self.zero(ty),
                }
                let idx = self.add_local(ty);
                self.local_map.insert(d.name.clone(), (idx, ty));
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
            Stmt::Switch(sw) => self.compile_switch(sw),
            other => Err(format!("wasm: unsupported statement: {:?}", other)),
        }
    }

    fn compile_switch(&mut self, sw: &crate::ast::SwitchStmt) -> Result<(), String> {
        use Instruction::*;
        let val_ty = self.infer(&sw.expr);
        if val_ty != WTy::I64 && val_ty != WTy::Bool && val_ty != WTy::Str && val_ty != WTy::Char {
            return Err("wasm: switch requires an integer, bool, char or string value".to_string());
        }
        let value = self.compile_expr(&sw.expr)?;
        let slot = self.add_local(value);
        self.push(LocalSet(slot));

        // Outer block gives each matched case a common exit target.
        let exit = self.labels.len() as u32;
        self.labels.push(CtrlLabel::Block);
        self.push(Block(BlockType::Empty));

        let mut emitted_default = false;
        for case in &sw.cases {
            let literal: Option<&Literal> = match &case.pattern {
                Pattern::Literal(l) => Some(l),
                Pattern::Wildcard => None,
                other => return Err(format!("wasm: unsupported switch pattern: {:?}", other)),
            };

            if let Some(lit) = literal {
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
                    _ => return Err("wasm: switch pattern type does not match value".to_string()),
                }
                self.labels.push(CtrlLabel::Block);
                self.push(If(BlockType::Empty));
                self.push_scope();
                self.compile_block(&case.body)?;
                self.pop_scope();
                self.br_to(exit);
                self.push(End);
                self.labels.pop();
            } else {
                self.push_scope();
                self.compile_block(&case.body)?;
                self.pop_scope();
                emitted_default = true;
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

    // ---- expressions --------------------------------------------------------

    fn compile_expr(&mut self, e: &Expr) -> Result<WTy, String> {
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
            Expr::This(_) => {
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
                for (arg, pt) in args.iter().zip(ptypes) {
                    let from = self.compile_expr(arg)?;
                    self.coerce(from, pt)?;
                }
                self.push(Call(func_idx));
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
            other => Err(format!("wasm: unsupported expression: {:?}", other)),
        }
    }

    fn compile_binary(&mut self, l: &Expr, op: &Operator, r: &Expr) -> Result<WTy, String> {
        use Instruction::*;
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
        match callee {
            Expr::Identifier(name, _) => {
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
                    _ => Err(format!("wasm: unknown function '{}'", name)),
                }
            }
            Expr::MemberAccess(obj, method, _) => {
                if let Some(class) = self.class_of_expr(obj) {
                    return self.compile_method_call(&class, obj, method, args);
                }
                if let Expr::Identifier(module, _) = obj.as_ref() {
                    self.compile_named_call(module, method, args)
                } else {
                    Err("wasm: only module/static calls are supported".to_string())
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
        let (param_types, ret, func_idx) = {
            let info = self
                .classes
                .get(class)
                .ok_or_else(|| format!("wasm: unknown class '{}'", class))?;
            let md = info
                .methods
                .iter()
                .find(|m| m.name == method)
                .ok_or_else(|| format!("wasm: class '{}' has no method '{}'", class, method))?;
            let ptypes: Vec<WTy> = md.params.iter().map(|p| ty_of(&p.typ)).collect();
            let r = md.return_type.as_ref().map(ty_of).unwrap_or(WTy::Void);
            let fname = format!("{}::{}", class, method);
            let fi = self
                .functions
                .get(&fname)
                .ok_or_else(|| format!("wasm: missing code for '{}'", fname))?;
            (ptypes, r, fi.func_idx)
        };
        self.compile_expr(obj)?;
        if args.len() != param_types.len() {
            return Err(format!(
                "wasm: method '{}' expects {} args, got {}",
                method,
                param_types.len(),
                args.len()
            ));
        }
        for (arg, pt) in args.iter().zip(param_types) {
            let from = self.compile_expr(arg)?;
            self.coerce(from, pt)?;
        }
        self.push(Instruction::Call(func_idx));
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
