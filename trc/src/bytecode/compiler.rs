// Titrate Alpha 0.1 – bytecode compiler
// Lowers the AST to bytecode chunks for the VM.
// Precision in every step – richie-rich90454, 2026

use std::collections::HashMap;

use crate::ast;
use super::frame::{ClassDef, EnumDef, FieldDef, FunctionDef, VariantDef};
use super::opcodes::{CastTarget, Chunk, OpCode};

// ---------------------------------------------------------------------------
// Compiled program – the output of the compiler
// ---------------------------------------------------------------------------

/// Everything the VM needs to execute a Titrate program.
pub struct CompiledProgram {
    pub functions: Vec<FunctionDef>,
    pub classes: Vec<ClassDef>,
    pub enums: Vec<EnumDef>,
    /// Ordered list of native function names the VM must resolve at startup.
    pub native_names: Vec<String>,
}

// ---------------------------------------------------------------------------
// Inferred type – used to pick the right typed opcode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InferredType {
    I8,
    I16,
    I32,
    I64,
    I128,
    U128,
    F32,
    F64,
    Bool,
    Char,
    String,
    Null,
    Void,
    Unknown,
}

// ---------------------------------------------------------------------------
// Local variable
// ---------------------------------------------------------------------------

struct Local {
    name: String,
    depth: usize,
    is_captured: bool,
    /// The stack slot assigned to this local.
    slot: u8,
}

// ---------------------------------------------------------------------------
// Loop bookkeeping for break/continue
// ---------------------------------------------------------------------------

struct LoopInfo {
    /// IP of the start of the loop (for `continue`).
    start_ip: usize,
    /// Locations to patch with the end-of-loop offset (for `break`).
    break_patches: Vec<(usize, u32)>,
}

// ---------------------------------------------------------------------------
// Compiler
// ---------------------------------------------------------------------------

pub struct Compiler {
    /// All compiled functions. Index 0 is reserved for the top-level "main" chunk.
    functions: Vec<FunctionDef>,
    /// All compiled classes.
    classes: Vec<ClassDef>,
    /// All compiled enums.
    enums: Vec<EnumDef>,
    /// Local variable slots for the current scope.
    locals: Vec<Local>,
    /// Scope depth (0 = top-level, 1 = inside function, etc.).
    scope_depth: usize,
    /// Current function index being compiled.
    current_function: usize,
    /// Function name → index mapping.
    function_map: HashMap<String, u16>,
    /// Class name → index mapping.
    class_map: HashMap<String, u16>,
    /// Enum name → index mapping.
    enum_map: HashMap<String, u16>,
    /// Loop stack for break/continue.
    loop_stack: Vec<LoopInfo>,
    /// Number of local slots used in current function.
    local_count: usize,
    /// Mapping from enum variant name → (enum_name, variant_index).
    variant_map: HashMap<String, (String, usize)>,
    /// Collected native function names.
    native_names: Vec<String>,
    /// Mapping from native name → index.
    native_map: HashMap<String, u16>,
}

impl Compiler {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    pub fn new() -> Self {
        // Slot 0 is reserved for the main chunk.
        let main_chunk = FunctionDef {
            name: "<main>".to_string(),
            arity: 0,
            chunk: Chunk::new(),
            is_method: false,
            is_constructor: false,
            local_count: 0,
        };

        Compiler {
            functions: vec![main_chunk],
            classes: Vec::new(),
            enums: Vec::new(),
            locals: Vec::new(),
            scope_depth: 0,
            current_function: 0,
            function_map: HashMap::new(),
            class_map: HashMap::new(),
            enum_map: HashMap::new(),
            loop_stack: Vec::new(),
            local_count: 0,
            variant_map: HashMap::new(),
            native_names: Vec::new(),
            native_map: HashMap::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Main entry point
    // -----------------------------------------------------------------------

    pub fn compile(&mut self, program: &ast::Program) -> Result<CompiledProgram, String> {
        // First pass: register all classes, enums, and functions (names and arities).
        for decl in &program.declarations {
            match decl {
                ast::Declaration::Class(class_decl) => self.register_class(class_decl)?,
                ast::Declaration::Enum(enum_decl) => self.register_enum(enum_decl),
                ast::Declaration::Function(fn_decl) => self.register_function(fn_decl),
                _ => {}
            }
        }

        // Second pass: compile all function bodies and class methods.
        for decl in &program.declarations {
            match decl {
                ast::Declaration::VarDecl(var_decl) => {
                    self.compile_var_decl(var_decl, 0)?;
                }
                ast::Declaration::ConstDecl(const_decl) => {
                    self.compile_var_decl(const_decl, 0)?;
                }
                ast::Declaration::Function(fn_decl) => {
                    self.compile_function(fn_decl)?;
                }
                ast::Declaration::Class(class_decl) => {
                    self.compile_class_methods(class_decl)?;
                }
                ast::Declaration::Enum(_) => {
                    // Already registered; no bodies to compile.
                }
                ast::Declaration::Interface(_) => {
                    // Interfaces have no runtime representation.
                }
            }
        }

        // If there's a `main` function, emit a CALL to it from the top-level chunk.
        self.current_function = 0;
        if let Some(&main_idx) = self.function_map.get("main") {
            self.emit_opcode(OpCode::CALL, 0);
            self.emit_u16(main_idx, 0);
            self.emit_u8(0, 0); // 0 arguments
        }

        // Emit a final RET for the main chunk.
        self.emit_opcode(OpCode::RET, 0);

        // Store the local count for the main chunk.
        self.functions[0].local_count = self.local_count;

        Ok(CompiledProgram {
            functions: std::mem::take(&mut self.functions),
            classes: std::mem::take(&mut self.classes),
            enums: std::mem::take(&mut self.enums),
            native_names: std::mem::take(&mut self.native_names),
        })
    }

    // -----------------------------------------------------------------------
    // First-pass registration
    // -----------------------------------------------------------------------

    fn register_function(&mut self, fn_decl: &ast::FnDecl) {
        let idx = self.functions.len() as u16;
        self.function_map.insert(fn_decl.name.clone(), idx);
        self.functions.push(FunctionDef {
            name: fn_decl.name.clone(),
            arity: fn_decl.params.len(),
            chunk: Chunk::new(),
            is_method: false,
            is_constructor: false,
            local_count: 0,
        });
    }

    fn register_class(&mut self, class_decl: &ast::ClassDecl) -> Result<(), String> {
        let class_idx = self.classes.len() as u16;

        if self.class_map.contains_key(&class_decl.name) {
            return Err(format!("Duplicate class '{}'", class_decl.name));
        }
        self.class_map.insert(class_decl.name.clone(), class_idx);

        let parent_idx = class_decl.parent.as_ref().and_then(|p| {
            self.class_map.get(p.name()).copied()
        });

        let mut fields = Vec::new();
        let mut field_inits = Vec::new();
        let mut methods = HashMap::new();
        let mut constructor = None;

        for member in &class_decl.members {
            match member {
                ast::ClassMember::Field(field_decl) => {
                    let has_init = field_decl.init.is_some();
                    if field_decl.init.is_some() {
                        let init_chunk = Chunk::new();
                        // We'll compile the init expression later; for now just
                        // record a placeholder. The actual compilation happens in
                        // compile_class_methods.
                        field_inits.push((field_decl.name.clone(), init_chunk));
                    }
                    fields.push(FieldDef {
                        name: field_decl.name.clone(),
                        has_init,
                    });
                }
                ast::ClassMember::Method(method_decl) => {
                    let fn_idx = self.functions.len() as u16;
                    self.functions.push(FunctionDef {
                        name: format!("{}.{}", class_decl.name, method_decl.name),
                        arity: method_decl.params.len(),
                        chunk: Chunk::new(),
                        is_method: true,
                        is_constructor: false,
                        local_count: 0,
                    });
                    methods.insert(method_decl.name.clone(), fn_idx);
                }
                ast::ClassMember::Constructor(ctor_decl) => {
                    let fn_idx = self.functions.len() as u16;
                    self.functions.push(FunctionDef {
                        name: format!("{}.<init>", class_decl.name),
                        arity: ctor_decl.params.len(),
                        chunk: Chunk::new(),
                        is_method: true,
                        is_constructor: true,
                        local_count: 0,
                    });
                    methods.insert("init".to_string(), fn_idx);
                    constructor = Some(fn_idx);
                }
            }
        }

        self.classes.push(ClassDef {
            name: class_decl.name.clone(),
            parent: parent_idx,
            fields,
            methods,
            constructor,
            field_inits,
        });

        Ok(())
    }

    fn register_enum(&mut self, enum_decl: &ast::EnumDecl) {
        let enum_idx = self.enums.len() as u16;
        self.enum_map.insert(enum_decl.name.clone(), enum_idx);

        let variants: Vec<VariantDef> = enum_decl
            .variants
            .iter()
            .enumerate()
            .map(|(i, v)| {
                // Register each variant name so we can look it up during call compilation.
                self.variant_map
                    .insert(v.name.clone(), (enum_decl.name.clone(), i));
                VariantDef {
                    name: v.name.clone(),
                    field_count: v.fields.len(),
                }
            })
            .collect();

        self.enums.push(EnumDef {
            name: enum_decl.name.clone(),
            variants,
        });
    }

    // -----------------------------------------------------------------------
    // Chunk access helpers
    // -----------------------------------------------------------------------

    fn current_chunk(&mut self) -> &mut Chunk {
        &mut self.functions[self.current_function].chunk
    }

    fn emit_opcode(&mut self, op: OpCode, line: u32) {
        self.current_chunk().write_opcode(op, line);
    }

    fn emit_u8(&mut self, value: u8, line: u32) {
        self.current_chunk().write_u8(value, line);
    }

    fn emit_u16(&mut self, value: u16, line: u32) {
        self.current_chunk().write_u16(value, line);
    }

    fn emit_i16(&mut self, value: i16, line: u32) {
        self.current_chunk().write_i16(value, line);
    }

    fn current_ip(&mut self) -> usize {
        self.current_chunk().code.len()
    }

    fn intern_string(&mut self, s: &str) -> u16 {
        self.current_chunk().add_string(s)
    }

    fn patch_i16_at(&mut self, offset: usize, value: i16) {
        let bytes = value.to_be_bytes();
        let chunk = self.current_chunk();
        chunk.code[offset] = bytes[0];
        chunk.code[offset + 1] = bytes[1];
    }

    // -----------------------------------------------------------------------
    // Local variable management
    // -----------------------------------------------------------------------

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope_depth -= 1;
        // Pop locals that belong to the exited scope.
        while self.locals.last().map_or(false, |l| l.depth > self.scope_depth) {
            let _local = self.locals.pop().unwrap();
            // Emit POP to clean up the stack slot at runtime.
            self.emit_opcode(OpCode::POP, 0);
            // Note: we do NOT decrement local_count here. Slot numbers are
            // fixed at compile time and must remain stable.
        }
    }

