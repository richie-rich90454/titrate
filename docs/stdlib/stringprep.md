# StringPrep

The `tt.text.StringPrep` module implements RFC 3454 stringprep tables for Unicode code-point classification and a basic `prepare()` profile. It mirrors Python's `stringprep` module, exposing table predicates (`in_table_a1` ... `in_table_d2`), map functions (`map_b1`, `map_b2`, `map_b3`, `map_c11`, `map_c12`), and a `prepare()` profile that applies the mapping steps (B.1 -> B.2 -> B.3) then rejects any prohibited code points (C.1.2 through C.9). Tables are loaded from `text/stringprep_tables.json` via `DataFile` on first use.

## Import

```titrate
import tt::text::StringPrep;
```

## Table Predicates

Each predicate returns `true` when the given code point is in the named RFC 3454 table. Tables are loaded lazily from the bundled data file.

- `StringPrep.in_table_a1(code: int): bool` — table A.1 (unassigned code points)
- `StringPrep.in_table_b1(code: int): bool` — table B.1 (commonly mapped to nothing)
- `StringPrep.in_table_b2(code: int): bool` — table B.2 (case folding for chars with decomposition)
- `StringPrep.in_table_b3(code: int): bool` — table B.3 (case folding for chars without decomposition)
- `StringPrep.in_table_c11(code: int): bool` — table C.1.1 (ASCII control characters)
- `StringPrep.in_table_c12(code: int): bool` — table C.1.2 (non-ASCII control characters)
- `StringPrep.in_table_c21(code: int): bool` — table C.2.1 (ASCII spaces)
- `StringPrep.in_table_c22(code: int): bool` — table C.2.2 (non-ASCII spaces)
- `StringPrep.in_table_c3(code: int): bool` — table C.3 (private use)
- `StringPrep.in_table_c4(code: int): bool` — table C.4 (non-character code points)
- `StringPrep.in_table_c5(code: int): bool` — table C.5 (surrogate)
- `StringPrep.in_table_c6(code: int): bool` — table C.6 (inappropriate for plain text)
- `StringPrep.in_table_c7(code: int): bool` — table C.7 (inappropriate for canonical representation)
- `StringPrep.in_table_c8(code: int): bool` — table C.8 (change display properties)
- `StringPrep.in_table_c9(code: int): bool` — table C.9 (tagging characters)
- `StringPrep.in_table_d1(code: int): bool` — table D.1 ( bidi R or AL)
- `StringPrep.in_table_d2(code: int): bool` — table D.2 (bidi L)

```titrate
let isControl: bool = StringPrep.in_table_c11(0x0000);
```

## Map Functions

- `StringPrep.map_b1(code: int): string` — B.1 maps to nothing; returns `""` if in B.1, otherwise the original character
- `StringPrep.map_c11(code: int): string` — C.1.1 has no mapping table; returns the original character
- `StringPrep.map_c12(code: int): string` — C.1.2 has no mapping table; returns the original character
- `StringPrep.map_b2(code: int): string` — B.2 case folding; returns the mapped string or the original character
- `StringPrep.map_b3(code: int): string` — B.3 case folding; returns the mapped string or the original character

```titrate
let folded: string = StringPrep.map_b3(0x0041);  // 'A' -> 'a'
```

## prepare

- `StringPrep.prepare(s: string): string` — apply the RFC 3454 mapping steps (B.1 -> B.2 -> B.3), then reject any code point in the prohibited tables (C.1.2, C.2.1, C.2.2, C.3, C.4, C.5, C.6, C.7, C.8, C.9). NFKC normalization and Bidi checks are skipped for simplicity. Throws `"stringprep: prohibited code point U+XXXX"` if a prohibited character is encountered.

```titrate
let prepared: string = StringPrep.prepare("Hello");
```

## Usage Example

```titrate
import tt::text::StringPrep;

public fn main(): void {
    let input: string = "Hello, World!";
    try {
        let prepared: string = StringPrep.prepare(input);
        io::println("Prepared: " + prepared);
    } catch (e: string) {
        io::println("Rejected: " + e);
    }
}
```
