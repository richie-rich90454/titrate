# Inspect

The `tt.lang.Inspect` module mirrors Python's `inspect` module. It provides runtime introspection over functions, classes, modules, and the call stack: `getmembers`, `isfunction`, `isclass`, `ismethod`, `ismodule`, `signature`, `getsource`, `getfile`, `currentframe`, `stack`.

## Import

```titrate
import tt::lang::Inspect;
```

## Member

`Member` is a (name, value) pair returned by `getmembers`.

- `Member.init(name: string, value: Variant)`
- `name(): string`
- `value(): Variant`
- `toString(): string`

## Signature

`Signature` describes a function's parameter list and return type.

- `Signature.init(params: ArrayList<Parameter>, returnType: string)`
- `params(): ArrayList<Parameter>`
- `returnType(): string`
- `bind(args: ArrayList<Variant>): BoundArguments` — bind positional arguments to parameter slots
- `toString(): string` — `(a: int, b: int): int`

### Parameter

- `Parameter.init(name: string, kind: int, typeName: string, defaultValue: Variant)`
- `name(): string`
- `kind(): int` — one of `POSITIONAL_ONLY`, `POSITIONAL_OR_KEYWORD`, `VAR_POSITIONAL`, `KEYWORD_ONLY`, `VAR_KEYWORD`
- `typeName(): string`
- `defaultValue(): Variant`
- `hasDefault(): bool`

## SourceInfo

`SourceInfo` describes the source location of a declaration.

- `SourceInfo.init(file: string, line: int, column: int, endLine: int)`
- `file(): string`, `line(): int`, `column(): int`, `endLine(): int`
- `toString(): string`

## Functions

### getmembers

Return a list of `(name, value)` pairs for every public member of `obj`, sorted alphabetically by name.

**Parameters:** `obj: Variant`
**Returns:** `ArrayList<Member>`

```titrate
let m: ArrayList<Member> = getmembers(list);
var i: int = 0;
while (i < m.size()) {
    io::println(m.get(i).name());
    i = i + 1;
}
```

### isfunction

Return `true` if `obj` is a function (top-level or nested `fn` value).

**Parameters:** `obj: Variant`
**Returns:** `bool`

### isclass

Return `true` if `obj` is a class object (the metaclass, not an instance).

**Parameters:** `obj: Variant`
**Returns:** `bool`

### ismethod

Return `true` if `obj` is a bound method.

**Parameters:** `obj: Variant`
**Returns:** `bool`

### ismodule

Return `true` if `obj` is a module object.

**Parameters:** `obj: Variant`
**Returns:** `bool`

### signature

Return the `Signature` of `fn`.

**Parameters:** `fn: Variant`
**Returns:** `Signature`

```titrate
let sig: Signature = signature(Integer.parseInt);
io::println(sig.toString());  // "(s: string): int"
```

### getsource

Return the source code of `obj` as a string, if available. Returns the empty string if the source is not available.

**Parameters:** `obj: Variant`
**Returns:** `string`

### getfile

Return the file name of the source file that defines `obj`, or the empty string.

**Parameters:** `obj: Variant`
**Returns:** `string`

### getSourceInfo

Return the `SourceInfo` (file, line, column) for `obj`, or null.

**Parameters:** `obj: Variant`
**Returns:** `SourceInfo`

### currentframe

Return a `FrameInfo` representing the current execution frame.

**Returns:** `FrameInfo`

### stack

Return the call stack as a list of `FrameInfo`, with the current frame at the bottom.

**Returns:** `ArrayList<FrameInfo>`

### formatStack

Format the call stack as a list of human-readable strings, one per frame.

**Returns:** `ArrayList<string>`

```titrate
let frames: ArrayList<string> = formatStack();
var i: int = 0;
while (i < frames.size()) {
    io::println(frames.get(i));
    i = i + 1;
}
```

## FrameInfo

- `FrameInfo.init(filename: string, line: int, function: string, frame: Variant)`
- `filename(): string` — file name of the source
- `line(): int` — line number in the source
- `function(): string` — function name
- `frame(): Variant` — the underlying frame object (opaque)
- `toString(): string`
