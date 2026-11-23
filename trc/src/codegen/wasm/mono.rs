//! Monomorphization of generic functions and generic classes.
//!
//! The bytecode VM erases generic type parameters to a dynamic `Variant`.
//! WebAssembly has no universal value type, so the WASM backend specializes
//! each generic call site by the physical type of its arguments (or by the
//! expected type from the call site) and rewrites the AST before lowering.
//! Type parameters with no inferable type fall back to the dynamic `Variant`
//! representation, matching the bytecode VM.

use crate::ast::{Block, Expr, FnDecl, Program, Type};
use std::collections::HashMap;

struct Ctx {
    generics: HashMap<String, usize>,
    defs: HashMap<String, FnDecl>,
    fn_params: HashMap<String, Vec<Type>>,
    specs: HashMap<String, FnDecl>,
    generic_classes: HashMap<String, crate::ast::ClassDecl>,
    class_params: HashMap<String, Vec<String>>,
    class_specs: HashMap<String, crate::ast::ClassDecl>,
    env: HashMap<String, Type>,
    current_return: Option<Type>,
}

pub(crate) fn monomorphize(program: &Program) -> Result<Program, String> {
    let mut ctx = Ctx {
        generics: HashMap::new(),
        defs: HashMap::new(),
        fn_params: HashMap::new(),
        specs: HashMap::new(),
        generic_classes: HashMap::new(),
        class_params: HashMap::new(),
        class_specs: HashMap::new(),
        env: HashMap::new(),
        current_return: None,
    };
    for d in &program.declarations {
        match d {
            crate::ast::Declaration::Function(f) => {
                ctx.env.insert(f.name.clone(), f.return_type.clone().unwrap_or(Type::simple("void")));
                ctx.fn_params
                    .insert(f.name.clone(), f.params.iter().map(|p| p.typ.clone()).collect());
                if !f.type_params.is_empty() {
                    ctx.generics.insert(f.name.clone(), f.type_params.len());
                    ctx.defs.insert(f.name.clone(), f.clone());
                }
            }
            crate::ast::Declaration::Class(c) if !c.type_params.is_empty() => {
                ctx.class_params.insert(
                    c.name.clone(),
                    c.type_params.iter().map(|p| p.name.clone()).collect(),
                );
                ctx.generic_classes.insert(c.name.clone(), c.clone());
            }
            _ => {}
        }
    }
    if ctx.generics.is_empty() && ctx.generic_classes.is_empty() {
        return Ok(program.clone());
    }

    let mut out = program.clone();
    for d in &mut out.declarations {
        match d {
            crate::ast::Declaration::Function(f) => {
                if ctx.generics.contains_key(&f.name) {
                    continue;
                }
                let mut env = ctx.env.clone();
                for p in &f.params {
                    env.insert(p.name.clone(), p.typ.clone());
                }
                let saved = std::mem::replace(&mut ctx.env, env);
                let saved_ret = std::mem::replace(&mut ctx.current_return, f.return_type.clone());
                f.body = rewrite_block(&f.body, &mut ctx)?;
                ctx.env = saved;
                ctx.current_return = saved_ret;
            }
            crate::ast::Declaration::Class(c) => {
                if ctx.generic_classes.contains_key(&c.name) {
                    continue;
                }
                let mut env = ctx.env.clone();
                for m in &c.members {
                    if let crate::ast::ClassMember::Field(f) = m {
                        env.insert(f.name.clone(), f.typ.clone());
                    }
                }
                let saved = std::mem::replace(&mut ctx.env, env);
                for m in &mut c.members {
                    match m {
                        crate::ast::ClassMember::Method(md)
                        | crate::ast::ClassMember::Constructor(md) => {
                            let mut menv = ctx.env.clone();
                            for p in &md.params {
                                menv.insert(p.name.clone(), p.typ.clone());
                            }
                            let saved2 = std::mem::replace(&mut ctx.env, menv);
                            let saved_ret =
                                std::mem::replace(&mut ctx.current_return, md.return_type.clone());
                            md.body = rewrite_block(&md.body, &mut ctx)?;
                            ctx.env = saved2;
                            ctx.current_return = saved_ret;
                        }
                        crate::ast::ClassMember::Field(_) => {}
                    }
                }
                ctx.env = saved;
            }
            _ => {}
        }
    }
    let mut decls: Vec<crate::ast::Declaration> = out
        .declarations
        .into_iter()
        .filter(|d| match d {
            crate::ast::Declaration::Function(f) => !ctx.generics.contains_key(&f.name),
            crate::ast::Declaration::Class(c) => !ctx.generic_classes.contains_key(&c.name),
            _ => true,
        })
        .collect();
    let mut names: Vec<String> = ctx.specs.keys().cloned().collect();
    names.sort();
    for n in names {
        decls.push(crate::ast::Declaration::Function(ctx.specs[&n].clone()));
    }
    let mut cnames: Vec<String> = ctx.class_specs.keys().cloned().collect();
    cnames.sort();
    for n in cnames {
        decls.push(crate::ast::Declaration::Class(ctx.class_specs[&n].clone()));
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
        Expr::Call(c, ..) => match c.as_ref() {
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

/// Match a generic return type against an expected type, collecting bindings
/// for the declared type parameters.
fn unify(pattern: &Type, concrete: &Type, params: &[String], out: &mut HashMap<String, Type>) {
    if let Type::Named { name, params: ps } = pattern {
        if ps.is_empty() && params.contains(name) {
            let self_ref = matches!(concrete, Type::Named { name: c, params: cp } if cp.is_empty() && params.contains(c));
            let bound = if self_ref { Type::simple("Variant") } else { concrete.clone() };
            out.entry(name.clone()).or_insert(bound);
            return;
        }
        if let Type::Named { name: cname, params: cps } = concrete {
            if name == cname && ps.len() == cps.len() {
                for (p, c) in ps.iter().zip(cps) {
                    unify(p, c, params, out);
                }
            }
        }
    }
}

fn mangle_type(t: &Type) -> String {
    match t {
        Type::Named { name, params } => {
            if params.is_empty() {
                name.clone()
            } else {
                let mut s = name.clone();
                for p in params {
                    s.push_str("__");
                    s.push_str(&mangle_type(p));
                }
                s
            }
        }
        Type::Ref(inner) => format!("Ref_{}", mangle_type(inner)),
        Type::MutRef(inner) => format!("MutRef_{}", mangle_type(inner)),
        Type::Tuple(ts) => format!(
            "Tuple_{}",
            ts.iter().map(mangle_type).collect::<Vec<_>>().join("_")
        ),
    }
}

fn mangle(name: &str, args: &[Type]) -> String {
    if args.is_empty() {
        return name.to_string();
    }
    let mut s = name.to_string();
    for a in args {
        s.push_str("__");
        s.push_str(&mangle_type(a));
    }
    s
}

/// Specialize a generic class for a concrete set of type arguments.
fn specialize_class(name: &str, targs: &[Type], ctx: &mut Ctx) -> Result<String, String> {
    let mangled = mangle(name, targs);
    if ctx.class_specs.contains_key(&mangled) {
        return Ok(mangled);
    }
    let names = ctx
        .class_params
        .get(name)
        .cloned()
        .ok_or_else(|| format!("wasm: unknown generic class '{}'", name))?;
    let map: HashMap<String, Type> = names.iter().cloned().zip(targs.iter().cloned()).collect();
    let base = ctx.generic_classes[name].clone();
    ctx.class_specs.insert(mangled.clone(), base.clone());

    let mut c = base;
    c.name = mangled.clone();
    c.type_params.clear();
    c.parent = c.parent.as_ref().map(|p| map_type(p, &map, ctx)).transpose()?;
    c.ifaces = c
        .ifaces
        .iter()
        .map(|t| map_type(t, &map, ctx))
        .collect::<Result<_, _>>()?;
    for m in &mut c.members {
        match m {
            crate::ast::ClassMember::Field(f) => {
                f.typ = map_type(&f.typ, &map, ctx)?;
                f.init = f.init.as_ref().map(|e| subst_expr(e, &map, ctx)).transpose()?;
            }
            crate::ast::ClassMember::Method(md) | crate::ast::ClassMember::Constructor(md) => {
                for p in &mut md.params {
                    p.typ = map_type(&p.typ, &map, ctx)?;
                }
                md.return_type = md.return_type.as_ref().map(|t| map_type(t, &map, ctx)).transpose()?;
                md.body = map_block(&md.body, &map, ctx)?;
                let mut env = ctx.env.clone();
                for p in &md.params {
                    env.insert(p.name.clone(), p.typ.clone());
                }
                let saved = std::mem::replace(&mut ctx.env, env);
                let saved_ret = std::mem::replace(&mut ctx.current_return, md.return_type.clone());
                md.body = rewrite_block(&md.body, ctx)?;
                ctx.env = saved;
                ctx.current_return = saved_ret;
            }
        }
    }
    ctx.class_specs.insert(mangled.clone(), c);
    Ok(mangled)
}

fn map_type(t: &Type, map: &HashMap<String, Type>, ctx: &mut Ctx) -> Result<Type, String> {
    let concrete = substitute(t, map);
    rewrite_type(&concrete, ctx)
}

fn map_block(
    block: &[crate::ast::Stmt],
    map: &HashMap<String, Type>,
    ctx: &mut Ctx,
) -> Result<Block, String> {
    block.iter().map(|s| subst_stmt(s, map, ctx)).collect()
}

#[allow(clippy::too_many_lines)]
fn subst_stmt(
    s: &crate::ast::Stmt,
    map: &HashMap<String, Type>,
    ctx: &mut Ctx,
) -> Result<crate::ast::Stmt, String> {
    use crate::ast::Stmt;
    Ok(match s {
        Stmt::VarDecl(v) | Stmt::ConstDecl(v) => {
            let mut nv = v.clone();
            nv.typ = nv.typ.as_ref().map(|t| map_type(t, map, ctx)).transpose()?;
            nv.init = nv.init.as_ref().map(|e| subst_expr(e, map, ctx)).transpose()?;
            if matches!(s, Stmt::VarDecl(_)) { Stmt::VarDecl(nv) } else { Stmt::ConstDecl(nv) }
        }
        Stmt::Expr(e) => Stmt::Expr(subst_expr(e, map, ctx)?),
        Stmt::Throw(e, p) => Stmt::Throw(subst_expr(e, map, ctx)?, *p),
        Stmt::Return(e) => Stmt::Return(e.as_ref().map(|e| subst_expr(e, map, ctx)).transpose()?),
        Stmt::Block(b) => Stmt::Block(map_block(b, map, ctx)?),
        Stmt::If(i) => {
            let mut ni = i.clone();
            ni.condition = subst_expr(&i.condition, map, ctx)?;
            ni.then_branch = map_block(&i.then_branch, map, ctx)?;
            ni.else_branch = ni.else_branch.as_ref().map(|b| map_block(b, map, ctx)).transpose()?;
            Stmt::If(ni)
        }
        Stmt::While(w) => {
            let mut nw = w.clone();
            nw.condition = subst_expr(&w.condition, map, ctx)?;
            nw.body = map_block(&w.body, map, ctx)?;
            Stmt::While(nw)
        }
        Stmt::DoWhile(d) => {
            let mut nd = d.clone();
            nd.condition = subst_expr(&d.condition, map, ctx)?;
            nd.body = map_block(&d.body, map, ctx)?;
            Stmt::DoWhile(nd)
        }
        Stmt::For(f) => {
            let mut nf = f.clone();
            nf.iterable = subst_expr(&f.iterable, map, ctx)?;
            nf.body = map_block(&f.body, map, ctx)?;
            Stmt::For(nf)
        }
        Stmt::CFor(c) => {
            let mut nc = c.clone();
            nc.init = nc
                .init
                .as_ref()
                .map(|s| subst_stmt(s.as_ref(), map, ctx))
                .transpose()?
                .map(Box::new);
            nc.condition = nc.condition.as_ref().map(|e| subst_expr(e, map, ctx)).transpose()?;
            nc.increment = nc.increment.as_ref().map(|e| subst_expr(e, map, ctx)).transpose()?;
            nc.body = map_block(&c.body, map, ctx)?;
            Stmt::CFor(nc)
        }
        Stmt::Switch(sw) => {
            let mut ns = sw.clone();
            ns.expr = subst_expr(&sw.expr, map, ctx)?;
            ns.cases = ns
                .cases
                .iter()
                .map(|c| Ok(crate::ast::Case { pattern: c.pattern.clone(), body: map_block(&c.body, map, ctx)? }))
                .collect::<Result<Vec<_>, String>>()?;
            ns.default = ns.default.as_ref().map(|b| map_block(b, map, ctx)).transpose()?;
            Stmt::Switch(ns)
        }
        Stmt::TryCatch { try_block, catch_var, catch_var_type, catch_block, span } => Stmt::TryCatch {
            try_block: map_block(try_block, map, ctx)?,
            catch_var: catch_var.clone(),
            catch_var_type: catch_var_type.as_ref().map(|t| map_type(t, map, ctx)).transpose()?,
            catch_block: map_block(catch_block, map, ctx)?,
            span: *span,
        },
        Stmt::TupleDestructure { names, expr, mutable, span } => Stmt::TupleDestructure {
            names: names.clone(),
            expr: subst_expr(expr, map, ctx)?,
            mutable: *mutable,
            span: *span,
        },
        Stmt::WhileLet(w) => {
            let mut nw = w.clone();
            nw.expr = subst_expr(&w.expr, map, ctx)?;
            nw.body = map_block(&w.body, map, ctx)?;
            Stmt::WhileLet(nw)
        }
        Stmt::With(w) => {
            let mut nw = w.clone();
            nw.resource_expr = subst_expr(&w.resource_expr, map, ctx)?;
            nw.var_type = nw.var_type.as_ref().map(|t| map_type(t, map, ctx)).transpose()?;
            nw.body = map_block(&w.body, map, ctx)?;
            Stmt::With(nw)
        }
        other => other.clone(),
    })
}

fn subst_expr(
    e: &Expr,
    map: &HashMap<String, Type>,
    ctx: &mut Ctx,
) -> Result<Expr, String> {
    Ok(match e {
        Expr::Cast(inner, t, s) => Expr::Cast(
            Box::new(subst_expr(inner, map, ctx)?),
            map_type(t, map, ctx)?,
            *s,
        ),
        Expr::New(t, args, s) => Expr::New(
            map_type(t, map, ctx)?,
            args.iter().map(|a| subst_expr(a, map, ctx)).collect::<Result<_, _>>()?,
            *s,
        ),
        Expr::Is(inner, t, s) => Expr::Is(
            Box::new(subst_expr(inner, map, ctx)?),
            map_type(t, map, ctx)?,
            *s,
        ),
        Expr::Binary(a, op, b, s) => Expr::Binary(
            Box::new(subst_expr(a, map, ctx)?),
            op.clone(),
            Box::new(subst_expr(b, map, ctx)?),
            *s,
        ),
        Expr::Unary(op, a, s) => Expr::Unary(op.clone(), Box::new(subst_expr(a, map, ctx)?), *s),
        Expr::Call(c, args, type_args, s) => Expr::Call(
            Box::new(subst_expr(c, map, ctx)?),
            args.iter().map(|a| subst_expr(a, map, ctx)).collect::<Result<_, _>>()?,
            type_args.clone(),
            *s,
        ),
        Expr::Assign(a, b, s) => Expr::Assign(
            Box::new(subst_expr(a, map, ctx)?),
            Box::new(subst_expr(b, map, ctx)?),
            *s,
        ),
        Expr::Ternary { condition, then_expr, else_expr, span } => Expr::Ternary {
            condition: Box::new(subst_expr(condition, map, ctx)?),
            then_expr: Box::new(subst_expr(then_expr, map, ctx)?),
            else_expr: Box::new(subst_expr(else_expr, map, ctx)?),
            span: *span,
        },
        Expr::Index(a, b, s) => Expr::Index(
            Box::new(subst_expr(a, map, ctx)?),
            Box::new(subst_expr(b, map, ctx)?),
            *s,
        ),
        Expr::MemberAccess(o, m, s) => Expr::MemberAccess(Box::new(subst_expr(o, map, ctx)?), m.clone(), *s),
        Expr::StaticCall { class_name, method, args, span } => Expr::StaticCall {
            class_name: class_name.clone(),
            method: method.clone(),
            args: args.iter().map(|a| subst_expr(a, map, ctx)).collect::<Result<_, _>>()?,
            span: *span,
        },
        Expr::ErrorPropagation(inner, s) => Expr::ErrorPropagation(Box::new(subst_expr(inner, map, ctx)?), *s),
        Expr::Tuple(xs, s) => Expr::Tuple(
            xs.iter().map(|x| subst_expr(x, map, ctx)).collect::<Result<_, _>>()?,
            *s,
        ),
        Expr::Closure { params, return_type, body, expr, captured_vars, span } => Expr::Closure {
            params: params
                .iter()
                .map(|(n, t)| Ok((n.clone(), map_type(t, map, ctx)?)))
                .collect::<Result<Vec<_>, String>>()?,
            return_type: map_type(return_type, map, ctx)?,
            body: map_block(body, map, ctx)?,
            expr: expr.as_ref().map(|e| subst_expr(e, map, ctx)).transpose()?.map(Box::new),
            captured_vars: captured_vars.clone(),
            span: *span,
        },
        other => other.clone(),
    })
}

/// Rewrite a type so generic class references become specialized class names.
fn rewrite_type(t: &Type, ctx: &mut Ctx) -> Result<Type, String> {
    match t {
        Type::Named { name, params } => {
            let rewritten: Vec<Type> = params
                .iter()
                .map(|p| rewrite_type(p, ctx))
                .collect::<Result<_, _>>()?;
            if ctx.generic_classes.contains_key(name) && !rewritten.is_empty() {
                let mangled = specialize_class(name, &rewritten, ctx)?;
                Ok(Type::simple(&mangled))
            } else {
                Ok(Type::Named { name: name.clone(), params: rewritten })
            }
        }
        Type::Ref(inner) => Ok(Type::Ref(Box::new(rewrite_type(inner, ctx)?))),
        Type::MutRef(inner) => Ok(Type::MutRef(Box::new(rewrite_type(inner, ctx)?))),
        Type::Tuple(ts) => Ok(Type::Tuple(
            ts.iter().map(|t| rewrite_type(t, ctx)).collect::<Result<_, _>>()?,
        )),
    }
}

fn specialize(
    name: &str,
    args: &[Expr],
    ctx: &mut Ctx,
    span: crate::ast::Span,
    expected: Option<&Type>,
) -> Result<String, String> {
    let count = ctx.generics[name];
    let base = ctx.defs[name].clone();
    let tparam_names: Vec<String> = base.type_params.iter().map(|p| p.name.clone()).collect();
    let mut deduced: HashMap<String, Type> = HashMap::new();
    if let Some(ret) = &base.return_type {
        if let Some(exp) = expected {
            unify(ret, exp, &tparam_names, &mut deduced);
        }
    }
    let mut targs = Vec::new();
    for (i, pname) in tparam_names.iter().enumerate().take(count) {
        let t = args
            .get(i)
            .and_then(|a| physical_type(a, &ctx.env))
            .map(|t| concrete_param(&t))
            .or_else(|| deduced.get(pname).cloned())
            .ok_or_else(|| format!("wasm: cannot infer generic argument {} of '{}'", i + 1, name))?;
        targs.push(t);
    }
    let map: HashMap<String, Type> = tparam_names
        .iter()
        .cloned()
        .zip(targs.iter().cloned())
        .collect();
    let suffix: Vec<String> = targs.iter().map(mangle_type).collect();
    let new_name = format!("{}__{}", name, suffix.join("_"));
    if !ctx.specs.contains_key(&new_name) {
        let mut f = base;
        f.name = new_name.clone();
        f.type_params.clear();
        f.where_clause.clear();
        f.params = f
            .params
            .iter()
            .map(|p| {
                Ok::<crate::ast::Param, String>(crate::ast::Param {
                    name: p.name.clone(),
                    typ: map_type(&p.typ, &map, ctx)?,
                })
            })
            .collect::<Result<_, _>>()?;
        f.return_type = f.return_type.as_ref().map(|t| map_type(t, &map, ctx)).transpose()?;
        let mut env = ctx.env.clone();
        for p in &f.params {
            env.insert(p.name.clone(), p.typ.clone());
        }
        let saved = std::mem::replace(&mut ctx.env, env);
        let saved_ret = std::mem::replace(&mut ctx.current_return, f.return_type.clone());
        f.body = rewrite_block(&f.body, ctx)?;
        ctx.env = saved;
        ctx.current_return = saved_ret;
        ctx.specs.insert(new_name.clone(), f);
    }
    let _ = span;
    Ok(new_name)
}

fn rewrite_expr(e: &Expr, ctx: &mut Ctx, expected: Option<&Type>) -> Result<Expr, String> {
    let out = match e {
        Expr::Call(callee, args, type_args, span) => {
            let callee_name = match callee.as_ref() {
                Expr::Identifier(n, _) => Some(n.clone()),
                _ => None,
            };
            let arg_expectations: Vec<Option<Type>> = callee_name
                .as_ref()
                .and_then(|n| ctx.fn_params.get(n))
                .map(|ps| ps.iter().cloned().map(Some).collect())
                .unwrap_or_else(|| vec![None; args.len()]);
            let new_args: Vec<Expr> = args
                .iter()
                .zip(arg_expectations.iter())
                .map(|(a, e)| rewrite_expr(a, ctx, e.as_ref()))
                .collect::<Result<_, _>>()?;
            let new_callee = match callee.as_ref() {
                Expr::Identifier(name, _) if ctx.generics.contains_key(name) => {
                    let sp = specialize(name, &new_args, ctx, *span, expected)?;
                    Expr::Identifier(sp, *span)
                }
                other => other.clone(),
            };
            Expr::Call(Box::new(new_callee), new_args, type_args.clone(), *span)
        }
        Expr::Binary(a, op, b, s) => Expr::Binary(
            Box::new(rewrite_expr(a, ctx, None)?),
            op.clone(),
            Box::new(rewrite_expr(b, ctx, None)?),
            *s,
        ),
        Expr::Unary(op, a, s) => Expr::Unary(op.clone(), Box::new(rewrite_expr(a, ctx, None)?), *s),
        Expr::Assign(a, b, s) => Expr::Assign(
            Box::new(rewrite_expr(a, ctx, None)?),
            Box::new(rewrite_expr(b, ctx, expected)?),
            *s,
        ),
        Expr::Ternary { condition, then_expr, else_expr, span } => Expr::Ternary {
            condition: Box::new(rewrite_expr(condition, ctx, None)?),
            then_expr: Box::new(rewrite_expr(then_expr, ctx, expected)?),
            else_expr: Box::new(rewrite_expr(else_expr, ctx, expected)?),
            span: *span,
        },
        Expr::Index(a, b, s) => Expr::Index(
            Box::new(rewrite_expr(a, ctx, None)?),
            Box::new(rewrite_expr(b, ctx, None)?),
            *s,
        ),
        Expr::MemberAccess(o, m, s) => Expr::MemberAccess(Box::new(rewrite_expr(o, ctx, None)?), m.clone(), *s),
        Expr::New(t, args, s) => Expr::New(
            rewrite_type(t, ctx)?,
            args.iter().map(|a| rewrite_expr(a, ctx, None)).collect::<Result<_, _>>()?,
            *s,
        ),
        Expr::StaticCall { class_name, method, args, span } => Expr::StaticCall {
            class_name: class_name.clone(),
            method: method.clone(),
            args: args
                .iter()
                .map(|a| rewrite_expr(a, ctx, None))
                .collect::<Result<_, _>>()?,
            span: *span,
        },
        Expr::Cast(inner, t, s) => {
            let ty = rewrite_type(t, ctx)?;
            Expr::Cast(Box::new(rewrite_expr(inner, ctx, Some(&ty))?), ty, *s)
        }
        Expr::Is(inner, t, s) => Expr::Is(
            Box::new(rewrite_expr(inner, ctx, None)?),
            rewrite_type(t, ctx)?,
            *s,
        ),
        Expr::ErrorPropagation(inner, s) => {
            Expr::ErrorPropagation(Box::new(rewrite_expr(inner, ctx, None)?), *s)
        }
        Expr::Tuple(xs, s) => Expr::Tuple(
            xs.iter()
                .map(|x| rewrite_expr(x, ctx, None))
                .collect::<Result<_, _>>()?,
            *s,
        ),
        other => other.clone(),
    };
    Ok(out)
}

fn rewrite_block(block: &[crate::ast::Stmt], ctx: &mut Ctx) -> Result<Block, String> {
    use crate::ast::Stmt;
    let mut out = Vec::new();
    for s in block {
        out.push(match s {
            Stmt::Expr(e) => Stmt::Expr(rewrite_expr(e, ctx, None)?),
            Stmt::Throw(e, p) => Stmt::Throw(rewrite_expr(e, ctx, None)?, *p),
            Stmt::VarDecl(v) => {
                let mut nv = v.clone();
                nv.typ = nv.typ.as_ref().map(|t| rewrite_type(t, ctx)).transpose()?;
                nv.init = nv
                    .init
                    .as_ref()
                    .map(|e| rewrite_expr(e, ctx, nv.typ.as_ref()))
                    .transpose()?;
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
                nv.typ = nv.typ.as_ref().map(|t| rewrite_type(t, ctx)).transpose()?;
                nv.init = nv
                    .init
                    .as_ref()
                    .map(|e| rewrite_expr(e, ctx, nv.typ.as_ref()))
                    .transpose()?;
                Stmt::ConstDecl(nv)
            }
            Stmt::Return(e) => {
                let expected = ctx.current_return.clone();
                Stmt::Return(
                    e.as_ref()
                        .map(|e| rewrite_expr(e, ctx, expected.as_ref()))
                        .transpose()?,
                )
            }
            Stmt::Block(b) => Stmt::Block(rewrite_block(b, ctx)?),
            Stmt::If(i) => {
                let mut ni = i.clone();
                ni.condition = rewrite_expr(&i.condition, ctx, None)?;
                ni.then_branch = rewrite_block(&i.then_branch, ctx)?;
                ni.else_branch = ni.else_branch.map(|b| rewrite_block(&b, ctx)).transpose()?;
                Stmt::If(ni)
            }
            Stmt::While(w) => {
                let mut nw = w.clone();
                nw.condition = rewrite_expr(&w.condition, ctx, None)?;
                nw.body = rewrite_block(&w.body, ctx)?;
                Stmt::While(nw)
            }
            Stmt::DoWhile(d) => {
                let mut nd = d.clone();
                nd.condition = rewrite_expr(&d.condition, ctx, None)?;
                nd.body = rewrite_block(&d.body, ctx)?;
                Stmt::DoWhile(nd)
            }
            Stmt::For(f) => {
                let mut nf = f.clone();
                nf.iterable = rewrite_expr(&f.iterable, ctx, None)?;
                nf.body = rewrite_block(&f.body, ctx)?;
                Stmt::For(nf)
            }
            Stmt::CFor(c) => {
                let mut nc = c.clone();
                nc.init = nc
                    .init
                    .as_ref()
                    .map(|s| {
                        rewrite_block(std::slice::from_ref(s.as_ref()), ctx)
                            .map(|b| Box::new(b.into_iter().next().unwrap()))
                    })
                    .transpose()?;
                nc.condition = nc.condition.map(|e| rewrite_expr(&e, ctx, None)).transpose()?;
                nc.increment = nc.increment.map(|e| rewrite_expr(&e, ctx, None)).transpose()?;
                nc.body = rewrite_block(&c.body, ctx)?;
                Stmt::CFor(nc)
            }
            Stmt::Switch(sw) => {
                let mut ns = sw.clone();
                ns.expr = rewrite_expr(&sw.expr, ctx, None)?;
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
                    catch_var_type: catch_var_type.as_ref().map(|t| rewrite_type(t, ctx)).transpose()?,
                    catch_block: rewrite_block(catch_block, ctx)?,
                    span: *span,
                }
            }
            Stmt::TupleDestructure { names, expr, mutable, span } => Stmt::TupleDestructure {
                names: names.clone(),
                expr: rewrite_expr(expr, ctx, None)?,
                mutable: *mutable,
                span: *span,
            },
            Stmt::WhileLet(w) => {
                let mut nw = w.clone();
                nw.expr = rewrite_expr(&w.expr, ctx, None)?;
                nw.body = rewrite_block(&w.body, ctx)?;
                Stmt::WhileLet(nw)
            }
            Stmt::With(w) => {
                let mut nw = w.clone();
                nw.resource_expr = rewrite_expr(&w.resource_expr, ctx, None)?;
                nw.var_type = nw.var_type.as_ref().map(|t| rewrite_type(t, ctx)).transpose()?;
                nw.body = rewrite_block(&w.body, ctx)?;
                Stmt::With(nw)
            }
            other => other.clone(),
        });
    }
    Ok(out)
}