    fn declare_local(&mut self, name: &str) -> u8 {
        let slot = self.local_count as u8;
        self.locals.push(Local {
            name: name.to_string(),
            depth: self.scope_depth,
            is_captured: false,
            slot,
        });
        self.local_count += 1;
        slot
    }

    fn resolve_local(&self, name: &str) -> Option<u8> {
        // Search from the end (most recent) to find the innermost variable.
        for local in self.locals.iter().rev() {
            if local.name == name {
                return Some(local.slot);
            }
        }
        None
    }

    // -----------------------------------------------------------------------
    // Native function registration
    // -----------------------------------------------------------------------

    fn get_or_add_native(&mut self, name: &str) -> u16 {
        if let Some(&idx) = self.native_map.get(name) {
            return idx;
        }
        let idx = self.native_names.len() as u16;
        self.native_names.push(name.to_string());
        self.native_map.insert(name.to_string(), idx);
        idx
    }

    // -----------------------------------------------------------------------
    // Second-pass compilation
    // -----------------------------------------------------------------------

    fn compile_function(&mut self, fn_decl: &ast::FnDecl) -> Result<(), String> {
        let fn_idx = *self
            .function_map
            .get(&fn_decl.name)
            .ok_or_else(|| format!("Function '{}' not registered", fn_decl.name))?
            as usize;

        let saved_function = self.current_function;
        let saved_locals = std::mem::take(&mut self.locals);
        let saved_local_count = self.local_count;
        let saved_scope_depth = self.scope_depth;

        self.current_function = fn_idx;
        self.locals.clear();
        self.local_count = 0;
        self.scope_depth = 0;

        self.begin_scope();

        // Parameters become local variables (slot 0, 1, 2, ...).
        for param in &fn_decl.params {
            self.declare_local(&param.name);
        }

        self.compile_block(&fn_decl.body)?;

        // Ensure every function ends with RET.
        self.emit_opcode(OpCode::PUSH_VOID, 0);
        self.emit_opcode(OpCode::RET, 0);

        self.end_scope();

        // Store the number of local slots needed by this function.
        self.functions[fn_idx].local_count = self.local_count;

        self.current_function = saved_function;
        self.locals = saved_locals;
        self.local_count = saved_local_count;
        self.scope_depth = saved_scope_depth;

        Ok(())
    }

    fn compile_class_methods(&mut self, class_decl: &ast::ClassDecl) -> Result<(), String> {
        let class_idx = *self
            .class_map
            .get(&class_decl.name)
            .ok_or_else(|| format!("Class '{}' not registered", class_decl.name))?
            as usize;

        // Compile field initialisers.
        for member in &class_decl.members {
            if let ast::ClassMember::Field(field_decl) = member {
                if let Some(ref init_expr) = field_decl.init {
                    // Find the matching field_init slot in the class def.
                    let saved_fn = self.current_function;
                    // We compile field inits into the main chunk for simplicity.
                    // The VM will execute them during object construction.
                    self.current_function = 0;
                    self.compile_expr(init_expr)?;
                    self.emit_opcode(OpCode::POP, 0);
                    self.current_function = saved_fn;
                }
            }
        }

        // Compile each method body.
        for member in &class_decl.members {
            match member {
                ast::ClassMember::Method(method_decl) => {
                    let method_fn_idx = self
                        .classes
                        .get(class_idx)
                        .and_then(|c| c.methods.get(&method_decl.name))
                        .copied()
                        .ok_or_else(|| {
                            format!(
                                "Method '{}' not found in class '{}'",
                                method_decl.name, class_decl.name
                            )
                        })?;

                    self.compile_method_body(
                        method_fn_idx as usize,
                        &method_decl.params,
                        &method_decl.body,
                    )?;
                }
                ast::ClassMember::Constructor(ctor_decl) => {
                    let ctor_fn_idx = self
                        .classes
                        .get(class_idx)
                        .and_then(|c| c.constructor)
                        .ok_or_else(|| {
                            format!("Constructor not found in class '{}'", class_decl.name)
                        })?;

                    self.compile_method_body(
                        ctor_fn_idx as usize,
                        &ctor_decl.params,
                        &ctor_decl.body,
                    )?;
                }
                ast::ClassMember::Field(_) => {}
            }
        }

        Ok(())
    }

