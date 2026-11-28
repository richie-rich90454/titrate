//! Titrate Alpha 0.2 - JIT tier: ORC compilation queue and job lifecycle.
//!
//! This module implements the scheduling layer only. It turns hot functions
//! into compilation jobs, drives each job through
//! `Queued -> Compiling -> Ready | Failed`, and keeps state safe to query from
//! interpreter threads while a compiler worker runs. Jobs are ordered by
//! priority (higher first, FIFO for ties) so the hottest function is compiled
//! first when several become hot together.
//!
//! The default queue uses [`UnavailableCompileHook`], which returns an explicit
//! error instead of pretending to produce code. With the `jit` feature on a
//! supported target, `JitEngine` wires in the ORC-backed hook from
//! `jit::compile` without changing the state machine.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Mutex;

use crate::jit::{NativeCode, UNAVAILABLE_ERROR};

/// Priority of a compilation job. Higher values are compiled first.
pub type CompilePriority = i32;

/// Priority assigned by [`CompileQueue::enqueue`].
pub const DEFAULT_PRIORITY: CompilePriority = 0;

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

/// Cumulative counters for a compilation queue.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct QueueStats {
    /// Jobs accepted, including retries and invalidations.
    pub enqueued: u64,
    /// Failed jobs that were re-enqueued for another attempt.
    pub retries: u64,
    /// `Ready` jobs invalidated so they can be recompiled.
    pub invalidations: u64,
    /// Jobs that produced native code.
    pub succeeded: u64,
    /// Jobs whose compile hook reported an error.
    pub failed: u64,
}

/// Produces native code for one function. The queue calls this outside its
/// internal lock so several jobs can compile in parallel.
pub trait CompileHook: Send + Sync {
    fn compile(&self, function: &str) -> Result<NativeCode, String>;
}

/// Fallback hook for builds without the JIT feature and for unsupported
/// targets. Every request is refused with the same explicit diagnostic used by
/// `JitEngine::compile`.
#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableCompileHook;

impl CompileHook for UnavailableCompileHook {
    fn compile(&self, _function: &str) -> Result<NativeCode, String> {
        Err(UNAVAILABLE_ERROR.to_string())
    }
}

/// A scheduled job. `BinaryHeap` pops the greatest item, so a higher priority
/// and a lower sequence number (earlier enqueue) both sort first.
#[derive(Clone, Debug, Eq, PartialEq)]
struct QueuedJob {
    priority: CompilePriority,
    seq: u64,
    function: String,
}

impl Ord for QueuedJob {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.seq.cmp(&self.seq))
            .then_with(|| self.function.cmp(&other.function))
    }
}

impl PartialOrd for QueuedJob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Default)]
struct QueueState {
    states: HashMap<String, CompilationState>,
    jobs: BinaryHeap<QueuedJob>,
    priorities: HashMap<String, CompilePriority>,
    next_seq: u64,
    stats: QueueStats,
}

