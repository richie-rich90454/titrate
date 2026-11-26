// Titrate Alpha 0.2 - JIT tier foundation

pub mod profile;
pub mod orc;
#[cfg(feature = "jit")]
pub mod compile;

use std::collections::HashMap;
#[cfg(feature = "jit")]
use std::sync::Arc;

#[cfg(feature = "jit")]
use self::compile::{call_entry, IrRegistry, OrcJitCompileHook, jit_supported};
#[cfg(feature = "jit")]
use self::orc::{CompilationState, CompileQueue};
use self::profile::HotFunctionProfiler;

/// Diagnostic used when no JIT backend exists for this build or target.
pub const UNAVAILABLE_ERROR: &str = "JIT compilation not available in this build";

/// A registered piece of executable machine code for one function.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeCode {
    /// Address of the executable code.
    pub address: usize,
    /// Size of the executable code in bytes (zero when unknown).
    pub size: usize,
}

/// JIT tier facade. Profiling always works. With the `jit` feature on a
/// supported target, hot functions are queued and compiled into an ORC-backed
/// execution session; otherwise `compile` reports that limitation explicitly.
#[derive(Debug)]
pub struct JitEngine {
    profiler: HotFunctionProfiler,
    native_code: HashMap<String, NativeCode>,
    #[cfg(feature = "jit")]
    queue: CompileQueue,
    #[cfg(feature = "jit")]
    modules: Arc<IrRegistry>,
}

impl JitEngine {
    pub fn new() -> Self {
        Self::with_profiler(HotFunctionProfiler::new())
    }

    pub fn with_profiler(profiler: HotFunctionProfiler) -> Self {
        #[cfg(feature = "jit")]
        let modules = Arc::new(IrRegistry::new());
        #[cfg(feature = "jit")]
        let queue = CompileQueue::new(Box::new(OrcJitCompileHook::new(Arc::clone(&modules))));
        Self {
            profiler,
            native_code: HashMap::new(),
            #[cfg(feature = "jit")]
            queue,
            #[cfg(feature = "jit")]
            modules,
        }
    }

    pub fn profiler(&self) -> &HotFunctionProfiler {
        &self.profiler
    }

    pub fn profiler_mut(&mut self) -> &mut HotFunctionProfiler {
        &mut self.profiler
    }

    /// Record a call and report whether the function just became hot.
    pub fn record_call(&mut self, function: &str) -> bool {
        let became_hot = self.profiler.record_call(function);
        #[cfg(feature = "jit")]
        if became_hot {
            self.queue.enqueue(function);
        }
        became_hot
    }

    /// Whether a function has reached the hot threshold.
    pub fn is_hot(&self, function: &str) -> bool {
        self.profiler.is_hot(function)
    }

    /// Names of all currently hot functions.
    pub fn hot_functions(&self) -> Vec<&str> {
        self.profiler.hot_functions()
    }

    /// Attach native code produced elsewhere (for example by a future
    /// compiler backend). A null address is rejected because it is not
    /// executable code.
    pub fn register_native_code(&mut self, function: &str, address: usize, size: usize) -> Result<(), String> {
        if address == 0 {
            return Err("cannot register native code at address 0".to_string());
        }
        self.native_code.insert(function.to_string(), NativeCode { address, size });
        Ok(())
    }

    /// Look up native code previously attached to a function, including code
    /// installed by the JIT compilation queue when the feature is enabled.
    pub fn native_code(&self, function: &str) -> Option<NativeCode> {
        if let Some(code) = self.native_code.get(function) {
            return Some(*code);
        }
        #[cfg(feature = "jit")]
        {
            self.queue.state(function).and_then(|state| state.native_code())
        }
        #[cfg(not(feature = "jit"))]
        {
            None
        }
    }

