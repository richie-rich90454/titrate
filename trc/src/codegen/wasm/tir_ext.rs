//! Extended TIR-driven emission path.
//!
//! [`crate::codegen::wasm::lower_tir_to_wasm`] emits the TIR constructs it can
//! model and rejects the rest. This module sits in front of it: it rewrites
//! the additional constructs into the vocabulary that lowerer understands,
//! then hands the expanded program to the same emitter so both paths produce
//! the same binary format.
//!
//! Currently expanded here:
//! - `for (x in collection)` over an explicitly typed `ArrayList<T>` or
//!   `HashMap<K, V>` becomes an indexed `while` loop that drives the existing
//!   `list_*` / `map_*` host imports (container iteration helper).
//! - type-independent container aliases (`ArrayList.size`, `HashMap.keys`,
//!   ...) become the corresponding host calls.
//! - compound `HashMap.getOrDefault` becomes a TIR ternary over the existing
//!   `map_contains_*` / `map_get_*` imports. `putIfAbsent` needs a boxed value
//!   and is reported as unmapped rather than lowered incorrectly.
//! - derived numeric methods (`log1p`, `expm1`, `sinh`, `cosh`, `tanh`,
//!   `sign`) become `math_exp` / `math_ln` calls plus TIR arithmetic.
//!
//! Anything that cannot be mapped produces an explicit `wasm TIR ext:`
//! diagnostic. No construct is silently dropped.

use std::collections::{HashMap, HashSet};

use crate::ast::{Literal, Operator, Span, UnOp};
use crate::tir::{CallTarget, NameId, SimdHint, TirExpr, TirProgram, TirStmt, TirType};

use super::containers::{self, NativeRewriteKind};
use super::numeric::{self, DerivedNumericOp, NumericPlan};
use super::tir_lower::{lower_tir_to_wasm, TirWasmShape};
use super::{runtime, WTy};

/// Structural view of the extended emission, including the base shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedShape {
    pub base: TirWasmShape,
    /// Number of `TirStmt::For` loops expanded into indexed loops.
    pub expanded_fors: usize,
    /// Number of calls rewritten to host imports or derived expressions.
    pub rewritten_calls: usize,
}

fn tir_wty(ty: &TirType, program: &TirProgram) -> Option<WTy> {
    match ty {
        TirType::Named { name, .. } => match program.name(*name)? {
            "void" => Some(WTy::Void),
            "bool" => Some(WTy::Bool),
            "char" => Some(WTy::Char),
            "string" => Some(WTy::Str),
            "float" | "double" | "half" | "quad" | "f64" => Some(WTy::F64),
            "byte" | "short" | "int" | "long" | "vast" | "uvast" | "size" | "u8" | "u16"
            | "u32" | "u64" => Some(WTy::I64),
            _ => Some(WTy::Handle),
        },
        TirType::Ref(_) | TirType::MutRef(_) | TirType::Tuple(_) => Some(WTy::Handle),
    }
}

fn collect_declared_types(block: &[TirStmt], out: &mut HashMap<NameId, TirType>) {
    for statement in block {
        match statement {
            TirStmt::Let { name, typ: Some(typ), .. } => {
                out.insert(*name, typ.clone());
            }
            TirStmt::Block(body) => collect_declared_types(body, out),
            TirStmt::If { then_branch, else_branch, .. } => {
                collect_declared_types(then_branch, out);
                if let Some(body) = else_branch {
                    collect_declared_types(body, out);
                }
            }
            TirStmt::While { body, .. } | TirStmt::For { body, .. } => {
                collect_declared_types(body, out);
            }
            _ => {}
        }
    }
}

fn lit_i64(value: i64, span: Span) -> TirExpr {
    TirExpr::Literal(Literal::Int(value), span)
}

fn lit_f64(value: f64, span: Span) -> TirExpr {
    TirExpr::Literal(Literal::Float(value), span)
}

