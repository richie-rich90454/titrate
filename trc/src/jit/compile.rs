//! Feature-gated JIT compilation tier backed by LLVM's ORC/MCJIT execution
//! engine (inkwell's `ExecutionEngine`).
//!
//! The frontend hands the JIT textual LLVM IR for a hot function; this module
//! verifies it, loads it into a long-lived execution session, resolves the
//! zero-argument `i64` entry symbol, and exposes a typed call through that
//! address. Every failure is reported as a descriptive error so the caller can
//! keep interpreting the function instead.

use std::collections::HashMap;
use std::mem::{size_of, transmute};
use std::sync::{Arc, Mutex};

use inkwell::context::Context;
use inkwell::execution_engine::ExecutionEngine;
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::module::Module;
use inkwell::targets::{InitializationConfig, Target};
use inkwell::OptimizationLevel;

use crate::jit::orc::CompileHook;
use crate::jit::{NativeCode, UNAVAILABLE_ERROR};

/// Signature of every JIT entry stub. The frontend lowers a hot function to a
/// wrapper with this ABI and [`entry_symbol`] as its symbol.
pub type JitEntryFn = unsafe extern "C" fn() -> u64;

/// Whether this target can host the inkwell JIT session.
pub const fn jit_supported() -> bool {
    !cfg!(target_os = "wasm32") && (cfg!(target_arch = "x86") || cfg!(target_arch = "x86_64"))
}

/// LLVM symbol used as the entry point for `function`.
///
/// Invalid identifier characters are folded into underscores so the symbol can
/// be named directly in textual IR.
pub fn entry_symbol(function: &str) -> String {
    let mut symbol = String::with_capacity(function.len() + 10);
    for ch in function.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' {
            symbol.push(ch);
        } else {
            symbol.push('_');
        }
    }
    symbol.push_str("_jit_entry");
    symbol
}

/// Thread-safe registry of textual LLVM IR keyed by Titrate function name.
#[derive(Debug, Default)]
pub struct IrRegistry {
    modules: Mutex<HashMap<String, String>>,
}

impl IrRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register (or replace) the IR used to compile `function`.
    pub fn insert(&self, function: &str, ir: &str) {
        let mut modules = self.modules.lock().expect("JIT IR registry lock poisoned");
        modules.insert(function.to_string(), ir.to_string());
    }

    /// IR registered for `function`, if any.
    pub fn get(&self, function: &str) -> Option<String> {
        self.modules
            .lock()
            .expect("JIT IR registry lock poisoned")
            .get(function)
            .cloned()
    }

    /// Number of registered IR modules.
    pub fn len(&self) -> usize {
        self.modules.lock().expect("JIT IR registry lock poisoned").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Parse textual IR into a module belonging to `context` without executing it.
fn parse_module<'ctx>(context: &'ctx Context, ir: &str, function: &str) -> Result<Module<'ctx>, String> {
    let mut bytes = ir.as_bytes().to_vec();
    bytes.push(0);
    let buffer = MemoryBuffer::create_from_memory_range_copy(&bytes, "titrate_jit_ir");
    context
        .create_module_from_ir(buffer)
        .map_err(|e| format!("failed to parse JIT IR for '{}': {}", function, e.to_string()))
}

/// Check that `symbol` is a zero-argument function returning `i64`. This runs
/// after LLVM verification and before the module is handed to the engine, so a
/// wrong ABI is rejected with a descriptive error instead of an invalid call.
fn entry_signature_is_valid<'ctx>(context: &'ctx Context, module: &Module<'ctx>, symbol: &str) -> bool {
    let Some(function) = module.get_function(symbol) else {
        return false;
    };
    let ty = function.get_type();
    if ty.count_param_types() != 0 {
        return false;
    }
    match ty.get_return_type() {
        Some(ret) if ret.is_int_type() => {
            ret.into_int_type().get_bit_width() == context.i64_type().get_bit_width()
        }
        _ => false,
    }
}

struct JitSession<'ctx> {
    context: &'ctx Context,
    engine: ExecutionEngine<'ctx>,
}

impl JitSession<'static> {
    fn create() -> Result<Self, String> {
        Target::initialize_native(&InitializationConfig::default())
            .map_err(|e| format!("failed to initialize native JIT target: {}", e))?;
        // The context must outlive the engine, so both are process-lifetime.
        // Creating one is a one-time allocation; all later compiles reuse it.
        let context: &'static Context = Box::leak(Box::new(Context::create()));
        let module = context.create_module("titrate_jit_runtime");
        let engine = module
            .create_jit_execution_engine(OptimizationLevel::Default)
            .map_err(|e| format!("failed to create ORC JIT execution engine: {}", e))?;
        Ok(Self { context, engine })
    }

    fn compile(&self, function: &str, symbol: &str, ir: &str) -> Result<NativeCode, String> {
        let module = parse_module(self.context, ir, function)?;
        module
            .verify()
            .map_err(|e| format!("JIT module for '{}' failed verification: {}", function, e.to_string()))?;
        if module.get_function(symbol).is_none() {
            return Err(format!("JIT module for '{}' has no '{}' entry", function, symbol));
        }
        if !entry_signature_is_valid(self.context, &module, symbol) {
            return Err(format!(
                "JIT entry '{}' for '{}' must have signature i64 ()",
                symbol, function
            ));
        }
        // The engine takes ownership of the module, so its generated code stays
        // valid for as long as this session lives.
        self.engine
            .add_module(&module)
            .map_err(|_| format!("JIT engine could not load module for '{}'", function))?;
        let address = self
            .engine
            .get_function_address(symbol)
            .map_err(|e| format!("failed to resolve JIT entry '{}' for '{}': {}", symbol, function, e))?;
        if address == 0 {
            return Err(format!("JIT entry '{}' for '{}' resolved to address 0", symbol, function));
        }
        Ok(NativeCode { address, size: 0 })
    }
}

