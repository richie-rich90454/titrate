// Expression type inference and expression dispatch

use super::native_bridge;
use super::types as llvm_types;
use super::LlvmBackend;
use crate::ast::{Expr, Literal, Operator, Type, UnOp};
use inkwell::AddressSpace;
use inkwell::types::BasicTypeEnum;
use inkwell::values::BasicValueEnum;

impl<'ctx> LlvmBackend<'ctx> {
    /// Infer the type of an expression. This is a simple local inference
    /// that does not consult the scope; it only works for literals and
    /// simple expressions where the type is obvious. For identifiers, it
    /// looks up the local variable's type.
    pub(super) fn infer_expr_type(&self, expr: &Expr) -> Type {
        match expr {
            Expr::Literal(lit, _) => match lit {
                Literal::Int(_) => Type::simple("int"),
                Literal::Float(_) => Type::simple("double"),
                Literal::Bool(_) => Type::simple("bool"),
                Literal::Char(_) => Type::simple("char"),
                Literal::String(_) => Type::simple("string"),
                Literal::Null => Type::simple("void"),
            },
            Expr::Identifier(name, _) => {
                // First check if we have a stored Titrate type for this variable.
                // Prefer the full type (it retains generic arguments like
                // Box<int>); fall back to the bare name string.
                if let Some(local) = self.locals.get(name) {
                    if let Some(ref full) = local.full_type {
                        return full.clone();
                    }
                    if let Some(ref titrate_type) = local.titrate_type {
                        return Type::simple(titrate_type);
                    }
                }
                self.locals
                    .get(name)
                    .map(|v| {
                        // Reverse-map the LLVM type back to a Titrate type name.
                        self.llvm_basic_type_to_titrate_type(v.ty)
                    })
                    .unwrap_or_else(|| Type::simple("unknown"))
            }
            Expr::Binary(left, op, _, _) => {
                let lt = self.infer_expr_type(left);
                match op {
                    Operator::Eq
                    | Operator::Ne
                    | Operator::Lt
                    | Operator::Gt
                    | Operator::Le
                    | Operator::Ge
                    | Operator::And
                    | Operator::Or => Type::simple("bool"),
                    _ => lt,
                }
            }
            Expr::Unary(UnOp::Not, _, _) => Type::simple("bool"),
            Expr::Unary(UnOp::Neg | UnOp::BitNot, operand, _) => self.infer_expr_type(operand),
            Expr::This(_) => {
                if let Some(ref name) = self.current_class_name {
                    Type::simple(name)
                } else {
                    Type::simple("unknown")
                }
            }
            Expr::Call(callee, _, _) => {
                // Handle bare function calls like toString, parseInt, etc.
                match callee.as_ref() {
                    Expr::Identifier(name, _) => {
                        // User-defined function: use its declared return type.
                        if let Some(fn_decl) = self.function_decls.get(name) {
                            if let Some(ret) = &fn_decl.return_type {
                                return ret.clone();
                            }
                        }
                        match name.as_str() {
                            "toString" => Type::simple("string"),
                            "parseInt" => Type::simple("int"),
                            "println" => Type::simple("void"),
                            _ => Type::simple("unknown"),
                        }
                    }
                    // obj.method(args): infer return type from method and object type.
                    Expr::MemberAccess(obj, method, _) => {
                        let obj_ty = self.infer_expr_type(obj);
                        let type_name = obj_ty.name();
                        // Concrete generic receivers substitute first: the
                        // bare template return (e.g. T) must never leak out.
                        if !obj_ty.params().is_empty() {
                            if let Some(mangled) = self.mono_mangled_for(&obj_ty) {
                                if let Some(ret) =
                                    self.resolve_method_return(&mangled, method)
                                {
                                    return ret;
                                }
                            }
                            if let Some(ret) = self.generic_method_return(&obj_ty, method) {
                                return ret;
                            }
                        }
                        if let Some(ret) = self.resolve_method_return(type_name, method) {
                            return ret;
                        }
                        // Interface dispatch shapes calls by the declared
                        // signature; use its return type (mirrors the
                        // runtime path in compile_call).
                        if self.is_interface_ty(&obj_ty) {
                            if let Some(sig) = self
                                .iface_method_sigs
                                .get(&(type_name.to_string(), method.clone()))
                            {
                                if let Some(ret) = sig.ret.as_ref() {
                                    return ret.clone();
                                }
                            }
                        }
                        let native_prefix: &str = match type_name {
                            "ArrayList" | "array" => "ArrayList",
                            "HashMap" => "HashMap",
                            "string" => "String",
                            other => other,
                        };
                        let native_name = format!("{}_{}", native_prefix, method);
                        // Element-type-aware `ArrayList.get`: an `ArrayList<double>`
                        // yields a double, not the generic string default.
                        if native_prefix == "ArrayList" && method == "get" {
                            return match self.array_element_type_name(obj) {
                                Some(elem) => Type::simple(&elem),
                                None => native_bridge::infer_native_return_type(&native_name),
                            };
                        }
                        native_bridge::infer_native_return_type(&native_name)
                    }
                    _ => Type::simple("unknown"),
                }
            }
            Expr::StaticCall {
                class_name, method, ..
            } => {
                // Return type for known static calls
                match (class_name.as_str(), method.as_str()) {
                    ("ArrayList", "size") => Type::simple("int"),
                    ("ArrayList", "get") => Type::simple("string"),
                    ("Integer", "parseInt") | ("Integer", "parseOr") => Type::simple("int"),
                    ("Integer", "toString")
                    | ("Double", "toString")
                    | ("Long", "toString")
                    | ("Boolean", "toString") => Type::simple("string"),
                    ("Double", "parse") | ("Double", "parseDouble") => Type::simple("double"),
                    ("Long", "parseLong") => Type::simple("long"),
                    ("String", "length") => Type::simple("int"),
                    ("String", "charAt") => Type::simple("string"),
                    ("String", "substring") => Type::simple("string"),
                    ("String", "indexOf") => Type::simple("int"),
                    ("String", "toUpperCase") | ("String", "toLowerCase") => Type::simple("string"),
                    ("String", "trim") | ("String", "trimStart") | ("String", "trimEnd") => {
                        Type::simple("string")
                    }
                    ("String", "startsWith") | ("String", "endsWith") => Type::simple("bool"),
                    ("String", "replace") => Type::simple("string"),
                    ("String", "split") => Type::simple("array"),
                    ("String", "padLeft") | ("String", "padRight") => Type::simple("string"),
                    ("String", "fromCharCode") => Type::simple("string"),
                    ("String", "join") => Type::simple("string"),
                    ("Math", _) | ("MathAdvanced", _) | ("MathTrig", _) => Type::simple("double"),
                    ("Regex", "match") | ("Regex", "fullMatch") | ("Regex", "matchWithFlags") => {
                        Type::simple("bool")
                    }
                    ("Regex", "find") | ("Regex", "replace") | ("Regex", "subN") => {
                        Type::simple("string")
                    }
                    ("Regex", "groupCount") => Type::simple("int"),
                    ("Json", "parse") => Type::simple("JsonValue"),
                    ("Json", "stringify") => Type::simple("string"),
                    ("Hash", _) => Type::simple("string"),
                    ("Base64", _) | ("Hex", _) | ("Url", _) => Type::simple("string"),
                    ("TypeName", "of") => Type::simple("string"),
                    ("Sys", "args") => Type::simple("array"),
                    ("Sys", "env") | ("Sys", "workingDir") => Type::simple("string"),
                    ("Sys", _) => Type::simple("void"),
                    ("Time", "now") | ("Time", "millis") | ("Time", "nanos") => {
                        Type::simple("long")
                    }
                    ("Time", "format") => Type::simple("string"),
                    ("Time", _) => Type::simple("int"),
                    ("Os", "name") | ("Os", "arch") | ("Os", "family") => Type::simple("string"),
                    ("Os", _) => Type::simple("long"),
                    ("Path", "join")
                    | ("Path", "basename")
                    | ("Path", "dirname")
                    | ("Path", "extension") => Type::simple("string"),
                    ("Path", _) => Type::simple("bool"),
                    ("File", "readFile") | ("File", "readLine") | ("File", "readChunk") => {
                        Type::simple("string")
                    }
                    ("File", "readLines") => Type::simple("array"),
                    ("File", "size") | ("File", "tell") | ("File", "lastModified") => {
                        Type::simple("long")
                    }
                    ("File", _) => Type::simple("void"),
                    ("Subprocess", _) => Type::simple("string"),
                    ("Socket", _) => Type::simple("void"),
                    _ => Type::simple("unknown"),
                }
            }
            // MemberAccess: obj.field — infer the field type from the class info.
            Expr::MemberAccess(obj, field, _) => {
                let obj_ty = self.infer_expr_type(obj);
                let class_name = obj_ty.name();
                // First try the Titrate type name cache (populated during class compilation).
                let cache_key = format!("{}_{}", class_name, field);
                if let Some(ty_name) = self.field_titrate_type_names.get(&cache_key) {
                    return Type::simple(ty_name);
                }
                // Fall back to LLVM type reverse-mapping from class_infos.
                if let Some(class_info) = self.class_infos.get(class_name).cloned() {
                    if let Some((_, field_ty)) = class_info.fields.iter().find(|(n, _)| n == field)
                    {
                        return self.llvm_basic_type_to_titrate_type(*field_ty);
                    }
                }
                Type::simple("unknown")
            }
            // Cast: the result type is the cast target type.
            Expr::Cast(_, target, _) => Type::simple(target.name()),
            // Construction: the result type is the constructed type.
            Expr::New(type_name, _, _) => type_name.clone(),
            _ => Type::simple("unknown"),
        }
    }

