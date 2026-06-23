// Titrate build tool – pipette core library
// Precision in every step – richie-rich90454, 2026

pub mod config;
pub mod project;

pub mod build;
pub mod run;
pub mod test_runner;
pub mod doc;
pub mod clean;
pub mod lint;
pub mod format;
pub mod bench;
pub mod deps;
pub mod watch;
pub mod serialize;
pub mod coverage;

// Re-export public API so that main.rs can use `pipette::build`, etc.
pub use build::{build, build_with_profile};
pub use run::run;
pub use test_runner::test;
pub use doc::doc;
pub use clean::clean;
pub use lint::lint;
pub use format::fmt;
pub use bench::{bench, bench_compare_native, bench_native_vs_bytecode};
pub use deps::{outdated, tree};
pub use watch::watch;
pub use coverage::coverage;

// ---------------------------------------------------------------------------
// Build Profile
// ---------------------------------------------------------------------------

/// Build profile controlling optimization level and debug info.
#[derive(Debug, Clone, PartialEq)]
pub enum BuildProfile {
    Debug,
    Release,
}

impl std::fmt::Display for BuildProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildProfile::Debug => write!(f, "debug"),
            BuildProfile::Release => write!(f, "release"),
        }
    }
}

impl std::str::FromStr for BuildProfile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "debug" => Ok(BuildProfile::Debug),
            "release" => Ok(BuildProfile::Release),
            _ => Err(format!("Unknown build profile: '{}'. Use 'debug' or 'release'.", s)),
        }
    }
}
