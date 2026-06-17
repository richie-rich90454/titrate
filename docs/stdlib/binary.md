# binary

The `tt.binary` module provides binary data packing and unpacking using format strings, similar to Python's `struct` module. Struct allows you to encode values into binary strings and decode binary strings back into values.

```titrate
import tt::binary::Struct;
```

## Struct

Binary data pack/unpack with format strings.

- `Struct.pack(format: string, values: ArrayList<Variant>): string` — pack values into binary string
- `Struct.unpack(format: string, data: string): ArrayList<Variant>` — unpack binary string into values
- `Struct.calcSize(format: string): int` — calculate size of packed data
- `Struct.iterUnpack(format: string, data: string): ArrayList<ArrayList<Variant>>` — iteratively unpack

### Format Characters

| Character | Type | Size |
|-----------|------|------|
| `b` | signed byte | 1 |
| `B` | unsigned byte | 1 |
| `h` | signed short | 2 |
| `H` | unsigned short | 2 |
| `i` | signed int | 4 |
| `I` | unsigned int | 4 |
| `q` | signed long | 8 |
| `Q` | unsigned long | 8 |
| `f` | float | 4 |
| `d` | double | 8 |
| `s` | string | variable |
| `?` | bool | 1 |

### Byte Order Prefixes

| Prefix | Byte Order |
|--------|------------|
| `<` | little-endian |
| `>` | big-endian |
| `=` | native |
| `!` | network / big-endian |

```titrate
let values: ArrayList<Variant> = new ArrayList<Variant>();
values.add(42 as Variant);
values.add(3 as Variant);
let packed: string = Struct.pack("<ih", values);
let size: int = Struct.calcSize("<ih");
let unpacked: ArrayList<Variant> = Struct.unpack("<ih", packed);
let iter: ArrayList<ArrayList<Variant>> = Struct.iterUnpack("<ih", packed);
```

## All Format Characters

- `Struct.formatChar(type: string): string` — get format character for type
- Supported: b/B (byte), h/H (short), i/I (int), l/L (long), f (float), d (double), s (string), ? (bool)

## Byte Order Prefixes

- `Struct.nativeOrder(): string` — native byte order prefix (@)
- `Struct.littleEndian(): string` — little-endian prefix (<)
- `Struct.bigEndian(): string` — big-endian prefix (>)
- `Struct.networkOrder(): string` — network byte order prefix (!)

## Advanced Operations

- `Struct.packInto(format: string, buffer: ArrayList<byte>, offset: int, values: ArrayList<Variant>): void` — pack into buffer at offset
- `Struct.unpackFrom(format: string, buffer: ArrayList<byte>, offset: int): ArrayList<Variant>` — unpack from buffer at offset
- `Struct.iterUnpack(format: string, buffer: ArrayList<byte>): ArrayList<ArrayList<Variant>>` — iterate unpack
- `Struct.calcsize(format: string): int` — calculate size of struct

## Struct Class

- `Struct.init(format: string)` — create reusable Struct instance
- `Struct.pack(values: ArrayList<Variant>): ArrayList<byte>` — pack values
- `Struct.unpack(buffer: ArrayList<byte>): ArrayList<Variant>` — unpack buffer
- `Struct.size(): int` — struct size
