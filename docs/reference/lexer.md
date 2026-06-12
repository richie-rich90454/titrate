# Lexer Tokens

The Titrate lexer (also called the scanner or tokenizer) is the first phase of the compiler. It reads source code as a stream of characters and converts it into a stream of **tokens** — the smallest meaningful units that the parser can work with.

## Introduction to the Lexer

The lexer performs several critical tasks:

1. **Tokenization**: Breaking source text into tokens (keywords, identifiers, operators, literals, punctuation)
2. **Whitespace removal**: Discarding spaces, tabs, and newlines that aren't significant
3. **Comment removal**: Stripping out single-line and multi-line comments
4. **Literal parsing**: Converting text representations of numbers and strings into their internal forms
5. **Error detection**: Reporting invalid characters or malformed tokens

The lexer operates as a single-pass scanner, reading characters left-to-right and producing tokens on demand. It is implemented in `trc/src/lexer/scanner.rs`.

## Tokenization Process

The lexer follows a straightforward process:

```
Source Code          Lexer           Token Stream
──────────────  →  ────────  →  ──────────────────────
let x: int = 42;     [Let, Identifier("x"), Colon, Int, Equals, IntLiteral(42), Semicolon]
```

### How It Works

1. **Peek and consume**: The lexer peeks at the next character and decides what kind of token to produce
2. **Longest match**: When multiple tokens could start with the same character (e.g., `=` vs `==` vs `=>`), the lexer always chooses the longest match
3. **Keyword check**: After reading an identifier, the lexer checks if it matches a reserved keyword
4. **Emit token**: The completed token is emitted with its type and (for literals) its value

### Example Tokenization

```titrate
let sum: int = a + b;
```

Produces the following token stream:

| # | Token | Lexeme |
|---|-------|--------|
| 1 | `Let` | `let` |
| 2 | `Identifier("sum")` | `sum` |
| 3 | `Colon` | `:` |
| 4 | `Int` | `int` |
| 5 | `Equals` | `=` |
| 6 | `Identifier("a")` | `a` |
| 7 | `Plus` | `+` |
| 8 | `Identifier("b")` | `b` |
| 9 | `Semicolon` | `;` |

## Keywords

Keywords are reserved identifiers that have special meaning in the language. They cannot be used as variable, function, or class names.

### Access Modifiers

| Token | Lexeme | Description |
|-------|--------|-------------|
| `Public` | `public` | Makes a declaration visible outside its module |
| `Private` | `private` | Restricts a declaration to its containing class or module |

### Declaration Keywords

| Token | Lexeme | Description |
|-------|--------|-------------|
| `Fn` | `fn` | Declares a function or method |
| `Class` | `class` | Declares a class type |
| `Interface` | `interface` | Declares an interface type |
| `Enum` | `enum` | Declares an enumeration type |
| `Extends` | `extends` | Indicates class inheritance |
| `Implements` | `implements` | Indicates interface implementation |
| `Let` | `let` | Declares an immutable variable binding |
| `Var` | `var` | Declares a mutable variable binding |
| `Const` | `const` | Declares a compile-time constant |

### Control Flow Keywords

| Token | Lexeme | Description |
|-------|--------|-------------|
| `Do` | `do` | Begins a do-while loop body |
| `If` | `if` | Conditional branch |
| `Else` | `else` | Alternative branch (if-else, if-elif) |
| `While` | `while` | While loop condition |
| `For` | `for` | For-in loop iteration |
| `Return` | `return` | Returns a value from a function |
| `Break` | `break` | Exits the innermost loop |
| `Continue` | `continue` | Skips to the next loop iteration |
| `Switch` | `switch` | Multi-way branch statement |
| `Case` | `case` | A case label in a switch statement |
| `Default` | `default` | The default case in a switch statement |
| `With` | `with` | Context manager statement |

### Literal Keywords

| Token | Lexeme | Description |
|-------|--------|-------------|
| `True` | `true` | Boolean true literal |
| `False` | `false` | Boolean false literal |
| `Null` | `null` | Null reference literal |

### Object-Oriented Keywords

