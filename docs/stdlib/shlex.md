# shlex

The `tt.text` module provides `Shlex` — shell-like lexical parsing for splitting strings into tokens respecting quotes and escapes.

```titrate
import tt.text.Shlex;
```

## Shlex

A configurable shell-like lexer that splits strings into tokens, handling quoted strings, escape sequences, and comments.

- `fn init()` — create a Shlex instance with default settings
- `split(s: string): ArrayList<string>` — split a string into tokens using shell-like rules

```titrate
let lexer: Shlex = new Shlex();
let tokens: ArrayList<string> = lexer.split('hello "world of shells" --flag=value');
// ["hello", "world of shells", "--flag=value"]
```

## Free Functions

- `Shlex.split(s: string): ArrayList<string>` — split a string into tokens using shell-like quoting rules (convenience function)
- `Shlex.quote(s: string): string` — return a shell-escaped version of the string, adding quotes if necessary

```titrate
let parts: ArrayList<string> = Shlex.split('cp "my file.txt" backup/');
// ["cp", "my file.txt", "backup/"]

io::println(Shlex.quote("hello world"));   // "'hello world'"
io::println(Shlex.quote("simple"));        // "simple"
```
