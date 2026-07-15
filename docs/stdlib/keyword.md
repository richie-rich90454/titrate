# Keyword

The `tt.lang.Keyword` module mirrors Python's `keyword` module. It exposes Titrate's keyword list (loaded from `data/lang/keywords.json`) and helper functions to test whether a string is a (hard or soft) keyword.

## Import

```titrate
import tt::lang::Keyword;
```

## Functions

### iskeyword

Return `true` if `name` is a Titrate reserved keyword that cannot be used as an identifier (e.g., `fn`, `if`, `while`, `return`).

**Parameters:** `name: string`
**Returns:** `bool`

```titrate
io::println(Boolean.toString(iskeyword("fn")));  // true
io::println(Boolean.toString(iskeyword("foo")));  // false
```

### issoftkeyword

Return `true` if `name` is a soft keyword — contextually reserved but usable as an identifier in most positions (e.g., `type`, `where`, `as`).

**Parameters:** `name: string`
**Returns:** `bool`

### kwlist

Return the sorted list of all hard keywords.

**Returns:** `ArrayList<string>`

```titrate
let kws: ArrayList<string> = kwlist();
io::println(Integer.toString(kws.size()));  // 30+
```

### softkwlist

Return the sorted list of all soft keywords.

**Returns:** `ArrayList<string>`

### keywordCount

Return the number of hard keywords.

**Returns:** `int`

### softKeywordCount

Return the number of soft keywords.

**Returns:** `int`

## Hard keywords

The hard keyword list (loaded from `data/lang/keywords.json`) is the set of tokens that the parser reserves unconditionally:

`class`, `interface`, `enum`, `extends`, `implements`, `super`, `this`, `self`, `new`, `public`, `private`, `fn`, `let`, `var`, `const`, `import`, `module`, `if`, `else`, `while`, `for`, `do`, `switch`, `case`, `default`, `return`, `break`, `continue`, `with`, `where`, `try`, `catch`, `finally`, `throw`, `as`, `is`, `type`, `in`, `null`, `true`, `false`, `void`, `bool`, `byte`, `short`, `int`, `long`, `vast`, `uvast`, `float`, `double`, `half`, `quad`, `char`, `string`, `size`, `u8`, `u16`, `u32`, `u64`, `Result`, `Ok`, `Err`, `Optional`, `Owned`, `region`, `unsafe`

The exact list is loaded from `data/lang/keywords.json` so that new keywords added between releases are reflected without code changes.

## Soft keywords

Soft keywords are reserved only in specific syntactic positions; in most contexts they are still valid identifiers:

`type`, `as`, `is`, `where`, `with`, `match`, `static`

## Notes

- The lists are loaded from `data/lang/keywords.json` at module load. Adding a keyword to that file makes `iskeyword` return `true` for the new keyword without recompiling the module.
- Hard keywords cannot be used as identifiers anywhere. Soft keywords can be used as identifiers in most positions but are reserved in specific grammatical contexts.
- `keywordCount` and `softKeywordCount` are convenience functions whose return values may change between Titrate versions.
