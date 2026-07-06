# Error Code Catalog

Every error raised by the Titrate compiler is assigned a stable code in
one of four ranges:

| Range    | Stage          | Module                  |
|----------|----------------|-------------------------|
| `E0xxx`  | Lexer          | `trc::lexer`            |
| `E1xxx`  | Parser         | `trc::parser`           |
| `E2xxx`  | Semantic       | `trc::analyzer`         |
| `E3xxx`  | Codegen        | `trc::codegen`          |

Each error is rendered in the canonical diagnostic format:

```text
error[E0XXX]: <message>
  --> file.tr:L:C
   |
 L | <source line>
   | ^^^
```

The `code` constants live in [`trc/src/errors.rs`](../../trc/src/errors.rs).
The `classify_lexer_error`, `classify_parse_error`, and
`classify_semantic_error` functions map free-form error messages to the
stable codes; `render_error` produces the canonical diagnostic.

---

## Lexical errors (`E0xxx`)

| Code   | Constant                       | Message                                       | Remediation                                                              |
|--------|--------------------------------|-----------------------------------------------|--------------------------------------------------------------------------|
| E0001  | `LEX_UNTERMINATED_STRING`      | `Unterminated string at L:C`                  | Add the closing `"` before newline/EOF.                                 |
| E0002  | `LEX_UNTERMINATED_CHAR`        | `Unterminated char at L:C`                    | Add the closing `'`.                                                     |
| E0003  | `LEX_INVALID_STRING_ESCAPE`    | `Unknown char escape ...`                     | Use a supported escape: `\n \t \r \\ \" \' \0 \b \f \u{...}`.            |
| E0004  | `LEX_INVALID_HEX_ESCAPE`       | `Invalid hex escape ...`                      | Use `\xHH` with one or two hex digits.                                   |
| E0005  | `LEX_INVALID_UNICODE_ESCAPE`   | `Invalid unicode escape ...`                  | Use `\u{XXXX}` with 1–6 hex digits.                                      |
| E0006  | `LEX_EMPTY_UNICODE_ESCAPE`     | `Empty unicode escape ...`                    | Provide at least one hex digit inside `\u{...}`.                         |
| E0007  | `LEX_UNTERMINATED_UNICODE_ESCAPE` | `Unterminated unicode escape ...`          | Close the `{...}` of `\u{...}`.                                          |
| E0008  | `LEX_INVALID_CHAR_ESCAPE`      | `Unknown char escape ...`                     | Use a supported char escape (see E0003).                                 |
| E0009  | `LEX_INVALID_HEX_LITERAL`      | `Invalid hex literal at L:C: <reason>`        | Use `0x` followed by hex digits, e.g. `0xFF`.                            |
| E0010  | `LEX_INVALID_OCT_LITERAL`      | `Invalid oct literal at L:C`                  | Use `0o` followed by digits 0–7, e.g. `0o77`.                            |
| E0011  | `LEX_INVALID_BIN_LITERAL`      | `Invalid bin literal at L:C`                  | Use `0b` followed by 0/1 digits, e.g. `0b1010`.                          |
| E0012  | `LEX_INVALID_DEC_LITERAL`      | `Invalid decimal literal at L:C`              | Use only digits 0–9 (and `_` separators).                                |
| E0013  | `LEX_INVALID_FLOAT_LITERAL`    | `Invalid float literal at L:C`                | Use `digits.digits` with optional exponent, e.g. `3.14`, `1e10`.         |
| E0014  | `LEX_INVALID_CHAR_LITERAL`     | `Invalid char literal at L:C`                 | A char literal holds one Unicode codepoint: `'a'`, `'\n'`.               |
| E0015  | `LEX_INVALID_BYTE_LITERAL`     | `Invalid byte literal at L:C`                 | A byte literal holds one ASCII byte: `b'x'`, `b'\n'`, `b'\x41'`.         |
| E0016  | `LEX_NUMBER_OUT_OF_RANGE`      | `number out of range ...`                     | Use a wider type (`long`, `vast`) or `half`/`quad` float suffix.         |
| E0017  | `LEX_INVALID_RAW_STRING_DELIM` | `Invalid raw string delimiter ...`            | Use `r"..."`, `r#"..."#`, or `r##"..."##` with matching `#` counts.      |
| E0018  | `LEX_UNTERMINATED_RAW_STRING`  | `Unterminated raw string ...`                 | Close the raw string with the matching `"###` delimiter.                 |
| E0019  | `LEX_INVALID_NUMBER_SUFFIX`    | `Invalid number suffix ...`                   | Use `h` (half), `q` (quad), or no suffix.                                |
| E0020  | `LEX_INVALID_FLOAT_SUFFIX`     | `Invalid float suffix ...`                    | Float suffixes are `h` (half) and `q` (quad) only.                       |
| E0021  | `LEX_MALFORMED_NUMBER`         | `Malformed number ...`                        | Use `digits.digits` or `digits.digits[eE][+-]digits`.                    |
| E0022  | `LEX_UNTERMINATED_BLOCK_COMMENT` | `Unterminated block comment ...`            | Close the block comment with `*/`.                                       |
| E0023  | `LEX_UNRECOGNIZED_CHAR`        | `Unrecognized character ...`                  | Use only ASCII or Unicode identifier characters.                         |
| E0099  | `LEX_UNKNOWN`                  | (fallback for unmatched lexer messages)       | Inspect the raw message; if it should be classified, extend `errors.rs`. |

