// Titrate Alpha 0.2 – bytecode compiler
// Lowers the AST to bytecode chunks for the VM.
// Precision in every step – richie-rich90454, 2026

mod chunk;
mod expr;
mod generics;
mod inference;
mod optimization;
mod resolver;
mod stmt;
mod symbols;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::ast;
use super::frame::{ClassDef, EnumDef, FieldDef, FunctionDef, VariantDef};
use super::chunk::Chunk;
use super::opcodes::OpCode;

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
    /// Number of module-level global variable slots.
    pub global_count: usize,
}

// ---------------------------------------------------------------------------
// Module – represents a single source file in the module graph
// ---------------------------------------------------------------------------

pub(super) struct Module {
    /// Dotted module name, e.g. "tt.lang.Integer"
    pub name: String,
    /// Absolute path to the source file.
    #[allow(dead_code)]
    pub file_path: PathBuf,
    /// Parsed AST; `None` if not yet loaded.
    pub program: Option<ast::Program>,
    /// Whether this module has been compiled into bytecode.
    pub compiled: bool,
}

// ---------------------------------------------------------------------------
// Symbol – an imported declaration tracked in the symbol table
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(super) enum Symbol {
    Function(u16),
    Class(u16),
    Enum(u16),
}

// ---------------------------------------------------------------------------
// Inferred type – used to pick the right typed opcode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InferredType {
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
    Class,
    Unknown,
}

// ---------------------------------------------------------------------------
// Local variable
// ---------------------------------------------------------------------------

pub(super) struct Local {
    pub name: String,
    pub depth: usize,
    #[allow(dead_code)]
    pub is_captured: bool,
    /// The stack slot assigned to this local.
    pub slot: u8,
}

// ---------------------------------------------------------------------------
// Loop bookkeeping for break/continue
// ---------------------------------------------------------------------------

pub(super) struct LoopInfo {
    /// IP that `continue` jumps to. For `while`/`for` this is the condition
    /// check; for `do-while` it is the condition check *after* the body.
    pub continue_ip: usize,
    /// Locations to patch with the end-of-loop offset (for `break`).
    pub break_patches: Vec<(usize, u32)>,
}

// ---------------------------------------------------------------------------
// A decoded instruction used during optimization passes
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(super) struct DecodedInstr {
    pub opcode: OpCode,
    pub operands: Vec<u8>,
    pub line: u32,
    /// For jump instructions: the instruction index of the target.
    /// Set during decode when jump targets are resolved.
    pub jump_target_idx: Option<usize>,
}

// ---------------------------------------------------------------------------
// Compiler
// ---------------------------------------------------------------------------

