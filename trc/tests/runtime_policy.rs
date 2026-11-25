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

// Denied families, shared by the direct policy check and the compiled-program
// checks so the two cannot drift apart.
const DANGEROUS_NATIVES: &[&str] = &[
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

#[test]
fn untrusted_denies_dangerous_natives_trusted_allows() {
    let untrusted = RuntimePolicy::untrusted(sandbox());
    for &name in DANGEROUS_NATIVES {
        assert!(!untrusted.allows_native(name), "{} must be denied", name);
    }
    let trusted = RuntimePolicy::trusted();
    for &name in DANGEROUS_NATIVES {
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
    vm.load_program(compile_source("public fn main(): void { println(\"hello world\"); }"));
    let err = vm.run().expect_err("output budget must stop the program");
    assert!(err.contains("output budget"), "unexpected error: {}", err);

    assert!(
        natives::checked_len(32, "test").is_err(),
        "allocation budget must be applied to native reads"
    );
    assert!(natives::checked_len(8, "test").is_ok());
}

fn run_untrusted(src: &str) -> Result<(), String> {
    let mut vm = bytecode::Vm::new();
    vm.set_policy(RuntimePolicy::untrusted(sandbox()));
    vm.load_program(compile_source(src));
    vm.run()
}

#[test]
fn untrusted_compiled_subprocess_native_fails_fatally() {
    let err = run_untrusted("public fn main(): void { Subprocess_runFull(\"echo\"); }")
        .expect_err("subprocess native must be denied");
    assert!(err.contains("Subprocess_runFull"), "unexpected error: {}", err);
    assert!(err.contains("denied"), "unexpected error: {}", err);
}

#[test]
fn untrusted_compiled_network_natives_fail_fatally() {
    for (call, native) in [
        ("Net_connect(\"example.com\", 80)", "Net_connect"),
        ("Http_get(\"http://example.com\")", "Http_get"),
    ] {
        let src = format!("public fn main(): void {{ {}; }}", call);
        let err = run_untrusted(&src).expect_err("network native must be denied");
        assert!(err.contains(native), "unexpected error: {}", err);
        assert!(err.contains("denied"), "unexpected error: {}", err);
    }
}

#[test]
fn untrusted_compiled_ffi_native_fails_fatally() {
    let err = run_untrusted("public fn main(): void { Ctypes_dlopen(\"libc.so.6\"); }")
        .expect_err("FFI native must be denied");
    assert!(err.contains("Ctypes_dlopen"), "unexpected error: {}", err);
    assert!(err.contains("denied"), "unexpected error: {}", err);
}

#[test]
fn untrusted_compiled_denials_cover_all_dangerous_families() {
    // A bare `Class_method(...)` call is lowered to STATIC_CALL("Class", "method");
    // the gate checks the reconstructed `Class_method` name.
    for &native in DANGEROUS_NATIVES {
        let src = format!("public fn main(): void {{ {}(\"x\"); }}", native);
        let err = run_untrusted(&src)
            .expect_err("dangerous native must be denied when reached from compiled code");
        assert!(err.contains(native), "unexpected error for {}: {}", native, err);
        assert!(err.contains("denied"), "unexpected error for {}: {}", native, err);
    }
}

#[test]
fn untrusted_denies_native_reachable_only_through_builtin_fallback() {
    // Subprocess_runFull is in the builtin lookup table, but a fresh VM does
    // not register it. A compiled call can reach it only through the
    // lookup_builtin_native fallback in exec_static_call.
    assert!(natives::lookup_builtin_native("Subprocess_runFull").is_some());
    let mut trusted = bytecode::Vm::new();
    let err = trusted
        .call_native_by_name("Subprocess_runFull", &[])
        .expect_err("fallback native must not be pre-registered");
    assert!(
        err.contains("Unknown native function"),
        "unexpected error: {}",
        err
    );

    let err = run_untrusted("public fn main(): void { Subprocess_runFull(\"echo\"); }")
        .expect_err("fallback native must be denied");
    assert!(err.contains("Subprocess_runFull"), "unexpected error: {}", err);
    assert!(err.contains("denied"), "unexpected error: {}", err);
    assert!(
        !err.contains("Unknown static call"),
        "gate must deny before the fallback lookup: {}",
        err
    );
}

#[test]
fn untrusted_compiled_file_read_is_confined_to_sandbox_root() {
    let root = std::env::temp_dir().join(format!("titrate_sandbox_{}", std::process::id()));
    let shadow = root.join("etc").join("passwd");
    std::fs::create_dir_all(shadow.parent().expect("sandbox parent")).expect("create sandbox dir");
    std::fs::write(&shadow, "SANDBOX_SENTINEL").expect("write sandbox file");

    let src = "public fn main(): void { println(File_readFile(\"/etc/passwd\")); }";
    let mut vm = bytecode::Vm::new();
    vm.set_policy(RuntimePolicy::untrusted(root.clone()));
    vm.load_program(compile_source(src));
    vm.run().expect("confined file read must not be fatal");
    assert_eq!(vm.output, vec!["SANDBOX_SENTINEL".to_string()]);

    std::fs::remove_dir_all(&root).expect("remove sandbox dir");
}

#[test]
fn trusted_mode_compiles_and_loads_denied_native_sources() {
    let sources = [
        "public fn main(): void { Subprocess_runFull(\"echo\"); }",
        "public fn main(): void { Net_connect(\"example.com\", 80); }",
        "public fn main(): void { Http_get(\"http://example.com\"); }",
        "public fn main(): void { Ctypes_dlopen(\"libc.so.6\"); }",
    ];
    for src in sources {
        let compiled = compile_source(src);
        let mut vm = bytecode::Vm::new();
        vm.set_policy(RuntimePolicy::trusted());
        vm.load_program(compiled);
    }
}
