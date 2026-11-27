//! Lowering from the parsed AST into the shared TIR.

use std::collections::{HashMap, HashSet};

use crate::ast::{self, Access, Span};

use super::types::*;

#[derive(Clone)]
enum FunctionSlot {
    Free(ast::FnDecl),
    Method(ClassId, ast::MethodDecl),
}

#[derive(Default)]
struct Lowerer {
    names: NameInterner,
    function_slots: Vec<FunctionSlot>,
    functions: Vec<TirFunction>,
    fn_by_name: HashMap<String, FunctionId>,
    method_by_class: HashMap<(String, String), FunctionId>,
    global_decls: Vec<(ast::VarDecl, bool)>,
    globals: Vec<TirGlobal>,
    class_decls: Vec<ast::ClassDecl>,
    class_by_name: HashMap<String, ClassId>,
    class_order: Vec<ClassId>,
    classes: HashMap<ClassId, ClassLayout>,
    vtables: Vec<VTable>,
    building: HashSet<ClassId>,
    current_capabilities: Capabilities,
}

impl Lowerer {
    fn new() -> Self {
        Self::default()
    }

    fn next_function_id(&self) -> Result<FunctionId, String> {
        let raw = u16::try_from(self.function_slots.len())
            .map_err(|_| "too many functions to lower (u16 id space exhausted)".to_string())?;
        Ok(FunctionId::new(raw))
    }

    fn next_class_id(&self) -> Result<ClassId, String> {
        let raw = u16::try_from(self.class_decls.len())
            .map_err(|_| "too many classes to lower (u16 id space exhausted)".to_string())?;
        Ok(ClassId::new(raw))
    }

    fn register(&mut self, program: &ast::Program) -> Result<(), String> {
        for declaration in &program.declarations {
            match declaration {
                ast::Declaration::Function(f) => {
                    let id = self.next_function_id()?;
                    if self.fn_by_name.insert(f.name.clone(), id).is_some() {
                        return Err(format!("duplicate function '{}'", f.name));
                    }
                    self.function_slots.push(FunctionSlot::Free(f.clone()));
                }
                ast::Declaration::Class(class) => {
                    let id = self.next_class_id()?;
                    if self.class_by_name.insert(class.name.clone(), id).is_some() {
                        return Err(format!("duplicate class '{}'", class.name));
                    }
                    self.class_decls.push(class.clone());
                    self.class_order.push(id);
                    for member in &class.members {
                        if let ast::ClassMember::Method(method) = member {
                            let method_id = self.next_function_id()?;
                            if self
                                .method_by_class
                                .insert((class.name.clone(), method.name.clone()), method_id)
                                .is_some()
                            {
                                return Err(format!(
                                    "duplicate method '{}' on class '{}'",
                                    method.name, class.name
                                ));
                            }
                            self.function_slots
                                .push(FunctionSlot::Method(id, method.clone()));
                        }
                    }
                }
                ast::Declaration::VarDecl(var) => self.global_decls.push((var.clone(), false)),
                ast::Declaration::ConstDecl(var) => self.global_decls.push((var.clone(), true)),
                other => {
                    return Err(format!(
                        "TIR lowering does not support {} declaration at {}",
                        declaration_kind(other),
                        declaration_span(other)
                    ));
                }
            }
        }
        Ok(())
    }

    fn lower_bodies(&mut self) -> Result<(), String> {
        let slots = self.function_slots.clone();
        for (index, slot) in slots.into_iter().enumerate() {
            let id = FunctionId::new(
                u16::try_from(index)
                    .map_err(|_| "too many functions to lower".to_string())?,
            );
            let function = self.lower_function(id, slot)?;
            self.functions.push(function);
        }

        let globals = self.global_decls.clone();
        for (decl, is_const) in globals {
            let name = self.names.intern(&decl.name)?;
            let typ = decl.typ.as_ref().map(|t| self.lower_type(t)).transpose()?;
            let init = decl.init.as_ref().map(|e| self.lower_expr(e)).transpose()?;
            self.globals.push(TirGlobal {
                name,
                typ,
                init,
                is_const,
                is_public: false,
            });
        }
        Ok(())
    }