| Token | Lexeme | Description |
|-------|--------|-------------|
| `New` | `new` | Creates a new heap-allocated instance |
| `This` | `this` | Reference to the current instance |
| `Super` | `super` | Reference to the parent class |

### Result Type Keywords

| Token | Lexeme | Description |
|-------|--------|-------------|
| `Result` | `Result` | The Result type for error handling |
| `Ok` | `Ok` | Success variant of Result |
| `Err` | `Err` | Error variant of Result |

> **Note:** `Ok` and `Err` are lexer keywords with dedicated parsing and bytecode opcodes. The lowercase `ok()` and `err()` are standard library convenience functions that produce the same `Result` values. Both forms are always available.

### Ownership Keywords

| Token | Lexeme | Description |
|-------|--------|-------------|
| `Owned` | `Owned` | Single-owner smart pointer type |
| `Region` | `region` | Scoped allocation arena |
| `Unsafe` | `unsafe` | Suspends borrow checking |

### Type Operation Keywords

| Token | Lexeme | Description |
|-------|--------|-------------|
| `As` | `as` | Type cast operator |
| `Is` | `is` | Type check operator |
| `Type` | `type` | Type alias declaration |

### Module Keywords

| Token | Lexeme | Description |
|-------|--------|-------------|
| `Import` | `import` | Imports a module |
| `Module` | `module` | Declares a module |
| `Where` | `where` | Constraint clause (for generics) |

## Type Keywords

Type keywords are reserved words that name the built-in primitive types:

| Token | Lexeme | Size | Description |
|-------|--------|------|-------------|
| `Void` | `void` | 0 | No value (return type only) |
| `Bool` | `bool` | 1 | Boolean (`true`/`false`) |
| `Byte` | `byte` | 1 | Signed 8-bit integer |
| `Short` | `short` | 2 | Signed 16-bit integer |
| `Int` | `int` | 4 | Signed 32-bit integer |
| `Long` | `long` | 8 | Signed 64-bit integer |
| `Vast` | `vast` | 16 | Signed 128-bit integer |
| `Uvast` | `uvast` | 16 | Unsigned 128-bit integer |
| `Float` | `float` | 4 | IEEE 754 single-precision |
| `Double` | `double` | 8 | IEEE 754 double-precision |
| `Half` | `half` | 2 | IEEE 754 half-precision |
| `Quad` | `quad` | 16 | IEEE 754 quad-precision |
| `Char` | `char` | 2 | Unicode character (UTF-16) |
| `String` | `string` | ref | UTF-8 string (reference type) |
| `Size` | `size` | 8 | Platform-size unsigned integer |
| `U8` | `u8` | 1 | Unsigned 8-bit integer |
| `U16` | `u16` | 2 | Unsigned 16-bit integer |
| `U32` | `u32` | 4 | Unsigned 32-bit integer |
| `U64` | `u64` | 8 | Unsigned 64-bit integer |

## Operators and Punctuation

### Arithmetic Operators

| Token | Lexeme | Description |
|-------|--------|-------------|
| `Plus` | `+` | Addition |
| `Minus` | `-` | Subtraction / negation |
| `Star` | `*` | Multiplication |
| `Slash` | `/` | Division |
| `Percent` | `%` | Modulo |

### Comparison Operators

| Token | Lexeme | Description |
|-------|--------|-------------|
| `EqualEqual` | `==` | Equality |
| `NotEqual` | `!=` | Inequality |
| `Less` | `<` | Less than |
| `Greater` | `>` | Greater than |
| `LessEqual` | `<=` | Less than or equal |
| `GreaterEqual` | `>=` | Greater than or equal |

### Logical Operators

| Token | Lexeme | Description |
|-------|--------|-------------|
| `AndAnd` | `&&` | Logical AND (short-circuit) |
| `OrOr` | `\|\|` | Logical OR (short-circuit) |
| `Not` | `!` | Logical NOT |

### Bitwise Operators

