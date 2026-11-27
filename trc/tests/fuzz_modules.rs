//! Deterministic fuzz harness for randomly generated import graphs.
//!
//! Each iteration writes a small set of module files into a private temporary
//! directory, then lexes, parses, and compiles a root program through the
//! bytecode compiler's module resolver. Cyclic and missing imports are
//! generated on purpose; they must produce structured errors, never panics or
//! hangs.
//!
//! The generator is a linear congruential generator with a fixed seed, so a
//! failure is reproducible from the iteration number alone. No external
//! crates and no host commands are used.

use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use trc::bytecode;
use trc::lexer;
use trc::parser;

/// 64-bit linear congruential generator (Numerical Recipes constants).
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg {
            state: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state >> 32
    }

    /// Inclusive range.
    fn range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() as usize % (hi - lo + 1))
    }
}

const PREFIX: &str = "fzmod";
const MODULE_COUNT: usize = 5;

fn module_name(index: usize) -> String {
    format!("{}_{}", PREFIX, index)
}

/// One import statement. Most point at a generated module (possibly the
/// importing module itself, forming a cycle); some are deliberately dangling
/// multi-segment paths or globs so the resolver's error paths are exercised.
fn random_import(rng: &mut Lcg) -> String {
    let target = module_name(rng.range(0, MODULE_COUNT - 1));
    match rng.range(0, 11) {
        0..=8 => format!("import {};\n", target),
        9 => format!("import {}::missing::Symbol;\n", target),
        10 => "import fzmod_sub::*;\n".to_string(),
        _ => "import missing_module;\n".to_string(),
    }
}

fn random_graph(rng: &mut Lcg) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = Vec::new();

    for index in 0..MODULE_COUNT {
        let mut source = String::new();
        let import_count = rng.range(0, 3);
        for _ in 0..import_count {
            source.push_str(&random_import(rng));
        }
        source.push_str(&format!(
            "public fn f{}(): int {{ return {}; }}\n",
            index, index
        ));
        files.push((format!("{}.tr", module_name(index)), source));
    }

    let mut main_source = String::new();
    let import_count = rng.range(1, 4);
    for _ in 0..import_count {
        main_source.push_str(&random_import(rng));
    }
    main_source.push_str("public fn main(): void { }\n");
    files.push(("main.tr".to_string(), main_source));

    files
}

fn temp_root(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("trc_fuzz_modules_{}_{}", std::process::id(), tag))
}

fn prepare_root(root: &PathBuf) {
    let _ = fs::remove_dir_all(root);
    fs::create_dir_all(root.join("fzmod_sub")).expect("create temporary module directory");
    fs::write(
        root.join("fzmod_sub").join("a.tr"),
        "public fn sub_a(): int { return 1; }\n",
    )
    .expect("write glob module a");
    fs::write(
        root.join("fzmod_sub").join("b.tr"),
        "public fn sub_b(): int { return 2; }\n",
    )
    .expect("write glob module b");
}

fn write_graph(root: &PathBuf, files: &[(String, String)]) {
    for (name, source) in files {
        fs::write(root.join(name), source)
            .unwrap_or_else(|error| panic!("write module '{}': {}", name, error));
    }
}

fn compile_graph(root: PathBuf) -> Result<(), String> {
    let main_source = fs::read_to_string(root.join("main.tr"))
        .map_err(|error| format!("read main.tr: {}", error))?;
    let tokens = lexer::tokenize(&main_source).map_err(|error| error.to_string())?;
    let program = parser::parse(tokens).map_err(|error| error.to_string())?;
    let mut compiler = bytecode::Compiler::new();
    compiler.compile_with_modules(&program, &root)?;
    Ok(())
}

fn compile_with_timeout(root: PathBuf) -> Result<Result<(), String>, &'static str> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let result = catch_unwind(AssertUnwindSafe(|| compile_graph(root)));
            let _ = tx.send(result);
        })
        .map_err(|_| "spawn-failed")?;

    match rx.recv_timeout(Duration::from_millis(1_000)) {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(_)) => Err("panic"),
        Err(mpsc::RecvTimeoutError::Timeout) => Err("timeout"),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err("panic"),
    }
}

const ITERATIONS: usize = 1_000;

#[test]
fn import_graph_fuzz_never_panics_or_hangs() {
    let root = temp_root("graph");
    prepare_root(&root);
    let mut rng = Lcg::new(0x494D_504F_5254_5331);
    let mut failures = 0usize;
    let mut succeeded = 0usize;

    for iteration in 0..ITERATIONS {
        let files = random_graph(&mut rng);
        write_graph(&root, &files);

        match compile_with_timeout(root.clone()) {
            Ok(Ok(())) => succeeded += 1,
            Ok(Err(_message)) => {
                // Missing modules and invalid globs are structured errors.
            }
            Err("timeout") => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Timeout on import graph fuzz iteration {}", iteration);
                }
            }
            Err("panic") => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Panic on import graph fuzz iteration {}", iteration);
                }
            }
            Err(other) => {
                failures += 1;
                if failures <= 3 {
                    eprintln!("Thread error on import graph fuzz iteration {}: {}", iteration, other);
                }
            }
        }
    }

    let _ = fs::remove_dir_all(&root);

    assert!(succeeded > 0, "generator produced no resolvable import graphs");
    assert_eq!(
        failures, 0,
        "import graph fuzz reported {} failures out of {} iterations",
        failures, ITERATIONS
    );
}

#[test]
fn cyclic_import_graph_never_hangs() {
    let root = temp_root("cycle");
    prepare_root(&root);

    write_graph(
        &root,
        &[
            (
                format!("{}.tr", module_name(0)),
                format!("import {};\npublic fn f0(): int {{ return 0; }}\n", module_name(1)),
            ),
            (
                format!("{}.tr", module_name(1)),
                format!("import {};\npublic fn f1(): int {{ return 1; }}\n", module_name(2)),
            ),
            (
                format!("{}.tr", module_name(2)),
                format!("import {};\npublic fn f2(): int {{ return 2; }}\n", module_name(0)),
            ),
            ("main.tr".to_string(), format!("import {};\npublic fn main(): void {{ }}\n", module_name(0))),
        ],
    );

    let outcome = compile_with_timeout(root.clone());
    let _ = fs::remove_dir_all(&root);

    match outcome {
        Ok(Ok(())) | Ok(Err(_)) => {}
        Err("timeout") => panic!("cyclic import graph hung the module resolver"),
        Err("panic") => panic!("cyclic import graph panicked the module resolver"),
        Err(other) => panic!("cyclic import graph thread error: {}", other),
    }
}
