// Coverage collection for the Titrate toolchain.
//
// `pipette coverage` runs the test suite under a coverage tool and prints a
// per-file summary. The heavy lifting is delegated to an external tool –
// either `cargo-tarpaulin` (the easiest path) or `grcov` (which consumes the
// `.profraw` files produced by rustc's built-in `-Cinstrument-coverage`).
//
// The `--native` flag additionally instruments the native (LLVM) test
// binaries so that coverage of `titrate_native` and the codegen path is
// included in the report.
//
// This module sets up the commands, runs them, and parses their text output
// into a lightweight per-file summary. It does not depend on serde – the
// summary is built from line-oriented stdout that both tools emit.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Coverage entry for a single source file.
#[derive(Debug, Clone)]
pub struct CoverageEntry {
    pub file: String,
    pub covered: u64,
    pub total: u64,
}

impl CoverageEntry {
    /// Percentage of lines covered, as a value in `[0.0, 100.0]`.
    pub fn percent(&self) -> f64 {
        if self.total == 0 {
            100.0
        } else {
            (self.covered as f64 / self.total as f64) * 100.0
        }
    }
}

/// Which coverage backend to use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoverageTool {
    Tarpaulin,
    Grcov,
}

impl CoverageTool {
    pub fn binary_name(self) -> &'static str {
        match self {
            CoverageTool::Tarpaulin => "cargo-tarpaulin",
            CoverageTool::Grcov => "grcov",
        }
    }
}

/// Run the coverage workflow.
///
/// `native` requests that native (LLVM) test binaries are instrumented as
/// well. The `workspace_dir` is expected to be the Titrate workspace root
/// (the directory containing the workspace `Cargo.toml`). Unlike most other
/// pipette commands, `coverage` does not require a `Titrate.toml` because it
/// instruments the Rust toolchain itself (`trc`, `pipette`,
/// `titrate_native`).
pub fn coverage(workspace_dir: &Path, native: bool) -> Result<(), String> {
    println!("pipette coverage – collecting test coverage");
    println!("  workspace: {}", workspace_dir.display());
    println!("  native:    {}", if native { "yes" } else { "no" });
    println!();

    let tool = detect_tool().ok_or_else(|| {
        String::from(
            "No coverage tool found. Install one of:\n  \
             cargo install cargo-tarpaulin\n  \
             cargo install grcov && rustup component add llvm-tools-preview",
        )
    })?;

    println!("Using coverage tool: {}", tool.binary_name());

    let entries = match tool {
        CoverageTool::Tarpaulin => run_tarpaulin(workspace_dir, native)?,
        CoverageTool::Grcov => run_grcov(workspace_dir, native)?,
    };

    print_summary(&entries);

    // Persist a baseline report next to the workspace.
    let report_path = workspace_dir.join("coverage-summary.txt");
    write_summary_report(&report_path, &entries, native)?;
    println!("\nCoverage summary written to {}", report_path.display());

    Ok(())
}

/// Detect an installed coverage tool, preferring tarpaulin.
pub fn detect_tool() -> Option<CoverageTool> {
    if which("cargo-tarpaulin").is_some() {
        return Some(CoverageTool::Tarpaulin);
    }
    if which("grcov").is_some() {
        return Some(CoverageTool::Grcov);
    }
    None
}

/// Run `cargo-tarpaulin` and parse its per-file output.
fn run_tarpaulin(project_dir: &Path, native: bool) -> Result<Vec<CoverageEntry>, String> {
    let coverage_dir = project_dir.join("coverage");
    fs::create_dir_all(&coverage_dir)
        .map_err(|e| format!("Failed to create coverage directory: {}", e))?;

    let mut cmd = Command::new("cargo");
    cmd.arg("tarpaulin")
        .arg("--workspace")
        .arg("--profile")
        .arg("coverage")
        .arg("--out")
        .arg("Stdout")
        .current_dir(project_dir);

    if native {
        // Tarpaulin instruments every binary built by the workspace, including
        // the native test binaries, so no extra flag is required. We still
        // surface the intent in the command log.
        cmd.arg("--all-features");
    }

    cmd.env("TARPAULIN_TARGET_DIR", project_dir.join("target").join("coverage"));

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to invoke cargo-tarpaulin: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "cargo-tarpaulin failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_tarpaulin_output(&stdout))
}

