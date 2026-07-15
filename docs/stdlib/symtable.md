# SymTable

The `tt.lang.SymTable` module mirrors Python's `symtable` module. It exposes the symbol table that the Titrate compiler builds during name resolution — the set of names used and defined in each scope, along with their usage flags (read, written, parameter, free, cell, etc.).

## Import

```titrate
import tt::lang::SymTable;
```

## Symbol

`Symbol` describes a single name in a scope.

### Construction

`Symbol` instances are produced by the compiler; user code does not usually construct them.

### Fields & methods

- `name(): string` — the identifier
- `scope(): int` — one of `SCOPE_LOCAL`, `SCOPE_GLOBAL_EXPLICIT`, `SCOPE_GLOBAL_IMPLICIT`, `SCOPE_FREE`, `SCOPE_CELL`, `SCOPE_UNKNOWN`
- `isReferenced(): bool` — true if the name is read
- `isAssigned(): bool` — true if the name is written
- `isParameter(): bool` — true if the name is a function parameter
- `isFree(): bool` — true if the name is a free variable (defined in an enclosing scope)
- `isCell(): bool` — true if the name is captured by a nested function (becomes a closure cell)
- `isImported(): bool` — true if the name comes from an `import` statement
- `isWildcardImport(): bool` — true if the name comes from `import module::*`
- `toString(): string`

## SymbolTable

`SymbolTable` is the root of a scope tree.

### Construction

- `SymbolTable.init(name: string, type: int)` — type is one of `TYPE_MODULE`, `TYPE_FUNCTION`, `TYPE_CLASS`, `TYPE_LAMBDA`, `TYPE_COMPREHENSION`

### Methods

- `name(): string` — name of the scope (e.g., `"top"`, the function name)
- `type(): int` — kind of the scope
- `symbols(): ArrayList<Symbol>` — symbols defined in this scope
- `lookup(name: string): Symbol` — find a symbol by name, or null
- `children(): ArrayList<SymbolTable>` — nested scopes
- `parent(): SymbolTable` — the enclosing scope, or null for the module-level table
- `isNested(): bool` — true if this scope is nested inside another function
- `isOptimized(): bool` — true if the compiler can use local-variable fast slots
- `hasVarargs(): bool` — true if the function takes a varargs parameter
- `hasKwargs(): bool` — true if the function takes a kwargs parameter
- `lineno(): int` — source line of the scope's declaration
- `toString(): string`

```titrate
let st: SymbolTable = symtable("fn f(a: int): int { return a + 1; }", "test.tr");
let top: SymbolTable = st.children().get(0);
io::println(top.name());  // "f"
let s: Symbol = top.lookup("a");
io::println(Boolean.toString(s.isParameter()));  // true
```

## Top-level functions

### symtable

Parse `source` and return the root `SymbolTable` (a `TYPE_MODULE` table whose `children()` are the top-level functions and classes).

**Parameters:** `source: string`, `filename: string`
**Returns:** `SymbolTable`

### dump

Return a human-readable string representation of the entire symbol-table tree rooted at `table`.

**Parameters:** `table: SymbolTable`
**Returns:** `string`

```titrate
let st: SymbolTable = symtable("let x = 1; fn f(): void { x = 2; }", "demo.tr");
io::println(dump(st));
```

## Scope type constants

- `TYPE_MODULE: int = 0`
- `TYPE_FUNCTION: int = 1`
- `TYPE_CLASS: int = 2`
- `TYPE_LAMBDA: int = 3`
- `TYPE_COMPREHENSION: int = 4`

## Scope kind constants

- `SCOPE_LOCAL: int = 0`
- `SCOPE_GLOBAL_EXPLICIT: int = 1` — declared by an explicit `global` statement
- `SCOPE_GLOBAL_IMPLICIT: int = 2` — implicit module-global
- `SCOPE_FREE: int = 3` — bound in an enclosing function
- `SCOPE_CELL: int = 4` — captured by an inner function
- `SCOPE_UNKNOWN: int = 5`
