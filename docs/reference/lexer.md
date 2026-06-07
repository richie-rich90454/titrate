# Lexer Tokens

## Keywords

| Token | Lexeme |
|-------|--------|
| `Public` | `public` |
| `Private` | `private` |
| `Fn` | `fn` |
| `Class` | `class` |
| `Interface` | `interface` |
| `Enum` | `enum` |
| `Extends` | `extends` |
| `Implements` | `implements` |
| `Let` | `let` |
| `Var` | `var` |
| `Const` | `const` |
| `If` | `if` |
| `Else` | `else` |
| `While` | `while` |
| `For` | `for` |
| `Return` | `return` |
| `Break` | `break` |
| `Continue` | `continue` |
| `Switch` | `switch` |
| `Case` | `case` |
| `Default` | `default` |
| `True` | `true` |
| `False` | `false` |
| `Null` | `null` |
| `New` | `new` |
| `This` | `this` |
| `Super` | `super` |
| `Result` | `Result` |
| `Ok` | `Ok` |
| `Err` | `Err` |
| `Owned` | `Owned` |
| `Region` | `region` |
| `Unsafe` | `unsafe` |
| `As` | `as` |
| `Is` | `is` |
| `Type` | `type` |
| `Import` | `import` |
| `Module` | `module` |

## Type Keywords

`void`, `bool`, `byte`, `short`, `int`, `long`, `vast`, `uvast`, `float`, `double`, `half`, `quad`, `char`, `string`, `size`, `u8`, `u16`, `u32`, `u64`

## Operators

`+`, `-`, `*`, `/`, `%`, `=`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||`, `!`, `&`, `|`, `^`, `~`, `<<`, `>>`, `::`, `=>`, `?`, `.`, `,`, `;`, `:`, `(`, `)`, `{`, `}`, `[`, `]`, `&mut`

## Literals

- Integer: `42`, `0xFF`, `0o77`, `0b1010`
- Float: `3.14`, `1.5h` (half), `2.0q` (quad)
- String: `"hello"`
- Char: `'a'`
- Bool: `true`, `false`
- Null: `null`

## Comments

- Line: `// comment`
- Block: `/* comment */`
