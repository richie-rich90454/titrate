//! Startup and compile-latency probe for Phase 7.
//!
//! `StartupProbe` records timestamps inline, without allocating, so it can be
//! placed on the cold path. `measure_pipeline` times a real lex → parse →
//! analyze → compile → load → run sequence and reports its stages. The budget
//! constant is a target to check against, never a claim that it is met.

use std::fmt::Write as _;
use std::hint::black_box;
use std::time::{Duration, Instant};

use crate::{analyzer, bytecode, lexer, parser};

/// Target for first output in script mode, from the roadmap.
pub const STARTUP_BUDGET_MS: u64 = 50;

const MAX_MARKS: usize = 16;

#[derive(Clone, Copy, Debug)]
struct Mark {
    label: &'static str,
    at: Instant,
}

pub struct StartupProbe {
    started: Instant,
    marks: [Option<Mark>; MAX_MARKS],
    count: usize,
}

impl Default for StartupProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl StartupProbe {
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            marks: [None; MAX_MARKS],
            count: 0,
        }
    }

    /// Record a checkpoint. Returns false if the fixed mark table is full.
    pub fn mark(&mut self, label: &'static str) -> bool {
        if self.count == MAX_MARKS {
            return false;
        }
        self.marks[self.count] = Some(Mark {
            label,
            at: Instant::now(),
        });
        self.count += 1;
        true
    }

    pub fn mark_count(&self) -> usize {
        self.count
    }

    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// Time between two named checkpoints, in recording order.
    pub fn interval(&self, from: &'static str, to: &'static str) -> Option<Duration> {
        let marks = &self.marks[..self.count];
        let start = marks.iter().flatten().find(|mark| mark.label == from)?.at;
        let end = marks.iter().flatten().find(|mark| mark.label == to)?.at;
        if end < start {
            return None;
        }
        Some(end.duration_since(start))
    }

    pub fn write_summary(&self, out: &mut String) {
        let mut previous = self.started;
        for mark in self.marks[..self.count].iter().flatten() {
            let interval = mark.at.duration_since(previous);
            let total = mark.at.duration_since(self.started);
            let _ = writeln!(
                out,
                "{:<14} {:>9.3} ms (+{:>8.3} ms)",
                mark.label,
                total.as_secs_f64() * 1000.0,
                interval.as_secs_f64() * 1000.0
            );
            previous = mark.at;
        }
        let _ = writeln!(
            out,
            "total          {:>9.3} ms",
            self.elapsed().as_secs_f64() * 1000.0
        );
    }

    pub fn within_budget(&self, budget: Duration) -> bool {
        self.elapsed() <= budget
    }
}

/// Per-stage timings for one full pipeline run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PipelineTiming {
    pub lex: Duration,
    pub parse: Duration,
    pub analyze: Duration,
    pub compile: Duration,
    pub load: Duration,
    pub run: Duration,
}

impl PipelineTiming {
    pub fn total(&self) -> Duration {
        self.lex + self.parse + self.analyze + self.compile + self.load + self.run
    }

    /// Everything up to and including the first program output.
    pub fn to_first_output(&self) -> Duration {
        self.total()
    }

    pub fn within_budget(&self, budget: Duration) -> bool {
        self.to_first_output() <= budget
    }
}

/// Cost of constructing a VM, which registers every native at startup.
pub fn measure_vm_start() -> Duration {
    let start = Instant::now();
    let vm = bytecode::Vm::new();
    let elapsed = start.elapsed();
    black_box(vm);
    elapsed
}

/// Time the real compiler-and-VM pipeline for one source string.
pub fn measure_pipeline(source: &str) -> Result<PipelineTiming, String> {
    let start = Instant::now();
    let tokens = lexer::tokenize(source)?;
    let lex = start.elapsed();

    let start = Instant::now();
    let ast = parser::parse(tokens)?;
    let parse = start.elapsed();

    let start = Instant::now();
    let typed = analyzer::analyze(&ast).map_err(|errors| errors.join("\n"))?;
    let analyze = start.elapsed();

    let start = Instant::now();
    let mut compiler = bytecode::Compiler::new();
    let compiled = compiler.compile(&typed)?;
    let compile = start.elapsed();

    let start = Instant::now();
    let mut vm = bytecode::Vm::new();
    vm.load_program(compiled);
    let load = start.elapsed();

    let start = Instant::now();
    vm.run()?;
    let run = start.elapsed();

    Ok(PipelineTiming {
        lex,
        parse,
        analyze,
        compile,
        load,
        run,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINI: &str = r#"public fn main(): void { io::println("ok"); }"#;

    #[test]
    fn probe_records_marks_and_intervals() {
        let mut probe = StartupProbe::new();
        assert!(probe.mark("lex"));
        let mut sink = 0u64;
        for i in 0..10_000 {
            sink = sink.wrapping_add(black_box(i));
        }
        black_box(sink);
        assert!(probe.mark("parse"));
        assert_eq!(probe.mark_count(), 2);
        assert!(probe.interval("lex", "parse").is_some());
        assert!(probe.interval("parse", "lex").is_none());
        assert!(probe.interval("missing", "parse").is_none());
        let mut summary = String::new();
        probe.write_summary(&mut summary);
        assert!(summary.contains("lex"));
        assert!(summary.contains("parse"));
        assert!(summary.contains("total"));
        assert!(probe.within_budget(Duration::from_secs(10)));
    }

    #[test]
    fn probe_marks_are_bounded() {
        let mut probe = StartupProbe::new();
        let labels = [
            "a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7", "a8", "a9", "b0", "b1", "b2", "b3",
            "b4", "b5", "b6", "b7",
        ];
        for label in labels {
            probe.mark(label);
        }
        assert_eq!(probe.mark_count(), MAX_MARKS);
    }

    #[test]
    fn vm_start_measurement_is_finite() {
        let elapsed = measure_vm_start();
        assert!(elapsed < Duration::from_secs(30));
    }

    #[test]
    fn pipeline_measurement_covers_every_stage() {
        let timing = measure_pipeline(MINI).expect("mini program should run");
        assert_eq!(timing.total(), timing.to_first_output());
        assert!(timing.within_budget(Duration::from_secs(30)));
    }

    #[test]
    fn pipeline_measurement_reports_source_errors() {
        assert!(measure_pipeline("fn (").is_err());
    }
}
