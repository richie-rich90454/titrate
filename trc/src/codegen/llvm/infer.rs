// Expression type inference and expression dispatch

use super::native_bridge;
use super::types as llvm_types;
use super::LlvmBackend;
use crate::ast::{ClassMember, Expr, Literal, Operator, Type, UnOp};
use inkwell::AddressSpace;
use inkwell::types::BasicTypeEnum;
use inkwell::values::BasicValueEnum;

impl<'ctx> LlvmBackend<'ctx> {
    /// Infer the type of an expression. This is a simple local inference
    /// that does not consult the scope; it only works for literals and
    /// simple expressions where the type is obvious. For identifiers, it
    /// looks up the local variable's type.
    pub(super) fn infer_expr_type(&self, expr: &Expr) -> Type {
        self.infer_expr_type_at_depth(expr, 0)
    }

    fn infer_expr_type_at_depth(&self, expr: &Expr, depth: u32) -> Type {
        if depth >= 256 {
            return Type::simple("unknown");
        }
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
                // A Rust-style `self` receiver has the enclosing class type.
                if name == "self" {
                    if let Some(ref cls) = self.current_class_name {
                        return Type::simple(cls);
                    }
                }
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
                    .or_else(|| {
                        // Globals use the declared type: containers share the
                        // string LLVM layout but need a different marshalling tag.
                        self.globals.get(name).map(|g| g.titrate_ty.clone())
                    })
                    .unwrap_or_else(|| Type::simple("unknown"))
            }
            Expr::Binary(left, op, _, _) => {
                let lt = self.infer_expr_type_at_depth(left, depth + 1);
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
            Expr::Unary(UnOp::Neg | UnOp::BitNot, operand, _) => {
                self.infer_expr_type_at_depth(operand, depth + 1)
            }
            Expr::This(_) => {
                if let Some(ref name) = self.current_class_name {
                    Type::simple(name)
                } else {
                    Type::simple("unknown")
                }
            }
            Expr::Call(callee, call_args, explicit_type_args, _) => {
                // Handle bare function calls like toString, parseInt, etc.
                let args = call_args;
                match callee.as_ref() {
                    Expr::Identifier(name, _) => {
                        // A local closure returns its recorded signature's type;
                        // without this the result is typed "unknown" and native
                        // call marshalling corrupts numeric values.
                        if let Some(local) = self.locals.get(name) {
                            if let Some(sig) = &local.closure_sig {
                                return sig.ret.clone();
                            }
                            if let Some(full) = &local.full_type {
                                if full.name() == "fn" {
                                    if let Some(ret) = full.params().last() {
                                        return ret.clone();
                                    }
                                }
                            }
                        }
                        // User-defined function: use its declared return type.
                        if let Some(fn_decl) = self.function_decls.get(name) {
                            if let Some(ret) = &fn_decl.return_type {
                                return ret.clone();
                            }
                        }
                        // Generic function: deduce concrete type arguments from
                        // the call arguments and substitute into its return type.
                        if let Some(overloads) = self.generic_fn_decls.get(name) {
                            if let Some(decl) = overloads.iter().find(|d| d.params.len() == args.len()) {
                                let names: Vec<String> = decl.type_params.iter().map(|tp| tp.name.clone()).collect();
                                let formals: Vec<Type> = decl.params.iter().map(|p| p.typ.clone()).collect();
                                let actuals: Vec<Type> = args.iter().map(|a| self.infer_expr_type_at_depth(a, depth + 1)).collect();
                                // Explicit type arguments bind the
                                // specialization; deduction is only a fallback.
                                let deduced = if !explicit_type_args.is_empty() {
                                    Some(explicit_type_args.clone())
                                } else {
                                    Self::deduce_type_args(&formals, &actuals, &names)
                                };
                                if let Some(deduced) = deduced {
                                    let map: std::collections::HashMap<String, Type> =
                                        names.into_iter().zip(deduced).collect();
                                    if let Some(ret) = &decl.return_type {
                                        return crate::bytecode::compiler::Compiler::substitute_type(ret, &map);
                                    }
                                }
                            }
                        }
                        if native_bridge::is_native_function(name) {
                            return native_bridge::infer_native_return_type(name);
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
                        let obj_ty = self.infer_expr_type_at_depth(obj, depth + 1);
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
                        // A module-qualified call (`Math.PI()`) has a module
                        // identifier as its receiver, which is not a local and
                        // therefore infers as "unknown". Use the module name as
                        // the native prefix so its return type is correct.
                        let module_prefix = if type_name == "unknown" {
                            if let Expr::Identifier(id, _) = obj.as_ref() {
                                if !self.locals.contains_key(id) {
                                    Some(id.clone())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        // A module-qualified call to a user function must use
                        // its declared return type; the synthetic `Module_fn`
                        // native name below would otherwise infer `int`.
                        if let Some(module) = &module_prefix {
                            // Generic function: prefer explicit type arguments
                            // over deduction (a `null` literal infers void,
                            // which must not override a `<string>` annotation).
                            if let Some(overloads) = self.generic_fn_decls.get(method) {
                                if let Some(decl) =
                                    overloads.iter().find(|d| d.params.len() == args.len())
                                {
                                    let names: Vec<String> = decl
                                        .type_params
                                        .iter()
                                        .map(|tp| tp.name.clone())
                                        .collect();
                                    let formals: Vec<Type> = decl
                                        .params
                                        .iter()
                                        .map(|p| p.typ.clone())
                                        .collect();
                                    let actuals: Vec<Type> =
                                        args.iter().map(|a| self.infer_expr_type_at_depth(a, depth + 1)).collect();
                                    // Explicit type arguments take precedence.
                                    let deduced = if !explicit_type_args.is_empty() {
                                        Some(explicit_type_args.clone())
                                    } else {
                                        Self::deduce_type_args(&formals, &actuals, &names)
                                    };
                                    if let Some(deduced) = deduced {
                                        let map: std::collections::HashMap<String, Type> =
                                            names.into_iter().zip(deduced).collect();
                                        if let Some(ret) = &decl.return_type {
                                            return crate::bytecode::compiler::Compiler::substitute_type(ret, &map);
                                        }
                                    }
                                }
                            }
                            let arg_types: Vec<Type> =
                                args.iter().map(|a| self.infer_expr_type_at_depth(a, depth + 1)).collect();
                            if let Some(ret) =
                                self.module_fn_return_type(module, method, &arg_types)
                            {
                                return ret;
                            }
                        }
                        let native_prefix: &str = match module_prefix.as_deref().unwrap_or(type_name) {
                            "ArrayList" | "array" => "ArrayList",
                            "HashMap" => "HashMap",
                            "string" => "String",
                            other => other,
                        };
                        let native_name = format!("{}_{}", native_prefix, method);
                        // Element-type-aware `ArrayList.get`: an `ArrayList<double>`
                        // yields a double, not the generic string default.
                        if native_prefix == "ArrayList" && method == "get" {
                            return match self.array_element_type(obj) {
                                Some(elem) => elem,
                                None => native_bridge::infer_native_return_type(&native_name),
                            };
                        }
                        if native_prefix == "HashMap" && method == "get" {
                            return obj_ty
                                .params()
                                .get(1)
                                .cloned()
                                .unwrap_or_else(|| native_bridge::infer_native_return_type(&native_name));
                        }
                        native_bridge::infer_native_return_type(&native_name)
                    }
                    _ => Type::simple("unknown"),
                }
            }
            Expr::StaticCall {
                class_name,
                method,
                args,
                ..
            } => {
                // A module-qualified generic function resolves through the
                // generic declarations, not the fixed native name table.
                if let Some(overloads) = self.generic_fn_decls.get(method) {
                    if let Some(decl) = overloads.iter().find(|d| d.params.len() == args.len()) {
                        let names: Vec<String> = decl
                            .type_params
                            .iter()
                            .map(|tp| tp.name.clone())
                            .collect();
                        let formals: Vec<Type> =
                            decl.params.iter().map(|p| p.typ.clone()).collect();
                        let actuals: Vec<Type> =
                            args.iter().map(|a| self.infer_expr_type_at_depth(a, depth + 1)).collect();
                        if let Some(deduced) =
                            Self::deduce_type_args(&formals, &actuals, &names)
                        {
                            let map: std::collections::HashMap<String, Type> =
                                names.into_iter().zip(deduced).collect();
                            if let Some(ret) = &decl.return_type {
                                return crate::bytecode::compiler::Compiler::substitute_type(ret, &map);
                            }
                        }
                    }
                }
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
                    ("Math", "MAX_INT")
                    | ("Math", "MIN_INT")
                    | ("Math", "absInt")
                    | ("Math", "round") => Type::simple("long"),
                    ("Math", "sign") => Type::simple("int"),
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
                let obj_ty = self.infer_expr_type_at_depth(obj, depth + 1);

                // A monomorphized generic class stores the substituted field
                // types under its mangled name; the bare name still holds the
                // unresolved type parameter (e.g. `B` instead of `int`).
                let class_name = self
                    .mono_mangled_for(&obj_ty)
                    .unwrap_or_else(|| obj_ty.name().to_string());
                // Inherited fields are not repeated in a subclass's ClassInfo,
                // so walk the parent chain before giving up on the type.
                let mut current = Some(self.resolve_class_name(&class_name));
                let mut visited = std::collections::HashSet::new();
                while let Some(name) = current {
                    if !visited.insert(name.clone()) {
                        break;
                    }
                    let cache_key = format!("{}_{}", name, field);
                    if let Some(ty) = self.field_titrate_type_names.get(&cache_key) {
                        return ty.clone();
                    }
                    if let Some(class_info) = self.class_infos.get(&name) {
                        if let Some((_, field_ty)) =
                            class_info.fields.iter().find(|(n, _)| n == field)
                        {
                            // An object field's LLVM type is an opaque pointer;
                            // reverse mapping it yields "unknown". Keep walking
                            // so the declaring ancestor's cache can supply the
                            // real class name.
                            let inferred = self.llvm_basic_type_to_titrate_type(*field_ty);
                            if inferred.name() != "unknown" {
                                return inferred;
                            }
                        }
                    }
                    if let Some(class_decl) = self.class_decls.get(&name) {
                        if let Some(ClassMember::Field(f)) = class_decl
                            .members
                            .iter()
                            .find(|m| matches!(m, ClassMember::Field(f) if f.name == *field))
                        {
                            return f.typ.clone();
                        }
                    }
                    current = self
                        .class_infos
                        .get(&name)
                        .and_then(|ci| ci.parent.clone())
                        .or_else(|| {
                            self.class_decls
                                .get(&name)
                                .and_then(|cd| cd.parent.as_ref().map(|t| t.name().to_string()))
                        })
                        .map(|p| self.resolve_class_name(&p));
                }
                Type::simple("unknown")
            }
            // A ternary's type is its then-branch type; both branches share
            // a type in well-formed programs.
            Expr::Ternary { then_expr, .. } => self.infer_expr_type_at_depth(then_expr, depth + 1),
            // Cast: the result type is the cast target type, including its
            // generic arguments (needed to index a cast container result).
            Expr::Cast(_, target, _) => target.clone(),
            // Construction: the result type is the constructed type.
            Expr::New(type_name, _, _) => type_name.clone(),
            // Tuple literals carry their element types, enabling `t[0]` to
            // infer the selected slot's type.
            Expr::Tuple(elements, _) => {
                Type::Tuple(elements.iter().map(|e| self.infer_expr_type_at_depth(e, depth + 1)).collect())
            }
            Expr::Index(obj, index, _) => {
                if let Some(elem) = self.array_element_type(obj) {
                    return elem;
                }
                let obj_ty = self.infer_expr_type_at_depth(obj, depth + 1);
                if let Type::Tuple(elems) = &obj_ty {
                    if let Expr::Literal(Literal::Int(n), _) = index.as_ref() {
                        if *n >= 0 {
                            if let Some(elem) = elems.get(*n as usize) {
                                return elem.clone();
                            }
                        }
                    }
                }
                if obj_ty.name() == "string" {
                    return Type::simple("string");
                }
                // A numeric slot of a class instance uses the declared field
                // type (field 0 in the LLVM struct is the vtable pointer).
                if let Expr::Literal(Literal::Int(n), _) = index.as_ref() {
                    if *n >= 0 {
                        if let Some(info) = self.class_infos.get(obj_ty.name()) {
                            let slot = (*n as usize).saturating_add(1);
                            if let Some((_, field_ty)) = info.fields.get(slot) {
                                return self.llvm_basic_type_to_titrate_type(*field_ty);
                            }
                        }
                    }
                }
                Type::simple("unknown")
            }
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
                match llvm_types::llvm_type(self.context, &ty)? {
                    // A literal can be wider than the inferred hint (e.g. a
                    // `long` constant passed where the hint says `int`). LLVM
                    // asserts on const_int when the value does not fit; widen
                    // to i64 in that case so the full value survives.
                    BasicTypeEnum::IntType(it) => {
                        let bits = it.get_bit_width();
                        let too_wide = if *v < 0 {
                            bits < 64 && *v < -(1i64 << (bits - 1))
                        } else {
                            bits < 64 && (*v as u64) >= (1u64 << bits)
                        };
                        let target = if too_wide {
                            self.context.i64_type()
                        } else {
                            it
                        };
                        Ok(target.const_int(*v as u64, *v < 0).into())
                    }
                    BasicTypeEnum::FloatType(ft) => Ok(ft.const_float(*v as f64).into()),
                    other => Err(format!("integer literal cannot have type {:?}", other)),
                }
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
        // A Rust-style `self` receiver aliases the implicit `this` pointer.
        if name == "self" {
            if let Some(this_ptr) = self.current_this {
                return Ok(this_ptr.into());
            }
        }
        // Locals (including UPPER_CASE constants) win over the class-name
        // fallback below; otherwise `const MAX = 100` would load null.
        if let Some(var) = self.locals.get(name) {
            return self
                .builder
                .build_load(var.ty, var.ptr, name)
                .map_err(|e| format!("build_load '{}' failed: {:?}", name, e));
        }
        if let Some(g) = self.globals.get(name) {
            return self
                .builder
                .build_load(g.ty, g.ptr, name)
                .map_err(|e| format!("build_load global '{}' failed: {:?}", name, e));
        }
        // Check if this is a known class type name (e.g., JsonValue, Regex).
        // In that case, return a null pointer of the appropriate type.
        if self.class_infos.contains_key(name) || name.starts_with(|c: char| c.is_uppercase()) {
            let ptr_ty = self.context.ptr_type(AddressSpace::default());
            return Ok(ptr_ty.const_null().into());
        }
        Err(format!("codegen: unknown variable '{}'", name))
    }

    /// Maximum nesting allowed while lowering expressions. The parser already
    /// bounds source nesting, but synthesized/desugared ASTs can exceed it.
    const MAX_EXPR_DEPTH: u32 = 256;

    /// Lower `obj[index]` by target shape. Tuple and class values use literal
    /// field slots; strings use `String_charAt`; every container (including
    /// results of calls and interface fat pointers) goes through the shared
    /// `{ i64, ptr }` layout. Unsupported combinations are reported instead
    /// of being read as null or guessed from the LLVM value kind.
    pub(super) fn compile_index_expr(
        &mut self,
        obj: &Expr,
        index: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let obj_ty = self.infer_expr_type(obj);
        let ty_name = obj_ty.name();

        if matches!(obj, Expr::Tuple(_, _)) || matches!(obj_ty, Type::Tuple(_)) {
            return self.compile_slot_index(obj, index, "tuple");
        }
        // A user class instance lowers to a pointer whose first field is the
        // vtable; a numeric index addresses one of its declared fields.
        if !llvm_types::is_primitive_name(ty_name)
            && !super::arrays::is_container_layout_name(ty_name)
            && self.class_infos.contains_key(ty_name)
        {
            return self.compile_slot_index(obj, index, ty_name);
        }
        if ty_name == "string" {
            // String indexing returns a one-character string.
            let arg_vals = vec![self.compile_expr(obj)?, self.compile_expr(index)?];
            return self.compile_native_call(
                "String_charAt",
                &arg_vals,
                &[obj_ty, Type::simple("int")],
            );
        }
        if self.array_element_type(obj).is_some()
            || matches!(ty_name, "array" | "ArrayList" | "HashMap")
        {
            return self.compile_array_get_string(obj, index);
        }
        if matches!(ty_name, "Variant" | "unknown") {
            // An opaque handle may hold a container whose elements are
            // TitrateValues; read the raw element payload.
            return self.compile_array_get_raw_payload(obj, index);
        }
        Err(format!(
            "codegen: unsupported index target type '{}'; indexable values are arrays, ArrayList, HashMap, string, tuple and class instances",
            ty_name
        ))
    }

    /// Compile any expression to a `BasicValueEnum`. Bounds recursive lowering
    /// so deeply nested constant expressions return an error instead of
    /// overflowing the native stack.
    pub(super) fn compile_expr(&mut self, expr: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        if self.expr_depth >= Self::MAX_EXPR_DEPTH {
            return Err(format!(
                "codegen: expression recursion depth {} exceeded",
                Self::MAX_EXPR_DEPTH
            ));
        }
        self.expr_depth += 1;
        let result = self.compile_expr_inner(expr);
        self.expr_depth -= 1;
        result
    }

    fn compile_expr_inner(&mut self, expr: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
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
            Expr::Call(callee, args, type_args, _) => {
                self.compile_call_with_type_args(callee, args, type_args)
            }
            Expr::New(type_name, args, _) => self.compile_new(type_name, args),
            Expr::MemberAccess(object, field, _) => self.compile_member_access(object, field),
            Expr::This(span) => self.compile_this(span),
            Expr::Is(obj, ty, _) => self.compile_is(obj, ty),
            Expr::Cast(obj, ty, _) => self.compile_cast(obj, ty),
            Expr::OwnedDeref(inner, _) => self.compile_owned_deref(inner),
            Expr::RefExpr(inner, ref_kind, _) => self.compile_ref_expr(inner, ref_kind),
            Expr::RegionAlloc(ty, init, _) => self.compile_region_alloc(ty, init),
            Expr::UnsafeBlock(block, _) => self.compile_unsafe_block(block),
            Expr::RegionBlock(block, _) => {
                // Region allocations are freed at scope end; a tail call
                // inside would skip that cleanup.
                self.cleanup_scope_depth += 1;
                let v = self.compile_unsafe_block(block)?;
                self.cleanup_scope_depth -= 1;
                Ok(v)
            }
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
            Expr::Index(obj, index, _) => self.compile_index_expr(obj, index),
            _ => Err(format!("codegen: unsupported expression: {:?}", expr)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LlvmBackend;
    use crate::ast::{Access, Declaration, Expr, FnDecl, Literal, Operator, Span, Type};
    use crate::{analyzer, lexer, parser};
    use inkwell::context::Context;

    fn backend_with_pick<'ctx>(context: &'ctx Context) -> LlvmBackend<'ctx> {
        let tokens =
            lexer::tokenize("public fn pick<T>(x: T): T { return x; } public fn main(): void {}")
                .expect("tokenize");
        let ast = parser::parse(tokens).expect("parse");
        let typed = analyzer::analyze(&ast).expect("analyze");
        let mut backend = LlvmBackend::new(context, "test");
        for decl in &typed.declarations {
            if let Declaration::Function(f) = decl {
                backend
                    .generic_fn_decls
                    .entry(f.name.clone())
                    .or_default()
                    .push(f.clone());
            }
        }
        backend
    }

    fn int_arg() -> Expr {
        Expr::Literal(Literal::Int(3), Span::unknown())
    }

    #[test]
    fn explicit_type_arg_overrides_deduction() {
        let context = Context::create();
        let backend = backend_with_pick(&context);
        let expr = Expr::Call(
            Box::new(Expr::Identifier("pick".to_string(), Span::unknown())),
            vec![int_arg()],
            vec![Type::simple("double")],
            Span::unknown(),
        );
        assert_eq!(backend.infer_expr_type(&expr).name(), "double");
    }

    #[test]
    fn module_qualified_generic_uses_explicit_type_arg() {
        let context = Context::create();
        let backend = backend_with_pick(&context);
        let expr = Expr::Call(
            Box::new(Expr::MemberAccess(
                Box::new(Expr::Identifier("holder".to_string(), Span::unknown())),
                "pick".to_string(),
                Span::unknown(),
            )),
            vec![int_arg()],
            vec![Type::simple("double")],
            Span::unknown(),
        );
        assert_eq!(backend.infer_expr_type(&expr).name(), "double");
    }

    #[test]
    fn static_generic_call_infers_substituted_return_type() {
        let context = Context::create();
        let backend = backend_with_pick(&context);
        let expr = Expr::StaticCall {
            class_name: "Holder".to_string(),
            method: "pick".to_string(),
            args: vec![int_arg()],
            span: Span::unknown(),
        };
        assert_eq!(backend.infer_expr_type(&expr).name(), "int");
    }

    /// A field inherited from a parent class must keep its declared type when
    /// inferred through the child receiver (`this.root` in a subclass).
    #[test]
    fn inherited_field_type_is_resolved_through_parent_chain() {
        let source = r#"
            class Tk { }
            class Base { public Tk root; }
            class Child extends Base { }
        "#;
        let tokens = lexer::tokenize(source).expect("tokenize");
        let ast = parser::parse(tokens).expect("parse");
        let typed = analyzer::analyze(&ast).expect("analyze");
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "inherited_field");
        for decl in &typed.declarations {
            if let Declaration::Class(c) = decl {
                backend.class_decls.insert(c.name.clone(), c.clone());
                backend.compile_class_decl(c).expect("compile class");
            }
        }
        backend.current_class_name = Some("Child".to_string());
        let expr = Expr::MemberAccess(
            Box::new(Expr::This(Span::unknown())),
            "root".to_string(),
            Span::unknown(),
        );
        let inferred = backend.infer_expr_type(&expr);
        assert_eq!(inferred.name(), "Tk", "cache: {:?}", backend.field_titrate_type_names);
    }

    #[test]
    fn deeply_nested_expression_is_rejected() {
        // A synthetic 300-deep tree needs more than the default 2 MiB test
        // thread stack to unwind the recursive lowering frames before the
        // 256-depth guard can fire. Run it on a larger stack so the guard is
        // what rejects the expression, not the OS stack limit.
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let context = Context::create();
                let mut backend = LlvmBackend::new(&context, "depth-test");
                let mut expr = int_arg();
                for _ in 0..300 {
                    expr = Expr::Binary(
                        Box::new(expr),
                        Operator::Add,
                        Box::new(int_arg()),
                        Span::unknown(),
                    );
                }

                let err = backend
                    .compile_expr(&expr)
                    .expect_err("deeply nested constants must error, not overflow the stack");
                assert!(err.contains("recursion depth"), "{}", err);
                assert_eq!(backend.expr_depth, 0, "depth guard must unwind cleanly");
            })
            .expect("spawn depth-test thread")
            .join()
            .expect("depth-test thread must not panic");
    }

    #[test]
    fn out_of_range_integer_literals_widen_without_asserting() {
        let context = Context::create();
        let backend = LlvmBackend::new(&context, "literal-test");

        let wide = backend
            .compile_literal(&Literal::Int(300), Some(&Type::simple("byte")))
            .expect("a literal wider than its hint must be widened, not asserted");
        assert!(wide.is_int_value());
        let wide_int = wide.into_int_value();
        assert_eq!(wide_int.get_type(), context.i64_type());
        assert_eq!(wide_int.get_zero_extended_constant(), Some(300));

        let negative_min = backend
            .compile_literal(&Literal::Int(i64::MIN), Some(&Type::simple("int")))
            .expect("i64::MIN must widen without an LLVM const_int assertion");
        assert_eq!(
            negative_min.into_int_value().get_sign_extended_constant(),
            Some(i64::MIN)
        );

        let vast = backend
            .compile_literal(&Literal::Int(42), Some(&Type::simple("vast")))
            .expect("vast literals should compile");
        assert_eq!(vast.into_int_value().get_type(), context.i128_type());
    }

    #[test]
    fn container_call_result_infers_indexed_element_type() {
        let context = Context::create();
        let mut backend = backend_with_pick(&context);
        backend.function_decls.insert(
            "make".to_string(),
            FnDecl {
                access: Access::Public,
                name: "make".to_string(),
                type_params: vec![],
                params: vec![],
                return_type: Some(Type::generic("ArrayList", vec![Type::simple("double")])),
                body: vec![],
                sugar: false,
                where_clause: vec![],
                span: Span::unknown(),
            },
        );
        let call = Expr::Call(
            Box::new(Expr::Identifier("make".to_string(), Span::unknown())),
            vec![],
            vec![],
            Span::unknown(),
        );
        let index = Expr::Index(Box::new(call), Box::new(int_arg()), Span::unknown());
        assert_eq!(backend.infer_expr_type(&index).name(), "double");
    }

    #[test]
    fn tuple_and_string_index_types() {
        let context = Context::create();
        let backend = backend_with_pick(&context);
        let tuple = Expr::Tuple(
            vec![
                Expr::Literal(Literal::Int(1), Span::unknown()),
                Expr::Literal(Literal::String("x".to_string()), Span::unknown()),
            ],
            Span::unknown(),
        );
        let first = Expr::Index(
            Box::new(tuple.clone()),
            Box::new(Expr::Literal(Literal::Int(0), Span::unknown())),
            Span::unknown(),
        );
        assert_eq!(backend.infer_expr_type(&first).name(), "int");
        let second = Expr::Index(
            Box::new(tuple),
            Box::new(Expr::Literal(Literal::Int(1), Span::unknown())),
            Span::unknown(),
        );
        assert_eq!(backend.infer_expr_type(&second).name(), "string");
        let string_index = Expr::Index(
            Box::new(Expr::Literal(Literal::String("abc".to_string()), Span::unknown())),
            Box::new(int_arg()),
            Span::unknown(),
        );
        assert_eq!(backend.infer_expr_type(&string_index).name(), "string");
    }

    #[test]
    fn unsupported_index_target_is_reported() {
        let context = Context::create();
        let mut backend = LlvmBackend::new(&context, "index-error-test");
        let err = backend
            .compile_index_expr(&int_arg(), &int_arg())
            .err()
            .expect("scalars must not be indexed");
        assert!(
            err.contains("unsupported index target type 'int'"),
            "{}",
            err
        );
    }
}