fn binary(op: Operator, left: TirExpr, right: TirExpr, span: Span) -> TirExpr {
    TirExpr::Binary {
        op,
        left: Box::new(left),
        right: Box::new(right),
        span,
    }
}

struct Rewriter {
    program: TirProgram,
    var_types: HashMap<NameId, TirType>,
    used_names: HashSet<String>,
    counter: u32,
    expanded_fors: usize,
    rewritten_calls: usize,
}

impl Rewriter {
    fn new(program: TirProgram) -> Self {
        let mut var_types = HashMap::new();
        for function in &program.functions {
            for param in &function.params {
                var_types.insert(param.name, param.typ.clone());
            }
            collect_declared_types(&function.body, &mut var_types);
        }
        for global in &program.globals {
            if let Some(typ) = &global.typ {
                var_types.insert(global.name, typ.clone());
            }
        }
        let used_names = (0..program.names.len())
            .filter_map(|index| {
                let id = NameId::new(u16::try_from(index).unwrap_or(u16::MAX));
                program.names.get(id).map(str::to_string)
            })
            .collect();
        Self {
            program,
            var_types,
            used_names,
            counter: 0,
            expanded_fors: 0,
            rewritten_calls: 0,
        }
    }

    fn name_of(&self, id: NameId) -> String {
        self.program.name(id).unwrap_or("<invalid name>").to_string()
    }

    fn intern(&mut self, name: &str) -> Result<NameId, String> {
        let id = self.program.names.intern(name)?;
        self.used_names.insert(name.to_string());
        Ok(id)
    }

    fn fresh(&mut self, hint: &str) -> Result<NameId, String> {
        loop {
            let candidate = format!("__tir_ext_{}_{}", hint, self.counter);
            self.counter = self
                .counter
                .checked_add(1)
                .ok_or_else(|| "wasm TIR ext: synthesized-name counter overflow".to_string())?;
            if !self.used_names.contains(&candidate) {
                return self.intern(&candidate);
            }
        }
    }

    fn ty_named(&mut self, name: &str, params: Vec<TirType>) -> Result<TirType, String> {
        Ok(TirType::Named { name: self.intern(name)?, params })
    }

    fn infer_wty(&self, expression: &TirExpr) -> Option<WTy> {
        match expression {
            TirExpr::Literal(literal, _) => Some(match literal {
                Literal::Int(_) => WTy::I64,
                Literal::Float(_) => WTy::F64,
                Literal::Bool(_) => WTy::Bool,
                Literal::Char(_) => WTy::Char,
                Literal::String(_) => WTy::Str,
                Literal::Null => WTy::Handle,
            }),
            TirExpr::Var(name, _) => self.var_types.get(name).and_then(|ty| tir_wty(ty, &self.program)),
            TirExpr::This(_) => Some(WTy::Handle),
            TirExpr::Unit(_) => Some(WTy::Void),
            TirExpr::Cast { typ, .. } => tir_wty(typ, &self.program),
            TirExpr::Ternary { then_expr, .. } => self.infer_wty(then_expr),
            TirExpr::New { .. } | TirExpr::Tuple { .. } => Some(WTy::Handle),
            _ => None,
        }
    }

    fn native_call(&mut self, host: &str, args: Vec<TirExpr>, span: Span) -> Result<TirExpr, String> {
        if runtime::import_index(host).is_none() {
            return Err(format!("wasm TIR ext: '{}' is not in the host runtime contract", host));
        }
        let name = self.intern(host)?;
        Ok(TirExpr::Call {
            target: CallTarget::Native { name },
            type_args: Vec::new(),
            args,
            span,
        })
    }

    fn run(&mut self) -> Result<(), String> {
        let functions = std::mem::take(&mut self.program.functions);
        let mut rewritten = Vec::with_capacity(functions.len());
        for mut function in functions {
            self.rewrite_block(&mut function.body)?;
            rewritten.push(function);
        }
        self.program.functions = rewritten;
        Ok(())
    }