    fn compile_method_body(
        &mut self,
        fn_idx: usize,
        params: &[ast::Param],
        body: &ast::Block,
    ) -> Result<(), String> {
        let saved_function = self.current_function;
        let saved_locals = std::mem::take(&mut self.locals);
        let saved_local_count = self.local_count;
        let saved_scope_depth = self.scope_depth;

        self.current_function = fn_idx;
        self.locals.clear();
        self.local_count = 0;
        self.scope_depth = 0;

        self.begin_scope();

        // Slot 0 = "this"
        self.declare_local("this");

        // Parameters start at slot 1.
        for param in params {
            self.declare_local(&param.name);
        }

        self.compile_block(body)?;

        // Ensure every method ends with RET.
        self.emit_opcode(OpCode::PUSH_VOID, 0);
        self.emit_opcode(OpCode::RET, 0);

        self.end_scope();

        // Store the number of local slots needed by this method.
        self.functions[fn_idx].local_count = self.local_count;

        self.current_function = saved_function;
        self.locals = saved_locals;
        self.local_count = saved_local_count;
        self.scope_depth = saved_scope_depth;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Statement compilation
    // -----------------------------------------------------------------------

    fn compile_block(&mut self, block: &ast::Block) -> Result<(), String> {
        for stmt in block {
            self.compile_stmt(stmt)?;
        }
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &ast::Stmt) -> Result<(), String> {
        match stmt {
            ast::Stmt::VarDecl(var_decl) => {
                self.compile_var_decl(var_decl, 0)?;
            }
            ast::Stmt::ConstDecl(const_decl) => {
                self.compile_var_decl(const_decl, 0)?;
            }
            ast::Stmt::Expr(expr) => {
                self.compile_expr(expr)?;
                // Expression statements: the result stays on the stack.
                // Pop it unless it's void (the VM may optimise this).
                self.emit_opcode(OpCode::POP, 0);
            }
            ast::Stmt::If(if_stmt) => {
                self.compile_if(if_stmt)?;
            }
            ast::Stmt::While(while_stmt) => {
                self.compile_while(while_stmt)?;
            }
            ast::Stmt::For(for_stmt) => {
                self.compile_for(for_stmt)?;
            }
            ast::Stmt::Return(expr) => {
                if let Some(value) = expr {
                    self.compile_expr(value)?;
                } else {
                    self.emit_opcode(OpCode::PUSH_VOID, 0);
                }
                self.emit_opcode(OpCode::RET, 0);
            }
            ast::Stmt::Break => {
                self.compile_break(0)?;
            }
            ast::Stmt::Continue => {
                self.compile_continue(0)?;
            }
            ast::Stmt::Switch(switch_stmt) => {
                self.compile_switch(switch_stmt)?;
            }
            ast::Stmt::Block(block) => {
                self.begin_scope();
                self.compile_block(block)?;
                self.end_scope();
            }
        }
        Ok(())
    }

    fn compile_var_decl(&mut self, var_decl: &ast::VarDecl, line: u32) -> Result<(), String> {
        if let Some(ref init) = var_decl.init {
            self.compile_expr(init)?;
        } else {
            self.emit_opcode(OpCode::PUSH_NULL, line);
        }
        let slot = self.declare_local(&var_decl.name);
        self.emit_opcode(OpCode::STORE_LOCAL, line);
        self.emit_u8(slot, line);
        Ok(())
    }

    fn compile_if(&mut self, if_stmt: &ast::IfStmt) -> Result<(), String> {
        // compile condition
        self.compile_expr(&if_stmt.condition)?;
        self.emit_opcode(OpCode::JMP_IF_FALSE, 0);
        let else_jump_offset = self.current_ip();
        self.emit_i16(0, 0); // placeholder

        // compile then_branch
        self.compile_block(&if_stmt.then_branch)?;

        if let Some(ref else_branch) = if_stmt.else_branch {
            // Jump over else branch after then.
            self.emit_opcode(OpCode::JMP, 0);
            let end_jump_offset = self.current_ip();
            self.emit_i16(0, 0); // placeholder

            // Patch else jump to land here.
            let else_start = self.current_ip();
            let then_instr_end = else_jump_offset + 2; // past the i16 operand
            let offset = (else_start - then_instr_end) as i16;
            self.patch_i16_at(else_jump_offset, offset);

            // compile else_branch
            self.compile_block(else_branch)?;

            // Patch end jump.
            let end_ip = self.current_ip();
            let end_instr_end = end_jump_offset + 2;
            let offset = (end_ip - end_instr_end) as i16;
            self.patch_i16_at(end_jump_offset, offset);
        } else {
            // No else branch: patch the JMP_IF_FALSE to jump to end.
            let end_ip = self.current_ip();
            let then_instr_end = else_jump_offset + 2;
            let offset = (end_ip - then_instr_end) as i16;
            self.patch_i16_at(else_jump_offset, offset);
        }

        Ok(())
    }

    fn compile_while(&mut self, while_stmt: &ast::WhileStmt) -> Result<(), String> {
        let loop_start = self.current_ip();

        self.loop_stack.push(LoopInfo {
            start_ip: loop_start,
            break_patches: Vec::new(),
        });

        // compile condition
        self.compile_expr(&while_stmt.condition)?;
        self.emit_opcode(OpCode::JMP_IF_FALSE, 0);
        let exit_jump_offset = self.current_ip();
        self.emit_i16(0, 0); // placeholder

        // compile body
        self.compile_block(&while_stmt.body)?;

        // Jump back to loop start.
        self.emit_opcode(OpCode::JMP, 0);
        let current = self.current_ip() + 2; // after the i16 operand
        let offset = (loop_start as isize - current as isize) as i16;
        self.emit_i16(offset, 0);

        // Patch the exit jump.
        let end_ip = self.current_ip();
        let exit_instr_end = exit_jump_offset + 2;
        let offset = (end_ip - exit_instr_end) as i16;
        self.patch_i16_at(exit_jump_offset, offset);

        // Patch all break jumps.
        let loop_info = self.loop_stack.pop().unwrap();
        for (patch_offset, _line) in &loop_info.break_patches {
            let patch_instr_end = *patch_offset + 2;
            let offset = (end_ip - patch_instr_end) as i16;
            self.patch_i16_at(*patch_offset, offset);
        }

        Ok(())
    }

    fn compile_for(&mut self, for_stmt: &ast::ForStmt) -> Result<(), String> {
        self.begin_scope();

        // Compile the iterable expression and store it in a local.
        self.compile_expr(&for_stmt.iterable)?;
        let iter_slot = self.declare_local("__iter");
        self.emit_opcode(OpCode::STORE_LOCAL, 0);
        self.emit_u8(iter_slot, 0);

        // Initialize the index counter to 0.
        self.emit_opcode(OpCode::PUSH_I64, 0);
        let bytes = 0i64.to_be_bytes();
        for &b in &bytes {
            self.emit_u8(b, 0);
        }
        let idx_slot = self.declare_local("__iter_idx");
        self.emit_opcode(OpCode::STORE_LOCAL, 0);
        self.emit_u8(idx_slot, 0);

        // Get the length of the iterable and store it.
        self.emit_opcode(OpCode::LOAD_LOCAL, 0);
        self.emit_u8(iter_slot, 0);
        self.emit_opcode(OpCode::ARRAY_LEN, 0);
        let len_slot = self.declare_local("__iter_len");
        self.emit_opcode(OpCode::STORE_LOCAL, 0);
        self.emit_u8(len_slot, 0);

        let loop_start = self.current_ip();

        self.loop_stack.push(LoopInfo {
            start_ip: loop_start,
            break_patches: Vec::new(),
        });

        // Check: idx < len
        self.emit_opcode(OpCode::LOAD_LOCAL, 0);
        self.emit_u8(idx_slot, 0);
        self.emit_opcode(OpCode::LOAD_LOCAL, 0);
        self.emit_u8(len_slot, 0);
        self.emit_opcode(OpCode::LT_I64, 0);
        self.emit_opcode(OpCode::JMP_IF_FALSE, 0);
        let exit_jump_offset = self.current_ip();
        self.emit_i16(0, 0); // placeholder

        // Load the element: __iter[__iter_idx]
        self.emit_opcode(OpCode::LOAD_LOCAL, 0);
        self.emit_u8(iter_slot, 0);
        self.emit_opcode(OpCode::LOAD_LOCAL, 0);
        self.emit_u8(idx_slot, 0);
        self.emit_opcode(OpCode::ARRAY_GET, 0);

        // Store the element in the loop variable.
        let loop_var_slot = self.declare_local(&for_stmt.var);
        self.emit_opcode(OpCode::STORE_LOCAL, 0);
        self.emit_u8(loop_var_slot, 0);

        // compile body
        self.compile_block(&for_stmt.body)?;

        // Increment the index: __iter_idx = __iter_idx + 1
        self.emit_opcode(OpCode::LOAD_LOCAL, 0);
        self.emit_u8(idx_slot, 0);
        self.emit_opcode(OpCode::PUSH_I64, 0);
        let one_bytes = 1i64.to_be_bytes();
        for &b in &one_bytes {
            self.emit_u8(b, 0);
        }
        self.emit_opcode(OpCode::ADD_I64, 0);
        self.emit_opcode(OpCode::STORE_LOCAL, 0);
        self.emit_u8(idx_slot, 0);

        // Jump back to loop start.
        self.emit_opcode(OpCode::JMP, 0);
        let current = self.current_ip() + 2;
        let offset = (loop_start as isize - current as isize) as i16;
        self.emit_i16(offset, 0);

        // Patch the exit jump.
        let end_ip = self.current_ip();
        let exit_instr_end = exit_jump_offset + 2;
        let offset = (end_ip - exit_instr_end) as i16;
        self.patch_i16_at(exit_jump_offset, offset);

        // Patch all break jumps.
        let loop_info = self.loop_stack.pop().unwrap();
        for (patch_offset, _line) in &loop_info.break_patches {
            let patch_instr_end = *patch_offset + 2;
            let offset = (end_ip - patch_instr_end) as i16;
            self.patch_i16_at(*patch_offset, offset);
        }

        self.end_scope();
        Ok(())
    }

    fn compile_break(&mut self, line: u32) -> Result<(), String> {
        if self.loop_stack.is_empty() {
            return Err("'break' outside of loop".to_string());
        }
        self.emit_opcode(OpCode::JMP, line);
        let patch_offset = self.current_ip();
        self.emit_i16(0, line); // placeholder
        self.loop_stack
            .last_mut()
            .unwrap()
            .break_patches
            .push((patch_offset, line));
        Ok(())
    }

    fn compile_continue(&mut self, line: u32) -> Result<(), String> {
        if self.loop_stack.is_empty() {
            return Err("'continue' outside of loop".to_string());
        }
        let loop_start = self.loop_stack.last().unwrap().start_ip;
        self.emit_opcode(OpCode::JMP, line);
        let current = self.current_ip() + 2;
        let offset = (loop_start as isize - current as isize) as i16;
        self.emit_i16(offset, line);
        Ok(())
    }

    fn compile_switch(&mut self, switch_stmt: &ast::SwitchStmt) -> Result<(), String> {
        // Compile the subject expression.
        self.compile_expr(&switch_stmt.expr)?;

        let mut end_jumps: Vec<usize> = Vec::new();

        for case in &switch_stmt.cases {
            // DUP the subject for matching.
            self.emit_opcode(OpCode::DUP, 0);

            // Compile the pattern match.
            self.compile_pattern_match(&case.pattern)?;

            // If pattern doesn't match, jump to next case.
            self.emit_opcode(OpCode::JMP_IF_FALSE, 0);
            let next_case_offset = self.current_ip();
            self.emit_i16(0, 0); // placeholder

            // Pattern matched: store extracted fields into local variables.
            if let ast::Pattern::Constructor { bindings, .. } = &case.pattern {
                if !bindings.is_empty() {
                    // Fields are on stack in order (first deepest, last on top).
                    // Store them in reverse order.
                    for binding in bindings.iter().rev() {
                        if binding != "_" {
                            let slot = self.declare_local(binding);
                            self.emit_opcode(OpCode::STORE_LOCAL, 0);
                            self.emit_u8(slot, 0);
                        } else {
                            // Wildcard: just pop the field.
                            self.emit_opcode(OpCode::POP, 0);
                        }
                    }
                }
            }

            // POP the subject.
            self.emit_opcode(OpCode::POP, 0);

            // Compile case body.
            self.compile_block(&case.body)?;

            // Jump to end of switch (so we don't fall through).
            self.emit_opcode(OpCode::JMP, 0);
            let end_jump_offset = self.current_ip();
            self.emit_i16(0, 0); // placeholder
            end_jumps.push(end_jump_offset);

            // Patch the next-case jump to land here.
            let next_ip = self.current_ip();
            let next_instr_end = next_case_offset + 2;
            let offset = (next_ip - next_instr_end) as i16;
            self.patch_i16_at(next_case_offset, offset);
        }

        // Default case (if any).
        if let Some(ref default_body) = switch_stmt.default {
            // POP the subject (no case matched).
            self.emit_opcode(OpCode::POP, 0);
            self.compile_block(default_body)?;
        } else {
            // No default: just pop the subject.
            self.emit_opcode(OpCode::POP, 0);
        }

        // Patch all end jumps.
        let end_ip = self.current_ip();
        for offset in &end_jumps {
            let instr_end = *offset + 2;
            let jump = (end_ip - instr_end) as i16;
            self.patch_i16_at(*offset, jump);
        }

        Ok(())
    }

    fn compile_pattern_match(&mut self, pattern: &ast::Pattern) -> Result<(), String> {
        match pattern {
            ast::Pattern::Literal(lit) => {
                // Compile the literal, then emit equality comparison.
                self.compile_literal(lit)?;
                let ty = self.infer_literal_type(lit);
                self.emit_eq_opcode(ty);
            }
            ast::Pattern::Wildcard => {
                // Always matches. Replace the DUP'd subject with true.
                self.emit_opcode(OpCode::POP, 0);
                self.emit_opcode(OpCode::PUSH_BOOL, 0);
                self.emit_u8(1, 0);
            }
            ast::Pattern::Constructor { name, bindings } => {
                // Check if this is an enum variant.
                if let Some((_enum_name, _variant_idx)) = self.variant_map.get(name) {
                    // MATCH_ENUM: u16 variant name index + i16 jump offset
                    let variant_idx = self.intern_string(name);
                    self.emit_opcode(OpCode::MATCH_ENUM, 0);
                    self.emit_u16(variant_idx, 0);
                    // The jump offset for MATCH_ENUM is handled by the VM at runtime.
                    // We emit 0 as placeholder; the VM reads the variant name and decides.
                    self.emit_i16(0, 0);
                } else if name == "Ok" {
                    self.emit_opcode(OpCode::MATCH_OK, 0);
                    self.emit_i16(0, 0);
                } else if name == "Err" {
                    self.emit_opcode(OpCode::MATCH_ERR, 0);
                    self.emit_i16(0, 0);
                } else {
                    return Err(format!("Unknown pattern constructor '{}'", name));
                }

                // If there are bindings, we don't need to do anything extra here;
                // the VM will extract the fields after a successful match.
                let _ = bindings;
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Expression compilation
    // -----------------------------------------------------------------------

    fn compile_expr(&mut self, expr: &ast::Expr) -> Result<(), String> {
        match expr {
            ast::Expr::Literal(lit) => {
                self.compile_literal(lit)?;
            }
            ast::Expr::Identifier(name) => {
                self.compile_identifier(name)?;
            }
            ast::Expr::Binary(left, op, right) => {
                self.compile_binary(left, op, right)?;
            }
            ast::Expr::Unary(op, operand) => {
                self.compile_unary(op, operand)?;
            }
            ast::Expr::Call(callee, args) => {
                self.compile_call(callee, args)?;
            }
            ast::Expr::MemberAccess(obj, member) => {
                self.compile_member_access(obj, member)?;
            }
            ast::Expr::Index(obj, index) => {
                self.compile_expr(obj)?;
                self.compile_expr(index)?;
                self.emit_opcode(OpCode::ARRAY_GET, 0);
            }
            ast::Expr::New(typ, args) => {
                self.compile_new(typ, args)?;
            }
            ast::Expr::This => {
                // In methods, "this" is always slot 0.
                self.emit_opcode(OpCode::LOAD_LOCAL, 0);
                self.emit_u8(0, 0);
            }
            ast::Expr::Super => {
                // "super" resolves to "this" for method dispatch.
                self.emit_opcode(OpCode::LOAD_LOCAL, 0);
                self.emit_u8(0, 0);
            }
            ast::Expr::OwnedDeref(inner) => {
                self.compile_expr(inner)?;
                self.emit_opcode(OpCode::UNBOX_VALUE, 0);
            }
            ast::Expr::RegionAlloc(_typ, init) => {
                self.compile_expr(init)?;
                self.emit_opcode(OpCode::REGION_ALLOC, 0);
            }
            ast::Expr::RefExpr(inner, kind) => {
                self.compile_expr(inner)?;
                match kind {
                    ast::RefKind::Immutable => self.emit_opcode(OpCode::REF_IMMUTABLE, 0),
                    ast::RefKind::Mutable => self.emit_opcode(OpCode::REF_MUTABLE, 0),
                }
            }
            ast::Expr::UnsafeBlock(block) => {
                // Compile as a regular block.
                self.begin_scope();
                self.compile_block(block)?;
                self.end_scope();
            }
            ast::Expr::ErrorPropagation(inner) => {
                self.compile_expr(inner)?;
                self.emit_opcode(OpCode::UNWRAP_OR_PROPAGATE, 0);
            }
            ast::Expr::Cast(inner, target_type) => {
                self.compile_expr(inner)?;
                let cast_target = self.type_to_cast_target(target_type);
                self.emit_opcode(OpCode::CAST, 0);
                self.emit_u8(cast_target as u8, 0);
            }
            ast::Expr::StaticCall {
                class_name,
                method,
                args,
            } => {
                self.compile_static_call(class_name, method, args)?;
            }
            ast::Expr::Assign(target, value) => {
                self.compile_assign(target, value)?;
            }
        }
        Ok(())
    }

    fn compile_literal(&mut self, lit: &ast::Literal) -> Result<(), String> {
        match lit {
            ast::Literal::Int(v) => {
                self.emit_opcode(OpCode::PUSH_I64, 0);
                let bytes = (*v as i64).to_be_bytes();
                for &b in &bytes {
                    self.emit_u8(b, 0);
                }
            }
            ast::Literal::Float(v) => {
                self.emit_opcode(OpCode::PUSH_F64, 0);
                let bytes = (*v as f64).to_be_bytes();
                for &b in &bytes {
                    self.emit_u8(b, 0);
                }
            }
            ast::Literal::Bool(b) => {
                self.emit_opcode(OpCode::PUSH_BOOL, 0);
                self.emit_u8(if *b { 1 } else { 0 }, 0);
            }
            ast::Literal::Char(c) => {
                self.emit_opcode(OpCode::PUSH_CHAR, 0);
                let bytes = (*c as u32).to_be_bytes();
                for &b in &bytes {
                    self.emit_u8(b, 0);
                }
            }
            ast::Literal::String(s) => {
                let idx = self.intern_string(s);
                self.emit_opcode(OpCode::PUSH_STRING, 0);
                self.emit_u16(idx, 0);
            }
            ast::Literal::Null => {
                self.emit_opcode(OpCode::PUSH_NULL, 0);
            }
        }
        Ok(())
    }

    fn compile_identifier(&mut self, name: &str) -> Result<(), String> {
        // Check locals first.
        if let Some(slot) = self.resolve_local(name) {
            self.emit_opcode(OpCode::LOAD_LOCAL, 0);
            self.emit_u8(slot, 0);
            return Ok(());
        }

        // Check if it's a known function.
        if let Some(&fn_idx) = self.function_map.get(name) {
            self.emit_opcode(OpCode::PUSH_VOID, 0); // placeholder – function refs not yet in value
            let _ = fn_idx;
            // For now, function calls are handled directly in compile_call.
            // If we reach here, it's a bare function reference.
            return Ok(());
        }

        // Check if it's an enum variant (bare reference without call).
        if self.variant_map.contains_key(name) {
            // This is a partial application – the variant will be called later.
            // For now, emit a placeholder.
            self.emit_opcode(OpCode::PUSH_NULL, 0);
            return Ok(());
        }

        // Unknown identifier – could be a global or builtin.
        // Emit a LOAD_LOCAL with slot 0 as a fallback; the VM should handle this.
        // In practice, the analyzer should catch undefined variables.
        self.emit_opcode(OpCode::LOAD_LOCAL, 0);
        self.emit_u8(0, 0);
        Ok(())
    }

    fn compile_binary(
        &mut self,
        left: &ast::Expr,
        op: &ast::Operator,
        right: &ast::Expr,
    ) -> Result<(), String> {
        // Short-circuit for And/Or.
        match op {
            ast::Operator::And => {
                // And: compile left, JMP_IF_FALSE(skip), compile right, JMP(end),
                //      (skip:) PUSH_BOOL(false), (end:)
                self.compile_expr(left)?;
                self.emit_opcode(OpCode::JMP_IF_FALSE, 0);
                let skip_offset = self.current_ip();
                self.emit_i16(0, 0); // placeholder

                self.compile_expr(right)?;
                self.emit_opcode(OpCode::JMP, 0);
                let end_offset = self.current_ip();
                self.emit_i16(0, 0); // placeholder

                // skip: PUSH_BOOL(false)
                let skip_ip = self.current_ip();
                self.patch_i16_at(skip_offset, (skip_ip - (skip_offset + 2)) as i16);
                self.emit_opcode(OpCode::PUSH_BOOL, 0);
                self.emit_u8(0, 0);

                // end:
                let end_ip = self.current_ip();
                self.patch_i16_at(end_offset, (end_ip - (end_offset + 2)) as i16);
                return Ok(());
            }
            ast::Operator::Or => {
                // Or: compile left, JMP_IF_TRUE(skip), compile right, JMP(end),
                //     (skip:) PUSH_BOOL(true), (end:)
                self.compile_expr(left)?;
                self.emit_opcode(OpCode::JMP_IF_TRUE, 0);
                let skip_offset = self.current_ip();
                self.emit_i16(0, 0); // placeholder

                self.compile_expr(right)?;
                self.emit_opcode(OpCode::JMP, 0);
                let end_offset = self.current_ip();
                self.emit_i16(0, 0); // placeholder

                // skip: PUSH_BOOL(true)
                let skip_ip = self.current_ip();
                self.patch_i16_at(skip_offset, (skip_ip - (skip_offset + 2)) as i16);
                self.emit_opcode(OpCode::PUSH_BOOL, 0);
                self.emit_u8(1, 0);

                // end:
                let end_ip = self.current_ip();
                self.patch_i16_at(end_offset, (end_ip - (end_offset + 2)) as i16);
                return Ok(());
            }
            _ => {}
        }

        // Non-short-circuit binary operators.
        self.compile_expr(left)?;
        self.compile_expr(right)?;

        let left_type = self.infer_expr_type(left);
        let right_type = self.infer_expr_type(right);
        let result_type = self.wider_type(left_type, right_type);

        match op {
            ast::Operator::Add => {
                if result_type == InferredType::String {
                    // Pick the right string concatenation opcode based on operand types.
                    if left_type == InferredType::String && right_type == InferredType::String {
                        self.emit_opcode(OpCode::STR_CONCAT, 0);
                    } else if left_type == InferredType::String {
                        // String + non-String
                        self.emit_opcode(OpCode::STR_CONCAT_RIGHT, 0);
                    } else if right_type == InferredType::String {
                        // non-String + String
                        self.emit_opcode(OpCode::STR_CONCAT_LEFT, 0);
                    } else {
                        // Both non-String but result is String (e.g., toString calls)
                        self.emit_opcode(OpCode::STR_CONCAT, 0);
                    }
                } else {
                    self.emit_add_opcode(result_type);
                }
            }
            ast::Operator::Sub => self.emit_sub_opcode(result_type),
            ast::Operator::Mul => self.emit_mul_opcode(result_type),
            ast::Operator::Div => self.emit_div_opcode(result_type),
            ast::Operator::Mod => self.emit_mod_opcode(result_type),
            ast::Operator::Eq => self.emit_eq_opcode(result_type),
            ast::Operator::Ne => self.emit_ne_opcode(result_type),
            ast::Operator::Lt => self.emit_lt_opcode(result_type),
            ast::Operator::Gt => self.emit_gt_opcode(result_type),
            ast::Operator::Le => self.emit_le_opcode(result_type),
            ast::Operator::Ge => self.emit_ge_opcode(result_type),
            ast::Operator::BitAnd => self.emit_bitand_opcode(result_type),
            ast::Operator::BitOr => self.emit_bitor_opcode(result_type),
            ast::Operator::BitXor => self.emit_bitxor_opcode(result_type),
            ast::Operator::BitShl => self.emit_shl_opcode(result_type),
            ast::Operator::BitShr => self.emit_shr_opcode(result_type),
            ast::Operator::And | ast::Operator::Or => {
                unreachable!("And/Or handled above")
            }
        }

        Ok(())
    }

    fn compile_unary(&mut self, op: &ast::UnOp, operand: &ast::Expr) -> Result<(), String> {
        self.compile_expr(operand)?;
        let ty = self.infer_expr_type(operand);
        match op {
            ast::UnOp::Neg => self.emit_neg_opcode(ty),
            ast::UnOp::Not => {
                self.emit_opcode(OpCode::NOT, 0);
            }
            ast::UnOp::BitNot => self.emit_bitnot_opcode(ty),
        }
        Ok(())
    }

    fn compile_call(&mut self, callee: &ast::Expr, args: &[ast::Expr]) -> Result<(), String> {
        // Special case: super(...) call in a constructor.
        if let ast::Expr::Super = callee {
            // Compile arguments so they're consumed, then emit POPs.
            // super() calls are handled by the VM during NEW if there's a
            // parent class. For now, just discard the arguments.
            for arg in args {
                self.compile_expr(arg)?;
            }
            for _ in args {
                self.emit_opcode(OpCode::POP, 0);
            }
            self.emit_opcode(OpCode::PUSH_VOID, 0);
            return Ok(());
        }

        // Special case: Identifier("Ok") → RESULT_OK
        if let ast::Expr::Identifier(name) = callee {
            if name == "Ok" {
                if args.len() != 1 {
                    return Err("Ok() expects exactly 1 argument".to_string());
                }
                self.compile_expr(&args[0])?;
                self.emit_opcode(OpCode::RESULT_OK, 0);
                return Ok(());
            }
            if name == "Err" {
                if args.len() != 1 {
                    return Err("Err() expects exactly 1 argument".to_string());
                }
                self.compile_expr(&args[0])?;
                self.emit_opcode(OpCode::RESULT_ERR, 0);
                return Ok(());
            }

            // Check if it's an enum variant constructor.
            if let Some((enum_name, _variant_idx)) = self.variant_map.get(name) {
                let enum_idx = *self.enum_map.get(enum_name).unwrap() as u16;
                let variant_name_idx = self.intern_string(name);
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::ENUM_NEW, 0);
                self.emit_u16(enum_idx, 0);
                self.emit_u16(variant_name_idx, 0);
                self.emit_u8(args.len() as u8, 0);
                return Ok(());
            }

            // Check if it's a known function.
            if let Some(&fn_idx) = self.function_map.get(name) {
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::CALL, 0);
                self.emit_u16(fn_idx, 0);
                self.emit_u8(args.len() as u8, 0);
                return Ok(());
            }
        }

        // Special case: MemberAccess callee → method call.
        if let ast::Expr::MemberAccess(ref obj, ref method) = *callee {
            // Check for static calls like io.println, Integer.toString, etc.
            if let ast::Expr::Identifier(ref obj_name) = **obj {
                if self.is_builtin_object(obj_name) {
                    self.compile_static_call(obj_name, method, args)?;
                    return Ok(());
                }
                // Check if obj_name is a class name.
                if self.class_map.contains_key(obj_name) {
                    self.compile_static_call(obj_name, method, args)?;
                    return Ok(());
                }
            }

            // Regular method call: compile obj, then args, then INVOKE_VIRTUAL.
            self.compile_expr(obj)?;
            for arg in args {
                self.compile_expr(arg)?;
            }
            let method_idx = self.intern_string(method);
            self.emit_opcode(OpCode::INVOKE_VIRTUAL, 0);
            self.emit_u16(method_idx, 0);
            self.emit_u8(args.len() as u8, 0);
            return Ok(());
        }

        // General case: compile callee, then args, then CALL.
        self.compile_expr(callee)?;
        for arg in args {
            self.compile_expr(arg)?;
        }
        self.emit_opcode(OpCode::CALL, 0);
        // Use function index 0 as placeholder; the VM will use the callee on the stack.
        self.emit_u16(0, 0);
        self.emit_u8(args.len() as u8, 0);

        Ok(())
    }

    fn compile_member_access(&mut self, obj: &ast::Expr, member: &str) -> Result<(), String> {
        // Check for static member access patterns.
        if let ast::Expr::Identifier(ref obj_name) = *obj {
            // io.println etc. are handled in compile_call via MemberAccess callee.
            // Here we handle bare member access (not a call).
            if self.is_builtin_object(obj_name) {
                // This is a reference to a builtin object's member.
                // It will typically be used in a call context, which is handled above.
                self.emit_opcode(OpCode::PUSH_NULL, 0);
                return Ok(());
            }
        }

        // Regular field access: compile obj, then GET_FIELD.
        self.compile_expr(obj)?;
        let field_idx = self.intern_string(member);
        self.emit_opcode(OpCode::GET_FIELD, 0);
        self.emit_u16(field_idx, 0);

        Ok(())
    }

    fn compile_new(&mut self, typ: &ast::Type, args: &[ast::Expr]) -> Result<(), String> {
        let class_name = typ.name();

        // Handle built-in types that aren't user-defined classes
        match class_name {
            "ArrayList" => {
                // Compile arguments (none expected, but handle gracefully)
                for arg in args {
                    self.compile_expr(arg)?;
                }
                // Emit ARRAY_NEW with size 0, then wrap as ArrayList ClassInstance
                // We use a special approach: emit PUSH_NULL as marker, then ARRAY_NEW(0)
                // Actually, let's just emit NEW with a special high class index
                // that the VM recognizes as built-in.
                // Simpler: register ArrayList and HashMap as pseudo-classes.
                let class_idx = self.get_or_create_builtin_class("ArrayList");
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::NEW, 0);
                self.emit_u16(class_idx, 0);
                return Ok(());
            }
            "HashMap" => {
                let class_idx = self.get_or_create_builtin_class("HashMap");
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::NEW, 0);
                self.emit_u16(class_idx, 0);
                return Ok(());
            }
            _ => {}
        }

        let class_idx = *self
            .class_map
            .get(class_name)
            .ok_or_else(|| format!("Unknown class '{}' in new expression", class_name))?;

        // Compile arguments.
        for arg in args {
            self.compile_expr(arg)?;
        }

        self.emit_opcode(OpCode::NEW, 0);
        self.emit_u16(class_idx, 0);

        // If the class has a constructor, the VM will call it after allocation.
        // The constructor call is implicit in the NEW opcode.

        Ok(())
    }

