//! Library surface for the deterministic verification fuzz harnesses.

pub mod abi;
pub mod common;
pub mod differential;
pub mod expressions;
pub mod module_graphs;
pub mod native_args;

use std::panic::{catch_unwind, AssertUnwindSafe};

use common::HarnessReport;

/// One fuzz harness entry: a stable name and its deterministic runner.
pub type FuzzCase = (&'static str, fn(usize, u64) -> Result<(), String>);

/// Every harness the CI gate script runs, in a stable order.
pub const CASES: &[FuzzCase] = &[
    ("native_args", native_args::run),
    ("expressions", expressions::run),
    ("module_graphs", module_graphs::run),
    ("differential", differential::run),
];

/// Run one named harness. A panic anywhere in a harness becomes a structured
/// failing report rather than an aborted CI job.
pub fn run_case(
    name: &str,
    iterations: usize,
    seed: u64,
) -> Result<HarnessReport, String> {
    let (_, run) = CASES
        .iter()
        .find(|(case_name, _)| *case_name == name)
        .ok_or_else(|| {
            format!(
                "unknown harness '{}'; expected one of: {}",
                name,
                CASES
                    .iter()
                    .map(|(case_name, _)| *case_name)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
    let outcome = catch_unwind(AssertUnwindSafe(|| run(iterations, seed)));
    Ok(match outcome {
        Ok(Ok(())) => HarnessReport::pass(name),
        Ok(Err(error)) => HarnessReport::fail(name, error),
        Err(_) => HarnessReport::fail(name, "harness panicked"),
    })
}

/// Run every harness, used by local checks and by ad-hoc investigations.
pub fn run_all(iterations: usize, seed: u64) -> Vec<HarnessReport> {
    CASES
        .iter()
        .map(|(name, _)| {
            run_case(name, iterations, seed)
                .unwrap_or_else(|error| HarnessReport::fail(*name, error))
        })
        .collect()
}
