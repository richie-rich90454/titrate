//! Lightweight monomorphization of generic top-level functions.
//!
//! The bytecode VM erases generic type parameters to a dynamic `Variant`.
//! WebAssembly has no universal value type, so the WASM backend specializes
//! each generic call site by the physical type of its arguments and rewrites
//! the AST before lowering. Type parameters that cannot be inferred from an
//! argument produce an explicit `wasm:` error rather than a miscompile.

use crate::ast::{Block, Expr, FnDecl, Program, Type};
use std::collections::HashMap;

struct Ctx {
    generics: HashMap<String, usize>,
    defs: HashMap<String, FnDecl>,
    specs: HashMap<String, FnDecl>,
    env: HashMap<String, Type>,
}

pub(crate) fn monomorphize(program: &Program) -> Result<Program, String> {
    let mut ctx = Ctx {
        generics: HashMap::new(),
        defs: HashMap::new(),
        specs: HashMap::new(),
        env: HashMap::new(),
    };
    for d in &program.declarations {
        if let crate::ast::Declaration::Function(f) = d {
            ctx.env.insert(f.name.clone(), f.return_type.clone().unwrap_or(Type::simple("void")));
            if !f.type_params.is_empty() {
                ctx.generics.insert(f.name.clone(), f.type_params.len());
                ctx.defs.insert(f.name.clone(), f.clone());
            }
        }
    }
    if ctx.generics.is_empty() {
        return Ok(program.clone());
    }

    let mut out = program.clone();
    for d in &mut out.declarations {
        if let crate::ast::Declaration::Function(f) = d {
            if ctx.generics.contains_key(&f.name) {
                continue;
            }
            let mut env = ctx.env.clone();
            for p in &f.params {
                env.insert(p.name.clone(), p.typ.clone());
            }
            let saved = std::mem::replace(&mut ctx.env, env);
            f.body = rewrite_block(&f.body, &mut ctx)?;
            ctx.env = saved;
        }
    }
    let mut decls: Vec<crate::ast::Declaration> = out
        .declarations
        .into_iter()
        .filter(|d| match d {
            crate::ast::Declaration::Function(f) => !ctx.generics.contains_key(&f.name),
            _ => true,
        })
        .collect();
    let mut names: Vec<String> = ctx.specs.keys().cloned().collect();
    names.sort();
    for n in names {
        decls.push(crate::ast::Declaration::Function(ctx.specs[&n].clone()));
    }
    out.declarations = decls;
    Ok(out)
}

fn physical_type(e: &Expr, env: &HashMap<String, Type>) -> Option<Type> {
    match e {
        Expr::Literal(l, _) => Some(match l {
            crate::ast::Literal::Int(_) => Type::simple("long"),
            crate::ast::Literal::Float(_) => Type::simple("double"),
            crate::ast::Literal::Bool(_) => Type::simple("bool"),
            crate::ast::Literal::Char(_) => Type::simple("char"),
            crate::ast::Literal::String(_) | crate::ast::Literal::Null => Type::simple("string"),
        }),
        Expr::Identifier(n, _) => env.get(n).cloned(),
        Expr::New(t, _, _) => Some(t.clone()),
        Expr::Call(c, _, _) => match c.as_ref() {
            Expr::Identifier(n, _) => env.get(n).cloned(),
            _ => None,
        },
        Expr::Cast(_, t, _) => Some(t.clone()),
        Expr::Ternary { then_expr, .. } => physical_type(then_expr, env),
        _ => None,
    }
}

fn concrete_param(t: &Type) -> Type {
    match t {
        Type::Named { name, params } if name == "ArrayList" || name == "HashMap" => {
            params.first().cloned().unwrap_or_else(|| Type::simple("Variant"))
        }
        other => other.clone(),
    }
}

fn substitute(t: &Type, args: &HashMap<String, Type>) -> Type {
    match t {
        Type::Named { name, params } => match args.get(name) {
            Some(real) => real.clone(),
            None => Type::Named {
                name: name.clone(),
                params: params.iter().map(|p| substitute(p, args)).collect(),
            },
        },
        Type::Ref(inner) => Type::Ref(Box::new(substitute(inner, args))),
        Type::MutRef(inner) => Type::MutRef(Box::new(substitute(inner, args))),
        Type::Tuple(ts) => Type::Tuple(ts.iter().map(|t| substitute(t, args)).collect()),
    }
}