    /// Get or create a built-in pseudo-class (ArrayList, HashMap, etc.)
    fn get_or_create_builtin_class(&mut self, name: &str) -> u16 {
        if let Some(&idx) = self.class_map.get(name) {
            return idx;
        }
        let idx = self.classes.len() as u16;
        let class_def = ClassDef {
            name: name.to_string(),
            parent: None,
            fields: Vec::new(),
            methods: HashMap::new(),
            constructor: None,
            field_inits: Vec::new(),
        };
        self.classes.push(class_def);
        self.class_map.insert(name.to_string(), idx);
        idx
    }

    fn compile_static_call(
        &mut self,
        class_name: &str,
        method: &str,
        args: &[ast::Expr],
    ) -> Result<(), String> {
        // Compile arguments.
        for arg in args {
            self.compile_expr(arg)?;
        }

        let class_idx = self.intern_string(class_name);
        let method_idx = self.intern_string(method);

        self.emit_opcode(OpCode::STATIC_CALL, 0);
        self.emit_u16(class_idx, 0);
        self.emit_u16(method_idx, 0);
        self.emit_u8(args.len() as u8, 0);

        Ok(())
    }

    fn compile_assign(&mut self, target: &ast::Expr, value: &ast::Expr) -> Result<(), String> {
        self.compile_expr(value)?;

        match target {
            ast::Expr::Identifier(name) => {
                if let Some(slot) = self.resolve_local(name) {
                    self.emit_opcode(OpCode::STORE_LOCAL, 0);
                    self.emit_u8(slot, 0);
                } else {
                    return Err(format!("Cannot assign to undefined variable '{}'", name));
                }
            }
            ast::Expr::MemberAccess(obj, member) => {
                self.compile_expr(obj)?;
                let field_idx = self.intern_string(member);
                self.emit_opcode(OpCode::SET_FIELD, 0);
                self.emit_u16(field_idx, 0);
            }
            ast::Expr::Index(obj, index) => {
                self.compile_expr(obj)?;
                self.compile_expr(index)?;
                self.emit_opcode(OpCode::ARRAY_SET, 0);
            }
            _ => {
                return Err("Invalid assignment target".to_string());
            }
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Type inference helpers
    // -----------------------------------------------------------------------

    fn infer_expr_type(&self, expr: &ast::Expr) -> InferredType {
        match expr {
            ast::Expr::Literal(lit) => self.infer_literal_type(lit),
            ast::Expr::Identifier(name) => self.infer_identifier_type(name),
            ast::Expr::Binary(left, op, right) => {
                let lt = self.infer_expr_type(left);
                let rt = self.infer_expr_type(right);
                match op {
                    ast::Operator::Add
                    | ast::Operator::Sub
                    | ast::Operator::Mul
                    | ast::Operator::Div
                    | ast::Operator::Mod => {
                        // If either side is String, it's string concatenation
                        if lt == InferredType::String || rt == InferredType::String {
                            InferredType::String
                        } else {
                            self.wider_type(lt, rt)
                        }
                    }
                    ast::Operator::Eq
                    | ast::Operator::Ne
                    | ast::Operator::Lt
                    | ast::Operator::Gt
                    | ast::Operator::Le
                    | ast::Operator::Ge => InferredType::Bool,
                    ast::Operator::And | ast::Operator::Or => InferredType::Bool,
                    ast::Operator::BitAnd
                    | ast::Operator::BitOr
                    | ast::Operator::BitXor
                    | ast::Operator::BitShl
                    | ast::Operator::BitShr => self.wider_type(lt, rt),
                }
            }
            ast::Expr::Unary(op, operand) => {
                let ot = self.infer_expr_type(operand);
                match op {
                    ast::UnOp::Neg => ot,
                    ast::UnOp::Not => InferredType::Bool,
                    ast::UnOp::BitNot => ot,
                }
            }
            ast::Expr::Call(callee, _args) => {
                // Check for toString calls on builtin objects
                if let ast::Expr::MemberAccess(_, method) = callee.as_ref() {
                    if method == "toString" {
                        return InferredType::String;
                    }
                }
                if let ast::Expr::Identifier(name) = callee.as_ref() {
                    if name == "Ok" || name == "Err" {
                        return InferredType::Unknown; // Result type
                    }
                }
                InferredType::Unknown
            }
            ast::Expr::MemberAccess(_, _) => InferredType::Unknown,
            ast::Expr::Index(_, _) => InferredType::Unknown,
            ast::Expr::New(_, _) => InferredType::Unknown,
            ast::Expr::This => InferredType::Unknown,
            ast::Expr::Super => InferredType::Unknown,
            ast::Expr::OwnedDeref(inner) => self.infer_expr_type(inner),
            ast::Expr::RegionAlloc(_, _) => InferredType::Unknown,
            ast::Expr::RefExpr(_, _) => InferredType::Unknown,
            ast::Expr::UnsafeBlock(_) => InferredType::Unknown,
            ast::Expr::ErrorPropagation(_) => InferredType::Unknown,
            ast::Expr::Cast(_, target_type) => self.type_to_inferred(target_type),
            ast::Expr::StaticCall { method, .. } => {
                // toString always returns String
                if method == "toString" {
                    InferredType::String
                } else if method == "parseInt" {
                    InferredType::Unknown // Result type
                } else {
                    InferredType::Unknown
                }
            }
            ast::Expr::Assign(_, _) => InferredType::Unknown,
        }
    }

    fn infer_literal_type(&self, lit: &ast::Literal) -> InferredType {
        match lit {
            ast::Literal::Int(_) => InferredType::I64,
            ast::Literal::Float(_) => InferredType::F64,
            ast::Literal::Bool(_) => InferredType::Bool,
            ast::Literal::Char(_) => InferredType::Char,
            ast::Literal::String(_) => InferredType::String,
            ast::Literal::Null => InferredType::Null,
        }
    }

    fn infer_identifier_type(&self, name: &str) -> InferredType {
        // Check if it's a local variable with a known type.
        for local in self.locals.iter().rev() {
            if local.name == name {
                // We don't track types on locals yet, so default to Unknown.
                return InferredType::Unknown;
            }
        }
        InferredType::Unknown
    }

    fn wider_type(&self, a: InferredType, b: InferredType) -> InferredType {
        if a == b {
            return a;
        }
        // If either side is String, the result is String (concatenation).
        if a == InferredType::String || b == InferredType::String {
            return InferredType::String;
        }
        // Promote to the wider type.
        match (a, b) {
            (InferredType::F64, _) | (_, InferredType::F64) => InferredType::F64,
            (InferredType::F32, _) | (_, InferredType::F32) => InferredType::F32,
            (InferredType::I64, _) | (_, InferredType::I64) => InferredType::I64,
            (InferredType::I32, _) | (_, InferredType::I32) => InferredType::I32,
            (InferredType::I16, _) | (_, InferredType::I16) => InferredType::I16,
            _ => InferredType::I64, // default
        }
    }

    fn type_to_inferred(&self, typ: &ast::Type) -> InferredType {
        match typ.name() {
            "byte" => InferredType::I8,
            "short" => InferredType::I16,
            "int" => InferredType::I32,
            "long" => InferredType::I64,
            "vast" => InferredType::I128,
            "uvast" => InferredType::U128,
            "float" => InferredType::F32,
            "double" => InferredType::F64,
            "bool" => InferredType::Bool,
            "char" => InferredType::Char,
            "string" | "String" => InferredType::String,
            _ => InferredType::Unknown,
        }
    }

    fn type_to_cast_target(&self, typ: &ast::Type) -> CastTarget {
        match typ.name() {
            "byte" => CastTarget::Byte,
            "short" => CastTarget::Short,
            "int" => CastTarget::Int,
            "long" => CastTarget::Long,
            "vast" => CastTarget::Vast,
            "uvast" => CastTarget::Uvast,
            "float" => CastTarget::Float,
            "double" => CastTarget::Double,
            "half" => CastTarget::Half,
            "quad" => CastTarget::Quad,
            "char" => CastTarget::Char,
            "string" | "String" => CastTarget::String,
            "bool" => CastTarget::Bool,
            _ => CastTarget::Long, // safe default
        }
    }

    fn is_builtin_object(&self, name: &str) -> bool {
        matches!(
            name,
            "io"
                | "Integer"
                | "Double"
                | "Float"
                | "Long"
                | "Byte"
                | "Short"
                | "Half"
                | "Quad"
                | "Vast"
                | "Uvast"
                | "Boolean"
                | "Char"
                | "String_"
                | "ArrayList"
                | "HashMap"
                | "malloc"
                | "free"
        )
    }

    // -----------------------------------------------------------------------
    // Typed opcode emission helpers
    // -----------------------------------------------------------------------

    fn emit_add_opcode(&mut self, ty: InferredType) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::ADD_I32,
                InferredType::F32 => OpCode::ADD_F32,
                InferredType::F64 => OpCode::ADD_F64,
                _ => OpCode::ADD_I64, // default for I64, I128, U128, Unknown
            },
            0,
        );
    }

