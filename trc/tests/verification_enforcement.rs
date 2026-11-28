//! Phase 3 verification enforcement.
//!
//! Part 1 checks that the gates recorded in HANDOFF section 0 are declared in
//! CI workflows, and that the fuzz gate script drives every harness.
//! Part 2 checks that each harness reports malformed inputs with descriptive
//! errors instead of panicking or silently accepting them.
//!
//! A workspace-wide `cargo test --all` in CI declares every per-target test
//! gate; that subsumption is checked explicitly rather than pretending each
//! named test target has its own workflow step.

#[allow(dead_code)]
#[path = "../fuzz/common.rs"]
mod common;
#[allow(dead_code)]
#[path = "../fuzz/abi.rs"]
mod abi;
#[allow(dead_code)]
#[path = "../fuzz/expressions.rs"]
mod expressions;
#[allow(dead_code)]
#[path = "../fuzz/module_graphs.rs"]
mod module_graphs;
#[allow(dead_code)]
#[path = "../fuzz/differential.rs"]
mod differential;

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    common::workspace_root()
}

/// Backtick-quoted cargo commands in the HANDOFF ground-truth table.
fn handoff_gate_commands() -> Vec<String> {
    let handoff = fs::read_to_string(workspace_root().join("HANDOFF.md"))
        .expect("HANDOFF.md should be readable");
    let section = handoff
        .split("## 0. Ground truth")
        .nth(1)
        .expect("HANDOFF section 0 should exist");
    let section = section.split("\n---").next().unwrap_or(section);
    section
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::trim)
        .filter(|command| command.starts_with("cargo "))
        .map(str::to_string)
        .collect()
}

fn normalize(command: &str) -> Vec<String> {
    let mut tokens: Vec<String> = command
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|ch| ch == '`' || ch == '"' || ch == '\'')
                .to_string()
        })
        .filter(|token| !token.is_empty())
        .collect();
    if let Some(position) = tokens.iter().position(|token| token == "--jobs") {
        tokens.remove(position);
        if tokens.get(position).is_some_and(|token| token == "1") {
            tokens.remove(position);
        }
    }
    tokens
}

/// Shell command lines that invoke cargo in any workflow.
fn ci_commands() -> Vec<Vec<String>> {
    let workflows = workspace_root().join(".github/workflows");
    let mut commands = Vec::new();
    for entry in fs::read_dir(&workflows).expect(".github/workflows should be readable") {
        let path = entry.expect("valid workflow directory entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yml") {
            continue;
        }
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {}", path.display(), error));
        for line in source.lines() {
            let Some(start) = line.find("cargo ") else {
                continue;
            };
            let mut command = &line[start..];
            if let Some(comment) = command.find(" #") {
                command = &command[..comment];
            }
            command = command.split(" 2>&1").next().unwrap_or(command);
            command = command.split(" | ").next().unwrap_or(command);
            let tokens = normalize(command);
            if !tokens.is_empty() {
                commands.push(tokens);
            }
        }
    }
    commands
}

/// A CI command covers a gate when they are the same command (ignoring the
/// Windows-only `--jobs 1` flag) or the CI command runs the whole workspace
/// for the same cargo subcommand.
fn covers(gate: &[String], ci_commands: &[Vec<String>]) -> bool {
    let mut gate_tokens = gate.to_vec();
    gate_tokens.sort_unstable();
    ci_commands.iter().any(|ci| {
        let mut ci_tokens = ci.clone();
        ci_tokens.sort_unstable();
        if gate_tokens == ci_tokens {
            return true;
        }
        let subcommand = gate.get(1).map(String::as_str).unwrap_or("");
        matches!(subcommand, "build" | "test" | "clippy")
            && ci.get(1).map(String::as_str) == Some(subcommand)
            && ci.iter().any(|token| token == "--all")
    })
}

#[test]
fn handoff_ground_truth_gates_are_declared_in_ci() {
    let gates = handoff_gate_commands();
    assert!(
        gates.len() >= 8,
        "parsed only {} cargo commands from HANDOFF section 0; gate parser is broken",
        gates.len()
    );
    let ci = ci_commands();
    assert!(!ci.is_empty(), "no cargo commands found in CI workflows");
    let missing: Vec<&String> = gates
        .iter()
        .filter(|gate| !covers(&normalize(gate), &ci))
        .collect();
    assert!(
        missing.is_empty(),
        "HANDOFF section 0 gates are not declared in CI: {:?}",
        missing
    );
}

#[test]
fn ci_declares_workspace_build_test_and_clippy_umbrellas() {
    let source = fs::read_to_string(workspace_root().join(".github/workflows/ci.yml"))
        .expect("ci.yml should be readable");
    for gate in [
        "cargo build --all --jobs 1",
        "cargo test --all --jobs 1",
        "cargo clippy --all -- -D warnings",
        "cargo build -p titrate_native --release --jobs 1",
    ] {
        assert!(source.contains(gate), "ci.yml does not declare '{}'", gate);
    }
}

#[test]
fn ci_gate_script_declares_every_fuzz_harness() {
    let script = fs::read_to_string(workspace_root().join("scripts/verify-fuzz.ps1"))
        .expect("scripts/verify-fuzz.ps1 should be readable");
    for name in ["native_args", "expressions", "module_graphs", "differential"] {
        assert!(
            script.contains(name),
            "fuzz gate script does not run the '{}' harness",
            name
        );
    }
    assert!(script.contains("--iterations"), "fuzz gate must pin iterations");
    assert!(
        script.contains("TRC_FUZZ_JSON:"),
        "fuzz gate must consume machine-readable harness output"
    );
}

#[test]
fn native_argument_decoder_rejects_malformed_buffers_with_descriptive_errors() {
    for (label, bytes) in abi::malformed_cases() {
        let error = abi::decode(&bytes)
            .err()
            .unwrap_or_else(|| panic!("malformed case '{}' was accepted", label));
        assert!(
            abi::is_descriptive_error(&error),
            "malformed case '{}' produced an undescriptive error: {:?}",
            label,
            error
        );
    }
    let empty_string = abi::encode(abi::TAG_STRING, &abi::length_pointer_payload(0, 0));
    assert_eq!(
        abi::decode(&empty_string).expect("empty string header is valid"),
        abi::Decoded::Str {
            len: 0,
            has_data: false
        }
    );
}

#[test]
fn expression_harness_reports_malformed_fragment_as_structured_error() {
    let error = expressions::run_fragment(expressions::division_by_zero_fragment())
        .expect_err("a literal division by zero must not evaluate silently");
    let lowercase = error.to_ascii_lowercase();
    assert!(
        lowercase.contains("division") || lowercase.contains("zero"),
        "structured error does not name the defect: {}",
        error
    );
}

#[test]
fn module_graph_harness_reports_unresolvable_import() {
    let root = module_graphs::temp_root("missing_import_test");
    module_graphs::write_graph(&root, &module_graphs::missing_import_case())
        .expect("write missing-import graph");
    let outcome = module_graphs::compile_graph_at(&root);
    let _ = fs::remove_dir_all(&root);
    let error = outcome.expect_err("an unresolvable import must be a structured error");
    assert!(
        error.contains("Cannot resolve module"),
        "unexpected module resolution error: {}",
        error
    );
}

#[test]
fn differential_harness_rejects_malformed_source() {
    let error = differential::analyze_source("\"unterminated")
        .expect_err("an unterminated string must be rejected before backend comparison");
    assert!(
        error.starts_with("lexer:") && error.to_ascii_lowercase().contains("unterminated"),
        "unexpected lexer error: {}",
        error
    );
}