fn specialize(name: &str, args: &[Expr], ctx: &mut Ctx, span: crate::ast::Span) -> Result<String, String> {
    let count = ctx.generics[name];
    let mut targs = Vec::new();
    for i in 0..count {
        let t = args
            .get(i)
            .and_then(|a| physical_type(a, &ctx.env))
            .ok_or_else(|| format!("wasm: cannot infer generic argument {} of '{}'", i + 1, name))?;
        targs.push(concrete_param(&t));
    }
    let base = ctx.defs[name].clone();
    let map: HashMap<String, Type> = base
        .type_params
        .iter()
        .zip(targs.iter())
        .map(|(p, t)| (p.name.clone(), t.clone()))
        .collect();
    let suffix: Vec<String> = targs.iter().map(|t| t.to_string()).collect();
    let new_name = format!("{}__{}", name, suffix.join("_"));
    if !ctx.specs.contains_key(&new_name) {
        let mut f = base;
        f.name = new_name.clone();
        f.type_params.clear();
        f.where_clause.clear();
        f.params = f
            .params
            .iter()
            .map(|p| crate::ast::Param {
                name: p.name.clone(),
                typ: substitute(&p.typ, &map),
            })
            .collect();
        f.return_type = f.return_type.as_ref().map(|t| substitute(t, &map));
        let mut env = ctx.env.clone();
        for p in &f.params {
            env.insert(p.name.clone(), p.typ.clone());
        }
        let saved = std::mem::replace(&mut ctx.env, env);
        f.body = rewrite_block(&f.body, ctx)?;
        ctx.env = saved;
        ctx.specs.insert(new_name.clone(), f);
    }
    let _ = span;
    Ok(new_name)
}

fn rewrite_expr(e: &Expr, ctx: &mut Ctx) -> Result<Expr, String> {
    let out = match e {
        Expr::Call(callee, args, span) => {
            let new_args: Vec<Expr> = args
                .iter()
                .map(|a| rewrite_expr(a, ctx))
                .collect::<Result<_, _>>()?;
            let new_callee = match callee.as_ref() {
                Expr::Identifier(name, _) if ctx.generics.contains_key(name) => {
                    let sp = specialize(name, &new_args, ctx, *span)?;
                    Expr::Identifier(sp, *span)
                }
                other => other.clone(),
            };
            Expr::Call(Box::new(new_callee), new_args, *span)
        }
        Expr::Binary(a, op, b, s) => Expr::Binary(
            Box::new(rewrite_expr(a, ctx)?),
            op.clone(),
            Box::new(rewrite_expr(b, ctx)?),
            *s,
        ),
        Expr::Unary(op, a, s) => Expr::Unary(op.clone(), Box::new(rewrite_expr(a, ctx)?), *s),
        Expr::Assign(a, b, s) => Expr::Assign(
            Box::new(rewrite_expr(a, ctx)?),
            Box::new(rewrite_expr(b, ctx)?),
            *s,
        ),
        Expr::Ternary { condition, then_expr, else_expr, span } => Expr::Ternary {
            condition: Box::new(rewrite_expr(condition, ctx)?),
            then_expr: Box::new(rewrite_expr(then_expr, ctx)?),
            else_expr: Box::new(rewrite_expr(else_expr, ctx)?),
            span: *span,
        },
        Expr::Index(a, b, s) => Expr::Index(
            Box::new(rewrite_expr(a, ctx)?),
            Box::new(rewrite_expr(b, ctx)?),
            *s,
        ),
        Expr::MemberAccess(o, m, s) => Expr::MemberAccess(Box::new(rewrite_expr(o, ctx)?), m.clone(), *s),
        Expr::New(t, args, s) => Expr::New(
            t.clone(),
            args.iter().map(|a| rewrite_expr(a, ctx)).collect::<Result<_, _>>()?,
            *s,
        ),
        Expr::StaticCall { class_name, method, args, span } => Expr::StaticCall {
            class_name: class_name.clone(),
            method: method.clone(),
            args: args.iter().map(|a| rewrite_expr(a, ctx)).collect::<Result<_, _>>()?,
            span: *span,
        },
        Expr::Cast(inner, t, s) => Expr::Cast(Box::new(rewrite_expr(inner, ctx)?), t.clone(), *s),
        Expr::ErrorPropagation(inner, s) => Expr::ErrorPropagation(Box::new(rewrite_expr(inner, ctx)?), *s),
        Expr::Tuple(xs, s) => Expr::Tuple(
            xs.iter().map(|x| rewrite_expr(x, ctx)).collect::<Result<_, _>>()?,
            *s,
        ),
        other => other.clone(),
    };
    Ok(out)
}

