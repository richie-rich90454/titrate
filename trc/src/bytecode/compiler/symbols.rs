// Symbol table and scope management

use crate::ast;
use super::Compiler;

impl Compiler {
    // -----------------------------------------------------------------------
    // Local variable management
    // -----------------------------------------------------------------------

    pub(super) fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    pub(super) fn end_scope(&mut self) {
        self.scope_depth -= 1;
        // Remove locals that belong to the exited scope from the compile-time list.
        // We do NOT emit POP here because the VM pre-allocates all local slots
        // when a function is called, and the RET instruction cleans up the
        // entire frame. Emitting POP would shrink the runtime stack below
        // the pre-allocated area, causing LOAD_LOCAL to fail.
        while self.locals.last().map_or(false, |l| l.depth > self.scope_depth) {
            let _local = self.locals.pop().unwrap();
        }
    }

    pub(super) fn declare_local(&mut self, name: &str) -> Result<u8, String> {
        if self.local_count >= 255 {
            return Err("Too many local variables in function (max 255)".to_string());
        }
        let slot = self.local_count as u8;
        self.locals.push(super::Local {
            name: name.to_string(),
            depth: self.scope_depth,
            is_captured: false,
            slot,
            is_upvalue: false,
            upvalue_idx: 0,
        });
        self.local_count += 1;
        Ok(slot)
    }

    /// Declare a captured upvalue as a local-like entry so that identifier
    /// resolution inside the closure body can find it. Reads/writes of this
    /// local emit `GET_UPVALUE`/`SET_UPVALUE` with `upvalue_idx` instead of
    /// the regular `LOAD_LOCAL`/`STORE_LOCAL` with `slot`.
    pub(super) fn declare_upvalue(&mut self, name: &str, upvalue_idx: u8) -> Result<u8, String> {
        if self.local_count >= 255 {
            return Err("Too many local variables in function (max 255)".to_string());
        }
        let slot = self.local_count as u8;
        self.locals.push(super::Local {
            name: name.to_string(),
            depth: self.scope_depth,
            is_captured: false,
            slot,
            is_upvalue: true,
            upvalue_idx,
        });
        self.local_count += 1;
        Ok(slot)
    }

    pub(super) fn resolve_local(&self, name: &str) -> Option<u8> {
        // "self" resolves to the same slot as "this" in method bodies
        let lookup_name = if name == "self" { "this" } else { name };
        // Search from the end (most recent) to find the innermost variable.
        for local in self.locals.iter().rev() {
            if local.name == lookup_name {
                return Some(local.slot);
            }
        }
        None
    }

    /// Returns true if the local occupying `slot` is a captured upvalue
    /// (rather than a real stack slot). Use this after `resolve_local` to
    /// decide whether to emit `GET_UPVALUE`/`SET_UPVALUE` instead of
    /// `LOAD_LOCAL`/`STORE_LOCAL`.
    pub(super) fn is_local_upvalue(&self, slot: u8) -> bool {
        self.locals
            .iter()
            .rev()
            .find(|l| l.slot == slot)
            .map_or(false, |l| l.is_upvalue)
    }

    /// Returns the upvalue index for the local at `slot`. Only meaningful
    /// when `is_local_upvalue(slot)` returns true.
    pub(super) fn get_upvalue_index(&self, slot: u8) -> u8 {
        for local in self.locals.iter().rev() {
            if local.slot == slot && local.is_upvalue {
                return local.upvalue_idx;
            }
        }
        0
    }

    // -----------------------------------------------------------------------
    // Native function registration
    // -----------------------------------------------------------------------

    #[allow(dead_code)]
    pub(super) fn get_or_add_native(&mut self, name: &str) -> u16 {
        if let Some(&idx) = self.native_map.get(name) {
            return idx;
        }
        let idx = self.native_names.len() as u16;
        self.native_names.push(name.to_string());
        self.native_map.insert(name.to_string(), idx);
        idx
    }

    // -----------------------------------------------------------------------
    // First-pass registration
    // -----------------------------------------------------------------------

