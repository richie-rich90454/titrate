//! Regression tests for native/VM parity gaps closed in the container
//! reference-semantics work: invocable closures, interface dispatch through
//! fat pointers, and transparent container returns.
//!
//! Each test compiles a small Titrate program with `trc --native --release`,
//! runs the resulting binary, and asserts stdout matches the bytecode VM
//! byte-for-byte (expected outputs were captured from the VM).
//!
//! All tests are marked `#[ignore]` because they require:
//!   1. LLVM development files (LLVM-C.lib) to be installed and discoverable
//!      via `LLVM_SYS_221_PREFIX` or `C:\Program Files\LLVM`.
//!   2. The `titrate_native` static library built in release mode:
//!      `cargo build -p titrate_native --release`.
//!   3. A working system linker (clang / link.exe / gcc).
//!
//! Run them explicitly with:
//!   cargo test --test native_gaps -- --ignored

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

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

fn assert_vm_parity(name: &str, source: &str, expected: &str) {
    let actual = run_native(name, source);
    assert_eq!(
        actual.trim_end(),
        expected.trim_end(),
        "native output must match the VM for {}",
        name
    );
}

#[test]
#[ignore = "requires LLVM dev files, a system linker, and titrate_native built"]
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
#[ignore = "requires LLVM dev files, a system linker, and titrate_native built"]
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
#[ignore = "requires LLVM dev files, a system linker, and titrate_native built"]
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
#[ignore = "requires LLVM dev files, a system linker, and titrate_native built"]
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
#[ignore = "requires LLVM dev files, a system linker, and titrate_native built"]
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
#[ignore = "requires LLVM dev files, a system linker, and titrate_native built"]
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
#[ignore = "requires LLVM dev files, a system linker, and titrate_native built"]
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
#[ignore = "requires LLVM dev files, a system linker, and titrate_native built"]
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
#[ignore = "requires LLVM dev files, a system linker, and titrate_native built"]
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
