//! Single-binary entry point for all verification fuzz harnesses.
//!
//! Usage: trc-fuzz --case <name> [--iterations N] [--seed S]
//! Prints one `TRC_FUZZ_JSON:` line. Exit 0 = pass, 1 = harness failure,
//! 2 = bad invocation. No measured numbers are printed.

use std::process::exit;

use trc_fuzz::{common, run_case};

fn parse_args() -> Result<(String, usize, u64), String> {
    let mut case: Option<String> = None;
    let mut iterations = common::iteration_count(common::DEFAULT_ITERATIONS)?;
    let mut seed = common::DEFAULT_SEED;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--case" => {
                case = Some(args.next().ok_or("--case requires a name")?);
            }
            "--iterations" => {
                let raw = args.next().ok_or("--iterations requires a count")?;
                iterations = raw
                    .parse::<usize>()
                    .map_err(|error| format!("invalid --iterations '{}': {}", raw, error))?;
                if !(1..=1_000_000).contains(&iterations) {
                    return Err(format!("--iterations must be in 1..=1000000, got {}", iterations));
                }
            }
            "--seed" => {
                let raw = args.next().ok_or("--seed requires an integer")?;
                seed = raw
                    .parse::<u64>()
                    .map_err(|error| format!("invalid --seed '{}': {}", raw, error))?;
            }
            other => return Err(format!("unknown argument '{}'", other)),
        }
    }

    let case = case.ok_or("--case is required")?;
    Ok((case, iterations, seed))
}

fn main() {
    let (case, iterations, seed) = match parse_args() {
        Ok(values) => values,
        Err(error) => {
            eprintln!("{}", error);
            exit(2);
        }
    };
    let report = match run_case(&case, iterations, seed) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{}", error);
            exit(2);
        }
    };
    println!("{}", report.json_line());
    if !report.passed {
        exit(1);
    }
}
