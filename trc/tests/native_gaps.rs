//! Regression tests for native/VM parity gaps closed in the container
//! reference-semantics work: invocable closures, interface dispatch through
//! fat pointers, and transparent container returns.
//!
//! Each test compiles a small Titrate program with `trc --native --release`,
//! runs the resulting binary, and asserts stdout matches the bytecode VM
//! byte-for-byte. The VM reference is produced in-process from the same source;
//! a captured literal is used only when that VM path is unavailable, and the
//! path actually used is reported per test.
//!
//! Each test checks at runtime for LLVM development files (LLVM-C.lib via
//! `LLVM_SYS_221_PREFIX` or `C:\Program Files\LLVM`) and the release
//! `titrate_native` library, and skips with a warning when either is absent.
//! Build the prerequisites with:
//!   cargo build -p titrate_native --release

pub mod native_capability;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use trc::analyzer;
use trc::bytecode;
use trc::lexer;
use trc::parser;

// Linking is serialized: parallel links of distinct binaries are fine, but
// lock contention on the linker/temp outputs has flaked before (LNK1104).
static LINK_LOCK: Mutex<()> = Mutex::new(());

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

/// Compile `source` natively, run the binary, and return its stdout with
/// line endings normalized. `name` must be unique per test so parallel runs
/// never share output files.
fn run_native(name: &str, source: &str) -> String {
    let _guard = LINK_LOCK.lock().unwrap();
    let trc = trc_binary().expect("trc binary not found; run `cargo build -p trc` first");

    let dir = env::temp_dir();
    let src_path = dir.join(format!("trc_gap_{}.tr", name));
    fs::write(&src_path, source).expect("failed to write repro source");

    let compile_output = Command::new(&trc)
        .arg("--native")
        .arg("--release")
        .arg(src_path.to_str().unwrap())
        .current_dir(&dir)
        .output()
        .expect("failed to invoke trc");
    assert!(
        compile_output.status.success(),
        "trc --native --release failed for {}.\nstdout: {}\nstderr: {}",
        name,
        String::from_utf8_lossy(&compile_output.stdout),
        String::from_utf8_lossy(&compile_output.stderr),
    );

    let exe_name = if cfg!(windows) {
        format!("trc_gap_{}_native.exe", name)
    } else {
        format!("trc_gap_{}_native", name)
    };
    let native_exe = dir.join(&exe_name);
    assert!(
        native_exe.is_file(),
        "native binary was not produced at {}",
        native_exe.display(),
    );

    let run_output = Command::new(&native_exe)
        .current_dir(&dir)
        .output()
        .expect("failed to run the native binary");
    assert!(
        run_output.status.success(),
        "native binary exited with status {:?}\nstdout: {}\nstderr: {}",
        run_output.status.code(),
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr),
    );

    let stdout = String::from_utf8_lossy(&run_output.stdout)
        .replace("\r\n", "\n");
    let _ = fs::remove_file(&src_path);
    let _ = fs::remove_file(&native_exe);
    stdout
}

/// Compile and run `source` in-process through the bytecode VM, returning its
/// stdout with a trailing newline so it is byte-comparable to native stdout.
fn run_vm(name: &str, source: &str) -> Result<String, String> {
    let tokens = lexer::tokenize(source).map_err(|e| format!("lexer: {}", e))?;
    let ast = parser::parse(tokens).map_err(|e| format!("parser: {}", e))?;
    let typed = analyzer::analyze(&ast).map_err(|e| format!("analyzer: {:?}", e))?;
    let lines = bytecode::execute(&typed)
        .map_err(|e| format!("execute ({}): {}", name, e))?;
    let mut stdout = lines.join("\n");
    if !lines.is_empty() {
        stdout.push('\n');
    }
    Ok(stdout.replace("\r\n", "\n"))
}

fn assert_vm_parity(name: &str, source: &str, expected: &str) {
    if !native_capability::native_ready(native_capability::LibProfile::Release) {
        return;
    }
    let actual = run_native(name, source);
    match run_vm(name, source) {
        Ok(vm) => {
            eprintln!(
                "native_gaps[{}]: compared against live in-process bytecode VM output",
                name
            );
            assert_eq!(
                actual, vm,
                "native stdout must match bytecode VM stdout byte-for-byte for {}",
                name
            );
        }
        Err(e) => {
            eprintln!(
                "native_gaps[{}]: VM reference unavailable ({}); fell back to captured VM literal",
                name, e
            );
            assert_eq!(
                actual.trim_end(),
                expected.trim_end(),
                "native output must match captured VM literal for {}",
                name
            );
        }
    }
}