    fn rewrite_block(&mut self, block: &mut Vec<TirStmt>) -> Result<(), String> {
        let statements = std::mem::take(block);
        let mut out = Vec::with_capacity(statements.len());
        for statement in statements {
            match statement {
                TirStmt::For { var, iterable, body, span, .. } => {
                    let iterable = self.rewrite_expr(iterable)?;
                    let mut body = body;
                    self.rewrite_block(&mut body)?;
                    out.extend(self.desugar_for(var, &iterable, body, span)?);
                    self.expanded_fors += 1;
                }
                TirStmt::Block(mut body) => {
                    self.rewrite_block(&mut body)?;
                    out.push(TirStmt::Block(body));
                }
                TirStmt::Expr(expression) => {
                    out.push(TirStmt::Expr(self.rewrite_expr(expression)?));
                }
                TirStmt::Let { name, typ, init, mutable, span } => {
                    let init = match init {
                        Some(expression) => Some(self.rewrite_expr(expression)?),
                        None => None,
                    };
                    if let Some(typ) = &typ {
                        self.var_types.insert(name, typ.clone());
                    }
                    out.push(TirStmt::Let { name, typ, init, mutable, span });
                }
                TirStmt::Return(value) => {
                    let value = match value {
                        Some(expression) => Some(self.rewrite_expr(expression)?),
                        None => None,
                    };
                    out.push(TirStmt::Return(value));
                }
                TirStmt::If { cond, then_branch, else_branch, span } => {
                    let cond = self.rewrite_expr(cond)?;
                    let mut then_branch = then_branch;
                    self.rewrite_block(&mut then_branch)?;
                    let else_branch = match else_branch {
                        Some(mut body) => {
                            self.rewrite_block(&mut body)?;
                            Some(body)
                        }
                        None => None,
                    };
                    out.push(TirStmt::If { cond, then_branch, else_branch, span });
                }
                TirStmt::While { cond, body, simd, span } => {
                    let cond = self.rewrite_expr(cond)?;
                    let mut body = body;
                    self.rewrite_block(&mut body)?;
                    out.push(TirStmt::While { cond, body, simd, span });
                }
                other => out.push(other),
            }
        }
        *block = out;
        Ok(())
    }

    fn rewrite_expr(&mut self, expression: TirExpr) -> Result<TirExpr, String> {
        Ok(match expression {
            TirExpr::Unary { op, operand, span } => TirExpr::Unary {
                op,
                operand: Box::new(self.rewrite_expr(*operand)?),
                span,
            },
            TirExpr::Binary { op, left, right, span } => TirExpr::Binary {
                op,
                left: Box::new(self.rewrite_expr(*left)?),
                right: Box::new(self.rewrite_expr(*right)?),
                span,
            },
            TirExpr::Assign { target, value, span } => TirExpr::Assign {
                target: Box::new(self.rewrite_expr(*target)?),
                value: Box::new(self.rewrite_expr(*value)?),
                span,
            },
            TirExpr::Cast { value, typ, span } => TirExpr::Cast {
                value: Box::new(self.rewrite_expr(*value)?),
                typ,
                span,
            },
            TirExpr::Is { value, typ, span } => TirExpr::Is {
                value: Box::new(self.rewrite_expr(*value)?),
                typ,
                span,
            },
            TirExpr::Ref { value, kind, span } => TirExpr::Ref {
                value: Box::new(self.rewrite_expr(*value)?),
                kind,
                span,
            },
            TirExpr::Field { object, field, slot, span } => TirExpr::Field {
                object: Box::new(self.rewrite_expr(*object)?),
                field,
                slot,
                span,
            },
            TirExpr::New { class, args, span } => {
                let mut rewritten = Vec::with_capacity(args.len());
                for arg in args {
                    rewritten.push(self.rewrite_expr(arg)?);
                }
                TirExpr::New { class, args: rewritten, span }
            }
            TirExpr::Tuple { items, span } => {
                let mut rewritten = Vec::with_capacity(items.len());
                for item in items {
                    rewritten.push(self.rewrite_expr(item)?);
                }
                TirExpr::Tuple { items: rewritten, span }
            }
            TirExpr::Ternary { condition, then_expr, else_expr, span } => TirExpr::Ternary {
                condition: Box::new(self.rewrite_expr(*condition)?),
                then_expr: Box::new(self.rewrite_expr(*then_expr)?),
                else_expr: Box::new(self.rewrite_expr(*else_expr)?),
                span,
            },
            TirExpr::Call { target, type_args: _, args, span } => {
                let mut rewritten = Vec::with_capacity(args.len());
                for arg in args {
                    rewritten.push(self.rewrite_expr(arg)?);
                }
                self.rewrite_call(target, rewritten, span)?
            }
            other => other,
        })
    }

