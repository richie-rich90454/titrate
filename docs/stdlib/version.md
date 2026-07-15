# Version

The `tt::lang::Version` module provides C++ `<version>` parity. It exposes `__cpp_lib_*` feature-test macro equivalents as boolean constants and year values. All features are set to `true` because Titrate provides equivalents. Macro data is loaded from `data/lang/version_macros.json` via `DataFile`.

## Import

```titrate
import tt::lang::Version;
```

## API Reference

### Feature-Test Macro Queries

#### `isSupported(name: string): bool`

Returns `true` if the named feature-test macro is supported. All macros return `true` because Titrate provides equivalents.

#### `macroYear(name: string): int`

Returns the C++ standard year-month value for the named macro (e.g. `201603`), or `0` if the name is unknown.

#### `cppName(name: string): string`

Returns the C++ macro name (e.g. `"__cpp_lib_coroutine"`) for a feature, or `""` if the name is unknown.

#### `allMacroNames(): ArrayList<string>`

Returns a list of all feature-test macro names.

#### `report(): string`

Returns a human-readable report of all feature-test macros, including their C++ name, year, and support status.

### Boolean Constant Accessors

Each function corresponds to a `__cpp_lib_*` feature-test macro and returns a boolean indicating support:

- `cppLibParallelAlgorithms(): bool` — `__cpp_lib_parallel_algorithm`
- `cppLibCoroutines(): bool` — `__cpp_lib_coroutine`
- `cppLibConcepts(): bool` — `__cpp_lib_concepts`
- `cppLibFormat(): bool` — `__cpp_lib_format`
- `cppLibRanges(): bool` — `__cpp_lib_ranges`
- `cppLibThread(): bool` — `__cpp_lib_thread`
- `cppLibAtomic(): bool` — `__cpp_lib_atomic`
- `cppLibCharconv(): bool` — `__cpp_lib_charconv`
- `cppLibChrono(): bool` — `__cpp_lib_chrono`
- `cppLibFilesystem(): bool` — `__cpp_lib_filesystem`
- `cppLibOptional(): bool` — `__cpp_lib_optional`
- `cppLibVariant(): bool` — `__cpp_lib_variant`
- `cppLibStringView(): bool` — `__cpp_lib_string_view`
- `cppLibSpan(): bool` — `__cpp_lib_span`
- `cppLibCompare(): bool` — `__cpp_lib_compare`
- `cppLibNumbers(): bool` — `__cpp_lib_numbers`
- `cppLibMemoryResource(): bool` — `__cpp_lib_memory_resource`

### Compile-Time Constants

These mirror C++ `constexpr bool` values in `<version>`. Values are loaded from `data/lang/version_macros.json` so no large hardcoded lookup table is present in source.

- `CPP_LIB_PARALLEL_ALGORITHMS: bool`
- `CPP_LIB_COROUTINES: bool`
- `CPP_LIB_CONCEPTS: bool`
- `CPP_LIB_FORMAT: bool`
- `CPP_LIB_RANGES: bool`
- `CPP_LIB_THREAD: bool`
- `CPP_LIB_ATOMIC: bool`
- `CPP_LIB_CHARCONV: bool`
- `CPP_LIB_CHRONO: bool`
- `CPP_LIB_FILESYSTEM: bool`
- `CPP_LIB_OPTIONAL: bool`
- `CPP_LIB_VARIANT: bool`
- `CPP_LIB_STRING_VIEW: bool`
- `CPP_LIB_SPAN: bool`
- `CPP_LIB_COMPARE: bool`
- `CPP_LIB_NUMBERS: bool`
- `CPP_LIB_MEMORY_RESOURCE: bool`

### Titrate Version Constants

- `TITRATE_VERSION: int` — the Titrate language version (analog of `__cplusplus`), e.g. `202403`
- `TITRATE_VERSION_STRING: string` — the human-readable version string, e.g. `"2024.03"`

## Usage Examples

### Checking Feature Support

```titrate
import tt::lang::Version;
import tt::io::IO;

public fn main(): void {
    if (Version.isSupported("cppLibCoroutines")) {
        IO.println("Coroutines are supported");
    }
    let year: int = Version.macroYear("cppLibCoroutines");
    IO.println("Standardized: " + Integer.toString(year));
    let cpp: string = Version.cppName("cppLibCoroutines");
    IO.println("C++ macro: " + cpp);
}
```

### Using Boolean Accessors

```titrate
import tt::lang::Version;

if (Version.cppLibConcepts()) {
    io::println("Concepts support is available");
}
if (Version.cppLibFormat()) {
    io::println("Formatting library is available");
}
```

### Listing All Feature-Test Macros

```titrate
import tt::lang::Version;
import tt::util::ArrayList;

let names: ArrayList<string> = Version.allMacroNames();
var i: int = 0;
while (i < names.size()) {
    let name: string = names.get(i);
    let supported: bool = Version.isSupported(name);
    let year: int = Version.macroYear(name);
    io::println(name + " -> year=" + Integer.toString(year) + " supported=" + (supported ? "true" : "false"));
    i = i + 1;
}
```

### Generating a Full Report

```titrate
import tt::lang::Version;

let report: string = Version.report();
io::println(report);
```

### Checking the Titrate Version

```titrate
import tt::lang::Version;

io::println("Titrate version: " + Version.TITRATE_VERSION_STRING);
io::println("Version code: " + Integer.toString(Version.TITRATE_VERSION));
```
