// Titrate Alpha 0.1 – bytecode compiler
// Lowers the AST to bytecode chunks for the VM.
// Precision in every step – richie-rich90454, 2026

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

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
// Module – represents a single source file in the module graph
// ---------------------------------------------------------------------------

struct Module {
    /// Dotted module name, e.g. "tt.lang.Integer"
    name: String,
    /// Absolute path to the source file.
    file_path: PathBuf,
    /// Parsed AST; `None` if not yet loaded.
    program: Option<ast::Program>,
    /// Whether this module has been compiled into bytecode.
    compiled: bool,
}

// ---------------------------------------------------------------------------
// Symbol – an imported declaration tracked in the symbol table
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum Symbol {
    Function(u16),
    Class(u16),
    Enum(u16),
}

// ---------------------------------------------------------------------------
// ModuleResolver – resolves import paths to file paths and caches results
// ---------------------------------------------------------------------------

struct ModuleResolver {
    /// Cache: dotted module name → resolved file path.
    cache: HashMap<String, PathBuf>,
}

impl ModuleResolver {
    fn new() -> Self {
        ModuleResolver {
            cache: HashMap::new(),
        }
    }

    /// Resolve an import path like `["tt", "lang", "Integer"]` to a file path.
    /// Searches in `root_dir` and `root_dir/lib/`.
    fn resolve(
        &mut self,
        import_path: &[String],
        root_dir: &Path,
    ) -> Result<PathBuf, String> {
        let dotted = import_path.join(".");

        // Check cache first.
        if let Some(path) = self.cache.get(&dotted) {
            return Ok(path.clone());
        }

        // Convert import path segments to a relative file path: tt/lang/Integer.tr
        let mut relative = PathBuf::new();
        for seg in &import_path[..import_path.len().saturating_sub(1)] {
            relative.push(seg);
        }
        // The last segment is the file name.
        if let Some(last) = import_path.last() {
            relative.push(format!("{}.tr", last));
        }

        // Search directories: root_dir first, then root_dir/lib/
        let search_dirs = vec![root_dir.to_path_buf(), root_dir.join("lib")];

        for dir in &search_dirs {
            let candidate = dir.join(&relative);
            if candidate.exists() {
                self.cache.insert(dotted, candidate.clone());
                return Ok(candidate);
            }
        }

        Err(format!(
            "Cannot resolve module '{}' – searched in {}",
            dotted,
            search_dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>().join(", ")
        ))
    }

