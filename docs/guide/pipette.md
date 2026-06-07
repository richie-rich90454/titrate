# Pipette

Pipette is the build tool and package manager for Titrate. It handles project creation, compilation, testing, and dependency management.

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
    if 1 + 1 != 2 {
        io::println("FAIL: 1 + 1 != 2");
    }
}
```

## Watching for Changes

```bash
pipette watch
```

Watches the `src/` directory for file changes and automatically rebuilds and reruns the project. Useful for rapid iteration.

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
