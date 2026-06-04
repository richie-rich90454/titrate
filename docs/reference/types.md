# Types

## Primitive Types

| Type | Size | Description |
|------|------|-------------|
| `void` | 0 | No value |
| `bool` | 1 | Boolean |
| `byte` | 8 | Signed 8-bit integer |
| `short` | 16 | Signed 16-bit integer |
| `int` | 32 | Signed 32-bit integer |
| `long` | 64 | Signed 64-bit integer |
| `vast` | 128 | Signed 128-bit integer |
| `uvast` | 128 | Unsigned 128-bit integer |
| `float` | 32 | 32-bit IEEE 754 |
| `double` | 64 | 64-bit IEEE 754 |
| `half` | 16 | 16-bit float (simulated) |
| `quad` | 128 | 128-bit float (simulated) |
| `char` | 32 | Unicode scalar |
| `string` | — | UTF-8 string |
| `size` | ptr | Pointer-sized unsigned |

## Composite Types

- `Owned<T>` — heap-allocated, move-semantics
- `Result<T, E>` — success or error
- `array<T>` — fixed-size array
- Class instances
- Enum instances

## Type Parameters

```titrate
ArrayList<int>
HashMap<string, double>
```
