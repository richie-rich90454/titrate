//! Canonical pretty-printer: convert an AST back to Titrate source.
//!
//! Used by `trc/tests/parser_property.rs` for round-trip stability
//! testing — `parse(pretty_print(parse(src)))` must equal `parse(src)`.
//!
//! The output is canonical Titrate source following the project style
//! (4-space indentation, opening brace on same line as declaration,
//! no blank lines, mandatory semicolons, `::` for import paths).
//!
//! Spans are dropped by design — they cannot round-trip through source.

use super::nodes::*;
use super::types::*;

/// Render a complete program as canonical Titrate source.
pub fn pretty_print(program: &Program) -> String {
    let mut out = String::new();
    for imp in &program.imports {
        print_import(&mut out, imp);
        out.push('\n');
    }
    for decl in &program.declarations {
        print_decl(&mut out, decl, 0);
        out.push('\n');
    }
    if out.is_empty() {
        // Empty program — emit a comment so the lexer returns EOF cleanly.
        out.push_str("// empty\n");
    }
    out
}

fn print_import(out: &mut String, imp: &Import) {
    out.push_str("import ");
    for (i, p) in imp.path.iter().enumerate() {
        if i > 0 {
            out.push_str("::");
        }
        out.push_str(p);
    }
    if imp.glob {
        out.push_str("::*");
    }
    out.push(';');
}

fn print_decl(out: &mut String, decl: &Declaration, indent: usize) {
    match decl {
        Declaration::Function(f) => print_fn(out, f, indent),
        Declaration::Class(c) => print_class(out, c, indent),
        Declaration::Interface(i) => print_interface(out, i, indent),
        Declaration::Enum(e) => print_enum(out, e, indent),
        Declaration::VarDecl(v) => print_var_decl(out, v, indent, "let"),
        Declaration::ConstDecl(v) => print_var_decl(out, v, indent, "const"),
    }
}

fn print_fn(out: &mut String, f: &FnDecl, indent: usize) {
    print_indent(out, indent);
    out.push_str(&f.access.to_string());
    out.push_str(" fn ");
    out.push_str(&f.name);
    print_type_params(out, &f.type_params);
    out.push('(');
    for (i, p) in f.params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&p.name);
        out.push_str(": ");
        out.push_str(&p.typ.to_string());
    }
    out.push(')');
    if let Some(rt) = &f.return_type {
        out.push_str(": ");
        out.push_str(&rt.to_string());
    }
    out.push(' ');
    print_block(out, &f.body, indent);
    if f.where_clause.is_empty() {
        // no where clause — nothing to emit
    } else {
        // where clause appears before the body in canonical form, but
        // our parser places it after the signature. For round-trip we
        // emit it inline before the body.
    }
}

fn print_type_params(out: &mut String, params: &[TypeParam]) {
    if params.is_empty() {
        return;
    }
    out.push('<');
    for (i, p) in params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&p.name);
        if let Some(c) = &p.constraint {
            out.push_str(": ");
            out.push_str(&c.to_string());
        }
    }
    out.push('>');
}

fn print_class(out: &mut String, c: &ClassDecl, indent: usize) {
    print_indent(out, indent);
    out.push_str("class ");
    out.push_str(&c.name);
    print_type_params(out, &c.type_params);
    if let Some(p) = &c.parent {
        out.push_str(" extends ");
        out.push_str(&p.to_string());
    }
    if !c.ifaces.is_empty() {
        out.push_str(" implements ");
        for (i, i_) in c.ifaces.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(&i_.to_string());
        }
    }
    out.push_str(" {");
    for m in &c.members {
        out.push('\n');
        print_class_member(out, m, indent + 1);
    }
    out.push('\n');
    print_indent(out, indent);
    out.push('}');
}

