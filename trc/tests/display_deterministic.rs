//! Verifies that bytecode-VM display of class instances and maps is
//! deterministic across runs (declaration/insertion order must not depend on
//! randomized HashMap iteration order).

use std::path::PathBuf;

use trc::bytecode;
use trc::lexer;
use trc::parser;

const SOURCE: &str = r#"
class Record {
    public int zeta;
    public int alpha;
    public int mu;
    public int beta;
    public int omega;
    public int gamma;
    public int delta;
    public int kappa;

    public fn init() {
        this.zeta = 26;
        this.alpha = 1;
        this.mu = 13;
        this.beta = 2;
        this.omega = 24;
        this.gamma = 3;
        this.delta = 4;
        this.kappa = 11;
    }
}

public fn main(): void {
    let r = new Record();
    io::println(r);

    let m = new HashMap<string, int>();
    m.put("zeta", 26);
    m.put("alpha", 1);
    m.put("mu", 13);
    m.put("beta", 2);
    m.put("omega", 24);
    m.put("gamma", 3);
    m.put("delta", 4);
    m.put("kappa", 11);
    io::println(m);
}
"#;

fn run_program() -> String {
    let tokens = lexer::tokenize(SOURCE).expect("lexer failed");
    let ast = parser::parse(tokens).expect("parser failed");
    let root_dir = PathBuf::from("..");
    let mut compiler = bytecode::Compiler::new();
    let compiled = compiler
        .compile_with_modules(&ast, &root_dir)
        .expect("compiler failed");
    let mut vm = bytecode::Vm::new();
    vm.set_working_dir(root_dir);
    vm.load_program(compiled);
    vm.run().expect("vm failed");
    vm.output.join("\n")
}

#[test]
fn display_is_identical_across_runs() {
    const RUNS: usize = 32;

    let first = run_program();
    for run in 1..RUNS {
        let output = run_program();
        assert_eq!(
            output, first,
            "display changed between run 0 and run {run}"
        );
    }

    let lines: Vec<&str> = first.lines().collect();
    assert_eq!(lines.len(), 2, "expected class line and map line, got {first:?}");

    // Field-name sort order, not declaration order.
    assert_eq!(
        lines[0],
        "Record(alpha: 1, beta: 2, delta: 4, gamma: 3, kappa: 11, mu: 13, omega: 24, zeta: 26)"
    );

    // Map fields are a HashMap too: "_keys" sorts before "_values".
    let keys_pos = lines[1]
        .find("_keys")
        .expect("map display should expose _keys");
    let values_pos = lines[1]
        .find("_values")
        .expect("map display should expose _values");
    assert!(keys_pos < values_pos, "map fields not in sorted order: {}", lines[1]);

    // Map entries themselves keep insertion order.
    assert!(
        lines[1].contains("[zeta, alpha, mu, beta, omega, gamma, delta, kappa]"),
        "map keys not in insertion order: {}",
        lines[1]
    );
}
