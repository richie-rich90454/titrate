// Call compilation

use crate::ast;
use super::super::opcodes::OpCode;
use super::{Compiler, Symbol};

impl Compiler {
    pub(super) fn compile_call(&mut self, callee: &ast::Expr, args: &[ast::Expr], line: u32) -> Result<(), String> {
        // Special case: super(...) call in a constructor.
        if let ast::Expr::Super(_) = callee {
            // super(args) calls the parent class's constructor.
            // We look up the parent class's constructor function index and
            // call it directly with `this` as the first argument.
            let parent_ctor_idx = if let Some(class_idx) = self.current_class {
                let parent_idx = self.classes[class_idx as usize].parent;
                if let Some(pidx) = parent_idx {
                    self.classes[pidx as usize].constructor
                } else {
                    return Err("super() call but class has no parent".to_string());
                }
            } else {
                return Err("super() call outside of class constructor".to_string());
            };

            match parent_ctor_idx {
                Some(ctor_fn_idx) => {
                    // Stack: [this, arg0, arg1, ...]
                    // Load `this` (slot 0 in method body)
                    self.emit_opcode(OpCode::LOAD_LOCAL, line);
                    self.emit_u8(0, line);
                    // Compile arguments
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    // Use CALL_SUPER which sets base to `this` position
                    self.emit_opcode(OpCode::CALL_SUPER, line);
                    self.emit_u16(ctor_fn_idx, line);
                    self.emit_u8(args.len() as u8, line);
                    // The constructor returns the instance; pop it.
                    self.emit_opcode(OpCode::POP, line);
                    self.emit_opcode(OpCode::PUSH_VOID, line);
                }
                None => {
                    // Parent has no explicit constructor; just discard args.
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    for _ in args {
                        self.emit_opcode(OpCode::POP, line);
                    }
                    self.emit_opcode(OpCode::PUSH_VOID, line);
                }
            }
            return Ok(());
        }

        // Special case: this(args) call in a constructor (constructor delegation).
        // this(args) calls another constructor of the same class with matching arity.
        if let ast::Expr::This(_) = callee {
            if let Some(class_idx) = self.current_class {
                let class_name = self.classes[class_idx as usize].name.clone();
                let ctor_pattern = format!("{}.<init>", class_name);
                let ctor_arity = args.len();
                // Find the constructor function entry by matching arity.
                let ctor_fn_idx = self.functions.iter().enumerate()
                    .find(|(_, f)| f.name == ctor_pattern && f.arity == ctor_arity)
                    .map(|(i, _)| i as u16);
                if let Some(fn_idx) = ctor_fn_idx {
                    // Stack: [this, arg0, arg1, ...]
                    // Load `this` (slot 0 in method body)
                    self.emit_opcode(OpCode::LOAD_LOCAL, line);
                    self.emit_u8(0, line);
                    // Compile arguments
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    // Use CALL_SUPER which sets base to `this` position
                    self.emit_opcode(OpCode::CALL_SUPER, line);
                    self.emit_u16(fn_idx, line);
                    self.emit_u8(args.len() as u8, line);
                    // The delegated constructor returns void; pop it.
                    self.emit_opcode(OpCode::POP, line);
                    self.emit_opcode(OpCode::PUSH_VOID, line);
                    return Ok(());
                }
            }
        }

        // Special case: Identifier("Ok") → RESULT_OK, Identifier("Err") → RESULT_ERR
        // Only exact capitalized forms trigger the built-in Result constructors;
        // lowercase `ok()` / `err()` resolve to user-defined functions.
        if let ast::Expr::Identifier(name, _) = callee {
            if name == "Ok" {
                if args.len() != 1 {
                    return Err("Ok() expects exactly 1 argument".to_string());
                }
                self.compile_expr(&args[0])?;
                self.emit_opcode(OpCode::RESULT_OK, line);
                return Ok(());
            }
            if name == "Err" {
                if args.len() != 1 {
                    return Err("Err() expects exactly 1 argument".to_string());
                }
                self.compile_expr(&args[0])?;
                self.emit_opcode(OpCode::RESULT_ERR, line);
                return Ok(());
            }

            // Check if it's an enum variant constructor.
            if let Some((enum_name, _variant_idx)) = self.variant_map.get(name).cloned() {
                let enum_name_idx = self.intern_string(&enum_name);
                let variant_name_idx = self.intern_string(name);
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::ENUM_NEW, line);
                self.emit_u16(enum_name_idx, line);
                self.emit_u16(variant_name_idx, line);
                self.emit_u8(args.len() as u8, line);
                return Ok(());
            }

            // Check if it's a known function.
            if let Some(&fn_idx) = self.function_map.get(name) {
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::CALL, line);
                self.emit_u16(fn_idx, line);
                self.emit_u8(args.len() as u8, line);
                return Ok(());
            }