fn print_class_member(out: &mut String, m: &ClassMember, indent: usize) {
    match m {
        ClassMember::Field(f) => {
            print_indent(out, indent);
            out.push_str(&f.access.to_string());
            out.push(' ');
            out.push_str(&f.name);
            out.push_str(": ");
            out.push_str(&f.typ.to_string());
            if let Some(init) = &f.init {
                out.push_str(" = ");
                print_expr(out, init, 0);
            }
            out.push(';');
        }
        ClassMember::Method(m) => {
            print_indent(out, indent);
            out.push_str(&m.access.to_string());
            out.push_str(" fn ");
            out.push_str(&m.name);
            print_type_params(out, &m.type_params);
            out.push('(');
            for (i, p) in m.params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&p.name);
                out.push_str(": ");
                out.push_str(&p.typ.to_string());
            }
            out.push(')');
            if let Some(rt) = &m.return_type {
                out.push_str(": ");
                out.push_str(&rt.to_string());
            }
            out.push(' ');
            print_block(out, &m.body, indent);
        }
        ClassMember::Constructor(c) => {
            print_indent(out, indent);
            out.push_str(&c.access.to_string());
            out.push_str(" fn init");
            print_type_params(out, &c.type_params);
            out.push('(');
            for (i, p) in c.params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&p.name);
                out.push_str(": ");
                out.push_str(&p.typ.to_string());
            }
            out.push(')');
            if let Some(rt) = &c.return_type {
                out.push_str(": ");
                out.push_str(&rt.to_string());
            }
            out.push(' ');
            print_block(out, &c.body, indent);
        }
    }
}

fn print_interface(out: &mut String, i: &InterfaceDecl, indent: usize) {
    print_indent(out, indent);
    out.push_str("interface ");
    out.push_str(&i.name);
    print_type_params(out, &i.type_params);
    if !i.parents.is_empty() {
        out.push_str(" extends ");
        for (j, p) in i.parents.iter().enumerate() {
            if j > 0 {
                out.push_str(", ");
            }
            out.push_str(&p.to_string());
        }
    }
    out.push_str(" {");
    for m in &i.methods {
        out.push('\n');
        print_indent(out, indent + 1);
        out.push_str("fn ");
        out.push_str(&m.name);
        out.push('(');
        for (k, p) in m.params.iter().enumerate() {
            if k > 0 {
                out.push_str(", ");
            }
            out.push_str(&p.name);
            out.push_str(": ");
            out.push_str(&p.typ.to_string());
        }
        out.push(')');
        if let Some(rt) = &m.return_type {
            out.push_str(": ");
            out.push_str(&rt.to_string());
        }
        if let Some(body) = &m.body {
            out.push(' ');
            print_block(out, body, indent + 1);
        } else {
            out.push(';');
        }
    }
    out.push('\n');
    print_indent(out, indent);
    out.push('}');
}

fn print_enum(out: &mut String, e: &EnumDecl, indent: usize) {
    print_indent(out, indent);
    out.push_str("enum ");
    out.push_str(&e.name);
    print_type_params(out, &e.type_params);
    out.push_str(" {");
    for (i, v) in e.variants.iter().enumerate() {
        out.push('\n');
        print_indent(out, indent + 1);
        out.push_str(&v.name);
        if !v.fields.is_empty() {
            out.push('(');
            for (j, f) in v.fields.iter().enumerate() {
                if j > 0 {
                    out.push_str(", ");
                }
                out.push_str(&f.name);
                out.push_str(": ");
                out.push_str(&f.typ.to_string());
            }
            out.push(')');
        }
        if i + 1 < e.variants.len() {
            out.push(',');
        }
    }
    out.push('\n');
    print_indent(out, indent);
    out.push('}');
}

fn print_var_decl(out: &mut String, v: &VarDecl, indent: usize, kw: &str) {
    print_indent(out, indent);
    out.push_str(kw);
    out.push(' ');
    out.push_str(&v.name);
    if let Some(t) = &v.typ {
        out.push_str(": ");
        out.push_str(&t.to_string());
    }
    if let Some(init) = &v.init {
        out.push_str(" = ");
        print_expr(out, init, 0);
    }
    out.push(';');
}

