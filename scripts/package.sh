#!/usr/bin/env bash
# Titrate Packaging Script — Linux / macOS
# Creates a distributable package of the Titrate compiler and standard library.
# Usage: ./scripts/package.sh [--release] [--output <dir>]
set -euo pipefail

RELEASE=false
OUTPUT=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --release) RELEASE=true; shift ;;
        --output) OUTPUT="$2"; shift 2 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TRC_DIR="$PROJECT_ROOT/trc"
PROFILE="debug"
if $RELEASE; then
    PROFILE="release"
fi
BIN_DIR="$TRC_DIR/target/$PROFILE"

# Determine output directory
if [ -z "$OUTPUT" ]; then
    VERSION="0.2.0"
    ARCH=$(uname -m)
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    OUTPUT="$PROJECT_ROOT/dist/titrate-$VERSION-$OS-$ARCH"
fi

echo -e "\033[36m=== Titrate Packaging Script ===\033[0m"
echo -e "\033[90mOutput: $OUTPUT\033[0m"

# Step 1: Build if needed
echo -e "\n\033[33m[1/5] Ensuring build exists...\033[0m"
if [ ! -f "$BIN_DIR/trc" ]; then
    echo -e "  \033[90mBinary not found. Building...\033[0m"
    BUILD_ARGS="build"
    if $RELEASE; then
        BUILD_ARGS="$BUILD_ARGS --release"
    fi
    (cd "$TRC_DIR" && cargo $BUILD_ARGS)
else
    echo -e "  \033[32mBinary found at $BIN_DIR\033[0m"
fi

# Step 2: Create output directory structure
echo -e "\n\033[33m[2/5] Creating package directory structure...\033[0m"
rm -rf "$OUTPUT"
mkdir -p "$OUTPUT/bin"
mkdir -p "$OUTPUT/lib/tt"
mkdir -p "$OUTPUT/examples"
echo -e "  \033[32mDirectory structure created.\033[0m"

# Step 3: Copy binaries
echo -e "\n\033[33m[3/5] Copying binaries...\033[0m"
cp "$BIN_DIR/trc" "$OUTPUT/bin/"
echo -e "  \033[32mtrc copied.\033[0m"

if [ -f "$BIN_DIR/pipette" ]; then
    cp "$BIN_DIR/pipette" "$OUTPUT/bin/"
    echo -e "  \033[32mpipette copied.\033[0m"
fi

# Copy titrate_native shared library
if [[ "$OSTYPE" == "darwin"* ]]; then
    NATIVE_LIB="$BIN_DIR/libtitrate_native.dylib"
elif [[ "$OSTYPE" == "linux"* ]]; then
    NATIVE_LIB="$BIN_DIR/libtitrate_native.so"
fi
if [ -n "${NATIVE_LIB:-}" ] && [ -f "${NATIVE_LIB:-}" ]; then
    cp "$NATIVE_LIB" "$OUTPUT/bin/"
    echo -e "  \033[32mtitrate_native shared library copied.\033[0m"
fi

# Step 4: Copy standard library
echo -e "\n\033[33m[4/5] Copying standard library...\033[0m"
cp -r "$PROJECT_ROOT/lib/tt/"* "$OUTPUT/lib/tt/"
TR_COUNT=$(find "$OUTPUT/lib/tt" -name "*.tr" | wc -l)
echo -e "  \033[32m$TR_COUNT standard library files copied.\033[0m"

# Step 5: Copy examples and docs
echo -e "\n\033[33m[5/5] Copying examples and documentation...\033[0m"
if [ -d "$PROJECT_ROOT/examples" ]; then
    cp "$PROJECT_ROOT/examples/"*.tr "$OUTPUT/examples/" 2>/dev/null || true
    EX_COUNT=$(find "$OUTPUT/examples" -name "*.tr" | wc -l)
    echo -e "  \033[32m$EX_COUNT example files copied.\033[0m"
fi

if [ -f "$PROJECT_ROOT/README.md" ]; then
    cp "$PROJECT_ROOT/README.md" "$OUTPUT/"
    echo -e "  \033[32mREADME.md copied.\033[0m"
fi

if [ -f "$PROJECT_ROOT/AGENTS.md" ]; then
    cp "$PROJECT_ROOT/AGENTS.md" "$OUTPUT/"
    echo -e "  \033[32mAGENTS.md (syntax reference) copied.\033[0m"
fi

# Create a run script
cat > "$OUTPUT/trc.sh" << 'RUNEOF'
#!/usr/bin/env bash
TITRATE_HOME="$(cd "$(dirname "$0")" && pwd)"
export PATH="$TITRATE_HOME/bin:$PATH"
exec "$TITRATE_HOME/bin/trc" "$@"
RUNEOF
chmod +x "$OUTPUT/trc.sh"
echo -e "  \033[32mtrc.sh launcher created.\033[0m"

# Print summary
echo -e "\n\033[36m=== Package Complete ===\033[0m"
echo -e "\033[90mLocation: $OUTPUT\033[0m"
echo -e "\n\033[37mContents:\033[0m"
find "$OUTPUT" -type f | while read -r f; do
    REL="${f#$OUTPUT/}"
    SIZE=$(du -h "$f" | cut -f1)
    echo -e "  \033[90m$REL ($SIZE)\033[0m"
done

echo -e "\n\033[37mUsage:\033[0m"
echo -e "  \033[90m$OUTPUT/trc.sh hello.tr\033[0m"
echo -e "  \033[90m$OUTPUT/bin/trc --native --release hello.tr\033[0m"
echo -e "  \033[90mexport PATH=\"$OUTPUT/bin:\$PATH\"  # Add to PATH\033[0m"