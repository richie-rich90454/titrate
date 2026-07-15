# Tokenize

The `tt.lang.Tokenize` module provides Titrate source-code tokenization. It mirrors Python's `tokenize` module, exposing `tokenize(source)`, `generateTokens(source)`, `untokenize(tokens)`, and the `TokenInfo` class. Token type constants are reused from `tt::lang::Token` (`NAME`, `NUMBER`, `STRING`, `OP`, `NEWLINE`, `INDENT`, `DEDENT`, `COMMENT`, `ENDMARKER`). Indentation is tracked with a stack so that `INDENT`/`DEDENT` tokens are emitted at the start of logical lines.

## Import

```titrate
import tt::lang::Tokenize;
```

## Classes

### TokenInfo

Mirrors Python's `tokenize.TokenInfo` namedtuple: `(type, string, start, end, line)`. The `start` and `end` positions are encoded as separate `(row, col)` fields.

**Fields:**
- `type: int` — token type constant (e.g. `Token.NAME`, `Token.NUMBER`)
- `string: string` — the matched source text
- `startRow: int` / `startCol: int` — start position (1-based row, 0-based column)
- `endRow: int` / `endCol: int` — end position
- `line: string` — the source line containing the token

**Constructors:**
- `init(type: int, str: string, startRow: int, startCol: int, endRow: int, endCol: int, line: string)`

**Methods:**
- `typeName(): string` — return the human-readable type name (e.g. `"NAME"`)
- `toString(): string` — full debug representation

### TokenScanner

The internal scanner that drives tokenization. Exposed for callers that need to scan incrementally.

- `init(source: string)`
- `scanAll(): ArrayList<TokenInfo>` — scan the entire source and return all tokens

## Functions

### tokenize

- `Tokenize.tokenize(source: string): ArrayList<TokenInfo>` — tokenize a full Titrate source string and return the list of `TokenInfo`, including `INDENT`/`DEDENT`/`NEWLINE`/`ENDMARKER`. Operates on a full source string instead of a `readline` callable (Titrate closures over mutable state are cumbersome).

```titrate
let tokens: ArrayList<TokenInfo> = Tokenize.tokenize("let x = 42");
```

### generateTokens

- `Tokenize.generateTokens(source: string): ArrayList<TokenInfo>` — alias for `tokenize`; mirrors Python's `tokenize.generate_tokens`. Returns the same token stream.

### untokenize

- `Tokenize.untokenize(tokens: ArrayList<TokenInfo>): string` — reconstruct source text from a token list. Inserts spaces between tokens and newlines after `NEWLINE` tokens. Indentation is reconstructed from `INDENT`/`DEDENT` tokens.

```titrate
let tokens = Tokenize.tokenize("let x = 42");
let restored: string = Tokenize.untokenize(tokens);
```

## Usage Example

```titrate
import tt::lang::Tokenize;

public fn main(): void {
    let source: string = "fn greet(name: string): void {\n    io::println(name);\n}\n";
    let tokens: ArrayList<TokenInfo> = Tokenize.tokenize(source);
    var i: int = 0;
    while (i < tokens.size()) {
        let tok: TokenInfo = tokens.get(i);
        io::println(tok.typeName() + ": '" + tok.string + "'");
        i = i + 1;
    }
    let restored: string = Tokenize.untokenize(tokens);
    io::println("Restored source:");
    io::println(restored);
}
```
