# Cmd

The `tt.tooling.Cmd` module provides a framework for building line-oriented command interpreters (REPLs). It mirrors Python's `cmd` module, exposing the `Cmd` class with `cmdloop`, `onecmd`, `precmd`/`postcmd`, `preloop`/`postloop`, `emptyline`, `default`, `do_help`, and `completedefault`. Subclasses register commands via `registerCommand` and implement `do_<command>` methods.

## Import

```titrate
import tt::tooling::Cmd;
```

## Classes

### Cmd

A simple framework for writing line-oriented command interpreters. Subclasses define `do_<command>` methods to implement commands.

**Fields:**
- `prompt: string` — prompt issued at the start of each iteration (default `"(Cmd) "`)
- `intro: string` — banner printed once when the loop starts (default `""`)
- `docLeader: string` — header printed before documentation listings
- `docHeader: string` — header for documented commands (default `"Documented commands (type help <topic>):"`)
- `miscHeader: string` — header for miscellaneous help topics
- `undocHeader: string` — header for undocumented commands
- `ruler: string` — character used to draw separator lines under headers (default `"="`)
- `useRawInput: bool` — whether to use raw input mode (default `true`)
- `_commandNames: ArrayList<string>` — registry of recognized command names

**Constructors:**
- `init()` — initialize all fields to defaults with an empty command registry

**Lifecycle methods (override to customize):**
- `preloop(): void` — called once before the loop begins
- `postloop(): void` — called once after the loop ends
- `precmd(line: string): string` — pre-process a command line; returns the (possibly modified) line
- `postcmd(stop: bool, line: string): bool` — post-process a command result; `stop` is `true` if the command requested termination. Returns the (possibly modified) stop flag.

**Built-in command handlers:**
- `emptyline(): bool` — called when an empty line is entered (default no-op returning `false`)
- `default(line: string): bool` — called for unrecognized commands (prints `*** Unknown syntax: <line>`)
- `do_help(arg: string): bool` — print help; with no `arg`, lists registered commands
- `do_exit(arg: string): bool` — return `true` to stop the loop
- `do_quit(arg: string): bool` — alias for `do_exit`
- `do_eof(arg: string): bool` — handle end-of-input (returns `true` to stop)

**Dispatch:**
- `onecmd(line: string): bool` — dispatch a single command line through `precmd` -> `_dispatch` -> `postcmd`; returns `true` if the loop should stop
- `completedefault(text: string, line: string, begidx: int, endidx: int): ArrayList<string>` — default Tab-completion returning an empty list; override for custom completion

**Registration:**
- `registerCommand(name: string): void` — register a command name so `_dispatch` will recognize it

**Main loop:**
- `cmdloop(intro: string): void` — run the REPL: calls `preloop`, prints `intro` (argument or field), reads lines and dispatches via `onecmd` until stopped, then calls `postloop`

```titrate
public class MyShell extends Cmd {
    public fn init() {
        super.init();
        this.prompt = "mysh> ";
        this.registerCommand("greet");
        this.registerCommand("exit");
    }

    public fn do_greet(arg: string): bool {
        io::println("Hello, " + arg + "!");
        return false;
    }
}

public fn main(): void {
    let shell = new MyShell();
    shell.cmdloop("Welcome. Type 'help' for commands.");
}
```

## Usage Example

```titrate
import tt::tooling::Cmd;

public class CalculatorShell extends Cmd {
    public fn init() {
        super.init();
        this.prompt = "calc> ";
        this.registerCommand("add");
        this.registerCommand("exit");
    }

    public fn do_add(arg: string): bool {
        let parts: ArrayList<string> = String.split(arg, " ");
        let a: int = Integer.parseInt(String.trim(parts.get(0)));
        let b: int = Integer.parseInt(String.trim(parts.get(1)));
        io::println(Integer.toString(a + b));
        return false;
    }
}

public fn main(): void {
    let calc = new CalculatorShell();
    calc.cmdloop("Calculator. Try: add 2 3");
}
```