    /// Resolve the declared return type of a class method, walking the parent
    /// chain for inherited methods.
    pub(super) fn resolve_method_return(&self, class_name: &str, method: &str) -> Option<Type> {
        let mut cur = Some(class_name.to_string());
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        while let Some(cname) = cur {
            if !visited.insert(cname.clone()) {
                break;
            }
            if let Some(methods) = self.class_method_returns.get(&cname) {
                if let Some(ret) = methods.get(method) {
                    return Some(ret.clone());
                }
            }
            cur = self
                .class_decls
                .get(&cname)
                .and_then(|cd| cd.parent.as_ref().map(|t| t.name().to_string()));
        }
        None
    }

    /// Best-effort reverse mapping from an LLVM basic type to a Titrate type.
    pub(super) fn llvm_basic_type_to_titrate_type(&self, ty: BasicTypeEnum<'ctx>) -> Type {
        let s = ty.print_to_string().to_string();
        match s.as_str() {
            "i1" => Type::simple("bool"),
            "i8" => Type::simple("byte"),
            "i16" => Type::simple("short"),
            "i32" => Type::simple("int"),
            "i64" => Type::simple("long"),
            "i128" => Type::simple("vast"),
            "float" => Type::simple("float"),
            "double" => Type::simple("double"),
            "half" => Type::simple("half"),
            "fp128" => Type::simple("quad"),
            _ => {
                // Check if it's the string struct type.
                let string_ty = llvm_types::string_type(self.context);
                if s == string_ty.print_to_string().to_string() {
                    return Type::simple("string");
                }
                Type::simple("unknown")
            }
        }
    }