fn rewrite_block(block: &Block, ctx: &mut Ctx) -> Result<Block, String> {
    use crate::ast::Stmt;
    let mut out = Vec::new();
    for s in block {
        out.push(match s {
            Stmt::Expr(e) => Stmt::Expr(rewrite_expr(e, ctx)?),
            Stmt::Throw(e, p) => Stmt::Throw(rewrite_expr(e, ctx)?, *p),
            Stmt::VarDecl(v) => {
                let mut nv = v.clone();
                nv.init = nv.init.map(|e| rewrite_expr(&e, ctx)).transpose()?;
                if let Some(t) = &nv.typ {
                    ctx.env.insert(nv.name.clone(), t.clone());
                } else if let Some(e) = &nv.init {
                    if let Some(t) = physical_type(e, &ctx.env) {
                        ctx.env.insert(nv.name.clone(), concrete_param(&t));
                    }
                }
                Stmt::VarDecl(nv)
            }
            Stmt::ConstDecl(v) => {
                let mut nv = v.clone();
                nv.init = nv.init.map(|e| rewrite_expr(&e, ctx)).transpose()?;
                Stmt::ConstDecl(nv)
            }
            Stmt::Return(e) => Stmt::Return(
                e.as_ref().map(|e| rewrite_expr(e, ctx)).transpose()?,
            ),
            Stmt::Block(b) => Stmt::Block(rewrite_block(b, ctx)?),
            Stmt::If(i) => {
                let mut ni = i.clone();
                ni.condition = rewrite_expr(&i.condition, ctx)?;
                ni.then_branch = rewrite_block(&i.then_branch, ctx)?;
                ni.else_branch = ni.else_branch.map(|b| rewrite_block(&b, ctx)).transpose()?;
                Stmt::If(ni)
            }
            Stmt::While(w) => {
                let mut nw = w.clone();
                nw.condition = rewrite_expr(&w.condition, ctx)?;
                nw.body = rewrite_block(&w.body, ctx)?;
                Stmt::While(nw)
            }
            Stmt::DoWhile(d) => {
                let mut nd = d.clone();
                nd.condition = rewrite_expr(&d.condition, ctx)?;
                nd.body = rewrite_block(&d.body, ctx)?;
                Stmt::DoWhile(nd)
            }
            Stmt::For(f) => {
                let mut nf = f.clone();
                nf.iterable = rewrite_expr(&f.iterable, ctx)?;
                nf.body = rewrite_block(&f.body, ctx)?;
                Stmt::For(nf)
            }
            Stmt::CFor(c) => {
                let mut nc = c.clone();
                nc.condition = nc.condition.map(|e| rewrite_expr(&e, ctx)).transpose()?;
                nc.increment = nc.increment.map(|e| rewrite_expr(&e, ctx)).transpose()?;
                nc.body = rewrite_block(&c.body, ctx)?;
                Stmt::CFor(nc)
            }
            Stmt::Switch(sw) => {
                let mut ns = sw.clone();
                ns.expr = rewrite_expr(&sw.expr, ctx)?;
                ns.cases = ns.cases.iter_mut().map(|c| {
                    let body = rewrite_block(&c.body, ctx)?;
                    Ok(crate::ast::Case { pattern: c.pattern.clone(), body })
                }).collect::<Result<Vec<_>, String>>()?;
                ns.default = ns.default.map(|b| rewrite_block(&b, ctx)).transpose()?;
                Stmt::Switch(ns)
            }
            Stmt::TryCatch { try_block, catch_var, catch_var_type, catch_block, span } => {
                Stmt::TryCatch {
                    try_block: rewrite_block(try_block, ctx)?,
                    catch_var: catch_var.clone(),
                    catch_var_type: catch_var_type.clone(),
                    catch_block: rewrite_block(catch_block, ctx)?,
                    span: *span,
                }
            }
            Stmt::TupleDestructure { names, expr, mutable, span } => Stmt::TupleDestructure {
                names: names.clone(),
                expr: rewrite_expr(expr, ctx)?,
                mutable: *mutable,
                span: *span,
            },
            other => other.clone(),
        });
    }
    Ok(out)
}
