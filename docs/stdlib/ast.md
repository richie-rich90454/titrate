# Ast

The `tt.lang.Ast` module mirrors Python's `ast` module. It exposes the Titrate AST as a hierarchy of `AstNode` subclasses, plus `parse`, `walk`, `dump`, `unparse`, `literal_eval`, and `get_docstring` helpers. The AST is read-only; modifying a node's fields does not affect the source.

## Import

```titrate
import tt::lang::Ast;
```

## Top-level functions

### parse

Parse `source` (a Titrate source string) into an `AstModule`. Raises `SyntaxError` on parse failure.

**Parameters:** `source: string`, `filename: string` (optional, defaults to `"<string>"`)
**Returns:** `AstModule`

```titrate
let mod: AstModule = parse("fn add(a: int, b: int): int { return a + b; }");
io::println(mod.body().size());  // 1
```

### walk

Yield every node in the tree rooted at `node` in depth-first pre-order.

**Parameters:** `node: AstNode`
**Returns:** `ArrayList<AstNode>`

### dump

Return a JSON-ish representation of the tree rooted at `node`, suitable for debugging.

**Parameters:** `node: AstNode`, `indent: int` (optional, default 0)
**Returns:** `string`

```titrate
let mod: AstModule = parse("let x = 1 + 2;");
io::println(dump(mod));
// Module:
//   body: [
//     VarDecl(name=x, type=, value=BinaryOp(op=+, left=Literal(1), right=Literal(2)))
//   ]
```

### unparse

Reconstruct a source-code string from an AST node. The output is normalized and may not match the input text character-for-character.

**Parameters:** `node: AstNode`
**Returns:** `string`

### literal_eval

Evaluate a literal expression (integer, float, string, boolean, null, list, dict, tuple) into the corresponding runtime value. Raises `ValueError` if the node is not a literal.

**Parameters:** `node: AstNode`
**Returns:** `Variant`

```titrate
let v: Variant = literal_eval(parseExpr("[1, 2, 3]"));
io::println(v.toString());  // [1, 2, 3]
```

### get_docstring

If `node` is a module, function, or class whose first statement is a string literal expression, return that string. Otherwise return the empty string.

**Parameters:** `node: AstNode`
**Returns:** `string`

```titrate
let mod: AstModule = parse("\"A sample module.\"\nfn f(): void {}");
io::println(get_docstring(mod));  // "A sample module."
```

## AstNode

The base class for all AST nodes. Concrete subclasses are listed below.

### Common fields

- `kind: string` — the node kind (`"Module"`, `"FunctionDecl"`, `"If"`, etc.)
- `location: SourceLocation` — source span (file, line, column) of the node

### Common methods

- `getKind(): string`
- `getChildren(): ArrayList<AstNode>` — direct children of the node
- `accept(visitor: fn(AstNode): void): void` — call `visitor` on this node and recurse

## Concrete node types

### Module
- `AstModule.init(body: ArrayList<AstStmt>)`
- `body(): ArrayList<AstStmt>`

### FunctionDecl
- `AstFunctionDecl.init(name: string, params: ArrayList<AstParam>, returnType: AstTypeRef, body: AstBlock)`
- `name(): string`, `params(): ArrayList<AstParam>`, `returnType(): AstTypeRef`, `body(): AstBlock`

### ClassDecl
- `AstClassDecl.init(name: string, baseClasses: ArrayList<AstTypeRef>, members: ArrayList<AstNode>)`
- `name()`, `baseClasses()`, `members()`

### VarDecl
- `AstVarDecl.init(name: string, typeRef: AstTypeRef, value: AstExpr)`
- `name()`, `typeRef()`, `value()`

### Statements
- `AstStmt` — base for statements
- `AstBlock.init(stmts: ArrayList<AstStmt>)` — `{ ... }` block
- `AstIf.init(cond: AstExpr, thenBody: AstStmt, elseBody: AstStmt)`
- `AstWhile.init(cond: AstExpr, body: AstStmt)`
- `AstFor.init(varName: string, iter: AstExpr, body: AstStmt)`
- `AstReturn.init(value: AstExpr)`
- `AstExprStmt.init(expr: AstExpr)` — expression used as a statement

### Expressions
- `AstExpr` — base for expressions
- `AstBinaryOp.init(op: string, left: AstExpr, right: AstExpr)` — operators like `+`, `==`, `&&`
- `AstUnaryOp.init(op: string, operand: AstExpr)` — prefix operators like `!`, `-`
- `AstLiteral.init(value: Variant)` — integer/float/string/boolean/null literal
- `AstIdentifier.init(name: string)` — variable or function name
- `AstCall.init(callee: AstExpr, args: ArrayList<AstExpr>)` — function call
- `AstMemberAccess.init(obj: AstExpr, member: string)` — `obj.field`
- `AstIndexAccess.init(obj: AstExpr, index: AstExpr)` — `obj[index]`

```titrate
let src: string = "fn fib(n: int): int { if (n < 2) { return n; } return fib(n-1) + fib(n-2); }";
let mod: AstModule = parse(src);
let nodes: ArrayList<AstNode> = walk(mod);
io::println(Integer.toString(nodes.size()));  // total node count
```

## Helper types

### AstParam
- `AstParam.init(name: string, typeRef: AstTypeRef)`
- `name(): string`, `typeRef(): AstTypeRef`

### AstTypeRef
- `AstTypeRef.init(name: string, args: ArrayList<AstTypeRef>)` — type with optional generic arguments
- `name(): string`, `args(): ArrayList<AstTypeRef>`

### SourceLocation
- `SourceLocation.init(filename: string, line: int, column: int)`
- `filename(): string`, `line(): int`, `column(): int`