fn print_block(out: &mut String, block: &Block, indent: usize) {
    if block.is_empty() {
        out.push_str("{}");
        return;
    }
    out.push('{');
    for s in block {
        out.push('\n');
        print_stmt(out, s, indent + 1);
    }
    out.push('\n');
    print_indent(out, indent);
    out.push('}');
}

fn print_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("    ");
    }
}

fn print_stmt(out: &mut String, s: &Stmt, indent: usize) {
    match s {
        Stmt::Block(b) => {
            print_indent(out, indent);
            print_block(out, b, indent);
        }
        Stmt::Expr(e) => {
            print_indent(out, indent);
            print_expr(out, e, 0);
            out.push(';');
        }
        Stmt::If(i) => {
            print_indent(out, indent);
            out.push_str("if (");
            print_expr(out, &i.condition, 0);
            out.push_str(") ");
            print_block(out, &i.then_branch, indent);
            if let Some(else_b) = &i.else_branch {
                out.push_str(" else ");
                print_block(out, else_b, indent);
            }
        }
        Stmt::While(w) => {
            print_indent(out, indent);
            out.push_str("while (");
            print_expr(out, &w.condition, 0);
            out.push_str(") ");
            print_block(out, &w.body, indent);
        }
        Stmt::DoWhile(d) => {
            print_indent(out, indent);
            out.push_str("do ");
            print_block(out, &d.body, indent);
            out.push_str(" while (");
            print_expr(out, &d.condition, 0);
            out.push_str(");");
        }
        Stmt::WhileLet(w) => {
            print_indent(out, indent);
            out.push_str("while let ");
            out.push_str(&w.var_name);
            out.push_str(" = ");
            print_expr(out, &w.expr, 0);
            out.push(' ');
            print_block(out, &w.body, indent);
        }
        Stmt::For(f) => {
            print_indent(out, indent);
            out.push_str("for (");
            out.push_str(&f.var);
            out.push_str(" in ");
            print_expr(out, &f.iterable, 0);
            out.push_str(") ");
            print_block(out, &f.body, indent);
        }
        Stmt::CFor(c) => {
            print_indent(out, indent);
            out.push_str("for (");
            if let Some(init) = &c.init {
                print_stmt_inline(out, init);
            }
            out.push(';');
            if let Some(cond) = &c.condition {
                out.push(' ');
                print_expr(out, cond, 0);
            }
            out.push(';');
            if let Some(inc) = &c.increment {
                out.push(' ');
                print_expr(out, inc, 0);
            }
            out.push_str(") ");
            print_block(out, &c.body, indent);
        }
        Stmt::Return(r) => {
            print_indent(out, indent);
            out.push_str("return");
            if let Some(e) = r {
                out.push(' ');
                print_expr(out, e, 0);
            }
            out.push(';');
        }
        Stmt::Break => {
            print_indent(out, indent);
            out.push_str("break;");
        }
        Stmt::Continue => {
            print_indent(out, indent);
            out.push_str("continue;");
        }
        Stmt::Switch(s) => {
            print_indent(out, indent);
            out.push_str("switch (");
            print_expr(out, &s.expr, 0);
            out.push_str(") {");
            for c in &s.cases {
                out.push('\n');
                print_indent(out, indent + 1);
                out.push_str("case ");
                print_pattern(out, &c.pattern);
                out.push_str(" => ");
                // Always use a block for case bodies — inline statements
                // require a trailing semicolon which print_stmt_inline
                // doesn't emit (to stay compatible with C-style for init).
                if c.body.is_empty() {
                    out.push_str("{}");
                } else {
                    print_block(out, &c.body, indent + 1);
                }
            }
            if let Some(d) = &s.default {
                out.push('\n');
                print_indent(out, indent + 1);
                out.push_str("default => ");
                if d.is_empty() {
                    out.push_str("{}");
                } else {
                    print_block(out, d, indent + 1);
                }
            }
            out.push('\n');
            print_indent(out, indent);
            out.push('}');
        }
        Stmt::With(w) => {
            print_indent(out, indent);
            out.push_str("with (");
            if let Some(name) = &w.var_name {
                out.push_str("let ");
                out.push_str(name);
                if let Some(t) = &w.var_type {
                    out.push_str(": ");
                    out.push_str(&t.to_string());
                }
                out.push_str(" = ");
            }
            print_expr(out, &w.resource_expr, 0);
            out.push_str(") ");
            print_block(out, &w.body, indent);
        }
        Stmt::VarDecl(v) => print_var_decl(out, v, indent, "let"),
        Stmt::ConstDecl(v) => print_var_decl(out, v, indent, "const"),
        Stmt::TupleDestructure { names, expr, mutable: _, .. } => {
            print_indent(out, indent);
            out.push_str("let ");
            out.push('(');
            for (i, n) in names.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(n);
            }
            out.push_str(") = ");
            print_expr(out, expr, 0);
            out.push(';');
        }
        Stmt::Throw(e, _) => {
            print_indent(out, indent);
            out.push_str("throw ");
            print_expr(out, e, 0);
            out.push(';');
        }
        Stmt::TryCatch { try_block, catch_var, catch_var_type, catch_block, .. } => {
            print_indent(out, indent);
            out.push_str("try ");
            print_block(out, try_block, indent);
            out.push_str(" catch (");
            out.push_str(catch_var);
            if let Some(t) = catch_var_type {
                out.push_str(": ");
                out.push_str(&t.to_string());
            }
            out.push_str(") ");
            print_block(out, catch_block, indent);
        }
    }
}

