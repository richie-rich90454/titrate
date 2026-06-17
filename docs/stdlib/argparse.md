# argparse

The `tt.argparse` module provides command-line argument parsing with support for positional arguments, flags, and options.

```titrate
import tt.argparse;
```

## ArgumentParser

### Creating a Parser

- `new ArgumentParser(name: string)` — create a parser with the program name
- `new ArgumentParser(name: string, description: string)` — create a parser with a description shown in help text

```titrate
let parser = new ArgumentParser("myapp", "A sample application");
```

### Adding Arguments

- `.addArg(name: string): void` — add a positional argument
- `.addArg(name: string, description: string): void` — add a positional argument with a description
- `.addFlag(name: string): void` — add a boolean flag (e.g. `--verbose`)
- `.addFlag(name: string, description: string): void` — add a flag with a description
- `.addOption(name: string, short: char): void` — add an option with a short form (e.g. `-o`)
- `.addOption(name: string, short: char, default: string): void` — add an option with a default value
- `.addOption(name: string, short: char, description: string): void` — add an option with a description

```titrate
let parser = new ArgumentParser("myapp", "A file processor");
parser.addArg("input", "Input file path");
parser.addFlag("verbose", "Enable verbose output");
parser.addOption("output", 'o', "Output file path");
parser.addOption("format", 'f', "json");  // default is "json"
```

### Parsing

- `.parse(args: ArrayList<string>): Arguments` — parse an array of argument strings
- `.parseOrExit(args: ArrayList<string>): Arguments` — parse and print help on error, then exit

```titrate
let args = Sys.args();
let parsed = parser.parse(args);
```

### Help

- `.help(): string` — generate a help message
- `.printHelp(): void` — print the help message to stdout

```titrate
parser.printHelp();
// Usage: myapp [options] <input>
//
// A file processor
//
// Arguments:
//   input        Input file path
//
// Options:
//   --verbose    Enable verbose output
//   -o, --output Output file path
//   -f, --format Output format (default: json)
```

## Arguments

The `Arguments` type holds the parsed values.

### Accessing Values

- `.get(name: string): string` — get the value of a positional argument or option
- `.has(name: string): bool` — check if a flag was present
- `.getInt(name: string): int` — get a value parsed as an integer
- `.getDouble(name: string): double` — get a value parsed as a double

```titrate
let input = parsed.get("input");
let verbose = parsed.has("verbose");
let output = parsed.get("output");
let format = parsed.get("format");
```

## Complete Example

```titrate
import tt.argparse;
import tt.sys;
import tt.io;

public fn main(): void {
    let parser = new ArgumentParser("grep", "Search for patterns in files");
    parser.addArg("pattern", "The pattern to search for");
    parser.addArg("file", "The file to search in");
    parser.addFlag("ignore-case", "Case-insensitive matching");
    parser.addFlag("line-numbers", "Show line numbers");
    parser.addOption("context", 'c', "2");  // lines of context, default 2

    let args = parser.parseOrExit(Sys.args());

    let pattern = args.get("pattern");
    let file = args.get("file");
    let ignoreCase = args.has("ignore-case");
    let showNumbers = args.has("line-numbers");
    let context = args.getInt("context");

    io::println("Searching for '" + pattern + "' in " + file);
    // ... search logic ...
}
```

## Subcommands

- `Argparse.addSubparser(parser: Parser, name: string, help: string): Parser` — add subcommand
- `Argparse.parseKnownArgs(parser: Parser, args: ArrayList<string>): HashMap<string, Variant>` — parse known args only

## Mutually Exclusive Groups

- `Argparse.addMutuallyExclusiveGroup(parser: Parser, required: bool): ArgumentGroup` — create mutually exclusive group
- `ArgumentGroup.addArgument(name: string, options: HashMap<string, Variant>): void` — add argument to group

## Argument Groups

- `Argparse.addArgumentGroup(parser: Parser, title: string): ArgumentGroup` — create argument group
- `ArgumentGroup.addArgument(name: string, options: HashMap<string, Variant>): void` — add argument to group

## Custom Types

- `Argparse.registerType(parser: Parser, typeName: string, converter: fn(string): Variant): void` — register custom type

## Custom Actions

- `Argparse.registerAction(parser: Parser, actionName: string, action: fn(string, Variant, Variant): void): void` — register custom action