    fn lower_function(&mut self, id: FunctionId, slot: FunctionSlot) -> Result<TirFunction, String> {
        let (raw_name, type_params, params, return_type, body, access) = match slot {
            FunctionSlot::Free(decl) => (
                decl.name,
                decl.type_params,
                decl.params,
                decl.return_type,
                decl.body,
                decl.access,
            ),
            FunctionSlot::Method(class_id, decl) => {
                let class_name = self.class_decls[class_id.as_u16() as usize].name.clone();
                (
                    format!("{}.{}", class_name, decl.name),
                    decl.type_params,
                    decl.params,
                    decl.return_type,
                    decl.body,
                    decl.access,
                )
            }
        };

        let name = self.names.intern(&raw_name)?;
        let type_params = type_params
            .iter()
            .map(|param| self.names.intern(&param.name))
            .collect::<Result<Vec<_>, _>>()?;
        let params = params
            .iter()
            .map(|param| {
                Ok(TirParam {
                    name: self.names.intern(&param.name)?,
                    typ: self.lower_type(&param.typ)?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let return_type = return_type.as_ref().map(|t| self.lower_type(t)).transpose()?;
        let body = self.lower_block(&body)?;
        let capability = std::mem::replace(&mut self.current_capabilities, Capabilities::NONE);
        let simd = if has_loop(&body) {
            SimdHint::Allow
        } else {
            SimdHint::None
        };

        Ok(TirFunction {
            id,
            name,
            type_params,
            params,
            return_type,
            body,
            capability,
            simd,
            is_public: access == Access::Public,
        })
    }

    fn lower_type(&mut self, ty: &ast::Type) -> Result<TirType, String> {
        Ok(match ty {
            ast::Type::Named { name, params } => TirType::Named {
                name: self.names.intern(name)?,
                params: params
                    .iter()
                    .map(|param| self.lower_type(param))
                    .collect::<Result<Vec<_>, _>>()?,
            },
            ast::Type::Ref(inner) => TirType::Ref(Box::new(self.lower_type(inner)?)),
            ast::Type::MutRef(inner) => TirType::MutRef(Box::new(self.lower_type(inner)?)),
            ast::Type::Tuple(items) => TirType::Tuple(
                items
                    .iter()
                    .map(|item| self.lower_type(item))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        })
    }

    fn lower_block(&mut self, block: &ast::Block) -> Result<Vec<TirStmt>, String> {
        block.iter().map(|stmt| self.lower_stmt(stmt)).collect()
    }

    fn lower_stmt(&mut self, stmt: &ast::Stmt) -> Result<TirStmt, String> {
        Ok(match stmt {
            ast::Stmt::Block(block) => TirStmt::Block(self.lower_block(block)?),
            ast::Stmt::Expr(expr) => TirStmt::Expr(self.lower_expr(expr)?),
            ast::Stmt::VarDecl(decl) => TirStmt::Let {
                name: self.names.intern(&decl.name)?,
                typ: decl.typ.as_ref().map(|t| self.lower_type(t)).transpose()?,
                init: decl.init.as_ref().map(|e| self.lower_expr(e)).transpose()?,
                mutable: decl.mutable,
                span: decl.span,
            },
            ast::Stmt::ConstDecl(decl) => TirStmt::Let {
                name: self.names.intern(&decl.name)?,
                typ: decl.typ.as_ref().map(|t| self.lower_type(t)).transpose()?,
                init: decl.init.as_ref().map(|e| self.lower_expr(e)).transpose()?,
                mutable: false,
                span: decl.span,
            },
            ast::Stmt::Return(value) => {
                TirStmt::Return(value.as_ref().map(|e| self.lower_expr(e)).transpose()?)
            }
            ast::Stmt::Break => TirStmt::Break,
            ast::Stmt::Continue => TirStmt::Continue,
            ast::Stmt::If(stmt) => TirStmt::If {
                cond: self.lower_expr(&stmt.condition)?,
                then_branch: self.lower_block(&stmt.then_branch)?,
                else_branch: stmt
                    .else_branch
                    .as_ref()
                    .map(|block| self.lower_block(block))
                    .transpose()?,
                span: stmt.span,
            },
            ast::Stmt::While(stmt) => TirStmt::While {
                cond: self.lower_expr(&stmt.condition)?,
                body: self.lower_block(&stmt.body)?,
                simd: SimdHint::None,
                span: stmt.span,
            },
            ast::Stmt::For(stmt) => TirStmt::For {
                var: self.names.intern(&stmt.var)?,
                iterable: self.lower_expr(&stmt.iterable)?,
                body: self.lower_block(&stmt.body)?,
                simd: SimdHint::Allow,
                span: stmt.span,
            },
            other => {
                return Err(format!(
                    "TIR lowering does not support {} statement at {}",
                    statement_kind(other),
                    statement_span(other)
                ));
            }
        })
    }

    fn lower_expr(&mut self, expr: &ast::Expr) -> Result<TirExpr, String> {
        Ok(match expr {
            ast::Expr::Literal(value, span) => TirExpr::Literal(value.clone(), *span),
            ast::Expr::Identifier(name, span) => TirExpr::Var(self.names.intern(name)?, *span),
            ast::Expr::This(span) => TirExpr::This(*span),
            ast::Expr::Unit(span) => TirExpr::Unit(*span),
            ast::Expr::Unary(op, operand, span) => TirExpr::Unary {
                op: op.clone(),
                operand: Box::new(self.lower_expr(operand)?),
                span: *span,
            },
            ast::Expr::Binary(left, op, right, span) => TirExpr::Binary {
                op: op.clone(),
                left: Box::new(self.lower_expr(left)?),
                right: Box::new(self.lower_expr(right)?),
                span: *span,
            },
            ast::Expr::Assign(target, value, span) => {
                self.current_capabilities.insert(Capabilities::MUTATION);
                TirExpr::Assign {
                    target: Box::new(self.lower_expr(target)?),
                    value: Box::new(self.lower_expr(value)?),
                    span: *span,
                }
            }
            ast::Expr::Cast(value, ty, span) => TirExpr::Cast {
                value: Box::new(self.lower_expr(value)?),
                typ: self.lower_type(ty)?,
                span: *span,
            },
            ast::Expr::Is(value, ty, span) => TirExpr::Is {
                value: Box::new(self.lower_expr(value)?),
                typ: self.lower_type(ty)?,
                span: *span,
            },
            ast::Expr::RefExpr(value, kind, span) => TirExpr::Ref {
                value: Box::new(self.lower_expr(value)?),
                kind: kind.clone(),
                span: *span,
            },
            ast::Expr::Call(callee, args, type_args, span) => {
                let target = self.resolve_call(callee)?;
                self.note_target_effects(&target);
                TirExpr::Call {
                    target,
                    type_args: type_args
                        .iter()
                        .map(|ty| self.lower_type(ty))
                        .collect::<Result<Vec<_>, _>>()?,
                    args: args
                        .iter()
                        .map(|arg| self.lower_expr(arg))
                        .collect::<Result<Vec<_>, _>>()?,
                    span: *span,
                }
            }
            ast::Expr::StaticCall { class_name, method, args, span } => {
                let target = if let Some(&id) =
                    self.method_by_class.get(&(class_name.clone(), method.clone()))
                {
                    CallTarget::Direct(id)
                } else {
                    CallTarget::External { name: self.names.intern(method)? }
                };
                TirExpr::Call {
                    target,
                    type_args: Vec::new(),
                    args: args
                        .iter()
                        .map(|arg| self.lower_expr(arg))
                        .collect::<Result<Vec<_>, _>>()?,
                    span: *span,
                }
            }
            ast::Expr::MemberAccess(object, field, span) => {
                let field_id = self.names.intern(field)?;
                let object = self.lower_expr(object)?;
                let slot = match &object {
                    TirExpr::New { class, .. } => {
                        let class_name = self.names.get(*class).map(|name| name.to_string());
                        match class_name.and_then(|name| self.class_by_name.get(&name).copied()) {
                            Some(class_id) => {
                                self.ensure_layout(class_id)?;
                                self.classes[&class_id]
                                    .fields
                                    .iter()
                                    .find(|slot| slot.name == field_id)
                                    .map(|slot| slot.slot)
                            }
                            None => None,
                        }
                    }
                    _ => None,
                };
                TirExpr::Field {
                    object: Box::new(object),
                    field: field_id,
                    slot,
                    span: *span,
                }
            }
            ast::Expr::New(ty, args, span) => TirExpr::New {
                class: self.names.intern(ty.name())?,
                args: args
                    .iter()
                    .map(|arg| self.lower_expr(arg))
                    .collect::<Result<Vec<_>, _>>()?,
                span: *span,
            },
            ast::Expr::Tuple(items, span) => TirExpr::Tuple {
                items: items
                    .iter()
                    .map(|item| self.lower_expr(item))
                    .collect::<Result<Vec<_>, _>>()?,
                span: *span,
            },
            ast::Expr::Ternary { condition, then_expr, else_expr, span } => TirExpr::Ternary {
                condition: Box::new(self.lower_expr(condition)?),
                then_expr: Box::new(self.lower_expr(then_expr)?),
                else_expr: Box::new(self.lower_expr(else_expr)?),
                span: *span,
            },
            other => {
                return Err(format!(
                    "TIR lowering does not support expression at {}: {:?}",
                    expression_span(other),
                    other
                ));
            }
        })
    }

    fn resolve_call(&mut self, callee: &ast::Expr) -> Result<CallTarget, String> {
        match callee {
            ast::Expr::Identifier(name, _) => {
                if let Some(&id) = self.fn_by_name.get(name) {
                    return Ok(CallTarget::Direct(id));
                }
                Ok(CallTarget::External { name: self.names.intern(name)? })
            }
            ast::Expr::MemberAccess(base, method, _) => {
                let method_id = self.names.intern(method)?;
                if let ast::Expr::Identifier(qualifier, _) = &**base {
                    if let Some(&id) = self.method_by_class.get(&(qualifier.clone(), method.clone()))
                    {
                        return Ok(CallTarget::Direct(id));
                    }
                    // shortcut: module/native names are distinguished from user names
                    // by a separator; full module resolution belongs to a later pass.
                    let native = format!("{}.{}", qualifier, method);
                    return Ok(CallTarget::Native { name: self.names.intern(&native)? });
                }
                if let ast::Expr::New(ty, _, _) = &**base {
                    if let Some(&class_id) = self.class_by_name.get(ty.name()) {
                        self.ensure_layout(class_id)?;
                        let vtable_id = self.classes[&class_id].vtable;
                        let vtable = &self.vtables[vtable_id.as_u16() as usize];
                        if let Some(index) = vtable.entries.iter().position(|e| e.method == method_id)
                        {
                            return Ok(CallTarget::Virtual {
                                vtable: vtable_id,
                                slot: u16::try_from(index)
                                    .map_err(|_| "vtable slot overflow".to_string())?,
                            });
                        }
                    }
                }
                Ok(CallTarget::External { name: method_id })
            }
            other => Err(format!("unsupported call callee at {}: {:?}", expression_span(other), other)),
        }
    }

    fn note_target_effects(&mut self, target: &CallTarget) {
        let name = match target {
            CallTarget::Native { name } | CallTarget::External { name } => {
                match self.names.get(*name) {
                    Some(name) => name.to_string(),
                    None => return,
                }
            }
            CallTarget::Direct(_) | CallTarget::Virtual { .. } => return,
        };
        note_effects(&name, &mut self.current_capabilities);
    }

    fn ensure_layout(&mut self, id: ClassId) -> Result<(), String> {
        if self.classes.contains_key(&id) {
            return Ok(());
        }
        if !self.building.insert(id) {
            return Err(format!(
                "cyclic inheritance involving class '{}'",
                self.class_decls[id.as_u16() as usize].name
            ));
        }

        let decl = self.class_decls[id.as_u16() as usize].clone();
        let parent = match decl.parent.as_ref() {
            Some(ty) => {
                let parent_name = ty.name();
                let parent_id = *self.class_by_name.get(parent_name).ok_or_else(|| {
                    format!("class '{}' extends unknown class '{}'", decl.name, parent_name)
                })?;
                self.ensure_layout(parent_id)?;
                Some(parent_id)
            }
            None => None,
        };
        let (parent_slots, mut offset) = match parent {
            Some(parent_id) => {
                let layout = &self.classes[&parent_id];
                (layout.total_slots, layout.size_bytes)
            }
            None => (0, POINTER_SIZE),
        };
        let mut alignment = POINTER_SIZE;
        let mut fields = Vec::new();
        let mut seen = HashSet::new();

        for member in &decl.members {
            if let ast::ClassMember::Field(field) = member {
                if !seen.insert(field.name.clone()) {
                    return Err(format!(
                        "duplicate field '{}' in class '{}'",
                        field.name, decl.name
                    ));
                }
                let typ = self.lower_type(&field.typ)?;
                let (size, field_align) = typ.size_align(&self.names)?;
                if size == 0 {
                    return Err(format!(
                        "field '{}' in class '{}' has a zero-sized type",
                        field.name, decl.name
                    ));
                }
                offset = align_up(offset, field_align);
                let own_index = u16::try_from(fields.len())
                    .map_err(|_| format!("too many fields in class '{}'", decl.name))?;
                let slot = parent_slots
                    .checked_add(own_index)
                    .ok_or_else(|| format!("too many fields in class '{}'", decl.name))?;
                fields.push(FieldSlot {
                    name: self.names.intern(&field.name)?,
                    typ,
                    slot,
                    offset,
                });
                offset += size;
                alignment = alignment.max(field_align);
            }
        }

        let own_slots = u16::try_from(fields.len())
            .map_err(|_| format!("too many fields in class '{}'", decl.name))?;
        let total_slots = parent_slots
            .checked_add(own_slots)
            .ok_or_else(|| format!("too many fields in class '{}'", decl.name))?;
        let vtable = self.build_vtable(&decl)?;
        let name = self.names.intern(&decl.name)?;
        self.classes.insert(
            id,
            ClassLayout {
                name,
                parent,
                fields,
                total_slots,
                size_bytes: align_up(offset, alignment),
                vtable,
            },
        );
        self.building.remove(&id);
        Ok(())
    }

    fn build_vtable(&mut self, decl: &ast::ClassDecl) -> Result<VTableId, String> {
        let class = self.names.intern(&decl.name)?;
        let mut entries = match decl
            .parent
            .as_ref()
            .and_then(|ty| self.class_by_name.get(ty.name()).copied())
        {
            Some(parent_id) => {
                let parent_vtable = self.classes[&parent_id].vtable;
                self.vtables[parent_vtable.as_u16() as usize].entries.clone()
            }
            None => Vec::new(),
        };

        let mut methods: Vec<&ast::MethodDecl> = decl
            .members
            .iter()
            .filter_map(|member| match member {
                ast::ClassMember::Method(method) => Some(method),
                _ => None,
            })
            .collect();
        methods.sort_by(|left, right| left.name.cmp(&right.name));
        for method in methods {
            let name = self.names.intern(&method.name)?;
            let target = *self
                .method_by_class
                .get(&(decl.name.clone(), method.name.clone()))
                .ok_or_else(|| {
                    format!("missing lowered method '{}' on '{}'", method.name, decl.name)
                })?;
            if let Some(slot) = entries.iter_mut().find(|entry| entry.method == name) {
                slot.target = target;
            } else {
                entries.push(VTableSlot { method: name, target });
            }
        }

        let raw = u16::try_from(self.vtables.len())
            .map_err(|_| "too many vtables to lower".to_string())?;
        self.vtables.push(VTable { class, entries });
        Ok(VTableId::new(raw))
    }

    fn finish(self) -> TirProgram {
        let classes = self
            .class_order
            .iter()
            .map(|id| self.classes[id].clone())
            .collect();
        TirProgram {
            names: self.names,
            functions: self.functions,
            globals: self.globals,
            classes,
            vtables: self.vtables,
        }
    }
}

fn has_loop(statements: &[TirStmt]) -> bool {
    statements.iter().any(|stmt| match stmt {
        TirStmt::While { .. } | TirStmt::For { .. } => true,
        TirStmt::Block(body) => has_loop(body),
        TirStmt::If { then_branch, else_branch, .. } => {
            has_loop(then_branch)
                || else_branch.as_ref().is_some_and(|body| has_loop(body))
        }
        _ => false,
    })
}

fn note_effects(name: &str, capabilities: &mut Capabilities) {
    if name == "print" || name == "println" || name.starts_with("IO_") {
        capabilities.insert(Capabilities::IO);
    }
    if name.starts_with("File_") || name.starts_with("Dir_") || name.starts_with("Path_") {
        capabilities.insert(Capabilities::FILE_SYSTEM);
    }
    if name.starts_with("Net_") {
        capabilities.insert(Capabilities::NETWORK);
    }
    if name.starts_with("Time_") {
        capabilities.insert(Capabilities::TIME);
    }
    if name.starts_with("Random_") {
        capabilities.insert(Capabilities::RANDOM);
    }
    if name.contains('_') || name.contains('.') {
        capabilities.insert(Capabilities::NATIVE);
    }
}

fn declaration_kind(declaration: &ast::Declaration) -> &'static str {
    match declaration {
        ast::Declaration::Function(_) => "function",
        ast::Declaration::Class(_) => "class",
        ast::Declaration::Interface(_) => "interface",
        ast::Declaration::Enum(_) => "enum",
        ast::Declaration::VarDecl(_) => "var",
        ast::Declaration::ConstDecl(_) => "const",
    }
}

fn declaration_span(declaration: &ast::Declaration) -> Span {
    match declaration {
        ast::Declaration::Function(decl) => decl.span,
        ast::Declaration::Class(decl) => decl.span,
        ast::Declaration::Interface(decl) => decl.span,
        ast::Declaration::Enum(decl) => decl.span,
        ast::Declaration::VarDecl(decl) => decl.span,
        ast::Declaration::ConstDecl(decl) => decl.span,
    }
}

fn statement_kind(statement: &ast::Stmt) -> &'static str {
    match statement {
        ast::Stmt::DoWhile(_) => "do-while",
        ast::Stmt::WhileLet(_) => "while-let",
        ast::Stmt::CFor(_) => "C-style for",
        ast::Stmt::Switch(_) => "switch",
        ast::Stmt::With(_) => "with",
        ast::Stmt::TupleDestructure { .. } => "tuple destructure",
        ast::Stmt::Throw(..) => "throw",
        ast::Stmt::TryCatch { .. } => "try/catch",
        _ => "other",
    }
}

fn statement_span(statement: &ast::Stmt) -> Span {
    match statement {
        ast::Stmt::DoWhile(stmt) => stmt.span,
        ast::Stmt::WhileLet(stmt) => stmt.span,
        ast::Stmt::CFor(stmt) => stmt.span,
        ast::Stmt::Switch(stmt) => stmt.span,
        ast::Stmt::With(stmt) => stmt.span,
        ast::Stmt::TupleDestructure { span, .. } => *span,
        ast::Stmt::Throw(_, span) => *span,
        ast::Stmt::TryCatch { span, .. } => *span,
        _ => Span::unknown(),
    }
}

fn expression_span(expression: &ast::Expr) -> Span {
    match expression {
        ast::Expr::Literal(_, span)
        | ast::Expr::Identifier(_, span)
        | ast::Expr::Binary(_, _, _, span)
        | ast::Expr::Unary(_, _, span)
        | ast::Expr::Call(_, _, _, span)
        | ast::Expr::MemberAccess(_, _, span)
        | ast::Expr::Index(_, _, span)
        | ast::Expr::New(_, _, span)
        | ast::Expr::This(span)
        | ast::Expr::Super(span)
        | ast::Expr::OwnedDeref(_, span)
        | ast::Expr::RegionAlloc(_, _, span)
        | ast::Expr::RefExpr(_, _, span)
        | ast::Expr::UnsafeBlock(_, span)
        | ast::Expr::RegionBlock(_, span)
        | ast::Expr::ErrorPropagation(_, span)
        | ast::Expr::Cast(_, _, span)
        | ast::Expr::Is(_, _, span)
        | ast::Expr::Assign(_, _, span)
        | ast::Expr::Ternary { span, .. }
        | ast::Expr::Range(_, _, span)
        | ast::Expr::RangeInclusive(_, _, span)
        | ast::Expr::Unit(span)
        | ast::Expr::Tuple(_, span)
        | ast::Expr::Closure { span, .. } => *span,
        ast::Expr::StaticCall { span, .. } => *span,
    }
}

/// Lower a parsed program into the shared TIR. Unsupported constructs produce
/// an error naming the construct and its source location.
pub fn lower(program: &ast::Program) -> Result<TirProgram, String> {
    let mut lowerer = Lowerer::new();
    lowerer.register(program)?;
    lowerer.lower_bodies()?;
    let order = lowerer.class_order.clone();
    for id in order {
        lowerer.ensure_layout(id)?;
    }
    Ok(lowerer.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span() -> Span {
        Span::new(1, 1)
    }

    fn field(name: &str, typ: ast::Type) -> ast::ClassMember {
        ast::ClassMember::Field(ast::FieldDecl {
            access: Access::Public,
            name: name.to_string(),
            typ,
            init: None,
            span: span(),
        })
    }

    fn method(name: &str, return_type: ast::Type) -> ast::ClassMember {
        ast::ClassMember::Method(ast::MethodDecl {
            access: Access::Public,
            name: name.to_string(),
            type_params: vec![],
            params: vec![],
            return_type: Some(return_type),
            body: vec![],
            where_clause: vec![],
            span: span(),
        })
    }

    fn class(
        name: &str,
        parent: Option<ast::Type>,
        members: Vec<ast::ClassMember>,
    ) -> ast::Declaration {
        ast::Declaration::Class(ast::ClassDecl {
            name: name.to_string(),
            type_params: vec![],
            parent,
            ifaces: vec![],
            members,
            span: span(),
        })
    }

    fn function(name: &str, body: ast::Block) -> ast::Declaration {
        ast::Declaration::Function(ast::FnDecl {
            access: Access::Public,
            name: name.to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body,
            sugar: false,
            where_clause: vec![],
            span: span(),
        })
    }

    fn call(name: &str, type_args: Vec<ast::Type>) -> ast::Expr {
        ast::Expr::Call(
            Box::new(ast::Expr::Identifier(name.to_string(), span())),
            vec![],
            type_args,
            span(),
        )
    }

    #[test]
    fn lowers_class_layout_and_vtable() {
        let program = ast::Program {
            imports: vec![],
            declarations: vec![class(
                "Point",
                None,
                vec![
                    field("x", ast::Type::simple("int")),
                    field("y", ast::Type::simple("double")),
                    method("area", ast::Type::simple("double")),
                ],
            )],
        };

        let tir = lower(&program).unwrap();
        assert_eq!(tir.classes.len(), 1);
        let layout = &tir.classes[0];
        assert_eq!(tir.name(layout.name), Some("Point"));
        assert_eq!(layout.fields.len(), 2);
        assert_eq!(layout.fields[0].offset, 8);
        assert_eq!(layout.fields[1].offset, 16);
        assert_eq!(layout.total_slots, 2);
        assert_eq!(layout.size_bytes, 24);
        assert_eq!(tir.name(layout.fields[0].name), Some("x"));
        assert_eq!(tir.name(layout.fields[1].name), Some("y"));

        let vtable = tir.vtable(layout.vtable);
        assert_eq!(vtable.entries.len(), 1);
        assert_eq!(tir.name(vtable.entries[0].method), Some("area"));
        assert_eq!(tir.name(tir.function(vtable.entries[0].target).name), Some("Point.area"));
    }

    #[test]
    fn inherits_field_slots_and_vtable_entries() {
        let program = ast::Program {
            imports: vec![],
            declarations: vec![
                class("Base", None, vec![field("a", ast::Type::simple("int"))]),
                class(
                    "Derived",
                    Some(ast::Type::simple("Base")),
                    vec![field("b", ast::Type::simple("int"))],
                ),
            ],
        };

        let tir = lower(&program).unwrap();
        let derived = &tir.classes[1];
        assert_eq!(derived.parent, Some(ClassId::new(0)));
        assert_eq!(derived.fields[0].slot, 1);
        assert_eq!(derived.fields[0].offset, 16);
        assert_eq!(derived.total_slots, 2);
    }

    #[test]
    fn lowers_explicit_call_type_args() {
        let program = ast::Program {
            imports: vec![],
            declarations: vec![function(
                "main",
                vec![ast::Stmt::Expr(call(
                    "map",
                    vec![ast::Type::simple("int"), ast::Type::simple("string")],
                ))],
            )],
        };

        let tir = lower(&program).unwrap();
        let call_expr = match &tir.functions[0].body[0] {
            TirStmt::Expr(expr) => expr,
            _ => panic!("expected expression statement"),
        };
        match call_expr {
            TirExpr::Call { target, type_args, .. } => {
                assert_eq!(type_args.len(), 2);
                for (arg, expected) in type_args.iter().zip(["int", "string"]) {
                    match arg {
                        TirType::Named { name, .. } => {
                            assert_eq!(tir.name(*name), Some(expected));
                        }
                        _ => panic!("expected named type"),
                    }
                }
                assert!(matches!(target, CallTarget::External { .. }));
            }
            _ => panic!("expected call"),
        }
    }

    #[test]
    fn resolves_local_calls_and_records_capabilities() {
        let program = ast::Program {
            imports: vec![],
            declarations: vec![
                function("helper", vec![]),
                function(
                    "main",
                    vec![
                        ast::Stmt::Expr(call("helper", vec![])),
                        ast::Stmt::Expr(ast::Expr::Call(
                            Box::new(ast::Expr::Identifier("println".to_string(), span())),
                            vec![ast::Expr::Literal(ast::Literal::String("hi".to_string()), span())],
                            vec![],
                            span(),
                        )),
                    ],
                ),
            ],
        };

        let tir = lower(&program).unwrap();
        assert!(tir.functions[0].capability.is_pure());
        assert!(tir.functions[1].capability.contains(Capabilities::IO));
        assert!(!tir.functions[1].capability.contains(Capabilities::NATIVE));
        let target = match &tir.functions[1].body[0] {
            TirStmt::Expr(TirExpr::Call { target, .. }) => target,
            _ => panic!("expected call"),
        };
        assert_eq!(target, &CallTarget::Direct(FunctionId::new(0)));
    }

    #[test]
    fn resolves_field_slots_on_construction() {
        let access = ast::Expr::MemberAccess(
            Box::new(ast::Expr::New(ast::Type::simple("Point"), vec![], span())),
            "y".to_string(),
            span(),
        );
        let program = ast::Program {
            imports: vec![],
            declarations: vec![
                class(
                    "Point",
                    None,
                    vec![
                        field("x", ast::Type::simple("int")),
                        field("y", ast::Type::simple("double")),
                    ],
                ),
                function("main", vec![ast::Stmt::Expr(access)]),
            ],
        };

        let tir = lower(&program).unwrap();
        match &tir.functions[0].body[0] {
            TirStmt::Expr(TirExpr::Field { slot, .. }) => assert_eq!(*slot, Some(1)),
            _ => panic!("expected field access"),
        }
    }

    #[test]
    fn rejects_unsupported_statements() {
        let program = ast::Program {
            imports: vec![],
            declarations: vec![function(
                "main",
                vec![ast::Stmt::Throw(
                    ast::Expr::Literal(ast::Literal::String("boom".to_string()), span()),
                    span(),
                )],
            )],
        };

        let error = lower(&program).unwrap_err();
        assert!(error.contains("throw"), "unexpected error: {}", error);
    }
}