    fn rewrite_call(
        &mut self,
        target: CallTarget,
        args: Vec<TirExpr>,
        span: Span,
    ) -> Result<TirExpr, String> {
        let name = match &target {
            CallTarget::Native { name } => self.name_of(*name),
            _ => return Ok(TirExpr::Call { target, type_args: Vec::new(), args, span }),
        };

        if let Some(host) = containers::host_alias(&name) {
            self.rewritten_calls += 1;
            return self.native_call(host, args, span);
        }

        if name.starts_with("HashMap.") || name.starts_with("Map.") {
            let key = args.get(1).and_then(|arg| self.infer_wty(arg)).unwrap_or(WTy::Unknown);
            let value = if args.len() >= 3 {
                self.infer_wty(&args[2]).unwrap_or(WTy::Unknown)
            } else {
                WTy::Unknown
            };
            if let Some(rewrite) = containers::plan_native_typed(&name, key, value) {
                self.rewritten_calls += 1;
                return self.emit_container_rewrite(rewrite.kind, args, span);
            }
            return Err(format!(
                "wasm TIR ext: container native '{}' has no typed lowering",
                name
            ));
        }

        if let Some(NumericPlan::Derived(op)) = numeric::plan(&name) {
            self.rewritten_calls += 1;
            return self.emit_numeric_derived(op, args, span);
        }

        Ok(TirExpr::Call { target, type_args: Vec::new(), args, span })
    }

    fn emit_container_rewrite(
        &mut self,
        kind: NativeRewriteKind,
        args: Vec<TirExpr>,
        span: Span,
    ) -> Result<TirExpr, String> {
        match kind {
            NativeRewriteKind::Host(host) => self.native_call(&host, args, span),
            NativeRewriteKind::GetOrDefault { contains, get } => {
                if args.len() < 3 {
                    return Err("wasm TIR ext: getOrDefault expects three arguments".to_string());
                }
                let map = args[0].clone();
                let key = args[1].clone();
                let fallback = args[2].clone();
                let condition = self.native_call(&contains, vec![map.clone(), key.clone()], span)?;
                let primary = self.native_call(&get, vec![map, key], span)?;
                Ok(TirExpr::Ternary {
                    condition: Box::new(condition),
                    then_expr: Box::new(primary),
                    else_expr: Box::new(fallback),
                    span,
                })
            }
            NativeRewriteKind::PutIfAbsent { .. } => Err(
                "wasm TIR ext: putIfAbsent needs a boxed value; use the AST lowering".to_string(),
            ),
        }
    }