    /// Compile `function` to native code. Without the `jit` feature, or on an
    /// unsupported target, the request is refused explicitly instead of
    /// silently doing nothing.
    pub fn compile(&self, function: &str) -> Result<NativeCode, String> {
        #[cfg(not(feature = "jit"))]
        let _ = function;
        #[cfg(feature = "jit")]
        {
            if !jit_supported() {
                return Err(UNAVAILABLE_ERROR.to_string());
            }
            if let Some(code) = self.native_code(function) {
                return Ok(code);
            }
            self.queue.enqueue(function);
            let mut compiled = None;
            while let Some(result) = self.queue.process_next() {
                if result.function == function {
                    compiled = Some(result.state);
                }
            }
            match compiled {
                Some(CompilationState::Ready(code)) => Ok(code),
                Some(CompilationState::Failed(message)) => Err(message),
                _ => Err(format!("JIT compilation for '{}' did not produce code", function)),
            }
        }
        #[cfg(not(feature = "jit"))]
        {
            Err(UNAVAILABLE_ERROR.to_string())
        }
    }

    /// Register textual LLVM IR for a hot function. The IR must define the
    /// `i64 ()` entry named by [`compile::entry_symbol`].
    #[cfg(feature = "jit")]
    pub fn register_module_ir(&self, function: &str, ir: &str) {
        self.modules.insert(function, ir);
    }

    /// Compilation queue wired to the JIT session (feature builds only).
    #[cfg(feature = "jit")]
    pub fn queue(&self) -> &CompileQueue {
        &self.queue
    }

    /// Compile and invoke a hot function through its typed entry pointer.
    /// On any failure the caller should keep using the interpreter.
    #[cfg(feature = "jit")]
    pub fn invoke(&self, function: &str) -> Result<u64, String> {
        self.compile(function)?;
        // Only call code produced by this session; addresses supplied through
        // `register_native_code` are not known to be executable.
        let code = self
            .queue
            .state(function)
            .and_then(|state| state.native_code())
            .ok_or_else(|| format!("JIT code for '{}' is not available in this session", function))?;
        call_entry(code.address)
    }
}

impl Default for JitEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "jit"))]
    #[test]
    fn compile_reports_missing_compiler() {
        let engine = JitEngine::new();
        let err = engine.compile("main").expect_err("this build has no JIT compiler");
        assert_eq!(err, UNAVAILABLE_ERROR);
    }

    #[cfg(feature = "jit")]
    #[test]
    fn compile_without_registered_ir_reports_a_descriptive_error() {
        let engine = JitEngine::new();
        if !jit_supported() {
            assert_eq!(engine.compile("main").expect_err("unsupported target"), UNAVAILABLE_ERROR);
            return;
        }
        let err = engine.compile("main").expect_err("no IR should be compiled");
        assert_eq!(err, "no LLVM IR registered for JIT function 'main'");
        assert!(engine.queue().state("main").is_some());
    }

    #[cfg(feature = "jit")]
    #[test]
    fn hot_functions_are_wired_into_the_compilation_queue() {
        let mut engine = JitEngine::with_profiler(HotFunctionProfiler::with_config(2, 0.5, 1_000));
        engine.record_call("work");
        assert_eq!(engine.queue().state("work"), None);
        engine.record_call("work");
        assert_eq!(engine.queue().state("work"), Some(CompilationState::Queued));
    }

    #[cfg(feature = "jit")]
    #[test]
    fn unavailable_hook_is_kept_for_unsupported_targets() {
        let queue = CompileQueue::unavailable();
        queue.enqueue("work");
        assert_eq!(
            queue.process_next().expect("one job").state,
            CompilationState::Failed(UNAVAILABLE_ERROR.to_string())
        );
    }

    #[test]
    fn profiling_reports_hot_functions() {
        let mut engine = JitEngine::with_profiler(HotFunctionProfiler::with_config(2, 0.5, 1_000));
        engine.record_call("work");
        assert!(!engine.is_hot("work"));
        engine.record_call("work");
        assert!(engine.is_hot("work"));
        assert_eq!(engine.hot_functions(), vec!["work"]);
    }

    #[test]
    fn native_code_can_be_registered_and_read_back() {
        let mut engine = JitEngine::new();
        assert_eq!(engine.native_code("main"), None);
        engine.register_native_code("main", 0x1000, 64).expect("non-null code should register");
        assert_eq!(
            engine.native_code("main"),
            Some(NativeCode { address: 0x1000, size: 64 })
        );
    }

    #[test]
    fn null_native_code_is_rejected() {
        let mut engine = JitEngine::new();
        let err = engine
            .register_native_code("main", 0, 64)
            .expect_err("null address should not register");
        assert_eq!(err, "cannot register native code at address 0");
        assert_eq!(engine.native_code("main"), None);
    }
}