| Token | Lexeme | Description |
|-------|--------|-------------|
| `Ampersand` | `&` | Bitwise AND / immutable borrow |
| `Pipe` | `\|` | Bitwise OR |
| `Caret` | `^` | Bitwise XOR |
| `Tilde` | `~` | Bitwise NOT (complement) |
| `LeftShift` | `<<` | Left shift |
| `RightShift` | `>>` | Right shift |

### Increment / Decrement

| Token | Lexeme | Description |
|-------|--------|-------------|
| `PlusPlus` | `++` | Increment by 1 |
| `MinusMinus` | `--` | Decrement by 1 |

### Assignment Operators

| Token | Lexeme | Description |
|-------|--------|-------------|
| `Equals` | `=` | Assignment |
| `PlusEqual` | `+=` | Add and assign |
| `MinusEqual` | `-=` | Subtract and assign |
| `StarEqual` | `*=` | Multiply and assign |
| `SlashEqual` | `/=` | Divide and assign |
| `PercentEqual` | `%=` | Modulo and assign |
| `AmpersandEqual` | `&=` | Bitwise AND and assign |
| `PipeEqual` | `\|=` | Bitwise OR and assign |
| `CaretEqual` | `^=` | Bitwise XOR and assign |
| `LeftShiftEqual` | `<<=` | Left shift and assign |
| `RightShiftEqual` | `>>=` | Right shift and assign |

### Other Operators

| Token | Lexeme | Description |
|-------|--------|-------------|
| `ColonColon` | `::` | Import path separator / alt member access |
| `Arrow` | `->` | Thin arrow (return type in some contexts) |
| `FatArrow` | `=>` | Fat arrow (lambda body, match arm) |
| `Question` | `?` | Ternary conditional / early return on error |
| `Dot` | `.` | Member access |
| `DotDot` | `..` | Exclusive range |
| `DotDotEq` | `..=` | Inclusive range |
| `RefMut` | `&mut` | Mutable borrow operator |

> **Note:** The `::` operator is used in import paths (e.g., `import tt::math::Math`) and is also supported for member access for developers coming from C++. However, `.` (dot) is the preferred and idiomatic form for method calls in Titrate (e.g., `Math.sqrt(2.0)` instead of `Math::sqrt(2.0)`).

### Punctuation

| Token | Lexeme | Description |
|-------|--------|-------------|
| `Comma` | `,` | Separator in lists |
| `Semicolon` | `;` | Statement terminator |
| `Colon` | `:` | Type annotation separator |
| `LeftParen` | `(` | Open parenthesis |
| `RightParen` | `)` | Close parenthesis |
| `LeftBrace` | `{` | Open brace |
| `RightBrace` | `}` | Close brace |
| `LeftBracket` | `[` | Open bracket |
| `RightBracket` | `]` | Close bracket |

## Literals

Literals are tokens that represent fixed values in the source code.

### Integer Literals

Integers can be written in decimal, hexadecimal, octal, or binary:

| Format | Prefix | Example | Value |
|--------|--------|---------|-------|
| Decimal | (none) | `42` | 42 |
| Hexadecimal | `0x` | `0xFF` | 255 |
| Octal | `0o` | `0o77` | 63 |
| Binary | `0b` | `0b1010` | 10 |

```titrate
let dec: int = 42;       // decimal
let hex: int = 0xFF;     // hexadecimal = 255
let oct: int = 0o77;     // octal = 63
let bin: int = 0b1010;   // binary = 10
```

Integer literals are stored as `IntLiteral(i64)` tokens.

### Floating-Point Literals

Floating-point numbers use a decimal point. Titrate supports three floating-point widths via suffixes:

| Suffix | Type | Size | Example |
|--------|------|------|---------|
| (none) | `double` | 8 bytes | `3.14` |
| `h` | `half` | 2 bytes | `1.5h` |
| `q` | `quad` | 16 bytes | `2.0q` |

```titrate
let d: double = 3.14;    // double-precision (default)
let h: half = 1.5h;      // half-precision
let q: quad = 2.0q;      // quad-precision
```

Float literals are stored as `FloatLiteral { value: f64, suffix: Option<FloatSuffix> }` tokens.

