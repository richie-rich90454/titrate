//! Titrate Alpha 0.2 - JIT tier: ORC compilation queue and job lifecycle.
//!
//! This module implements the scheduling layer only. It turns hot functions
//! into compilation jobs, drives each job through
//! `Queued -> Compiling -> Ready | Failed`, and keeps state safe to query from
//! interpreter threads while a compiler worker runs.
//!
//! Native compilation is not available in this build: the default queue uses
//! [`UnavailableCompileHook`], which returns an explicit error instead of
//! pretending to produce code. A future ORC backend plugs in another
//! [`CompileHook`] without changing the queue or its state machine.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use crate::jit::NativeCode;

/// Lifecycle state of a single compilation job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompilationState {
    /// Accepted and waiting for a compiler slot.
    Queued,
    /// A worker is producing native code.
    Compiling,
    /// Native code is installed and callable.
    Ready(NativeCode),
    /// Compilation failed; carries the diagnostic.
    Failed(String),
}

impl CompilationState {
    /// Whether the job still needs compiler attention.
    pub fn is_live(&self) -> bool {
        matches!(self, Self::Queued | Self::Compiling)
    }

    /// Installed code, if the job reached `Ready`.
    pub fn native_code(&self) -> Option<NativeCode> {
        match self {
            Self::Ready(code) => Some(*code),
            _ => None,
        }
    }
}

/// Outcome of driving one queued job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompilationResult {
    pub function: String,
    pub state: CompilationState,
}

/// Produces native code for one function. The queue calls this outside its
/// internal lock so several jobs can compile in parallel.
pub trait CompileHook: Send + Sync {
    fn compile(&self, function: &str) -> Result<NativeCode, String>;
}

/// Current-tier hook. There is no ORC backend in this build, so every request
/// is refused with the same explicit diagnostic used by `JitEngine::compile`.
#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableCompileHook;

impl CompileHook for UnavailableCompileHook {
    fn compile(&self, _function: &str) -> Result<NativeCode, String> {
        Err("JIT compilation not available in this build".to_string())
    }
}

#[derive(Default)]
struct QueueState {
    states: HashMap<String, CompilationState>,
    order: VecDeque<String>,
}

/// Thread-safe FIFO compilation queue with an explicit lifecycle state machine.
pub struct CompileQueue {
    state: Mutex<QueueState>,
    hook: Box<dyn CompileHook>,
}

impl CompileQueue {
    pub fn new(hook: Box<dyn CompileHook>) -> Self {
        Self { state: Mutex::new(QueueState::default()), hook }
    }

    /// Queue whose compile hook always reports the missing backend.
    pub fn unavailable() -> Self {
        Self::new(Box::new(UnavailableCompileHook))
    }

    /// Schedule `function`. Returns true only when a new job was created.
    /// Live and ready jobs are left alone; failed jobs are reset for a retry.
    pub fn enqueue(&self, function: &str) -> bool {
        let mut state = self.state.lock().expect("compile queue lock poisoned");
        match state.states.get(function) {
            Some(CompilationState::Failed(_)) => {}
            Some(_) => return false,
            None => {}
        }
        state.states.insert(function.to_string(), CompilationState::Queued);
        state.order.push_back(function.to_string());
        true
    }

    /// Invalidate installed code so the function can be recompiled, for example
    /// after deoptimization. Returns true when a `Ready` job was requeued.
    pub fn invalidate(&self, function: &str) -> bool {
        let mut state = self.state.lock().expect("compile queue lock poisoned");
        if matches!(state.states.get(function), Some(CompilationState::Ready(_))) {
            state.states.insert(function.to_string(), CompilationState::Queued);
            state.order.push_back(function.to_string());
            true
        } else {
            false
        }
    }

    /// Current state of `function`, if it has ever been queued.
    pub fn state(&self, function: &str) -> Option<CompilationState> {
        self.state
            .lock()
            .expect("compile queue lock poisoned")
            .states
            .get(function)
            .cloned()
    }

    /// Number of jobs waiting to be compiled.
    pub fn pending_count(&self) -> usize {
        self.state.lock().expect("compile queue lock poisoned").order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending_count() == 0
    }

    /// All known jobs sorted by function name for deterministic reporting.
    pub fn snapshot(&self) -> Vec<(String, CompilationState)> {
        let state = self.state.lock().expect("compile queue lock poisoned");
        let mut jobs: Vec<(String, CompilationState)> = state
            .states
            .iter()
            .map(|(name, job)| (name.clone(), job.clone()))
            .collect();
        jobs.sort_by(|a, b| a.0.cmp(&b.0));
        jobs
    }

    /// Compile the next queued job in FIFO order, or `None` if the queue is empty.
    pub fn process_next(&self) -> Option<CompilationResult> {
        let function = loop {
            let mut state = self.state.lock().expect("compile queue lock poisoned");
            let function = state.order.pop_front()?;
            if matches!(state.states.get(&function), Some(CompilationState::Queued)) {
                state.states.insert(function.clone(), CompilationState::Compiling);
                break function;
            }
            // Stale order entry: drop it and look at the next one.
        };

        // Compilation runs without the lock so queries and enqueues stay responsive.
        let final_state = match self.hook.compile(&function) {
            Ok(code) => CompilationState::Ready(code),
            Err(message) => CompilationState::Failed(message),
        };
        self.state
            .lock()
            .expect("compile queue lock poisoned")
            .states
            .insert(function.clone(), final_state.clone());
        Some(CompilationResult { function, state: final_state })
    }