    pub(super) fn register_function(&mut self, fn_decl: &ast::FnDecl) {
        if !fn_decl.type_params.is_empty() {
            // Generic function: store for later instantiation.
            let idx = self.generic_functions.len();
            self.generic_function_map.insert(fn_decl.name.clone(), idx);
            self.generic_functions.push(fn_decl.clone());
            return;
        }

        let idx = self.functions.len() as u16;
        self.function_map.insert(fn_decl.name.clone(), idx);
        self.functions.push(super::FunctionDef {
            name: fn_decl.name.clone(),
            arity: fn_decl.params.len(),
            chunk: super::Chunk::new(),
            is_method: false,
            is_constructor: false,
            local_count: 0,
        });
    }

    pub(super) fn register_class(&mut self, class_decl: &ast::ClassDecl) -> Result<(), String> {
        if !class_decl.type_params.is_empty() {
            // Generic class: store for later instantiation.
            let idx = self.generic_classes.len();
            self.generic_class_map.insert(class_decl.name.clone(), idx);
            self.generic_classes.push(class_decl.clone());
            return Ok(());
        }

        let class_idx = self.classes.len() as u16;

        if self.class_map.contains_key(&class_decl.name) {
            // Specialized generic classes may already be registered via
            // mono_cache.  Return Ok instead of erroring so that multiple
            // compilation passes don't trip over the same specialization.
            if class_decl.name.contains("__") {
                return Ok(());
            }
            return Err(format!("Duplicate class '{}'", class_decl.name));
        }
        self.class_map.insert(class_decl.name.clone(), class_idx);

        let parent_idx = class_decl.parent.as_ref().and_then(|p| {
            self.class_map.get(p.name()).copied()
        });

        let mut fields = Vec::new();
        let mut field_inits = Vec::new();
        let mut methods = std::collections::HashMap::new();
        let mut constructor = None;

        for member in &class_decl.members {
            match member {
                ast::ClassMember::Field(field_decl) => {
                    let has_init = field_decl.init.is_some();
                    if field_decl.init.is_some() {
                        let init_chunk = super::Chunk::new();
                        // We'll compile the init expression later; for now just
                        // record a placeholder. The actual compilation happens in
                        // compile_class_methods.
                        field_inits.push((field_decl.name.clone(), init_chunk));
                    }
                    fields.push(super::FieldDef {
                        name: field_decl.name.clone(),
                        has_init,
                    });
                }
                ast::ClassMember::Method(method_decl) => {
                    let fn_idx = self.functions.len() as u16;
                    self.functions.push(super::FunctionDef {
                        name: format!("{}.{}", class_decl.name, method_decl.name),
                        arity: method_decl.params.len(),
                        chunk: super::Chunk::new(),
                        is_method: true,
                        is_constructor: false,
                        local_count: 0,
                    });
                    methods.insert(method_decl.name.clone(), fn_idx);
                }
                ast::ClassMember::Constructor(ctor_decl) => {
                    let fn_idx = self.functions.len() as u16;
                    self.functions.push(super::FunctionDef {
                        name: format!("{}.<init>", class_decl.name),
                        arity: ctor_decl.params.len(),
                        chunk: super::Chunk::new(),
                        is_method: true,
                        is_constructor: true,
                        local_count: 0,
                    });
                    methods.insert("init".to_string(), fn_idx);
                    constructor = Some(fn_idx);
                }
            }
        }

        self.classes.push(super::ClassDef {
            name: class_decl.name.clone(),
            parent: parent_idx,
            fields,
            methods,
            constructor,
            field_inits,
        });

        Ok(())
    }

    pub(super) fn register_enum(&mut self, enum_decl: &ast::EnumDecl) {
        let enum_idx = self.enums.len() as u16;
        self.enum_map.insert(enum_decl.name.clone(), enum_idx);

        let variants: Vec<super::VariantDef> = enum_decl
            .variants
            .iter()
            .enumerate()
            .map(|(i, v)| {
                // Register each variant name so we can look it up during call compilation.
                self.variant_map
                    .insert(v.name.clone(), (enum_decl.name.clone(), i));
                super::VariantDef {
                    name: v.name.clone(),
                    field_count: v.fields.len(),
                }
            })
            .collect();

        self.enums.push(super::EnumDef {
            name: enum_decl.name.clone(),
            variants,
        });
    }
}
