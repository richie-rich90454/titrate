//! Deterministic PRNG, iteration configuration, and JSON reporting shared by
//! the verification fuzz harnesses. Standard library only.

use std::env;
use std::fmt::Write as _;
use std::path::PathBuf;

/// Fixed iteration count used when the CI gate script does not override it.
pub const DEFAULT_ITERATIONS: usize = 64;
/// Seed used by every harness when the caller does not supply one.
pub const DEFAULT_SEED: u64 = 0x4655_5A5A_5652_4331;
/// Environment variable the CI gate script uses to pin the iteration count.
pub const ENV_ITERATIONS: &str = "TITRATE_FUZZ_ITERATIONS";
/// Prefix that makes harness output machine-readable without publishing counts.
pub const JSON_PREFIX: &str = "TRC_FUZZ_JSON:";

/// 64-bit linear congruential generator (Numerical Recipes constants). The
/// low bits of an LCG are poorly distributed, so derived values use the high
/// half of the state.
pub struct Lcg {
    state: u64,
}

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state >> 32
    }

    /// Inclusive range.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() as usize % (hi - lo + 1))
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        assert!(!items.is_empty(), "fuzz generator has an empty choice set");
        let index = self.range(0, items.len() - 1);
        &items[index]
    }

    pub fn boolean(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// Fill a byte buffer with high-quality bits from successive draws.
    pub fn fill(&mut self, out: &mut [u8]) {
        for byte in out {
            *byte = (self.next_u64() >> 56) as u8;
        }
    }
}

/// Read the pinned iteration count. A malformed override is an error rather
/// than a silent fallback, so a misconfigured CI run cannot claim coverage it
/// did not perform.
pub fn iteration_count(default: usize) -> Result<usize, String> {
    match env::var(ENV_ITERATIONS) {
        Ok(raw) => {
            let count: usize = raw.parse().map_err(|error| {
                format!("{}: invalid iteration count '{}': {}", ENV_ITERATIONS, raw, error)
            })?;
            if !(1..=1_000_000).contains(&count) {
                return Err(format!(
                    "{}: iteration count must be in 1..=1000000, got {}",
                    ENV_ITERATIONS, count
                ));
            }
            Ok(count)
        }
        Err(env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(format!("{}: {}", ENV_ITERATIONS, error)),
    }
}

/// Workspace root, discovered from the tracked handoff document so the same
/// helper works in the fuzz crate and in `trc` integration tests.
pub fn workspace_root() -> PathBuf {
    let start = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in start.ancestors() {
        if ancestor.join("HANDOFF.md").is_file() {
            return ancestor.to_path_buf();
        }
    }
    start
}

/// One harness result. Deliberately carries no measured numbers: CI checks
/// status only, so no timing or coverage figure is published by accident.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessReport {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}

impl HarnessReport {
    pub fn pass(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            error: None,
        }
    }

    pub fn fail(name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            error: Some(error.into()),
        }
    }

    /// Compact single-line JSON payload (without the prefix).
    pub fn json(&self) -> String {
        let mut out = String::from("{\"schema\":\"titrate-fuzz-v1\",\"name\":");
        let _ = write!(out, "{},\"status\":\"{}\"", json_string(&self.name), if self.passed { "pass" } else { "fail" });
        if let Some(error) = &self.error {
            let _ = write!(out, ",\"error\":{}", json_string(error));
        }
        out.push('}');
        out
    }

    /// Full machine-readable line consumed by scripts/verify-fuzz.ps1.
    pub fn json_line(&self) -> String {
        format!("{}{}", JSON_PREFIX, self.json())
    }
}

/// Minimal JSON string escaping; avoids adding a serialization dependency.
pub fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