    /// Compile a literal value to an LLVM constant.
    pub(super) fn compile_literal(
        &self,
        lit: &Literal,
        hint: Option<&Type>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match lit {
            Literal::Int(v) => {
                // Use the hint type if provided, otherwise default to int (i32).
                let ty = hint.cloned().unwrap_or_else(|| Type::simple("int"));
                let int_ty = llvm_types::llvm_type(self.context, &ty)?.into_int_type();
                Ok(int_ty.const_int(*v as u64, false).into())
            }
            Literal::Float(v) => {
                let ty = hint.cloned().unwrap_or_else(|| Type::simple("double"));
                let float_ty = llvm_types::llvm_type(self.context, &ty)?.into_float_type();
                Ok(float_ty.const_float(*v).into())
            }
            Literal::Bool(b) => {
                let i1_ty = self.context.bool_type();
                Ok(i1_ty.const_int(if *b { 1 } else { 0 }, false).into())
            }
            Literal::Char(c) => {
                let i32_ty = self.context.i32_type();
                Ok(i32_ty.const_int(*c as u64, false).into())
            }
            Literal::String(s) => {
                // String literals need &mut self for the counter, so we handle
                // them separately via compile_string_expr.
                // This branch shouldn't be reached; compile_expr handles strings.
                let _ = s;
                Err("string literal should be handled by compile_string_expr".to_string())
            }
            Literal::Null => {
                let i8_ptr = self.context.ptr_type(AddressSpace::default());
                Ok(i8_ptr.const_null().into())
            }
        }
    }