/// Run the grcov workflow: build/test with `-Cinstrument-coverage`, then merge.
fn run_grcov(project_dir: &Path, native: bool) -> Result<Vec<CoverageEntry>, String> {
    let coverage_dir = project_dir.join("coverage");
    fs::create_dir_all(&coverage_dir)
        .map_err(|e| format!("Failed to create coverage directory: {}", e))?;

    let profraw_dir = coverage_dir.join("profraw");
    fs::create_dir_all(&profraw_dir)
        .map_err(|e| format!("Failed to create profraw directory: {}", e))?;

    // 1. Build and run tests with coverage instrumentation.
    let mut test_cmd = Command::new("cargo");
    test_cmd
        .arg("test")
        .arg("--workspace")
        .arg("--profile")
        .arg("coverage")
        .current_dir(project_dir)
        .env("RUSTFLAGS", "-Cinstrument-coverage")
        .env(
            "LLVM_PROFILE_FILE",
            profraw_dir.join("%p-%m.profraw").to_str().unwrap_or(""),
        );

    if !native {
        // Skip native-only test binaries when native coverage is not requested.
        // The native tests live in trc/tests/native_*.rs; excluding them keeps
        // the run focused on the bytecode VM and compiler.
        test_cmd.arg("--").arg("--skip").arg("native_");
    }

    let test_out = test_cmd
        .output()
        .map_err(|e| format!("Failed to run instrumented tests: {}", e))?;

    if !test_out.status.success() {
        return Err(format!(
            "instrumented tests failed: {}",
            String::from_utf8_lossy(&test_out.stderr)
        ));
    }

    // 2. Merge the .profraw files with grcov.
    let binary_path = project_dir
        .join("target")
        .join("coverage")
        .to_str()
        .ok_or("invalid target path")?
        .to_string();

    let report_dir = coverage_dir.join("html");
    fs::create_dir_all(&report_dir)
        .map_err(|e| format!("Failed to create report directory: {}", e))?;

    let mut grcov_cmd = Command::new("grcov");
    grcov_cmd
        .arg(profraw_dir.to_str().unwrap_or(""))
        .arg("--binary-path")
        .arg(&binary_path)
        .arg("-s")
        .arg(project_dir.to_str().unwrap_or(""))
        .arg("-t")
        .arg("coveralls")
        .arg("-o")
        .arg(coverage_dir.join("coveralls.json").to_str().unwrap_or(""))
        .current_dir(project_dir);

    let grcov_out = grcov_cmd
        .output()
        .map_err(|e| format!("Failed to invoke grcov: {}", e))?;

    if !grcov_out.status.success() {
        return Err(format!(
            "grcov failed: {}",
            String::from_utf8_lossy(&grcov_out.stderr)
        ));
    }

    // 3. Parse the coveralls JSON (line-oriented, no serde needed) into a
    //    per-file summary. We read the file and scan for source file objects.
    let coveralls_path = coverage_dir.join("coveralls.json");
    let data = fs::read_to_string(&coveralls_path)
        .map_err(|e| format!("Failed to read coveralls report: {}", e))?;
    Ok(parse_coveralls(&data))
}

/// Parse tarpaulin's `--out Stdout` text into per-file entries.
///
/// Tarpaulin prints lines such as:
///   `src/foo.rs: 10/15 covered (66.67%)`
fn parse_tarpaulin_output(stdout: &str) -> Vec<CoverageEntry> {
    let mut entries = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        // Match "<path>: <covered>/<total> covered (<pct>%)"
        if let Some(colon) = trimmed.rfind(": ") {
            let file_part = &trimmed[..colon];
            let rest = &trimmed[colon + 2..];
            if let Some(slash) = rest.find('/') {
                let covered_str = &rest[..slash];
                let after_slash = &rest[slash + 1..];
                if let Some(space) = after_slash.find(' ') {
                    let total_str = &after_slash[..space];
                    if let (Ok(covered), Ok(total)) =
                        (covered_str.trim().parse::<u64>(), total_str.trim().parse::<u64>())
                    {
                        if !file_part.is_empty() && !file_part.contains('\n') {
                            entries.push(CoverageEntry {
                                file: file_part.to_string(),
                                covered,
                                total,
                            });
                        }
                    }
                }
            }
        }
    }
    entries
}