    fn emit_numeric_derived(
        &mut self,
        op: DerivedNumericOp,
        args: Vec<TirExpr>,
        span: Span,
    ) -> Result<TirExpr, String> {
        if args.is_empty() {
            return Err("wasm TIR ext: numeric method expects an argument".to_string());
        }
        let x = args[0].clone();
        Ok(match op {
            DerivedNumericOp::Log1p => {
                let inner = binary(Operator::Add, x, lit_f64(1.0, span), span);
                self.native_call("math_ln", vec![inner], span)?
            }
            DerivedNumericOp::Expm1 => {
                let exp = self.native_call("math_exp", vec![x], span)?;
                binary(Operator::Sub, exp, lit_f64(1.0, span), span)
            }
            DerivedNumericOp::Sinh | DerivedNumericOp::Cosh => {
                let exp_x = self.native_call("math_exp", vec![x.clone()], span)?;
                let neg_x = TirExpr::Unary { op: UnOp::Neg, operand: Box::new(x), span };
                let exp_neg = self.native_call("math_exp", vec![neg_x], span)?;
                let numerator = if op == DerivedNumericOp::Sinh {
                    binary(Operator::Sub, exp_x, exp_neg, span)
                } else {
                    binary(Operator::Add, exp_x, exp_neg, span)
                };
                binary(Operator::Div, numerator, lit_f64(2.0, span), span)
            }
            DerivedNumericOp::Tanh => {
                let doubled = binary(Operator::Mul, x, lit_f64(2.0, span), span);
                let exp = self.native_call("math_exp", vec![doubled], span)?;
                let numerator = binary(Operator::Sub, exp.clone(), lit_f64(1.0, span), span);
                let denominator = binary(Operator::Add, exp, lit_f64(1.0, span), span);
                binary(Operator::Div, numerator, denominator, span)
            }
            DerivedNumericOp::Sign => {
                let negative = TirExpr::Ternary {
                    condition: Box::new(binary(Operator::Lt, x.clone(), lit_f64(0.0, span), span)),
                    then_expr: Box::new(lit_f64(-1.0, span)),
                    else_expr: Box::new(lit_f64(0.0, span)),
                    span,
                };
                TirExpr::Ternary {
                    condition: Box::new(binary(Operator::Gt, x, lit_f64(0.0, span), span)),
                    then_expr: Box::new(lit_f64(1.0, span)),
                    else_expr: Box::new(negative),
                    span,
                }
            }
        })
    }

    fn iterable_type(&self, expression: &TirExpr) -> Result<(TirType, String, TirType), String> {
        let typ = match expression {
            TirExpr::Var(name, _) => self.var_types.get(name).cloned().ok_or_else(|| {
                format!("wasm TIR ext: cannot infer the type of for-in iterable '{}'", self.name_of(*name))
            })?,
            other => {
                return Err(format!(
                    "wasm TIR ext: unsupported for-in iterable {:?}; bind it to a typed local first",
                    other
                ))
            }
        };
        let (container, element) = match &typ {
            TirType::Named { name, params } => {
                let container = self.name_of(*name);
                let element = params.first().cloned().ok_or_else(|| {
                    format!("wasm TIR ext: for-in over '{}' needs an explicit element type", container)
                })?;
                (container, element)
            }
            _ => {
                return Err(
                    "wasm TIR ext: for-in iterable must have a named collection type".to_string()
                )
            }
        };
        Ok((typ, container, element))
    }