/// LLVM objects are neither `Send` nor `Sync`. All access to the session goes
/// through `inner`, and entry addresses are only called while the owning
/// `JitEngine` is alive.
struct SharedSession {
    inner: Mutex<Option<JitSession<'static>>>,
}

// SAFETY: the mutex serializes every use of the LLVM context and engine.
unsafe impl Send for SharedSession {}
unsafe impl Sync for SharedSession {}

/// Compilation hook that turns registered IR into executable code.
pub struct OrcJitCompileHook {
    modules: Arc<IrRegistry>,
    session: SharedSession,
}

impl OrcJitCompileHook {
    pub fn new(modules: Arc<IrRegistry>) -> Self {
        Self {
            modules,
            session: SharedSession { inner: Mutex::new(None) },
        }
    }
}

impl CompileHook for OrcJitCompileHook {
    fn compile(&self, function: &str) -> Result<NativeCode, String> {
        if !jit_supported() {
            return Err(UNAVAILABLE_ERROR.to_string());
        }
        let symbol = entry_symbol(function);
        let ir = self
            .modules
            .get(function)
            .ok_or_else(|| format!("no LLVM IR registered for JIT function '{}'", function))?;
        let mut guard = self
            .session
            .inner
            .lock()
            .map_err(|_| "JIT session lock poisoned".to_string())?;
        if guard.is_none() {
            *guard = Some(JitSession::create()?);
        }
        let session = guard
            .as_ref()
            .ok_or_else(|| "JIT session was not initialized".to_string())?;
        session.compile(function, &symbol, &ir)
    }
}

/// Invoke a resolved entry through its typed function pointer.
///
/// The caller must keep the JIT session that produced `address` alive for the
/// duration of this call; `JitEngine::invoke` does that by borrowing itself.
pub(crate) fn call_entry(address: usize) -> Result<u64, String> {
    if address == 0 {
        return Err("cannot invoke JIT entry at address 0".to_string());
    }
    if size_of::<usize>() != size_of::<JitEntryFn>() {
        return Err(format!(
            "JIT function pointer width {} does not match address width {}",
            size_of::<JitEntryFn>(),
            size_of::<usize>()
        ));
    }
    let entry: JitEntryFn = unsafe { transmute(address) };
    Ok(unsafe { entry() })
}

#[cfg(test)]
mod tests {
    use super::*;

    const IR: &str = "define i64 @work_jit_entry() {\nentry:\n  ret i64 42\n}\n";

    #[test]
    fn entry_symbol_is_stable_and_sanitized() {
        assert_eq!(entry_symbol("work"), "work_jit_entry");
        assert_eq!(entry_symbol("my-module.fn"), "my_module_fn_jit_entry");
    }

    #[test]
    fn ir_can_be_parsed_and_verified_without_an_execution_engine() {
        let context = Context::create();
        let module = parse_module(&context, IR, "work").expect("valid IR should parse");
        assert!(module.verify().is_ok());
        assert!(module.get_function("work_jit_entry").is_some());
    }

    #[test]
    fn invalid_ir_is_reported_with_the_function_name() {
        let context = Context::create();
        let err = parse_module(&context, "not llvm ir", "work").expect_err("invalid IR should fail");
        assert!(err.contains("work"), "error should name the function: {}", err);
    }

    #[test]
    fn entry_signature_is_checked_without_an_execution_engine() {
        let context = Context::create();

        let module = parse_module(&context, IR, "work").expect("valid IR should parse");
        assert!(entry_signature_is_valid(&context, &module, "work_jit_entry"));

        let wrong_return = "define i32 @work_jit_entry() {\nentry:\n  ret i32 42\n}\n";
        let module = parse_module(&context, wrong_return, "work").expect("valid IR should parse");
        assert!(!entry_signature_is_valid(&context, &module, "work_jit_entry"));

        let wrong_args = "define i64 @work_jit_entry(i64 %x) {\nentry:\n  ret i64 %x\n}\n";
        let module = parse_module(&context, wrong_args, "work").expect("valid IR should parse");
        assert!(!entry_signature_is_valid(&context, &module, "work_jit_entry"));

        assert!(!entry_signature_is_valid(&context, &module, "absent_jit_entry"));
    }

    #[test]
    fn registry_counts_registered_modules() {
        let registry = IrRegistry::new();
        assert!(registry.is_empty());
        registry.insert("work", IR);
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.get("work").as_deref(), Some(IR));
        registry.insert("work", IR);
        assert_eq!(registry.len(), 1, "re-registration replaces rather than adds");
    }

    #[test]
    fn missing_ir_fails_before_creating_a_session() {
        let hook = OrcJitCompileHook::new(Arc::new(IrRegistry::new()));
        let err = hook.compile("work").expect_err("unregistered function should fail");
        if jit_supported() {
            assert_eq!(err, "no LLVM IR registered for JIT function 'work'");
        } else {
            assert_eq!(err, UNAVAILABLE_ERROR);
        }
    }

    #[test]
    fn null_entry_address_is_rejected_without_calling_it() {
        assert_eq!(
            call_entry(0).expect_err("null address must not be invoked"),
            "cannot invoke JIT entry at address 0"
        );
    }
}
