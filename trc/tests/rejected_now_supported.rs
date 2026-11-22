//! Three-way output parity for constructs the bytecode VM previously rejected.
//!
//! Each construct is run from the same Titrate source through the bytecode VM,
//! the LLVM native backend (`--native --release`) and the WASM host. The
//! non-ignored tests cover VM/WASM parity on every `cargo test` run; the
//! `#[ignore]`d tests add the native leg, which needs LLVM dev files, a system
//! linker and a prebuilt `cargo build -p titrate_native --release` library.

mod wasm_common;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

// Linking is serialized: parallel links of distinct binaries flake with LNK1104
// on Windows.
static LINK_LOCK: Mutex<()> = Mutex::new(());

const STRING_FOR_IN: &str = r#"
public fn main(): void {
    for (c in "abc") {
        io::println(c);
    }
}
"#;

const PRINTLN_REF: &str = r#"
public fn main(): void {
    let value = 42;
    let r = &value;
    io::println(r);
}
"#;

const WITH_NON_RESOURCE: &str = r#"
public fn main(): void {
    with (42) {
        io::println("in with");
    }
    io::println("done");
}
"#;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("trc lives inside the workspace")
        .to_path_buf()
}

fn trc_binary() -> PathBuf {
    let root = workspace_root();
    let exe = if cfg!(windows) { "trc.exe" } else { "trc" };
    for profile in &["debug", "release"] {
        let p = root.join("target").join(profile).join(exe);
        if p.is_file() {
            return p;
        }
    }
    panic!("trc binary not found; run `cargo build -p trc` first");
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn wasm_stdout(src: &str) -> String {
    let lines = wasm_common::run_wasm(&wasm_common::compile_wasm(src));
    lines.into_iter().map(|l| l + "\n").collect()
}

fn run_vm_binary(trc: &Path, dir: &Path, stem: &str) -> String {
    let source = dir.join(format!("{}.tr", stem));
    let out = Command::new(trc)
        .arg(&source)
        .current_dir(dir)
        .output()
        .expect("run bytecode VM");
    assert!(out.status.success(), "VM failed:\n{}", normalize(&out.stderr));
    normalize(&out.stdout)
}

fn run_native_binary(trc: &Path, dir: &Path, stem: &str) -> String {
    let source = dir.join(format!("{}.tr", stem));
    let out = Command::new(trc)
        .arg("--native")
        .arg("--release")
        .arg(&source)
        .current_dir(dir)
        .output()
        .expect("compile native");
    assert!(
        out.status.success(),
        "native compile failed:\n{}",
        normalize(&out.stderr)
    );
    let exe = if cfg!(windows) {
        dir.join(format!("{}_native.exe", stem))
    } else {
        dir.join(format!("{}_native", stem))
    };
    let run = Command::new(&exe).current_dir(dir).output().expect("run native");
    assert!(
        run.status.success(),
        "native run failed:\n{}",
        normalize(&run.stderr)
    );
    normalize(&run.stdout)
}

/// Run the source on all three backends and assert byte-identical stdout.
#[allow(dead_code)]
fn assert_three_way_parity(src: &str, stem: &str) {
    let _guard = LINK_LOCK.lock().unwrap();
    let trc = trc_binary();
    let dir = workspace_root().join("target").join("rns_parity");
    fs::create_dir_all(&dir).expect("create scratch dir");
    let source = dir.join(format!("{}.tr", stem));
    fs::write(&source, src).expect("write source");

    let vm = run_vm_binary(&trc, &dir, stem);
    let native = run_native_binary(&trc, &dir, stem);
    let wasm = wasm_stdout(src);

    assert_eq!(vm, wasm, "bytecode VM / WASM output diverged");
    assert_eq!(native, wasm, "native / WASM output diverged");
}

// --- Fast parity legs (always run) -----------------------------------------

#[test]
fn string_for_in_vm_matches_wasm() {
    assert_eq!(
        wasm_common::run_vm(STRING_FOR_IN),
        vec!["a", "b", "c"],
    );
    assert_eq!(
        wasm_common::run_wasm(&wasm_common::compile_wasm(STRING_FOR_IN)),
        vec!["a", "b", "c"],
    );
}

#[test]
fn println_ref_vm_matches_wasm() {
    assert_eq!(wasm_common::run_vm(PRINTLN_REF), vec!["ref(0)"]);
    assert_eq!(
        wasm_common::run_wasm(&wasm_common::compile_wasm(PRINTLN_REF)),
        vec!["ref(0)"],
    );
}

#[test]
fn with_non_resource_vm_matches_wasm() {
    assert_eq!(
        wasm_common::run_vm(WITH_NON_RESOURCE),
        vec!["in with", "done"],
    );
    assert_eq!(
        wasm_common::run_wasm(&wasm_common::compile_wasm(WITH_NON_RESOURCE)),
        vec!["in with", "done"],
    );
}

// --- Full three-way parity (needs LLVM + native lib) -----------------------

#[test]
#[ignore = "needs LLVM, a linker, and titrate_native built in release mode"]
fn string_for_in_three_way_parity() {
    assert_three_way_parity(STRING_FOR_IN, "rns_string_for_in");
}

#[test]
#[ignore = "needs LLVM, a linker, and titrate_native built in release mode"]
fn println_ref_three_way_parity() {
    assert_three_way_parity(PRINTLN_REF, "rns_println_ref");
}

#[test]
#[ignore = "needs LLVM, a linker, and titrate_native built in release mode"]
fn with_non_resource_three_way_parity() {
    assert_three_way_parity(WITH_NON_RESOURCE, "rns_with_non_resource");
}
