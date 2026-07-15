# Code

The `tt.tooling.Code` module provides a Python `code` module analog for interactive REPL interpreters: `InteractiveInterpreter`, `InteractiveConsole`, `interact`, `push`, `runsource`, `runcode`, and `compile_command`.

## Import

```titrate
import tt::tooling::Code;
```

## Classes

### InteractiveInterpreter

Base class: runs Titrate source strings in an interactive context.

**Fields:**
- `locals: HashMap<string, Variant>`
- `filename: string` — defaults to `"<console>"`

**Methods:**
- `runsource(source: string): bool` — run a string of source; returns `true` if more input is needed (incomplete), `false` if executed or errored
- `runcode(code: string): void` — execute a complete code string, catching and reporting errors
- `showtraceback(exc: string): void` — display a traceback for an exception
- `write(data: string): void` — write to the console
- `reset(): void` — reset the interpreter's local namespace

### InteractiveConsole

An `InteractiveInterpreter` with line-buffered input and history. Extends `InteractiveInterpreter`.

**Fields:**
- `prompt: string` — defaults to `">>> "`
- `promptContinuation: string` — defaults to `"... "`
- `buffer: ArrayList<string>`
- `history: ArrayList<string>`

**Methods:**
- `push(line: string): bool` — push a single line of input; returns `true` if more is needed
- `interact(banner: string): void` — begin the interactive loop reading from stdin until EOF
- `reset(): void` — reset the console state (buffer and history preserved)

## Functions

### interact

Convenience: create a console and start the interactive loop.

**Parameters:** `banner: string`
**Returns:** `void`

```titrate
interact("Titrate console");
```

### compile_command

Determine whether a source string is a complete command. Mirrors `codeop.compile_command`.

**Parameters:** `source: string`
**Returns:** `string` — the source if complete, `""` if incomplete, `null` on syntax error

```titrate
let r: string = compile_command("let x = 1");
// "let x = 1" if complete
```
