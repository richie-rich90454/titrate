# Building Native Binaries

This guide covers everything you need to compile Titrate programs to
native executables: prerequisites, basic and release builds, the
`pipette` build tool integration, project-level linker configuration,
and troubleshooting the most common issues.

## Prerequisites

The native backend needs three things the bytecode VM does not:

1. **LLVM 15 or newer** — the compiler uses the `inkwell` crate, which
   links against `LLVM-C.lib` and the LLVM headers. LLVM 15, 16, and 17
   are all known to work.
2. **A system linker** — `link.exe` (MSVC) on Windows, `clang`/`gcc`/
   `ld` on Linux, or `ld64`/`clang` on macOS. The `lld` linker from
   the LLVM project also works on every platform.
3. **The `titrate_native` static library** — run `cargo build -p titrate_native` to produce it. This
   yields the static library (`libtitrate_native.a` on Unix,
   `titrate_native.lib` on Windows) in `target/debug/`. The
   `crate-type = ["staticlib", "rlib"]` declaration in
   `titrate_native/Cargo.toml` ensures both the static library and the
   in-tree `rlib` are produced.

You can verify LLVM is installed and visible by running:

```bash
llvm-config --version    # should print 15.x.x or higher
```

### Installing LLVM

#### Windows

The easiest path is the official installer from
<https://releases.llvm.org/>, or via Chocolatey:

```powershell
choco install llvm -y
```

Make sure `LLVM-C.lib` and the `llvm-c/` headers are on your `LIB` and
`INCLUDE` paths. The MSVC build tools (`link.exe`) come with Visual
Studio Build Tools or the full Visual Studio installation.

#### Linux (apt / Debian / Ubuntu)

```bash
sudo apt install llvm-15-dev libpolly-15-dev clang-15 lld-15
```

For other LLVM versions, substitute the version number. The
`llvm-config-15` binary must be on your `PATH`.

#### Linux (dnf / Fedora / RHEL)

```bash
sudo dnf install llvm-devel clang lld
```

#### macOS (Homebrew)

```bash
brew install llvm@15
```

Homebrew doesn't symlink LLVM by default, so you'll need to put it on
your path or set `LLVM_SYS_150_PREFIX`:

```bash
export LLVM_SYS_150_PREFIX=/opt/homebrew/opt/llvm@15
```

Apple's built-in `ld64` works as the linker; you don't need to install
`lld` separately on macOS.

## Basic Compilation

Use `trc` directly with the `--native` flag:

```bash
# Debug build (no LLVM optimizations)
trc --native hello.tr
```

This produces a native executable in the same directory as the source
file. On Windows it's `hello.exe`; on Linux and macOS it's `hello` with
no extension.

The debug build is a faithful, unoptimized translation of your source.
It's useful for inspecting what the codegen produces, but it's **not
representative of native performance** — for that you need release mode.

## Release Mode

```bash
# Release build (enables LLVM optimization passes)
trc --native --release hello.tr
```

Release mode turns on the optimizations that make native compilation
worthwhile:

| Optimization | Effect |
|---|---|
| `alwaysinline` attribute | Forces inlining of small internal functions, eliminating call overhead for helpers like `distance()` and `sqrt()`. |
| `fastcc` calling convention | Uses a faster calling convention for internal functions. |
| `llvm.loop.vectorize.enable` metadata | Hints the LLVM loop vectorizer to auto-vectorize `for` and `while` loops. |
| `llvm.loop.vectorize.width` metadata | Suggests a vectorization width of 4 (suitable for SSE/AVX). |
| `llvm.memset.p0i8.i64` intrinsic | Zero-initializes class allocations with a single memset call. |
| Pointer-arithmetic array loops | Hoists base pointers and increments by element size, eliminating repeated index multiplication. |

If your release build isn't dramatically faster than your debug build,
your program is probably I/O-bound or the hot loop can't be vectorized.
See [Native MD Simulation](./native-md) for profiling tips.

## Using pipette

The `pipette` build tool supports the native backend end-to-end. From
any project with a `Titrate.toml`:

```bash
# Build the project natively (release mode by default)
pipette build --native

# Build and run
pipette run --native

# Side-by-side benchmark: bytecode vs native
pipette bench --compare-native
```

`pipette build --native` produces the executable in the project's
`target/` directory, mirroring the layout used for bytecode builds.

## Titrate.toml Configuration

Project-level configuration lives in `Titrate.toml`. The `[native]`
section controls linker flags and optional native-build settings:

```toml
[package]
name = "myproject"
version = "0.1.0"
entry = "src/main.tr"

[native]
# Extra flags passed to the linker when building a native executable.
# Use this to link against system C libraries your program needs.
linker_flags = ["-lm", "-lpthread"]

# Optional: override the system linker. Defaults to the platform default
# (link.exe on Windows, clang/gcc on Linux, clang on macOS).
linker = "clang"

# Optional: emit LLVM IR alongside the executable for inspection.
emit_ir = false
```

The `linker_flags` field is the most commonly used — it lets you pull in
C libraries that your `@native`-annotated functions call into. See
[Wrapping C Libraries](./native-cbind) for the full story on FFI.

## Inspecting the Generated IR

The `--emit-ir` flag writes the LLVM IR for a program to a `.ll` file
beside the source. With `--native`, the IR is written before the linker
is invoked; without `--native`, only the `.ll` file is written and the
compiler exits without linking.

```bash
# Emit IR and continue to a native executable
trc --native --emit-ir examples/hello.tr
```

This writes the IR to `examples/hello.ll`. For an optimized IR dump,
add `--release`:

```bash
trc --native --release --emit-ir program.tr
```

Look for:

- `define internal ... @distance(...)` with `alwaysinline` — confirms
  inlining hints are applied.
- `call fastcc` — confirms the fast calling convention.
- `!llvm.loop !{!"llvm.loop.vectorize.enable", i1 true}` — confirms
  vectorization metadata.
- `call void @llvm.memset.p0i8.i64(...)` — confirms memset zero-init.

The IR is the ground truth for what the optimizer did. When in doubt
about whether an optimization fired, read the IR.

## Troubleshooting

### "LLVM-C.lib not found"

The `inkwell` crate can't find the LLVM development files. Fix:

- **Windows**: install LLVM via Chocolatey or the official installer,
  and make sure `LLVM-C.lib` is on your `LIB` path.
- **Linux**: install the `llvm-15-dev` (or equivalent) package.
- **macOS**: `brew install llvm@15` and set
  `LLVM_SYS_150_PREFIX=/opt/homebrew/opt/llvm@15`.

Verify with `llvm-config --version`.

### "link.exe / clang / gcc not found"

The linker isn't on your `PATH`.

- **Windows**: install Visual Studio Build Tools, or run from a
  "Developer Command Prompt" that sets up the MSVC environment.
- **Linux/macOS**: install `clang` or `gcc` via your package manager.
  `lld` from the LLVM project is a drop-in replacement if you'd rather
  not install a separate system linker.

### "undefined reference to titrate_Math_sin"

The native bridge couldn't find a wrapper for the native function you
called. This usually means the function exists in the bytecode VM's
native table but hasn't been wrapped in `titrate_native` yet. See
[Wrapping C Libraries](./native-cbind) for how to add new wrappers.

### Native build is slower than bytecode

If your release-mode native build is *slower* than the bytecode VM, the
likely causes are:

1. **You forgot `--release`** — debug native builds are unoptimized and
   can be slower than the bytecode VM.
2. **Your program is I/O-bound** — the native backend can't speed up
   kernel calls. Profile to confirm where time is spent.
3. **Native bridge overhead dominates** — every call from native code
   back into the Titrate runtime (e.g. `io::println`, `ArrayList.get`)
   marshals values through a C-ABI tagged union. For collection-heavy
   or I/O-heavy code this overhead can dominate.

### Build hangs at link time

Large release builds can take a while to link, especially with LTO
enabled. If the link step is genuinely stuck (not just slow), try a
different linker — `lld` is dramatically faster than the GNU or MSVC
default linkers.

### "cargo build" fails with inkwell errors

The `inkwell` crate is picky about LLVM versions. Make sure the LLVM
version `inkwell` was built against matches the system LLVM. If you
upgrade LLVM, do a clean rebuild:

```bash
cargo clean
cargo build --release
```

## See Also

- [Why Native?](./native-intro) — what the native backend is and when
  to use it.
- [Ownership on LLVM](./native-ownership) — how `Owned<T>`, borrows,
  regions, and `unsafe` lower to LLVM IR.
- [Wrapping C Libraries](./native-cbind) — the native bridge and how to
  extend it.
- [Native MD Simulation](./native-md) — benchmarks and profiling tips.
- [Build Tool](./build-tool) — the `pipette` build tool reference.