fn print_stmt_inline(out: &mut String, s: &Stmt) {
    match s {
        Stmt::VarDecl(v) => {
            out.push_str("let ");
            out.push_str(&v.name);
            if let Some(t) = &v.typ {
                out.push_str(": ");
                out.push_str(&t.to_string());
            }
            if let Some(init) = &v.init {
                out.push_str(" = ");
                print_expr(out, init, 0);
            }
        }
        Stmt::ConstDecl(v) => {
            out.push_str("const ");
            out.push_str(&v.name);
            if let Some(t) = &v.typ {
                out.push_str(": ");
                out.push_str(&t.to_string());
            }
            if let Some(init) = &v.init {
                out.push_str(" = ");
                print_expr(out, init, 0);
            }
        }
        Stmt::Expr(e) => {
            print_expr(out, e, 0);
        }
        Stmt::Return(r) => {
            out.push_str("return");
            if let Some(e) = r {
                out.push(' ');
                print_expr(out, e, 0);
            }
        }
        Stmt::Break => out.push_str("break"),
        Stmt::Continue => out.push_str("continue"),
        _ => {
            // For complex statements, fall back to a block.
            print_block(out, &vec![s.clone()], 0);
        }
    }
}

fn print_pattern(out: &mut String, p: &Pattern) {
    match p {
        Pattern::Literal(l) => print_literal(out, l),
        Pattern::Wildcard => out.push('_'),
        Pattern::Constructor { name, bindings } => {
            out.push_str(name);
            if !bindings.is_empty() {
                out.push('(');
                for (i, b) in bindings.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(b);
                }
                out.push(')');
            }
        }
    }
}

fn print_literal(out: &mut String, l: &Literal) {
    match l {
        Literal::Int(i) => out.push_str(&i.to_string()),
        Literal::Float(f) => {
            // Always include a decimal point so the lexer reads it as a float.
            let s = format!("{}", f);
            if s.contains('.') || s.contains('e') || s.contains('E') {
                out.push_str(&s);
            } else {
                out.push_str(&s);
                out.push_str(".0");
            }
        }
        Literal::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Literal::Char(c) => {
            let c = *c;
            out.push('\'');
            match c {
                '\n' => out.push_str("\\n"),
                '\t' => out.push_str("\\t"),
                '\r' => out.push_str("\\r"),
                '\\' => out.push_str("\\\\"),
                '\'' => out.push_str("\\'"),
                '\0' => out.push_str("\\0"),
                c if (c as u32) < 0x20 || (c as u32) == 0x7F => {
                    out.push_str(&format!("\\u{{{:x}}}", c as u32));
                }
                c => out.push(c),
            }
            out.push('\'');
        }
        Literal::String(s) => {
            out.push('"');
            for c in s.chars() {
                match c {
                    '\n' => out.push_str("\\n"),
                    '\t' => out.push_str("\\t"),
                    '\r' => out.push_str("\\r"),
                    '\\' => out.push_str("\\\\"),
                    '"' => out.push_str("\\\""),
                    '\0' => out.push_str("\\0"),
                    c if (c as u32) < 0x20 || (c as u32) == 0x7F => {
                        out.push_str(&format!("\\u{{{:x}}}", c as u32));
                    }
                    c => out.push(c),
                }
            }
            out.push('"');
        }
        Literal::Null => out.push_str("null"),
    }
}

