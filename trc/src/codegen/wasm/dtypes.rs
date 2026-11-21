//! Detailed type tracking for composite values in the WASM backend.
//!
//! `WTy` describes only the physical WASM representation. `DType` keeps the
//! logical Titrate type so composite operations can choose the right host
//! import variant (for example `list_get_i64` vs `list_get_str`).

use super::json::JsonLit;
use super::{WasmBackend, WTy};
use crate::ast::{Expr, Literal, Type, UnOp};

fn json_dtype(lit: &JsonLit) -> DType {
    match lit {
        JsonLit::Null => DType::Str,
        JsonLit::Bool(_) => DType::Bool,
        JsonLit::Int(_) => DType::I64,
        JsonLit::Float(_) => DType::F64,
        JsonLit::Str(_) => DType::Str,
        JsonLit::Array(xs) => {
            let first = xs.first().map(json_dtype).unwrap_or(DType::Unknown);
            if xs.iter().all(|x| json_dtype(x) == first) {
                DType::List(Box::new(first))
            } else {
                DType::Unknown
            }
        }
        JsonLit::Object(entries) => {
            let first = entries.first().map(|(_, v)| json_dtype(v)).unwrap_or(DType::Unknown);
            if entries.iter().all(|(_, v)| json_dtype(v) == first) {
                DType::Map(Box::new(DType::Str), Box::new(first))
            } else {
                DType::Unknown
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DType {
    Void,
    I64,
    F64,
    Bool,
    Char,
    Str,
    Obj(String),
    Enum(String),
    List(Box<DType>),
    Map(Box<DType>, Box<DType>),
    Tuple(Vec<DType>),
    ResultT(Box<DType>, Box<DType>),
    ClosureT(Vec<DType>, Box<DType>),
    Unknown,
}

impl DType {
    pub(crate) fn wty(&self) -> WTy {
        match self {
            DType::Void => WTy::Void,
            DType::I64 => WTy::I64,
            DType::F64 => WTy::F64,
            DType::Bool => WTy::Bool,
            DType::Char => WTy::Char,
            DType::Str => WTy::Str,
            _ => WTy::Handle,
        }
    }

    /// Host value tag (see `runtime::TAG_*`).
    pub(crate) fn tag(&self) -> i32 {
        match self {
            DType::F64 => 1,
            DType::Bool => 2,
            DType::Str | DType::Char => 3,
            DType::I64 => 0,
            _ => 4,
        }
    }

    /// Suffix used to build host import names such as `list_get_i64`.
    pub(crate) fn suffix(&self) -> &'static str {
        match self {
            DType::I64 => "i64",
            DType::F64 => "f64",
            DType::Bool => "bool",
            DType::Str | DType::Char => "str",
            _ => "obj",
        }
    }

    pub(crate) fn is_obj(&self) -> bool {
        matches!(
            self,
            DType::Obj(_)
                | DType::Enum(_)
                | DType::List(_)
                | DType::Map(_, _)
                | DType::Tuple(_)
                | DType::ResultT(_, _)
                | DType::ClosureT(_, _)
        )
    }
}

impl<'a> WasmBackend<'a> {
    pub(crate) fn dtype_of_type(&self, t: &Type) -> DType {
        match t {
            Type::Ref(inner) | Type::MutRef(inner) => self.dtype_of_type(inner),
            Type::Tuple(types) => DType::Tuple(types.iter().map(|t| self.dtype_of_type(t)).collect()),
            Type::Named { name, params } => match name.as_str() {
                "void" => DType::Void,
                "bool" => DType::Bool,
                "char" => DType::Char,
                "string" => DType::Str,
                "float" | "double" | "half" | "quad" | "f64" => DType::F64,
                "byte" | "short" | "int" | "long" | "vast" | "uvast" | "size" | "u8"
                | "u16" | "u32" | "u64" => DType::I64,
                "ArrayList" => DType::List(Box::new(
                    params.first().map(|t| self.dtype_of_type(t)).unwrap_or(DType::Unknown),
                )),
                "HashMap" => DType::Map(
                    Box::new(params.first().map(|t| self.dtype_of_type(t)).unwrap_or(DType::Unknown)),
                    Box::new(params.get(1).map(|t| self.dtype_of_type(t)).unwrap_or(DType::Unknown)),
                ),
                "Result" => DType::ResultT(
                    Box::new(params.first().map(|t| self.dtype_of_type(t)).unwrap_or(DType::Unknown)),
                    Box::new(params.get(1).map(|t| self.dtype_of_type(t)).unwrap_or(DType::Unknown)),
                ),
                "fn" => {
                    if params.is_empty() {
                        DType::ClosureT(vec![], Box::new(DType::Void))
                    } else {
                        let ret = self.dtype_of_type(params.last().unwrap());
                        let args = params[..params.len() - 1]
                            .iter()
                            .map(|t| self.dtype_of_type(t))
                            .collect();
                        DType::ClosureT(args, Box::new(ret))
                    }
                }
                n if self.enums.contains_key(n) => DType::Enum(n.to_string()),
                n => DType::Obj(n.to_string()),
            },
        }
    }

    pub(crate) fn dtype_of(&self, e: &Expr) -> DType {
        match e {
            Expr::Literal(l, _) => match l {
                Literal::Int(_) => DType::I64,
                Literal::Float(_) => DType::F64,
                Literal::Bool(_) => DType::Bool,
                Literal::Char(_) => DType::Char,
                Literal::String(_) | Literal::Null => DType::Str,
            },
            Expr::Identifier(n, _) => self
                .local_dtypes
                .get(n)
                .or_else(|| self.global_dtypes.get(n))
                .cloned()
                .unwrap_or(DType::Unknown),
            Expr::This(_) => self
                .current_class
                .as_ref()
                .map(|c| DType::Obj(c.clone()))
                .unwrap_or(DType::Unknown),
            Expr::New(t, _, _) => self.dtype_of_type(t),
            Expr::Call(callee, args, _) => {
                if let Expr::MemberAccess(obj, method, _) = callee.as_ref() {
                    if let Expr::Identifier(module, _) = obj.as_ref() {
                        if module == "Json" && method == "parse" {
                            if let Some(Expr::Literal(Literal::String(s), _)) = args.first() {
                                if let Ok(lit) = super::json::parse(s) {
                                    return json_dtype(&lit);
                                }
                            }
                        }
                    }
                }
                match callee.as_ref() {
                Expr::Identifier(n, _) if n == "Ok" || n == "Err" => DType::ResultT(
                    Box::new(args.first().map(|a| self.dtype_of(a)).unwrap_or(DType::Void)),
                    Box::new(DType::Unknown),
                ),
                Expr::Identifier(n, _) if self.variant_enum.contains_key(n) => {
                    let en = &self.variant_enum[n];
                    DType::Enum(en.clone())
                }
                Expr::Identifier(n, _) => {
                    if let Some(DType::ClosureT(_, ret)) = self.local_dtypes.get(n) {
                        return (**ret).clone();
                    }
                    self.functions.get(n).map(|f| f.ret_dtype.clone()).unwrap_or(DType::Unknown)
                }
                Expr::MemberAccess(obj, method, _) => {
                    if let Expr::Identifier(module, _) = obj.as_ref() {
                        return self.infer_named_dtype(module, method);
                    }
                    let od = self.dtype_of(obj);
                    match &od {
                        DType::List(elem) => match method.as_str() {
                            "get" | "remove" | "removeAt" | "pop" | "max" | "min" => (**elem).clone(),
                            "size" | "length" | "indexOf" | "lastIndexOf" => DType::I64,
                            "contains" | "isEmpty" => DType::Bool,
                            "toString" | "toArray" => DType::Str,
                            "addAll" | "reverse" | "clear" | "set" | "add" => DType::Void,
                            _ => DType::Unknown,
                        },
                        DType::Map(_, val) => match method.as_str() {
                            "get" | "remove" | "put" | "pop" | "setDefault" | "putIfAbsent"
                            | "replace" => (**val).clone(),
                            "size" => DType::I64,
                            "containsKey" | "containsValue" | "isEmpty" => DType::Bool,
                            "clear" => DType::Void,
                            "toString" => DType::Str,
                            "keys" | "values" | "entries" => {
                                DType::List(Box::new((**val).clone()))
                            }
                            _ => DType::Unknown,
                        },
                        DType::ResultT(ok, _) => match method.as_str() {
                            "unwrap" => (**ok).clone(),
                            "isOk" | "isErr" => DType::Bool,
                            _ => DType::Unknown,
                        },
                        DType::Obj(cls) => self
                            .all_class_methods
                            .get(cls)
                            .and_then(|m| m.get(method))
                            .and_then(|md| md.return_type.as_ref())
                            .map(|t| self.dtype_of_type(t))
                            .unwrap_or(DType::Unknown),
                        _ => DType::Unknown,
                    }
                }
                _ => DType::Unknown,
                }
            }
            Expr::StaticCall { class_name, method, args, .. } => {
                if class_name == "Json" && method == "parse" {
                    if let Some(Expr::Literal(Literal::String(s), _)) = args.first() {
                        if let Ok(lit) = super::json::parse(s) {
                            return json_dtype(&lit);
                        }
                    }
                }
                self.infer_named_dtype(class_name, method)
            }
            Expr::Cast(_, t, _) => self.dtype_of_type(t),
            Expr::Is(_, _, _) => DType::Bool,
            Expr::Unary(UnOp::Not, _, _) => DType::Bool,
            Expr::Unary(_, inner, _) => self.dtype_of(inner),
            Expr::Binary(l, op, r, _) => match op {
                crate::ast::Operator::Eq
                | crate::ast::Operator::Ne
                | crate::ast::Operator::Lt
                | crate::ast::Operator::Gt
                | crate::ast::Operator::Le
                | crate::ast::Operator::Ge
                | crate::ast::Operator::And
                | crate::ast::Operator::Or => DType::Bool,
                crate::ast::Operator::Add
                    if self.dtype_of(l) == DType::Str || self.dtype_of(r) == DType::Str =>
                {
                    DType::Str
                }
                _ => {
                    let lt = self.dtype_of(l);
                    let rt = self.dtype_of(r);
                    if lt == DType::F64 || rt == DType::F64 {
                        DType::F64
                    } else {
                        DType::I64
                    }
                }
            },
            Expr::Assign(l, _, _) => self.dtype_of(l),
            Expr::Ternary { then_expr, .. } => self.dtype_of(then_expr),
            Expr::MemberAccess(obj, f, _) => {
                let od = self.dtype_of(obj);
                if let DType::Obj(cls) = &od {
                    if let Some(t) = self.field_type(cls, f) {
                        return self.dtype_of_type(&t);
                    }
                }
                DType::Unknown
            }
            Expr::Tuple(elems, _) => DType::Tuple(elems.iter().map(|e| self.dtype_of(e)).collect()),
            Expr::Closure {
                params,
                return_type,
                ..
            } => DType::ClosureT(
                params.iter().map(|(_, t)| self.dtype_of_type(t)).collect(),
                Box::new(self.dtype_of_type(return_type)),
            ),
            Expr::ErrorPropagation(inner, _) => match self.dtype_of(inner) {
                DType::ResultT(ok, _) => *ok,
                _ => DType::Unknown,
            },
            Expr::OwnedDeref(inner, _) | Expr::RegionAlloc(_, inner, _) => self.dtype_of(inner),
            Expr::Range(_, _, _) | Expr::RangeInclusive(_, _, _) => DType::Obj("Range".to_string()),
            Expr::Unit(_) => DType::Void,
            _ => DType::Unknown,
        }
    }

    fn infer_named_dtype(&self, module: &str, method: &str) -> DType {
        match (module, method) {
            (_, "toString") => DType::Str,
            ("io" | "console", "println" | "print" | "eprintln" | "log") => DType::Void,
            ("Integer" | "Long" | "Int", _) => DType::I64,
            ("Double" | "Float", _) => DType::F64,
            ("String", "length" | "indexOf" | "compareTo" | "lastIndexOf") => DType::I64,
            ("String", "startsWith" | "endsWith" | "equals") => DType::Bool,
            ("String", _) => DType::Str,
            ("Math" | "MathAdvanced" | "MathTrig" | "Boolean" | "Char", _) => DType::F64,
            ("File", "readFile" | "read") => DType::Str,
            ("File", "exists") => DType::Bool,
            ("File", _) => DType::Void,
            ("Json", "stringify") => DType::Str,
            ("Json", "parse") => DType::Unknown,
            _ => DType::Unknown,
        }
    }
}

