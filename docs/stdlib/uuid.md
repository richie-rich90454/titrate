# uuid

The `tt.uuid` module provides UUID generation and validation utilities.

```titrate
import tt.uuid.Uuid;
```

## Uuid

All methods are static.

- `uuid4(): string` — generate a version 4 (random) UUID in the format `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx`, where `y` is one of `8`, `9`, `a`, `b`
- `random(): string` — generate a simple random UUID (32 hex characters with dashes, no version bits set)
- `isValid(uuid: string): bool` — validate whether a string is a properly formatted UUID (36 characters, dashes at positions 8, 13, 18, 23, hex elsewhere)

```titrate
let id = Uuid.uuid4();
io::println(id);               // e.g. "a1b2c3d4-e5f6-4a7b-8c9d-0e1f2a3b4c5d"
io::println(Boolean.toString(Uuid.isValid(id))); // true

let simple = Uuid.random();
io::println(Boolean.toString(Uuid.isValid(simple))); // true
```
