# Release Profiles: LTO and PGO

Two opt-in Cargo profiles produce faster native release builds of the compiler
and build tool. Both inherit from `release`, so optimization level, debug info,
panic strategy, and overflow checks are unchanged. Only whole-program
optimization differs.

| Profile | Build | Purpose |
|---------|-------|---------|
| `lto` | `cargo build --profile lto --workspace` | Fat link-time optimization with a single codegen unit |
| `pgo` | `cargo build --profile pgo --workspace` | Separate output directory for the two profile-guided build phases |

## Profile declarations

```toml
[profile.lto]
inherits = "release"
lto = "fat"
codegen-units = 1

[profile.pgo]
inherits = "release"
```

Cargo only reads `[profile.*]` sections from the **workspace root** manifest.
Keep one authoritative copy of these declarations in the root `Cargo.toml`;
identical sections in member manifests are ignored with a warning. The
existing `[profile.coverage]` definition is unchanged.

## LTO build

From the workspace root:

```bash
cargo build --profile lto --workspace
```

To build a single crate:

```bash
cargo build --profile lto -p trc
cargo build --profile lto -p pipette
```

Binaries land in `target/lto/`. Fat LTO performs whole-program optimization at
link time and `codegen-units = 1` gives LLVM the full crate at once. Expect
noticeably longer compile and link times, especially on a clean build; runtime
performance and binary size are the payoff.

## PGO workflow

The `pgo` profile deliberately contains **no** hard-coded compiler flags. The
same profile is used twice — once to emit profile data and once to consume it —
so the differentiating flags come from `RUSTFLAGS` at each invocation. Hard-
coding either flag in the manifest would make the other phase impossible to
build with the same profile.

### Prerequisites

```bash
rustup component add llvm-tools-preview
```

This installs `llvm-profdata`, which merges raw profile files. It is not a
rustup shim; locate it under `$(rustc --print sysroot)/lib/rustlib/` (the
`<host-triple>/bin/` subdirectory).

The examples below use PowerShell (Windows). Bash equivalents follow where the
syntax differs.

### 1. Build the instrumented binaries

```powershell
$env:RUSTFLAGS = "-Cprofile-generate=$PWD\pgo-data"
cargo build --profile pgo --workspace
$env:RUSTFLAGS = ""
```

```bash
RUSTFLAGS="-Cprofile-generate=$PWD/pgo-data" cargo build --profile pgo --workspace
```

Create `pgo-data` first if the compiler does not create it automatically:

```powershell
New-Item -ItemType Directory -Force pgo-data | Out-Null
```

### 2. Train on a representative workload

Run the instrumented binaries in `target/pgo/` on inputs that exercise the hot
paths you want to optimize. For the compiler and VM, the checked-in examples
are a reasonable starting point:

```powershell
& .\target\pgo\trc.exe examples\sorting.tr
& .\target\pgo\trc.exe examples\json_parse.tr
& .\target\pgo\trc.exe examples\math_demo.tr
```

```bash
./target/pgo/trc examples/sorting.tr
./target/pgo/trc examples/json_parse.tr
./target/pgo/trc examples/math_demo.tr
```

Add your own large, realistic programs; short hello-world runs produce little
useful data. Each run writes `*.profraw` files into `pgo-data/`.

If you exercise the native (LLVM) backend during training, note that `trc` only
searches `target/release` and `target/debug` for the native library. Copy
`titrate_native.lib` (Windows) or `libtitrate_native.a` (Unix) from
`target/pgo/` into `target/release/` first, or train through the bytecode VM
instead.

### 3. Merge the raw profiles

```powershell
$profdata = (Get-ChildItem (Join-Path (rustc --print sysroot) 'lib\rustlib') -Recurse -Filter llvm-profdata.exe | Select-Object -First 1).FullName
& $profdata merge -o pgo-data\merged.profdata (Get-ChildItem pgo-data\*.profraw | ForEach-Object FullName)
```

```bash
HOST="$(rustc -vV | sed -n 's/^host: //p')"
PROFDATA="$(rustc --print sysroot)/lib/rustlib/$HOST/bin/llvm-profdata"
"$PROFDATA" merge -o pgo-data/merged.profdata pgo-data/*.profraw
```

### 4. Build with the profile data

```powershell
$env:RUSTFLAGS = "-Cprofile-use=$PWD\pgo-data\merged.profdata"
cargo build --profile pgo --workspace
$env:RUSTFLAGS = ""
```

```bash
RUSTFLAGS="-Cprofile-use=$PWD/pgo-data/merged.profdata" cargo build --profile pgo --workspace
```

The optimized binaries replace the instrumented ones in `target/pgo/`. Changing
`RUSTFLAGS` changes Cargo's fingerprint, so each step rebuilds the workspace;
this is expected. Clear `RUSTFLAGS` before unrelated builds.

To combine profile guidance with LTO, keep the `-Cprofile-use` flag and select
the other profile:

```bash
RUSTFLAGS="-Cprofile-use=$PWD/pgo-data/merged.profdata" cargo build --profile lto --workspace
```