    fn desugar_for(
        &mut self,
        var: NameId,
        iterable: &TirExpr,
        body: Vec<TirStmt>,
        span: Span,
    ) -> Result<Vec<TirStmt>, String> {
        let (container_ty, container, element_ty) = self.iterable_type(iterable)?;
        let element = tir_wty(&element_ty, &self.program)
            .ok_or_else(|| "wasm TIR ext: invalid for-in element type".to_string())?;
        let plan = containers::iteration_plan(&container, element)?;

        let source = self.fresh("src")?;
        let index = self.fresh("idx")?;
        let length = self.fresh("len")?;
        let source_expr = TirExpr::Var(source, span);
        let index_expr = TirExpr::Var(index, span);
        let int_ty = self.ty_named("int", Vec::new())?;

        let mut prefix = vec![TirStmt::Let {
            name: source,
            typ: Some(container_ty),
            init: Some(iterable.clone()),
            mutable: false,
            span,
        }];

        let collection = if plan.needs_keys {
            let keys = self.fresh("keys")?;
            let list_ty = self.ty_named("ArrayList", vec![element_ty.clone()])?;
            let keys_call = self.native_call("map_keys", vec![source_expr.clone()], span)?;
            prefix.push(TirStmt::Let {
                name: keys,
                typ: Some(list_ty),
                init: Some(keys_call),
                mutable: false,
                span,
            });
            TirExpr::Var(keys, span)
        } else {
            source_expr
        };

        let size_host: &str = if plan.needs_keys { "list_size" } else { plan.size_host.as_str() };
        let size_call = self.native_call(size_host, vec![collection.clone()], span)?;
        prefix.push(TirStmt::Let {
            name: index,
            typ: Some(int_ty.clone()),
            init: Some(lit_i64(0, span)),
            mutable: true,
            span,
        });
        prefix.push(TirStmt::Let {
            name: length,
            typ: Some(int_ty),
            init: Some(size_call),
            mutable: false,
            span,
        });

        let get_call = self.native_call(&plan.get_host, vec![collection, index_expr.clone()], span)?;
        let mut loop_body = vec![TirStmt::Let {
            name: var,
            typ: Some(element_ty),
            init: Some(get_call),
            mutable: false,
            span,
        }];
        loop_body.extend(body);
        loop_body.push(TirStmt::Expr(TirExpr::Assign {
            target: Box::new(index_expr.clone()),
            value: Box::new(binary(
                Operator::Add,
                index_expr.clone(),
                lit_i64(1, span),
                span,
            )),
            span,
        }));

        prefix.push(TirStmt::While {
            cond: binary(Operator::Lt, index_expr, TirExpr::Var(length, span), span),
            body: loop_body,
            simd: SimdHint::None,
            span,
        });
        Ok(prefix)
    }
}

/// Lower a TIR program through the extended path, returning bytes and shape.
pub fn lower_tir_extended_to_wasm(
    program: &TirProgram,
) -> Result<(Vec<u8>, ExtendedShape), String> {
    let mut rewriter = Rewriter::new(program.clone());
    rewriter.run()?;
    let (bytes, base) = lower_tir_to_wasm(&rewriter.program)?;
    Ok((
        bytes,
        ExtendedShape {
            base,
            expanded_fors: rewriter.expanded_fors,
            rewritten_calls: rewriter.rewritten_calls,
        },
    ))
}

/// Lower a TIR program through the extended path to WASM bytes.
pub fn compile_tir_extended_to_wasm(program: &TirProgram) -> Result<Vec<u8>, String> {
    lower_tir_extended_to_wasm(program).map(|(bytes, _)| bytes)
}

#[cfg(test)]
mod parity_tests {
    use super::*;

    const MAGIC: [u8; 8] = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