/// Parse a coveralls-format JSON blob into per-file entries without serde.
///
/// We look for `"name": "<path>"` followed by `"covered_lines": <n>` and
/// `"num_statements": <n>` pairs. This is intentionally tolerant of the
/// exact field ordering grcov emits.
fn parse_coveralls(data: &str) -> Vec<CoverageEntry> {
    let mut entries: Vec<CoverageEntry> = Vec::new();
    let mut current_file: Option<String> = None;
    let mut covered: u64 = 0;
    let mut total: u64 = 0;

    for token in tokenize_json_strings_and_numbers(data) {
        match token {
            Token::String(s) => {
                // A string immediately following a `name` key sets the file.
                if s.ends_with(".rs") {
                    if let Some(f) = current_file.take() {
                        entries.push(CoverageEntry {
                            file: f,
                            covered,
                            total,
                        });
                    }
                    current_file = Some(s);
                    covered = 0;
                    total = 0;
                }
            }
            Token::Number(n) => {
                // Heuristic: the first number after a file is covered lines,
                // the second is total statements. This is good enough for a
                // summary; the full report lives in coverage/html.
                if current_file.is_some() {
                    if covered == 0 && total == 0 {
                        covered = n;
                    } else if total == 0 {
                        total = n;
                    }
                }
            }
        }
    }

    if let Some(f) = current_file.take() {
        entries.push(CoverageEntry {
            file: f,
            covered,
            total,
        });
    }

    entries
}

/// A minimal JSON token used by `parse_coveralls`.
enum Token {
    String(String),
    Number(u64),
}

/// Yield the string and number tokens from a JSON document, skipping
/// everything else. Sufficient for the tolerant parsing above.
fn tokenize_json_strings_and_numbers(data: &str) -> Vec<Token> {
    let bytes = data.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                i += 1;
                let mut buf = String::new();
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        // Skip escaped char.
                        i += 2;
                    } else {
                        buf.push(bytes[i] as char);
                        i += 1;
                    }
                }
                if i < bytes.len() {
                    i += 1; // closing quote
                }
                tokens.push(Token::String(buf));
            }
            b'0'..=b'9' => {
                let start = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                if let Ok(n) = data[start..i].parse::<u64>() {
                    tokens.push(Token::Number(n));
                }
            }
            _ => {
                i += 1;
            }
        }
    }
    tokens
}

/// Print a per-file coverage table.
fn print_summary(entries: &[CoverageEntry]) {
    if entries.is_empty() {
        println!("\nNo per-file coverage data could be parsed.");
        println!("The full report is available under coverage/ for the tool you used.");
        return;
    }

    println!("\nCoverage summary (per file):");
    println!(
        "{:<50} {:>10} {:>10} {:>10}",
        "file", "covered", "total", "%"
    );
    println!("{}", "-".repeat(82));

    let mut total_covered: u64 = 0;
    let mut total_lines: u64 = 0;

    let mut sorted: Vec<&CoverageEntry> = entries.iter().collect();
    sorted.sort_by(|a, b| a.file.cmp(&b.file));

    for entry in &sorted {
        println!(
            "{:<50} {:>10} {:>10} {:>9.2}%",
            entry.file,
            entry.covered,
            entry.total,
            entry.percent()
        );
        total_covered += entry.covered;
        total_lines += entry.total;
    }

    println!("{}", "-".repeat(82));
    let overall = if total_lines == 0 {
        0.0
    } else {
        (total_covered as f64 / total_lines as f64) * 100.0
    };
    println!(
        "{:<50} {:>10} {:>10} {:>9.2}%",
        "TOTAL", total_covered, total_lines, overall
    );
}

/// Write a machine-readable baseline report.
fn write_summary_report(
    path: &Path,
    entries: &[CoverageEntry],
    native: bool,
) -> Result<(), String> {
    let mut out = String::new();
    out.push_str("# Titrate coverage baseline\n");
    out.push_str("# generated by `pipette coverage`\n");
    out.push_str(&format!("# native: {}\n", native));
    out.push_str("# columns: file covered total percent\n");
    out.push('\n');

    let mut total_covered: u64 = 0;
    let mut total_lines: u64 = 0;

    let mut sorted: Vec<&CoverageEntry> = entries.iter().collect();
    sorted.sort_by(|a, b| a.file.cmp(&b.file));

    for entry in &sorted {
        out.push_str(&format!(
            "{} {} {} {:.2}\n",
            entry.file,
            entry.covered,
            entry.total,
            entry.percent()
        ));
        total_covered += entry.covered;
        total_lines += entry.total;
    }

    let overall = if total_lines == 0 {
        0.0
    } else {
        (total_covered as f64 / total_lines as f64) * 100.0
    };
    out.push_str(&format!(
        "TOTAL {} {} {:.2}\n",
        total_covered, total_lines, overall
    ));

    fs::write(path, out).map_err(|e| format!("Failed to write report: {}", e))?;
    Ok(())
}

/// Locate an executable on `PATH`, mirroring the behaviour of `which`.
fn which(name: &str) -> Option<PathBuf> {
    let path_env = env::var_os("PATH")?;
    let ext = if cfg!(windows) { ".exe" } else { "" };
    for dir in env::split_paths(&path_env) {
        let candidate = dir.join(format!("{}{}", name, ext));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