            // Check for a function defined in the current module BEFORE
            // consulting the imported symbol table. A module's own private
            // helpers (e.g. Bootstrap.tr's local `phi`) must shadow same-named
            // functions from imported modules (e.g. Math.phi), otherwise the
            // imported overload wins and the call site gets the wrong arity.
            // We search self.functions for any entry whose mangled name
            // matches "<current_module>.<name>" and pick the overload whose
            // arity matches the call site.
            if !self.current_module.is_empty() && self.current_module != "<main>" {
                let local_mangled = format!("{}.{}", self.current_module, name);
                let target_arity = args.len();
                let local_match = self.functions.iter().enumerate()
                    .find(|(_, f)| f.name == local_mangled && f.arity == target_arity)
                    .or_else(|| self.functions.iter().enumerate().find(|(_, f)| f.name == local_mangled))
                    .map(|(i, _)| i as u16);
                if let Some(fn_idx) = local_match {
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    self.emit_opcode(OpCode::CALL, line);
                    self.emit_u16(fn_idx, line);
                    self.emit_u8(args.len() as u8, line);
                    return Ok(());
                }
            }

            // Check the imported symbol table for functions BEFORE falling
            // back to mangled-name lookup. When two modules export a function
            // with the same name (e.g. String.count and XPath.count), an
            // explicit `import tt::xml::XPath` must win over a mangled-name
            // candidate from another module. Otherwise the caller cannot
            // disambiguate which `count` they meant.
            if let Some(symbol) = self.symbol_table.get(name).cloned() {
                match symbol {
                    Symbol::Function(fn_idx) => {
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit_opcode(OpCode::CALL, line);
                        self.emit_u16(fn_idx, line);
                        self.emit_u8(args.len() as u8, line);
                        return Ok(());
                    }
                    Symbol::GenericFunction(idx) => {
                        let key = self.generic_functions[idx].name.clone();
                        let type_param_count = self.generic_functions[idx].type_params.len();
                        let placeholder = ast::Type::Named { name: "Variant".to_string(), params: vec![] };
                        let type_args: Vec<ast::Type> = (0..type_param_count).map(|_| placeholder.clone()).collect();
                        let fn_idx = self.instantiate_generic_function(&key, &type_args, args.len())?;
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit_opcode(OpCode::CALL, line);
                        self.emit_u16(fn_idx, line);
                        self.emit_u8(args.len() as u8, line);
                        return Ok(());
                    }
                    _ => {}
                }
            }

            // Try mangled name (for private functions within the current module).
            // When multiple modules define a function with the same name, prefer:
            //   1. The current module's version
            //   2. A candidate whose arity matches the call
            //   3. The first candidate (fallback)
            let mangled_candidates: Vec<String> = self.function_map.keys()
                .filter(|k| k.ends_with(&format!(".{}", name)))
                .cloned()
                .collect();
            if !mangled_candidates.is_empty() {
                let arg_count = args.len();
                let current_module = &self.current_module;
                let in_module = |k: &str| {
                    !current_module.is_empty()
                        && current_module != "<main>"
                        && k.starts_with(&format!("{}.", current_module))
                };
                let arity_matches = |k: &str| {
                    self.function_map.get(k)
                        .and_then(|&idx| self.functions.get(idx as usize))
                        .map(|f| f.arity == arg_count)
                        .unwrap_or(false)
                };
                let mangled = mangled_candidates.iter()
                    .find(|k| in_module(k) && arity_matches(k))
                    .or_else(|| mangled_candidates.iter().find(|k| in_module(k)))
                    .or_else(|| mangled_candidates.iter().find(|k| arity_matches(k)))
                    .or_else(|| mangled_candidates.first())
                    .cloned();
                if let Some(mangled) = mangled {
                    if let Some(&fn_idx) = self.function_map.get(&mangled) {
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit_opcode(OpCode::CALL, line);
                        self.emit_u16(fn_idx, line);
                        self.emit_u8(args.len() as u8, line);
                        return Ok(());
                    }
                }
            }

