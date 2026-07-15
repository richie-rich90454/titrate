# XdrLib

The `tt.binary.XdrLib` module serializes and deserializes data using the External Data Representation (XDR) standard (RFC 4504). It mirrors Python's `xdrlib` module, exposing `Packer` (accumulates encoded bytes), `Unpacker` (reads from a buffer with a position cursor), and the dispatch helpers `pack_data` / `unpack_data`. All multi-byte values are big-endian; strings and byte arrays are padded to a multiple of 4 bytes with NUL bytes.

## Import

```titrate
import tt::binary::XdrLib;
```

## Classes

### Packer

Accumulates encoded XDR bytes.

**Fields:**
- `buffer: string` — accumulated packed bytes

**Constructors:**
- `init()` — creates an empty packer

**Methods:**
- `pack_int(n: int): void` — append a signed 32-bit integer (RFC 4504 §4.1)
- `pack_uint(n: long): void` — append an unsigned 32-bit integer (accepts `long` so values >= 2^31 are valid)
- `pack_float(f: double): void` — append an IEEE 754 single-precision float (§4.6)
- `pack_double(d: double): void` — append an IEEE 754 double-precision float (§4.7)
- `pack_string(s: string): void` — append a length-prefixed string padded to a multiple of 4 bytes (§4.11)
- `pack_bytes(b: string): void` — append a length-prefixed byte array padded to a multiple of 4 bytes (§4.10)
- `get_buffer(): string` — return the accumulated packed bytes
- `reset(): void` — clear the buffer

```titrate
let p: Packer = new Packer();
p.pack_int(42);
p.pack_string("hello");
let encoded: string = p.get_buffer();
```

### Unpacker

Reads XDR bytes from a buffer with a position cursor.

**Fields:**
- `data: string` — underlying buffer
- `position: int` — current read cursor

**Constructors:**
- `init(data: string)`

**Methods:**
- `unpack_int(): int` — read a signed 32-bit integer
- `unpack_uint(): long` — read an unsigned 32-bit integer as a `long`
- `unpack_float(): double` — read an IEEE 754 single-precision float
- `unpack_double(): double` — read an IEEE 754 double-precision float
- `unpack_string(): string` — read a length-prefixed string and skip padding
- `unpack_bytes(): string` — read a length-prefixed byte array and skip padding
- `get_position(): int` — return the current read cursor
- `set_position(pos: int): void` — set the read cursor
- `get_buffer(): string` — return the underlying data buffer

```titrate
let u: Unpacker = new Unpacker(encoded);
let n: int = u.unpack_int();
let s: string = u.unpack_string();
```

## Functions

### pack_data

- `XdrLib.pack_data(data: ArrayList<Pair<string, Variant>>, packer: Packer): void` — pack a list of `(type_name, value)` pairs. `type_name` is one of `"int"`, `"uint"`, `"float"`, `"double"`, `"string"`, `"bytes"`. Throws on unknown type names.

### unpack_data

- `XdrLib.unpack_data(data: ArrayList<string>, unpacker: Unpacker): ArrayList<Variant>` — unpack a list of values whose types are given by `data` (a list of type-name strings). Returns the unpacked values as `Variant`s.

```titrate
let schema = new ArrayList<string>();
schema.add("int"); schema.add("string");
let values: ArrayList<Variant> = XdrLib.unpack_data(schema, unpacker);
```

## Usage Example

```titrate
import tt::binary::XdrLib;
import tt::util::Pair;

public fn main(): void {
    let p: Packer = new Packer();
    p.pack_int(7);
    p.pack_double(3.14);
    p.pack_string("hello");
    let encoded: string = p.get_buffer();
    let u: Unpacker = new Unpacker(encoded);
    io::println(Integer.toString(u.unpack_int()));       // 7
    io::println(Double.toString(u.unpack_double()));     // 3.14
    io::println(u.unpack_string());                       // "hello"
}
```