    /// Compile an identifier reference to a loaded value.
    pub(super) fn compile_identifier_load(&self, name: &str) -> Result<BasicValueEnum<'ctx>, String> {
        // Locals (including UPPER_CASE constants) win over the class-name
        // fallback below; otherwise `const MAX = 100` would load null.
        if let Some(var) = self.locals.get(name) {
            return self
                .builder
                .build_load(var.ty, var.ptr, name)
                .map_err(|e| format!("build_load '{}' failed: {:?}", name, e));
        }
        // Check if this is a known class type name (e.g., JsonValue, Regex).
        // In that case, return a null pointer of the appropriate type.
        if self.class_infos.contains_key(name) || name.starts_with(|c: char| c.is_uppercase()) {
            let ptr_ty = self.context.ptr_type(AddressSpace::default());
            return Ok(ptr_ty.const_null().into());
        }
        Err(format!("codegen: unknown variable '{}'", name))
    }

    /// Compile any expression to a `BasicValueEnum`.
    pub(super) fn compile_expr(&mut self, expr: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        match expr {
            Expr::Literal(Literal::String(s), _) => {
                let sv = self.make_string_global(s);
                self.string_value_to_basic(sv)
            }
            Expr::Literal(lit, _) => self.compile_literal(lit, None),
            Expr::Identifier(name, _) => self.compile_identifier_load(name),
            Expr::Binary(left, op, right, _) => self.compile_binary(left, op, right),
            Expr::Unary(op, operand, _) => self.compile_unary(op, operand),
            Expr::Assign(target, value, _) => self.compile_assign(target, value),
            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
                ..
            } => self.compile_ternary(condition, then_expr, else_expr),
            Expr::Call(callee, args, _) => self.compile_call(callee, args),
            Expr::New(type_name, args, _) => self.compile_new(type_name, args),
            Expr::MemberAccess(object, field, _) => self.compile_member_access(object, field),
            Expr::This(span) => self.compile_this(span),
            Expr::Is(obj, ty, _) => self.compile_is(obj, ty),
            Expr::Cast(obj, ty, _) => self.compile_cast(obj, ty),
            Expr::OwnedDeref(inner, _) => self.compile_owned_deref(inner),
            Expr::RefExpr(inner, ref_kind, _) => self.compile_ref_expr(inner, ref_kind),
            Expr::RegionAlloc(ty, init, _) => self.compile_region_alloc(ty, init),
            Expr::UnsafeBlock(block, _) => self.compile_unsafe_block(block),
            Expr::Closure {
                params,
                return_type,
                body,
                expr,
                captured_vars,
                ..
            } => self.compile_closure(params, return_type, body, expr.as_deref(), captured_vars),
            Expr::ErrorPropagation(inner, _) => self.compile_error_propagation(inner),
            Expr::Tuple(elements, _) => self.compile_tuple(elements),
            Expr::Unit(_) => Ok(self.context.i32_type().const_int(0, false).into()),
            Expr::StaticCall {
                class_name,
                method,
                args,
                ..
            } => self.compile_static_call(class_name, method, args),
            _ => Err(format!("codegen: unsupported expression: {:?}", expr)),
        }
    }
}
