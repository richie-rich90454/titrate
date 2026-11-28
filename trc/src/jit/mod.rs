// Titrate Alpha 0.2 - JIT tier foundation

pub mod profile;
pub mod orc;
#[cfg(feature = "jit")]
pub mod compile;

use std::collections::HashMap;
#[cfg(feature = "jit")]
use std::sync::Arc;

#[cfg(feature = "jit")]
use self::compile::{call_entry, jit_supported, IrRegistry, OrcJitCompileHook};
use self::orc::{CompilationState, CompileQueue};
use self::profile::HotFunctionProfiler;

/// Diagnostic used when no JIT backend exists for this build or target.
/// Every non-`jit` entry point returns exactly this string.
pub const UNAVAILABLE_ERROR: &str = "JIT compilation not available in this build";

/// Upper bound on queue jobs processed by one synchronous [`JitEngine::compile`]
/// call, so a large or cyclical queue cannot stall the caller indefinitely.
const MAX_COMPILE_STEPS: usize = 1_024;

/// A registered piece of executable machine code for one function.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeCode {
    /// Address of the executable code.
    pub address: usize,
    /// Size of the executable code in bytes (zero when unknown).
    pub size: usize,
}

/// Point-in-time counters for the JIT tier. Profiling and queue bookkeeping are
/// available in every build; `available` is false without the `jit` feature or
/// on an unsupported target.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct JitStatistics {
    pub available: bool,
    pub registered_ir: usize,
    pub native_code_entries: usize,
    pub hot_functions: usize,
    pub queued: usize,
    pub compiling: usize,
    pub ready: usize,
    pub failed: usize,
    pub pending: usize,
    pub compiles_succeeded: u64,
    pub compiles_failed: u64,
    pub retries: u64,
    pub invalidations: u64,
}

/// Result of a JIT-aware call-site dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchOutcome<T> {
    /// Compiled machine code ran; carries the raw `i64` entry result.
    Jit(u64),
    /// No usable machine code was available, so the interpreter ran.
    Interpreted(T),
}

impl<T> DispatchOutcome<T> {
    pub fn is_jit(&self) -> bool {
        matches!(self, Self::Jit(_))
    }

    pub fn is_interpreted(&self) -> bool {
        matches!(self, Self::Interpreted(_))
    }
}