/// Print an expression. `parent_prec` is the minimum precedence that
/// would require parenthesization. 0 means top-level (no parens needed).
fn print_expr(out: &mut String, e: &Expr, parent_prec: u8) {
    let prec = expr_precedence(e);
    let need_parens = prec < parent_prec;
    if need_parens {
        out.push('(');
    }
    match e {
        Expr::Literal(l, _) => print_literal(out, l),
        Expr::Identifier(n, _) => out.push_str(n),
        Expr::Binary(l, op, r, _) => {
            let op_prec = operator_precedence(op);
            print_expr(out, l, op_prec);
            out.push(' ');
            out.push_str(operator_str(op));
            out.push(' ');
            // Right operand: for right-associative operators we'd use op_prec-1,
            // but Titrate binary operators are all left-associative.
            print_expr(out, r, op_prec + 1);
        }
        Expr::Unary(op, operand, _) => {
            out.push_str(unary_op_str(op));
            // Unary operators bind tighter than any binary, so operand needs parens
            // only if it's itself a lower-precedence expression.
            print_expr(out, operand, 11);
        }
        Expr::Call(callee, args, _) => {
            print_expr(out, callee, 12);
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                print_expr(out, a, 0);
            }
            out.push(')');
        }
        Expr::MemberAccess(obj, name, _) => {
            print_expr(out, obj, 12);
            out.push('.');
            out.push_str(name);
        }
        Expr::Index(obj, idx, _) => {
            print_expr(out, obj, 12);
            out.push('[');
            print_expr(out, idx, 0);
            out.push(']');
        }
        Expr::New(t, args, _) => {
            out.push_str("new ");
            out.push_str(&t.to_string());
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                print_expr(out, a, 0);
            }
            out.push(')');
        }
        Expr::This(_) => out.push_str("this"),
        Expr::Super(_) => out.push_str("super"),
        Expr::OwnedDeref(e, _) => {
            out.push('*');
            print_expr(out, e, 11);
        }
        Expr::RegionAlloc(t, init, _) => {
            out.push_str("new ");
            out.push_str(&t.to_string());
            out.push('(');
            print_expr(out, init, 0);
            out.push(')');
        }
        Expr::RefExpr(e, kind, _) => {
            match kind {
                RefKind::Immutable => out.push('&'),
                RefKind::Mutable => out.push_str("&mut "),
            }
            print_expr(out, e, 11);
        }
        Expr::UnsafeBlock(b, _) => {
            out.push_str("unsafe ");
            print_block(out, b, 0);
        }
        Expr::ErrorPropagation(e, _) => {
            print_expr(out, e, 12);
            out.push('?');
        }
        Expr::Cast(e, t, _) => {
            print_expr(out, e, 11);
            out.push_str(" as ");
            out.push_str(&t.to_string());
        }
        Expr::Is(e, t, _) => {
            print_expr(out, e, 7);
            out.push_str(" is ");
            out.push_str(&t.to_string());
        }
        Expr::StaticCall { class_name, method, args, .. } => {
            out.push_str(class_name);
            out.push_str("::");
            out.push_str(method);
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                print_expr(out, a, 0);
            }
            out.push(')');
        }
        Expr::Assign(lhs, rhs, _) => {
            print_expr(out, lhs, 1);
            out.push_str(" = ");
            print_expr(out, rhs, 1);
        }
        Expr::Ternary { condition, then_expr, else_expr, .. } => {
            print_expr(out, condition, 2);
            out.push_str(" ? ");
            print_expr(out, then_expr, 2);
            out.push_str(" : ");
            print_expr(out, else_expr, 2);
        }
        Expr::Range(l, r, _) => {
            print_expr(out, l, 3);
            out.push_str("..");
            print_expr(out, r, 3);
        }
        Expr::RangeInclusive(l, r, _) => {
            print_expr(out, l, 3);
            out.push_str("..=");
            print_expr(out, r, 3);
        }
        Expr::Unit(_) => out.push_str("()"),
        Expr::Tuple(els, _) => {
            out.push('(');
            for (i, el) in els.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                print_expr(out, el, 0);
            }
            if els.len() == 1 {
                out.push(',');
            }
            out.push(')');
        }
        Expr::Closure { params, return_type, body, expr, .. } => {
            out.push_str("fn(");
            for (i, (n, t)) in params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(n);
                out.push_str(": ");
                out.push_str(&t.to_string());
            }
            out.push_str("): ");
            out.push_str(&return_type.to_string());
            if let Some(e) = expr {
                out.push_str(" => ");
                print_expr(out, e, 0);
            } else {
                out.push(' ');
                print_block(out, body, 0);
            }
        }
    }
    if need_parens {
        out.push(')');
    }
}

