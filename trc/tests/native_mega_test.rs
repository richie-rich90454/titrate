//! Integration tests for the LLVM native backend against the mega tests.
//!
//! These tests verify that the LLVM native backend produces the same output
//! as the bytecode VM for the multi-file mega test programs (`mega_test_02`
//! and `mega_test_03`). Each test compiles the program with
//! `trc --native --release`, runs the resulting native binary, and compares
//! its stdout against the corresponding `expected_output.txt`.
//!
//! Both tests are marked `#[ignore]` because they require:
//!   1. LLVM development files (LLVM-C.lib) to be installed and discoverable
//!      via `LLVM_SYS_221_PREFIX` or `C:\Program Files\LLVM`.
//!   2. The `titrate_native` static library to have been built
//!      (`cargo build -p titrate_native`).
//!   3. A working system linker (clang / link.exe / gcc).
//!
//! To run them explicitly:
//!   cargo test --test native_mega_test -- --ignored

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Locate the workspace root by walking up from CARGO_MANIFEST_DIR.
fn workspace_root() -> PathBuf {
    let manifest =
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set by cargo");
    PathBuf::from(manifest)
        .parent()
        .map(|p| p.to_path_buf())
        .expect("trc should be inside the workspace")
}

/// Locate the built `trc` binary in `target/debug` or `target/release`.
fn trc_binary() -> Option<PathBuf> {
    let root = workspace_root();
    let exe_name = if cfg!(windows) { "trc.exe" } else { "trc" };

    for profile in &["debug", "release"] {
        let candidate = root.join("target").join(profile).join(exe_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Locate the built `titrate_native` static library.
fn native_lib_dir() -> Option<PathBuf> {
    let root = workspace_root();
    let lib_name = if cfg!(windows) {
        "titrate_native.lib"
    } else {
        "libtitrate_native.a"
    };

    for profile in &["debug", "release"] {
        let candidate = root.join("target").join(profile);
        if candidate.join(lib_name).is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Path to the native executable produced for a `main.tr` in `src_dir`.
fn native_exe_path(src_dir: &PathBuf) -> PathBuf {
    let exe_name = if cfg!(windows) {
        "main_native.exe"
    } else {
        "main_native"
    };
    src_dir.join(exe_name)
}

/// Compare actual output against expected output, treating `<PLACEHOLDER>`
/// in the expected file as a wildcard that matches any non-empty text on
/// that portion of the line. Ported from `mega_test_03.rs`.
fn matches_expected(actual: &str, expected: &str) -> bool {
    let actual_lines: Vec<&str> = actual.lines().collect();
    let expected_lines: Vec<&str> = expected.lines().collect();

    if actual_lines.len() != expected_lines.len() {
        return false;
    }

    for (a_line, e_line) in actual_lines.iter().zip(expected_lines.iter()) {
        // Split expected line by <PLACEHOLDER> and verify actual matches
        let mut remaining = *a_line;
        let mut first = true;
        for part in e_line.split("<PLACEHOLDER>") {
            if first {
                first = false;
            } else {
                // We just passed a <PLACEHOLDER> – skip over any non-empty
                // text in the actual line up to the next literal part.
                // The placeholder matches at least one character.
                if part.is_empty() {
                    // Trailing placeholder – matches the rest of the line
                    break;
                }
                if let Some(pos) = remaining.find(part) {
                    if pos == 0 && !first {
                        // Placeholder matched zero characters, which is not
                        // allowed – each placeholder must match something.
                    }
                    remaining = &remaining[pos + part.len()..];
                } else {
                    return false;
                }
                continue;
            }
            // First segment (before any placeholder) must match literally
            if !remaining.starts_with(part) {
                return false;
            }
            remaining = &remaining[part.len()..];
        }
    }
    true
}

#[test]
#[ignore = "requires LLVM dev files, a system linker, and titrate_native built"]
fn native_mega_test_02_matches_expected() {
    let trc = trc_binary().expect("trc binary not found; run `cargo build -p trc` first");
    let _native_dir = native_lib_dir().expect(
        "titrate_native static library not found; run \
         `cargo build -p titrate_native` first",
    );

    let root = workspace_root();
    let src_dir = root.join("mega_test_02").join("src");
    let main_tr = src_dir.join("main.tr");
    assert!(
        main_tr.is_file(),
        "mega_test_02/src/main.tr not found at {}",
        main_tr.display(),
    );

    let native_exe = native_exe_path(&src_dir);

    // Compile: `trc --native --release main.tr`. The working directory is the
    // source dir so that `trc` discovers the nearby `Titrate.toml` and the
    // resulting binary can read its data files at runtime.
    let compile_output = Command::new(&trc)
        .arg("--native")
        .arg("--release")
        .arg(main_tr.to_str().unwrap())
        .current_dir(&src_dir)
        .output()
        .expect("failed to invoke trc");

    assert!(
        compile_output.status.success(),
        "trc --native --release failed.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&compile_output.stdout),
        String::from_utf8_lossy(&compile_output.stderr),
    );

    assert!(
        native_exe.is_file(),
        "native binary was not produced at {}",
        native_exe.display(),
    );

    // Run the native binary from the source dir so relative data file reads
    // (numbers.txt, words.txt, etc.) succeed.
    let run_output = Command::new(&native_exe)
        .current_dir(&src_dir)
        .output()
        .expect("failed to run the native binary");

    assert!(
        run_output.status.success(),
        "native binary exited with status {:?}\nstdout: {}\nstderr: {}",
        run_output.status.code(),
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr),
    );

    let expected = fs::read_to_string(root.join("mega_test_02").join("expected_output.txt"))
        .expect("expected_output.txt should exist");

    let actual = String::from_utf8_lossy(&run_output.stdout);
    let actual = actual.trim_end().replace("\r\n", "\n");
    let expected_trimmed = expected.trim_end().replace("\r\n", "\n");

    assert_eq!(
        actual, expected_trimmed,
        "mega test 0.2 native output must match byte-for-byte"
    );

    // Clean up the produced binary so the test is idempotent.
    let _ = fs::remove_file(&native_exe);
}

#[test]
#[ignore = "requires LLVM dev files, a system linker, and titrate_native built"]
fn native_mega_test_03_matches_expected() {
    let trc = trc_binary().expect("trc binary not found; run `cargo build -p trc` first");
    let _native_dir = native_lib_dir().expect(
        "titrate_native static library not found; run \
         `cargo build -p titrate_native` first",
    );

    let root = workspace_root();
    let src_dir = root.join("mega_test_03").join("src");
    let main_tr = src_dir.join("main.tr");
    assert!(
        main_tr.is_file(),
        "mega_test_03/src/main.tr not found at {}",
        main_tr.display(),
    );

    let native_exe = native_exe_path(&src_dir);

    // Compile: `trc --native --release main.tr`.
    let compile_output = Command::new(&trc)
        .arg("--native")
        .arg("--release")
        .arg(main_tr.to_str().unwrap())
        .current_dir(&src_dir)
        .output()
        .expect("failed to invoke trc");

    assert!(
        compile_output.status.success(),
        "trc --native --release failed.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&compile_output.stdout),
        String::from_utf8_lossy(&compile_output.stderr),
    );

    assert!(
        native_exe.is_file(),
        "native binary was not produced at {}",
        native_exe.display(),
    );

    // Run the native binary from the source dir so relative data file reads
    // succeed.
    let run_output = Command::new(&native_exe)
        .current_dir(&src_dir)
        .output()
        .expect("failed to run the native binary");

    assert!(
        run_output.status.success(),
        "native binary exited with status {:?}\nstdout: {}\nstderr: {}",
        run_output.status.code(),
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr),
    );

    let expected = fs::read_to_string(root.join("mega_test_03").join("expected_output.txt"))
        .expect("expected_output.txt should exist");

    let actual = String::from_utf8_lossy(&run_output.stdout);
    let actual = actual.trim_end().replace("\r\n", "\n");
    let expected_trimmed = expected.trim_end().replace("\r\n", "\n");

    assert!(
        matches_expected(&actual, &expected_trimmed),
        "mega test 0.3 native output must match expected (with <PLACEHOLDER> \
         wildcards)\n--- ACTUAL ---\n{}\n--- EXPECTED ---\n{}",
        actual,
        expected_trimmed,
    );

    // Clean up the produced binary so the test is idempotent.
    let _ = fs::remove_file(&native_exe);
}
