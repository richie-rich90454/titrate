# Ctypes

The `tt.ffi.Ctypes` module provides a Python `ctypes` analog for the foreign function interface: `CDLL`, `WinDLL`, `Structure`, `Union`, `Pointer`, `ByRef`, `Func`, and the predefined `c_*` C type descriptors (`c_int`, `c_char_p`, `c_void_p`, `c_double`, `c_bool`, `c_long`, …). It is backed by `titrate_native` VM primitives (`Ctypes_dlopen`, `Ctypes_dlsym`, `Ctypes_call`) when available, with in-process emulation otherwise.

## Import

```titrate
import tt::ffi::Ctypes;
```

## Predefined C Type Constants

Each `c_*` is a `CType` constant describing a C scalar.

- `c_bool` (1 byte, `int`)
- `c_byte`, `c_ubyte` (1 byte)
- `c_char` (1 byte, `char`), `c_char_p` (8 bytes, `ptr`)
- `c_short`, `c_ushort` (2 bytes)
- `c_int`, `c_uint` (4 bytes)
- `c_long`, `c_ulong`, `c_longlong`, `c_ulonglong` (8 bytes)
- `c_float` (4 bytes, `float`), `c_double` (8 bytes, `float`)
- `c_void_p` (8 bytes, `ptr`)

## Classes

### CType

A C type descriptor carrying its name, size, and kind.

**Fields:** `name: string`, `size: int`, `kind: string` (`"int"`, `"uint"`, `"char"`, `"ptr"`, `"float"`)

**Constructor:** `CType(name: string, size: int, kind: string)`

### CData

A value paired with its `CType`. Mirrors `ctypes` instances.

**Fields:** `type: CType`, `value: Variant`

**Methods:** `asInt(): int`, `asDouble(): double`, `asString(): string`

### Field

A `(name, type)` pair describing a `Structure` or `Union` member.

**Constructor:** `Field(name: string, type: CType)`

### Structure

Base class for C struct analogs. Subclasses declare `_fields_` as a list of `Field` instances; the layout is computed with C-style alignment.

**Methods:**
- `_setFields(fields: ArrayList<Field>): void` — declare the fields (call from subclass constructor)
- `setField(name: string, value: Variant): void`
- `getField(name: string): Variant` — `null` if unset
- `sizeOf(): int` — total size in bytes of the laid-out structure
- `toBytes(): string` — serialize the structure to a byte string

### Union

All members share offset 0; size is the max member size.

**Methods:** same shape as `Structure` (`_setFields`, `setField`, `getField`, `sizeOf`).

### Pointer / ByRef

- `Pointer(target: CType, address: int)` — a pointer wraps an address and a target `CType`. Methods: `isNull(): bool`, `asInt(): int`.
- `ByRef(target: Structure)` — a lightweight reference used to pass a `Structure` by address.

### CDLL / WinDLL

A loaded shared library.

**Methods:**
- `init(name: string)` — load via `Ctypes_dlopen`
- `get(name: string): Func` — look up a symbol; returns a `Func` wrapper
- `callName(name: string, args: ArrayList<Variant>): Variant` — convenience: look up and call
- `close(): void` — close the library handle

`WinDLL` extends `CDLL` and sets `useStdcall = true` for Windows stdcall convention.

### Func

A resolved symbol from a shared library.

**Methods:**
- `init(library: CDLL, name: string, address: int)`
- `setArgTypes(types: ArrayList<CType>): void`
- `setReturnType(type: CType): void` (default `c_int`)
- `call(args: ArrayList<Variant>): Variant` — invoke the foreign function (throws `AttributeError` if undefined)

## Functions

### CDLL_load

Load a shared library by name or path.

**Parameters:** `name: string`
**Returns:** `CDLL`

### POINTER

Construct a `Pointer` type for the given target type.

**Parameters:** `target: CType`
**Returns:** `Pointer`

### pointer

Construct a pointer to an address.

**Parameters:** `target: CType`, `address: int`
**Returns:** `Pointer`

### byref

Pass a `Structure` by reference.

**Parameters:** `target: Structure`
**Returns:** `ByRef`

### sizeof

Return the size in bytes of a `CType`, `Structure`, or `Union`.

**Parameters:** `obj: Variant`
**Returns:** `int`

### alignment

Return the alignment requirement of a `CType`.

**Parameters:** `obj: Variant`
**Returns:** `int`

### cast

Cast a pointer/integer to a different type.

**Parameters:** `obj: Variant`, `targetType: CType`
**Returns:** `CData`

```titrate
let lib: CDLL = CDLL_load("libc.so.6");
let getTime: Func = lib.get("time");
getTime.setReturnType(c_long);
let args = new ArrayList<Variant>();
let t: Variant = getTime.call(args);
```
