use super::*;
use super::types::{
    literal_type, operator_method_name, is_string_type, is_owned_type, is_result_type,
};

impl Analyzer {
    pub(super) fn infer_expr_type(&self, expr: &ast::Expr, scope: &Rc<RefCell<Scope>>) -> ast::Type {
        match expr {
            ast::Expr::Literal(lit, _) => literal_type(lit),
            ast::Expr::Unit(_) => ast::Type::simple("void"),
            ast::Expr::Identifier(name, _) => {
                match scope.borrow().lookup(name) {
                    Some(Symbol::Variable { typ, .. }) => typ,
                    Some(Symbol::Function(f)) => {
                        // Function as a value 鈥?we return a function type.
                        // For simplicity, return the return type.
                        f.return_type.clone().unwrap_or(ast::Type::simple("void"))
                    }
                    Some(Symbol::Variant { enum_name, .. }) => {
                        ast::Type::simple(&enum_name)
                    }
                    Some(Symbol::Class(c)) => ast::Type::simple(&c.name),
                    Some(Symbol::Enum(e)) => ast::Type::simple(&e.name),
                    Some(Symbol::Interface(i)) => ast::Type::simple(&i.name),
                    None => ast::Type::simple("unknown"),
                }
            }
            ast::Expr::Binary(left, op, _right, _) => {
                let left_type = self.infer_expr_type(left, scope);
                // Check for operator overload 鈥?if found, return the method's return type
                let method_name = operator_method_name(op);
                if !method_name.is_empty() {
                    if let Some(Symbol::Class(class_decl)) = scope.borrow().lookup(left_type.name()) {
                        for member in &class_decl.members {
                            if let ast::ClassMember::Method(m) = member {
                                if m.name == method_name {
                                    return m.return_type.clone().unwrap_or(ast::Type::simple("unknown"));
                                }
                            }
                        }
                    }
                }
                match op {
                    ast::Operator::Add => {
                        if is_string_type(&left_type) {
                            return ast::Type::simple("string");
                        }
                        left_type
                    }
                    ast::Operator::Sub
                    | ast::Operator::Mul
                    | ast::Operator::Div
                    | ast::Operator::Mod => left_type,
                    ast::Operator::Eq
                    | ast::Operator::Ne
                    | ast::Operator::Lt
                    | ast::Operator::Gt
                    | ast::Operator::Le
                    | ast::Operator::Ge => ast::Type::simple("bool"),
                    ast::Operator::And | ast::Operator::Or => ast::Type::simple("bool"),
                    ast::Operator::BitAnd
                    | ast::Operator::BitOr
                    | ast::Operator::BitXor
                    | ast::Operator::BitShl
                    | ast::Operator::BitShr
                    | ast::Operator::BitUshr => left_type,
                }
            }
            ast::Expr::Unary(unop, operand, _) => {
                let operand_type = self.infer_expr_type(operand, scope);
                match unop {
                    ast::UnOp::Neg => operand_type,
                    ast::UnOp::Not => ast::Type::simple("bool"),
                    ast::UnOp::BitNot => operand_type,
                }
            }
            ast::Expr::Call(callee, _args, _) => {
                match callee.as_ref() {
                    ast::Expr::Identifier(name, _) => {
                        if let Some(Symbol::Function(f)) = scope.borrow().lookup(name) {
                            f.return_type.clone().unwrap_or(ast::Type::simple("void"))
                        } else if let Some(Symbol::Variant { enum_name, .. }) = scope.borrow().lookup(name) {
                            ast::Type::simple(&enum_name)
                        } else {
                            ast::Type::simple("unknown")
                        }
                    }
                    ast::Expr::MemberAccess(obj, method, _) => {
                        let obj_type = self.infer_expr_type(obj, scope);
                        if let Some(Symbol::Class(class_decl)) = scope.borrow().lookup(obj_type.name()) {
                            for member in &class_decl.members {
                                match member {
                                    ast::ClassMember::Method(m) if m.name == *method => {
                                        return m.return_type.clone().unwrap_or(ast::Type::simple("void"));
                                    }
                                    ast::ClassMember::Constructor(m) if m.name == *method => {
                                        return m.return_type.clone().unwrap_or(ast::Type::simple("void"));
                                    }
                                    _ => {}
                                }
                            }
                        }
                        // toString on primitives returns string.
                        if method == "toString" {
                            return ast::Type::simple("string");
                        }
                        ast::Type::simple("unknown")
                    }
                    _ => ast::Type::simple("unknown"),
                }
            }
            ast::Expr::MemberAccess(obj, _field, _) => {
                let _obj_type = self.infer_expr_type(obj, scope);
                // Without class field info, return unknown.
                ast::Type::simple("unknown")
            }
            ast::Expr::Index(_obj, _idx, _) => {
                // Without array element type info, return unknown.
                ast::Type::simple("unknown")
            }
            ast::Expr::New(typ, _args, _) => typ.clone(),
            ast::Expr::This(_) => {
                // Look up "this" in scope.
                if let Some(Symbol::Variable { typ, .. }) = scope.borrow().lookup("this") {
                    typ
                } else {
                    ast::Type::simple("unknown")
                }
            }
            ast::Expr::Super(_) => {
                if let Some(Symbol::Variable { typ, .. }) = scope.borrow().lookup("this") {
                    typ
                } else {
                    ast::Type::simple("unknown")
                }
            }
            ast::Expr::OwnedDeref(inner, _) => {
                let inner_type = self.infer_expr_type(inner, scope);
                if is_owned_type(&inner_type) {
                    if let Some(inner_param) = inner_type.params().first() {
                        return inner_param.clone();
                    }
                }
                inner_type
            }
            ast::Expr::RegionAlloc(typ, _region, _) => typ.clone(),
            ast::Expr::RefExpr(inner, ref_kind, _) => {
                let inner_type = self.infer_expr_type(inner, scope);
                match ref_kind {
                    ast::RefKind::Immutable => {
                        ast::Type::Ref(Box::new(inner_type))
                    }
                    ast::RefKind::Mutable => {
                        ast::Type::MutRef(Box::new(inner_type))
                    }
                }
            }
            ast::Expr::UnsafeBlock(block, _) => {
                // Type of an unsafe block is the type of its last expression.
                if let Some(last_stmt) = block.last() {
                    match last_stmt {
                        ast::Stmt::Expr(e) => self.infer_expr_type(e, scope),
                        _ => ast::Type::simple("void"),
                    }
                } else {
                    ast::Type::simple("void")
                }
            }
            ast::Expr::ErrorPropagation(inner, _) => {
                let inner_type = self.infer_expr_type(inner, scope);
                if is_result_type(&inner_type) {
                    if let Some(ok_type) = inner_type.params().first() {
                        return ok_type.clone();
                    }
                }
                inner_type
            }
            ast::Expr::Cast(_inner, target_type, _) => target_type.clone(),
            ast::Expr::StaticCall { .. } => {
                // For toString, returns string.
                ast::Type::simple("string")
            }
            ast::Expr::Assign(_target, value, _) => {
                self.infer_expr_type(value, scope)
            }
            ast::Expr::Ternary { then_expr, else_expr, .. } => {
                let then_type = self.infer_expr_type(then_expr, scope);
                let else_type = self.infer_expr_type(else_expr, scope);
                if then_type == else_type {
                    then_type
                } else {
                    ast::Type::simple("auto")
                }
            }
            ast::Expr::Tuple(elements, _) => {
                let types: Vec<ast::Type> = elements
                    .iter()
                    .map(|e| self.infer_expr_type(e, scope))
                    .collect();
                ast::Type::Tuple(types)
            }
            ast::Expr::Range(_, _, _) => ast::Type::simple("Range"),
            ast::Expr::RangeInclusive(_, _, _) => ast::Type::simple("Range"),
            ast::Expr::Closure { .. } => ast::Type::simple("function"),
        }
    }

    // -----------------------------------------------------------------------
    // Borrow-escape detection
    // -----------------------------------------------------------------------

    /// Check if an expression is a borrow of a local variable.
    pub(super) fn expr_borrows_local(&self, expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::RefExpr(inner, _, _) => {
                if let ast::Expr::Identifier(name, _) = inner.as_ref() {
                    if self.local_vars.contains(name) {
                        return true;
                    }
                }
                false
            }
            ast::Expr::Call(callee, args, _) => {
                // A function call might return a borrow.
                // For Alpha, we check if any argument is a borrow of a local.
                if self.expr_borrows_local(callee) {
                    return true;
                }
                for arg in args {
                    if self.expr_borrows_local(arg) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }
}
