# Build Tool

Pipette is the build tool and package manager for Titrate. It handles project creation, compilation, testing, dependency management, and a variety of development workflow commands.

## Creating a Project

```bash
pipette new myproject
```

This creates a project skeleton:

```
myproject/
  Titrate.toml
  src/
    main.tr
```

The generated `src/main.tr` contains a hello-world program:

```titrate
import tt::io::println;

public fn main(): void {
    println("Hello from myproject!");
}
```

## Building

```bash
pipette build
```

Compiles all source files, resolves imports, and produces bytecode. The output is placed in the `build/` directory.

### Build Profiles

Pipette supports two build profiles:

| Profile | Flag | Description |
|---------|------|-------------|
| Debug | (default) | No optimizations, full debug info |
| Release | `--release` | Optimized build with constant folding and dead code elimination |

```bash
pipette build              # debug build
pipette build --release    # release build
```

Release builds run the compiler's optimization passes (constant folding and dead code elimination) to produce more efficient bytecode. See [Optimizations](./optimizations) for details.

## Running

```bash
pipette run
```

Builds the project (if needed) and then executes it on the Titrate VM. This is the most common command during development.

## Testing

```bash
pipette test
```

Runs all test functions in the project. Test functions are public functions whose name starts with `test_`:

```titrate
public fn test_addition(): void {
    if (1 + 1 != 2) {
        io::println("FAIL: 1 + 1 != 2");
    }
}
```

## Benchmarking

```bash
pipette bench
```

Finds and runs benchmark files. Benchmark files are source files ending in `_bench.tr` located anywhere in the `src/` directory:

```
src/
  main.tr
  sorting_bench.tr
  math_bench.tr
```

Each benchmark file is compiled and executed. Pipette reports which benchmarks pass or fail.

## Cleaning

```bash
pipette clean
```

Removes the `build/` and `target/` directories. Use this to force a full rebuild or to reclaim disk space:

```bash
pipette clean
# Removed build/
```

## Linting

```bash
pipette lint
```

Runs the semantic analyzer on all `.tr` source files in the project and reports errors and warnings. Unlike `build`, lint only performs analysis — it does not produce bytecode:

```bash
pipette lint
#   src/main.tr OK
#   src/utils.tr ERROR: ...
```

This is useful for catching type errors, unused variables, and unreachable code without a full compilation cycle.

## Formatting

```bash
pipette fmt
```

Formats all `.tr` source files in the project according to the standard Titrate style. This automatically rewrites files in place.

## Dependency Management

### Checking for Outdated Dependencies

```bash
pipette outdated
```

Checks whether newer versions of your dependencies are available:

```bash
pipette outdated
# Checking dependencies for updates...
#
# Dependency          Current         Latest
# --------------------------------------------------
# serde               0.3.0           (unavailable)
```

> Note: Remote version checking requires a Titrate package registry, which is not yet available. The command reports current versions and notes when checking is unavailable.

### Dependency Tree

```bash
pipette tree
```

Displays the project's dependency tree:

```bash
pipette tree
# myproject
# ├── serde v0.3.0
# └── mylib (git)
#     └── https://github.com/example/mylib
```

## Watching for Changes

```bash
pipette watch
```

Watches the `src/` directory for file changes and automatically rebuilds and reruns the project. Useful for rapid iteration.

## Documentation Generation

```bash
pipette doc
```

Generates API documentation from the project's source files.

## Titrate.toml

The project manifest file, `Titrate.toml`, lives in the project root:

```toml
[package]
name = "myproject"
version = "0.1.0"
entry = "src/main.tr"

[dependencies]
```

### Fields

| Field | Description |
|-------|-------------|
| `name` | Project name |
| `version` | Semantic version |
| `entry` | Path to the entry point file (relative to project root) |

## Dependencies

Pipette supports dependencies from Git repositories and local paths.

### Git Dependencies

```toml
[dependencies]
serde = { git = "https://github.com/example/titrate-serde", tag = "v0.3.0" }
```

### Local Path Dependencies

```toml
[dependencies]
mylib = { path = "../mylib" }
```

After adding a dependency, run `pipette build` to fetch and compile it. Dependencies are automatically resolved and compiled before the main project.

## Command Reference

| Command | Description |
|---------|-------------|
| `pipette new <name>` | Create a new project |
| `pipette build [--release]` | Compile the project |
| `pipette run` | Build and run the project |
| `pipette test` | Run test functions |
| `pipette bench` | Run benchmark files |
| `pipette clean` | Remove build output |
| `pipette lint` | Run the analyzer on all source files |
| `pipette fmt` | Format source files |
| `pipette outdated` | Check for dependency updates |
| `pipette tree` | Show the dependency tree |
| `pipette watch` | Watch for changes and rebuild |
| `pipette doc` | Generate API documentation |

## What's Next?

- [Optimizations](./optimizations) — compiler optimization passes
- [Error Handling](./error-handling) — Result types and compiler diagnostics
- [Getting Started](./getting-started) — basic project setup