/// JIT tier facade. Profiling, code registration, and the compilation queue
/// work in every build. With the `jit` feature on a supported target, hot
/// functions are compiled into an ORC-backed execution session; otherwise the
/// compile and invoke paths return [`UNAVAILABLE_ERROR`] and callers keep
/// interpreting.
#[derive(Debug)]
pub struct JitEngine {
    profiler: HotFunctionProfiler,
    native_code: HashMap<String, NativeCode>,
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
        #[cfg(not(feature = "jit"))]
        let queue = CompileQueue::unavailable();
        Self {
            profiler,
            native_code: HashMap::new(),
            queue,
            #[cfg(feature = "jit")]
            modules,
        }
    }

    /// Whether this build and target can actually compile JIT code.
    pub fn is_available(&self) -> bool {
        #[cfg(feature = "jit")]
        {
            jit_supported()
        }
        #[cfg(not(feature = "jit"))]
        {
            false
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
            // Hotness drives priority: the more calls a function has, the
            // earlier it is compiled when several jobs are waiting.
            let score = self.profiler.call_count(function) as i32;
            self.queue.enqueue_with_priority(function, score);
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

    /// Look up machine code previously attached to a function, including code
    /// installed by the JIT compilation queue when the feature is enabled.
    pub fn native_code(&self, function: &str) -> Option<NativeCode> {
        if let Some(code) = self.native_code.get(function) {
            return Some(*code);
        }
        self.session_code(function)
    }

    /// Code produced by this engine's own session. Unlike
    /// [`JitEngine::register_native_code`], these addresses are known to be
    /// executable and are the only ones [`JitEngine::invoke`] will call.
    fn session_code(&self, function: &str) -> Option<NativeCode> {
        #[cfg(feature = "jit")]
        {
            self.queue.state(function).and_then(|state| state.native_code())
        }
        #[cfg(not(feature = "jit"))]
        {
            let _ = function;
            None
        }
    }

    /// Compile `function` to native code. Without the `jit` feature, or on an
    /// unsupported target, the request is refused with [`UNAVAILABLE_ERROR`]
    /// instead of silently doing nothing.
    pub fn compile(&self, function: &str) -> Result<NativeCode, String> {
        if !self.is_available() {
            return Err(UNAVAILABLE_ERROR.to_string());
        }
        if let Some(code) = self.session_code(function) {
            return Ok(code);
        }
        self.queue.enqueue(function);
        for _ in 0..MAX_COMPILE_STEPS {
            if !matches!(self.queue.state(function), Some(ref state) if state.is_live()) {
                break;
            }
            if self.queue.process_next().is_none() {
                break;
            }
        }
        match self.queue.state(function) {
            Some(CompilationState::Ready(code)) => Ok(code),
            Some(CompilationState::Failed(message)) => Err(message),
            _ => Err(format!("JIT compilation for '{}' did not produce code", function)),
        }
    }

    /// Register textual LLVM IR for a hot function. Feature builds expect the
    /// `i64 ()` entry named by `compile::entry_symbol`; in a non-`jit` build
    /// registration is inert and [`JitEngine::compile`] and
    /// [`JitEngine::invoke`] report the unavailable backend explicitly.
    pub fn register_module_ir(&self, function: &str, ir: &str) {
        #[cfg(feature = "jit")]
        {
            self.modules.insert(function, ir);
        }
        #[cfg(not(feature = "jit"))]
        {
            let _ = (function, ir);
        }
    }

    /// Compilation queue wired to the JIT session. The state machine and its
    /// statistics are observable in every build; without the feature its hook
    /// fails every job with [`UNAVAILABLE_ERROR`].
    pub fn queue(&self) -> &CompileQueue {
        &self.queue
    }

    /// Point-in-time counters for profiling, registration, and compilation.
    pub fn stats(&self) -> JitStatistics {
        let mut stats = JitStatistics {
            available: self.is_available(),
            hot_functions: self.profiler.hot_functions().len(),
            native_code_entries: self.native_code.len(),
            registered_ir: {
                #[cfg(feature = "jit")]
                {
                    self.modules.len()
                }
                #[cfg(not(feature = "jit"))]
                {
                    0
                }
            },
            ..JitStatistics::default()
        };
        for (_, state) in self.queue.snapshot() {
            match state {
                CompilationState::Queued => stats.queued += 1,
                CompilationState::Compiling => stats.compiling += 1,
                CompilationState::Ready(_) => stats.ready += 1,
                CompilationState::Failed(_) => stats.failed += 1,
            }
        }
        let queue = self.queue.stats();
        stats.pending = self.queue.pending_count();
        stats.compiles_succeeded = queue.succeeded;
        stats.compiles_failed = queue.failed;
        stats.retries = queue.retries;
        stats.invalidations = queue.invalidations;
        stats
    }

    /// Compile and invoke a hot function through its typed entry pointer.
    /// On any failure the caller should keep using the interpreter.
    pub fn invoke(&self, function: &str) -> Result<u64, String> {
        #[cfg(not(feature = "jit"))]
        {
            let _ = function;
            Err(UNAVAILABLE_ERROR.to_string())
        }
        #[cfg(feature = "jit")]
        {
            let code = self.compile(function)?;
            call_entry(code.address)
        }
    }

    /// VM call-site hook: run compiled machine code when this session has it,
    /// otherwise run the interpreter fallback. Any JIT failure deoptimizes the
    /// function and falls back, so an invalid or stale entry cannot crash the
    /// VM.
    pub fn dispatch<T, F>(&self, function: &str, fallback: F) -> DispatchOutcome<T>
    where
        F: FnOnce() -> T,
    {
        if self.session_code(function).is_some() {
            if let Ok(value) = self.invoke(function) {
                return DispatchOutcome::Jit(value);
            }
            self.deoptimize(function);
        }
        DispatchOutcome::Interpreted(fallback())
    }

    /// Drop installed machine code so the function is recompiled on its next
    /// hot call. Returns true when a `Ready` job was requeued.
    pub fn deoptimize(&self, function: &str) -> bool {
        self.queue.invalidate(function)
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
    fn no_jit_error_paths_are_byte_identical() {
        let mut engine = JitEngine::new();
        assert!(!engine.is_available());
        assert_eq!(engine.compile("main").expect_err("no compiler"), UNAVAILABLE_ERROR);
        assert_eq!(engine.invoke("main").expect_err("no compiler"), UNAVAILABLE_ERROR);

        // Registration is inert but must not panic or create executable code.
        engine.register_module_ir("main", "define i64 @main_jit_entry() {\nentry:\n  ret i64 1\n}\n");
        assert_eq!(engine.compile("main").expect_err("registration cannot enable JIT"), UNAVAILABLE_ERROR);

        // The queue still reports the missing backend explicitly.
        engine.queue().enqueue("work");
        assert_eq!(
            engine.queue().process_next().expect("one job").state,
            CompilationState::Failed(UNAVAILABLE_ERROR.to_string())
        );
        assert!(!engine.stats().available);
        assert_eq!(engine.stats().registered_ir, 0);
        assert_eq!(engine.stats().compiles_failed, 1);
    }

    #[cfg(feature = "jit")]
    #[test]
    fn compile_without_registered_ir_reports_a_descriptive_error() {
        let engine = JitEngine::new();
        if !jit_supported() {
            assert_eq!(engine.compile("main").expect_err("unsupported target"), UNAVAILABLE_ERROR);
            assert_eq!(engine.invoke("main").expect_err("unsupported target"), UNAVAILABLE_ERROR);
            return;
        }
        let err = engine.compile("main").expect_err("no IR should be compiled");
        assert_eq!(err, "no LLVM IR registered for JIT function 'main'");
        assert_eq!(
            engine.queue().state("main"),
            Some(CompilationState::Failed(err.clone()))
        );
    }

    #[cfg(feature = "jit")]
    #[test]
    fn registered_ir_is_counted_in_statistics() {
        let engine = JitEngine::new();
        engine.register_module_ir("work", "define i64 @work_jit_entry() {\nentry:\n  ret i64 42\n}\n");
        assert_eq!(engine.stats().registered_ir, 1);
        assert_eq!(engine.stats().available, jit_supported());
    }

    #[cfg(feature = "jit")]
    #[test]
    fn hot_functions_are_wired_into_the_compilation_queue_with_priority() {
        let mut engine = JitEngine::with_profiler(HotFunctionProfiler::with_config(2, 0.5, 1_000));
        engine.record_call("work");
        assert_eq!(engine.queue().state("work"), None);
        engine.record_call("work");
        assert_eq!(engine.queue().state("work"), Some(CompilationState::Queued));
        assert_eq!(engine.queue().priority("work"), Some(2));
        assert_eq!(engine.stats().queued, 1);
    }

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
        assert_eq!(engine.stats().native_code_entries, 1);
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

    #[test]
    fn dispatch_falls_back_when_no_session_code_is_available() {
        let mut engine = JitEngine::new();
        // Even an address supplied by another backend is not trusted as an
        // entry point; dispatch must keep interpreting.
        engine.register_native_code("main", 0x1000, 64).expect("non-null code should register");
        let outcome = engine.dispatch("main", || 7);
        assert_eq!(outcome, DispatchOutcome::Interpreted(7));
        assert!(outcome.is_interpreted());
        assert!(!outcome.is_jit());
    }

    #[test]
    fn statistics_reflect_queue_states() {
        let engine = JitEngine::new();
        engine.queue().enqueue("a");
        engine.queue().enqueue("b");
        engine.queue().process_next();
        let stats = engine.stats();
        assert_eq!(stats.queued, 1);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.compiles_failed, 1);
        assert_eq!(stats.compiles_succeeded, 0);
    }
}