    /// Resolve a glob import like `import tt::io::*` to all .tr files in the
    /// directory corresponding to the import path.
    fn resolve_glob(
        &mut self,
        import_path: &[String],
        root_dir: &Path,
    ) -> Result<Vec<PathBuf>, String> {
        // The directory is the full import path converted to a path.
        let mut dir_relative = PathBuf::new();
        for seg in import_path {
            dir_relative.push(seg);
        }

        let search_dirs = vec![root_dir.to_path_buf(), root_dir.join("lib")];

        for dir in &search_dirs {
            let candidate = dir.join(&dir_relative);
            if candidate.is_dir() {
                let mut files = Vec::new();
                if let Ok(entries) = std::fs::read_dir(&candidate) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("tr") {
                            files.push(path);
                        }
                    }
                }
                if !files.is_empty() {
                    files.sort();
                    return Ok(files);
                }
            }
        }

        Err(format!(
            "Cannot resolve glob import '{}' – no .tr files found in {}",
            import_path.join("."),
            search_dirs.iter().map(|d| d.join(&dir_relative).display().to_string()).collect::<Vec<_>>().join(", ")
        ))
    }
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
    /// Current class index being compiled (for super() resolution).
    current_class: Option<u16>,
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
    /// Monomorphization cache: mangled name → index.
    mono_cache: HashMap<String, u16>,
    /// Generic function declarations (not registered in function_map).
    generic_functions: Vec<ast::FnDecl>,
    /// Generic class declarations (not registered in class_map).
    generic_classes: Vec<ast::ClassDecl>,
    /// Map from generic function base name → index in generic_functions.
    generic_function_map: HashMap<String, usize>,
    /// Map from generic class base name → index in generic_classes.
    generic_class_map: HashMap<String, usize>,
    /// Loaded modules (indexed by position).
    modules: Vec<Module>,
    /// Map from dotted module name → index in modules.
    module_map: HashMap<String, usize>,
    /// Symbol table for the current module: name → Symbol.
    symbol_table: HashMap<String, Symbol>,
    /// Module resolver for resolving import paths to file paths.
    resolver: ModuleResolver,
    /// Set of module names currently being processed (for circular import detection).
    processing: HashSet<String>,
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
            resolver: ModuleResolver::new(),
            processing: HashSet::new(),
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

        Ok(CompiledProgram {
            functions: std::mem::take(&mut self.functions),
            classes: std::mem::take(&mut self.classes),
            enums: std::mem::take(&mut self.enums),
            native_names: std::mem::take(&mut self.native_names),
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

        // Step 4: Compile all modules in dependency order.
        for &module_idx in &order {
            if self.modules[module_idx].compiled {
                continue;
            }
            // Clone to avoid borrow conflicts.
            let (prog_opt, module_name) = {
                let m = &self.modules[module_idx];
                (m.program.clone(), m.name.clone())
            };
            if let Some(ref prog) = prog_opt {
                // Register public declarations from this module with mangled names.
                self.register_module_declarations(prog, &module_name)?;
                // Compile the module's declarations.
                self.compile_module_program(prog)?;
            }
            self.modules[module_idx].compiled = true;
        }

        // Step 5: Now compile the root program.
        // First pass: register all classes, enums, and functions.
        for decl in &program.declarations {
            match decl {
                ast::Declaration::Class(class_decl) => self.register_class(class_decl)?,
                ast::Declaration::Enum(enum_decl) => self.register_enum(enum_decl),
                ast::Declaration::Function(fn_decl) => self.register_function(fn_decl),
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

        Ok(CompiledProgram {
            functions: std::mem::take(&mut self.functions),
            classes: std::mem::take(&mut self.classes),
            enums: std::mem::take(&mut self.enums),
            native_names: std::mem::take(&mut self.native_names),
        })
    }

    /// Process imports from a program: resolve paths and load modules.
    fn process_imports(&mut self, program: &ast::Program, root_dir: &Path) -> Result<(), String> {
        for import in &program.imports {
            let path = &import.path;

            // Check for glob import (last segment is "*").
            if path.last().map(|s| s.as_str()) == Some("*") {
                let dir_path = &path[..path.len() - 1];
                let files = self.resolver.resolve_glob(dir_path, root_dir)?;
                for file in files {
                    // Compute the dotted name from the file path relative to root_dir.
                    let dotted = Self::path_to_dotted_name(&file, root_dir);
                    self.load_module(&file, &dotted)?;
                }
            } else {
                let file_path = self.resolver.resolve(path, root_dir)?;
                let dotted = path.join(".");
                self.load_module(&file_path, &dotted)?;
            }
        }
        Ok(())
    }

    /// Convert a file path to a dotted module name by making it relative to
    /// the root directory and replacing separators with ".".
    fn path_to_dotted_name(file_path: &Path, root_dir: &Path) -> String {
        file_path
            .strip_prefix(root_dir)
            .unwrap_or(file_path)
            .with_extension("")
            .iter()
            .filter_map(|c| c.to_str())
            .collect::<Vec<_>>()
            .join(".")
    }

    /// Register declarations from a module with mangled names.
    fn register_module_declarations(&mut self, program: &ast::Program, module_name: &str) -> Result<(), String> {
        for decl in &program.declarations {
            match decl {
                ast::Declaration::Function(fn_decl) => {
                    // Only register public functions.
                    if fn_decl.access != ast::Access::Public {
                        continue;
                    }
                    if !fn_decl.type_params.is_empty() {
                        // Generic function: store for later instantiation with mangled base name.
                        let mangled_base = format!("{}.{}", module_name, fn_decl.name);
                        let idx = self.generic_functions.len();
                        let mut mangled_fn = fn_decl.clone();
                        mangled_fn.name = mangled_base.clone();
                        self.generic_function_map.insert(mangled_base, idx);
                        self.generic_functions.push(mangled_fn);
                        continue;
                    }
                    let mangled = format!("{}.{}", module_name, fn_decl.name);
                    let idx = self.functions.len() as u16;
                    self.function_map.insert(mangled.clone(), idx);
                    self.functions.push(FunctionDef {
                        name: mangled,
                        arity: fn_decl.params.len(),
                        chunk: Chunk::new(),
                        is_method: false,
                        is_constructor: false,
                        local_count: 0,
                    });
                }
                ast::Declaration::Class(class_decl) => {
                    // Only register public classes.
                    if class_decl.name.starts_with('_') {
                        // Convention: classes starting with _ are private.
                        // But we use the access field instead.
                    }
                    // We register all classes from imported modules; visibility
                    // filtering happens at the symbol table level.
                    if !class_decl.type_params.is_empty() {
                        let mangled_base = format!("{}.{}", module_name, class_decl.name);
                        let idx = self.generic_classes.len();
                        let mut mangled_class = class_decl.clone();
                        mangled_class.name = mangled_base.clone();
                        self.generic_class_map.insert(mangled_base, idx);
                        self.generic_classes.push(mangled_class);
                        continue;
                    }
                    let mangled = format!("{}.{}", module_name, class_decl.name);
                    // Avoid duplicate registration.
                    if self.class_map.contains_key(&mangled) {
                        continue;
                    }
                    let class_idx = self.classes.len() as u16;
                    self.class_map.insert(mangled.clone(), class_idx);

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
                                    field_inits.push((field_decl.name.clone(), init_chunk));
                                }
                                fields.push(FieldDef {
                                    name: field_decl.name.clone(),
                                    has_init,
                                });
                            }
                            ast::ClassMember::Method(method_decl) => {
                                let method_mangled = format!("{}.{}", mangled, method_decl.name);
                                let fn_idx = self.functions.len() as u16;
                                self.functions.push(FunctionDef {
                                    name: method_mangled,
                                    arity: method_decl.params.len(),
                                    chunk: Chunk::new(),
                                    is_method: true,
                                    is_constructor: false,
                                    local_count: 0,
                                });
                                methods.insert(method_decl.name.clone(), fn_idx);
                            }
                            ast::ClassMember::Constructor(ctor_decl) => {
                                let ctor_mangled = format!("{}.<init>", mangled);
                                let fn_idx = self.functions.len() as u16;
                                self.functions.push(FunctionDef {
                                    name: ctor_mangled,
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
                        name: mangled,
                        parent: parent_idx,
                        fields,
                        methods,
                        constructor,
                        field_inits,
                    });
                }
                ast::Declaration::Enum(enum_decl) => {
                    // Only register public enums.
                    let mangled = format!("{}.{}", module_name, enum_decl.name);
                    if self.enum_map.contains_key(&mangled) {
                        continue;
                    }
                    let enum_idx = self.enums.len() as u16;
                    self.enum_map.insert(mangled.clone(), enum_idx);

                    let variants: Vec<VariantDef> = enum_decl
                        .variants
                        .iter()
                        .enumerate()
                        .map(|(i, v)| {
                            self.variant_map
                                .insert(v.name.clone(), (mangled.clone(), i));
                            VariantDef {
                                name: v.name.clone(),
                                field_count: v.fields.len(),
                            }
                        })
                        .collect();

                    self.enums.push(EnumDef {
                        name: mangled,
                        variants,
                    });
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Compile a module's program (second pass: function bodies and methods).
    fn compile_module_program(&mut self, program: &ast::Program) -> Result<(), String> {
        for decl in &program.declarations {
            match decl {
                ast::Declaration::Function(fn_decl) => {
                    if fn_decl.access != ast::Access::Public {
                        continue;
                    }
                    if fn_decl.type_params.is_empty() {
                        // Find the mangled name in function_map.
                        // The module name prefix was added during registration.
                        // We look it up by the mangled name.
                        // Since we don't have the module name here, we search
                        // by the function name suffix.
                        if let Some(fn_idx) = self.function_map.iter()
                            .find(|(k, _)| k.ends_with(&format!(".{}", fn_decl.name)))
                            .map(|(_, &v)| v)
                        {
                            let saved_function = self.current_function;
                            let saved_locals = std::mem::take(&mut self.locals);
                            let saved_local_count = self.local_count;
                            let saved_scope_depth = self.scope_depth;

                            self.current_function = fn_idx as usize;
                            self.locals.clear();
                            self.local_count = 0;
                            self.scope_depth = 0;

                            self.begin_scope();
                            for param in &fn_decl.params {
                                self.declare_local(&param.name);
                            }
                            self.compile_block(&fn_decl.body)?;
                            self.emit_opcode(OpCode::PUSH_VOID, 0);
                            self.emit_opcode(OpCode::RET, 0);
                            self.end_scope();

                            self.functions[fn_idx as usize].local_count = self.local_count;

                            self.current_function = saved_function;
                            self.locals = saved_locals;
                            self.local_count = saved_local_count;
                            self.scope_depth = saved_scope_depth;
                        }
                    }
                }
                ast::Declaration::Class(class_decl) => {
                    if class_decl.type_params.is_empty() {
                        // Find the mangled class name.
                        if let Some((_mangled, class_idx)) = self.class_map.iter()
                            .find(|(k, _)| k.ends_with(&format!(".{}", class_decl.name)))
                            .map(|(k, &v)| (k.clone(), v))
                        {
                            let class_idx_usize = class_idx as usize;

                            // Compile field initialisers.
                            for member in &class_decl.members {
                                if let ast::ClassMember::Field(field_decl) = member {
                                    if let Some(ref init_expr) = field_decl.init {
                                        let saved_fn = self.current_function;
                                        self.current_function = 0;
                                        self.compile_expr(init_expr)?;
                                        self.emit_opcode(OpCode::POP, 0);
                                        self.current_function = saved_fn;
                                    }
                                }
                            }

                            // Compile methods.
                            for member in &class_decl.members {
                                match member {
                                    ast::ClassMember::Method(method_decl) => {
                                        let method_fn_idx = self
                                            .classes
                                            .get(class_idx_usize)
                                            .and_then(|c| c.methods.get(&method_decl.name))
                                            .copied();
                                        if let Some(fn_idx) = method_fn_idx {
                                            self.compile_method_body(
                                                fn_idx as usize,
                                                &method_decl.params,
                                                &method_decl.body,
                                            )?;
                                        }
                                    }
                                    ast::ClassMember::Constructor(ctor_decl) => {
                                        let ctor_fn_idx = self
                                            .classes
                                            .get(class_idx_usize)
                                            .and_then(|c| c.constructor);
                                        if let Some(fn_idx) = ctor_fn_idx {
                                            self.compile_method_body(
                                                fn_idx as usize,
                                                &ctor_decl.params,
                                                &ctor_decl.body,
                                            )?;
                                        }
                                    }
                                    ast::ClassMember::Field(_) => {}
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Register imported symbols from a program's imports into the symbol table.
    /// Only public declarations from imported modules are registered.
    fn register_imported_symbols(&mut self, program: &ast::Program) -> Result<(), String> {
        for import in &program.imports {
            let path = &import.path;

            // Determine which modules and which names to import.
            if path.last().map(|s| s.as_str()) == Some("*") {
                // Glob import: import all public symbols from the module.
                let dir_path = &path[..path.len() - 1];
                let dotted_dir = dir_path.join(".");

                // Collect module indices that match the glob.
                let matching_indices: Vec<usize> = self.modules.iter().enumerate()
                    .filter(|(_, m)| m.program.is_some() && (m.name.starts_with(&dotted_dir) || m.name == dotted_dir))
                    .map(|(i, _)| i)
                    .collect();

                for idx in matching_indices {
                    self.register_public_symbols_from_module_index(idx);
                }
            } else {
                // Specific import: import tt::lang::Integer
                // The last segment is the symbol name; the rest is the module path.
                if path.len() < 2 {
                    // Single-segment import: treat as a module name.
                    let dotted = path.join(".");
                    if let Some(idx) = self.module_map.get(&dotted).copied() {
                        self.register_public_symbols_from_module_index(idx);
                    }
                } else {
                    // Multi-segment import: the last segment could be a specific
                    // symbol or the module name itself.
                    let symbol_name = path.last().unwrap().clone();
                    let module_path = &path[..path.len() - 1];
                    let dotted_module = module_path.join(".");

                    // Try to find the module.
                    if let Some(&idx) = self.module_map.get(&dotted_module) {
                        // Extract the program data to avoid borrow conflicts.
                        let prog_data = self.modules[idx].program.clone();
                        if let Some(ref prog) = prog_data {
                            // Register only the specific symbol.
                            for decl in &prog.declarations {
                                let (decl_name, is_public) = match decl {
                                    ast::Declaration::Function(f) => (&f.name, f.access == ast::Access::Public),
                                    ast::Declaration::Class(c) => (&c.name, true),
                                    ast::Declaration::Enum(e) => (&e.name, true),
                                    _ => continue,
                                };
                                if decl_name == &symbol_name && is_public {
                                    let mangled = format!("{}.{}", dotted_module, decl_name);
                                    match decl {
                                        ast::Declaration::Function(_) => {
                                            if let Some(&fn_idx) = self.function_map.get(&mangled) {
                                                self.symbol_table.insert(symbol_name.clone(), Symbol::Function(fn_idx));
                                            }
                                        }
                                        ast::Declaration::Class(_) => {
                                            if let Some(&class_idx) = self.class_map.get(&mangled) {
                                                self.symbol_table.insert(symbol_name.clone(), Symbol::Class(class_idx));
                                            }
                                        }
                                        ast::Declaration::Enum(_) => {
                                            if let Some(&enum_idx) = self.enum_map.get(&mangled) {
                                                self.symbol_table.insert(symbol_name.clone(), Symbol::Enum(enum_idx));
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    } else {
                        // Maybe the full path is the module name (e.g., import tt::lang::Integer
                        // where Integer.tr is a file in tt/lang/).
                        let dotted = path.join(".");
                        if let Some(&idx) = self.module_map.get(&dotted) {
                            self.register_public_symbols_from_module_index(idx);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Register all public symbols from a module (by index) into the symbol table.
    fn register_public_symbols_from_module_index(&mut self, module_idx: usize) {
        let (prog_data, module_name) = {
            let m = &self.modules[module_idx];
            (m.program.clone(), m.name.clone())
        };
        if let Some(ref prog) = prog_data {
            for decl in &prog.declarations {
                let (decl_name, is_public) = match decl {
                    ast::Declaration::Function(f) => (&f.name, f.access == ast::Access::Public),
                    ast::Declaration::Class(c) => (&c.name, true),
                    ast::Declaration::Enum(e) => (&e.name, true),
                    _ => continue,
                };
                if !is_public {
                    continue;
                }
                let mangled = format!("{}.{}", module_name, decl_name);
                match decl {
                    ast::Declaration::Function(_) => {
                        if let Some(&fn_idx) = self.function_map.get(&mangled) {
                            self.symbol_table.insert(decl_name.clone(), Symbol::Function(fn_idx));
                        }
                    }
                    ast::Declaration::Class(_) => {
                        if let Some(&class_idx) = self.class_map.get(&mangled) {
                            self.symbol_table.insert(decl_name.clone(), Symbol::Class(class_idx));
                        }
                    }
                    ast::Declaration::Enum(_) => {
                        if let Some(&enum_idx) = self.enum_map.get(&mangled) {
                            self.symbol_table.insert(decl_name.clone(), Symbol::Enum(enum_idx));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Topological sort of modules based on import dependencies.
    /// Returns module indices in dependency order (dependencies first).
    fn topological_sort(&self) -> Result<Vec<usize>, String> {
        let n = self.modules.len();
        let mut visited = vec![false; n];
        let mut in_stack = vec![false; n];
        let mut order = Vec::new();

        for i in 0..n {
            self.topo_visit(i, &mut visited, &mut in_stack, &mut order)?;
        }

        Ok(order)
    }

    fn topo_visit(
        &self,
        node: usize,
        visited: &mut Vec<bool>,
        in_stack: &mut Vec<bool>,
        order: &mut Vec<usize>,
    ) -> Result<(), String> {
        if in_stack[node] {
            return Err(format!(
                "Circular import detected involving module '{}'",
                self.modules[node].name
            ));
        }
        if visited[node] {
            return Ok(());
        }

        in_stack[node] = true;

        // Visit dependencies (imports).
        if let Some(ref prog) = self.modules[node].program {
            for import in &prog.imports {
                let path = &import.path;
                // Find the module this import refers to.
                if path.last().map(|s| s.as_str()) == Some("*") {
                    // Glob import: find all modules in the directory.
                    let dir_path = &path[..path.len() - 1];
                    let dotted_dir = dir_path.join(".");
                    for (idx, module) in self.modules.iter().enumerate() {
                        if module.name.starts_with(&dotted_dir) || module.name == dotted_dir {
                            self.topo_visit(idx, visited, in_stack, order)?;
                        }
                    }
                } else {
                    // Try the full path as a module name.
                    let dotted = path.join(".");
                    if let Some(&dep_idx) = self.module_map.get(&dotted) {
                        self.topo_visit(dep_idx, visited, in_stack, order)?;
                    } else {
                        // Try the parent path as a module.
                        if path.len() > 1 {
                            let parent_dotted = path[..path.len() - 1].join(".");
                            if let Some(&dep_idx) = self.module_map.get(&parent_dotted) {
                                self.topo_visit(dep_idx, visited, in_stack, order)?;
                            }
                        }
                    }
                }
            }
        }

        in_stack[node] = false;
        visited[node] = true;
        order.push(node);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // First-pass registration
    // -----------------------------------------------------------------------

    fn register_function(&mut self, fn_decl: &ast::FnDecl) {
        if !fn_decl.type_params.is_empty() {
            // Generic function: store for later instantiation.
            let idx = self.generic_functions.len();
            self.generic_function_map.insert(fn_decl.name.clone(), idx);
            self.generic_functions.push(fn_decl.clone());
            return;
        }

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
        if !class_decl.type_params.is_empty() {
            // Generic class: store for later instantiation.
            let idx = self.generic_classes.len();
            self.generic_class_map.insert(class_decl.name.clone(), idx);
            self.generic_classes.push(class_decl.clone());
            return Ok(());
        }

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
    // Monomorphization: name mangling, type substitution, instantiation
    // -----------------------------------------------------------------------

    /// Generate a mangled name for a generic specialization.
    /// E.g. mangle_name("Box", [int]) → "Box__int"
    fn mangle_name(base: &str, type_args: &[ast::Type]) -> String {
        if type_args.is_empty() {
            return base.to_string();
        }
        let mut name = base.to_string();
        for arg in type_args {
            name.push_str("__");
            name.push_str(&Self::type_to_mangle_string(arg));
        }
        name
    }

    fn type_to_mangle_string(ty: &ast::Type) -> String {
        match ty {
            ast::Type::Named { name, params } => {
                if params.is_empty() {
                    name.clone()
                } else {
                    let mut s = name.clone();
                    for p in params {
                        s.push_str("__");
                        s.push_str(&Self::type_to_mangle_string(p));
                    }
                    s
                }
            }
        }
    }

    /// Substitute type parameters with concrete types.
    /// E.g. if type_args = {"T": int}, then T → int, Owned<T> → Owned<int>.
    fn substitute_type(ty: &ast::Type, type_args: &HashMap<String, ast::Type>) -> ast::Type {
        match ty {
            ast::Type::Named { name, params } => {
                // If this is a simple type parameter reference, substitute it.
                if params.is_empty() {
                    if let Some(concrete) = type_args.get(name) {
                        return concrete.clone();
                    }
                }
                // Otherwise, recursively substitute in params.
                let new_params: Vec<ast::Type> = params
                    .iter()
                    .map(|p| Self::substitute_type(p, type_args))
                    .collect();
                ast::Type::Named {
                    name: name.clone(),
                    params: new_params,
                }
            }
        }
    }

    fn substitute_expr(expr: &ast::Expr, type_args: &HashMap<String, ast::Type>) -> ast::Expr {
        match expr {
            ast::Expr::Literal(lit, span) => ast::Expr::Literal(lit.clone(), *span),
            ast::Expr::Identifier(name, span) => ast::Expr::Identifier(name.clone(), *span),
            ast::Expr::Binary(left, op, right, span) => ast::Expr::Binary(
                Box::new(Self::substitute_expr(left, type_args)),
                op.clone(),
                Box::new(Self::substitute_expr(right, type_args)),
                *span,
            ),
            ast::Expr::Unary(op, operand, span) => ast::Expr::Unary(
                op.clone(),
                Box::new(Self::substitute_expr(operand, type_args)),
                *span,
            ),
            ast::Expr::Call(callee, args, span) => ast::Expr::Call(
                Box::new(Self::substitute_expr(callee, type_args)),
                args.iter().map(|a| Self::substitute_expr(a, type_args)).collect(),
                *span,
            ),
            ast::Expr::MemberAccess(obj, member, span) => ast::Expr::MemberAccess(
                Box::new(Self::substitute_expr(obj, type_args)),
                member.clone(),
                *span,
            ),
            ast::Expr::Index(obj, index, span) => ast::Expr::Index(
                Box::new(Self::substitute_expr(obj, type_args)),
                Box::new(Self::substitute_expr(index, type_args)),
                *span,
            ),
            ast::Expr::New(typ, args, span) => ast::Expr::New(
                Self::substitute_type(typ, type_args),
                args.iter().map(|a| Self::substitute_expr(a, type_args)).collect(),
                *span,
            ),
            ast::Expr::This(span) => ast::Expr::This(*span),
            ast::Expr::Super(span) => ast::Expr::Super(*span),
            ast::Expr::OwnedDeref(inner, span) => ast::Expr::OwnedDeref(
                Box::new(Self::substitute_expr(inner, type_args)),
                *span,
            ),
            ast::Expr::RegionAlloc(typ, init, span) => ast::Expr::RegionAlloc(
                Self::substitute_type(typ, type_args),
                Box::new(Self::substitute_expr(init, type_args)),
                *span,
            ),
            ast::Expr::RefExpr(inner, kind, span) => ast::Expr::RefExpr(
                Box::new(Self::substitute_expr(inner, type_args)),
                kind.clone(),
                *span,
            ),
            ast::Expr::UnsafeBlock(block, span) => ast::Expr::UnsafeBlock(
                block.iter().map(|s| Self::substitute_stmt(s, type_args)).collect(),
                *span,
            ),
            ast::Expr::ErrorPropagation(inner, span) => ast::Expr::ErrorPropagation(
                Box::new(Self::substitute_expr(inner, type_args)),
                *span,
            ),
            ast::Expr::Cast(inner, target_type, span) => ast::Expr::Cast(
                Box::new(Self::substitute_expr(inner, type_args)),
                Self::substitute_type(target_type, type_args),
                *span,
            ),
            ast::Expr::StaticCall { class_name, method, args, span } => ast::Expr::StaticCall {
                class_name: class_name.clone(),
                method: method.clone(),
                args: args.iter().map(|a| Self::substitute_expr(a, type_args)).collect(),
                span: *span,
            },
            ast::Expr::Assign(target, value, span) => ast::Expr::Assign(
                Box::new(Self::substitute_expr(target, type_args)),
                Box::new(Self::substitute_expr(value, type_args)),
                *span,
            ),
        }
    }

    fn substitute_stmt(stmt: &ast::Stmt, type_args: &HashMap<String, ast::Type>) -> ast::Stmt {
        match stmt {
            ast::Stmt::VarDecl(var_decl) => {
                ast::Stmt::VarDecl(Self::substitute_var_decl(var_decl, type_args))
            }
            ast::Stmt::ConstDecl(var_decl) => {
                ast::Stmt::ConstDecl(Self::substitute_var_decl(var_decl, type_args))
            }
            ast::Stmt::Expr(expr) => {
                ast::Stmt::Expr(Self::substitute_expr(expr, type_args))
            }
            ast::Stmt::If(if_stmt) => ast::Stmt::If(ast::IfStmt {
                condition: Self::substitute_expr(&if_stmt.condition, type_args),
                then_branch: if_stmt.then_branch.iter().map(|s| Self::substitute_stmt(s, type_args)).collect(),
                else_branch: if_stmt.else_branch.as_ref().map(|b| b.iter().map(|s| Self::substitute_stmt(s, type_args)).collect()),
                span: if_stmt.span,
            }),
            ast::Stmt::While(while_stmt) => ast::Stmt::While(ast::WhileStmt {
                condition: Self::substitute_expr(&while_stmt.condition, type_args),
                body: while_stmt.body.iter().map(|s| Self::substitute_stmt(s, type_args)).collect(),
                span: while_stmt.span,
            }),
            ast::Stmt::For(for_stmt) => ast::Stmt::For(ast::ForStmt {
                var: for_stmt.var.clone(),
                iterable: Self::substitute_expr(&for_stmt.iterable, type_args),
                body: for_stmt.body.iter().map(|s| Self::substitute_stmt(s, type_args)).collect(),
                span: for_stmt.span,
            }),
            ast::Stmt::Return(expr) => {
                ast::Stmt::Return(expr.as_ref().map(|e| Self::substitute_expr(e, type_args)))
            }
            ast::Stmt::Break => ast::Stmt::Break,
            ast::Stmt::Continue => ast::Stmt::Continue,
            ast::Stmt::Switch(switch_stmt) => ast::Stmt::Switch(ast::SwitchStmt {
                expr: Self::substitute_expr(&switch_stmt.expr, type_args),
                cases: switch_stmt.cases.iter().map(|c| ast::Case {
                    pattern: c.pattern.clone(),
                    body: c.body.iter().map(|s| Self::substitute_stmt(s, type_args)).collect(),
                }).collect(),
                default: switch_stmt.default.as_ref().map(|b| b.iter().map(|s| Self::substitute_stmt(s, type_args)).collect()),
                span: switch_stmt.span,
            }),
            ast::Stmt::Block(block) => {
                ast::Stmt::Block(block.iter().map(|s| Self::substitute_stmt(s, type_args)).collect())
            }
        }
    }

    fn substitute_var_decl(var_decl: &ast::VarDecl, type_args: &HashMap<String, ast::Type>) -> ast::VarDecl {
        ast::VarDecl {
            name: var_decl.name.clone(),
            typ: var_decl.typ.as_ref().map(|t| Self::substitute_type(t, type_args)),
            init: var_decl.init.as_ref().map(|e| Self::substitute_expr(e, type_args)),
            mutable: var_decl.mutable,
            span: var_decl.span,
        }
    }

    fn substitute_class_member(member: &ast::ClassMember, type_args: &HashMap<String, ast::Type>) -> ast::ClassMember {
        match member {
            ast::ClassMember::Field(field_decl) => {
                ast::ClassMember::Field(ast::FieldDecl {
                    access: field_decl.access.clone(),
                    name: field_decl.name.clone(),
                    typ: Self::substitute_type(&field_decl.typ, type_args),
                    init: field_decl.init.as_ref().map(|e| Self::substitute_expr(e, type_args)),
                    span: field_decl.span,
                })
            }
            ast::ClassMember::Method(method_decl) => {
                ast::ClassMember::Method(Self::substitute_method_decl(method_decl, type_args))
            }
            ast::ClassMember::Constructor(ctor_decl) => {
                ast::ClassMember::Constructor(Self::substitute_method_decl(ctor_decl, type_args))
            }
        }
    }

    fn substitute_method_decl(method_decl: &ast::MethodDecl, type_args: &HashMap<String, ast::Type>) -> ast::MethodDecl {
        let specialized_params: Vec<ast::Param> = method_decl.params.iter()
            .map(|p| ast::Param {
                name: p.name.clone(),
                typ: Self::substitute_type(&p.typ, type_args),
            })
            .collect();

        let specialized_return_type = method_decl.return_type.as_ref()
            .map(|t| Self::substitute_type(t, type_args));

        let specialized_body: Vec<ast::Stmt> = method_decl.body.iter()
            .map(|s| Self::substitute_stmt(s, type_args))
            .collect();

        ast::MethodDecl {
            access: method_decl.access.clone(),
            name: method_decl.name.clone(),
            type_params: method_decl.type_params.clone(),
            params: specialized_params,
            return_type: specialized_return_type,
            body: specialized_body,
            span: method_decl.span,
        }
    }

    /// Instantiate a generic function with concrete type arguments.
    /// Returns the function index of the specialized function.
    fn instantiate_generic_function(&mut self, base_name: &str, type_args: &[ast::Type]) -> Result<u16, String> {
        let mangled = Self::mangle_name(base_name, type_args);

        // Check cache.
        if let Some(&idx) = self.mono_cache.get(&mangled) {
            return Ok(idx);
        }

        // Find the generic function declaration.
        let generic_idx = *self.generic_function_map.get(base_name)
            .ok_or_else(|| format!("Generic function '{}' not found", base_name))?;
        let generic_fn = self.generic_functions[generic_idx].clone();

        if type_args.len() != generic_fn.type_params.len() {
            return Err(format!(
                "Generic function '{}' expects {} type argument(s), got {}",
                base_name, generic_fn.type_params.len(), type_args.len()
            ));
        }

        // Build type_args map.
        let type_args_map: HashMap<String, ast::Type> = generic_fn.type_params.iter()
            .zip(type_args.iter())
            .map(|(param, arg)| (param.clone(), arg.clone()))
            .collect();

        // Substitute types in the function declaration.
        let specialized_params: Vec<ast::Param> = generic_fn.params.iter()
            .map(|p| ast::Param {
                name: p.name.clone(),
                typ: Self::substitute_type(&p.typ, &type_args_map),
            })
            .collect();

        let specialized_return_type = generic_fn.return_type.as_ref()
            .map(|t| Self::substitute_type(t, &type_args_map));

        let specialized_body: Vec<ast::Stmt> = generic_fn.body.iter()
            .map(|s| Self::substitute_stmt(s, &type_args_map))
            .collect();

        let specialized_fn = ast::FnDecl {
            access: generic_fn.access.clone(),
            name: mangled.clone(),
            type_params: vec![],
            params: specialized_params,
            return_type: specialized_return_type,
            body: specialized_body,
            sugar: generic_fn.sugar,
            span: generic_fn.span,
        };

        // Register the specialized function.
        let fn_idx = self.functions.len() as u16;
        self.function_map.insert(mangled.clone(), fn_idx);
        self.functions.push(FunctionDef {
            name: mangled.clone(),
            arity: specialized_fn.params.len(),
            chunk: Chunk::new(),
            is_method: false,
            is_constructor: false,
            local_count: 0,
        });

        // Compile the specialized function body.
        self.compile_function(&specialized_fn)?;

        // Cache the result.
        self.mono_cache.insert(mangled, fn_idx);

        Ok(fn_idx)
    }

    /// Instantiate a generic class with concrete type arguments.
    /// Returns the class index of the specialized class.
    fn instantiate_generic_class(&mut self, base_name: &str, type_args: &[ast::Type]) -> Result<u16, String> {
        let mangled = Self::mangle_name(base_name, type_args);

        // Check cache.
        if let Some(&idx) = self.mono_cache.get(&mangled) {
            return Ok(idx);
        }

        // Find the generic class declaration.
        let generic_idx = *self.generic_class_map.get(base_name)
            .ok_or_else(|| format!("Generic class '{}' not found", base_name))?;
        let generic_class = self.generic_classes[generic_idx].clone();

        if type_args.len() != generic_class.type_params.len() {
            return Err(format!(
                "Generic class '{}' expects {} type argument(s), got {}",
                base_name, generic_class.type_params.len(), type_args.len()
            ));
        }

        // Build type_args map.
        let type_args_map: HashMap<String, ast::Type> = generic_class.type_params.iter()
            .zip(type_args.iter())
            .map(|(param, arg)| (param.clone(), arg.clone()))
            .collect();

        // Substitute types in class members.
        let specialized_members: Vec<ast::ClassMember> = generic_class.members.iter()
            .map(|m| Self::substitute_class_member(m, &type_args_map))
            .collect();

        let specialized_parent = generic_class.parent.as_ref()
            .map(|t| Self::substitute_type(t, &type_args_map));

        let specialized_ifaces: Vec<ast::Type> = generic_class.ifaces.iter()
            .map(|t| Self::substitute_type(t, &type_args_map))
            .collect();

        let specialized_class = ast::ClassDecl {
            name: mangled.clone(),
            type_params: vec![],
            parent: specialized_parent,
            ifaces: specialized_ifaces,
            members: specialized_members,
            span: generic_class.span,
        };

        // Register the specialized class.
        self.register_class(&specialized_class)?;

        // Compile the specialized class methods.
        self.compile_class_methods(&specialized_class)?;

        // Get the class index.
        let class_idx = *self.class_map.get(&mangled).unwrap();

        // Cache the result.
        self.mono_cache.insert(mangled, class_idx);

        Ok(class_idx)
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
        // Remove locals that belong to the exited scope from the compile-time list.
        // We do NOT emit POP here because the VM pre-allocates all local slots
        // when a function is called, and the RET instruction cleans up the
        // entire frame. Emitting POP would shrink the runtime stack below
        // the pre-allocated area, causing LOAD_LOCAL to fail.
        while self.locals.last().map_or(false, |l| l.depth > self.scope_depth) {
            let _local = self.locals.pop().unwrap();
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
            as u16;

        let saved_class = self.current_class;
        self.current_class = Some(class_idx);

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
                        .get(class_idx as usize)
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
                        .get(class_idx as usize)
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

        self.current_class = saved_class;
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
                self.compile_var_decl(var_decl)?;
            }
            ast::Stmt::ConstDecl(const_decl) => {
                self.compile_var_decl(const_decl)?;
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

    fn compile_var_decl(&mut self, var_decl: &ast::VarDecl) -> Result<(), String> {
        let line = var_decl.span.line;
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
        let line = if_stmt.span.line;
        // compile condition
        self.compile_expr(&if_stmt.condition)?;
        self.emit_opcode(OpCode::JMP_IF_FALSE, line);
        let else_jump_offset = self.current_ip();
        self.emit_i16(0, line); // placeholder

        // compile then_branch
        self.compile_block(&if_stmt.then_branch)?;

        if let Some(ref else_branch) = if_stmt.else_branch {
            // Jump over else branch after then.
            self.emit_opcode(OpCode::JMP, line);
            let end_jump_offset = self.current_ip();
            self.emit_i16(0, line); // placeholder

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
        let line = while_stmt.span.line;
        let loop_start = self.current_ip();

        self.loop_stack.push(LoopInfo {
            start_ip: loop_start,
            break_patches: Vec::new(),
        });

        // compile condition
        self.compile_expr(&while_stmt.condition)?;
        self.emit_opcode(OpCode::JMP_IF_FALSE, line);
        let exit_jump_offset = self.current_ip();
        self.emit_i16(0, line); // placeholder

        // compile body
        self.compile_block(&while_stmt.body)?;

        // Jump back to loop start.
        self.emit_opcode(OpCode::JMP, line);
        let current = self.current_ip() + 2; // after the i16 operand
        let offset = (loop_start as isize - current as isize) as i16;
        self.emit_i16(offset, line);

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
        let line = for_stmt.span.line;
        self.begin_scope();

        // Compile the iterable expression and store it in a local.
        self.compile_expr(&for_stmt.iterable)?;
        let iter_slot = self.declare_local("__iter");
        self.emit_opcode(OpCode::STORE_LOCAL, line);
        self.emit_u8(iter_slot, line);

        // Initialize the index counter to 0.
        self.emit_opcode(OpCode::PUSH_I64, line);
        let bytes = 0i64.to_be_bytes();
        for &b in &bytes {
            self.emit_u8(b, line);
        }
        let idx_slot = self.declare_local("__iter_idx");
        self.emit_opcode(OpCode::STORE_LOCAL, line);
        self.emit_u8(idx_slot, line);

        // Get the length of the iterable and store it.
        self.emit_opcode(OpCode::LOAD_LOCAL, line);
        self.emit_u8(iter_slot, line);
        self.emit_opcode(OpCode::ARRAY_LEN, line);
        let len_slot = self.declare_local("__iter_len");
        self.emit_opcode(OpCode::STORE_LOCAL, line);
        self.emit_u8(len_slot, line);

        let loop_start = self.current_ip();

        self.loop_stack.push(LoopInfo {
            start_ip: loop_start,
            break_patches: Vec::new(),
        });

        // Check: idx < len
        self.emit_opcode(OpCode::LOAD_LOCAL, line);
        self.emit_u8(idx_slot, line);
        self.emit_opcode(OpCode::LOAD_LOCAL, line);
        self.emit_u8(len_slot, line);
        self.emit_opcode(OpCode::LT_I64, line);
        self.emit_opcode(OpCode::JMP_IF_FALSE, line);
        let exit_jump_offset = self.current_ip();
        self.emit_i16(0, line); // placeholder

        // Load the element: __iter[__iter_idx]
        self.emit_opcode(OpCode::LOAD_LOCAL, line);
        self.emit_u8(iter_slot, line);
        self.emit_opcode(OpCode::LOAD_LOCAL, line);
        self.emit_u8(idx_slot, line);
        self.emit_opcode(OpCode::ARRAY_GET, line);

        // Store the element in the loop variable.
        let loop_var_slot = self.declare_local(&for_stmt.var);
        self.emit_opcode(OpCode::STORE_LOCAL, line);
        self.emit_u8(loop_var_slot, line);

        // compile body
        self.compile_block(&for_stmt.body)?;

        // Increment the index: __iter_idx = __iter_idx + 1
        self.emit_opcode(OpCode::LOAD_LOCAL, line);
        self.emit_u8(idx_slot, line);
        self.emit_opcode(OpCode::PUSH_I64, line);
        let one_bytes = 1i64.to_be_bytes();
        for &b in &one_bytes {
            self.emit_u8(b, line);
        }
        self.emit_opcode(OpCode::ADD_I64, line);
        self.emit_opcode(OpCode::STORE_LOCAL, line);
        self.emit_u8(idx_slot, line);

        // Jump back to loop start.
        self.emit_opcode(OpCode::JMP, line);
        let current = self.current_ip() + 2;
        let offset = (loop_start as isize - current as isize) as i16;
        self.emit_i16(offset, line);

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
        let line = switch_stmt.span.line;
        // Compile the subject expression.
        self.compile_expr(&switch_stmt.expr)?;

        let mut end_jumps: Vec<usize> = Vec::new();

        for case in &switch_stmt.cases {
            // DUP the subject for matching.
            self.emit_opcode(OpCode::DUP, line);

            // Compile the pattern match.
            self.compile_pattern_match(&case.pattern)?;

            // If pattern doesn't match, jump to next case.
            self.emit_opcode(OpCode::JMP_IF_FALSE, line);
            let next_case_offset = self.current_ip();
            self.emit_i16(0, line); // placeholder

            // Pattern matched: store extracted fields into local variables.
            if let ast::Pattern::Constructor { bindings, .. } = &case.pattern {
                if !bindings.is_empty() {
                    // Fields are on stack in order (first deepest, last on top).
                    // Store them in reverse order.
                    for binding in bindings.iter().rev() {
                        if binding != "_" {
                            let slot = self.declare_local(binding);
                            self.emit_opcode(OpCode::STORE_LOCAL, line);
                            self.emit_u8(slot, line);
                        } else {
                            // Wildcard: just pop the field.
                            self.emit_opcode(OpCode::POP, line);
                        }
                    }
                }
            }

            // POP the subject.
            self.emit_opcode(OpCode::POP, line);

            // Compile case body.
            self.compile_block(&case.body)?;

            // Jump to end of switch (so we don't fall through).
            self.emit_opcode(OpCode::JMP, line);
            let end_jump_offset = self.current_ip();
            self.emit_i16(0, line); // placeholder
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
            self.emit_opcode(OpCode::POP, line);
            self.compile_block(default_body)?;
        } else {
            // No default: just pop the subject.
            self.emit_opcode(OpCode::POP, line);
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
                self.compile_literal(lit, 0)?;
                let ty = self.infer_literal_type(lit);
                self.emit_eq_opcode(ty, 0);
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
            ast::Expr::Literal(lit, span) => {
                self.compile_literal(lit, span.line)?;
            }
            ast::Expr::Identifier(name, span) => {
                self.compile_identifier(name, span.line)?;
            }
            ast::Expr::Binary(left, op, right, span) => {
                self.compile_binary(left, op, right, span.line)?;
            }
            ast::Expr::Unary(op, operand, span) => {
                self.compile_unary(op, operand, span.line)?;
            }
            ast::Expr::Call(callee, args, span) => {
                self.compile_call(callee, args, span.line)?;
            }
            ast::Expr::MemberAccess(obj, member, span) => {
                self.compile_member_access(obj, member, span.line)?;
            }
            ast::Expr::Index(obj, index, span) => {
                self.compile_expr(obj)?;
                self.compile_expr(index)?;
                self.emit_opcode(OpCode::ARRAY_GET, span.line);
            }
            ast::Expr::New(typ, args, span) => {
                self.compile_new(typ, args, span.line)?;
            }
            ast::Expr::This(span) => {
                // In methods, "this" is always slot 0.
                self.emit_opcode(OpCode::LOAD_LOCAL, span.line);
                self.emit_u8(0, span.line);
            }
            ast::Expr::Super(span) => {
                // "super" resolves to "this" for method dispatch.
                self.emit_opcode(OpCode::LOAD_LOCAL, span.line);
                self.emit_u8(0, span.line);
            }
            ast::Expr::OwnedDeref(inner, span) => {
                self.compile_expr(inner)?;
                self.emit_opcode(OpCode::UNBOX_VALUE, span.line);
            }
            ast::Expr::RegionAlloc(_typ, init, span) => {
                self.compile_expr(init)?;
                self.emit_opcode(OpCode::REGION_ALLOC, span.line);
            }
            ast::Expr::RefExpr(inner, kind, span) => {
                self.compile_expr(inner)?;
                match kind {
                    ast::RefKind::Immutable => self.emit_opcode(OpCode::REF_IMMUTABLE, span.line),
                    ast::RefKind::Mutable => self.emit_opcode(OpCode::REF_MUTABLE, span.line),
                }
            }
            ast::Expr::UnsafeBlock(block, _span) => {
                // Compile as a regular block.
                self.begin_scope();
                self.compile_block(block)?;
                self.end_scope();
            }
            ast::Expr::ErrorPropagation(inner, span) => {
                self.compile_expr(inner)?;
                self.emit_opcode(OpCode::UNWRAP_OR_PROPAGATE, span.line);
            }
            ast::Expr::Cast(inner, target_type, span) => {
                self.compile_expr(inner)?;
                let cast_target = self.type_to_cast_target(target_type);
                self.emit_opcode(OpCode::CAST, span.line);
                self.emit_u8(cast_target as u8, span.line);
            }
            ast::Expr::StaticCall {
                class_name,
                method,
                args,
                span,
            } => {
                self.compile_static_call(class_name, method, args, span.line)?;
            }
            ast::Expr::Assign(target, value, span) => {
                self.compile_assign(target, value, span.line)?;
            }
        }
        Ok(())
    }

    fn compile_literal(&mut self, lit: &ast::Literal, line: u32) -> Result<(), String> {
        match lit {
            ast::Literal::Int(v) => {
                self.emit_opcode(OpCode::PUSH_I64, line);
                let bytes = (*v as i64).to_be_bytes();
                for &b in &bytes {
                    self.emit_u8(b, line);
                }
            }
            ast::Literal::Float(v) => {
                self.emit_opcode(OpCode::PUSH_F64, line);
                let bytes = (*v as f64).to_be_bytes();
                for &b in &bytes {
                    self.emit_u8(b, line);
                }
            }
            ast::Literal::Bool(b) => {
                self.emit_opcode(OpCode::PUSH_BOOL, line);
                self.emit_u8(if *b { 1 } else { 0 }, line);
            }
            ast::Literal::Char(c) => {
                self.emit_opcode(OpCode::PUSH_CHAR, line);
                let bytes = (*c as u32).to_be_bytes();
                for &b in &bytes {
                    self.emit_u8(b, line);
                }
            }
            ast::Literal::String(s) => {
                let idx = self.intern_string(s);
                self.emit_opcode(OpCode::PUSH_STRING, line);
                self.emit_u16(idx, line);
            }
            ast::Literal::Null => {
                self.emit_opcode(OpCode::PUSH_NULL, line);
            }
        }
        Ok(())
    }

    fn compile_identifier(&mut self, name: &str, line: u32) -> Result<(), String> {
        // Check locals first.
        if let Some(slot) = self.resolve_local(name) {
            self.emit_opcode(OpCode::LOAD_LOCAL, line);
            self.emit_u8(slot, line);
            return Ok(());
        }

        // Check if it's a known function.
        if let Some(&fn_idx) = self.function_map.get(name) {
            self.emit_opcode(OpCode::PUSH_VOID, line); // placeholder – function refs not yet in value
            let _ = fn_idx;
            // For now, function calls are handled directly in compile_call.
            // If we reach here, it's a bare function reference.
            return Ok(());
        }

        // Check the imported symbol table.
        if let Some(symbol) = self.symbol_table.get(name).cloned() {
            match symbol {
                Symbol::Function(fn_idx) => {
                    // Imported function reference – handled in compile_call.
                    self.emit_opcode(OpCode::PUSH_VOID, line);
                    let _ = fn_idx;
                    return Ok(());
                }
                Symbol::Class(class_idx) => {
                    // Imported class reference – handled in compile_new.
                    let _ = class_idx;
                    self.emit_opcode(OpCode::PUSH_NULL, line);
                    return Ok(());
                }
                Symbol::Enum(enum_idx) => {
                    // Imported enum reference.
                    let _ = enum_idx;
                    self.emit_opcode(OpCode::PUSH_NULL, line);
                    return Ok(());
                }
            }
        }

        // Check if it's an enum variant (bare reference without call).
        if self.variant_map.contains_key(name) {
            // This is a partial application – the variant will be called later.
            // For now, emit a placeholder.
            self.emit_opcode(OpCode::PUSH_NULL, line);
            return Ok(());
        }

        // Unknown identifier – could be a global or builtin.
        // Emit a LOAD_LOCAL with slot 0 as a fallback; the VM should handle this.
        // In practice, the analyzer should catch undefined variables.
        self.emit_opcode(OpCode::LOAD_LOCAL, line);
        self.emit_u8(0, line);
        Ok(())
    }

    fn compile_binary(
        &mut self,
        left: &ast::Expr,
        op: &ast::Operator,
        right: &ast::Expr,
        line: u32,
    ) -> Result<(), String> {
        // Short-circuit for And/Or.
        match op {
            ast::Operator::And => {
                // And: compile left, JMP_IF_FALSE(skip), compile right, JMP(end),
                //      (skip:) PUSH_BOOL(false), (end:)
                self.compile_expr(left)?;
                self.emit_opcode(OpCode::JMP_IF_FALSE, line);
                let skip_offset = self.current_ip();
                self.emit_i16(0, line); // placeholder

                self.compile_expr(right)?;
                self.emit_opcode(OpCode::JMP, line);
                let end_offset = self.current_ip();
                self.emit_i16(0, line); // placeholder

                // skip: PUSH_BOOL(false)
                let skip_ip = self.current_ip();
                self.patch_i16_at(skip_offset, (skip_ip - (skip_offset + 2)) as i16);
                self.emit_opcode(OpCode::PUSH_BOOL, line);
                self.emit_u8(0, line);

                // end:
                let end_ip = self.current_ip();
                self.patch_i16_at(end_offset, (end_ip - (end_offset + 2)) as i16);
                return Ok(());
            }
            ast::Operator::Or => {
                // Or: compile left, JMP_IF_TRUE(skip), compile right, JMP(end),
                //     (skip:) PUSH_BOOL(true), (end:)
                self.compile_expr(left)?;
                self.emit_opcode(OpCode::JMP_IF_TRUE, line);
                let skip_offset = self.current_ip();
                self.emit_i16(0, line); // placeholder

                self.compile_expr(right)?;
                self.emit_opcode(OpCode::JMP, line);
                let end_offset = self.current_ip();
                self.emit_i16(0, line); // placeholder

                // skip: PUSH_BOOL(true)
                let skip_ip = self.current_ip();
                self.patch_i16_at(skip_offset, (skip_ip - (skip_offset + 2)) as i16);
                self.emit_opcode(OpCode::PUSH_BOOL, line);
                self.emit_u8(1, line);

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
                        self.emit_opcode(OpCode::STR_CONCAT, line);
                    } else if left_type == InferredType::String {
                        // String + non-String
                        self.emit_opcode(OpCode::STR_CONCAT_RIGHT, line);
                    } else if right_type == InferredType::String {
                        // non-String + String
                        self.emit_opcode(OpCode::STR_CONCAT_LEFT, line);
                    } else {
                        // Both non-String but result is String (e.g., toString calls)
                        self.emit_opcode(OpCode::STR_CONCAT, line);
                    }
                } else {
                    self.emit_add_opcode(result_type, line);
                }
            }
            ast::Operator::Sub => self.emit_sub_opcode(result_type, line),
            ast::Operator::Mul => self.emit_mul_opcode(result_type, line),
            ast::Operator::Div => self.emit_div_opcode(result_type, line),
            ast::Operator::Mod => self.emit_mod_opcode(result_type, line),
            ast::Operator::Eq => self.emit_eq_opcode(result_type, line),
            ast::Operator::Ne => self.emit_ne_opcode(result_type, line),
            ast::Operator::Lt => self.emit_lt_opcode(result_type, line),
            ast::Operator::Gt => self.emit_gt_opcode(result_type, line),
            ast::Operator::Le => self.emit_le_opcode(result_type, line),
            ast::Operator::Ge => self.emit_ge_opcode(result_type, line),
            ast::Operator::BitAnd => self.emit_bitand_opcode(result_type, line),
            ast::Operator::BitOr => self.emit_bitor_opcode(result_type, line),
            ast::Operator::BitXor => self.emit_bitxor_opcode(result_type, line),
            ast::Operator::BitShl => self.emit_shl_opcode(result_type, line),
            ast::Operator::BitShr => self.emit_shr_opcode(result_type, line),
            ast::Operator::And | ast::Operator::Or => {
                unreachable!("And/Or handled above")
            }
        }

        Ok(())
    }

    fn compile_unary(&mut self, op: &ast::UnOp, operand: &ast::Expr, line: u32) -> Result<(), String> {
        self.compile_expr(operand)?;
        let ty = self.infer_expr_type(operand);
        match op {
            ast::UnOp::Neg => self.emit_neg_opcode(ty, line),
            ast::UnOp::Not => {
                self.emit_opcode(OpCode::NOT, line);
            }
            ast::UnOp::BitNot => self.emit_bitnot_opcode(ty, line),
        }
        Ok(())
    }

    fn compile_call(&mut self, callee: &ast::Expr, args: &[ast::Expr], line: u32) -> Result<(), String> {
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

        // Special case: Identifier("Ok") → RESULT_OK
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
            if let Some((enum_name, _variant_idx)) = self.variant_map.get(name) {
                let enum_idx = *self.enum_map.get(enum_name).unwrap() as u16;
                let variant_name_idx = self.intern_string(name);
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::ENUM_NEW, line);
                self.emit_u16(enum_idx, line);
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

            // Check the imported symbol table for functions.
            if let Some(Symbol::Function(fn_idx)) = self.symbol_table.get(name).cloned() {
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::CALL, line);
                self.emit_u16(fn_idx, line);
                self.emit_u8(args.len() as u8, line);
                return Ok(());
            }

            // Check if it's a generic function (needs type arguments).
            if self.generic_function_map.contains_key(name) {
                return Err(format!(
                    "Cannot call generic function '{}' without type arguments",
                    name
                ));
            }
        }

        // Special case: MemberAccess callee → method call.
        if let ast::Expr::MemberAccess(ref obj, ref method, _) = *callee {
            // Check for static calls like io.println, Integer.toString, etc.
            if let ast::Expr::Identifier(ref obj_name, _) = **obj {
                if self.is_builtin_object(obj_name) {
                    self.compile_static_call(obj_name, method, args, line)?;
                    return Ok(());
                }
                // Check if obj_name is a class name.
                if self.class_map.contains_key(obj_name) {
                    self.compile_static_call(obj_name, method, args, line)?;
                    return Ok(());
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

        // General case: compile callee, then args, then CALL.
        self.compile_expr(callee)?;
        for arg in args {
            self.compile_expr(arg)?;
        }
        self.emit_opcode(OpCode::CALL, line);
        // Use function index 0 as placeholder; the VM will use the callee on the stack.
        self.emit_u16(0, line);
        self.emit_u8(args.len() as u8, line);

        Ok(())
    }

    fn compile_member_access(&mut self, obj: &ast::Expr, member: &str, line: u32) -> Result<(), String> {
        // Check for static member access patterns.
        if let ast::Expr::Identifier(ref obj_name, _) = *obj {
            // io.println etc. are handled in compile_call via MemberAccess callee.
            // Here we handle bare member access (not a call).
            if self.is_builtin_object(obj_name) {
                // This is a reference to a builtin object's member.
                // It will typically be used in a call context, which is handled above.
                self.emit_opcode(OpCode::PUSH_NULL, line);
                return Ok(());
            }
        }

        // Regular field access: compile obj, then GET_FIELD.
        self.compile_expr(obj)?;
        let field_idx = self.intern_string(member);
        self.emit_opcode(OpCode::GET_FIELD, line);
        self.emit_u16(field_idx, line);

        Ok(())
    }

    fn compile_new(&mut self, typ: &ast::Type, args: &[ast::Expr], line: u32) -> Result<(), String> {
        let class_name = typ.name();
        let type_params = typ.params();

        // Handle built-in types that aren't user-defined classes
        match class_name {
            "ArrayList" | "HashMap" => {
                let class_idx = self.get_or_create_builtin_class(class_name, type_params);
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_opcode(OpCode::NEW, line);
                self.emit_u16(class_idx, line);
                return Ok(());
            }
            _ => {}
        }

        // Check if it's a generic class instantiation.
        if !type_params.is_empty() && self.generic_class_map.contains_key(class_name) {
            let class_idx = self.instantiate_generic_class(class_name, type_params)?;
            for arg in args {
                self.compile_expr(arg)?;
            }
            self.emit_opcode(OpCode::NEW, line);
            self.emit_u16(class_idx, line);
            return Ok(());
        }

        let class_idx = if let Some(&idx) = self.class_map.get(class_name) {
            idx
        } else if let Some(Symbol::Class(idx)) = self.symbol_table.get(class_name) {
            *idx
        } else {
            return Err(format!("Unknown class '{}' in new expression", class_name));
        };

        // Compile arguments.
        for arg in args {
            self.compile_expr(arg)?;
        }

        self.emit_opcode(OpCode::NEW, line);
        self.emit_u16(class_idx, line);

        // If the class has a constructor, the VM will call it after allocation.
        // The constructor call is implicit in the NEW opcode.

        Ok(())
    }

    /// Get or create a built-in pseudo-class (ArrayList, HashMap, etc.)
    /// Uses monomorphization naming: ArrayList<int> → ArrayList__int
    fn get_or_create_builtin_class(&mut self, name: &str, type_args: &[ast::Type]) -> u16 {
        let mangled = Self::mangle_name(name, type_args);

        if let Some(&idx) = self.class_map.get(&mangled) {
            return idx;
        }
        let idx = self.classes.len() as u16;
        let class_def = ClassDef {
            name: mangled.clone(),
            parent: None,
            fields: Vec::new(),
            methods: HashMap::new(),
            constructor: None,
            field_inits: Vec::new(),
        };
        self.classes.push(class_def);
        self.class_map.insert(mangled.clone(), idx);

        // Also cache in mono_cache.
        self.mono_cache.insert(mangled, idx);

        idx
    }

    fn compile_static_call(
        &mut self,
        class_name: &str,
        method: &str,
        args: &[ast::Expr],
        line: u32,
    ) -> Result<(), String> {
        // Compile arguments.
        for arg in args {
            self.compile_expr(arg)?;
        }

        let class_idx = self.intern_string(class_name);
        let method_idx = self.intern_string(method);

        self.emit_opcode(OpCode::STATIC_CALL, line);
        self.emit_u16(class_idx, line);
        self.emit_u16(method_idx, line);
        self.emit_u8(args.len() as u8, line);

        Ok(())
    }

    fn compile_assign(&mut self, target: &ast::Expr, value: &ast::Expr, line: u32) -> Result<(), String> {
        self.compile_expr(value)?;

        match target {
            ast::Expr::Identifier(name, _) => {
                if let Some(slot) = self.resolve_local(name) {
                    // DUP the value so it remains on the stack after STORE_LOCAL
                    // consumes the copy. The caller (compile_stmt) will emit POP
                    // for expression statements, which pops this DUP'd value.
                    self.emit_opcode(OpCode::DUP, line);
                    self.emit_opcode(OpCode::STORE_LOCAL, line);
                    self.emit_u8(slot, line);
                } else {
                    return Err(format!("Cannot assign to undefined variable '{}'", name));
                }
            }
            ast::Expr::MemberAccess(obj, member, _) => {
                self.compile_expr(obj)?;
                let field_idx = self.intern_string(member);
                self.emit_opcode(OpCode::SET_FIELD, line);
                self.emit_u16(field_idx, line);
            }
            ast::Expr::Index(obj, index, _) => {
                self.compile_expr(obj)?;
                self.compile_expr(index)?;
                self.emit_opcode(OpCode::ARRAY_SET, line);
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
            ast::Expr::Literal(lit, _) => self.infer_literal_type(lit),
            ast::Expr::Identifier(name, _) => self.infer_identifier_type(name),
            ast::Expr::Binary(left, op, right, _) => {
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
            ast::Expr::Unary(op, operand, _) => {
                let ot = self.infer_expr_type(operand);
                match op {
                    ast::UnOp::Neg => ot,
                    ast::UnOp::Not => InferredType::Bool,
                    ast::UnOp::BitNot => ot,
                }
            }
            ast::Expr::Call(callee, _args, _) => {
                // Check for toString calls on builtin objects
                if let ast::Expr::MemberAccess(_, method, _) = callee.as_ref() {
                    if method == "toString" {
                        return InferredType::String;
                    }
                }
                if let ast::Expr::Identifier(name, _) = callee.as_ref() {
                    if name == "Ok" || name == "Err" {
                        return InferredType::Unknown; // Result type
                    }
                }
                InferredType::Unknown
            }
            ast::Expr::MemberAccess(_, _, _) => InferredType::Unknown,
            ast::Expr::Index(_, _, _) => InferredType::Unknown,
            ast::Expr::New(_, _, _) => InferredType::Unknown,
            ast::Expr::This(_) => InferredType::Unknown,
            ast::Expr::Super(_) => InferredType::Unknown,
            ast::Expr::OwnedDeref(inner, _) => self.infer_expr_type(inner),
            ast::Expr::RegionAlloc(_, _, _) => InferredType::Unknown,
            ast::Expr::RefExpr(_, _, _) => InferredType::Unknown,
            ast::Expr::UnsafeBlock(_, _) => InferredType::Unknown,
            ast::Expr::ErrorPropagation(_, _) => InferredType::Unknown,
            ast::Expr::Cast(_, target_type, _) => self.type_to_inferred(target_type),
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
            ast::Expr::Assign(_, _, _) => InferredType::Unknown,
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

    fn emit_add_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::ADD_I32,
                InferredType::F32 => OpCode::ADD_F32,
                InferredType::F64 => OpCode::ADD_F64,
                _ => OpCode::ADD_I64, // default for I64, I128, U128, Unknown
            },
            line,
        );
    }

    fn emit_sub_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::SUB_I32,
                InferredType::F32 => OpCode::SUB_F32,
                InferredType::F64 => OpCode::SUB_F64,
                _ => OpCode::SUB_I64,
            },
            line,
        );
    }

    fn emit_mul_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::MUL_I32,
                InferredType::F32 => OpCode::MUL_F32,
                InferredType::F64 => OpCode::MUL_F64,
                _ => OpCode::MUL_I64,
            },
            line,
        );
    }

    fn emit_div_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::DIV_I32,
                InferredType::F32 => OpCode::DIV_F32,
                InferredType::F64 => OpCode::DIV_F64,
                _ => OpCode::DIV_I64,
            },
            line,
        );
    }

    fn emit_mod_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::MOD_I32,
                InferredType::F32 => OpCode::MOD_F32,
                InferredType::F64 => OpCode::MOD_F64,
                _ => OpCode::MOD_I64,
            },
            line,
        );
    }

    fn emit_neg_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::NEG_I32,
                InferredType::F32 => OpCode::NEG_F32,
                InferredType::F64 => OpCode::NEG_F64,
                _ => OpCode::NEG_I64,
            },
            line,
        );
    }

    fn emit_bitand_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::BITAND_I32,
                _ => OpCode::BITAND_I64,
            },
            line,
        );
    }

    fn emit_bitor_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::BITOR_I32,
                _ => OpCode::BITOR_I64,
            },
            line,
        );
    }

    fn emit_bitxor_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::BITXOR_I32,
                _ => OpCode::BITXOR_I64,
            },
            line,
        );
    }

    fn emit_shl_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::SHL_I32,
                _ => OpCode::SHL_I64,
            },
            line,
        );
    }

    fn emit_shr_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::SHR_I32,
                _ => OpCode::SHR_I64,
            },
            line,
        );
    }

    fn emit_bitnot_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::BITNOT_I32,
                _ => OpCode::BITNOT_I64,
            },
            line,
        );
    }

    fn emit_eq_opcode(&mut self, ty: InferredType, line: u32) {
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
            line,
        );
    }

    fn emit_ne_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::NE_I32,
                InferredType::F32 => OpCode::NE_F32,
                InferredType::F64 => OpCode::NE_F64,
                _ => OpCode::NE_I64,
            },
            line,
        );
    }

    fn emit_lt_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::LT_I32,
                InferredType::F32 => OpCode::LT_F32,
                InferredType::F64 => OpCode::LT_F64,
                _ => OpCode::LT_I64,
            },
            line,
        );
    }

    fn emit_gt_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::GT_I32,
                InferredType::F32 => OpCode::GT_F32,
                InferredType::F64 => OpCode::GT_F64,
                _ => OpCode::GT_I64,
            },
            line,
        );
    }

    fn emit_le_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::LE_I32,
                InferredType::F32 => OpCode::LE_F32,
                InferredType::F64 => OpCode::LE_F64,
                _ => OpCode::LE_I64,
            },
            line,
        );
    }

    fn emit_ge_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::GE_I32,
                InferredType::F32 => OpCode::GE_F32,
                InferredType::F64 => OpCode::GE_F64,
                _ => OpCode::GE_I64,
            },
            line,
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
        let compiled = compile_program(vec![ast::Declaration::VarDecl(ast::VarDecl {
            name: "x".to_string(),
            typ: None,
            init: Some(ast::Expr::Binary(
                Box::new(ast::Expr::Literal(ast::Literal::Int(1), su())),
                ast::Operator::Add,
                Box::new(ast::Expr::Literal(ast::Literal::Int(2), su())),
                su(),
            )),
            mutable: false,
            span: su(),
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
            type_params: vec!["T".to_string()],
            params: vec![ast::Param {
                name: "x".to_string(),
                typ: ast::Type::simple("T"),
            }],
            return_type: Some(ast::Type::simple("T")),
            body: vec![ast::Stmt::Return(Some(ast::Expr::Identifier("x".to_string(), su())))],
            sugar: false,
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
            type_params: vec!["T".to_string()],
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
            type_params: vec!["T".to_string()],
            params: vec![ast::Param {
                name: "x".to_string(),
                typ: ast::Type::simple("T"),
            }],
            return_type: Some(ast::Type::simple("T")),
            body: vec![ast::Stmt::Return(Some(ast::Expr::Identifier("x".to_string(), su())))],
            sugar: false,
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
        let mut resolver = ModuleResolver::new();
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
        let mut resolver = ModuleResolver::new();
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
        let mut resolver = ModuleResolver::new();
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
        let mut resolver = ModuleResolver::new();
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
                    span: su(),
                }),
            ],
        };

        // Register the module's declarations.
        compiler.register_module_declarations(&module_program, "mymod").unwrap();

        // The public function should be in function_map with mangled name.
        assert!(compiler.function_map.contains_key("mymod.visible_fn"),
            "public function should be registered with mangled name");

        // The private function should NOT be in function_map.
        assert!(!compiler.function_map.contains_key("mymod.hidden_fn"),
            "private function should NOT be registered with mangled name");
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
                span: su(),
            }],
            declarations: vec![],
        };

        let mut compiler = Compiler::new();
        let result = compiler.compile_with_modules(&main_program, &root);
        // Circular imports should be detected and reported as an error.
        assert!(result.is_err(), "circular import should be detected");
        let err = result.err().unwrap();
        assert!(err.contains("Circular import"), "error should mention circular import: {}", err);

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
                path: vec!["glob_import_test".to_string(), "utils".to_string(), "*".to_string()],
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

    // -- test_module_resolve_public_method ----------------------------------------

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
}
