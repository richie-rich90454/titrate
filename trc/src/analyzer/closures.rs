use super::*;

impl Analyzer {

    pub(super) fn collect_captured_vars(
        &self,
        expr: &Option<Box<ast::Expr>>,
        param_names: &[String],
        outer_scope: &Rc<RefCell<Scope>>,
        captured: &mut Vec<String>,
    ) {
        if let Some(e) = expr {
            self.collect_captured_vars_from_expr(e, param_names, outer_scope, captured);
        }
    }

    pub(super) fn collect_captured_vars_from_expr(
        &self,
        expr: &ast::Expr,
        param_names: &[String],
        outer_scope: &Rc<RefCell<Scope>>,
        captured: &mut Vec<String>,
    ) {
        match expr {
            ast::Expr::Identifier(name, _) => {
                if !param_names.contains(name) && outer_scope.borrow().lookup(name).is_some() {
                    if let Some(Symbol::Variable { .. }) = outer_scope.borrow().lookup(name) {
                        captured.push(name.clone());
                    }
                }
            }
            ast::Expr::Binary(left, _, right, _) => {
                self.collect_captured_vars_from_expr(left, param_names, outer_scope, captured);
                self.collect_captured_vars_from_expr(right, param_names, outer_scope, captured);
            }
            ast::Expr::Unary(_, operand, _) => {
                self.collect_captured_vars_from_expr(operand, param_names, outer_scope, captured);
            }
            ast::Expr::Call(callee, args, _) => {
                self.collect_captured_vars_from_expr(callee, param_names, outer_scope, captured);
                for arg in args {
                    self.collect_captured_vars_from_expr(arg, param_names, outer_scope, captured);
                }
            }
            ast::Expr::MemberAccess(obj, _, _) => {
                self.collect_captured_vars_from_expr(obj, param_names, outer_scope, captured);
            }
            ast::Expr::Index(obj, index, _) => {
                self.collect_captured_vars_from_expr(obj, param_names, outer_scope, captured);
                self.collect_captured_vars_from_expr(index, param_names, outer_scope, captured);
            }
            ast::Expr::Assign(target, value, _) => {
                self.collect_captured_vars_from_expr(target, param_names, outer_scope, captured);
                self.collect_captured_vars_from_expr(value, param_names, outer_scope, captured);
            }
            ast::Expr::Ternary { condition, then_expr, else_expr, .. } => {
                self.collect_captured_vars_from_expr(condition, param_names, outer_scope, captured);
                self.collect_captured_vars_from_expr(then_expr, param_names, outer_scope, captured);
                self.collect_captured_vars_from_expr(else_expr, param_names, outer_scope, captured);
            }
            ast::Expr::New(_, args, _) => {
                for arg in args {
                    self.collect_captured_vars_from_expr(arg, param_names, outer_scope, captured);
                }
            }
            ast::Expr::OwnedDeref(inner, _) | ast::Expr::ErrorPropagation(inner, _) | ast::Expr::Cast(inner, _, _) => {
                self.collect_captured_vars_from_expr(inner, param_names, outer_scope, captured);
            }
            ast::Expr::RefExpr(inner, _, _) => {
                self.collect_captured_vars_from_expr(inner, param_names, outer_scope, captured);
            }
            ast::Expr::RegionAlloc(_, init, _) => {
                self.collect_captured_vars_from_expr(init, param_names, outer_scope, captured);
            }
            ast::Expr::UnsafeBlock(block, _) => {
                for stmt in block {
                    self.collect_captured_vars_from_stmt(stmt, param_names, outer_scope, captured);
                }
            }
            ast::Expr::StaticCall { args, .. } => {
                for arg in args {
                    self.collect_captured_vars_from_expr(arg, param_names, outer_scope, captured);
                }
            }
            ast::Expr::Closure { body, expr: closure_expr, .. } => {
                if let Some(ref e) = closure_expr {
                    self.collect_captured_vars_from_expr(e, param_names, outer_scope, captured);
                }
                for stmt in body {
                    self.collect_captured_vars_from_stmt(stmt, param_names, outer_scope, captured);
                }
            }
            ast::Expr::Range(start, end, _) => {
                self.collect_captured_vars_from_expr(start, param_names, outer_scope, captured);
                self.collect_captured_vars_from_expr(end, param_names, outer_scope, captured);
            }
            ast::Expr::RangeInclusive(start, end, _) => {
                self.collect_captured_vars_from_expr(start, param_names, outer_scope, captured);
                self.collect_captured_vars_from_expr(end, param_names, outer_scope, captured);
            }
            _ => {}
        }
    }

