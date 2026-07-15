# Optparse

The `tt.argparse.ArgumentParser` module provides optparse-style command-line parsing via the `OptionParser` class. It mirrors Python's `optparse` module. `OptionParser` accumulates `Option` definitions via `addOption`, then `parseArgs(args)` returns an `OptionParserResult` containing `OptionValues` (parsed option values keyed by `dest`) and remaining positional `args`. Supported actions are `store`, `store_true`, `store_false`, `append`, `count`.

## Import

```titrate
import tt::argparse::ArgumentParser;
```

## Classes

### Option

A single option definition for `OptionParser`.

**Fields:**
- `shortOpt: string` — short option string (e.g. `"-f"`), or `""` if none
- `longOpt: string` — long option string (e.g. `"--file"`), or `""` if none
- `dest: string` — attribute name to store the value under
- `action: string` — `"store"`, `"store_true"`, `"store_false"`, `"append"`, or `"count"`
- `typ: string` — `"string"`, `"int"`, `"float"`, or `"bool"`
- `help: string` — help text
- `metavar: string` — placeholder shown in usage
- `defaultValue: Variant`

**Constructors:**
- `init()` — creates an option with defaults (`action="store"`, `typ="string"`)

**Methods:**
- `takesValue(): bool` — true if this option takes a value argument (action is `store` or `append`)
- `toString(): string` — returns `"-f/--file"` style representation

### OptionValues

Holds parsed option values with attribute-style access.

**Methods:**
- `get(key: string): Variant` — return the value for `key`
- `set(key: string, value: Variant): void` — set the value for `key`
- `has(key: string): bool` — true if `key` is set
- `toString(): string`

### OptionParserResult

Holds parsed values and remaining positional args.

**Fields:**
- `values: OptionValues`
- `args: ArrayList<string>`

**Constructors:**
- `init()` — creates an empty result

### OptionParser

The main optparse-style parser.

**Fields:**
- `_usage: string`
- `_prog: string`
- `_description: string`

**Constructors:**
- `init(usage: string)` — create a parser with a usage string
- `initWithProg(prog: string, usage: string)` — create a parser with a program name and usage string

**Methods:**
- `addOption(shortOpt: string, longOpt: string, dest: string, action: string, typ: string, help: string, metavar: string, defaultValue: Variant): Option` — add an option definition. Returns the created `Option`. If `dest` is empty, it is derived from `longOpt` (or `shortOpt`).
- `parseArgs(args: ArrayList<string>): OptionParserResult` — parse command-line args; returns an `OptionParserResult` with `values` (defaults applied first) and `args` (remaining positional args). Throws on errors.
- `error(msg: string): void` — print an error message and abort (mirrors `OptionParser.error`)
- `formatHelp(): string` — return the formatted help text

```titrate
let parser: OptionParser = new OptionParser("usage: prog [options]");
parser.addOption("-v", "--verbose", "verbose", "store_true", "bool", "enable verbose output", "", false);
parser.addOption("-c", "--count", "count", "store", "int", "iteration count", "N", 1);
let result: OptionParserResult = parser.parseArgs(args);
let verbose: bool = result.values.get("verbose") as bool;
let count: int = result.values.get("count") as int;
```

## Usage Example

```titrate
import tt::argparse::ArgumentParser;

public fn main(): void {
    let args = new ArrayList<string>();
    args.add("-v"); args.add("--count=5"); args.add("input.txt");
    let parser: OptionParser = new OptionParser("usage: myapp [options] file");
    parser.addOption("-v", "--verbose", "verbose", "store_true", "bool", "verbose output", "", false);
    parser.addOption("-c", "--count", "count", "store", "int", "iteration count", "N", 1);
    let result: OptionParserResult = parser.parseArgs(args);
    io::println("verbose: " + Boolean.toString(result.values.get("verbose") as bool));
    io::println("count: " + Integer.toString(result.values.get("count") as int));
    var i: int = 0;
    while (i < result.args.size()) {
        io::println("arg: " + result.args.get(i));
        i = i + 1;
    }
}
```