### String Literals

String literals are enclosed in double quotes and support escape sequences:

```titrate
let greeting: string = "Hello, world!";
let escaped: string = "Line 1\nLine 2\tTabbed";
```

String literals are stored as `StringLiteral(String)` tokens.

### Raw String Literals

Raw string literals are prefixed with `r` and do not process escape sequences. They are useful for regular expressions, file paths, and other strings that contain backslashes:

```titrate
let regex: string = r"\d+\.\d+";     // No need to escape backslashes
let path: string = r"C:\Users\name";  // Windows paths without escaping
```

Raw string literals are stored as `RawStringLiteral(String)` tokens.

### Character Literals

Character literals are enclosed in single quotes and represent a single Unicode character:

```titrate
let ch: char = 'a';
let newline: char = '\n';
```

Character literals are stored as `CharLiteral(char)` tokens.

### Byte Literals

Byte literals use a `b` prefix before single quotes and represent a single byte value (0–255):

```titrate
let byte: byte = b'A';   // ASCII value 65
```

Byte literals are stored as `ByteLiteral(u8)` tokens.

### Boolean Literals

```titrate
let yes: bool = true;
let no: bool = false;
```

Boolean literals are stored as `BoolLiteral(bool)` tokens.

### Null Literal

```titrate
let nothing: Variant = null;
```

The null literal is stored as a `NullLiteral` token.

## String Escape Sequences

Within regular string literals (not raw strings), the following escape sequences are recognized:

| Escape | Character | Description |
|--------|-----------|-------------|
| `\n` | U+000A | Newline (line feed) |
| `\r` | U+000D | Carriage return |
| `\t` | U+0009 | Horizontal tab |
| `\\` | U+005C | Backslash |
| `\"` | U+0022 | Double quote |
| `\'` | U+0027 | Single quote |
| `\0` | U+0000 | Null character |

```titrate
let multi: string = "Line 1\nLine 2\nLine 3";
let tabbed: string = "Name\tAge\tCity";
let quoted: string = "She said \"Hello\"";
let path: string = "C:\\Users\\name";  // or use raw strings: r"C:\Users\name"
```

## Comments

Comments are stripped by the lexer and do not produce tokens.

### Single-Line Comments

A single-line comment begins with `//` and extends to the end of the line:

```titrate
let x: int = 42;  // This is a comment
// This entire line is a comment
```

### Multi-Line Comments

A multi-line comment begins with `/*` and ends with `*/`. It can span multiple lines:

```titrate
/* This is a multi-line comment.
   It can span multiple lines.
   Useful for longer explanations. */
let x: int = 42;
```

Multi-line comments can also appear on a single line:

```titrate
let x: int = /* inline comment */ 42;
```

## Identifier Rules

Identifiers name variables, functions, classes, and other user-defined entities.

### Valid Identifiers

An identifier must:
- Start with a letter (`a`–`z`, `A`–`Z`) or underscore (`_`)
- Contain only letters, digits (`0`–`9`), and underscores
- Not be a reserved keyword

```titrate
// Valid identifiers
let name: string = "Alice";
let _count: int = 0;
let value2: double = 3.14;
let MAX_SIZE: int = 100;

// Invalid identifiers (would cause lexer errors)
// let 2ndValue = 10;    // starts with a digit
// let my-var = 5;       // contains a hyphen
// let class = "foo";    // reserved keyword
```

### Naming Conventions

While not enforced by the lexer, the following conventions are common in Titrate:

| Entity | Convention | Example |
|--------|-----------|---------|
| Variables | camelCase | `itemCount` |
| Constants | UPPER_SNAKE_CASE | `MAX_SIZE` |
| Functions | camelCase | `calculateTotal` |
| Classes | PascalCase | `ArrayList` |
| Interfaces | PascalCase | `Comparable` |
| Enums | PascalCase | `Color` |
| Private fields | camelCase | `internalState` |

## Whitespace Handling

Whitespace (spaces, tabs, newlines) is significant only as a separator between tokens. The lexer discards all whitespace after using it to delimit tokens:

```titrate
let x:int=42;     // Minimal whitespace — valid but hard to read
let x: int = 42;  // Conventional spacing — preferred
```

Indentation is **not** significant in Titrate (unlike Python). Code blocks are delimited by braces `{}`.

## Error Recovery

When the lexer encounters an invalid character, it produces an `Error(String)` token rather than immediately halting compilation. This allows the parser to report multiple errors in a single compilation pass.

Common lexer errors include:
- Unrecognized characters (e.g., `$`, `@`, `#`)
- Unterminated string literals
- Unterminated multi-line comments
- Invalid numeric literals

```titrate
let x: int = 42 $ 10;  // Error: unexpected character '$'
let s: string = "hello;  // Error: unterminated string literal
```

## Unicode Support

Titrate source files are encoded in UTF-8. The lexer handles Unicode in the following ways:

- **Identifiers**: May contain Unicode letters (but this is not recommended for consistency)
- **String literals**: May contain any Unicode characters
- **Character literals**: May contain Unicode characters
- **Comments**: May contain any Unicode characters

```titrate
let greeting: string = "こんにちは";  // Unicode in strings
let pi: double = 3.14159;            // ASCII identifiers (recommended)
```

## Tokenization Examples

### Variable Declaration

```titrate
let message: string = "hello";
```

| Token | Lexeme |
|-------|--------|
| `Let` | `let` |
| `Identifier("message")` | `message` |
| `Colon` | `:` |
| `String` | `string` |
| `Equals` | `=` |
| `StringLiteral("hello")` | `"hello"` |
| `Semicolon` | `;` |

### Function Definition

```titrate
public fn add(a: int, b: int): int {
    return a + b;
}
```

| Token | Lexeme |
|-------|--------|
| `Public` | `public` |
| `Fn` | `fn` |
| `Identifier("add")` | `add` |
| `LeftParen` | `(` |
| `Identifier("a")` | `a` |
| `Colon` | `:` |
| `Int` | `int` |
| `Comma` | `,` |
| `Identifier("b")` | `b` |
| `Colon` | `:` |
| `Int` | `int` |
| `RightParen` | `)` |
| `Colon` | `:` |
| `Int` | `int` |
| `LeftBrace` | `{` |
| `Return` | `return` |
| `Identifier("a")` | `a` |
| `Plus` | `+` |
| `Identifier("b")` | `b` |
| `Semicolon` | `;` |
| `RightBrace` | `}` |

### Import Statement

```titrate
import tt::math::Math;
```

| Token | Lexeme |
|-------|--------|
| `Import` | `import` |
| `Identifier("tt")` | `tt` |
| `ColonColon` | `::` |
| `Identifier("math")` | `math` |
| `ColonColon` | `::` |
| `Identifier("Math")` | `Math` |
| `Semicolon` | `;` |

### Class with Constructor

```titrate
public class Point {
    public fn init(x: double, y: double) {
        this.x = x;
        this.y = y;
    }
}
```

| Token | Lexeme |
|-------|--------|
| `Public` | `public` |
| `Class` | `class` |
| `Identifier("Point")` | `Point` |
| `LeftBrace` | `{` |
| `Public` | `public` |
| `Fn` | `fn` |
| `Identifier("init")` | `init` |
| `LeftParen` | `(` |
| `Identifier("x")` | `x` |
| `Colon` | `:` |
| `Double` | `double` |
| `Comma` | `,` |
| `Identifier("y")` | `y` |
| `Colon` | `:` |
| `Double` | `double` |
| `RightParen` | `)` |
| `LeftBrace` | `{` |
| `This` | `this` |
| `Dot` | `.` |
| `Identifier("x")` | `x` |
| `Equals` | `=` |
| `Identifier("x")` | `x` |
| `Semicolon` | `;` |
| `This` | `this` |
| `Dot` | `.` |
| `Identifier("y")` | `y` |
| `Equals` | `=` |
| `Identifier("y")` | `y` |
| `Semicolon` | `;` |
| `RightBrace` | `}` |
| `RightBrace` | `}` |