            // Check if it's a generic function (needs type arguments).
            // For module-level generics, the key is the mangled name
            // (e.g. "tt.math.ndarray.NDArrayMath.map"). Check both short
            // name and mangled name (current_module + "." + name).
            let generic_key = if self.generic_function_map.contains_key(name) {
                Some(name.to_string())
            } else if !self.current_module.is_empty() && self.current_module != "<main>" {
                let mangled = format!("{}.{}", self.current_module, name);
                if self.generic_function_map.contains_key(&mangled) {
                    Some(mangled)
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(key) = generic_key {
                let type_param_count = self.generic_functions[self.generic_function_map[&key]].type_params.len();
                let placeholder = ast::Type::Named { name: "Variant".to_string(), params: vec![] };
                let type_args: Vec<ast::Type> = (0..type_param_count).map(|_| placeholder.clone()).collect();
                let fn_idx = self.instantiate_generic_function(&key, &type_args, args.len())?;
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::CALL, line);
                self.emit_u16(fn_idx, line);
                self.emit_u8(args.len() as u8, line);
                return Ok(());
            }

            // Check if it's a native function call using the Module_function
            // convention (e.g., Math_sqrt, String_length). We emit a STATIC_CALL
            // so the VM can resolve it via its native lookup fallback.
            // Guard against identifiers that start with '_' (e.g. private helpers
            // like _quickSort) or are otherwise not in the Module_name form.
            if let Some(idx) = name.find('_') {
                let class_name = &name[..idx];
                let method_name = &name[idx + 1..];
                if !class_name.is_empty() {
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    let class_idx = self.intern_string(class_name);
                    let method_idx = self.intern_string(method_name);
                    self.emit_opcode(OpCode::STATIC_CALL, line);
                    self.emit_u16(class_idx, line);
                    self.emit_u16(method_idx, line);
                    self.emit_u8(args.len() as u8, line);
                    return Ok(());
                }
            }

            // Check if it's a registered native function (println, toString, parseInt).
            if self.native_map.contains_key(name) {
                let native_idx = self.native_map[name];
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::CALL_NATIVE, line);
                self.emit_u16(native_idx, line);
                self.emit_u8(args.len() as u8, line);
                return Ok(());
            }

