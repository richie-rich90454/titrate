# Contributing to Titrate

We're glad you want to contribute! Titrate is a young language, and every contribution — whether it's a bug fix, a new feature, improved documentation, or a better error message — makes a real difference.

## Code of Conduct

Be respectful, constructive, and inclusive. We're all here to build something useful. Disagreements are fine; personal attacks are not. If you see unacceptable behavior, report it privately to the maintainers.

## Getting Started

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/titrate.git
   cd titrate
   ```
3. Add the upstream remote:
   ```bash
   git remote add upstream https://github.com/richie-rich90454/titrate.git
   ```

### Build the Compiler

Titrate's compiler is written in Rust. Make sure you have [Rust installed](https://rustup.rs/), then:

```bash
cd trc
cargo build
```

This compiles the compiler (`trc`) and the VM. The resulting binary can run `.tr` files.

### Run the Tests

Before making changes, make sure the existing tests pass:

```bash
# Compiler/VM unit tests
cargo test --lib

# Standard library integration tests
cargo test --test stdlib_test

# End-to-end mega test
cargo test --test mega_test
```

All three should pass before you start working. If they don't, open an issue.

## Development Workflow

### Create a Branch

```bash
git checkout -b my-feature
```

Use a descriptive branch name: `fix-null-check`, `add-do-while`, `improve-error-messages`, etc.

### Make Your Changes

The codebase is organized into clear modules:

| Directory | What it contains |
|-----------|-----------------|
| `trc/src/lexer/` | Tokenizer (characters → tokens) |
| `trc/src/parser/` | Parser (tokens → AST) |
| `trc/src/analyzer/` | Type checker and semantic analysis |
| `trc/src/bytecode/compiler/` | Bytecode emission and optimization |
| `trc/src/bytecode/vm/` | Virtual machine and native functions |
| `lib/tt/` | Standard library (`.tr` files) |
| `docs/` | VitePress documentation site |
| `examples/` | Example Titrate programs |

### Running Tests During Development

Run the relevant test suite for the area you're working on:

```bash
# If you changed the lexer, parser, analyzer, or VM:
cargo test --lib

# If you changed the standard library:
cargo test --test stdlib_test

# If you changed anything that affects end-to-end behavior:
cargo test --test mega_test

# Run all tests:
cargo test
```

For faster iteration on a specific test:

```bash
cargo test --lib test_name
```

## Code Style

### Compiler Code (Rust)

Follow standard Rust conventions:
- `cargo fmt` for formatting
- `cargo clippy` for linting
- Use `Result` for error handling, not `unwrap()` in production code
- Add unit tests for new functionality

### Titrate Code (Standard Library and Examples)

Write **canonical Titrate**, not sugar forms. The standard library should demonstrate the recommended style:

- Use `name: Type` parameter order, not `Type name`
- Use `let`/`var`/`const` for all variable declarations
- Use `string` (lowercase), not `String`
- Use `fn name(params): ReturnType` for functions
- Use `new ClassName()` for constructors
- Use `String.length(s)`, not `s.length()`
- Use `value as Type` for casting, not `(Type) value`
- Use `is` for type checking, not `instanceof`
- Use `ok()`/`err()` for Result construction
- Use `.` for module method calls, not `::` (except in imports)
- Use `import tt::module::Name` for imports
- No `static` keyword — use top-level `fn` instead
- No `Object` type — use generics or `Variant`

### Documentation

- Write in clear, concise English
- Use code examples that actually compile and run
- Follow the existing documentation style in `docs/guide/`
- Use VitePress markdown extensions (`::: tip`, `::: warning`) where appropriate

## Commit Message Conventions

Write commit messages that explain *why*, not just *what*:

```
Add do-while loop support

The language was missing a do-while construct, which is the natural
way to express loops that must execute at least once. This adds
parsing, analysis, and bytecode emission for `do { ... } while (cond);`.
```

Format:
- First line: short summary (imperative mood)
- Blank line
- Detailed explanation (optional but encouraged for non-trivial changes)

Prefixes are optional but helpful:
- `fix:` for bug fixes
- `feat:` for new features
- `docs:` for documentation changes
- `refactor:` for code reorganization
- `test:` for test additions

## Pull Request Process

1. **Update your branch** with the latest upstream changes:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Run all tests** one final time:
   ```bash
   cargo test
   ```

3. **Push your branch** and open a pull request on GitHub.

4. **Describe your changes** in the PR description:
   - What problem does this solve?
   - How does it solve it?
   - Are there any breaking changes?
   - Which tests did you add/update?

5. **Respond to review feedback** promptly and push updates to the same branch.

6. **Keep the PR focused** — one feature or fix per PR. If you find additional improvements while working, open a separate PR.

## Reporting Bugs

Good bug reports help us fix things faster. Please include:

1. **Titrate version** — run `trc --version` or note the commit hash
2. **Operating system** — Windows, macOS, Linux, and version
3. **Minimal reproduction** — the smallest complete program that triggers the bug
4. **Expected behavior** — what you thought should happen
5. **Actual behavior** — what actually happened, including error messages

Example:

```
**Version:** trc 0.2.0 (commit abc1234)
**OS:** macOS 15.2

**Reproduction:**
```titrate
public fn main(): void {
    let x: int = 5;
    io::println(Integer.toString(x));
}
```

**Expected:** Prints "5"
**Actual:** Compiler panic with "unexpected token: IntLiteral"
```

## Suggesting Features

We welcome feature suggestions! Before opening an issue:

1. **Check existing issues** — someone may have already suggested it
2. **Consider scope** — does it fit the language's design philosophy?
3. **Provide motivation** — what problem does it solve? What's the use case?
4. **Sketch a design** — how would the syntax look? How would it interact with existing features?

Feature requests that include a concrete design and motivation are much more likely to be accepted.

## Documentation Contributions

Documentation is just as important as code. You can contribute by:

- **Fixing errors** — if something in the docs is wrong or outdated, open a PR
- **Adding examples** — practical examples help people learn faster
- **Improving clarity** — if a section confused you, it probably confuses others too
- **Adding new pages** — check for open documentation issues

The docs are built with VitePress and live in `docs/`. To preview locally:

```bash
cd docs
npm install
npm run dev
```

## Thank You

Every contribution matters, whether it's a one-line fix or a major feature. Thanks for helping make Titrate better.
