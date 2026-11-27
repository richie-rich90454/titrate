// Titrate Alpha 0.2 - JIT tier: hot-function profiling

use std::collections::HashMap;

/// Call count at which a function is considered hot.
const DEFAULT_THRESHOLD: u64 = 100;
/// Multiplier applied to every count on each decay step.
const DEFAULT_DECAY_FACTOR: f64 = 0.5;
/// Number of recorded calls between decay steps.
const DEFAULT_DECAY_INTERVAL: u64 = 1_000;
/// Tolerance for floating-point comparisons against an integer threshold.
const EPSILON: f64 = 1e-9;

/// Tracks per-function call counts and promotes functions to "hot" once their
/// count reaches the configured threshold. Counts decay over time so a
/// function that is no longer called stops consuming JIT budget.
#[derive(Clone, Debug)]
pub struct HotFunctionProfiler {
    counts: HashMap<String, f64>,
    threshold: f64,
    decay_factor: f64,
    decay_interval: u64,
    calls_since_decay: u64,
}

impl HotFunctionProfiler {
    /// Profiler with production defaults: hot after 100 calls, halve all
    /// counts every 1,000 calls.
    pub fn new() -> Self {
        Self::with_config(DEFAULT_THRESHOLD, DEFAULT_DECAY_FACTOR, DEFAULT_DECAY_INTERVAL)
    }

    /// Profiler with explicit tuning. Invalid values are clamped to a usable
    /// range: a threshold of at least one call, a decay factor in (0, 1], and
    /// an interval of at least one call.
    pub fn with_config(threshold: u64, decay_factor: f64, decay_interval: u64) -> Self {
        let threshold = threshold.max(1) as f64;
        let decay_factor = if decay_factor.is_finite() && decay_factor > 0.0 && decay_factor <= 1.0 {
            decay_factor
        } else {
            DEFAULT_DECAY_FACTOR
        };
        Self {
            counts: HashMap::new(),
            threshold,
            decay_factor,
            decay_interval: decay_interval.max(1),
            calls_since_decay: 0,
        }
    }

    /// Record one call to `function`. Returns true only when this call raises
    /// the function from cold to hot.
    pub fn record_call(&mut self, function: &str) -> bool {
        let was_hot = self.is_hot(function);
        match self.counts.get_mut(function) {
            Some(count) => *count += 1.0,
            None => {
                self.counts.insert(function.to_string(), 1.0);
            }
        }

        self.calls_since_decay += 1;
        if self.calls_since_decay >= self.decay_interval {
            self.decay();
        }

        self.is_hot(function) && !was_hot
    }

    /// Whether `function` has reached the hot threshold.
    pub fn is_hot(&self, function: &str) -> bool {
        self.counts
            .get(function)
            .is_some_and(|&count| count + EPSILON >= self.threshold)
    }

    /// Current hotness score for `function` (zero if never called).
    pub fn call_count(&self, function: &str) -> f64 {
        self.counts.get(function).copied().unwrap_or(0.0)
    }

    /// Names of all currently hot functions, sorted for deterministic output.
    pub fn hot_functions(&self) -> Vec<&str> {
        let mut hot: Vec<&str> = self
            .counts
            .iter()
            .filter(|(_, &count)| count + EPSILON >= self.threshold)
            .map(|(name, _)| name.as_str())
            .collect();
        hot.sort_unstable();
        hot
    }

    /// Multiply every count by the decay factor and restart the decay window.
    /// Scores that fall below the comparison tolerance are dropped to zero so
    /// dormant functions do not leave a permanent fractional tail.
    pub fn decay(&mut self) {
        for count in self.counts.values_mut() {
            *count *= self.decay_factor;
            if *count < EPSILON {
                *count = 0.0;
            }
        }
        self.calls_since_decay = 0;
    }

    /// Forget all recorded functions and reset the decay window.
    pub fn clear(&mut self) {
        self.counts.clear();
        self.calls_since_decay = 0;
    }
}

