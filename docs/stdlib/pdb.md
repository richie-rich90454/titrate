# Pdb

The `tt.debug.Pdb` module mirrors Python's `pdb` module. It provides an interactive source-level debugger with breakpoints, stepping, and variable inspection. Because Titrate has no real interactive terminal, the debugger reads commands from an input queue that tests can populate via `pushCommand`; the same command set is accepted as in CPython's `pdb`.

## Import

```titrate
import tt::debug::Pdb;
```

## Breakpoint

`Breakpoint` describes a single source-level breakpoint.

- `Breakpoint.init(filename: string, line: int, condition: string, temporary: bool)`
- `filename(): string`
- `line(): int`
- `condition(): string` — empty string means unconditional
- `temporary(): bool` — true if the breakpoint is removed after the first hit
- `isEnabled(): bool`
- `setEnabled(enabled: bool): void`
- `hitCount(): int`
- `incrementHitCount(): void`

## Pdb

The `Pdb` class drives the debugging session.

### Construction

- `Pdb.init()`
- `Pdb.init(commands: ArrayList<string>)` — pre-seed the input queue

### Session control

- `run(cmd: string): void` — execute the source string under the debugger
- `runeval(cmd: string): Variant` — execute and return the result of the trailing expression
- `runcall(fn: Variant, args: ArrayList<Variant>): Variant` — call a function under the debugger

### Tracing

- `setTrace(): void` — start tracing from the next executed statement
- `setContinue(): void` — stop tracing until the next breakpoint is hit
- `setNext(): void` — step over the current function call
- `setStep(): void` — single-step (into function calls)
- `setReturn(): void` — run until the current function returns
- `setQuit(): void` — terminate the debug session

### Breakpoints

- `setBreak(filename: string, line: int, condition: string, temporary: bool): int` — install a breakpoint and return its number
- `clearBreak(bpnum: int): void`
- `clearAllBreaks(): void`
- `getBreakpoints(): ArrayList<Breakpoint>`
- `enableBreak(bpnum: int, enabled: bool): void`

### Inspection

- `where(): string` — print the current call stack
- `list(filename: string, first: int, last: int): string` — list source lines
- `printExpression(expr: string): string` — evaluate `expr` in the current frame and return its string form
- `locals(): HashMap<string, Variant>` — current frame's local variables
- `globals(): HashMap<string, Variant>`

### Command input

- `pushCommand(cmd: string): void` — push a single command string onto the input queue
- `pushCommands(cmds: ArrayList<string>): void`

## Module-level functions

### set_trace

Install a `Pdb` tracer and break at the call site. Returns immediately if no debugger input is available.

**Returns:** `void`

```titrate
fn risky(): void {
    set_trace();
    // ... code below runs under the debugger ...
}
```

### post_mortem

Enter post-mortem debugging for the given `traceback` (a `Traceback` object). The most recent frame is shown; the user can inspect variables and walk the stack.

**Parameters:** `traceback: Variant`
**Returns:** `void`

### breakpoint

Drop into the debugger at the call site. Honours the `TITRATE_BREAKPOINT` environment variable: if it is set to `"0"`, the call is a no-op; otherwise it delegates to `set_trace`.

**Returns:** `void`

### run

Equivalent to `new Pdb().run(cmd)`.

**Parameters:** `cmd: string`
**Returns:** `void`

```titrate
let p: Pdb = new Pdb();
p.pushCommands(["break risky:3", "continue", "print x", "continue"]);
p.run("risky();");
```

## Command reference

When the debugger breaks, it reads commands from its input queue. The commands mirror CPython's `pdb`:

- `s`, `step` — step into
- `n`, `next` — step over
- `r`, `return` — run until return
- `c`, `continue` — continue until next breakpoint
- `q`, `quit` — terminate the session
- `b LINE`, `break LINE` — set a breakpoint at `LINE` in the current file
- `b FILE:LINE` — set a breakpoint in `FILE:LINE`
- `b FILE:LINE, COND` — set a conditional breakpoint
- `cl N`, `clear N` — clear breakpoint number `N`
- `disable N`, `enable N` — toggle a breakpoint
- `p EXPR`, `print EXPR` — print the value of `EXPR`
- `pp EXPR` — pretty-print the value of `EXPR`
- `l`, `list` — list source around the current line
- `w`, `where` — print the call stack
- `u`, `up` — move one frame up the stack
- `d`, `down` — move one frame down the stack
- `a`, `args` — print the arguments of the current function
- `h`, `help` — print help

Unknown commands are reported but do not terminate the session.
