#!/usr/bin/env bash
# Titrate Build Script — Linux / macOS
# Compiles the trc compiler, pipette build tool, and titrate_native runtime.
# Usage: ./scripts/build.sh [--release] [--clean] [--target <target>]
set -euo pipefail

RELEASE=false
CLEAN=false
TARGET="all"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --release) RELEASE=true; shift ;;
        --clean) CLEAN=true; shift ;;
        --target) TARGET="$2"; shift 2 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TRC_DIR="$PROJECT_ROOT/trc"

echo -e "\033[36m=== Titrate Build Script ===\033[0m"
echo -e "\033[90mProject root: $PROJECT_ROOT\033[0m"

# Check prerequisites
echo -e "\n\033[33m[1/4] Checking prerequisites...\033[0m"

# Check Rust
if command -v rustc &>/dev/null; then
    RUST_VERSION=$(rustc --version)
    echo -e "  \033[32mRust: $RUST_VERSION\033[0m"
else
    echo -e "  \033[31mERROR: Rust is not installed. Install from https://rustup.rs\033[0m"
    exit 1
fi

# Check LLVM
if command -v llvm-config &>/dev/null; then
    LLVM_VERSION=$(llvm-config --version)
    echo -e "  \033[32mLLVM: $LLVM_VERSION\033[0m"
else
    echo -e "  \033[33mWARNING: llvm-config not found. LLVM native backend may not compile.\033[0m"
    echo -e "  \033[33mInstall LLVM 22.1 via your package manager or from https://github.com/llvm/llvm-project/releases\033[0m"
fi

# Clean if requested
if $CLEAN; then
    echo -e "\n\033[33m[2/4] Cleaning previous build artifacts...\033[0m"
    (cd "$TRC_DIR" && cargo clean)
    echo -e "  \033[32mClean complete.\033[0m"
else
    echo -e "\n\033[90m[2/4] Skipping clean (use --clean to force).\033[0m"
fi

# Build
BUILD_FLAG=""
if $RELEASE; then
    BUILD_FLAG="--release"
fi

echo -e "\n\033[33m[3/4] Building Titrate workspace...\033[0m"
if $RELEASE; then
    echo -e "  \033[90mMode: Release (optimized)\033[0m"
else
    echo -e "  \033[90mMode: Debug (fast compile, no optimizations)\033[0m"
fi

BUILD_ARGS="build"
if $RELEASE; then
    BUILD_ARGS="$BUILD_ARGS --release"
fi
if [ "$TARGET" != "all" ]; then
    BUILD_ARGS="$BUILD_ARGS -p $TARGET"
fi

(cd "$TRC_DIR" && cargo $BUILD_ARGS)
echo -e "  \033[32mBuild complete.\033[0m"

# Verify binaries
echo -e "\n\033[33m[4/4] Verifying binaries...\033[0m"
PROFILE="debug"
if $RELEASE; then
    PROFILE="release"
fi
BIN_DIR="$TRC_DIR/target/$PROFILE"

for bin in trc pipette; do
    BIN_PATH="$BIN_DIR/$bin"
    if [ -f "$BIN_PATH" ]; then
        if [[ "$OSTYPE" == "darwin"* ]]; then
            SIZE=$(stat -f%z "$BIN_PATH" 2>/dev/null || echo "0")
        else
            SIZE=$(stat -c%s "$BIN_PATH" 2>/dev/null || echo "0")
        fi
        SIZE_MB=$(echo "scale=2; $SIZE / 1048576" | bc 2>/dev/null || echo "?")
        echo -e "  \033[32m$bin: ${SIZE_MB} MB\033[0m"
    else
        echo -e "  \033[33mWARNING: $bin not found\033[0m"
    fi
done

# Check titrate_native shared library
NATIVE_LIB=""
if [[ "$OSTYPE" == "darwin"* ]]; then
    NATIVE_LIB="$BIN_DIR/libtitrate_native.dylib"
elif [[ "$OSTYPE" == "linux"* ]]; then
    NATIVE_LIB="$BIN_DIR/libtitrate_native.so"
fi
if [ -n "$NATIVE_LIB" ] && [ -f "$NATIVE_LIB" ]; then
    echo -e "  \033[32mtitrate_native: shared library found\033[0m"
else
    echo -e "  \033[90mNOTE: titrate_native shared library not found (only needed for LLVM native backend)\033[0m"
fi

echo -e "\n\033[36m=== Build Complete ===\033[0m"
echo -e "\033[90mBinaries: $BIN_DIR\033[0m"
echo -e "\n\033[37mNext steps:\033[0m"
echo -e "  \033[90mRun tests:   cd trc && cargo test --lib\033[0m"
echo -e "  \033[90mRun a file:  $BIN_DIR/trc hello.tr\033[0m"
echo -e "  \033[90mFull test:   cd trc && cargo test --all\033[0m"