    fn read_leb(bytes: &[u8], position: &mut usize) -> u32 {
        let mut value = 0u32;
        let mut shift = 0;
        loop {
            assert!(*position < bytes.len(), "truncated LEB128 value");
            let byte = bytes[*position];
            *position += 1;
            value |= u32::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return value;
            }
            shift += 7;
        }
    }

    fn sections(bytes: &[u8]) -> Vec<(u8, &[u8])> {
        assert_eq!(&bytes[..8], &MAGIC);
        let mut position = 8usize;
        let mut result = Vec::new();
        while position < bytes.len() {
            let id = bytes[position];
            position += 1;
            let size = read_leb(bytes, &mut position) as usize;
            let end = position + size;
            assert!(end <= bytes.len(), "section {} is truncated", id);
            result.push((id, &bytes[position..end]));
            position = end;
        }
        result
    }

    fn section<'a, 'b>(sections: &'b [(u8, &'a [u8])], id: u8) -> &'a [u8] {
        sections
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .unwrap_or_else(|| panic!("missing section {}", id))
            .1
    }

    fn compile_source(source: &str) -> (Vec<u8>, TirProgram, ExtendedShape) {
        let tokens = crate::lexer::tokenize(source).expect("tokenize");
        let ast = crate::parser::parse(tokens).expect("parse");
        let tir = crate::tir::lower(&ast).expect("lower to TIR");
        let (bytes, shape) = lower_tir_extended_to_wasm(&tir).expect("lower to WASM");
        (bytes, tir, shape)
    }

    /// Assert the binary's structural contract without instantiating it.
    fn assert_module(bytes: &[u8], tir: &TirProgram) {
        let sections = sections(bytes);
        for id in [1u8, 2, 3, 5, 6, 7, 10] {
            assert!(sections.iter().any(|(candidate, _)| *candidate == id), "missing section {}", id);
        }

        let type_section = section(&sections, 1);
        let mut position = 0;
        assert_eq!(
            read_leb(type_section, &mut position) as usize,
            crate::codegen::wasm::runtime::IMPORTS.len() + tir.functions.len(),
            "type section must declare imports plus defined functions"
        );

        let function_section = section(&sections, 3);
        let mut position = 0;
        assert_eq!(read_leb(function_section, &mut position) as usize, tir.functions.len());

        let export_section = section(&sections, 7);
        let mut position = 0;
        let count = read_leb(export_section, &mut position);
        let mut names = Vec::new();
        for _ in 0..count {
            let length = read_leb(export_section, &mut position) as usize;
            let end = position + length;
            assert!(end <= export_section.len(), "truncated export name");
            names.push(std::str::from_utf8(&export_section[position..end]).unwrap().to_string());
            position = end;
            assert!(position < export_section.len(), "missing export kind");
            position += 1;
            let _ = read_leb(export_section, &mut position);
        }
        assert!(names.contains(&"main".to_string()), "{:?}", names);
        assert!(names.contains(&"memory".to_string()), "{:?}", names);
        assert!(names.contains(&"heap_ptr".to_string()), "{:?}", names);
    }

    #[test]
    fn parity_arithmetic_branches_and_native_math() {
        let source = r#"
            public fn main(): void {
                let n: int = 3;
                if (n > 2) {
                    n = n + 1;
                }
                while (n < 10) {
                    n = n + 1;
                }
                io::println(MathAdvanced.sqrt(4.0));
                io::println(n);
            }
        "#;
        let (bytes, tir, shape) = compile_source(source);
        assert_module(&bytes, &tir);
        assert_eq!(shape.expanded_fors, 0);
        assert!(shape.base.calls.iter().any(|call| {
            call.name == "MathAdvanced.sqrt" && call.target.starts_with("host:")
        }));
    }

    #[test]
    fn parity_for_in_over_typed_list() {
        let source = r#"
            public fn main(): void {
                let xs: ArrayList<int> = new ArrayList<int>();
                for (x in xs) {
                    io::println(x);
                }
            }
        "#;
        let (bytes, tir, shape) = compile_source(source);
        assert_module(&bytes, &tir);
        assert_eq!(shape.expanded_fors, 1);
        assert!(shape.base.calls.iter().any(|call| call.target == "list_size"));
        assert!(shape.base.calls.iter().any(|call| call.target == "list_get_i64"));
    }

    #[test]
    fn parity_field_access_and_class_construction() {
        let source = r#"
            public class Box {
                public int value;
            }
            public fn main(): void {
                let b = new Box();
                io::println(b.value);
            }
        "#;
        let (bytes, tir, shape) = compile_source(source);
        assert_module(&bytes, &tir);
        assert_eq!(shape.expanded_fors, 0);
        assert!(shape.base.field_accesses.iter().any(|field| field.field == "value"));
    }

    #[test]
    fn unsupported_iterable_is_an_explicit_error() {
        let source = r#"
            public fn main(): void {
                let n: int = 3;
                for (x in n) {
                    io::println(x);
                }
            }
        "#;
        let tokens = crate::lexer::tokenize(source).expect("tokenize");
        let ast = crate::parser::parse(tokens).expect("parse");
        let tir = crate::tir::lower(&ast).expect("lower to TIR");
        let error = compile_tir_extended_to_wasm(&tir).unwrap_err();
        assert!(error.contains("explicit element type"), "{}", error);
    }
}
