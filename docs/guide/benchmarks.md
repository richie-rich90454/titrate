# Benchmarks

The `trc` crate ships Criterion benchmarks for representative bytecode VM
workloads: recursive Fibonacci, sum of squares, string concatenation,
ArrayList build/iterate, HashMap put/get, and NDArray-style nested loops.
Each benchmark embeds its Titrate source and measures the compile + execute
pipeline through `trc::bytecode::execute`. No network or file access is
involved.

## Running

```bash
cargo bench -p trc
```

To compile the benchmarks without running them (used in CI to catch breakage):

```bash
cargo bench -p trc --no-run
```

Criterion writes detailed results under `target/criterion/`.

## Publishing numbers

Benchmark output is not published automatically. Before sharing any figures,
record them together with:

- CPU model and core count
- Amount of RAM
- Operating system and version
- Rust toolchain version (`rustc --version`)
- Titrate commit SHA
- The exact command used and any relevant build flags

Numbers from different machines or toolchains are not comparable. Always
quote the full environment alongside a result, and prefer a range across
several runs over a single sample.
