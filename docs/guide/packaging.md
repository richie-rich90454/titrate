# Packaging and Distribution

This guide covers creating distributable packages of the Titrate compiler and standard library for deployment.

## Quick Package

The packaging script bundles the compiler, standard library, and examples into a self-contained directory:

::: code-group

```bash [Linux/macOS]
./scripts/package.sh --release
```

```powershell [Windows]
.\scripts\package.ps1 -Release
```

:::

This creates a `dist/titrate-0.2.0-<os>-<arch>/` directory containing:

```
titrate-0.2.0-windows-x86_64/
├── bin/
│   ├── trc.exe              # Compiler binary
│   ├── pipette.exe          # Build tool
│   ├── titrate_native.dll   # Native runtime (Windows)
│   └── LLVM-C.dll           # LLVM runtime (if found)
├── lib/
│   └── tt/                  # Standard library (~400+ .tr files)
├── examples/                # Example programs
├── README.md                # Documentation
├── AGENTS.md                # Syntax reference
├── trc.bat                  # Windows launcher
└── trc.sh                   # Linux/macOS launcher
```

## Package Options

| Flag | Effect |
|------|--------|
| `--release` | Package release (optimized) binaries |
| `--output <dir>` | Custom output directory |

### Examples

::: code-group

```bash [Linux/macOS]
# Debug package
./scripts/package.sh

# Custom output directory
./scripts/package.sh --release --output /opt/titrate
```

```powershell [Windows]
# Debug package
.\scripts\package.ps1

# Custom output directory
.\scripts\package.ps1 -Release -Output "C:\Tools\Titrate"
```

:::

## Manual Packaging

If you prefer to build the package manually:

```bash
# 1. Build release binaries
cd trc
cargo build --release

# 2. Create package directory
mkdir -p dist/titrate/bin
mkdir -p dist/titrate/lib/tt
mkdir -p dist/titrate/examples

# 3. Copy binaries
cp target/release/trc dist/titrate/bin/
cp target/release/pipette dist/titrate/bin/

# 4. Copy standard library
cp -r ../lib/tt/* dist/titrate/lib/tt/

# 5. Copy examples
cp ../examples/*.tr dist/titrate/examples/

# 6. Copy documentation
cp ../README.md dist/titrate/
cp ../AGENTS.md dist/titrate/
```

## Using the Package

After packaging, you can run Titrate programs from the package directory:

::: code-group

```bash [Linux/macOS]
# Using the launcher script
./dist/titrate-0.2.0-linux-x86_64/trc.sh hello.tr

# Or add to PATH
export PATH="$(pwd)/dist/titrate-0.2.0-linux-x86_64/bin:$PATH"
trc hello.tr
```

```powershell [Windows]
# Using the launcher
.\dist\titrate-0.2.0-windows-x86_64\trc.bat hello.tr

# Or add to PATH
$env:Path = "C:\path\to\dist\titrate-0.2.0-windows-x86_64\bin;$env:Path"
trc hello.tr
```

:::

## Native Compilation in Packages

For native compilation (`--native` flag) to work in a packaged distribution, the LLVM runtime library must be accessible:

| Platform | Library | Location |
|----------|---------|----------|
| Windows | `LLVM-C.dll` | `bin/` directory |
| macOS | `libLLVM.dylib` | `/opt/homebrew/opt/llvm@22/lib/` |
| Linux | `libLLVM.so` | System library path |

The Windows packaging script automatically copies `LLVM-C.dll` if found at `C:\Program Files\LLVM\bin\`. On macOS and Linux, LLVM shared libraries must be present on the target system.

## Cross-Compilation

Titrate does not currently support cross-compilation of Titrate programs (the compiler itself can cross-compile the Titrate language to native code for the host platform only). Cross-compilation of the compiler itself is possible using Rust's cross-compilation support:

```bash
# Install the target
rustup target add x86_64-unknown-linux-gnu

# Build for Linux from macOS
cargo build --release --target x86_64-unknown-linux-gnu

# Build for Windows from Linux (requires mingw)
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

## Distribution Checklist

Before distributing a Titrate package, verify:

- [ ] `cargo test --lib` passes — 718+ tests
- [ ] `cargo test --test stdlib_test` passes — 53 tests
- [ ] `cargo test --test mega_test` passes — 1 end-to-end test
- [ ] `cargo clippy --all -- -D warnings` passes — zero warnings
- [ ] `trc hello.tr` runs successfully — bytecode VM mode
- [ ] `trc --native --release hello.tr` runs successfully — native compilation (if LLVM available)
- [ ] Standard library imports resolve correctly
- [ ] `pipette` commands work (if included)

## Versioning

The package version follows the project version in `trc/Cargo.toml`. Update the version in the packaging scripts when releasing a new version:

```powershell
# In scripts/package.ps1, line 18
$Version = "0.3.0"  # Update this
```

```bash
# In scripts/package.sh, line 29
VERSION="0.3.0"  # Update this
```

## Creating a Release

Full release workflow:

```bash
# 1. Ensure all tests pass
cd trc
cargo test --all
cargo clippy --all -- -D warnings

# 2. Build release binaries
cargo build --release

# 3. Package for distribution
cd ..
./scripts/package.sh --release

# 4. Archive the package
cd dist
tar -czf titrate-0.2.0-linux-x86_64.tar.gz titrate-0.2.0-linux-x86_64/
# On Windows: Compress-Archive -Path titrate-0.2.0-windows-x86_64 -DestinationPath titrate-0.2.0-windows-x86_64.zip

# 5. The archive is ready for upload to GitHub Releases
```

## Docker Build

For reproducible builds in a container:

```dockerfile
# Dockerfile
FROM rust:1.96-slim-bookworm

RUN apt update && apt install -y \
    llvm-22-dev \
    libclang-22-dev \
    cmake \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cd trc && cargo build --release

CMD ["/app/trc/target/release/trc"]
```

Build and run:

```bash
docker build -t titrate .
docker run --rm titrate --help
```