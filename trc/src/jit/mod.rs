// Titrate Alpha 0.2 - JIT tier foundation

pub mod profile;

use std::collections::HashMap;

use self::profile::HotFunctionProfiler;

/// A registered piece of executable machine code for one function.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeCode {
    /// Address of the executable code.
    pub address: usize,
    /// Size of the executable code in bytes (zero when unknown).
    pub size: usize,
}

/// JIT tier facade. Profiling works in this build; native compilation does not,
/// so `compile` reports that limitation instead of pretending to compile.
#[derive(Debug)]
pub struct JitEngine {
    profiler: HotFunctionProfiler,
    native_code: HashMap<String, NativeCode>,
}

impl JitEngine {
    pub fn new() -> Self {
        Self::with_profiler(HotFunctionProfiler::new())
    }

    pub fn with_profiler(profiler: HotFunctionProfiler) -> Self {
        Self {
            profiler,
            native_code: HashMap::new(),
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
        self.profiler.record_call(function)
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

    /// Look up native code previously attached to a function.
    pub fn native_code(&self, function: &str) -> Option<NativeCode> {
        self.native_code.get(function).copied()
    }

    /// Compile `function` to native code. This build has no JIT compiler, so
    /// the request is refused explicitly rather than silently doing nothing.
    pub fn compile(&self, _function: &str) -> Result<NativeCode, String> {
        Err("JIT compilation not available in this build".to_string())
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

    #[test]
    fn compile_reports_missing_compiler() {
        let engine = JitEngine::new();
        let err = engine.compile("main").expect_err("this build has no JIT compiler");
        assert_eq!(err, "JIT compilation not available in this build");
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
