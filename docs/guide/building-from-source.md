# Building Titrate from Source

This guide covers compiling the Titrate compiler (`trc`), build tool (`pipette`), and native runtime (`titrate_native`) from source on all supported platforms.

## Prerequisites

### All Platforms

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.96+ | Compiler toolchain |
| LLVM | 22.1 | Native code generation backend |
| Git | Any | Source control |

### Windows

Install **Visual Studio 2022** with the "Desktop development with C++" workload, or the standalone [Build Tools for Visual Studio](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022).

Install LLVM 22.1 from the [LLVM releases page](https://github.com/llvm/llvm-project/releases). Choose `LLVM-22.1.0-win64.exe` and install to `C:\Program Files\LLVM\`. Ensure "Add LLVM to the system PATH" is checked during installation.

```powershell
# Verify
rustc --version
llvm-config --version
```

### macOS

```bash
# Install LLVM
brew install llvm@22

# Add to PATH (add to ~/.zshrc or ~/.bash_profile)
export PATH="/opt/homebrew/opt/llvm@22/bin:$PATH"
export LDFLAGS="-L/opt/homebrew/opt/llvm@22/lib"
export CPPFLAGS="-I/opt/homebrew/opt/llvm@22/include"

# Verify
rustc --version
llvm-config --version
```

### Linux (Ubuntu/Debian)

```bash
# Install build dependencies
sudo apt update
sudo apt install -y build-essential cmake llvm-22-dev libclang-22-dev zlib1g-dev

# Verify
rustc --version
llvm-config-22 --version
```

### Linux (Fedora/RHEL)

```bash
sudo dnf install -y llvm22-devel clang22-devel cmake gcc-c++
```

### Linux (Arch)

```bash
sudo pacman -S llvm clang cmake
```

## Quick Build

The simplest way to build everything. Run from the repository root:

::: code-group

```bash [Linux/macOS]
./scripts/build.sh --release
```

```powershell [Windows]
.\scripts\build.ps1 -Release
```

:::

This produces:
- `trc/target/release/trc` — the Titrate compiler
- `trc/target/release/pipette` — the build tool
- `trc/target/release/titrate_native.dll` — native runtime (Windows)
- `trc/target/release/libtitrate_native.so` — native runtime (Linux)
- `trc/target/release/libtitrate_native.dylib` — native runtime (macOS)

## Build Options

### Build Script Flags

| Flag | Effect |
|------|--------|
| `--release` | Optimized build (slower compile, faster runtime) |
| `--clean` | Clean build artifacts before compiling |
| `--target <crate>` | Build only one crate (`trc`, `pipette`, or `titrate_native`) |

### Examples

::: code-group

```bash [Linux/macOS]
# Debug build (fast compile)
./scripts/build.sh

# Release build with clean
./scripts/build.sh --release --clean

# Build only the compiler
./scripts/build.sh --release --target trc
```

```powershell [Windows]
# Debug build (fast compile)
.\scripts\build.ps1

# Release build with clean
.\scripts\build.ps1 -Release -Clean

# Build only the compiler
.\scripts\build.ps1 -Release -Target trc
```

:::

## Manual Build with Cargo

If you prefer to use cargo directly:

```bash
cd trc

# Debug build
cargo build

# Release build
cargo build --release

# Build specific crate
cargo build -p trc --release
cargo build -p pipette --release
```

## Verifying the Build

### Run the Test Suite

```bash
cd trc

# Unit tests (compiler + VM)
cargo test --lib

# Standard library integration tests
cargo test --test stdlib_test

# End-to-end test
cargo test --test mega_test

# All tests
cargo test --all
```

### Run a Titrate Program

Create a test file:

```titrate
// hello.tr
public fn main(): void {
    io::println("Hello, Titrate!");
}
```

Run it:

::: code-group

```bash [Linux/macOS]
./trc/target/release/trc hello.tr
```

```powershell [Windows]
.\trc\target\release\trc.exe hello.tr
```

:::

Expected output:

```
Hello, Titrate!
```

### Compile to Native Executable

```bash
# Bytecode VM mode (default)
trc hello.tr

# Native compilation via LLVM
trc --native hello.tr

# Native with optimizations
trc --native --release hello.tr
```

## Troubleshooting

### LLVM not found

**Symptom:** Build fails with `LLVM not found` or linker errors referencing `LLVM*`.

**Solution:** Install LLVM 22.1 development files. On Ubuntu: `sudo apt install llvm-22-dev libclang-22-dev`. On macOS: `brew install llvm@22`. On Windows: download from [LLVM releases](https://github.com/llvm/llvm-project/releases).

Verify with `llvm-config --version`.

### Linker errors on Windows

**Symptom:** `LINK : fatal error LNK1181: cannot open input file 'LLVM-C.lib'`

**Solution:** Ensure LLVM is installed to `C:\Program Files\LLVM\` and the `lib` directory is in your library path. Re-run the LLVM installer and check "Add LLVM to the system PATH".

### Compiler panic (ICE)

**Symptom:** `error: internal compiler error: unexpected panic`

**Solution:** Clean the build cache and retry:

```bash
cd trc
cargo clean
cargo build --release
```

### Stack overflow during tests

**Symptom:** Tests crash with stack overflow in `stdlib_runtest`.

**Solution:** The test runner spawns worker threads. If you encounter stack overflows, increase the system stack size limit. On Linux: `ulimit -s 65536`. The test harness already configures 16MB worker thread stacks.

### Build takes too long

**Symptom:** Compilation exceeds 10 minutes.

**Solution:** Use a debug build for development (`cargo build` without `--release`). The LLVM and `inkwell` dependencies are the largest compilation units. Use `cargo build -p trc` to build only the compiler without the full workspace.

## Project Structure

```
titrate/
├── trc/                    # Compiler + VM (Rust workspace)
│   ├── src/
│   │   ├── lexer/          # Lexer and tokenizer
│   │   ├── parser/         # Recursive descent parser
│   │   ├── ast/            # Abstract syntax tree
│   │   ├── analyzer/       # Semantic analysis + type checking
│   │   ├── bytecode/       # Bytecode compiler + VM
│   │   │   ├── compiler/   # AST → bytecode compilation
│   │   │   └── vm/         # Stack-based virtual machine
│   │   ├── codegen/        # LLVM native code generation
│   │   └── interpreter/    # Tree-walking interpreter (removed)
│   └── tests/              # Integration tests
├── pipette/                # Build tool (pipette)
├── titrate_native/         # C-ABI native runtime bridge
├── lib/tt/                 # Standard library (Titrate source)
├── examples/               # Example programs
├── scripts/                # Build and packaging scripts
│   ├── build.ps1           # Windows build script
│   ├── build.sh            # Linux/macOS build script
│   ├── package.ps1         # Windows packaging script
│   └── package.sh          # Linux/macOS packaging script
└── docs/                   # VitePress documentation
```

## Continuous Integration

A typical CI pipeline for Titrate:

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]
jobs:
  build-and-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - name: Install LLVM
        run: sudo apt install -y llvm-22-dev libclang-22-dev
      - name: Build
        run: cd trc && cargo build --release
      - name: Lint
        run: cd trc && cargo clippy --all -- -D warnings
      - name: Test
        run: cd trc && cargo test --all
```