---

## Parse errors (`E1xxx`)

| Code   | Constant                          | Message                                              | Remediation                                                       |
|--------|-----------------------------------|------------------------------------------------------|-------------------------------------------------------------------|
| E1000  | `PARSE_EXPECTED_TOKEN`            | `Expected <token>, found <token> at L:C`             | Insert the expected token at the indicated position.              |
| E1001  | `PARSE_EXPECTED_NAME`             | `Expected name/identifier, found <tok>`              | Provide an identifier where required.                             |
| E1002  | `PARSE_EXPECTED_TYPE`             | `Expected type, found <tok>`                         | Provide a type annotation (primitive or user-defined).            |
| E1003  | `PARSE_EXPECTED_TYPE_PARAM`       | `Expected type parameter name, found <tok>`          | Name each type parameter, e.g. `<T>` not `<>`.                    |
| E1004  | `PARSE_EXPECTED_WHERE_CLAUSE`     | (where-clause parse failure)                          | Use `where T: Bound` syntax.                                      |
| E1005  | `PARSE_EXPECTED_DECLARATION`      | `Expected declaration, found <tok>`                  | Top-level must be `fn`, `class`, `interface`, `enum`, `let`, `var`, `const`, or `import`. |
| E1006  | `PARSE_EXPECTED_EXPRESSION`       | `Expected expression, found <tok>`                   | Provide an expression where one is required.                      |
| E1007  | `PARSE_EXPECTED_PATTERN`          | `Expected pattern, found <tok>`                      | Use a literal, `_`, or `Constructor(bindings)` in `case`.         |
| E1008  | `PARSE_EXPECTED_STATEMENT`        | `Expected statement, found <tok>`                    | Provide a valid statement (let/var/const/if/while/...).           |
| E1009  | `PARSE_EXPECTED_CLOSE_ANGLE`      | `Expected >, found <tok>`                            | Close generic parameter list with `>`.                            |
| E1010  | `PARSE_EXPECTED_SEMICOLON`        | `Expected Semicolon, found <tok>`                    | Add `;` at end of statement.                                      |
| E1011  | `PARSE_EXPECTED_BRACE`            | `Expected LeftBrace, found <tok>`                    | Open block with `{`.                                              |
| E1012  | `PARSE_EXPECTED_PAREN`            | `Expected LeftParen, found <tok>`                    | Open argument/condition list with `(`.                            |
| E1013  | `PARSE_EXPECTED_BRACKET`          | `Expected LeftBracket, found <tok>`                  | Open index/list with `[`.                                         |
| E1014  | `PARSE_EXPECTED_COLON`            | `Expected Colon, found <tok>`                        | Add `:` between name and type, or case and body.                  |
| E1015  | `PARSE_EXPECTED_COMMA`            | `Expected Comma, found <tok>`                        | Separate items with `,`.                                          |
| E1016  | `PARSE_EXPECTED_EQUALS`            | `Expected Equals, found <tok>`                       | Use `=` for initialization or assignment.                         |
| E1017  | `PARSE_EXPECTED_WHILE`            | `Expected 'while' after do block, found <tok>`       | `do { ... } while (cond);` requires the trailing `while`.         |
| E1018  | `PARSE_EXPECTED_IN`               | `Expected 'in' in for loop, found <tok>`             | `for (x in items)` requires the `in` keyword.                     |
| E1019  | `PARSE_EXPECTED_CATCH_OR_FINALLY` | `Expected 'catch' or 'finally', found <tok>`         | `try { ... }` must be followed by `catch (e: T) { ... }`.         |
| E1020  | `PARSE_EXPECTED_CASE_OR_DEFAULT`  | `Expected 'case' or 'default', found <tok>`          | A `switch` body contains `case <pat> => ...` and an optional `default => ...`. |
| E1021  | `PARSE_INVALID_PATTERN`           | `Invalid pattern ...`                                | Patterns are literal, `_`, or `Constructor(bindings)`.            |
| E1999  | `PARSE_UNKNOWN`                   | (fallback for unmatched parser messages)             | Inspect the raw message; if it should be classified, extend `errors.rs`. |

