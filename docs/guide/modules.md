# Modules

As your Titrate project grows, you'll want to split your code across multiple files. That's where modules come in. Titrate organizes code into modules — each `.tr` file is a module, and modules can import from one another using the `import` keyword. Think of modules as a way to keep your codebase tidy, reusable, and free of name collisions.

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

public fn main(): void {
    let x = Integer.parseInt("42");
    switch x {
        case Ok(n) => io::println(Integer.toString(n));
        case Err(e) => io::println("Parse failed: " + e);
    }
}
```

::: tip `::` is primarily for imports
The `::` syntax is used primarily in `import` statements. Once you've imported something, you use `.` (dot) to call its methods — like `Integer.parseInt("42")`, not `Integer::parseInt("42")`. Note that `::` also works for calling module-level functions (e.g., `Integer::parseInt("42")` is valid), but `.` is the recommended style.
:::

## The `tt` Namespace Convention

All standard library modules live under the `tt` namespace. This convention keeps the standard library organized and avoids conflicts with your own code:

| Namespace | Contents |
|-----------|----------|
| `tt::lang` | Core types: `Integer`, `Double`, `String`, `Bool` |
| `tt::util` | Data structures: `ArrayList`, `HashMap`, `HashSet` |
| `tt::io` | I/O functions: `println`, `print` |
| `tt::math` | Math types: `Math`, `NDArray`, `Matrix` |
| `tt::chem` | Chemistry types: `Atom`, `Molecule`, `ForceField` |
| `tt::units` | Unit types: `Base`, `Derived`, `Constants` |

When you create your own modules, they also live under `tt` by convention. For example, a file at `src/game/player.tr` would be imported as `tt::game::player`.

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
import tt::util::useful;  // OK
// import tt::utils::helper;  // Error: helper is private
```

Classes, functions, enums, and constants can all be marked `public`.

::: tip Start private, expose selectively
A good practice is to keep everything private by default and only add `public` when you actually need something from another module. This makes your module's public API small and intentional — easier to maintain and less likely to break when you refactor internals.
:::

## Multi-File Compilation

When using `pipette build` or `trc` with multiple source files, the compiler resolves all imports and compiles them together. The entry point is the file containing `public fn main(): void`.

With `pipette`, the project structure and entry point are defined in `Titrate.toml`.

## Circular Import Detection

Titrate detects circular imports at compile time. If module A imports module B, and module B imports module A (directly or transitively), the compiler will report an error:

```
error: circular import detected: tt::foo -> tt::bar -> tt::foo
```

To break a circular dependency, extract the shared code into a third module that both A and B can import without forming a cycle.

```
Before:  A → B → A  (circular!)

After:   A → C ← B  (C has the shared code, A and B both import C)
```

## Module Organization Best Practices

### Group by Feature, Not by Type

Organize your modules around features or domains rather than technical categories:

```
// Good — organized by feature
src/
  game/
    player.tr      // Player class + related functions
    enemy.tr       // Enemy class + related functions
    world.tr       // World class + related functions

// Avoid — organized by type
src/
  classes/
    player.tr
    enemy.tr
    world.tr
  functions/
    player_utils.tr
    enemy_utils.tr
```

### Keep Modules Focused

Each module should have a clear, single responsibility. If a file is getting long or doing too many things, it's a sign to split it:

```titrate
// Instead of one giant file:
// graphics.tr — handles rendering, textures, shaders, and animations

// Split into focused modules:
// graphics/render.tr
// graphics/texture.tr
// graphics/shader.tr
// graphics/animation.tr
```

### Use Top-Level Functions for Utility Modules

Not every module needs a class. For pure utility functions, use top-level `fn` declarations directly:

```titrate
// math/trig.tr — no class wrapper needed
public fn degreesToRadians(degrees: double): double {
    return degrees * 3.14159265 / 180.0;
}

public fn radiansToDegrees(radians: double): double {
    return radians * 180.0 / 3.14159265;
}
```

### Re-export Convenience

If your users commonly need several items from a sub-module, consider creating a "barrel" module that re-exports them:

```titrate
// game/common.tr — convenience module
import tt::game::player::Player;
import tt::game::enemy::Enemy;
import tt::game::world::World;

// Users can now: import tt::game::common;
// instead of three separate imports
```

## Try It Yourself

Create a small multi-module project with the following structure:

```
src/
  main.tr
  greeting/
    english.tr
```

1. In `greeting/english.tr`, define a public function `greet(name: string): void` that prints "Hello, " + name
2. Also define a private helper function `punctuate(s: string): string` that adds "!" to the end
3. In `main.tr`, import and use `greet`

```titrate
// greeting/english.tr
// Define your functions here

// main.tr
import tt::greeting::english;

public fn main(): void {
    english.greet("Titrate");
}
```

<details>
<summary>Show solution</summary>

```titrate
// greeting/english.tr
fn punctuate(s: string): string {
    return s + "!";
}

public fn greet(name: string): void {
    io::println(punctuate("Hello, " + name));
}
```

```titrate
// main.tr
import tt::greeting::english;

public fn main(): void {
    english.greet("Titrate");  // prints "Hello, Titrate!"
}
```

</details>