            // Implicit this.method() inside a class method: when a bare
            // identifier call is not a function, native, or variable, treat
            // it as a virtual call on the current instance if the current
            // class (or any parent) declares the method.
            if let Some(class_idx) = self.current_class {
                let target_arity = args.len();
                let mut search_idx = Some(class_idx);
                let mut found_method = None;
                while let Some(idx) = search_idx {
                    let class_def = &self.classes[idx as usize];
                    if let Some(indices) = class_def.methods.get(name) {
                        // Pick the overload whose arity matches the call site.
                        // If none matches exactly, fall back to the first
                        // overload — the VM will surface the mismatch.
                        let matched = indices.iter().copied().find(|&fidx| {
                            self.functions[fidx as usize].arity == target_arity
                        }).or_else(|| indices.first().copied());
                        if let Some(m) = matched {
                            found_method = Some(m);
                            break;
                        }
                    }
                    search_idx = class_def.parent;
                }
                if let Some(_method_fn_idx) = found_method {
                    // Load `this` (slot 0 in a method body).
                    self.emit_opcode(OpCode::LOAD_LOCAL, line);
                    self.emit_u8(0, line);
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    let method_idx = self.intern_string(name);
                    self.emit_opcode(OpCode::INVOKE_VIRTUAL, line);
                    self.emit_u16(method_idx, line);
                    self.emit_u8(args.len() as u8, line);
                    return Ok(());
                }
            }
        }

        // Special case: MemberAccess callee → method call.
        if let ast::Expr::MemberAccess(ref obj, ref method, _) = *callee {
            // `super.m(args)` — static dispatch to the parent class.
            // Virtual dispatch would re-enter an override on `this` and
            // loop forever, so resolve the parent target at compile time.
            if let ast::Expr::Super(_) = **obj {
                let parent_idx = if let Some(class_idx) = self.current_class {
                    match self.classes.get(class_idx as usize).and_then(|c| c.parent) {
                        Some(p) => p,
                        None => return Err("super call but class has no parent".to_string()),
                    }
                } else {
                    return Err("super call outside of a class".to_string());
                };
                if method == "init" {
                    // Parent constructor: prefer the overload matching the
                    // call arity (like exec_new does at runtime), falling
                    // back to the class's default constructor field.
                    let parent_name = self.classes[parent_idx as usize].name.clone();
                    let ctor_pattern = format!("{}.<init>", parent_name);
                    let arity_ctor = self
                        .functions
                        .iter()
                        .enumerate()
                        .find(|(_, f)| f.name == ctor_pattern && f.arity == args.len())
                        .map(|(i, _)| i as u16);
                    let ctor = arity_ctor.or(self.classes[parent_idx as usize].constructor);
                    match ctor {
                        Some(ctor_fn_idx) => {
                            self.emit_opcode(OpCode::LOAD_LOCAL, line);
                            self.emit_u8(0, line);
                            for arg in args {
                                self.compile_expr(arg)?;
                            }
                            self.emit_opcode(OpCode::CALL_SUPER, line);
                            self.emit_u16(ctor_fn_idx, line);
                            self.emit_u8(args.len() as u8, line);
                            self.emit_opcode(OpCode::POP, line);
                            self.emit_opcode(OpCode::PUSH_VOID, line);
                        }
                        None => {
                            for arg in args {
                                self.compile_expr(arg)?;
                            }
                            for _ in args {
                                self.emit_opcode(OpCode::POP, line);
                            }
                            self.emit_opcode(OpCode::PUSH_VOID, line);
                        }
                    }
                    return Ok(());
                }
                // Parent method: direct call with `this` as receiver.
                let candidates = self.classes[parent_idx as usize]
                    .methods
                    .get(method)
                    .cloned()
                    .unwrap_or_default();
                let target = candidates.iter().copied().find(|&i| {
                    self.functions[i as usize].arity == args.len()
                });
                match target {
                    Some(fn_idx) => {
                        self.emit_opcode(OpCode::LOAD_LOCAL, line);
                        self.emit_u8(0, line);
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit_opcode(OpCode::CALL_SUPER, line);
                        self.emit_u16(fn_idx, line);
                        self.emit_u8(args.len() as u8, line);
                        return Ok(());
                    }
                    None => {
                        return Err(format!(
                            "super call to unknown parent method '{}'",
                            method
                        ));
                    }
                }
            }
            // Check for static calls like io.println, Integer.toString, etc.
            if let ast::Expr::Identifier(ref obj_name, _) = **obj {
                if self.is_builtin_object(obj_name) {
                    self.compile_static_call(obj_name, method, args, line)?;
                    return Ok(());
                }
                // Check if obj_name is a class name.
                // class_map keys may be mangled (e.g., "tt.crypto.Hash"), so
                // check both the exact name and a suffix match (".Hash").
                if self.class_map.contains_key(obj_name) {
                    self.compile_static_call(obj_name, method, args, line)?;
                    return Ok(());
                }
                if self.class_map.keys().any(|k| k.ends_with(&format!(".{}", obj_name))) {
                    self.compile_static_call(obj_name, method, args, line)?;
                    return Ok(());
                }
                // Check if obj_name is a module name (not a local variable or
                // global variable) and method is a known function. This handles
                // module-qualified calls like Indicators.sma(...), etc.
                // IMPORTANT: we must skip this path when obj_name is a global
                // variable (e.g. _currentNs), otherwise the symbol-table
                // fallback would resolve a same-named imported function
                // (e.g. HashMap.setDefault) and emit a CALL instead of an
                // INVOKE_VIRTUAL on the global's class.
                if self.resolve_local(obj_name).is_none() && self.lookup_global(obj_name).is_none() {
                    // Try mangled name lookup FIRST: any key ending with ".module.function".
                    // This is more specific than the symbol table (which may contain
                    // a different module's function with the same name).
                    let suffix = format!(".{}.{}", obj_name, method);
                    let mangled_key = self.function_map.keys()
                        .find(|k| k.ends_with(&suffix))
                        .cloned();
                    if let Some(key) = mangled_key {
                        if let Some(&fn_idx) = self.function_map.get(&key) {
                            for arg in args {
                                self.compile_expr(arg)?;
                            }
                            self.emit_opcode(OpCode::CALL, line);
                            self.emit_u16(fn_idx, line);
                            self.emit_u8(args.len() as u8, line);
                            return Ok(());
                        }
                    }
                    // Try generic function lookup: search generic_function_map
                    // for a key ending with ".module.function". This handles
                    // Module.genericFunction<T>(args) calls where the type args
                    // were discarded by the parser.
                    let generic_mangled_key = self.generic_function_map.keys()
                        .find(|k| k.ends_with(&suffix))
                        .cloned();
                    if let Some(key) = generic_mangled_key {
                        let type_param_count = {
                            let idx = self.generic_function_map[&key];
                            self.generic_functions[idx].type_params.len()
                        };
                        let placeholder = ast::Type::Named { name: "Variant".to_string(), params: vec![] };
                        let type_args: Vec<ast::Type> = (0..type_param_count).map(|_| placeholder.clone()).collect();
                        let fn_idx = self.instantiate_generic_function(&key, &type_args, args.len())?;
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit_opcode(OpCode::CALL, line);
                        self.emit_u16(fn_idx, line);
                        self.emit_u8(args.len() as u8, line);
                        return Ok(());
                    }
                    // Fall back to symbol table (imported functions registered by name).
                    if let Some(symbol) = self.symbol_table.get(method).cloned() {
                        match symbol {
                            Symbol::Function(fn_idx) => {
                                for arg in args {
                                    self.compile_expr(arg)?;
                                }
                                self.emit_opcode(OpCode::CALL, line);
                                self.emit_u16(fn_idx, line);
                                self.emit_u8(args.len() as u8, line);
                                return Ok(());
                            }
                            Symbol::GenericFunction(idx) => {
                                let key = self.generic_functions[idx].name.clone();
                                let type_param_count = self.generic_functions[idx].type_params.len();
                                let placeholder = ast::Type::Named { name: "Variant".to_string(), params: vec![] };
                                let type_args: Vec<ast::Type> = (0..type_param_count).map(|_| placeholder.clone()).collect();
                                let fn_idx = self.instantiate_generic_function(&key, &type_args, args.len())?;
                                for arg in args {
                                    self.compile_expr(arg)?;
                                }
                                self.emit_opcode(OpCode::CALL, line);
                                self.emit_u16(fn_idx, line);
                                self.emit_u8(args.len() as u8, line);
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                    // If obj names a known module, the method does not exist
                    // anywhere it could resolve. Report it honestly instead
                    // of falling through to a virtual call on a module,
                    // which is not a value.
                    let module_suffix = format!(".{}", obj_name);
                    let is_module = obj_name.contains("::")
                        || self
                            .modules
                            .iter()
                            .any(|m| m.name == *obj_name || m.name.ends_with(&module_suffix));
                    if is_module {
                        return Err(format!("Unknown static call: {}.{}", obj_name, method));
                    }
                }
            }

            // Regular method call: compile obj, then args, then INVOKE_VIRTUAL.
            self.compile_expr(obj)?;
            for arg in args {
                self.compile_expr(arg)?;
            }
            let method_idx = self.intern_string(method);
            self.emit_opcode(OpCode::INVOKE_VIRTUAL, line);
            self.emit_u16(method_idx, line);
            self.emit_u8(args.len() as u8, line);
            return Ok(());
        }

        // General case: callee is a complex expression (e.g., a closure variable
        // or a parenthesised expression). We need to emit a CALL that uses the
        // function index stored in the Closure value on the stack.
        //
        // If the callee is an Identifier that resolved to a local variable, it
        // might be holding a Closure. We emit CALL_CLOSURE which pops the callee
        // from the stack and calls it by its embedded function index.
        if let ast::Expr::Identifier(name, _) = callee {
            // Check if it's a local variable (possibly holding a closure).
            if let Some(slot) = self.resolve_local(name) {
                // Load the closure value, then args, then CALL_CLOSURE.
                if self.is_local_upvalue(slot) {
                    let idx = self.get_upvalue_index(slot);
                    self.emit_opcode(OpCode::GET_UPVALUE, line);
                    self.emit_u8(idx, line);
                } else {
                    self.emit_opcode(OpCode::LOAD_LOCAL, line);
                    self.emit_u8(slot, line);
                }
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::CALL_CLOSURE, line);
                self.emit_u8(args.len() as u8, line);
                return Ok(());
            }
            // Check if it's a global variable (possibly holding a closure).
            if let Some(gidx) = self.lookup_global(name) {
                self.emit_opcode(OpCode::LOAD_GLOBAL, line);
                self.emit_u16(gidx, line);
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::CALL_CLOSURE, line);
                self.emit_u8(args.len() as u8, line);
                return Ok(());
            }
            return Err(format!("Cannot resolve function '{}' at line {}", name, line));
        }
        // For non-Identifier callees (e.g., parenthesised expressions, index
        // expressions), compile the callee and use CALL_CLOSURE.
        self.compile_expr(callee)?;
        for arg in args {
            self.compile_expr(arg)?;
        }
        self.emit_opcode(OpCode::CALL_CLOSURE, line);
        self.emit_u8(args.len() as u8, line);

        Ok(())
    }
}
