//! Regression tests for the G1 native-codegen crash wave.
//!
//! Each test compiles a small program with `trc --emit-ir --compat` and checks
//! the emitted IR. Compiling in a subprocess means an LLVM signature assertion
//! (which aborts the process) is reported as a test failure instead of taking
//! down the test runner.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("trc is inside the workspace")
        .to_path_buf()
}

fn trc_binary() -> PathBuf {
    let exe = if cfg!(windows) { "trc.exe" } else { "trc" };
    for profile in ["debug", "release"] {
        let candidate = workspace_root().join("target").join(profile).join(exe);
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!("trc binary not built");
}

struct CompileResult {
    ok: bool,
    output: String,
    ir: String,
}

fn compile_ir(name: &str, source: &str) -> CompileResult {
    let dir = workspace_root().join("target").join("g1_regressions");
    fs::create_dir_all(&dir).expect("create scratch dir");
    let src = dir.join(format!("{}.tr", name));
    fs::write(&src, source).expect("write source");
    let output = Command::new(trc_binary())
        .arg("--emit-ir")
        .arg("--compat")
        .arg(&src)
        .current_dir(&dir)
        .output()
        .expect("run trc");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let ir_path = dir.join(format!("{}.ll", name));
    let ir = fs::read_to_string(&ir_path).unwrap_or_default();
    CompileResult {
        ok: output.status.success(),
        output: text,
        ir,
    }
}

#[test]
fn optional_map_specialization_uses_concrete_closure_return() {
    let result = compile_ir(
        "g1_optional_map",
        r#"
import tt::lang::Optional;
public fn main(): void {
    let opt: Optional<int> = Optional<int>::of(5);
    let mapper: fn(int): int = fn(x: int): int { return x * 2; };
    let mapped: Optional<int> = opt.map(mapper);
    io::println(mapped.get());
}
"#,
    );
    assert!(result.ok, "compile failed: {}", result.output);
    assert!(
        result.ir.contains("Optional__int_map__int"),
        "missing specialized map symbol:\n{}",
        result.ir
    );
    assert!(
        result.ir.contains("call i32 %closure.fn"),
        "specialized map still used a wrong-width closure ABI:\n{}",
        result.ir
    );
}

#[test]
fn optional_map_chained_filter_keeps_specialized_receiver_type() {
    let result = compile_ir(
        "g1_optional_chain",
        r#"
import tt::lang::Optional;
public fn main(): void {
    let opt: Optional<int> = Optional<int>::of(5);
    let mapper: fn(int): int = fn(x: int): int { return x * 2; };
    let pred: fn(int): bool = fn(x: int): bool { return x > 5; };
    let mapped: Optional<int> = opt.map(mapper).filter(pred);
    io::println(mapped.isPresent());
}
"#,
    );
    assert!(result.ok, "compile failed: {}", result.output);
    assert!(
        !result.output.contains("cannot be dispatched"),
        "chained filter lost its receiver type: {}",
        result.output
    );
    assert!(
        result.ir.contains("Optional__int_filter"),
        "missing specialized filter dispatch:\n{}",
        result.ir
    );
}

#[test]
fn variant_bool_cast_extracts_tag_word_without_utf8_decode() {
    let result = compile_ir(
        "g1_variant_bool",
        r#"
import tt::lang::Variant;
public fn main(): void {
    let flag: Variant = true;
    let b: bool = flag as bool;
    io::println(Boolean.toString(b));
}
"#,
    );
    assert!(result.ok, "compile failed: {}", result.output);
    assert!(
        result.ir.contains("cast.variant.bool"),
        "bool cast did not extract the Variant tag word:\n{}",
        result.ir
    );
}

#[test]
fn i32_string_length_is_widened_before_scalar_compare() {
    let result = compile_ir(
        "g1_string_len_width",
        r#"
import tt::lang::Result;
import tt::lang::Optional;
public fn main(): void {
    let wrapped: Optional = Optional.of(Result.ok(7));
    io::println("ok");
}
"#,
    );
    assert!(result.ok, "compile failed: {}", result.output);
    assert!(
        result.ir.contains("str.scalar.len64"),
        "string length was not normalized before i64 compares:\n{}",
        result.ir
    );
}

#[test]
fn variant_literals_box_int_and_string_payloads() {
    let result = compile_ir(
        "g1_variant_literals",
        r#"
import tt::lang::Variant;
public fn main(): void {
    let a: Variant = 21;
    let b: Variant = "boom";
    let c: Variant = 0;
    io::println("ok");
}
"#,
    );
    assert!(result.ok, "compile failed: {}", result.output);
    assert!(
        result.ir.contains(".str.variant.literal"),
        "string literal was not boxed into the Variant slot:\n{}",
        result.ir
    );
    assert!(
        result.ir.contains(".titrate.numeric.box"),
        "integer literal was not boxed into the Variant slot:\n{}",
        result.ir
    );
}

#[test]
fn nested_closure_captures_transitive_outer_parameter() {
    let result = compile_ir(
        "g1_nested_capture",
        r#"
import tt::util::FunctionExt;
public fn main(): void {
    run_all_functionext_tests();
}
"#,
    );
    assert!(result.ok, "compile failed: {}", result.output);
    assert!(
        !result.output.contains("unknown variable 'a'"),
        "nested closure lost its outer parameter: {}",
        result.output
    );
}