    pub(super) fn collect_captured_vars_from_stmt(
        &self,
        stmt: &ast::Stmt,
        param_names: &[String],
        outer_scope: &Rc<RefCell<Scope>>,
        captured: &mut Vec<String>,
    ) {
        match stmt {
            ast::Stmt::Expr(expr) => {
                self.collect_captured_vars_from_expr(expr, param_names, outer_scope, captured);
            }
            ast::Stmt::VarDecl(vd) | ast::Stmt::ConstDecl(vd) => {
                if let Some(ref init) = vd.init {
                    self.collect_captured_vars_from_expr(init, param_names, outer_scope, captured);
                }
            }
            ast::Stmt::If(if_stmt) => {
                self.collect_captured_vars_from_expr(&if_stmt.condition, param_names, outer_scope, captured);
                for s in &if_stmt.then_branch {
                    self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                }
                if let Some(ref else_branch) = if_stmt.else_branch {
                    for s in else_branch {
                        self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                    }
                }
            }
            ast::Stmt::While(ws) => {
                self.collect_captured_vars_from_expr(&ws.condition, param_names, outer_scope, captured);
                for s in &ws.body {
                    self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                }
            }
            ast::Stmt::DoWhile(dw) => {
                for s in &dw.body {
                    self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                }
                self.collect_captured_vars_from_expr(&dw.condition, param_names, outer_scope, captured);
            }
            ast::Stmt::For(fs) => {
                self.collect_captured_vars_from_expr(&fs.iterable, param_names, outer_scope, captured);
                for s in &fs.body {
                    self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                }
            }
            ast::Stmt::Return(expr) => {
                if let Some(ref e) = expr {
                    self.collect_captured_vars_from_expr(e, param_names, outer_scope, captured);
                }
            }
            ast::Stmt::Block(block) => {
                for s in block {
                    self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                }
            }
            ast::Stmt::Switch(ss) => {
                self.collect_captured_vars_from_expr(&ss.expr, param_names, outer_scope, captured);
                for case in &ss.cases {
                    for s in &case.body {
                        self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                    }
                }
                if let Some(ref default) = ss.default {
                    for s in default {
                        self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                    }
                }
            }
            ast::Stmt::WhileLet(wls) => {
                self.collect_captured_vars_from_expr(&wls.expr, param_names, outer_scope, captured);
                for s in &wls.body {
                    self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                }
            }
            ast::Stmt::CFor(cfs) => {
                if let Some(ref init) = cfs.init {
                    self.collect_captured_vars_from_stmt(init, param_names, outer_scope, captured);
                }
                if let Some(ref cond) = cfs.condition {
                    self.collect_captured_vars_from_expr(cond, param_names, outer_scope, captured);
                }
                if let Some(ref inc) = cfs.increment {
                    self.collect_captured_vars_from_expr(inc, param_names, outer_scope, captured);
                }
                for s in &cfs.body {
                    self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                }
            }
            ast::Stmt::Break | ast::Stmt::Continue => {}
            ast::Stmt::With(ws) => {
                self.collect_captured_vars_from_expr(&ws.resource_expr, param_names, outer_scope, captured);
                for s in &ws.body {
                    self.collect_captured_vars_from_stmt(s, param_names, outer_scope, captured);
                }
            }
            ast::Stmt::TupleDestructure { expr, .. } => {
                self.collect_captured_vars_from_expr(expr, param_names, outer_scope, captured);
            }
        }
    }
}
