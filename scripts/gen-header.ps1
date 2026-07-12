<#
.SYNOPSIS
    Generates titrate_native/titrate_native.h from src/lib.rs and src/wrappers.rs.

.DESCRIPTION
    The titrate_native Rust crate exports two kinds of pub extern "C" functions:
      - 9 direct helpers in src/lib.rs (println, malloc/free, native_call, ...)
      - N per-function wrappers in src/wrappers.rs (titrate_<NativeName>)

    This script reads wrappers.rs, extracts every wrapper name via the regex
    `pub extern "C" fn titrate_(\w+)`, and emits a uniform C declaration for each:
        TitrateValue titrate_<name>(const TitrateValue* args, size_t arg_count);

    The 9 direct helpers have distinct, non-uniform signatures and are emitted
    verbatim from the hardcoded block below. The TitrateValue struct and the
    TitrateString / TitrateArray / TitrateHandle / TitratePayload typedefs are
    emitted at the top so the declarations are self-contained.

    Usage:
        pwsh scripts/gen-header.ps1
        powershell -ExecutionPolicy Bypass -File scripts/gen-header.ps1
#>
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot  = Split-Path -Parent $ScriptDir
$HeaderPath   = Join-Path $RepoRoot "titrate_native\titrate_native.h"
$WrappersPath = Join-Path $RepoRoot "titrate_native\src\wrappers.rs"

if (-not (Test-Path $WrappersPath)) { throw "wrappers.rs not found: $WrappersPath" }

$src = Get-Content -Raw $WrappersPath
# Skip the `titrate_` prefix by capturing only <Name>, since the regex anchors on
# `pub extern "C" fn titrate_(\w+)`.
$names = [regex]::Matches($src, 'pub(?:\s+unsafe)?\s+extern\s+"C"\s+fn\s+titrate_(\w+)\s*\(') |
         ForEach-Object { $_.Groups[1].Value } |
         Sort-Object -Unique
$wrapperCount = ($names | Measure-Object).Count
$directCount  = 9
$total        = $directCount + $wrapperCount
Write-Host "Found $wrapperCount wrappers in wrappers.rs + $directCount direct helpers in lib.rs = $total total"

$wrapperDecls = ($names | ForEach-Object { "TitrateValue titrate_" + $_ + "(const TitrateValue *args, size_t arg_count);" }) -join "`n"

$header = @"
/*
 * titrate_native.h - C-ABI native runtime bridge for the Titrate LLVM backend.
 *
 * Declares every pub extern "C" function exported by the titrate_native
 * Rust crate:
 *   - $directCount direct helpers in src/lib.rs (println, malloc/free, native_call, ...)
 *   - $wrapperCount per-function wrappers in src/wrappers.rs (titrate_<NativeName>)
 *   - $total exported functions in total
 *
 * Hand-written. Update when wrappers.rs changes.
 * See scripts/gen-header.ps1 for the generator.
 */
#ifndef TITRATE_NATIVE_H
#define TITRATE_NATIVE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ------------------------------------------------------------------ */
/* Type tags for the TitrateValue tagged union                        */
/* ------------------------------------------------------------------ */

#define TV_VOID            0
#define TV_NULL            1
#define TV_BOOL            2
#define TV_BYTE            3
#define TV_SHORT           4
#define TV_INT             5
#define TV_LONG            6
#define TV_VAST            7
#define TV_UVAST           8
#define TV_FLOAT           9
#define TV_DOUBLE         10
#define TV_CHAR           11
#define TV_STRING         12
#define TV_ARRAY          13
#define TV_CLASS_INSTANCE 14
#define TV_RESULT_OK      15
#define TV_RESULT_ERR     16
#define TV_ENUM_INSTANCE  17
#define TV_TUPLE          18
#define TV_HALF           19
#define TV_QUAD           20
#define TV_HANDLE         21

/* ------------------------------------------------------------------ */
/* C-ABI value types                                                  */
/* ------------------------------------------------------------------ */

/* UTF-8 string: length + pointer (buffer is NOT NUL-terminated). */
typedef struct {
    int64_t  len;
    uint8_t *ptr;
} TitrateString;

/* Array of TitrateValue: length + heap-allocated data pointer. */
typedef struct {
    int64_t              len;
    struct TitrateValue *data;
} TitrateArray;

/* Opaque handle for complex values (class instances, file handles, etc.). */
typedef struct {
    int64_t id;
    int32_t type_tag;
} TitrateHandle;

/* 16-byte payload union. raw[16] fixes the size at 16 bytes regardless of
 * whether the compiler supports __int128 (MSVC does not). */
typedef union {
    int8_t          bool_val;
    int8_t          byte_val;
    int16_t         short_val;
    int32_t         int_val;
    int64_t         long_val;
#if defined(__SIZEOF_INT128__) || defined(__GNUC__) || defined(__clang__)
    __int128            vast_val;
    unsigned __int128   uvast_val;
#endif
    float           float_val;
    double          double_val;
    uint32_t        char_val;
    TitrateString   string;
    TitrateArray    array;
    TitrateHandle   handle;
    uint8_t         raw[16];
} TitratePayload;

/* The canonical C-ABI value: tag + 16-byte payload (24 bytes total). */
typedef struct TitrateValue {
    int32_t        tag;
    int32_t        _pad;
    TitratePayload payload;
} TitrateValue;

/* ------------------------------------------------------------------ */
/* Direct C-ABI helpers (from src/lib.rs)                              */
/* These have distinct, non-uniform signatures.                       */
/* ------------------------------------------------------------------ */

/* Print a UTF-8 string followed by a newline. */
void titrate_println(int64_t len, const uint8_t *ptr);

/* Concatenate two strings into a freshly allocated buffer. */
uint8_t *titrate_string_concat(int64_t a_len, const uint8_t *a_ptr,
                               int64_t b_len, const uint8_t *b_ptr,
                               int64_t *out_len);

/* Free a buffer returned by titrate_string_concat or titrate_malloc. */
void titrate_free(uint8_t *ptr);

/* Allocate `size` bytes of heap memory. */
uint8_t *titrate_malloc(int64_t size);

/* Primitive printers. */
void titrate_println_int(int64_t v);
void titrate_println_double(double v);
void titrate_println_bool(int32_t v);
void titrate_println_char(int32_t v);

/* Generic native dispatch bridge: invoke a VM native function by name. */
int32_t titrate_native_call(const uint8_t *name_ptr, int64_t name_len,
                            const uint8_t *args_ptr, int64_t args_count,
                            uint8_t *result_ptr, int64_t *result_cap);

/* ------------------------------------------------------------------ */
/* Per-function C-ABI wrappers (from src/wrappers.rs)                 */
/* Uniform signature:                                                 */
/*   TitrateValue titrate_<Name>(const TitrateValue* args,            */
/*                               size_t arg_count);                   */
/* $wrapperCount wrappers in wrappers.rs + $directCount direct helpers in lib.rs = $total total exports. */
/* (was incorrectly documented as "359 wrappers" in the previous revision) */
/* ------------------------------------------------------------------ */

$wrapperDecls

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* TITRATE_NATIVE_H */
"@

[System.IO.File]::WriteAllText($HeaderPath, $header, (New-Object System.Text.UTF8Encoding($false)))
Write-Host "Wrote $HeaderPath ($wrapperCount wrapper declarations + $directCount direct helpers = $total total)"