    /// Drain the queue in FIFO order.
    pub fn process_all(&self) -> Vec<CompilationResult> {
        let mut results = Vec::new();
        while let Some(result) = self.process_next() {
            results.push(result);
        }
        results
    }
}

impl Default for CompileQueue {
    fn default() -> Self {
        Self::unavailable()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    fn code() -> NativeCode {
        NativeCode { address: 0x1000, size: 64 }
    }

    struct SequenceHook {
        outcomes: Mutex<VecDeque<Result<NativeCode, String>>>,
    }

    impl SequenceHook {
        fn new(outcomes: Vec<Result<NativeCode, String>>) -> Self {
            Self { outcomes: Mutex::new(VecDeque::from(outcomes)) }
        }
    }

    impl CompileHook for SequenceHook {
        fn compile(&self, _function: &str) -> Result<NativeCode, String> {
            self.outcomes
                .lock()
                .expect("scripted hook lock poisoned")
                .pop_front()
                .unwrap_or_else(|| Err("no scripted compile outcome".to_string()))
        }
    }

    #[test]
    fn enqueue_starts_queued_and_deduplicates() {
        let queue = CompileQueue::unavailable();
        assert!(queue.enqueue("work"));
        assert_eq!(queue.state("work"), Some(CompilationState::Queued));
        assert!(!queue.enqueue("work"), "duplicate should not create a second job");
        assert_eq!(queue.pending_count(), 1);
    }

    #[test]
    fn state_of_unknown_function_is_none() {
        let queue = CompileQueue::unavailable();
        assert_eq!(queue.state("missing"), None);
        assert!(queue.is_empty());
    }

    #[test]
    fn unavailable_hook_marks_job_failed_with_explicit_error() {
        let queue = CompileQueue::unavailable();
        queue.enqueue("work");
        let result = queue.process_next().expect("one job queued");
        assert_eq!(result.function, "work");
        assert_eq!(
            result.state,
            CompilationState::Failed("JIT compilation not available in this build".to_string())
        );
        assert_eq!(queue.state("work"), Some(result.state));
        assert_eq!(queue.pending_count(), 0);
    }

    #[test]
    fn successful_hook_marks_job_ready() {
        let queue = CompileQueue::new(Box::new(SequenceHook::new(vec![Ok(code())])));
        queue.enqueue("work");
        let result = queue.process_next().expect("one job queued");
        assert_eq!(result.state, CompilationState::Ready(code()));
        assert_eq!(result.state.native_code(), Some(code()));
        assert!(!result.state.is_live());
    }

    #[test]
    fn jobs_are_processed_in_fifo_order() {
        let queue = CompileQueue::unavailable();
        queue.enqueue("a");
        queue.enqueue("b");
        queue.enqueue("c");
        let results = queue.process_all();
        let names: Vec<&str> = results.iter().map(|r| r.function.as_str()).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
        assert!(queue.is_empty());
    }

    #[test]
    fn failed_job_can_be_retried() {
        let outcomes = vec![Err("transient failure".to_string()), Ok(code())];
        let queue = CompileQueue::new(Box::new(SequenceHook::new(outcomes)));
        queue.enqueue("work");
        assert_eq!(
            queue.process_next().expect("first job").state,
            CompilationState::Failed("transient failure".to_string())
        );
        assert!(queue.enqueue("work"), "failed job should re-enqueue");
        assert_eq!(queue.state("work"), Some(CompilationState::Queued));
        assert_eq!(
            queue.process_next().expect("retry job").state,
            CompilationState::Ready(code())
        );
    }

    #[test]
    fn ready_job_is_not_requeued_but_can_be_invalidated() {
        let queue = CompileQueue::new(Box::new(SequenceHook::new(vec![Ok(code())])));
        queue.enqueue("work");
        assert_eq!(queue.process_next().expect("one job").state, CompilationState::Ready(code()));
        assert!(!queue.enqueue("work"), "ready job must not be recompiled by enqueue");
        assert!(queue.is_empty());

        assert!(queue.invalidate("work"), "invalidate should requeue installed code");
        assert_eq!(queue.state("work"), Some(CompilationState::Queued));
        assert_eq!(queue.pending_count(), 1);
        assert!(!queue.invalidate("work"), "a queued job cannot be invalidated again");
    }

    #[test]
    fn processing_empty_queue_returns_none() {
        let queue = CompileQueue::unavailable();
        assert!(queue.process_next().is_none());
        assert!(queue.process_all().is_empty());
    }

    #[test]
    fn snapshot_is_sorted_by_function_name() {
        let queue = CompileQueue::unavailable();
        queue.enqueue("beta");
        queue.enqueue("alpha");
        let snapshot = queue.snapshot();
        let names: Vec<&str> = snapshot.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn concurrent_enqueues_are_deduplicated() {
        let queue = Arc::new(CompileQueue::unavailable());
        let mut handles = Vec::new();
        for _ in 0..8 {
            let worker = Arc::clone(&queue);
            handles.push(thread::spawn(move || {
                worker.enqueue("hot");
            }));
        }
        for handle in handles {
            handle.join().expect("enqueue worker should not panic");
        }
        assert_eq!(queue.pending_count(), 1);
        assert_eq!(queue.state("hot"), Some(CompilationState::Queued));
    }
}
