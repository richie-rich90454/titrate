use super::*;
use super::types::{
    is_numeric_type, is_bool_type, is_string_type, is_integer_type,
    is_owned_type, is_result_type, is_unknown_type, is_assignable,
    class_has_operator_method, static_class_for_primitive,
    INTEGER_TYPES, FLOAT_TYPES,
};
use super::errors::find_similar_names;

impl Analyzer {

    pub(super) fn analyze_expr(&mut self, expr: &mut ast::Expr, scope: &Rc<RefCell<Scope>>) {
        match expr {
            ast::Expr::Literal(_, _) => {}
            ast::Expr::Unit(_) => {}
            ast::Expr::Identifier(name, _) => {
                // Symbol resolution.
                let sym = scope.borrow().lookup(name);
                match sym {
                    None => {
                        let mut err = CompileError::new(format!(
                            "undeclared identifier: '{}'",
                            name
                        ));
                        // Suggest similar names from scope.
                        let similar = find_similar_names(name, scope, 2);
                        if let Some(best) = similar.first() {
                            err = err.suggest(Suggestion {
                                message: "a similar name exists in scope".to_string(),
                                replacement: Some(best.clone()),
                            });
                        }
                        self.error(err);
                    }
                    Some(Symbol::Variable { .. }) => {
                        // Track variable usage for unused variable detection.
                        self.used_vars.insert(name.clone());
                        // Ownership check: is the variable still live?
                        if !self.in_unsafe {
                            if let Some(state) = self.var_states.get(name) {
                                match state {
                                    VarState::Moved => {
                                        self.error(CompileError::new(format!(
                                            "use of moved variable: '{}'",
                                            name
                                        )).suggest(Suggestion {
                                            message: "this value was moved earlier; consider cloning or using a reference".to_string(),
                                            replacement: None,
                                        }));
                                    }
                                    VarState::BorrowedMutable => {
                                        // Reading while mutably borrowed is an error.
                                        self.error(CompileError::new(format!(
                                            "cannot read variable '{}' while it is mutably borrowed",
                                            name
                                        )).suggest(Suggestion {
                                            message: "wait for the mutable borrow to go out of scope before reading".to_string(),
                                            replacement: None,
                                        }));
                                    }
                                    VarState::Live | VarState::BorrowedImmutable => {}
                                }
                            }
                        }
                    }
                    Some(Symbol::Variant { .. }) => {
                        // Variant name used as constructor 閳?fine.
                    }
                    Some(Symbol::Function(_)) => {
                        // Function name used as value 閳?fine.
                    }
                    Some(Symbol::Class(_)) => {
                        // Class name used as type/constructor 閳?fine.
                    }
                    Some(Symbol::Interface(_)) => {
                        // Interface name 閳?fine.
                    }
                    Some(Symbol::Enum(_)) => {
                        // Enum name 閳?fine.
                    }
                }
            }
            ast::Expr::Binary(left, op, right, _) => {
                self.analyze_expr(left, scope);
                self.analyze_expr(right, scope);

                let left_type = self.infer_expr_type(left, scope);
                let right_type = self.infer_expr_type(right, scope);

                // Check if the left type is a class with an operator overload method
                let has_operator_overload = class_has_operator_method(&left_type, op, scope);

                // Skip type checking when types are unknown (from builtins, field access, etc.)
                // or when the left operand has an operator overload method
                if has_operator_overload {
                    // Operator overload method exists 閳?fine
                } else if is_unknown_type(&left_type) || is_unknown_type(&right_type) {
                    // Cannot verify 閳?skip
                } else {
                match op {
                    ast::Operator::Add => {
                        // Numeric addition or string concatenation.
                        if is_numeric_type(&left_type) && is_numeric_type(&right_type) {
                            // Fine.
                        } else if is_string_type(&left_type) || is_string_type(&right_type) {
                            // String concatenation 閳?fine.
                        } else {
                            self.error(CompileError::new(format!(
                                "operator + cannot be applied to {} and {}",
                                left_type, right_type
                            )).suggest(Suggestion {
                                message: "+ requires numeric or string operands".to_string(),
                                replacement: None,
                            }));
                        }
                    }
                    ast::Operator::Sub
                    | ast::Operator::Mul
                    | ast::Operator::Div
                    | ast::Operator::Mod => {
                        if !is_numeric_type(&left_type) || !is_numeric_type(&right_type) {
                            self.error(CompileError::new(format!(
                                "arithmetic operator requires numeric operands, found {} and {}",
                                left_type, right_type
                            )).suggest(Suggestion {
                                message: "use numeric types (int, long, float, double, etc.)".to_string(),
                                replacement: None,
                            }));
                        }
                    }
                    ast::Operator::Eq
                    | ast::Operator::Ne
                    | ast::Operator::Lt
                    | ast::Operator::Gt
                    | ast::Operator::Le
                    | ast::Operator::Ge => {
                        // Comparison operators 閳?fine for most types.
                    }
                    ast::Operator::And | ast::Operator::Or => {
                        if !is_bool_type(&left_type) || !is_bool_type(&right_type) {
                            self.error(CompileError::new(format!(
                                "logical operator requires bool operands, found {} and {}",
                                left_type, right_type
                            )).suggest(Suggestion {
                                message: "use comparison operators to produce bool values".to_string(),
                                replacement: None,
                            }));
                        }
                    }
                    ast::Operator::BitAnd
                    | ast::Operator::BitOr
                    | ast::Operator::BitXor
                    | ast::Operator::BitShl
                    | ast::Operator::BitShr
                    | ast::Operator::BitUshr => {
                        if !is_integer_type(&left_type) || !is_integer_type(&right_type) {
                            self.error(CompileError::new(format!(
                                "bitwise operator requires integer operands, found {} and {}",
                                left_type, right_type
                            )).suggest(Suggestion {
                                message: "use integer types (byte, short, int, long, vast, etc.)".to_string(),
                                replacement: None,
                            }));
                        }
                    }
                }
                }
            }
            ast::Expr::Unary(unop, operand, _) => {
                self.analyze_expr(operand, scope);
                let operand_type = self.infer_expr_type(operand, scope);
                match unop {
                    ast::UnOp::Neg => {
                        if !is_numeric_type(&operand_type) {
                            self.error(CompileError::new(format!(
                                "unary - requires numeric operand, found {}",
                                operand_type
                            )).suggest(Suggestion {
                                message: "use a numeric type (int, long, float, double, etc.)".to_string(),
                                replacement: None,
                            }));
                        }
                    }
                    ast::UnOp::Not => {
                        if !is_bool_type(&operand_type) {
                            self.error(CompileError::new(format!(
                                "unary ! requires bool operand, found {}",
                                operand_type
                            )).suggest(Suggestion {
                                message: "use a comparison expression to produce a bool".to_string(),
                                replacement: None,
                            }));
                        }
                    }
                    ast::UnOp::BitNot => {
                        if !is_integer_type(&operand_type) {
                            self.error(CompileError::new(format!(
                                "unary ~ requires integer operand, found {}",
                                operand_type
                            )).suggest(Suggestion {
                                message: "use an integer type (byte, short, int, long, vast, etc.)".to_string(),
                                replacement: None,
                            }));
                        }
                    }
                }
            }
            ast::Expr::Call(callee, args, _) => {
                // Check for toString desugaring BEFORE analyzing (which borrows immutably).
                // We need to detect: Call(MemberAccess(obj, "toString"), [])
                let desugar_info: Option<(String, ast::Expr)> = match callee.as_ref() {
                    ast::Expr::MemberAccess(obj, method, _) if method == "toString" => {
                        let obj_type = self.infer_expr_type(obj, scope);
                        static_class_for_primitive(&obj_type).map(|class_name| {
                            // We'll replace the whole expression after this match.
                            (class_name, *obj.clone())
                        })
                    }
                    _ => None,
                };

                if let Some((class_name, obj_expr)) = desugar_info {
                    *expr = ast::Expr::StaticCall {
                        class_name,
                        method: "toString".to_string(),
                        args: vec![obj_expr],
                        span: ast::Span::unknown(),
                    };
                    // Re-analyze the desugared form.
                    self.analyze_expr(expr, scope);
                    return;
                }

                // Convert Module.method(args) to StaticCall for known builtin wrappers.
                // e.g., Sys.args() -> StaticCall { class_name: "Sys", method: "args", ... }
                // Only convert if the method is a known static method on the module.
                if let ast::Expr::MemberAccess(obj, method, _) = callee.as_ref() {
                    if let ast::Expr::Identifier(ns, _) = obj.as_ref() {
                        let known_static_methods: &[(&str, &[&str])] = &[
                            ("Sys", &["args", "env", "workingDir", "setEnv", "exit", "sleep", "changeDir"]),
                            ("File", &["readFile", "writeFile", "readBytes", "writeBytes", "append", "readLines", "readLine", "readChunk", "size", "exists", "lastModified", "setModified", "flush", "truncate", "copy", "delete", "tryLock", "unlock", "seek", "tell", "rename", "write"]),
                            ("Dir", &["list", "create", "remove", "removeTree", "walk", "copy", "move"]),
                            ("Path", &["join", "exists", "isFile", "isDir", "basename", "dirname", "extension", "isSymlink"]),
                            ("Fs", &["exists", "isFile", "isDir", "size", "totalSpace", "freeSpace", "closeWatch", "pollWatchEvents"]),
                            ("Os", &["name", "arch", "family", "cpuCount", "userName", "hostName", "urandom", "chmod", "makedirs", "symlink", "readlink", "kill", "environ", "umask", "scandir", "environMap", "getpid", "getcwd", "chdir", "getenv", "setenv", "unsetenv", "system", "uname", "getppid", "strerror", "removedirs", "renames", "replace", "link", "utime", "lstat", "access", "release", "version"]),
                            ("Time", &["now", "sleep", "format", "getYear", "getMonth", "getDay", "getHour", "getMinute", "getSecond", "dayOfWeek", "dayOfYear", "monotonic", "perfCounter", "epochSeconds", "nanos", "millis"]),
                            ("Regex", &["match", "find", "replace", "groupCount", "findGroups", "findWithFlags", "matchWithFlags", "fullMatch", "subN", "findNamedCapture", "findAllNamedCaptures"]),
                            ("Json", &["parse", "stringify"]),
                            ("Hash", &["md5", "sha1", "sha256", "sha384", "sha512", "sha3_256", "sha3_384", "sha3_512", "blake2b", "blake2s", "crc32", "sha224", "sha3_224", "shake128", "shake256"]),
                            ("Base64", &["encode", "decode"]),
                            ("Hex", &["encode", "decode"]),
                            ("Url", &["encode", "decode"]),
                            ("Random", &["seed", "nextLong"]),
                            ("Env", &["get", "set", "vars", "unset"]),
                            ("Signal", &["register", "raise"]),
                            ("Process", &["id", "args", "spawn", "join", "terminate"]),
                            ("TypeName", &["of"]),
                            ("Gc", &["collect"]),
                            ("String", &["length", "charAt", "substring", "indexOf", "toUpperCase", "toLowerCase", "trim", "trimStart", "trimEnd", "startsWith", "endsWith", "replace", "split", "padLeft", "padRight", "fromCharCode", "join"]),
                            ("Math", &["sin", "cos", "tan", "asin", "acos", "atan", "atan2", "ln", "log10", "log2", "exp", "pow", "sqrt", "cbrt", "abs", "absInt", "floor", "ceil", "round", "random", "inf", "nan", "negInf", "maxDouble", "minDouble", "maxInt", "minInt", "nextUp", "nextDown", "ulp", "scalb", "fma", "getExponent"]),
                            ("MathAdvanced", &["sqrt", "pow", "exp", "ln", "log2", "log10", "cbrt", "hypot"]),
                            ("MathTrig", &["sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh"]),
                            ("Integer", &["parseInt", "parseOr", "toString"]),
                            ("Double", &["parse", "parseDouble", "toString"]),
                            ("Long", &["parseLong", "toString"]),
                            ("Float", &["toF32Bits", "fromF32Bits"]),
                            ("Subprocess", &["run", "exec", "popenWrite"]),
                            ("ZipFile", &["open", "entryCount", "entryName", "readEntry", "extractAll", "close"]),
                            ("ZipWriter", &["open", "addEntry", "close"]),
                            ("Sqlite", &["open", "execute", "query", "close", "lastInsertId", "nextRow", "getInt", "getString", "getDouble", "columnCount", "columnName", "closeResult", "executePrepared", "backup"]),
                            ("Socket", &["new", "connect", "bind", "listen", "accept", "send", "recv", "close", "setTimeout", "setNoDelay", "setReuseAddr", "setBroadcast", "setKeepAlive", "setLinger", "getLocalPort", "getRemotePort", "getLocalAddress", "getRemoteAddress", "inetPton", "inetNtop", "createConnection", "createServer", "getAddrInfo"]),
                            ("Thread", &["spawn", "spawnRunnable", "join", "sleep", "yield", "getId", "currentId", "detach"]),
                        ];
                        let mut should_desugar = false;
                        for (mod_name, methods) in known_static_methods {
                            if *mod_name == ns.as_str() && methods.contains(&method.as_str()) {
                                should_desugar = true;
                                break;
                            }
                        }
                        if should_desugar {
                            let mut call_args = Vec::new();
                            for arg in args.iter_mut() {
                                self.analyze_expr(arg, scope);
                                call_args.push(arg.clone());
                            }
                            *expr = ast::Expr::StaticCall {
                                class_name: ns.clone(),
                                method: method.clone(),
                                args: call_args,
                                span: ast::Span::unknown(),
                            };
                            self.analyze_expr(expr, scope);
                            return;
                        }
                    }
                }

                self.analyze_expr(callee, scope);
                for arg in args.iter_mut() {
                    self.analyze_expr(arg, scope);
                }

                // Check if callee is a function and validate argument count.
                match callee.as_ref() {
                    ast::Expr::Identifier(name, _) => {
                        if let Some(Symbol::Function(f)) = scope.borrow().lookup(name) {
                            // Skip param count check for native functions (empty body)
                            if !f.body.is_empty() && args.len() != f.params.len() {
                                self.error(format!(
                                    "function {} expects {} arguments, found {}",
                                    name,
                                    f.params.len(),
                                    args.len()
                                ));
                            }
                        }
                    }
                    ast::Expr::MemberAccess(obj, method, _) => {
                        // Check method calls on known types.
                        let obj_type = self.infer_expr_type(obj, scope);
                        if let Some(Symbol::Class(class_decl)) = scope.borrow().lookup(obj_type.name()) {
                            for member in &class_decl.members {
                                if let ast::ClassMember::Method(m) = member {
                                    if m.name == *method {
                                        if args.len() != m.params.len() {
                                            self.error(format!(
                                                "method {} expects {} arguments, found {}",
                                                method,
                                                m.params.len(),
                                                args.len()
                                            ));
                                        }
                                        break;
                                    }
                                }
                                if let ast::ClassMember::Constructor(m) = member {
                                    if m.name == *method {
                                        if args.len() != m.params.len() {
                                            self.error(format!(
                                                "constructor {} expects {} arguments, found {}",
                                                method,
                                                m.params.len(),
                                                args.len()
                                            ));
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            ast::Expr::MemberAccess(obj, field, _) => {
                self.analyze_expr(obj.as_mut(), scope);
                // If it's a method call, it will be handled in the Call branch above.
                // For field access, we just validate the object is not moved.
                let _ = field;
            }
            ast::Expr::Index(obj, idx, _) => {
                self.analyze_expr(obj.as_mut(), scope);
                self.analyze_expr(idx.as_mut(), scope);
            }
            ast::Expr::New(typ, args, _) => {
                for arg in args.iter_mut() {
                    self.analyze_expr(arg, scope);
                }
                // Verify the type exists.
                let type_name = typ.name();
                let sym = scope.borrow().lookup(type_name);
                match sym {
                    None => {
                        // Could be a primitive type like int, bool, etc.
                        if !INTEGER_TYPES.contains(&type_name)
                            && !FLOAT_TYPES.contains(&type_name)
                            && type_name != "bool"
                            && type_name != "char"
                            && type_name != "string"
                        {
                            self.error(format!("undeclared type: {}", type_name));
                        }
                    }
                    Some(Symbol::Class(_)) => {}
                    Some(Symbol::Variable { .. }) => {
                        // Native wrapper types like Regex, Thread, etc. are
                        // registered as Variable symbols but can be used with `new`.
                        let native_new_types = [
                            "Regex", "Thread", "Channel", "TcpServer",
                            "Socket", "UdpSocket", "Ssl", "Sqlite",
                            "Mmap", "ZipFile", "ZipWriter", "Mutex",
                            "RecursiveMutex", "SharedMutex", "CondVar",
                            "Semaphore", "OnceFlag", "AtomicInt", "AtomicBool",
                            "AtomicLong", "AtomicRef", "Subprocess",
                        ];
                        if !native_new_types.contains(&type_name) {
                            self.error(format!("{} is a variable, not a type", type_name));
                        }
                    }
                    Some(Symbol::Enum(_)) => {}
                    Some(Symbol::Interface(_)) => {}
                    Some(Symbol::Function(_)) => {
                        self.error(format!("{} is a function, not a type", type_name));
                    }
                    Some(Symbol::Variant { .. }) => {
                        self.error(format!("{} is a variant, not a type", type_name));
                    }
                }
            }
            ast::Expr::This(_) => {}
            ast::Expr::Super(_) => {}
            ast::Expr::OwnedDeref(inner, _) => {
                self.analyze_expr(inner, scope);
                let inner_type = self.infer_expr_type(inner, scope);
                if !is_owned_type(&inner_type) && !is_unknown_type(&inner_type) {
                    self.error(CompileError::new(format!(
                        "owned dereference requires Owned type, found {}",
                        inner_type
                    )).suggest(Suggestion {
                        message: "wrap the value in Owned<T> before dereferencing".to_string(),
                        replacement: None,
                    }));
                }
            }
            ast::Expr::RegionAlloc(_typ, region_expr, _) => {
                self.analyze_expr(region_expr, scope);
                // Track that this allocation belongs to the current region.
                // We'll check that region-allocated values don't escape.
            }
            ast::Expr::RefExpr(inner, ref_kind, _) => {
                self.analyze_expr(inner, scope);

                if self.in_unsafe {
                    return;
                }

                // Borrow checking.
                match inner.as_ref() {
                    ast::Expr::Identifier(name, _) => {
                        match ref_kind {
                            ast::RefKind::Immutable => {
                                if let Some(state) = self.var_states.get(name) {
                                    match state {
                                        VarState::Moved => {
                                            self.error(CompileError::new(format!(
                                                "cannot borrow moved variable: '{}'",
                                                name
                                            )).suggest(Suggestion {
                                                message: "the value was moved earlier; consider cloning or reassigning".to_string(),
                                                replacement: None,
                                            }));
                                        }
                                        VarState::BorrowedMutable => {
                                            self.error(CompileError::new(format!(
                                                "cannot immutably borrow '{}' while it is mutably borrowed",
                                                name
                                            )).suggest(Suggestion {
                                                message: "wait for the mutable borrow to go out of scope".to_string(),
                                                replacement: None,
                                            }));
                                        }
                                        VarState::Live | VarState::BorrowedImmutable => {
                                            self.var_states.insert(name.clone(), VarState::BorrowedImmutable);
                                        }
                                    }
                                }
                            }
                            ast::RefKind::Mutable => {
                                if let Some(state) = self.var_states.get(name) {
                                    match state {
                                        VarState::Moved => {
                                            self.error(CompileError::new(format!(
                                                "cannot mutably borrow moved variable: '{}'",
                                                name
                                            )).suggest(Suggestion {
                                                message: "the value was moved earlier; consider cloning or reassigning".to_string(),
                                                replacement: None,
                                            }));
                                        }
                                        VarState::BorrowedImmutable => {
                                            self.error(CompileError::new(format!(
                                                "cannot mutably borrow '{}' while it is immutably borrowed",
                                                name
                                            )).suggest(Suggestion {
                                                message: "wait for the immutable borrow(s) to go out of scope".to_string(),
                                                replacement: None,
                                            }));
                                        }
                                        VarState::BorrowedMutable => {
                                            self.error(CompileError::new(format!(
                                                "cannot mutably borrow '{}' more than once",
                                                name
                                            )).suggest(Suggestion {
                                                message: "consider using a shared reference (&T) instead, or restructuring the code".to_string(),
                                                replacement: None,
                                            }));
                                        }
                                        VarState::Live => {
                                            self.var_states.insert(name.clone(), VarState::BorrowedMutable);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        // Borrowing non-identifier expressions 閳?fine for now.
                    }
                }
            }
            ast::Expr::UnsafeBlock(block, _) => {
                let prev_unsafe = self.in_unsafe;
                self.in_unsafe = true;
                self.analyze_block(block, scope);
                self.in_unsafe = prev_unsafe;
            }
            ast::Expr::ErrorPropagation(inner, _) => {
                self.analyze_expr(inner, scope);
                let inner_type = self.infer_expr_type(inner, scope);
                // The operand must be a Result type.
                if !is_result_type(&inner_type) {
                    // For Alpha, we relax this 閳?the interpreter handles it.
                }
                // The function return type must be a Result.
                if let Some(ref ret) = self.current_return_type {
                    if !is_result_type(ret) {
                        let fn_name = self.current_fn_name.clone().unwrap_or_default();
                        self.error(CompileError::new(format!(
                            "? operator can only be used in functions returning Result, found return type {} in function '{}'",
                            ret, fn_name
                        )).suggest(Suggestion {
                            message: "change the function return type to Result<T, E>".to_string(),
                            replacement: None,
                        }));
                    }
                } else {
                    let fn_name = self.current_fn_name.clone().unwrap_or_default();
                    self.error(CompileError::new(format!(
                        "? operator used in function '{}' with no return type",
                        fn_name
                    )).suggest(Suggestion {
                        message: "add a Result return type to the function".to_string(),
                        replacement: None,
                    }));
                }
            }
            ast::Expr::Cast(inner, target_type, _) => {
                self.analyze_expr(inner, scope);
                let inner_type = self.infer_expr_type(inner, scope);
                // Casts between numeric types are fine.
                if is_numeric_type(&inner_type) && is_numeric_type(target_type) {
                    // Fine.
                } else if is_numeric_type(&inner_type) && is_bool_type(target_type) {
                    // Fine (non-zero to true).
                } else if is_bool_type(&inner_type) && is_numeric_type(target_type) {
                    // Fine.
                } else {
                    self.error(CompileError::new(format!(
                        "cannot cast from {} to {}",
                        inner_type, target_type
                    )).suggest(Suggestion {
                        message: "casts are supported between numeric types, or between numeric and bool".to_string(),
                        replacement: None,
                    }));
                }
            }
            ast::Expr::Is(inner, _, _) => {
                self.analyze_expr(inner, scope);
            }
            ast::Expr::StaticCall { class_name, method, args, span: _ } => {
                for arg in args.iter_mut() {
                    self.analyze_expr(arg, scope);
                }
                // Verify the class exists.
                let sym = scope.borrow().lookup(class_name);
                match sym {
                    None => {
                        // Could be a built-in like Integer, Boolean, etc.
                        // We don't error on these for the Alpha.
                    }
                    Some(Symbol::Class(_)) => {}
                    Some(Symbol::Variable { .. }) => {
                        // Built-in type wrappers like Integer, Double, etc.
                        // are registered as Variable symbols. Allow StaticCall on them.
                        let builtin_wrappers = [
                            "Integer", "Double", "Float", "Long", "Byte", "Short",
                            "Half", "Quad", "Vast", "Uvast", "Boolean", "Char",
                            "String_", "io", "Result", "Ok", "Err",
                            // Native module namespaces — allow static calls
                            // (e.g. Math::sin(x)) that map to C-ABI wrappers.
                            "Math", "MathAdvanced", "MathTrig", "String",
                            "Path", "File", "Dir", "Fs", "Os", "Time",
                            "Random", "Regex", "Json", "Base64", "Hex", "Url",
                            "Hash", "Hasher", "Hmac", "TypeName", "Gc",
                            "Socket", "UdpSocket", "Ssl", "Sqlite", "Mmap",
                            "ZipFile", "ZipWriter", "Thread", "Mutex",
                            "RecursiveMutex", "SharedMutex", "CondVar",
                            "Semaphore", "OnceFlag", "AtomicInt", "AtomicBool",
                            "AtomicLong", "AtomicRef", "Process", "Subprocess",
                            "Env", "Signal", "Sys",
                        ];
                        if !builtin_wrappers.contains(&class_name.as_str()) {
                            self.error(CompileError::new(format!(
                                "'{}' is not a class",
                                class_name
                            )).suggest(Suggestion {
                                message: "check the class name for typos".to_string(),
                                replacement: None,
                            }));
                        }
                    }
                    Some(_) => {
                        self.error(CompileError::new(format!(
                            "'{}' is not a class",
                            class_name
                        )).suggest(Suggestion {
                            message: "use a class name for static method calls".to_string(),
                            replacement: None,
                        }));
                    }
                }
                let _ = method;
            }
            ast::Expr::Assign(target, value, _) => {
                self.analyze_expr(value, scope);
                let value_type = self.infer_expr_type(value, scope);

                // Analyze the target expression as well.
                self.analyze_expr(target, scope);

                // Check the target is assignable.
                match target.as_ref() {
                    ast::Expr::Identifier(name, _) => {
                        let sym = scope.borrow().lookup(name);
                        match sym {
                            None => {
                                let mut err = CompileError::new(format!(
                                    "undeclared identifier in assignment: '{}'",
                                    name
                                ));
                                let similar = find_similar_names(name, scope, 2);
                                if let Some(best) = similar.first() {
                                    err = err.suggest(Suggestion {
                                        message: "a similar name exists in scope".to_string(),
                                        replacement: Some(best.clone()),
                                    });
                                }
                                self.error(err);
                            }
                            Some(Symbol::Variable { typ, mutable }) => {
                                if !mutable {
                                    self.error(CompileError::new(format!(
                                        "cannot assign to immutable variable: '{}'",
                                        name
                                    )).suggest(Suggestion {
                                        message: "declare the variable with 'var' or 'mut' to make it mutable".to_string(),
                                        replacement: None,
                                    }));
                                }
                                if !is_assignable(&value_type, &typ) {
                                    self.error(CompileError::new(format!(
                                        "type mismatch in assignment to '{}': cannot assign {} to {}",
                                        name, value_type, typ
                                    )).suggest(Suggestion {
                                        message: format!("expected type {}, found {}", typ, value_type),
                                        replacement: None,
                                    }));
                                }

                                // Ownership check: cannot assign to a borrowed variable.
                                if !self.in_unsafe {
                                    if let Some(state) = self.var_states.get(name) {
                                        match state {
                                            VarState::BorrowedImmutable => {
                                                self.error(CompileError::new(format!(
                                                    "cannot assign to '{}' while it is immutably borrowed",
                                                    name
                                                )).suggest(Suggestion {
                                                    message: "wait for the immutable borrow to go out of scope".to_string(),
                                                    replacement: None,
                                                }));
                                            }
                                            VarState::BorrowedMutable => {
                                                self.error(CompileError::new(format!(
                                                    "cannot assign to '{}' while it is mutably borrowed",
                                                    name
                                                )).suggest(Suggestion {
                                                    message: "wait for the mutable borrow to go out of scope".to_string(),
                                                    replacement: None,
                                                }));
                                            }
                                            VarState::Moved | VarState::Live => {}
                                        }
                                    }
                                }

                                // After assignment, the variable is live again.
                                self.var_states.insert(name.clone(), VarState::Live);
                            }
                            Some(_) => {
                                self.error(CompileError::new(format!(
                                    "cannot assign to non-variable: '{}'",
                                    name
                                )).suggest(Suggestion {
                                    message: "only variables can be assigned to".to_string(),
                                    replacement: None,
                                }));
                            }
                        }
                    }
                    ast::Expr::MemberAccess(_, _, _) | ast::Expr::Index(_, _, _) => {
                        // Already analyzed above.
                    }
                    _ => {
                        self.error(CompileError::new(
                            "invalid assignment target".to_string()
                        ).suggest(Suggestion {
                            message: "assignment target must be a variable, field access, or index".to_string(),
                            replacement: None,
                        }));
                    }
                }

                // Check if the value being assigned is a move of an Owned variable.
                if !self.in_unsafe {
                    if let ast::Expr::Identifier(src_name, _) = value.as_ref() {
                        let src_sym = scope.borrow().lookup(src_name);
                        if let Some(Symbol::Variable { typ, .. }) = src_sym {
                            if is_owned_type(&typ) {
                                self.var_states.insert(src_name.clone(), VarState::Moved);
                            }
                        }
                    }
                }
            }
            ast::Expr::Tuple(elements, _) => {
                for elem in elements.iter_mut() {
                    self.analyze_expr(elem, scope);
                }
            }
            ast::Expr::Range(start, end, _) => {
                self.analyze_expr(start, scope);
                self.analyze_expr(end, scope);
            }
            ast::Expr::RangeInclusive(start, end, _) => {
                self.analyze_expr(start, scope);
                self.analyze_expr(end, scope);
            }
            ast::Expr::Ternary { condition, then_expr, else_expr, .. } => {
                self.analyze_expr(condition, scope);
                self.analyze_expr(then_expr, scope);
                self.analyze_expr(else_expr, scope);
            }
            ast::Expr::Closure {
                params,
                return_type: _,
                body,
                expr: closure_expr,
                captured_vars,
                span: _,
            } => {
                // Create a new scope for the closure body.
                let closure_scope = Rc::new(RefCell::new(Scope::new(Some(scope.clone()))));

                // Define parameters in the closure scope.
                for (name, typ) in &mut *params {
                    closure_scope.borrow_mut().define(
                        name.clone(),
                        Symbol::Variable {
                            typ: typ.clone(),
                            mutable: false,
                        },
                    );
                }

                // Analyze the closure body in the new scope.
                if let Some(ref mut e) = closure_expr {
                    self.analyze_expr(e, &closure_scope);
                }
                for stmt in body.iter_mut() {
                    self.analyze_stmt(stmt, &closure_scope);
                }

                // Track which variables from outer scopes are referenced.
                // We do a simple scan: any identifier in the closure body
                // that is not a parameter and exists in the outer scope
                // is a captured variable.
                let mut captured = Vec::new();
                self.collect_captured_vars(closure_expr, &params.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>(), scope, &mut captured);
                for stmt in body.iter() {
                    self.collect_captured_vars_from_stmt(stmt, &params.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>(), scope, &mut captured);
                }
                captured.sort();
                captured.dedup();
                *captured_vars = captured;
            }
        }
    }
}