/// Thread-safe priority compilation queue with an explicit lifecycle state
/// machine and retry support.
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

    /// Schedule `function` with the default priority. Returns true only when a
    /// new job was created.
    pub fn enqueue(&self, function: &str) -> bool {
        self.enqueue_with_priority(function, DEFAULT_PRIORITY)
    }

    /// Schedule `function` at an explicit priority. Live and ready jobs are
    /// left alone; failed jobs keep their priority and are reset for a retry.
    pub fn enqueue_with_priority(&self, function: &str, priority: CompilePriority) -> bool {
        let mut state = self.state.lock().expect("compile queue lock poisoned");
        let retry = matches!(state.states.get(function), Some(CompilationState::Failed(_)));
        if !retry && state.states.contains_key(function) {
            return false;
        }
        let seq = state.next_seq;
        state.next_seq = state.next_seq.wrapping_add(1);
        let priority = if retry {
            state.priorities.get(function).copied().unwrap_or(priority)
        } else {
            priority
        };
        state.priorities.insert(function.to_string(), priority);
        state.states.insert(function.to_string(), CompilationState::Queued);
        state.jobs.push(QueuedJob { priority, seq, function: function.to_string() });
        state.stats.enqueued += 1;
        if retry {
            state.stats.retries += 1;
        }
        true
    }

    /// Invalidate installed code so the function can be recompiled, for example
    /// after deoptimization. The original priority is retained. Returns true
    /// when a `Ready` job was requeued.
    pub fn invalidate(&self, function: &str) -> bool {
        let mut state = self.state.lock().expect("compile queue lock poisoned");
        if !matches!(state.states.get(function), Some(CompilationState::Ready(_))) {
            return false;
        }
        let priority = state.priorities.get(function).copied().unwrap_or(DEFAULT_PRIORITY);
        let seq = state.next_seq;
        state.next_seq = state.next_seq.wrapping_add(1);
        state.states.insert(function.to_string(), CompilationState::Queued);
        state.jobs.push(QueuedJob { priority, seq, function: function.to_string() });
        state.stats.invalidations += 1;
        true
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

    /// Priority remembered for `function`, if it has ever been queued.
    pub fn priority(&self, function: &str) -> Option<CompilePriority> {
        self.state
            .lock()
            .expect("compile queue lock poisoned")
            .priorities
            .get(function)
            .copied()
    }

    /// Number of jobs waiting to be compiled.
    pub fn pending_count(&self) -> usize {
        self.state.lock().expect("compile queue lock poisoned").jobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending_count() == 0
    }

    /// Cumulative counters for this queue.
    pub fn stats(&self) -> QueueStats {
        self.state.lock().expect("compile queue lock poisoned").stats
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

    /// Compile the highest-priority queued job, or `None` if the queue is empty.
    pub fn process_next(&self) -> Option<CompilationResult> {
        let function = loop {
            let mut state = self.state.lock().expect("compile queue lock poisoned");
            let job = state.jobs.pop()?;
            if matches!(state.states.get(&job.function), Some(CompilationState::Queued)) {
                state.states.insert(job.function.clone(), CompilationState::Compiling);
                break job.function;
            }
            // Stale order entry: drop it and look at the next one.
        };

        // Compilation runs without the lock so queries and enqueues stay responsive.
        let final_state = match self.hook.compile(&function) {
            Ok(code) => CompilationState::Ready(code),
            Err(message) => CompilationState::Failed(message),
        };
        let mut state = self.state.lock().expect("compile queue lock poisoned");
        state.states.insert(function.clone(), final_state.clone());
        match &final_state {
            CompilationState::Ready(_) => state.stats.succeeded += 1,
            CompilationState::Failed(_) => state.stats.failed += 1,
            _ => {}
        }
        Some(CompilationResult { function, state: final_state })
    }

    /// Drain the queue in priority order.
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

impl std::fmt::Debug for CompileQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompileQueue")
            .field("pending", &self.pending_count())
            .field("states", &self.snapshot())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
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
        assert_eq!(queue.priority("missing"), None);
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
            CompilationState::Failed(UNAVAILABLE_ERROR.to_string())
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
    fn higher_priority_jobs_are_processed_first() {
        let queue = CompileQueue::new(Box::new(SequenceHook::new(vec![
            Ok(code()), Ok(code()), Ok(code()),
        ])));
        queue.enqueue_with_priority("cold", -1);
        queue.enqueue_with_priority("hot", 10);
        queue.enqueue_with_priority("warm", 5);
        let results = queue.process_all();
        let names: Vec<&str> = results.iter().map(|r| r.function.as_str()).collect();
        assert_eq!(names, vec!["hot", "warm", "cold"]);
    }

    #[test]
    fn equal_priority_jobs_keep_fifo_order() {
        let queue = CompileQueue::new(Box::new(SequenceHook::new(vec![
            Ok(code()), Ok(code()), Ok(code()),
        ])));
        queue.enqueue("first");
        queue.enqueue("second");
        queue.enqueue("third");
        let results = queue.process_all();
        let names: Vec<&str> = results.iter().map(|r| r.function.as_str()).collect();
        assert_eq!(names, vec!["first", "second", "third"]);
    }

    #[test]
    fn failed_job_can_be_retried_with_its_priority() {
        let outcomes = vec![Err("transient failure".to_string()), Ok(code())];
        let queue = CompileQueue::new(Box::new(SequenceHook::new(outcomes)));
        queue.enqueue_with_priority("work", 7);
        assert_eq!(
            queue.process_next().expect("first job").state,
            CompilationState::Failed("transient failure".to_string())
        );
        assert!(queue.enqueue("work"), "failed job should re-enqueue");
        assert_eq!(queue.state("work"), Some(CompilationState::Queued));
        assert_eq!(queue.priority("work"), Some(7));
        assert_eq!(
            queue.process_next().expect("retry job").state,
            CompilationState::Ready(code())
        );
    }

    #[test]
    fn ready_job_is_not_requeued_but_can_be_invalidated() {
        let queue = CompileQueue::new(Box::new(SequenceHook::new(vec![Ok(code())])));
        queue.enqueue_with_priority("work", 4);
        assert_eq!(queue.process_next().expect("one job").state, CompilationState::Ready(code()));
        assert!(!queue.enqueue("work"), "ready job must not be recompiled by enqueue");
        assert!(queue.is_empty());

        assert!(queue.invalidate("work"), "invalidate should requeue installed code");
        assert_eq!(queue.state("work"), Some(CompilationState::Queued));
        assert_eq!(queue.priority("work"), Some(4));
        assert_eq!(queue.pending_count(), 1);
        assert!(!queue.invalidate("work"), "a queued job cannot be invalidated again");
    }

    #[test]
    fn stats_count_lifecycle_transitions() {
        let queue = CompileQueue::new(Box::new(SequenceHook::new(vec![
            Ok(code()),
            Err("bad IR".to_string()),
        ])));
        queue.enqueue("good");
        queue.enqueue("bad");
        assert_eq!(queue.stats().enqueued, 2);

        queue.process_all();
        let stats = queue.stats();
        assert_eq!(stats.succeeded, 1);
        assert_eq!(stats.failed, 1);

        assert!(queue.enqueue("bad"));
        assert_eq!(queue.stats().retries, 1);
        assert_eq!(
            queue.process_next().expect("retried job").state,
            CompilationState::Failed("no scripted compile outcome".to_string())
        );
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