#[test]
fn gap_closure_container_param() {
    assert_vm_parity(
        "closure_container",
        r#"public fn main(): void {
    let bump = fn(xs: ArrayList<int>): void { xs.add(99); };
    let nums = new ArrayList<int>();
    nums.add(1);
    bump(nums);
    io::println(nums.size());
    io::println(nums.get(1));
    let inc = fn(x: int): int { return x + 1; };
    io::println(inc(41));
}
"#,
        "2\n99\n42\n",
    );
}

#[test]
fn gap_interface_container_param() {
    assert_vm_parity(
        "iface_container",
        r#"interface Sink {
    fn consume(items: ArrayList<int>): void;
}

public class VecSink implements Sink {
    public var total: int;

    public fn init() {
        this.total = 0;
    }

    public fn consume(items: ArrayList<int>): void {
        items.add(100);
        this.total = items.size();
    }
}

fn run(s: Sink, xs: ArrayList<int>): void {
    s.consume(xs);
}

public fn main(): void {
    let xs = new ArrayList<int>();
    xs.add(1);
    run(new VecSink(), xs);
    io::println(xs.size());
    io::println(xs.get(1));
}
"#,
        "2\n100\n",
    );
}

#[test]
fn gap_interface_plain_dispatch() {
    assert_vm_parity(
        "iface_plain",
        r#"interface Greeter {
    fn greet(): string;
}

public class Hi implements Greeter {
    public fn init() {
    }

    public fn greet(): string {
        return "hi";
    }
}

fn useG(g: Greeter): void {
    io::println(g.greet());
}

public fn main(): void {
    useG(new Hi());
}
"#,
        "hi\n",
    );
}

#[test]
fn gap_return_passthrough() {
    assert_vm_parity(
        "ret_passthrough",
        r#"fn make(): ArrayList<int> {
    let xs = new ArrayList<int>();
    xs.add(5);
    return xs;
}

fn ident(xs: ArrayList<int>): ArrayList<int> {
    return xs;
}

public fn main(): void {
    let a = make();
    a.add(6);
    io::println(a.size());
    io::println(a.get(0));
    io::println(a.get(1));
    let b = new ArrayList<int>();
    b.add(7);
    let c = ident(b);
    c.add(8);
    io::println(b.size());
    io::println(c.size());
}
"#,
        "2\n5\n6\n2\n2\n",
    );
}

#[test]
fn gap_return_method_and_transitive() {
    assert_vm_parity(
        "ret_method_transitive",
        r#"public class Box {
    public var n: int;

    public fn init() {
        this.n = 0;
    }

    public fn idem(v: ArrayList<int>): ArrayList<int> {
        return v;
    }
}

fn wrap(v: ArrayList<int>): ArrayList<int> {
    return v;
}

fn wrapwrap(v: ArrayList<int>): ArrayList<int> {
    return wrap(v);
}

public fn main(): void {
    let b = new ArrayList<int>();
    b.add(1);
    let c = wrap(b);
    c.add(2);
    io::println(b.size());
    let bx = new Box();
    let d = bx.idem(b);
    d.add(3);
    io::println(b.size());
    let e = wrapwrap(b);
    e.add(4);
    io::println(b.size());
}
"#,
        "2\n3\n4\n",
    );
}

#[test]
fn gap_return_passthrough_runs_once() {
    // A transparent call must still execute exactly once (side effects kept)
    // while its result aliases the argument's slot.
    assert_vm_parity(
        "ret_effects",
        r#"fn noisy(v: ArrayList<int>): ArrayList<int> {
    io::println("noisy ran");
    return v;
}

public fn main(): void {
    let b = new ArrayList<int>();
    b.add(1);
    let c = noisy(b);
    c.add(2);
    io::println(b.size());
    io::println(c.size());
}
"#,
        "noisy ran\n2\n2\n",
    );
}