The parser also enforces `MAX_RECURSION_DEPTH = 256`. Programs that
exceed this limit receive a structured `Maximum recursion depth 256
exceeded at L:C` error rather than crashing with a stack overflow.

---

## Semantic errors (`E2xxx`)

| Code   | Constant                          | Message                                                | Remediation                                                       |
|--------|-----------------------------------|--------------------------------------------------------|-------------------------------------------------------------------|
| E2000  | `SEM_UNDECLARED_IDENTIFIER`       | `undeclared identifier: 'name'`                        | Declare the name (`let`/`var`/`const`/`fn`/`class`) or import it. |
| E2001  | `SEM_USE_OF_MOVED_VARIABLE`       | `use of moved variable: 'name'`                        | Do not use a variable after its `Owned<T>` has been moved.        |
| E2002  | `SEM_DUPLICATE_DECLARATION`       | `duplicate declaration: 'name'`                        | Rename the duplicate or remove it.                                |
| E2003  | `SEM_TYPE_MISMATCH`               | `type mismatch in variable 'name': ...`                | Align the declared and assigned types.                            |
| E2004  | `SEM_RETURN_TYPE_MISMATCH`        | `return type mismatch: ...`                            | The returned value's type must match the function's return type.  |
| E2005  | `SEM_MISSING_RETURN`              | `function 'name' is missing a return statement`        | Add a `return expr;` on every path of a non-`void` function.      |
| E2006  | `SEM_CONDITION_NOT_BOOL`          | `condition must be bool, found <type>`                 | Use a `bool` expression in `if`/`while`/`do`/`for`/`?:`.          |
| E2007  | `SEM_FIELD_NOT_FOUND`             | `field 'name' not found on type <type>`                | Declare the field or use an existing field name.                  |
| E2008  | `SEM_METHOD_NOT_FOUND`            | `method 'name' not found on type <type>`               | Declare the method or use an existing method name.                |
| E2009  | `SEM_NOT_AN_ENUM_VARIANT`         | `<name> is not an enum variant`                        | Use a variant that belongs to the enum.                           |
| E2010  | `SEM_VARIANT_NOT_IN_ENUM`         | `variant <name> does not belong to enum <enum>`        | Use a variant declared in the enum.                               |
| E2011  | `SEM_TUPLE_ARITY_MISMATCH`        | `tuple destructuring expects N names, found M`         | Match the number of names to the tuple's arity.                   |
| E2012  | `SEM_TUPLE_NOT_TUPLE_TYPE`        | `tuple destructuring requires a tuple type`            | Use `let (a, b) = expr;` only with tuple-typed `expr`.            |
| E2999  | `SEM_UNKNOWN`                     | (fallback for unmatched analyzer messages)             | Inspect the raw message; if it should be classified, extend `errors.rs`. |

---

## Codegen errors (`E3xxx`)

| Code   | Constant          | Message                            | Remediation                                                  |
|--------|-------------------|------------------------------------|--------------------------------------------------------------|
| E3999  | `CODEGEN_UNKNOWN` | (fallback for codegen errors)      | Codegen errors are rare; inspect the message for specifics. |

---

## Diagnostic rendering

`trc::errors::render_error(code, message, file, line, col, source_line, span_len)`
produces the canonical diagnostic. Example output for E0001:

```text
error[E0001]: Unterminated string
  --> test.tr:1:10
   |
 1 | let s = "hello;
   |        ^^^^^^^
```

When `source_line` is `None`, the caret line is omitted. When `line` or
`col` is `None`, the `-->` line is omitted. When all three are `None`,
only the `error[E0XXX]: <message>` header is emitted.
