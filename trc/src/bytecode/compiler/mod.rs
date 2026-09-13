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
mod stmt_loops;
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
    GenericFunction(usize),
    Global(u16),
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

#[derive(Clone)]
pub(super) struct Local {
    pub name: String,
    pub depth: usize,
    #[allow(dead_code)]
    pub is_captured: bool,
    /// The stack slot assigned to this local.
    pub slot: u8,
    /// Whether this local represents a captured upvalue (rather than a
    /// real stack slot). When true, identifier reads/writes for this local
    /// emit `GET_UPVALUE`/`SET_UPVALUE` instead of `LOAD_LOCAL`/`STORE_LOCAL`.
    pub is_upvalue: bool,
    /// The upvalue index (only meaningful when `is_upvalue` is true).
    pub upvalue_idx: u8,
    /// The inferred type of this local (Unknown if not tracked).
    pub local_type: InferredType,
}

// ---------------------------------------------------------------------------
// Loop bookkeeping for break/continue
// ---------------------------------------------------------------------------

/// Sentinel value used for `continue_ip` in do-while loops where the
/// continue target is after the body and not yet known.
pub(super) const CONTINUE_PENDING: usize = usize::MAX;

pub(super) struct LoopInfo {
    /// IP that `continue` jumps to. For `while`/`for` this is the condition
    /// check; for `do-while` it starts as `CONTINUE_PENDING` and is patched
    /// after the body is compiled.
    pub continue_ip: usize,
    /// Locations to patch with the end-of-loop offset (for `break`).
    pub break_patches: Vec<(usize, u32)>,
    /// Locations to patch with the continue-target offset (for `continue`
    /// in do-while loops where the target is not yet known).
    pub continue_patches: Vec<(usize, u32)>,
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
    /// Interface declarations by (possibly mangled) name, for inheriting
    /// default method bodies into implementing classes.
    pub(super) interfaces: HashMap<String, ast::InterfaceDecl>,
    /// Loop stack for break/continue.
    pub(super) loop_stack: Vec<LoopInfo>,
    /// Number of lexically enclosing try blocks in the current function.
    /// Used to unregister exception handlers on early `return`.
    pub(super) handler_depth: usize,
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
    /// Current module being compiled (used for mangled global lookups).
    pub(super) current_module: String,
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
            param_types: Vec::new(),
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
            interfaces: HashMap::new(),
            loop_stack: Vec::new(),
            handler_depth: 0,
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
            current_module: "<main>".to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // Main entry point
    // -----------------------------------------------------------------------

    /// Look up a global variable by name, trying the mangled (module-qualified)
    /// name first, then the short name.  Returns the global slot index.
    pub(super) fn lookup_global(&self, name: &str) -> Option<u16> {
        if !self.current_module.is_empty() && self.current_module != "<main>" {
            let mangled = format!("{}.{}", self.current_module, name);
            if let Some(&idx) = self.global_map.get(&mangled) {
                return Some(idx);
            }
        }
        self.global_map.get(name).copied()
    }

    pub fn compile(&mut self, program: &ast::Program) -> Result<CompiledProgram, String> {
        self.current_module = "<main>".to_string();
        // Register built-in native functions so bare calls (println, toString, parseInt) work.
        self.get_or_add_native("println");
        self.get_or_add_native("toString");
        self.get_or_add_native("parseInt");
        // First pass: register all classes, enums, interfaces, and
        // functions (names and arities).
        for decl in &program.declarations {
            match decl {
                ast::Declaration::Class(class_decl) => self.register_class(class_decl)?,
                ast::Declaration::Enum(enum_decl) => self.register_enum(enum_decl),
                ast::Declaration::Interface(iface_decl) => {
                    self.interfaces
                        .insert(iface_decl.name.clone(), iface_decl.clone());
                }
                ast::Declaration::Function(fn_decl) => self.register_function(fn_decl),
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
        // Register built-in native functions so bare calls (println, toString, parseInt) work.
        self.get_or_add_native("println");
        self.get_or_add_native("toString");
        self.get_or_add_native("parseInt");
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
        self.current_module = "<main>".to_string();
        // First pass: register all classes, enums, interfaces, and functions.
        for decl in &program.declarations {
            match decl {
                ast::Declaration::Class(class_decl) => self.register_class(class_decl)?,
                ast::Declaration::Enum(enum_decl) => self.register_enum(enum_decl),
                ast::Declaration::Interface(iface_decl) => {
                    self.interfaces
                        .insert(iface_decl.name.clone(), iface_decl.clone());
                }
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
                ast::Declaration::ConstDecl(const_decl) if !self.global_map.contains_key(&const_decl.name) => {
                    let idx = self.globals.len() as u16;
                    self.globals.push(const_decl.name.clone());
                    self.global_map.insert(const_decl.name.clone(), idx);
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


#[cfg(test)]
mod tests;
