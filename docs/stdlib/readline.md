# Readline

The `tt.tooling.Readline` module provides line-editing and command-history facilities for interactive interpreters. It mirrors Python's `readline` module, exposing history read/write/append, item manipulation, completion registration, and a startup hook. State is held at module level so multiple callers share a single history buffer.

## Import

```titrate
import tt::tooling::Readline;
```

## Functions

### parse_and_bind

- `Readline.parse_and_bind(s: string): void` — parse a readline init line of the form `"key: action"` (e.g. `"tab: complete"`) and store the binding

```titrate
Readline.parse_and_bind("tab: complete");
```

### read_history

- `Readline.read_history(filename: string): void` — load history entries from `filename` (one entry per line) into the module-level buffer; a missing file is silently ignored

```titrate
Readline.read_history(".titrate_history");
```

### write_history

- `Readline.write_history(filename: string): void` — overwrite `filename` with the current history, one entry per line

```titrate
Readline.write_history(".titrate_history");
```

### append_history

- `Readline.append_history(filename: string): void` — append the current history to `filename`, preserving any existing content

### get_history_item

- `Readline.get_history_item(index: int): string` — return the history entry at the 1-based `index`, or `""` if out of range

```titrate
let first: string = Readline.get_history_item(1);
```

### get_current_history_length

- `Readline.get_current_history_length(): int` — return the number of entries currently in the history buffer

### get_history_length

- `Readline.get_history_length(): int` — return the configured maximum history length, or `-1` for unlimited

### set_history_length

- `Readline.set_history_length(length: int): void` — set the maximum history length; a negative value means unlimited. Excess entries are trimmed immediately.

```titrate
Readline.set_history_length(500);
```

### clear_history

- `Readline.clear_history(): void` — remove all entries from the history buffer

### remove_history_item

- `Readline.remove_history_item(index: int): void` — remove the entry at the 1-based `index`; out-of-range indices are ignored

### replace_history_item

- `Readline.replace_history_item(index: int, line: string): void` — replace the entry at the 1-based `index` with `line`; out-of-range indices are ignored

### set_completer

- `Readline.set_completer(completer: fn(string, int): string): void` — register a Tab-completion function called as `completer(text, state)` returning the `state`-th match or `""` when exhausted

```titrate
let completer: fn(string, int): string = fn(text: string, state: int): string => {
    let candidates = new ArrayList<string>();
    candidates.add("start"); candidates.add("stop"); candidates.add("status");
    if (state < candidates.size()) {
        return candidates.get(state);
    }
    return "";
};
Readline.set_completer(completer);
```

### get_completer

- `Readline.get_completer(): fn(string, int): string` — return the currently registered completer, or `null` if none

### set_startup_hook

- `Readline.set_startup_hook(hook: fn(): void): void` — register a hook invoked before each prompt is displayed

```titrate
Readline.set_startup_hook(fn(): void => { io::println("-- ready --"); });
```

### add_history

- `Readline.add_history(line: string): void` — append `line` to the history buffer, trimming to the configured maximum length

```titrate
Readline.add_history("print(\"hello\")");
```

## Usage Example

```titrate
import tt::tooling::Readline;

public fn main(): void {
    Readline.read_history(".titrate_history");
    Readline.set_history_length(100);
    Readline.add_history("io::println(\"first\")");
    Readline.add_history("io::println(\"second\")");
    io::println(Readline.get_history_item(1));
    Readline.write_history(".titrate_history");
    Readline.clear_history();
}
```
