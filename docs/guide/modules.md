# Modules

Titrate organizes code into modules. Each `.tr` file is a module, and modules can import from one another using the `import` keyword.

## Module Basics

Every `.tr` file is automatically a module. The module path is derived from its location in the file system relative to the project root.

```
src/
  main.tr
  math/
    utils.tr
    stats.tr
```

`math/utils.tr` is accessible as `tt::math::utils`.

## Import Syntax

Import a single item:

```titrate
import tt::lang::Integer;
```

Import multiple items by listing separate import statements:

```titrate
import tt::util::ArrayList;
import tt::util::HashMap;
```

After importing, you can use the items directly:

```titrate
import tt::lang::Integer;
import tt::io::println;

public fn main(): void {
    let x = Integer.parseInt("42");
    switch x {
        case Ok(n) => println(Integer.toString(n));
        case Err(e) => println("Parse failed: " + e);
    }
}
```

## Public and Private Visibility

By default, all declarations in a module are **private** — they cannot be imported from other modules. Use the `public` keyword to make a declaration visible outside its module.

```titrate
// utils.tr
fn helper(): void {
    // private — not importable
}

public fn useful(): void {
    // public — can be imported
}
```

```titrate
// main.tr
import tt::utils::useful;  // OK
// import tt::utils::helper;  // Error: helper is private
```

Classes, functions, enums, and constants can all be marked `public`.

## Multi-File Compilation

When using `pipette build` or `trc` with multiple source files, the compiler resolves all imports and compiles them together. The entry point is the file containing `public fn main(): void`.

With `pipette`, the project structure and entry point are defined in `Titrate.toml`.

## Circular Import Detection

Titrate detects circular imports at compile time. If module A imports module B, and module B imports module A (directly or transitively), the compiler will report an error:

```
error: circular import detected: tt::foo -> tt::bar -> tt::foo
```

To break a circular dependency, extract the shared code into a third module that both A and B can import without forming a cycle.