pub struct Compiler {
    /// All compiled functions. Index 0 is reserved for the top-level "main" chunk.
    pub(super) functions: Vec<FunctionDef>,
    /// All compiled classes.
    pub(super) classes: Vec<ClassDef>,
    /// All compiled enums.
    pub(super) enums: Vec<EnumDef>,
    /// Local variable slots for the current scope.
    pub(super) locals: Vec<Local>,
    /// Scope depth (0 = top-level, 1 = inside function, etc.).
    pub(super) scope_depth: usize,
    /// Current function index being compiled.
    pub(super) current_function: usize,
    /// Current class index being compiled (for super() resolution).
    pub(super) current_class: Option<u16>,
    /// Function name → index mapping.
    pub(super) function_map: HashMap<String, u16>,
    /// Class name → index mapping.
    pub(super) class_map: HashMap<String, u16>,
    /// Enum name → index mapping.
    pub(super) enum_map: HashMap<String, u16>,
    /// Loop stack for break/continue.
    pub(super) loop_stack: Vec<LoopInfo>,
    /// Number of local slots used in current function.
    pub(super) local_count: usize,
    /// Mapping from enum variant name → (enum_name, variant_index).
    pub(super) variant_map: HashMap<String, (String, usize)>,
    /// Collected native function names.
    pub(super) native_names: Vec<String>,
    /// Mapping from native name → index.
    #[allow(dead_code)]
    pub(super) native_map: HashMap<String, u16>,
    /// Monomorphization cache: mangled name → index.
    pub(super) mono_cache: HashMap<String, u16>,
    /// Generic function declarations (not registered in function_map).
    pub(super) generic_functions: Vec<ast::FnDecl>,
    /// Generic class declarations (not registered in class_map).
    pub(super) generic_classes: Vec<ast::ClassDecl>,
    /// Map from generic function base name → index in generic_functions.
    pub(super) generic_function_map: HashMap<String, usize>,
    /// Map from generic class base name → index in generic_classes.
    pub(super) generic_class_map: HashMap<String, usize>,
    /// Loaded modules (indexed by position).
    pub(super) modules: Vec<Module>,
    /// Map from dotted module name → index in modules.
    pub(super) module_map: HashMap<String, usize>,
    /// Symbol table for the current module: name → Symbol.
    pub(super) symbol_table: HashMap<String, Symbol>,
    /// Module resolver for resolving import paths to file paths.
    pub(super) resolver: resolver::ModuleResolver,
    /// Set of module names currently being processed (for circular import detection).
    #[allow(dead_code)]
    pub(super) processing: HashSet<String>,
    /// Counter for generating unique closure names.
    pub(super) closure_counter: usize,
    /// Module-level global variable names (indexed by slot).
    pub(super) globals: Vec<String>,
    /// Map from global variable name → index in globals.
    pub(super) global_map: HashMap<String, u16>,
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
            current_class: None,
            function_map: HashMap::new(),
            class_map: HashMap::new(),
            enum_map: HashMap::new(),
            loop_stack: Vec::new(),
            local_count: 0,
            variant_map: HashMap::new(),
            native_names: Vec::new(),
            native_map: HashMap::new(),
            mono_cache: HashMap::new(),
            generic_functions: Vec::new(),
            generic_classes: Vec::new(),
            generic_function_map: HashMap::new(),
            generic_class_map: HashMap::new(),
            modules: Vec::new(),
            module_map: HashMap::new(),
            symbol_table: HashMap::new(),
            resolver: resolver::ModuleResolver::new(),
            processing: HashSet::new(),
            closure_counter: 0,
            globals: Vec::new(),
            global_map: HashMap::new(),
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
                ast::Declaration::VarDecl(var_decl) => {
                    if !self.global_map.contains_key(&var_decl.name) {
                        let idx = self.globals.len() as u16;
                        self.globals.push(var_decl.name.clone());
                        self.global_map.insert(var_decl.name.clone(), idx);
                    }
                }
                ast::Declaration::ConstDecl(const_decl) => {
                    if !self.global_map.contains_key(&const_decl.name) {
                        let idx = self.globals.len() as u16;
                        self.globals.push(const_decl.name.clone());
                        self.global_map.insert(const_decl.name.clone(), idx);
                    }
                }
                _ => {}
            }
        }

        // Second pass: compile all function bodies and class methods.
        for decl in &program.declarations {
            match decl {
                ast::Declaration::VarDecl(var_decl) => {
                    self.compile_var_decl(var_decl)?;
                }
                ast::Declaration::ConstDecl(const_decl) => {
                    self.compile_var_decl(const_decl)?;
                }
                ast::Declaration::Function(fn_decl) => {
                    // Skip generic functions; they are instantiated on demand.
                    if fn_decl.type_params.is_empty() {
                        self.compile_function(fn_decl)?;
                    }
                }
                ast::Declaration::Class(class_decl) => {
                    // Skip generic classes; they are instantiated on demand.
                    if class_decl.type_params.is_empty() {
                        self.compile_class_methods(class_decl)?;
                    }
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

        // Run optimization passes.
        self.fold_constants();
        self.eliminate_dead_code();

        Ok(CompiledProgram {
            functions: std::mem::take(&mut self.functions),
            classes: std::mem::take(&mut self.classes),
            enums: std::mem::take(&mut self.enums),
            native_names: std::mem::take(&mut self.native_names),
            global_count: self.globals.len(),
        })
    }

    // -----------------------------------------------------------------------
    // Module system
    // -----------------------------------------------------------------------

    /// Resolve an import path to a file path.
    pub fn resolve_module(&mut self, import_path: &[String], root_dir: &Path) -> Result<PathBuf, String> {
        // Check if the last segment is "*" for glob imports.
        if import_path.last().map(|s| s.as_str()) == Some("*") {
            // Glob import: resolve to the directory and return a sentinel path.
            let dir_path = &import_path[..import_path.len() - 1];
            let files = self.resolver.resolve_glob(dir_path, root_dir)?;
            // Return the first file as a representative; the caller should use
            // resolve_glob directly for glob imports.
            files.into_iter().next().ok_or_else(|| "No files found for glob import".to_string())
        } else {
            self.resolver.resolve(import_path, root_dir)
        }
    }

    /// Load a module from the given file path, parse it, and add it to the
    /// module list. The `dotted_name` is the module's fully-qualified name
    /// (e.g., "circular_test.A"). Returns the module index.
    pub fn load_module(&mut self, path: &Path, dotted_name: &str) -> Result<usize, String> {
        // Check if already loaded.
        if let Some(&idx) = self.module_map.get(dotted_name) {
            return Ok(idx);
        }

        // Read and parse the source file.
        let source = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read module '{}': {}", path.display(), e))?;

        let tokens = crate::lexer::tokenize(&source)
            .map_err(|e| format!("Lexer error in module '{}': {}", path.display(), e))?;

        let program = crate::parser::parse(tokens)
            .map_err(|e| format!("Parser error in module '{}': {}", path.display(), e))?;

        let idx = self.modules.len();
        self.module_map.insert(dotted_name.to_string(), idx);
        self.modules.push(Module {
            name: dotted_name.to_string(),
            file_path: path.to_path_buf(),
            program: Some(program),
            compiled: false,
        });

        Ok(idx)
    }

    /// Compile a program with its module dependencies.
    /// This is the entry point for multi-file compilation.
    pub fn compile_with_modules(&mut self, program: &ast::Program, root_dir: &Path) -> Result<CompiledProgram, String> {
        // Step 1: Process imports from the root program and load all modules.
        self.process_imports(program, root_dir)?;

        // Step 2: Recursively process imports from all loaded modules.
        // We iterate until no new modules are discovered.
        let mut processed = 0;
        loop {
            let current_len = self.modules.len();
            if processed >= current_len {
                break;
            }
            // Clone programs to avoid borrow conflicts.
            let programs: Vec<Option<ast::Program>> = self.modules[processed..current_len]
                .iter()
                .map(|m| m.program.clone())
                .collect();
            for prog in programs.into_iter().flatten() {
                self.process_imports(&prog, root_dir)?;
            }
            processed = current_len;
        }

        // Step 3: Compute dependency order (topological sort).
        let order = self.topological_sort()?;

        // Step 4a: First pass – register ALL module declarations with mangled names.
        // This ensures every module's public symbols are available before any
        // module's imports are resolved or code is compiled.  This is essential
        // for handling circular imports (e.g. ArrayList ↔ ArrayListIterator ↔ Iterator).
        for &module_idx in &order {
            let (prog_opt, module_name) = {
                let m = &self.modules[module_idx];
                (m.program.clone(), m.name.clone())
            };
            if let Some(ref prog) = prog_opt {
                self.register_module_declarations(prog, &module_name)?;
            }
        }

        // Step 4b: Second pass – register imported symbols and compile each module.
        for &module_idx in &order {
            if self.modules[module_idx].compiled {
                continue;
            }
            let (prog_opt, module_name) = {
                let m = &self.modules[module_idx];
                (m.program.clone(), m.name.clone())
            };
            if let Some(ref prog) = prog_opt {
                // Register imported symbols so the module's code can reference
                // types and functions from its dependencies (e.g. Pair, ArrayList).
                self.register_imported_symbols(prog)?;
                // Compile the module's declarations.
                self.compile_module_program(prog, &module_name)?;
            }
            self.modules[module_idx].compiled = true;
        }

        // Step 5: Now compile the root program.
        // First pass: register all classes, enums, and functions.
        for decl in &program.declarations {
            match decl {
                ast::Declaration::Class(class_decl) => self.register_class(class_decl)?,
                ast::Declaration::Enum(enum_decl) => self.register_enum(enum_decl),
                ast::Declaration::Function(fn_decl) => {
                    self.register_function(fn_decl);
                }
                ast::Declaration::VarDecl(var_decl) => {
                    if !self.global_map.contains_key(&var_decl.name) {
                        let idx = self.globals.len() as u16;
                        self.globals.push(var_decl.name.clone());
                        self.global_map.insert(var_decl.name.clone(), idx);
                    }
                }
                ast::Declaration::ConstDecl(const_decl) => {
                    if !self.global_map.contains_key(&const_decl.name) {
                        let idx = self.globals.len() as u16;
                        self.globals.push(const_decl.name.clone());
                        self.global_map.insert(const_decl.name.clone(), idx);
                    }
                }
                _ => {}
            }
        }

        // Register imported symbols into the symbol table.
        self.register_imported_symbols(program)?;

        // Second pass: compile all function bodies and class methods.
        for decl in &program.declarations {
            match decl {
                ast::Declaration::VarDecl(var_decl) => {
                    self.compile_var_decl(var_decl)?;
                }
                ast::Declaration::ConstDecl(const_decl) => {
                    self.compile_var_decl(const_decl)?;
                }
                ast::Declaration::Function(fn_decl) => {
                    if fn_decl.type_params.is_empty() {
                        self.compile_function(fn_decl)?;
                    }
                }
                ast::Declaration::Class(class_decl) => {
                    if class_decl.type_params.is_empty() {
                        self.compile_class_methods(class_decl)?;
                    }
                }
                ast::Declaration::Enum(_) => {}
                ast::Declaration::Interface(_) => {}
            }
        }

        // If there's a `main` function, emit a CALL to it from the top-level chunk.
        self.current_function = 0;
        if let Some(&main_idx) = self.function_map.get("main") {
            self.emit_opcode(OpCode::CALL, 0);
            self.emit_u16(main_idx, 0);
            self.emit_u8(0, 0);
        }

        self.emit_opcode(OpCode::RET, 0);
        self.functions[0].local_count = self.local_count;

        // Run optimization passes.
        self.fold_constants();
        self.eliminate_dead_code();

        Ok(CompiledProgram {
            functions: std::mem::take(&mut self.functions),
            classes: std::mem::take(&mut self.classes),
            enums: std::mem::take(&mut self.enums),
            native_names: std::mem::take(&mut self.native_names),
            global_count: self.globals.len(),
        })
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

    fn su() -> ast::Span {
        ast::Span::unknown()
    }

    // -- test_compile_literal_int ------------------------------------------------

    #[test]
    fn test_compile_literal_int() {
        let compiled = compile_program(vec![ast::Declaration::VarDecl(ast::VarDecl {
            name: "x".to_string(),
            typ: None,
            init: Some(ast::Expr::Literal(ast::Literal::Int(42), su())),
            mutable: false,
            span: su(),
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
            init: Some(ast::Expr::Literal(ast::Literal::String("hello".to_string()), su())),
            mutable: false,
            span: su(),
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
        // Use variables so constant folding cannot eliminate the ADD opcode
        let compiled = compile_program(vec![
            ast::Declaration::VarDecl(ast::VarDecl {
                name: "a".to_string(),
                typ: None,
                init: Some(ast::Expr::Literal(ast::Literal::Int(1), su())),
                mutable: false,
                span: su(),
            }),
            ast::Declaration::VarDecl(ast::VarDecl {
                name: "b".to_string(),
                typ: None,
                init: Some(ast::Expr::Literal(ast::Literal::Int(2), su())),
                mutable: false,
                span: su(),
            }),
            ast::Declaration::VarDecl(ast::VarDecl {
                name: "x".to_string(),
                typ: None,
                init: Some(ast::Expr::Binary(
                    Box::new(ast::Expr::Identifier("a".to_string(), su())),
                    ast::Operator::Add,
                    Box::new(ast::Expr::Identifier("b".to_string(), su())),
                    su(),
                )),
                mutable: false,
                span: su(),
            }),
        ]);

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
                init: Some(ast::Expr::Literal(ast::Literal::Int(10), su())),
                mutable: false,
                span: su(),
            }),
            ast::Declaration::VarDecl(ast::VarDecl {
                name: "y".to_string(),
                typ: None,
                init: Some(ast::Expr::Identifier("x".to_string(), su())),
                mutable: false,
                span: su(),
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
            init: Some(ast::Expr::Literal(ast::Literal::Int(0), su())),
            mutable: false,
            span: su(),
        })]);

        // Build an if-else as a statement in the main chunk.
        let _program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::VarDecl(ast::VarDecl {
                name: "x".to_string(),
                typ: None,
                init: None,
                mutable: true,
                span: su(),
            })],
        };

        // We need to compile an if statement. Let's build it manually.
        let if_stmt = ast::Stmt::If(ast::IfStmt {
            condition: ast::Expr::Literal(ast::Literal::Bool(true), su()),
            then_branch: vec![ast::Stmt::Expr(ast::Expr::Literal(ast::Literal::Int(1), su()))],
            else_branch: Some(vec![ast::Stmt::Expr(ast::Expr::Literal(
                ast::Literal::Int(2),
                su(),
            ))]),
            span: su(),
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
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![if_stmt],
            sugar: false,
            where_clause: vec![],
            span: su(),
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
            condition: ast::Expr::Literal(ast::Literal::Bool(true), su()),
            body: vec![ast::Stmt::Break],
            span: su(),
        });

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_while".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![while_stmt],
            sugar: false,
            where_clause: vec![],
            span: su(),
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

    // -- test_compile_do_while_loop ----------------------------------------------

    #[test]
    fn test_compile_do_while_loop() {
        let do_while_stmt = ast::Stmt::DoWhile(ast::DoWhileStmt {
            body: vec![ast::Stmt::Expr(ast::Expr::Literal(ast::Literal::Int(1), su()))],
            condition: ast::Expr::Literal(ast::Literal::Bool(true), su()),
            span: su(),
        });

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_do_while".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![do_while_stmt],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::Function(fn_decl)],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        let fn_chunk = &compiled.functions[1].chunk;
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP_IF_TRUE as u8)),
            "do-while loop should emit JMP_IF_TRUE"
        );
    }

    // -- test_compile_function_call ----------------------------------------------

    #[test]
    fn test_compile_function_call() {
        let callee_fn = ast::FnDecl {
            access: ast::Access::Public,
            name: "add".to_string(),
            type_params: vec![],
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
                Box::new(ast::Expr::Identifier("a".to_string(), su())),
                ast::Operator::Add,
                Box::new(ast::Expr::Identifier("b".to_string(), su())),
                su(),
            )))],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let caller_fn = ast::FnDecl {
            access: ast::Access::Public,
            name: "main_fn".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::Expr(ast::Expr::Call(
                Box::new(ast::Expr::Identifier("add".to_string(), su())),
                vec![
                    ast::Expr::Literal(ast::Literal::Int(1), su()),
                    ast::Expr::Literal(ast::Literal::Int(2), su()),
                ],
                su(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: su(),
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
            type_params: vec![],
            parent: None,
            ifaces: vec![],
            members: vec![
                ast::ClassMember::Field(ast::FieldDecl {
                    access: ast::Access::Public,
                    name: "x".to_string(),
                    typ: ast::Type::simple("long"),
                    init: Some(ast::Expr::Literal(ast::Literal::Int(0), su())),
                    span: su(),
                }),
                ast::ClassMember::Field(ast::FieldDecl {
                    access: ast::Access::Public,
                    name: "y".to_string(),
                    typ: ast::Type::simple("long"),
                    init: Some(ast::Expr::Literal(ast::Literal::Int(0), su())),
                    span: su(),
                }),
            ],
            span: su(),
        };

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "make_point".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::Expr(ast::Expr::New(
                ast::Type::simple("Point"),
                vec![],
                su(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: su(),
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
            type_params: vec![],
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
            span: su(),
        };

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "make_circle".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::VarDecl(ast::VarDecl {
                name: "c".to_string(),
                typ: None,
                init: Some(ast::Expr::Call(
                    Box::new(ast::Expr::Identifier("SCircle".to_string(), su())),
                    vec![ast::Expr::Literal(ast::Literal::Float(3.0), su())],
                    su(),
                )),
                mutable: false,
                span: su(),
            })],
            sugar: false,
            where_clause: vec![],
            span: su(),
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
            expr: ast::Expr::Identifier("x".to_string(), su()),
            cases: vec![
                ast::Case {
                    pattern: ast::Pattern::Literal(ast::Literal::Int(1)),
                    body: vec![ast::Stmt::Expr(ast::Expr::Literal(ast::Literal::Int(
                        10,
                    ), su()))],
                },
                ast::Case {
                    pattern: ast::Pattern::Literal(ast::Literal::Int(2)),
                    body: vec![ast::Stmt::Expr(ast::Expr::Literal(ast::Literal::Int(
                        20,
                    ), su()))],
                },
            ],
            default: Some(vec![ast::Stmt::Expr(ast::Expr::Literal(
                ast::Literal::Int(0),
                su(),
            ))]),
            span: su(),
        });

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_switch".to_string(),
            type_params: vec![],
            params: vec![ast::Param {
                name: "x".to_string(),
                typ: ast::Type::simple("long"),
            }],
            return_type: None,
            body: vec![switch_stmt],
            sugar: false,
            where_clause: vec![],
            span: su(),
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

    // -- test_mangle_name --------------------------------------------------------

    #[test]
    fn test_mangle_name() {
        assert_eq!(Compiler::mangle_name("Box", &[]), "Box");
        assert_eq!(
            Compiler::mangle_name("Box", &[ast::Type::simple("int")]),
            "Box__int"
        );
        assert_eq!(
            Compiler::mangle_name("HashMap", &[ast::Type::simple("string"), ast::Type::simple("int")]),
            "HashMap__string__int"
        );
        assert_eq!(
            Compiler::mangle_name("Box", &[ast::Type::generic("Owned", vec![ast::Type::simple("int")])]),
            "Box__Owned__int"
        );
    }

    // -- test_substitute_type ----------------------------------------------------

    #[test]
    fn test_substitute_type() {
        let mut type_args = HashMap::new();
        type_args.insert("T".to_string(), ast::Type::simple("int"));

        // Simple type parameter substitution: T → int
        let ty = ast::Type::simple("T");
        let result = Compiler::substitute_type(&ty, &type_args);
        assert_eq!(result, ast::Type::simple("int"));

        // Non-parameterized type is unchanged: string → string
        let ty = ast::Type::simple("string");
        let result = Compiler::substitute_type(&ty, &type_args);
        assert_eq!(result, ast::Type::simple("string"));

        // Nested type parameter substitution: Owned<T> → Owned<int>
        let ty = ast::Type::generic("Owned", vec![ast::Type::simple("T")]);
        let result = Compiler::substitute_type(&ty, &type_args);
        assert_eq!(result, ast::Type::generic("Owned", vec![ast::Type::simple("int")]));

        // Multiple type parameters
        let mut type_args2 = HashMap::new();
        type_args2.insert("K".to_string(), ast::Type::simple("string"));
        type_args2.insert("V".to_string(), ast::Type::simple("int"));

        let ty = ast::Type::generic("Map", vec![ast::Type::simple("K"), ast::Type::simple("V")]);
        let result = Compiler::substitute_type(&ty, &type_args2);
        assert_eq!(result, ast::Type::generic("Map", vec![ast::Type::simple("string"), ast::Type::simple("int")]));
    }

    // -- test_instantiate_generic_function ----------------------------------------

    #[test]
    fn test_instantiate_generic_function() {
        let generic_fn = ast::FnDecl {
            access: ast::Access::Public,
            name: "id".to_string(),
            type_params: vec![ast::TypeParam { name: "T".to_string(), constraint: None }],
            params: vec![ast::Param {
                name: "x".to_string(),
                typ: ast::Type::simple("T"),
            }],
            return_type: Some(ast::Type::simple("T")),
            body: vec![ast::Stmt::Return(Some(ast::Expr::Identifier("x".to_string(), su())))],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let caller_fn = ast::FnDecl {
            access: ast::Access::Public,
            name: "caller".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::Expr(ast::Expr::Call(
                Box::new(ast::Expr::Identifier("id__int".to_string(), su())),
                vec![ast::Expr::Literal(ast::Literal::Int(42), su())],
                su(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![
                ast::Declaration::Function(generic_fn),
                ast::Declaration::Function(caller_fn),
            ],
        };

        let mut compiler = Compiler::new();

        // Register declarations first (first pass).
        for decl in &program.declarations {
            if let ast::Declaration::Function(fn_decl) = decl {
                compiler.register_function(fn_decl);
            }
        }

        // Now instantiate the generic function.
        let fn_idx = compiler.instantiate_generic_function("id", &[ast::Type::simple("int")])
            .expect("instantiation should succeed");
        assert!(fn_idx > 0, "instantiated function should have a valid index");

        // Verify the mangled name is in function_map.
        assert!(
            compiler.function_map.contains_key("id__int"),
            "function_map should contain mangled name 'id__int'"
        );

        // Verify mono_cache.
        assert!(
            compiler.mono_cache.contains_key("id__int"),
            "mono_cache should contain 'id__int'"
        );

        // Second instantiation should return the same index (cache hit).
        let fn_idx2 = compiler.instantiate_generic_function("id", &[ast::Type::simple("int")])
            .expect("second instantiation should succeed");
        assert_eq!(fn_idx, fn_idx2, "cached instantiation should return same index");

        // Now compile the full program.
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        // The specialized function should exist.
        let found = compiled.functions.iter().any(|f| f.name == "id__int");
        assert!(found, "compiled program should contain function 'id__int'");
    }

    // -- test_instantiate_generic_class -------------------------------------------

    #[test]
    fn test_instantiate_generic_class() {
        let generic_class = ast::ClassDecl {
            name: "Box".to_string(),
            type_params: vec![ast::TypeParam { name: "T".to_string(), constraint: None }],
            parent: None,
            ifaces: vec![],
            members: vec![
                ast::ClassMember::Field(ast::FieldDecl {
                    access: ast::Access::Public,
                    name: "value".to_string(),
                    typ: ast::Type::simple("T"),
                    init: None,
                    span: su(),
                }),
                ast::ClassMember::Method(ast::MethodDecl {
                    access: ast::Access::Public,
                    name: "getValue".to_string(),
                    type_params: vec![],
                    params: vec![],
                    return_type: Some(ast::Type::simple("T")),
                    body: vec![ast::Stmt::Return(Some(ast::Expr::MemberAccess(
                        Box::new(ast::Expr::This(su())),
                        "value".to_string(),
                        su(),
                    )))],
                    where_clause: vec![],
                    span: su(),
                }),
            ],
            span: su(),
        };

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "make_box".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::Expr(ast::Expr::New(
                ast::Type::generic("Box", vec![ast::Type::simple("int")]),
                vec![],
                su(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![
                ast::Declaration::Class(generic_class),
                ast::Declaration::Function(fn_decl),
            ],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        // The specialized class should exist with mangled name.
        let found_class = compiled.classes.iter().any(|c| c.name == "Box__int");
        assert!(found_class, "compiled program should contain class 'Box__int'");

        // The specialized method should exist.
        let found_method = compiled.functions.iter().any(|f| f.name == "Box__int.getValue");
        assert!(found_method, "compiled program should contain method 'Box__int.getValue'");
    }

    // -- test_generic_builtin_class ----------------------------------------------

    #[test]
    fn test_generic_builtin_class() {
        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_builtin".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![
                ast::Stmt::Expr(ast::Expr::New(
                    ast::Type::generic("ArrayList", vec![ast::Type::simple("int")]),
                    vec![],
                    su(),
                )),
                ast::Stmt::Expr(ast::Expr::New(
                    ast::Type::generic("HashMap", vec![ast::Type::simple("string"), ast::Type::simple("int")]),
                    vec![],
                    su(),
                )),
            ],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::Function(fn_decl)],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        // ArrayList<int> should create class "ArrayList__int"
        let found_al = compiled.classes.iter().any(|c| c.name == "ArrayList__int");
        assert!(found_al, "compiled program should contain class 'ArrayList__int'");

        // HashMap<string, int> should create class "HashMap__string__int"
        let found_hm = compiled.classes.iter().any(|c| c.name == "HashMap__string__int");
        assert!(found_hm, "compiled program should contain class 'HashMap__string__int'");
    }

    // -- test_generic_function_error_without_type_args ----------------------------

    #[test]
    fn test_generic_function_error_without_type_args() {
        let generic_fn = ast::FnDecl {
            access: ast::Access::Public,
            name: "id".to_string(),
            type_params: vec![ast::TypeParam { name: "T".to_string(), constraint: None }],
            params: vec![ast::Param {
                name: "x".to_string(),
                typ: ast::Type::simple("T"),
            }],
            return_type: Some(ast::Type::simple("T")),
            body: vec![ast::Stmt::Return(Some(ast::Expr::Identifier("x".to_string(), su())))],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        // Calling id(42) without type arguments should fail.
        let caller_fn = ast::FnDecl {
            access: ast::Access::Public,
            name: "caller".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::Expr(ast::Expr::Call(
                Box::new(ast::Expr::Identifier("id".to_string(), su())),
                vec![ast::Expr::Literal(ast::Literal::Int(42), su())],
                su(),
            ))],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![
                ast::Declaration::Function(generic_fn),
                ast::Declaration::Function(caller_fn),
            ],
        };

        let mut compiler = Compiler::new();
        let result = compiler.compile(&program);
        assert!(result.is_err(), "calling generic function without type args should fail");
        let err = result.err().unwrap();
        assert!(err.contains("generic function"), "error should mention generic function");
    }

    // -- Module system tests -----------------------------------------------------

    // -- test_module_resolve_path ------------------------------------------------

    #[test]
    fn test_module_resolve_path() {
        let mut resolver = resolver::ModuleResolver::new();
        let root = std::env::temp_dir();

        // Create a temporary directory structure: root/tt/lang/Integer.tr
        let dir = root.join("tt").join("lang");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("Integer.tr");
        std::fs::write(&file_path, "fn main() {}").unwrap();

        let import_path: Vec<String> = vec!["tt".to_string(), "lang".to_string(), "Integer".to_string()];
        let resolved = resolver.resolve(&import_path, &root).expect("should resolve");
        assert_eq!(resolved, file_path);

        // Cleanup
        std::fs::remove_dir_all(root.join("tt")).ok();
    }

    // -- test_module_resolve_cached ----------------------------------------------

    #[test]
    fn test_module_resolve_cached() {
        let mut resolver = resolver::ModuleResolver::new();
        let root = std::env::temp_dir();

        let dir = root.join("cache_test").join("sub");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("Module.tr");
        std::fs::write(&file_path, "fn main() {}").unwrap();

        let import_path: Vec<String> = vec!["cache_test".to_string(), "sub".to_string(), "Module".to_string()];

        // First resolve: file system lookup.
        let resolved1 = resolver.resolve(&import_path, &root).expect("should resolve");
        // Second resolve: should come from cache even if file is deleted.
        std::fs::remove_file(&file_path).unwrap();
        let resolved2 = resolver.resolve(&import_path, &root).expect("should resolve from cache");
        assert_eq!(resolved1, resolved2);

        // Cleanup
        std::fs::remove_dir_all(root.join("cache_test")).ok();
    }

    // -- test_module_resolve_not_found -------------------------------------------

    #[test]
    fn test_module_resolve_not_found() {
        let mut resolver = resolver::ModuleResolver::new();
        let root = std::env::temp_dir();

        let import_path: Vec<String> = vec!["nonexistent".to_string(), "Module".to_string()];
        let result = resolver.resolve(&import_path, &root);
        assert!(result.is_err(), "should fail for nonexistent module");
        let err = result.err().unwrap();
        assert!(err.contains("Cannot resolve module"), "error should mention resolution failure");
    }

    // -- test_module_resolve_glob ------------------------------------------------

    #[test]
    fn test_module_resolve_glob() {
        let mut resolver = resolver::ModuleResolver::new();
        let root = std::env::temp_dir();

        let dir = root.join("glob_test").join("io");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Reader.tr"), "fn main() {}").unwrap();
        std::fs::write(dir.join("Writer.tr"), "fn main() {}").unwrap();

        let import_path: Vec<String> = vec!["glob_test".to_string(), "io".to_string()];
        let files = resolver.resolve_glob(&import_path, &root).expect("should resolve glob");
        assert_eq!(files.len(), 2);
        assert!(files[0].file_name().unwrap().to_str().unwrap().contains("Reader"));
        assert!(files[1].file_name().unwrap().to_str().unwrap().contains("Writer"));

        // Cleanup
        std::fs::remove_dir_all(root.join("glob_test")).ok();
    }

    // -- test_symbol_table -------------------------------------------------------

    #[test]
    fn test_symbol_table() {
        let mut compiler = Compiler::new();

        // Manually insert symbols into the symbol table.
        compiler.symbol_table.insert("imported_fn".to_string(), Symbol::Function(5));
        compiler.symbol_table.insert("MyClass".to_string(), Symbol::Class(2));
        compiler.symbol_table.insert("Color".to_string(), Symbol::Enum(0));

        assert!(matches!(compiler.symbol_table.get("imported_fn"), Some(Symbol::Function(5))));
        assert!(matches!(compiler.symbol_table.get("MyClass"), Some(Symbol::Class(2))));
        assert!(matches!(compiler.symbol_table.get("Color"), Some(Symbol::Enum(0))));
        assert!(compiler.symbol_table.get("nonexistent").is_none());
    }

    // -- test_module_load --------------------------------------------------------

    #[test]
    fn test_module_load() {
        let root = std::env::temp_dir();
        let dir = root.join("load_test");
        std::fs::create_dir_all(&dir).unwrap();

        let source = r#"fn greet() { 1 + 2; }"#;
        let file_path = dir.join("Greeter.tr");
        std::fs::write(&file_path, source).unwrap();

        let mut compiler = Compiler::new();
        let idx = compiler.load_module(&file_path, "load_test.Greeter").expect("should load module");
        assert_eq!(idx, 0);
        assert_eq!(compiler.modules.len(), 1);
        assert_eq!(compiler.modules[0].name, "load_test.Greeter");
        assert!(compiler.modules[0].program.is_some());
        assert!(!compiler.modules[0].compiled);

        // Loading again should return the same index (cached).
        let idx2 = compiler.load_module(&file_path, "load_test.Greeter").expect("should load cached module");
        assert_eq!(idx, idx2);
        assert_eq!(compiler.modules.len(), 1);

        // Cleanup
        std::fs::remove_dir_all(&dir).ok();
    }

    // -- test_module_compile_with_modules ----------------------------------------

    #[test]
    fn test_module_compile_with_modules() {
        let root = std::env::temp_dir();
        let dir = root.join("compile_mod_test");
        std::fs::create_dir_all(&dir).unwrap();

        // Create a library module: Helper.tr with a public function.
        let helper_source = r#"public fn add(a: long, b: long): long { return a + b; }"#;
        std::fs::write(dir.join("Helper.tr"), helper_source).unwrap();

        // Create a main program that imports Helper.
        let main_program = ast::Program {
            imports: vec![ast::Import {
                path: vec!["compile_mod_test".to_string(), "Helper".to_string()],
                glob: false,
                span: su(),
            }],
            declarations: vec![ast::Declaration::Function(ast::FnDecl {
                access: ast::Access::Public,
                name: "main".to_string(),
                type_params: vec![],
                params: vec![],
                return_type: None,
                body: vec![ast::Stmt::Expr(ast::Expr::Call(
                    Box::new(ast::Expr::Identifier("add".to_string(), su())),
                    vec![
                        ast::Expr::Literal(ast::Literal::Int(1), su()),
                        ast::Expr::Literal(ast::Literal::Int(2), su()),
                    ],
                    su(),
                ))],
                sugar: false,
                where_clause: vec![],
                span: su(),
            })],
        };

        let mut compiler = Compiler::new();
        let result = compiler.compile_with_modules(&main_program, &root);
        assert!(result.is_ok(), "compile_with_modules should succeed: {:?}", result.err());

        let compiled = result.unwrap();
        // Should have the Helper.add function (mangled as "compile_mod_test.Helper.add") and the main function.
        let has_helper_add = compiled.functions.iter().any(|f| f.name == "compile_mod_test.Helper.add");
        assert!(has_helper_add, "compiled program should contain 'compile_mod_test.Helper.add' function");

        // Cleanup
        std::fs::remove_dir_all(&dir).ok();
    }

    // -- test_module_visibility --------------------------------------------------

    #[test]
    fn test_module_visibility() {
        let mut compiler = Compiler::new();

        // Create a module with a public and a private function.
        let module_program = ast::Program {
            imports: vec![],
            declarations: vec![
                ast::Declaration::Function(ast::FnDecl {
                    access: ast::Access::Public,
                    name: "visible_fn".to_string(),
                    type_params: vec![],
                    params: vec![],
                    return_type: None,
                    body: vec![],
                    sugar: false,
                    where_clause: vec![],
                    span: su(),
                }),
                ast::Declaration::Function(ast::FnDecl {
                    access: ast::Access::Private,
                    name: "hidden_fn".to_string(),
                    type_params: vec![],
                    params: vec![],
                    return_type: None,
                    body: vec![],
                    sugar: false,
                    where_clause: vec![],
                    span: su(),
                }),
            ],
        };

        // Register the module's declarations.
        compiler.register_module_declarations(&module_program, "mymod").unwrap();

        // The public function should be in function_map with mangled name.
        assert!(compiler.function_map.contains_key("mymod.visible_fn"),
            "public function should be registered with mangled name");

        // The private function should also be registered with mangled name
        // so that it can be called from within the same module.
        assert!(compiler.function_map.contains_key("mymod.hidden_fn"),
            "private function should be registered with mangled name for intra-module calls");
    }

    // -- test_module_circular_import ---------------------------------------------

    #[test]
    fn test_module_circular_import_detection() {
        let root = std::env::temp_dir();
        let dir = root.join("circular_test");
        std::fs::create_dir_all(&dir).unwrap();

        // Create two modules that import each other.
        let a_source = r#"import circular_test::B; fn a() {}"#;
        let b_source = r#"import circular_test::A; fn b() {}"#;

        std::fs::write(dir.join("A.tr"), a_source).unwrap();
        std::fs::write(dir.join("B.tr"), b_source).unwrap();

        let main_program = ast::Program {
            imports: vec![ast::Import {
                path: vec!["circular_test".to_string(), "A".to_string()],
                glob: false,
                span: su(),
            }],
            declarations: vec![],
        };

        let mut compiler = Compiler::new();
        let result = compiler.compile_with_modules(&main_program, &root);
        // Circular imports are now handled gracefully by breaking the cycle.
        // The compiler should succeed because declarations are registered
        // in a first pass before compiling function bodies.
        assert!(result.is_ok(), "circular import should be handled gracefully: {:?}", result.err());

        // Cleanup
        std::fs::remove_dir_all(&dir).ok();
    }

    // -- test_module_glob_import -------------------------------------------------

    #[test]
    fn test_module_glob_import() {
        let root = std::env::temp_dir();
        let dir = root.join("glob_import_test").join("utils");
        std::fs::create_dir_all(&dir).unwrap();

        // Create two utility modules.
        std::fs::write(dir.join("Math.tr"), "public fn sqrt(x: double): double { return x; }").unwrap();
        std::fs::write(dir.join("Str.tr"), "public fn trim(s: string): string { return s; }").unwrap();

        let main_program = ast::Program {
            imports: vec![ast::Import {
                path: vec!["glob_import_test".to_string(), "utils".to_string()],
                glob: true,
                span: su(),
            }],
            declarations: vec![],
        };

        let mut compiler = Compiler::new();
        let result = compiler.compile_with_modules(&main_program, &root);
        assert!(result.is_ok(), "glob import should succeed: {:?}", result.err());

        // Both modules should be loaded.
        assert!(compiler.modules.len() >= 2, "should have loaded at least 2 modules from glob");

        // Cleanup
        std::fs::remove_dir_all(root.join("glob_import_test")).ok();
    }

    // -- test_module_compile_imported_class ----------------------------------------

    #[test]
    fn test_module_compile_imported_class() {
        let root = std::env::temp_dir();
        let dir = root.join("class_import_test");
        std::fs::create_dir_all(&dir).unwrap();

        // Create a module with a public class.
        let helper_source = r#"public class Point { public long x; public long y; public fn getX(): long { return this.x; } }"#;
        std::fs::write(dir.join("Geometry.tr"), helper_source).unwrap();

        let main_program = ast::Program {
            imports: vec![ast::Import {
                path: vec!["class_import_test".to_string(), "Geometry".to_string()],
                glob: false,
                span: su(),
            }],
            declarations: vec![],
        };

        let mut compiler = Compiler::new();
        let result = compiler.compile_with_modules(&main_program, &root);
        assert!(result.is_ok(), "importing a class module should succeed: {:?}", result.err());

        let compiled = result.unwrap();
        let has_point = compiled.classes.iter().any(|c| c.name == "class_import_test.Geometry.Point");
        assert!(has_point, "compiled program should contain 'class_import_test.Geometry.Point' class");

        // Cleanup
        std::fs::remove_dir_all(&dir).ok();
    }

    // -- test_compile_closure -----------------------------------------------------

    #[test]
    fn test_compile_closure() {
        let closure_expr = ast::Expr::Closure {
            params: vec![("x".to_string(), ast::Type::simple("long"))],
            return_type: ast::Type::simple("long"),
            body: vec![],
            expr: Some(Box::new(ast::Expr::Binary(
                Box::new(ast::Expr::Identifier("x".to_string(), su())),
                ast::Operator::Add,
                Box::new(ast::Expr::Literal(ast::Literal::Int(1), su())),
                su(),
            ))),
            captured_vars: vec![],
            span: su(),
        };

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_closure".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::VarDecl(ast::VarDecl {
                name: "f".to_string(),
                typ: None,
                init: Some(closure_expr),
                mutable: false,
                span: su(),
            })],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::Function(fn_decl)],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        let fn_chunk = &compiled.functions[1].chunk;
        assert!(
            fn_chunk.code.contains(&(OpCode::CLOSURE_NEW as u8)),
            "closure expression should emit CLOSURE_NEW"
        );
    }

    // -- test_compile_tuple -------------------------------------------------------

    #[test]
    fn test_compile_tuple() {
        let tuple_expr = ast::Expr::Tuple(vec![
            ast::Expr::Literal(ast::Literal::Int(1), su()),
            ast::Expr::Literal(ast::Literal::Int(2), su()),
            ast::Expr::Literal(ast::Literal::Int(3), su()),
        ], su());

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_tuple".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![ast::Stmt::VarDecl(ast::VarDecl {
                name: "t".to_string(),
                typ: None,
                init: Some(tuple_expr),
                mutable: false,
                span: su(),
            })],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::Function(fn_decl)],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        let fn_chunk = &compiled.functions[1].chunk;
        assert!(
            fn_chunk.code.contains(&(OpCode::TUPLE_NEW as u8)),
            "tuple expression should emit TUPLE_NEW"
        );
    }

    // -- test_compile_operator_overload -------------------------------------------

    #[test]
    fn test_compile_operator_overload() {
        let class_decl = ast::ClassDecl {
            name: "Vec2".to_string(),
            type_params: vec![],
            parent: None,
            ifaces: vec![],
            members: vec![
                ast::ClassMember::Field(ast::FieldDecl {
                    access: ast::Access::Public,
                    name: "x".to_string(),
                    typ: ast::Type::simple("long"),
                    init: Some(ast::Expr::Literal(ast::Literal::Int(0), su())),
                    span: su(),
                }),
                ast::ClassMember::Field(ast::FieldDecl {
                    access: ast::Access::Public,
                    name: "y".to_string(),
                    typ: ast::Type::simple("long"),
                    init: Some(ast::Expr::Literal(ast::Literal::Int(0), su())),
                    span: su(),
                }),
                ast::ClassMember::Method(ast::MethodDecl {
                    access: ast::Access::Public,
                    name: "operator+".to_string(),
                    type_params: vec![],
                    params: vec![ast::Param {
                        name: "other".to_string(),
                        typ: ast::Type::simple("Vec2"),
                    }],
                    return_type: Some(ast::Type::simple("Vec2")),
                    body: vec![ast::Stmt::Return(Some(ast::Expr::New(
                        ast::Type::simple("Vec2"),
                        vec![],
                        su(),
                    )))],
                    where_clause: vec![],
                    span: su(),
                }),
            ],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::Class(class_decl)],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        assert_eq!(compiled.classes.len(), 1);
        assert_eq!(compiled.classes[0].name, "Vec2");
        // The operator+ method should be registered in the class methods
        assert!(
            compiled.classes[0].methods.contains_key("operator+"),
            "Vec2 class should have operator+ method"
        );
    }

    // -- test_compile_for_in -----------------------------------------------------

    #[test]
    fn test_compile_for_in() {
        let for_stmt = ast::Stmt::For(ast::ForStmt {
            var: "item".to_string(),
            iterable: ast::Expr::Identifier("list".to_string(), su()),
            body: vec![ast::Stmt::Expr(ast::Expr::Identifier("item".to_string(), su()))],
            span: su(),
        });

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_for_in".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![for_stmt],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::Function(fn_decl)],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        let fn_chunk = &compiled.functions[1].chunk;
        // For-in loop should contain JMP_IF_FALSE for the loop condition
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP_IF_FALSE as u8)),
            "for-in loop should emit JMP_IF_FALSE"
        );
        // And a JMP back to the start
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP as u8)),
            "for-in loop should emit JMP back to start"
        );
    }

    // -- test_compile_c_style_for ------------------------------------------------

    #[test]
    fn test_compile_c_style_for() {
        let cfor_stmt = ast::Stmt::CFor(ast::CForStmt {
            init: Some(Box::new(ast::Stmt::VarDecl(ast::VarDecl {
                name: "i".to_string(),
                typ: None,
                init: Some(ast::Expr::Literal(ast::Literal::Int(0), su())),
                mutable: true,
                span: su(),
            }))),
            condition: Some(ast::Expr::Binary(
                Box::new(ast::Expr::Identifier("i".to_string(), su())),
                ast::Operator::Lt,
                Box::new(ast::Expr::Literal(ast::Literal::Int(10), su())),
                su(),
            )),
            increment: Some(ast::Expr::Assign(
                Box::new(ast::Expr::Identifier("i".to_string(), su())),
                Box::new(ast::Expr::Binary(
                    Box::new(ast::Expr::Identifier("i".to_string(), su())),
                    ast::Operator::Add,
                    Box::new(ast::Expr::Literal(ast::Literal::Int(1), su())),
                    su(),
                )),
                su(),
            )),
            body: vec![ast::Stmt::Break],
            span: su(),
        });

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_c_for".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![cfor_stmt],
            sugar: false,
            where_clause: vec![],
            span: su(),
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
            "C-style for should emit JMP_IF_FALSE"
        );
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP as u8)),
            "C-style for should emit JMP"
        );
    }

    // -- test_compile_while_let --------------------------------------------------

    #[test]
    fn test_compile_while_let() {
        let while_let_stmt = ast::Stmt::WhileLet(ast::WhileLetStmt {
            var_name: "line".to_string(),
            expr: ast::Expr::Call(
                Box::new(ast::Expr::MemberAccess(
                    Box::new(ast::Expr::Identifier("file".to_string(), su())),
                    "readLine".to_string(),
                    su(),
                )),
                vec![],
                su(),
            ),
            body: vec![ast::Stmt::Break],
            span: su(),
        });

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_while_let".to_string(),
            type_params: vec![],
            params: vec![],
            return_type: None,
            body: vec![while_let_stmt],
            sugar: false,
            where_clause: vec![],
            span: su(),
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
            "while-let should emit JMP_IF_FALSE"
        );
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP as u8)),
            "while-let should emit JMP back to start"
        );
    }

    // -- test_compile_switch_enum ------------------------------------------------

    #[test]
    fn test_compile_switch_enum() {
        let enum_decl = ast::EnumDecl {
            name: "Color".to_string(),
            type_params: vec![],
            variants: vec![
                ast::Variant {
                    name: "Red".to_string(),
                    fields: vec![],
                },
                ast::Variant {
                    name: "Blue".to_string(),
                    fields: vec![],
                },
            ],
            span: su(),
        };

        let switch_stmt = ast::Stmt::Switch(ast::SwitchStmt {
            expr: ast::Expr::Identifier("color".to_string(), su()),
            cases: vec![
                ast::Case {
                    pattern: ast::Pattern::Constructor {
                        name: "Red".to_string(),
                        bindings: vec![],
                    },
                    body: vec![ast::Stmt::Expr(ast::Expr::Literal(ast::Literal::Int(1), su()))],
                },
                ast::Case {
                    pattern: ast::Pattern::Constructor {
                        name: "Blue".to_string(),
                        bindings: vec![],
                    },
                    body: vec![ast::Stmt::Expr(ast::Expr::Literal(ast::Literal::Int(2), su()))],
                },
            ],
            default: None,
            span: su(),
        });

        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "test_switch_enum".to_string(),
            type_params: vec![],
            params: vec![ast::Param {
                name: "color".to_string(),
                typ: ast::Type::simple("Color"),
            }],
            return_type: None,
            body: vec![switch_stmt],
            sugar: false,
            where_clause: vec![],
            span: su(),
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

        let fn_chunk = &compiled.functions[1].chunk;
        assert!(
            fn_chunk.code.contains(&(OpCode::DUP as u8)),
            "switch on enum should DUP the subject"
        );
        assert!(
            fn_chunk.code.contains(&(OpCode::JMP_IF_FALSE as u8)),
            "switch on enum should use JMP_IF_FALSE"
        );
    }

    // -- test_compile_where_clause -----------------------------------------------

    #[test]
    fn test_compile_where_clause() {
        // Compile a non-generic function with an empty where clause
        // (where clause is for type checking; compilation should succeed)
        let fn_decl = ast::FnDecl {
            access: ast::Access::Public,
            name: "foo".to_string(),
            type_params: vec![],
            params: vec![ast::Param {
                name: "x".to_string(),
                typ: ast::Type::simple("int"),
            }],
            return_type: Some(ast::Type::simple("void")),
            body: vec![ast::Stmt::Return(None)],
            sugar: false,
            where_clause: vec![],
            span: su(),
        };

        let program = ast::Program {
            imports: vec![],
            declarations: vec![ast::Declaration::Function(fn_decl)],
        };

        let mut compiler = Compiler::new();
        let compiled = compiler.compile(&program).expect("compilation should succeed");

        // The function should be compiled
        assert!(
            compiled.functions.len() >= 2,
            "compiled program should contain main and foo functions"
        );

        // Also test that a generic function with where clause can be registered
        let generic_fn = ast::FnDecl {
            access: ast::Access::Public,
            name: "bar".to_string(),
            type_params: vec![ast::TypeParam {
                name: "T".to_string(),
                constraint: Some(ast::Type::simple("Comparable")),
            }],
            params: vec![ast::Param {
                name: "x".to_string(),
                typ: ast::Type::simple("T"),
            }],
            return_type: Some(ast::Type::simple("void")),
            body: vec![ast::Stmt::Return(None)],
            sugar: false,
            where_clause: vec![ast::TypeParam {
                name: "T".to_string(),
                constraint: Some(ast::Type::simple("Comparable")),
            }],
            span: su(),
        };

        let mut compiler2 = Compiler::new();
        // Register the generic function (should not fail on registration)
        compiler2.register_function(&generic_fn);
        // Generic functions are stored in generic_function_map, not function_map
        assert!(
            compiler2.generic_function_map.contains_key("bar"),
            "generic function should be registered in generic_function_map"
        );
        assert_eq!(
            compiler2.generic_functions.len(),
            1,
            "generic_functions should contain one entry"
        );
    }

    // =========================================================================
    // Constant folding tests
    // =========================================================================

    /// Helper: build a chunk with PUSH_I64 + PUSH_I64 + binary-op, run
    /// constant folding, and return the resulting chunk.
    fn fold_i64_chunk(va: i64, vb: i64, op: OpCode) -> Chunk {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I64, 1);
        chunk.code.extend_from_slice(&va.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 8]);
        chunk.write_opcode(OpCode::PUSH_I64, 1);
        chunk.code.extend_from_slice(&vb.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 8]);
        chunk.write_opcode(op, 1);
        chunk.write_opcode(OpCode::RET, 1);
        Compiler::fold_constants_chunk(&mut chunk);
        chunk
    }

    fn fold_i32_chunk(va: i32, vb: i32, op: OpCode) -> Chunk {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&va.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&vb.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(op, 1);
        chunk.write_opcode(OpCode::RET, 1);
        Compiler::fold_constants_chunk(&mut chunk);
        chunk
    }

    #[test]
    fn test_fold_i64_add() {
        let chunk = fold_i64_chunk(3, 4, OpCode::ADD_I64);
        // After folding: PUSH_I64 7, RET  (no ADD_I64)
        assert!(!chunk.code.contains(&(OpCode::ADD_I64 as u8)),
            "ADD_I64 should be folded away");
        assert!(chunk.code.contains(&(OpCode::PUSH_I64 as u8)),
            "should still have PUSH_I64");
        // Find the PUSH_I64 and verify the value
        let push_offset = chunk.code.iter().position(|&b| b == OpCode::PUSH_I64 as u8).unwrap();
        let val = i64::from_be_bytes(chunk.code[push_offset+1..push_offset+9].try_into().unwrap());
        assert_eq!(val, 7);
    }

    #[test]
    fn test_fold_i64_sub() {
        let chunk = fold_i64_chunk(10, 3, OpCode::SUB_I64);
        assert!(!chunk.code.contains(&(OpCode::SUB_I64 as u8)));
        let push_offset = chunk.code.iter().position(|&b| b == OpCode::PUSH_I64 as u8).unwrap();
        let val = i64::from_be_bytes(chunk.code[push_offset+1..push_offset+9].try_into().unwrap());
        assert_eq!(val, 7);
    }

    #[test]
    fn test_fold_i64_mul() {
        let chunk = fold_i64_chunk(6, 7, OpCode::MUL_I64);
        assert!(!chunk.code.contains(&(OpCode::MUL_I64 as u8)));
        let push_offset = chunk.code.iter().position(|&b| b == OpCode::PUSH_I64 as u8).unwrap();
        let val = i64::from_be_bytes(chunk.code[push_offset+1..push_offset+9].try_into().unwrap());
        assert_eq!(val, 42);
    }

    #[test]
    fn test_fold_i32_add() {
        let chunk = fold_i32_chunk(20, 22, OpCode::ADD_I32);
        assert!(!chunk.code.contains(&(OpCode::ADD_I32 as u8)));
        let push_offset = chunk.code.iter().position(|&b| b == OpCode::PUSH_I32 as u8).unwrap();
        let val = i32::from_be_bytes(chunk.code[push_offset+1..push_offset+5].try_into().unwrap());
        assert_eq!(val, 42);
    }

    #[test]
    fn test_fold_i32_sub() {
        let chunk = fold_i32_chunk(100, 58, OpCode::SUB_I32);
        assert!(!chunk.code.contains(&(OpCode::SUB_I32 as u8)));
        let push_offset = chunk.code.iter().position(|&b| b == OpCode::PUSH_I32 as u8).unwrap();
        let val = i32::from_be_bytes(chunk.code[push_offset+1..push_offset+5].try_into().unwrap());
        assert_eq!(val, 42);
    }

    #[test]
    fn test_fold_i32_mul() {
        let chunk = fold_i32_chunk(6, 7, OpCode::MUL_I32);
        assert!(!chunk.code.contains(&(OpCode::MUL_I32 as u8)));
        let push_offset = chunk.code.iter().position(|&b| b == OpCode::PUSH_I32 as u8).unwrap();
        let val = i32::from_be_bytes(chunk.code[push_offset+1..push_offset+5].try_into().unwrap());
        assert_eq!(val, 42);
    }

    #[test]
    fn test_fold_string_concat() {
        let mut chunk = Chunk::new();
        let idx_a = chunk.add_string("hello");
        let idx_b = chunk.add_string(" world");
        chunk.write_opcode(OpCode::PUSH_STRING, 1);
        chunk.write_u16(idx_a, 1);
        chunk.write_opcode(OpCode::PUSH_STRING, 1);
        chunk.write_u16(idx_b, 1);
        chunk.write_opcode(OpCode::STR_CONCAT, 1);
        chunk.write_opcode(OpCode::RET, 1);

        Compiler::fold_constants_chunk(&mut chunk);

        // After folding: PUSH_STRING <new_idx>, RET  (no STR_CONCAT)
        assert!(!chunk.code.contains(&(OpCode::STR_CONCAT as u8)),
            "STR_CONCAT should be folded away");
        // The combined string "hello world" should be in the string table
        assert!(chunk.strings.iter().any(|s| s == "hello world"),
            "string table should contain 'hello world'");
    }

    #[test]
    fn test_fold_does_not_affect_non_constant() {
        // PUSH_I64 5, LOAD_LOCAL 0, ADD_I64 — should NOT fold
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I64, 1);
        chunk.code.extend_from_slice(&5i64.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 8]);
        chunk.write_opcode(OpCode::LOAD_LOCAL, 1);
        chunk.write_u8(0, 1);
        chunk.write_opcode(OpCode::ADD_I64, 1);
        chunk.write_opcode(OpCode::RET, 1);

        Compiler::fold_constants_chunk(&mut chunk);

        assert!(chunk.code.contains(&(OpCode::ADD_I64 as u8)),
            "ADD_I64 should NOT be folded when operands aren't both constants");
        assert!(chunk.code.contains(&(OpCode::LOAD_LOCAL as u8)),
            "LOAD_LOCAL should remain");
    }

    // =========================================================================
    // Dead code elimination tests
    // =========================================================================

    #[test]
    fn test_dead_code_after_ret() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&42i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::RET, 1);
        // Dead code: these should be eliminated
        chunk.write_opcode(OpCode::PUSH_I32, 1);
        chunk.code.extend_from_slice(&99i32.to_be_bytes());
        chunk.source_lines.extend_from_slice(&[1; 4]);
        chunk.write_opcode(OpCode::POP, 1);
        chunk.write_opcode(OpCode::RET, 1);

        Compiler::eliminate_dead_code_chunk(&mut chunk);

        // The dead PUSH_I32 99 and POP should be gone
        assert!(!chunk.code.contains(&(OpCode::POP as u8)),
            "POP after RET should be eliminated");
        // Only one PUSH_I32 should remain (the 42)
        let push_count = chunk.code.iter().filter(|&&b| b == OpCode::PUSH_I32 as u8).count();
        assert_eq!(push_count, 1, "only one PUSH_I32 should remain after dead code elimination");
    }

    #[test]
    fn test_dead_code_after_jmp() {
        let mut chunk = Chunk::new();
        // PUSH_BOOL true
        chunk.write_opcode(OpCode::PUSH_BOOL, 1);
        chunk.write_u8(1, 1);
        // JMP +3 (skip over the dead code)
        chunk.write_opcode(OpCode::JMP, 1);
        chunk.write_i16(3, 1);
        // Dead code:
        chunk.write_opcode(OpCode::PUSH_NULL, 1);
        chunk.write_opcode(OpCode::POP, 1);
        // Target of jump:
        chunk.write_opcode(OpCode::RET, 1);

        Compiler::eliminate_dead_code_chunk(&mut chunk);

        assert!(!chunk.code.contains(&(OpCode::PUSH_NULL as u8)),
            "PUSH_NULL after unconditional JMP should be eliminated");
        assert!(!chunk.code.contains(&(OpCode::POP as u8)),
            "POP after unconditional JMP should be eliminated");
    }

    #[test]
    fn test_dead_code_preserves_jump_targets() {
        // Code: JMP to label, dead code, label: RET
        // The jump target (label) must be preserved even though it follows
        // an unconditional JMP.
        let mut chunk = Chunk::new();
        // JMP +3 (jump over the dead PUSH_NULL+POP to the RET)
        chunk.write_opcode(OpCode::JMP, 1);
        chunk.write_i16(3, 1);
        // Dead code
        chunk.write_opcode(OpCode::PUSH_NULL, 1);
        chunk.write_opcode(OpCode::POP, 1);
        // Jump target: RET
        chunk.write_opcode(OpCode::RET, 1);

        Compiler::eliminate_dead_code_chunk(&mut chunk);

        // RET must still be present (it's a jump target)
        assert!(chunk.code.contains(&(OpCode::RET as u8)),
            "RET jump target must be preserved");
    }

    #[test]
    fn test_remove_unused_strings() {
        let mut chunk = Chunk::new();
        let used_idx = chunk.add_string("used");
        let _unused_idx = chunk.add_string("unused");
        chunk.write_opcode(OpCode::PUSH_STRING, 1);
        chunk.write_u16(used_idx, 1);
        chunk.write_opcode(OpCode::RET, 1);

        Compiler::remove_unused_strings(&mut chunk);

        assert_eq!(chunk.strings.len(), 1, "unused string should be removed");
        assert_eq!(chunk.strings[0], "used");
        // The PUSH_STRING operand should be remapped to index 0
        let push_offset = chunk.code.iter().position(|&b| b == OpCode::PUSH_STRING as u8).unwrap();
        let new_idx = u16::from_be_bytes([chunk.code[push_offset+1], chunk.code[push_offset+2]]);
        assert_eq!(new_idx, 0, "string index should be remapped to 0");
    }

    #[test]
    fn test_too_many_local_variables() {
        // Create a function with 256 parameters, which exceeds the 255 local variable limit
        let params: Vec<ast::Param> = (0..256)
            .map(|i| ast::Param {
                name: format!("v{}", i),
                typ: ast::Type::simple("long"),
            })
            .collect();

        let declarations = vec![ast::Declaration::Function(ast::FnDecl {
            access: ast::Access::Public,
            name: "too_many_locals".to_string(),
            type_params: vec![],
            params,
            return_type: None,
            body: vec![],
            sugar: false,
            where_clause: vec![],
            span: su(),
        })];

        let program = ast::Program {
            imports: vec![],
            declarations,
        };
        let mut compiler = Compiler::new();
        let result = compiler.compile(&program);
        match result {
            Err(msg) => assert_eq!(msg, "Too many local variables in function (max 255)",
                "Expected local variable overflow error, got: {}", msg),
            Ok(_) => panic!("Expected compilation error for too many local variables"),
        }
    }
}
