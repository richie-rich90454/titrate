# Dis

The `tt.lang.Dis` module mirrors Python's `dis` module. It provides bytecode disassembly for Titrate functions: `dis`, `distb`, `disco`, `disassemble`, `codeInfo`, `showCode`, and the `Bytecode` class that exposes a list of `Instruction`s.

## Import

```titrate
import tt::lang::Dis;
```

## Instruction

`Instruction` represents a single Titrate VM opcode in a disassembled bytecode listing.

- `Instruction.init(opcode: int, opname: string, arg: int, argrepr: string, offset: int, startsLine: int, isJumpTarget: bool)`
- `opcode(): int` — numeric opcode
- `opname(): string` — human-readable opcode name
- `arg(): int` — raw argument value, or -1 if the opcode takes no argument
- `argrepr(): string` — formatted argument (e.g., the resolved name for `LOAD_NAME`)
- `offset(): int` — byte offset of the instruction in the code unit
- `startsLine(): int` — source line that emitted this instruction, or -1
- `isJumpTarget(): bool` — true if some `JUMP_*` instruction targets this offset
- `toString(): string` — formatted one-line disassembly

## Bytecode

`Bytecode` wraps the full instruction list for a function or code object.

- `Bytecode.init(instructions: ArrayList<Instruction>, code: Variant)`
- `instructions(): ArrayList<Instruction>`
- `code(): Variant` — the underlying code object
- `codeInfo(): string` — formatted summary (name, argcount, locals, stack size)

## Top-level functions

### opname

Return the human-readable name for an opcode number.

**Parameters:** `opcode: int`
**Returns:** `string`

### opmap

Return a `HashMap<string, int>` mapping opcode names to opcode numbers (loaded from `data/lang/opcodes.json`).

**Returns:** `HashMap<string, int>`

### opnameFor

Return the name of the opcode that produces the given instruction argument. Useful for jump targets and name lookups.

**Parameters:** `arg: int`
**Returns:** `string`

### opcodeFor

Inverse of `opname`. Return the numeric opcode for a given opcode name, or -1 if unknown.

**Parameters:** `name: string`
**Returns:** `int`

### dis

Disassemble `fn` and print the result to standard output.

**Parameters:** `fn: Variant`
**Returns:** `void`

```titrate
fn add(a: int, b: int): int { return a + b; }
dis(add);
//  1           0 LOAD_FAST         0 (a)
//              2 LOAD_FAST         1 (b)
//              4 BINARY_ADD
//              6 RETURN_VALUE
```

### distb

Disassemble the bytecode of the most recent traceback (i.e., the function that was running when the last exception was raised).

**Returns:** `void`

### disco

Disassemble the code object `code` and print the result.

**Parameters:** `code: Variant`
**Returns:** `void`

### disassemble

Like `dis` but returns the formatted listing as a string instead of printing it.

**Parameters:** `fn: Variant`
**Returns:** `string`

### codeInfo

Return a formatted string with metadata about a code object (name, argument count, local count, stack size, constants).

**Parameters:** `fn: Variant`
**Returns:** `string`

```titrate
io::println(codeInfo(add));
// Name:        add
// Argcount:    2
// Locals:      2
// Stack:       2
```

### showCode

Print `codeInfo(fn)` to standard output. Equivalent to `io::println(codeInfo(fn))`.

**Parameters:** `fn: Variant`
**Returns:** `void`

### buildBytecode

Construct a `Bytecode` object from a function. Loads the opcode metadata from `data/lang/opcodes.json` if not already cached.

**Parameters:** `fn: Variant`
**Returns:** `Bytecode`

```titrate
let bc: Bytecode = buildBytecode(add);
let instrs: ArrayList<Instruction> = bc.instructions();
var i: int = 0;
while (i < instrs.size()) {
    io::println(instrs.get(i).toString());
    i = i + 1;
}
```

## Opcode categories

Titrate opcodes (loaded from `data/lang/opcodes.json`) follow the same grouping as CPython:

- **Stack manipulation:** `LOAD_CONST`, `LOAD_FAST`, `LOAD_NAME`, `STORE_FAST`, `STORE_NAME`, `POP_TOP`, `DUP_TOP`, `ROT_TWO`, `ROT_THREE`
- **Operators:** `BINARY_ADD`, `BINARY_SUBTRACT`, `BINARY_MULTIPLY`, `BINARY_DIVIDE`, `BINARY_MODULO`, `BINARY_POWER`, `BINARY_AND`, `BINARY_OR`, `BINARY_XOR`, `BINARY_LSHIFT`, `BINARY_RSHIFT`
- **Control flow:** `JUMP_ABSOLUTE`, `JUMP_FORWARD`, `POP_JUMP_IF_FALSE`, `POP_JUMP_IF_TRUE`, `GET_ITER`, `FOR_ITER`
- **Calls:** `CALL_FUNCTION`, `CALL_METHOD`, `RETURN_VALUE`
- **Imports:** `IMPORT_NAME`, `IMPORT_FROM`, `LOAD_ATTR`

The exact numeric values are stable within a Titrate version but may change between versions; always use `opname`/`opcodeFor` instead of hard-coding numbers.
