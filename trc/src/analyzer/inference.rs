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

                        // Handle static method calls on builtin wrappers (Integer.parseInt, etc.)
                        if let ast::Expr::Identifier(class_name, _) = obj.as_ref() {
                            if let Some(Symbol::Variable { .. }) = scope.borrow().lookup(class_name) {
                                // Check if this is a known static method
                                let result = match (class_name.as_str(), method.as_str()) {
                                    ("Integer", "parseInt") | ("Integer", "parseOr") => Some(ast::Type::simple("int")),
                                    ("Double", "parse") | ("Double", "parseDouble") => Some(ast::Type::simple("double")),
                                    ("Long", "parseLong") => Some(ast::Type::simple("long")),
                                    ("Float", "toF32Bits") | ("Float", "fromF32Bits") => Some(ast::Type::simple("float")),
                                    ("String", "length") => Some(ast::Type::simple("int")),
                                    ("String", "charAt") => Some(ast::Type::simple("string")),
                                    ("String", "substring") => Some(ast::Type::simple("string")),
                                    ("String", "indexOf") => Some(ast::Type::simple("int")),
                                    ("String", "toUpperCase") | ("String", "toLowerCase") => Some(ast::Type::simple("string")),
                                    ("String", "trim") | ("String", "trimStart") | ("String", "trimEnd") => Some(ast::Type::simple("string")),
                                    ("String", "startsWith") | ("String", "endsWith") => Some(ast::Type::simple("bool")),
                                    ("String", "replace") => Some(ast::Type::simple("string")),
                                    ("String", "split") => Some(ast::Type::generic("ArrayList", vec![ast::Type::simple("string")])),
                                    ("String", "padLeft") | ("String", "padRight") => Some(ast::Type::simple("string")),
                                    ("String", "fromCharCode") => Some(ast::Type::simple("string")),
                                    ("String", "join") => Some(ast::Type::simple("string")),
                                    _ => None,
                                };
                                if let Some(ret) = result {
                                    return ret;
                                }
                            }
                        }

                        // Handle generic container methods (ArrayList.get, HashMap.get, etc.)
                        {
                            let type_name = obj_type.name();
                            let type_name_str: &str = &type_name;
                            match (type_name_str, method.as_str()) {
                                ("ArrayList", "size") | ("ArrayList", "length") => return ast::Type::simple("int"),
                                ("ArrayList", "isEmpty") | ("ArrayList", "contains") => return ast::Type::simple("bool"),
                                ("ArrayList", "indexOf") => return ast::Type::simple("int"),
                                ("HashMap", "size") | ("HashMap", "length") => return ast::Type::simple("int"),
                                ("HashMap", "isEmpty") | ("HashMap", "containsKey") | ("HashMap", "containsValue") => return ast::Type::simple("bool"),
                                _ => {}
                            }
                        if let Some(first_param) = obj_type.params().first() {
                            match (type_name_str, method.as_str()) {
                                ("ArrayList", "get") => return first_param.clone(),
                                ("ArrayList", "first") | ("ArrayList", "last") => return first_param.clone(),
                                ("HashMap", "get") => {
                                    // HashMap.get returns the value type (second param)
                                    if let Some(second_param) = obj_type.params().get(1) {
                                        return second_param.clone();
                                    }
                                    return first_param.clone();
                                }
                                ("HashMap", "keys") => return ast::Type::generic("ArrayList", vec![first_param.clone()]),
                                ("HashMap", "values") => {
                                    // HashMap.values returns ArrayList of value type
                                    if let Some(second_param) = obj_type.params().get(1) {
                                        return ast::Type::generic("ArrayList", vec![second_param.clone()]);
                                    }
                                    return ast::Type::generic("ArrayList", vec![first_param.clone()]);
                                }
                                _ => {}
                            }
                        }
                        }
                        if let Some(Symbol::Class(class_decl)) = scope.borrow().lookup(&obj_type.name()) {
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
            ast::Expr::Is(_, _, _) => ast::Type::simple("bool"),
            ast::Expr::StaticCall { class_name, method, .. } => {
                // Resolve return type for known static methods.
                match (class_name.as_str(), method.as_str()) {
                    // Integer methods
                    ("Integer", "parseInt") | ("Integer", "parseOr") => ast::Type::simple("int"),
                    ("Integer", "toString") => ast::Type::simple("string"),
                    // Double methods
                    ("Double", "parse") | ("Double", "parseDouble") => ast::Type::simple("double"),
                    ("Double", "toString") => ast::Type::simple("string"),
                    // Long methods
                    ("Long", "parseLong") => ast::Type::simple("long"),
                    ("Long", "toString") => ast::Type::simple("string"),
                    // Float methods
                    ("Float", "toF32Bits") | ("Float", "fromF32Bits") => ast::Type::simple("float"),
                    // String methods
                    ("String", "length") => ast::Type::simple("int"),
                    ("String", "charAt") => ast::Type::simple("string"),
                    ("String", "substring") => ast::Type::simple("string"),
                    ("String", "indexOf") => ast::Type::simple("int"),
                    ("String", "toUpperCase") | ("String", "toLowerCase") => ast::Type::simple("string"),
                    ("String", "trim") | ("String", "trimStart") | ("String", "trimEnd") => ast::Type::simple("string"),
                    ("String", "startsWith") | ("String", "endsWith") => ast::Type::simple("bool"),
                    ("String", "replace") => ast::Type::simple("string"),
                    ("String", "split") => ast::Type::generic("ArrayList", vec![ast::Type::simple("string")]),
                    ("String", "padLeft") | ("String", "padRight") => ast::Type::simple("string"),
                    ("String", "fromCharCode") => ast::Type::simple("string"),
                    ("String", "join") => ast::Type::simple("string"),
                    // Math methods
                    ("Math", "sin") | ("Math", "cos") | ("Math", "tan") => ast::Type::simple("double"),
                    ("Math", "asin") | ("Math", "acos") | ("Math", "atan") | ("Math", "atan2") => ast::Type::simple("double"),
                    ("Math", "ln") | ("Math", "log10") | ("Math", "log2") => ast::Type::simple("double"),
                    ("Math", "exp") | ("Math", "pow") | ("Math", "sqrt") | ("Math", "cbrt") => ast::Type::simple("double"),
                    ("Math", "abs") | ("Math", "absInt") => ast::Type::simple("double"),
                    ("Math", "floor") | ("Math", "ceil") | ("Math", "round") => ast::Type::simple("double"),
                    ("Math", "random") => ast::Type::simple("double"),
                    ("Math", "inf") | ("Math", "nan") | ("Math", "negInf") => ast::Type::simple("double"),
                    ("Math", "maxDouble") | ("Math", "minDouble") => ast::Type::simple("double"),
                    ("Math", "maxInt") | ("Math", "minInt") => ast::Type::simple("int"),
                    ("Math", "nextUp") | ("Math", "nextDown") | ("Math", "ulp") | ("Math", "scalb") | ("Math", "fma") => ast::Type::simple("double"),
                    ("Math", "getExponent") => ast::Type::simple("int"),
                    // MathAdvanced
                    ("MathAdvanced", "sqrt") | ("MathAdvanced", "pow") => ast::Type::simple("double"),
                    ("MathAdvanced", "exp") | ("MathAdvanced", "ln") => ast::Type::simple("double"),
                    ("MathAdvanced", "log2") | ("MathAdvanced", "log10") => ast::Type::simple("double"),
                    ("MathAdvanced", "cbrt") | ("MathAdvanced", "hypot") => ast::Type::simple("double"),
                    // MathTrig
                    ("MathTrig", "sin") | ("MathTrig", "cos") | ("MathTrig", "tan") => ast::Type::simple("double"),
                    ("MathTrig", "asin") | ("MathTrig", "acos") | ("MathTrig", "atan") | ("MathTrig", "atan2") => ast::Type::simple("double"),
                    ("MathTrig", "sinh") | ("MathTrig", "cosh") | ("MathTrig", "tanh") => ast::Type::simple("double"),
                    // Hash methods
                    ("Hash", "md5") | ("Hash", "sha1") | ("Hash", "sha256") | ("Hash", "sha384") | ("Hash", "sha512") => ast::Type::simple("string"),
                    ("Hash", "sha3_256") | ("Hash", "sha3_384") | ("Hash", "sha3_512") => ast::Type::simple("string"),
                    ("Hash", "blake2b") | ("Hash", "blake2s") | ("Hash", "crc32") => ast::Type::simple("string"),
                    // Encoding methods
                    ("Base64", "encode") | ("Base64", "decode") => ast::Type::simple("string"),
                    ("Hex", "encode") | ("Hex", "decode") => ast::Type::simple("string"),
                    ("Url", "encode") | ("Url", "decode") => ast::Type::simple("string"),
                    // File methods
                    ("File", "readFile") | ("File", "readLine") | ("File", "readChunk") => ast::Type::simple("string"),
                    ("File", "readLines") => ast::Type::generic("ArrayList", vec![ast::Type::simple("string")]),
                    ("File", "readBytes") => ast::Type::generic("ArrayList", vec![ast::Type::simple("byte")]),
                    ("File", "size") | ("File", "tell") => ast::Type::simple("long"),
                    ("File", "exists") => ast::Type::simple("bool"),
                    ("File", "lastModified") => ast::Type::simple("long"),
                    ("File", "tryLock") => ast::Type::simple("bool"),
                    // Path methods
                    ("Path", "join") | ("Path", "basename") | ("Path", "dirname") | ("Path", "extension") => ast::Type::simple("string"),
                    ("Path", "exists") | ("Path", "isFile") | ("Path", "isDir") | ("Path", "isSymlink") => ast::Type::simple("bool"),
                    // Dir methods
                    ("Dir", "list") => ast::Type::generic("ArrayList", vec![ast::Type::simple("string")]),
                    ("Dir", "create") | ("Dir", "remove") | ("Dir", "removeTree") | ("Dir", "copy") | ("Dir", "move") => ast::Type::simple("void"),
                    // Time methods
                    ("Time", "now") | ("Time", "millis") | ("Time", "monotonic") | ("Time", "perfCounter") | ("Time", "epochSeconds") | ("Time", "nanos") => ast::Type::simple("long"),
                    ("Time", "format") | ("Time", "dayOfWeek") => ast::Type::simple("string"),
                    ("Time", "getYear") | ("Time", "getMonth") | ("Time", "getDay") | ("Time", "getHour") | ("Time", "getMinute") | ("Time", "getSecond") | ("Time", "dayOfYear") => ast::Type::simple("int"),
                    ("Time", "sleep") => ast::Type::simple("void"),
                    // Regex methods
                    ("Regex", "match") | ("Regex", "fullMatch") => ast::Type::simple("bool"),
                    ("Regex", "find") | ("Regex", "replace") | ("Regex", "subN") | ("Regex", "findWithFlags") | ("Regex", "matchWithFlags") => ast::Type::simple("string"),
                    ("Regex", "groupCount") => ast::Type::simple("int"),
                    // Json methods
                    ("Json", "parse") => ast::Type::simple("Variant"),
                    ("Json", "stringify") => ast::Type::simple("string"),
                    // Env / Os / Sys methods
                    ("Env", "get") | ("Env", "vars") | ("Os", "environ") | ("Os", "uname") | ("Os", "strerror") | ("Os", "userName") | ("Os", "hostName") | ("Os", "release") | ("Os", "version") => ast::Type::simple("string"),
                    ("Env", "set") | ("Env", "unset") | ("Os", "setenv") | ("Os", "unsetenv") | ("Os", "makedirs") | ("Os", "chmod") | ("Os", "utime") | ("Os", "link") | ("Os", "system") => ast::Type::simple("void"),
                    ("Os", "exists") | ("Os", "isFile") | ("Os", "isDir") | ("Os", "access") => ast::Type::simple("bool"),
                    ("Os", "size") | ("Os", "cpuCount") | ("Os", "getpid") | ("Os", "getppid") | ("Os", "umask") => ast::Type::simple("long"),
                    ("Os", "scandir") => ast::Type::generic("ArrayList", vec![ast::Type::simple("Variant")]),
                    ("Os", "getcwd") | ("Os", "chdir") => ast::Type::simple("string"),
                    ("Sys", "args") => ast::Type::generic("ArrayList", vec![ast::Type::simple("string")]),
                    ("Sys", "env") | ("Sys", "workingDir") => ast::Type::simple("string"),
                    ("Sys", "setEnv") | ("Sys", "setWorkingDir") | ("Sys", "exit") | ("Sys", "sleep") | ("Sys", "changeDir") => ast::Type::simple("void"),
                    // Process methods
                    ("Process", "id") | ("Process", "args") => ast::Type::simple("long"),
                    ("Process", "spawn") | ("Process", "join") | ("Process", "terminate") => ast::Type::simple("void"),
                    // Thread methods
                    ("Thread", "spawn") | ("Thread", "spawnRunnable") | ("Thread", "join") | ("Thread", "sleep") | ("Thread", "yield") | ("Thread", "detach") => ast::Type::simple("void"),
                    ("Thread", "getId") | ("Thread", "currentId") => ast::Type::simple("long"),
                    // Random methods
                    ("Random", "seed") => ast::Type::simple("void"),
                    ("Random", "nextLong") => ast::Type::simple("long"),
                    // Sqlite methods
                    ("Sqlite", "open") => ast::Type::simple("long"),
                    ("Sqlite", "execute") | ("Sqlite", "close") | ("Sqlite", "closeResult") => ast::Type::simple("void"),
                    ("Sqlite", "query") | ("Sqlite", "nextRow") => ast::Type::simple("bool"),
                    ("Sqlite", "lastInsertId") | ("Sqlite", "getInt") | ("Sqlite", "columnCount") => ast::Type::simple("int"),
                    ("Sqlite", "getString") => ast::Type::simple("string"),
                    ("Sqlite", "getDouble") => ast::Type::simple("double"),
                    ("Sqlite", "columnName") => ast::Type::simple("string"),
                    // Subprocess methods
                    ("Subprocess", "run") | ("Subprocess", "exec") | ("Subprocess", "popenWrite") => ast::Type::simple("string"),
                    // Gzip / Zlib methods
                    ("Gzip", "compress") | ("Gzip", "decompress") | ("Zlib", "compress") | ("Zlib", "decompress") => ast::Type::simple("string"),
                    // Zip methods
                    ("ZipFile", "open") => ast::Type::simple("long"),
                    ("ZipFile", "entryCount") => ast::Type::simple("int"),
                    ("ZipFile", "entryName") => ast::Type::simple("string"),
                    ("ZipFile", "readEntry") | ("ZipFile", "extractAll") => ast::Type::simple("string"),
                    ("ZipFile", "close") | ("ZipWriter", "close") => ast::Type::simple("void"),
                    ("ZipWriter", "open") => ast::Type::simple("long"),
                    ("ZipWriter", "addEntry") => ast::Type::simple("void"),
                    // Socket methods
                    ("Socket", "new") | ("Socket", "createConnection") | ("Socket", "createServer") => ast::Type::simple("long"),
                    ("Socket", "connect") | ("Socket", "bind") | ("Socket", "listen") | ("Socket", "accept") => ast::Type::simple("void"),
                    ("Socket", "send") | ("Socket", "recv") => ast::Type::simple("string"),
                    ("Socket", "close") | ("Socket", "setTimeout") | ("Socket", "setNoDelay") | ("Socket", "setReuseAddr") | ("Socket", "setBroadcast") | ("Socket", "setKeepAlive") | ("Socket", "setLinger") => ast::Type::simple("void"),
                    ("Socket", "getLocalPort") | ("Socket", "getRemotePort") => ast::Type::simple("int"),
                    ("Socket", "getLocalAddress") | ("Socket", "getRemoteAddress") | ("Socket", "inetPton") | ("Socket", "inetNtop") => ast::Type::simple("string"),
                    // Net methods
                    ("Net", "connect") | ("Net", "bind") | ("Net", "accept") => ast::Type::simple("long"),
                    ("Net", "send") | ("Net", "receive") | ("Net", "getLocalPort") | ("Net", "getRemotePort") => ast::Type::simple("int"),
                    ("Net", "getLocalAddress") | ("Net", "getRemoteAddress") => ast::Type::simple("string"),
                    ("Net", "close") | ("Net", "setTimeout") => ast::Type::simple("void"),
                    // Http methods
                    ("Http", "get") | ("Http", "post") | ("Http", "put") | ("Http", "delete") | ("Http", "patch") | ("Http", "head") => ast::Type::simple("string"),
                    ("Http", "setTimeout") | ("Http", "setFollowRedirects") => ast::Type::simple("void"),
                    // Dns methods
                    ("Dns", "lookup") | ("Dns", "reverseLookup") => ast::Type::simple("string"),
                    // TypeName
                    ("TypeName", "of") => ast::Type::simple("string"),
                    // Gc
                    ("Gc", "collect") => ast::Type::simple("void"),
                    // System
                    ("System", "currentTimeMillis") => ast::Type::simple("long"),
                    // Signal
                    ("Signal", "register") | ("Signal", "raise") => ast::Type::simple("int"),
                    // Titrate
                    ("Titrate", "version") => ast::Type::simple("string"),
                    // Default: string for unknown static calls
                    _ => ast::Type::simple("string"),
                }
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
