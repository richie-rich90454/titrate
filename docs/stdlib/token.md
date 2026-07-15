# Token

The `tt.lang.Token` module mirrors Python's `token` module. It exposes the Titrate tokenizer's token-type constants (loaded from `data/lang/token_types.json`) and helper functions for working with tokens.

## Import

```titrate
import tt::lang::Token;
```

## Token type constants

These are integer constants (exposed as zero-argument functions in Titrate) classifying each token the tokenizer produces. The complete list is loaded from `data/lang/token_types.json`; the standard set is:

### Whitespace & separators
- `NEWLINE(): int`
- `NL(): int` — non-logical newline (continuation line)
- `INDENT(): int`
- `DEDENT(): int`
- `COMMENT(): int`
- `ENDMARKER(): int` — end-of-input marker
- `ENCODING(): int` — implicit source-encoding declaration

### Identifiers & literals
- `NAME(): int`
- `NUMBER(): int`
- `STRING(): int`
- `FSTRING_START(): int`, `FSTRING_MIDDLE(): int`, `FSTRING_END(): int`
- `BYTES(): int`

### Operators & delimiters
- `OP(): int` — generic operator/delimiter (use `EXACT_TOKEN_TYPES` for finer classification)
- `ASYNC(): int`
- `AWAIT(): int`
- `ERRORTOKEN(): int` — token that caused a tokenizer error

### Exact-token names

For each punctuation token the tokenizer also exposes a name:

- `LPAR` `(`, `RPAR` `)`, `LSQB` `[`, `RSQB` `]`, `COLON` `:`, `COMMA` `,`, `SEMI` `;`
- `PLUS` `+`, `MINUS` `-`, `STAR` `*`, `SLASH` `/`, `VBAR` `|`, `AMPER` `&`, `LESS` `<`, `GREATER` `>`, `EQUAL` `=`, `DOT` `.`, `PERCENT` `%`, `AT` `@`
- `PLUSEQ` `+=`, `MINEQ` `-=`, `STAREQ` `*=`, `SLASHEQ` `/=`, `VBAREQ` `|=`, `AMPEREQ` `&=`, `LESSEQ` `<=`, `GREATEREQ` `>=`, `EQEQ` `==`, `NEQ` `!=`, `PERCENTEQ` `%=`, `ATEQ` `@=`
- `RARROW` `->`, `ELLIPSIS` `...`, `COLONEQ` `:=`
- `TILDE` `~`, `CIRCUMFLEX` `^`, `CIRCUMFLEXEQ` `^=`, `LTLT` `<<`, `GTGT` `>>`, `LTLEQ` `<<=`, `GTGTEQ` `>>=`
- `STARSTAR` `**`, `DOTDOTDOT` `...`, `DOLLAR` `$`, `EQEQUALS` `===`

## TokenInfo

`TokenInfo` describes a single token in a tokenized source stream.

- `TokenInfo.init(type: int, string: string, start: int, end: int, line: string)`
- `type(): int` — token-type constant
- `string(): string` — the matched text
- `start(): int` — start offset (0-based)
- `end(): int` — end offset (0-based, exclusive)
- `line(): string` — the source line containing the token
- `toString(): string` — `TokenInfo(type=NAME, string='foo', start=(1, 0), end=(1, 3), line='foo')`

## Functions

### tokName

Return the human-readable name of the token type `type`.

**Parameters:** `type: int`
**Returns:** `string`

```titrate
io::println(tokName(NAME()));  // "NAME"
```

### nameOf

Alias for `tokName`.

### valueOf

Inverse of `tokName`: given a token-type name, return the integer constant.

**Parameters:** `name: string`
**Returns:** `int` (or `-1` if unknown)

### isToken

Return `true` if `type` is a known token-type constant.

**Parameters:** `type: int`
**Returns:** `bool`

### isPunctuation

Return `true` if `type` is one of the punctuation tokens (`OP`, `LPAR`, `RPAR`, etc.).

**Parameters:** `type: int`
**Returns:** `bool`

### tokList

Return a list of `(name, value)` pairs for every known token type, sorted by value.

**Returns:** `ArrayList<(string, int)>`

```titrate
let all: ArrayList<(string, int)> = tokList();
io::println(Integer.toString(all.size()));  // ~60+
```

## Notes

- The token-type constants are loaded from `data/lang/token_types.json` at module load; modifying that file changes the values returned by the functions.
- All constants are exposed as zero-argument functions returning the integer value; this matches Titrate's convention for runtime-loaded constants.
- The `Tokenize` module (`tt.lang.Tokenize`) uses these constants when emitting `TokenInfo` instances.