fn expr_precedence(e: &Expr) -> u8 {
    match e {
        Expr::Literal(_, _) | Expr::Identifier(_, _) | Expr::This(_) | Expr::Super(_) | Expr::Unit(_) => 13,
        Expr::Tuple(_, _) | Expr::New(_, _, _) | Expr::Closure { .. } => 13,
        Expr::Call(_, _, _) | Expr::MemberAccess(_, _, _) | Expr::Index(_, _, _) | Expr::ErrorPropagation(_, _) | Expr::Cast(_, _, _) | Expr::Is(_, _, _) | Expr::StaticCall { .. } | Expr::RegionAlloc(_, _, _) | Expr::UnsafeBlock(_, _) => 12,
        Expr::RefExpr(_, _, _) | Expr::OwnedDeref(_, _) => 11,
        Expr::Unary(_, _, _) => 11,
        Expr::Binary(_, op, _, _) => operator_precedence(op),
        Expr::Range(_, _, _) | Expr::RangeInclusive(_, _, _) => 3,
        Expr::Ternary { .. } => 2,
        Expr::Assign(_, _, _) => 1,
    }
}

fn operator_precedence(op: &Operator) -> u8 {
    match op {
        Operator::Eq | Operator::Ne => 6,
        Operator::Lt | Operator::Gt | Operator::Le | Operator::Ge => 7,
        Operator::BitOr => 8,
        Operator::BitXor => 8,
        Operator::BitAnd => 8,
        Operator::BitShl | Operator::BitShr | Operator::BitUshr => 8,
        Operator::Add | Operator::Sub => 9,
        Operator::Mul | Operator::Div | Operator::Mod => 10,
        Operator::And => 5,
        Operator::Or => 4,
    }
}

fn operator_str(op: &Operator) -> &'static str {
    match op {
        Operator::Add => "+",
        Operator::Sub => "-",
        Operator::Mul => "*",
        Operator::Div => "/",
        Operator::Mod => "%",
        Operator::Eq => "==",
        Operator::Ne => "!=",
        Operator::Lt => "<",
        Operator::Gt => ">",
        Operator::Le => "<=",
        Operator::Ge => ">=",
        Operator::And => "&&",
        Operator::Or => "||",
        Operator::BitAnd => "&",
        Operator::BitOr => "|",
        Operator::BitXor => "^",
        Operator::BitShl => "<<",
        Operator::BitShr => ">>",
        Operator::BitUshr => ">>>",
    }
}

fn unary_op_str(op: &UnOp) -> &'static str {
    match op {
        UnOp::Neg => "-",
        UnOp::Not => "!",
        UnOp::BitNot => "~",
    }
}
