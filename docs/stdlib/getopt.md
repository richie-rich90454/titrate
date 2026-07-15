# Getopt

The `tt.argparse.ArgumentParser` module provides POSIX/GNU-style command-line option parsing via `getopt` and `gnu_getopt`. It mirrors Python's `getopt` module. The `GetoptOpt` class holds a parsed `(option, value)` pair (with `value` empty for flag-style options), and `GetoptResult` aggregates the parsed options and remaining positional args.

## Import

```titrate
import tt::argparse::ArgumentParser;
```

## Classes

### GetoptOpt

A single parsed option, mirroring Python's `(option, value)` tuple.

**Fields:**
- `option: string` — the option string (e.g. `"-v"`, `"--verbose"`)
- `value: string` — the option's argument (empty string for flag-style options)

**Constructors:**
- `init(option: string, value: string)`

**Methods:**
- `toString(): string` — returns `"(option, value)"` or `"(option,)"` when value is empty

### GetoptResult

Holds the parsed options and remaining positional args.

**Fields:**
- `opts: ArrayList<GetoptOpt>` — parsed options, in order
- `args: ArrayList<string>` — remaining positional args

**Constructors:**
- `init()` — creates an empty result

## Functions

### getopt

- `ArgumentParser.getopt(args: ArrayList<string>, shortopts: string, longopts: ArrayList<string>): GetoptResult` — POSIX-style getopt. Stops at the first non-option argument. `shortopts` is a string of short option letters, with `:` after an option that takes an argument (e.g. `"hv:c:"`). `longopts` is a list of long option names, with `=` suffix for those taking an argument (e.g. `["help", "verbose=", "count="]`).

```titrate
let args = new ArrayList<string>();
args.add("-v"); args.add("--count=3"); args.add("file.txt");
let r: GetoptResult = ArgumentParser.getopt(args, "vc:", new ArrayList<string>(["verbose", "count="]));
```

### gnu_getopt

- `ArgumentParser.gnu_getopt(args: ArrayList<string>, shortopts: string, longopts: ArrayList<string>): GetoptResult` — GNU-style getopt. Options and non-option arguments may be interspersed; positional args are permuted to the end.

## Option Syntax

- Short options: `-a`, `-abc` (cluster), `-cvalue` (attached argument), `-c value` (separated argument)
- Long options: `--name`, `--name=value`, `--name value`
- `--` terminates option processing; subsequent args are positional

## Usage Example

```titrate
import tt::argparse::ArgumentParser;

public fn main(): void {
    let args = new ArrayList<string>();
    args.add("-v"); args.add("--count=3"); args.add("input.txt");
    let result: GetoptResult = ArgumentParser.getopt(args, "vc:", new ArrayList<string>(["verbose", "count="]));
    var i: int = 0;
    while (i < result.opts.size()) {
        let opt: GetoptOpt = result.opts.get(i);
        io::println(opt.option + " = " + opt.value);
        i = i + 1;
    }
    var j: int = 0;
    while (j < result.args.size()) {
        io::println("arg: " + result.args.get(j));
        j = j + 1;
    }
}
```