    fn emit_sub_opcode(&mut self, ty: InferredType) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::SUB_I32,
                InferredType::F32 => OpCode::SUB_F32,
                InferredType::F64 => OpCode::SUB_F64,
                _ => OpCode::SUB_I64,
            },
            0,
        );
    }

    fn emit_mul_opcode(&mut self, ty: InferredType) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::MUL_I32,
                InferredType::F32 => OpCode::MUL_F32,
                InferredType::F64 => OpCode::MUL_F64,
                _ => OpCode::MUL_I64,
            },
            0,
        );
    }

    fn emit_div_opcode(&mut self, ty: InferredType) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::DIV_I32,
                InferredType::F32 => OpCode::DIV_F32,
                InferredType::F64 => OpCode::DIV_F64,
                _ => OpCode::DIV_I64,
            },
            0,
        );
    }

    fn emit_mod_opcode(&mut self, ty: InferredType) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::MOD_I32,
                InferredType::F32 => OpCode::MOD_F32,
                InferredType::F64 => OpCode::MOD_F64,
                _ => OpCode::MOD_I64,
            },
            0,
        );
    }

    fn emit_neg_opcode(&mut self, ty: InferredType) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::NEG_I32,
                InferredType::F32 => OpCode::NEG_F32,
                InferredType::F64 => OpCode::NEG_F64,
                _ => OpCode::NEG_I64,
            },
            0,
        );
    }

    fn emit_bitand_opcode(&mut self, ty: InferredType) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::BITAND_I32,
                _ => OpCode::BITAND_I64,
            },
            0,
        );
    }

    fn emit_bitor_opcode(&mut self, ty: InferredType) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::BITOR_I32,
                _ => OpCode::BITOR_I64,
            },
            0,
        );
    }

    fn emit_bitxor_opcode(&mut self, ty: InferredType) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::BITXOR_I32,
                _ => OpCode::BITXOR_I64,
            },
            0,
        );
    }

    fn emit_shl_opcode(&mut self, ty: InferredType) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::SHL_I32,
                _ => OpCode::SHL_I64,
            },
            0,
        );
    }

    fn emit_shr_opcode(&mut self, ty: InferredType) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::SHR_I32,
                _ => OpCode::SHR_I64,
            },
            0,
        );
    }

    fn emit_bitnot_opcode(&mut self, ty: InferredType) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::BITNOT_I32,
                _ => OpCode::BITNOT_I64,
            },
            0,
        );
    }

    fn emit_eq_opcode(&mut self, ty: InferredType) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::EQ_I32,
                InferredType::F32 => OpCode::EQ_F32,
                InferredType::F64 => OpCode::EQ_F64,
                InferredType::Bool => OpCode::EQ_BOOL,
                InferredType::Char => OpCode::EQ_CHAR,
                InferredType::String => OpCode::EQ_STRING,
                _ => OpCode::EQ_I64,
            },
            0,
        );
    }

    fn emit_ne_opcode(&mut self, ty: InferredType) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::NE_I32,
                InferredType::F32 => OpCode::NE_F32,
                InferredType::F64 => OpCode::NE_F64,
                _ => OpCode::NE_I64,
            },
            0,
        );
    }

    fn emit_lt_opcode(&mut self, ty: InferredType) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::LT_I32,
                InferredType::F32 => OpCode::LT_F32,
                InferredType::F64 => OpCode::LT_F64,
                _ => OpCode::LT_I64,
            },
            0,
        );
    }

    fn emit_gt_opcode(&mut self, ty: InferredType) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::GT_I32,
                InferredType::F32 => OpCode::GT_F32,
                InferredType::F64 => OpCode::GT_F64,
                _ => OpCode::GT_I64,
            },
            0,
        );
    }

    fn emit_le_opcode(&mut self, ty: InferredType) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::LE_I32,
                InferredType::F32 => OpCode::LE_F32,
                InferredType::F64 => OpCode::LE_F64,
                _ => OpCode::LE_I64,
            },
            0,
        );
    }

    fn emit_ge_opcode(&mut self, ty: InferredType) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::GE_I32,
                InferredType::F32 => OpCode::GE_F32,
                InferredType::F64 => OpCode::GE_F64,
                _ => OpCode::GE_I64,
            },
            0,
        );
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;
    use crate::bytecode::opcodes::OpCode;

    fn compile_program(declarations: Vec<ast::Declaration>) -> CompiledProgram {
        let program = ast::Program {
            imports: vec![],
            declarations,
        };
        let mut compiler = Compiler::new();
        compiler.compile(&program).expect("compilation should succeed")
    }

    // -- test_compile_literal_int ------------------------------------------------

    #[test]
    fn test_compile_literal_int() {
        let compiled = compile_program(vec![ast::Declaration::VarDecl(ast::VarDecl {
            name: "x".to_string(),
            typ: None,
            init: Some(ast::Expr::Literal(ast::Literal::Int(42))),
            mutable: false,
        })]);

        let main_chunk = &compiled.functions[0].chunk;
        // Should contain PUSH_I64 opcode somewhere.
        assert!(
            main_chunk.code.contains(&(OpCode::PUSH_I64 as u8)),
            "main chunk should contain PUSH_I64 for integer literal"
        );
    }

    // -- test_compile_literal_string ---------------------------------------------

    #[test]
    fn test_compile_literal_string() {
        let compiled = compile_program(vec![ast::Declaration::VarDecl(ast::VarDecl {
            name: "s".to_string(),
            typ: None,
            init: Some(ast::Expr::Literal(ast::Literal::String("hello".to_string()))),
            mutable: false,
        })]);

        let main_chunk = &compiled.functions[0].chunk;
        assert!(
            main_chunk.code.contains(&(OpCode::PUSH_STRING as u8)),
            "main chunk should contain PUSH_STRING for string literal"
        );
        assert!(
            main_chunk.strings.contains(&"hello".to_string()),
            "string table should contain 'hello'"
        );
    }

    // -- test_compile_binary_add -------------------------------------------------

    #[test]
    fn test_compile_binary_add() {
        let compiled = compile_program(vec![ast::Declaration::VarDecl(ast::VarDecl {
            name: "x".to_string(),
            typ: None,
            init: Some(ast::Expr::Binary(
                Box::new(ast::Expr::Literal(ast::Literal::Int(1))),
                ast::Operator::Add,
                Box::new(ast::Expr::Literal(ast::Literal::Int(2))),
            )),
            mutable: false,
        })]);

        let main_chunk = &compiled.functions[0].chunk;
        assert!(
            main_chunk.code.contains(&(OpCode::ADD_I64 as u8)),
            "main chunk should contain ADD_I64 for integer addition"
        );
    }

    // -- test_compile_var_decl_and_load ------------------------------------------

    #[test]
    fn test_compile_var_decl_and_load() {
        let compiled = compile_program(vec![
            ast::Declaration::VarDecl(ast::VarDecl {
                name: "x".to_string(),
                typ: None,
                init: Some(ast::Expr::Literal(ast::Literal::Int(10))),
                mutable: false,
            }),
            ast::Declaration::VarDecl(ast::VarDecl {
                name: "y".to_string(),
                typ: None,
                init: Some(ast::Expr::Identifier("x".to_string())),
                mutable: false,
            }),
        ]);

        let main_chunk = &compiled.functions[0].chunk;
        assert!(
            main_chunk.code.contains(&(OpCode::STORE_LOCAL as u8)),
            "main chunk should contain STORE_LOCAL"
        );
        assert!(
            main_chunk.code.contains(&(OpCode::LOAD_LOCAL as u8)),
            "main chunk should contain LOAD_LOCAL"
        );
    }

    // -- test_compile_if_else ----------------------------------------------------

    #[test]
    fn test_compile_if_else() {
        let _compiled = compile_program(vec![ast::Declaration::VarDecl(ast::VarDecl {
            name: "x".to_string(),
            typ: None,
            init: Some(ast::Expr::Literal(ast::Literal::Int(0))),
            mutable: false,
        })]);

        // Build an if-else as a statement in the main chunk.
        let _program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::VarDecl(ast::VarDecl {
                name: "x".to_string(),
                typ: None,
                init: None,
                mutable: true,
            })],
        };

        // We need to compile an if statement. Let's build it manually.
        let if_stmt = ast::Stmt::If(ast::IfStmt {
            condition: ast::Expr::Literal(ast::Literal::Bool(true)),
            then_branch: vec![ast::Stmt::Expr(ast::Expr::Literal(ast::Literal::Int(1)))],
            else_branch: Some(vec![ast::Stmt::Expr(ast::Expr::Literal(
                ast::Literal::Int(2),
            ))]),
        });

        let _program = ast::Program {
            imports: vec![],
            declarations: vec![],
        };

        // Since we can't easily add statements to the main chunk through
        // the public API (only declarations), let's test through the
        // compile_stmt method indirectly by using a function.
        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_if".to_string(),
            params: vec![],
            return_type: None,
            body: vec![if_stmt],
            sugar: false,
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::Function(fn_decl)],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        let fn_chunk = &compiled.functions[1].chunk;
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP_IF_FALSE as u8)),
            "if statement should emit JMP_IF_FALSE"
        );
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP as u8)),
            "if-else should emit JMP to skip else branch"
        );
    }

    // -- test_compile_while_loop -------------------------------------------------

    #[test]
    fn test_compile_while_loop() {
        let while_stmt = ast::Stmt::While(ast::WhileStmt {
            condition: ast::Expr::Literal(ast::Literal::Bool(true)),
            body: vec![ast::Stmt::Break],
        });

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_while".to_string(),
            params: vec![],
            return_type: None,
            body: vec![while_stmt],
            sugar: false,
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::Function(fn_decl)],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        let fn_chunk = &compiled.functions[1].chunk;
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP_IF_FALSE as u8)),
            "while loop should emit JMP_IF_FALSE"
        );
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP as u8)),
            "while loop should emit JMP back to start"
        );
    }

    // -- test_compile_function_call ----------------------------------------------

    #[test]
    fn test_compile_function_call() {
        let callee_fn = ast::FnDecl {
            access: ast::Access::Public,
            name: "add".to_string(),
            params: vec![
                ast::Param {
                    name: "a".to_string(),
                    typ: ast::Type::simple("long"),
                },
                ast::Param {
                    name: "b".to_string(),
                    typ: ast::Type::simple("long"),
                },
            ],
            return_type: Some(ast::Type::simple("long")),
            body: vec![ast::Stmt::Return(Some(ast::Expr::Binary(
                Box::new(ast::Expr::Identifier("a".to_string())),
                ast::Operator::Add,
                Box::new(ast::Expr::Identifier("b".to_string())),
            )))],
            sugar: false,
        };

        let caller_fn = ast::FnDecl {
            access: ast::Access::Public,
            name: "main_fn".to_string(),
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::Expr(ast::Expr::Call(
                Box::new(ast::Expr::Identifier("add".to_string())),
                vec![
                    ast::Expr::Literal(ast::Literal::Int(1)),
                    ast::Expr::Literal(ast::Literal::Int(2)),
                ],
            ))],
            sugar: false,
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![
                ast::Declaration::Function(callee_fn),
                ast::Declaration::Function(caller_fn),
            ],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        let caller_chunk = &compiled.functions[2].chunk;
        assert!(
            caller_chunk.code.contains(&(OpCode::CALL as u8)),
            "function call should emit CALL"
        );
    }

    // -- test_compile_class_new --------------------------------------------------

    #[test]
    fn test_compile_class_new() {
        let class_decl = ast::ClassDecl {
            name: "Point".to_string(),
            parent: None,
            ifaces: vec![],
            members: vec![
                ast::ClassMember::Field(ast::FieldDecl {
                    access: ast::Access::Public,
                    name: "x".to_string(),
                    typ: ast::Type::simple("long"),
                    init: Some(ast::Expr::Literal(ast::Literal::Int(0))),
                }),
                ast::ClassMember::Field(ast::FieldDecl {
                    access: ast::Access::Public,
                    name: "y".to_string(),
                    typ: ast::Type::simple("long"),
                    init: Some(ast::Expr::Literal(ast::Literal::Int(0))),
                }),
            ],
        };

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "make_point".to_string(),
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::Expr(ast::Expr::New(
                ast::Type::simple("Point"),
                vec![],
            ))],
            sugar: false,
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![
                ast::Declaration::Class(class_decl),
                ast::Declaration::Function(fn_decl),
            ],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        assert_eq!(compiled.classes.len(), 1);
        assert_eq!(compiled.classes[0].name, "Point");

        let fn_chunk = &compiled.functions[1].chunk;
        assert!(
            fn_chunk.code.contains(&(OpCode::NEW as u8)),
            "new expression should emit NEW"
        );
    }

    // -- test_compile_enum -------------------------------------------------------

    #[test]
    fn test_compile_enum() {
        let enum_decl = ast::EnumDecl {
            name: "Shape".to_string(),
            variants: vec![
                ast::Variant {
                    name: "SCircle".to_string(),
                    fields: vec![ast::Param {
                        name: "radius".to_string(),
                        typ: ast::Type::simple("double"),
                    }],
                },
                ast::Variant {
                    name: "SRect".to_string(),
                    fields: vec![
                        ast::Param {
                            name: "w".to_string(),
                            typ: ast::Type::simple("double"),
                        },
                        ast::Param {
                            name: "h".to_string(),
                            typ: ast::Type::simple("double"),
                        },
                    ],
                },
            ],
        };

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "make_circle".to_string(),
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::VarDecl(ast::VarDecl {
                name: "c".to_string(),
                typ: None,
                init: Some(ast::Expr::Call(
                    Box::new(ast::Expr::Identifier("SCircle".to_string())),
                    vec![ast::Expr::Literal(ast::Literal::Float(3.0))],
                )),
                mutable: false,
            })],
            sugar: false,
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![
                ast::Declaration::Enum(enum_decl),
                ast::Declaration::Function(fn_decl),
            ],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        assert_eq!(compiled.enums.len(), 1);
        assert_eq!(compiled.enums[0].name, "Shape");
        assert_eq!(compiled.enums[0].variants.len(), 2);
        assert_eq!(compiled.enums[0].variants[0].name, "SCircle");
        assert_eq!(compiled.enums[0].variants[0].field_count, 1);
        assert_eq!(compiled.enums[0].variants[1].name, "SRect");
        assert_eq!(compiled.enums[0].variants[1].field_count, 2);

        let fn_chunk = &compiled.functions[1].chunk;
        assert!(
            fn_chunk.code.contains(&(OpCode::ENUM_NEW as u8)),
            "enum variant constructor should emit ENUM_NEW"
        );
    }

    // -- test_compile_switch -----------------------------------------------------

    #[test]
    fn test_compile_switch() {
        let switch_stmt = ast::Stmt::Switch(ast::SwitchStmt {
            expr: ast::Expr::Identifier("x".to_string()),
            cases: vec![
                ast::Case {
                    pattern: ast::Pattern::Literal(ast::Literal::Int(1)),
                    body: vec![ast::Stmt::Expr(ast::Expr::Literal(ast::Literal::Int(
                        10,
                    )))],
                },
                ast::Case {
                    pattern: ast::Pattern::Literal(ast::Literal::Int(2)),
                    body: vec![ast::Stmt::Expr(ast::Expr::Literal(ast::Literal::Int(
                        20,
                    )))],
                },
            ],
            default: Some(vec![ast::Stmt::Expr(ast::Expr::Literal(
                ast::Literal::Int(0),
            ))]),
        });

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_switch".to_string(),
            params: vec![ast::Param {
                name: "x".to_string(),
                typ: ast::Type::simple("long"),
            }],
            return_type: None,
            body: vec![switch_stmt],
            sugar: false,
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::Function(fn_decl)],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        let fn_chunk = &compiled.functions[1].chunk;
        assert!(
            fn_chunk.code.contains(&(OpCode::DUP as u8)),
            "switch should DUP the subject for each case"
        );
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP_IF_FALSE as u8)),
            "switch cases should use JMP_IF_FALSE"
        );
    }
}
