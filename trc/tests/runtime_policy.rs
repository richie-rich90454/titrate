use std::path::PathBuf;

use trc::bytecode;
use trc::bytecode::vm::natives;
use trc::lexer;
use trc::parser;
use trc::runtime_policy::{ResourceBudgets, RuntimePolicy};

fn compile_source(src: &str) -> bytecode::CompiledProgram {
    let tokens = lexer::tokenize(src).expect("lex error");
    let ast = parser::parse(tokens).expect("parse error");
    let mut compiler = bytecode::Compiler::new();
    compiler
        .compile_with_modules(&ast, std::path::Path::new(".."))
        .expect("compiler error")
}

fn sandbox() -> PathBuf {
    std::env::current_dir().expect("current dir")
}

#[test]
fn untrusted_denies_dangerous_natives_trusted_allows() {
    let denied = [
        "Ctypes_dlopen",
        "Ctypes_call",
        "Subprocess_run",
        "Process_spawn",
        "Signal_raise",
        "Os_kill",
        "Sys_exit",
        "Net_connect",
        "Http_get",
        "Socket_new",
        "UdpSocket_new",
        "Ssl_connect",
        "Ftp_connect",
        "WinReg_OpenKey",
    ];
    let untrusted = RuntimePolicy::untrusted(sandbox());
    for name in denied {
        assert!(!untrusted.allows_native(name), "{} must be denied", name);
    }
    let trusted = RuntimePolicy::trusted();
    for name in denied {
        assert!(trusted.allows_native(name), "{} must be allowed", name);
    }
    assert!(untrusted.allows_native("println"));
    assert!(untrusted.allows_native("Math_sqrt"));
}

#[test]
fn vm_blocks_denied_native_dispatch() {
    let mut vm = bytecode::Vm::new();
    vm.set_policy(RuntimePolicy::untrusted(sandbox()));
    let err = vm
        .call_native_by_name("Net_connect", &[])
        .expect_err("network native must be denied");
    assert!(err.contains("denied"), "unexpected error: {}", err);

    let mut trusted = bytecode::Vm::new();
    let err = trusted
        .call_native_by_name("Net_connect", &[])
        .expect_err("native still fails on bad arguments");
    assert!(!err.contains("denied"), "trusted VM must not gate the native");
}

#[test]
fn untrusted_confines_paths_trusted_does_not() {
    let root = sandbox().join("sandbox");
    natives::set_native_working_dir(None);

    natives::set_sandbox_root(Some(root.clone()));
    let absolute = natives::resolve_path("/etc/passwd");
    assert!(absolute.starts_with(&root), "absolute path escaped: {:?}", absolute);
    assert_ne!(absolute, PathBuf::from("/etc/passwd"));
    let traversal = natives::resolve_path("../../etc/passwd");
    assert!(traversal.starts_with(&root), "traversal escaped: {:?}", traversal);

    natives::set_sandbox_root(None);
    assert_eq!(
        natives::resolve_path("/etc/passwd"),
        PathBuf::from("/etc/passwd")
    );
}

#[test]
fn untrusted_step_cap_is_fatal() {
    let mut vm = bytecode::Vm::new();
    vm.set_policy(RuntimePolicy::untrusted(sandbox()));
    vm.load_program(compile_source(
        "public fn main(): void { try { while (true) { } } catch (e: string) { io::println(\"caught\"); } }",
    ));
    let err = vm.run().expect_err("step cap must stop the program");
    assert!(err.contains("step limit"), "unexpected error: {}", err);
    assert!(!vm.output.iter().any(|line| line.contains("caught")));
}

#[test]
fn untrusted_output_and_allocation_budgets_are_enforced() {
    let mut vm = bytecode::Vm::new();
    vm.set_policy(
        RuntimePolicy::untrusted(sandbox()).with_budgets(ResourceBudgets {
            max_steps: 1_000_000,
            max_output_bytes: 8,
            max_allocation: 16,
        }),
    );
    vm.load_program(compile_source("public fn main(): void { io::println(\"hello world\"); }"));
    let err = vm.run().expect_err("output budget must stop the program");
    assert!(err.contains("output budget"), "unexpected error: {}", err);

    assert!(
        natives::checked_len(32, "test").is_err(),
        "allocation budget must be applied to native reads"
    );
    assert!(natives::checked_len(8, "test").is_ok());
}