impl Default for HotFunctionProfiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_function_is_cold() {
        let profiler = HotFunctionProfiler::new();
        assert!(!profiler.is_hot("main"));
        assert_eq!(profiler.call_count("main"), 0.0);
        assert!(profiler.hot_functions().is_empty());
    }

    #[test]
    fn function_becomes_hot_only_at_threshold() {
        let mut profiler = HotFunctionProfiler::with_config(3, 0.5, 1_000);
        assert!(!profiler.record_call("work"));
        assert!(!profiler.is_hot("work"));
        assert!(!profiler.record_call("work"));
        assert!(!profiler.is_hot("work"));
        assert!(profiler.record_call("work"), "third call should cross the threshold");
        assert!(profiler.is_hot("work"));
        assert!(!profiler.record_call("work"), "already-hot calls should not re-trigger");
        assert_eq!(profiler.call_count("work"), 4.0);
    }

    #[test]
    fn counts_are_tracked_per_function() {
        let mut profiler = HotFunctionProfiler::with_config(2, 0.5, 1_000);
        profiler.record_call("a");
        profiler.record_call("b");
        profiler.record_call("a");
        assert!(profiler.is_hot("a"));
        assert!(!profiler.is_hot("b"));
        assert_eq!(profiler.hot_functions(), vec!["a"]);
    }

    #[test]
    fn decay_cools_a_previously_hot_function() {
        let mut profiler = HotFunctionProfiler::with_config(3, 0.5, 1_000);
        for _ in 0..4 {
            profiler.record_call("work");
        }
        assert!(profiler.is_hot("work"));
        profiler.decay();
        assert_eq!(profiler.call_count("work"), 2.0);
        assert!(!profiler.is_hot("work"));
        assert!(profiler.record_call("work"));
        assert!(profiler.is_hot("work"));
    }

    #[test]
    fn decay_runs_automatically_at_the_configured_interval() {
        let mut profiler = HotFunctionProfiler::with_config(8, 0.5, 10);
        let mut crossed = false;
        for _ in 0..10 {
            crossed |= profiler.record_call("work");
        }
        assert!(crossed, "function should have crossed the threshold before decay");
        assert_eq!(profiler.call_count("work"), 5.0);
        assert!(!profiler.is_hot("work"), "automatic decay should cool the function");
    }

    #[test]
    fn decay_drops_negligible_scores_to_zero() {
        let mut profiler = HotFunctionProfiler::with_config(2, 0.5, 1_000);
        profiler.record_call("work");
        profiler.decay();
        assert_eq!(profiler.call_count("work"), 0.5);
        for _ in 0..40 {
            profiler.decay();
        }
        assert_eq!(profiler.call_count("work"), 0.0);
    }

    #[test]
    fn clear_resets_counts_and_decay_window() {
        let mut profiler = HotFunctionProfiler::with_config(1, 0.5, 1_000);
        profiler.record_call("work");
        assert!(profiler.is_hot("work"));
        profiler.clear();
        assert!(!profiler.is_hot("work"));
        assert_eq!(profiler.call_count("work"), 0.0);
        assert!(profiler.hot_functions().is_empty());
    }

    #[test]
    fn invalid_configuration_is_clamped() {
        let mut profiler = HotFunctionProfiler::with_config(0, 0.5, 1_000);
        assert!(!profiler.is_hot("main"), "unknown function stays cold even with threshold 0");
        assert!(profiler.record_call("main"), "threshold 0 should clamp to one call");

        let mut profiler = HotFunctionProfiler::with_config(2, 2.0, 1_000);
        profiler.record_call("work");
        profiler.record_call("work");
        profiler.decay();
        assert_eq!(profiler.call_count("work"), 1.0, "invalid decay factor should clamp to 0.5");

        let mut profiler = HotFunctionProfiler::with_config(2, 0.5, 0);
        profiler.record_call("work");
        assert_eq!(profiler.call_count("work"), 0.5, "invalid interval should clamp to 1");
    }
}