#[test]
fn gap_return_override_veto() {
    // Sub overrides the passthrough with a fresh list: no aliasing, the
    // argument is untouched.
    assert_vm_parity(
        "ret_override_veto",
        r#"public class Base {
    public fn idem(v: ArrayList<int>): ArrayList<int> {
        return v;
    }
}

public class Sub extends Base {
    public fn idem(v: ArrayList<int>): ArrayList<int> {
        let fresh = new ArrayList<int>();
        fresh.add(999);
        return fresh;
    }
}

fn getSub(): Sub {
    return new Sub();
}

public fn main(): void {
    let b = new ArrayList<int>();
    b.add(1);
    let c = getSub().idem(b);
    io::println(c.size());
    io::println(c.get(0));
    io::println(b.size());
}
"#,
        "1\n999\n1\n",
    );
}

#[test]
fn gap_return_virtual_veto() {
    // Inherited passthrough dispatched virtually to an overriding subclass:
    // the override wins, no aliasing.
    assert_vm_parity(
        "ret_virtual_veto",
        r#"public class Base {
    public fn idem(v: ArrayList<int>): ArrayList<int> {
        return v;
    }
}

public class Child extends Base {
}

public class SubChild extends Child {
    public fn idem(v: ArrayList<int>): ArrayList<int> {
        let fresh = new ArrayList<int>();
        fresh.add(999);
        return fresh;
    }
}

fn run(c: Child, b: ArrayList<int>): void {
    let r = c.idem(b);
    io::println(r.size());
    io::println(r.get(0));
    io::println(b.size());
}

public fn main(): void {
    let b = new ArrayList<int>();
    b.add(1);
    run(new SubChild(), b);
}
"#,
        "1\n999\n1\n",
    );
}

#[test]
fn gap_virtual_inherited_dispatch() {
    assert_vm_parity(
        "virtual_inherited",
        r#"public class Base {
    public fn who(): string {
        return "base";
    }

    public fn idem(v: ArrayList<int>): ArrayList<int> {
        io::println("base-idem");
        return v;
    }
}

public class Child extends Base {
}

public class SubChild extends Child {
    public fn idem(v: ArrayList<int>): ArrayList<int> {
        io::println("sub-idem");
        let fresh = new ArrayList<int>();
        fresh.add(999);
        return fresh;
    }
}

fn run(c: Child, b: ArrayList<int>): void {
    io::println(c.who());
    let r = c.idem(b);
    io::println(r.size());
}

public fn main(): void {
    let b = new ArrayList<int>();
    b.add(1);
    io::println("---sub---");
    run(new SubChild(), b);
    io::println("---child---");
    run(new Child(), b);
}
"#,
        "---sub---\nbase\nsub-idem\n1\n---child---\nbase\nbase-idem\n1\n",
    );
}

#[test]
fn gap_fn_field_call_with_capture() {
    // A function-valued field holds a {fn_ptr, capture} closure struct. Calls
    // through it must dispatch via the closure trampoline, not the vtable of
    // the receiver's class (R4).
    assert_vm_parity(
        "fn_field_capture",
        r#"public class Runner {
    private fn(): void cb;
    public fn init(f: fn(): void) { this.cb = f; }
    public fn run(): void {
        if (this.cb != null) { this.cb(); }
    }
}

public fn main(): void {
    let n = 41;
    let r = new Runner(fn(): void { io::println(n + 1); });
    r.run();
}
"#,
        "42\n",
    );
}

#[test]
fn gap_fn_field_inherited() {
    // The fn field is declared on the parent but invoked from a method on the
    // child; field lookup must walk the inheritance chain and the trampoline
    // must receive the closure's capture buffer as its hidden first argument.
    assert_vm_parity(
        "fn_field_inherited",
        r#"public class BaseRunner {
    public fn(int): int cb;
    public fn set(f: fn(int): int): void { this.cb = f; }
}

public class DerivedRunner extends BaseRunner {
    public fn apply(x: int): int { return this.cb(x); }
}

public fn main(): void {
    let k = 10;
    let r = new DerivedRunner();
    r.set(fn(x: int): int { return x + k; });
    io::println(r.apply(5));
}
"#,
        "15\n",
    );
}
