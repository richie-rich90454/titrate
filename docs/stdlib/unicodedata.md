# unicodedata

The `tt.text` module provides `Unicodedata` — access to the Unicode Character Database for character classification, properties, and normalization.

```titrate
import tt.text.Unicodedata;
```

## Unicodedata

Static methods for querying Unicode character properties and performing normalization. All methods are called on the `Unicodedata` module directly.

- `Unicodedata.category(ch: string): string` — return the general category (e.g. `"Lu"` for uppercase letter, `"Nd"` for decimal digit)
- `Unicodedata.decimal(ch: string): int` — return the decimal digit value; error if not a decimal digit
- `Unicodedata.numeric(ch: string): double` — return the numeric value (e.g. 0.5 for the fraction ½)
- `Unicodedata.isLetter(ch: string): bool` — check if the character is a letter
- `Unicodedata.isDigit(ch: string): bool` — check if the character is a digit
- `Unicodedata.isSpace(ch: string): bool` — check if the character is whitespace
- `Unicodedata.isUpper(ch: string): bool` — check if the character is uppercase
- `Unicodedata.isLower(ch: string): bool` — check if the character is lowercase
- `Unicodedata.isTitle(ch: string): bool` — check if the character is titlecase
- `Unicodedata.normalize(form: string, s: string): string` — normalize a string to the given form (`"NFC"`, `"NFD"`, `"NFKC"`, `"NFKD"`)
- `Unicodedata.name(ch: string): string` — return the Unicode name of the character
- `Unicodedata.lookup(name: string): string` — return the character with the given Unicode name
- `Unicodedata.bidirectional(ch: string): string` — return the bidirectional category
- `Unicodedata.combining(ch: string): int` — return the canonical combining class (0 if not a combining character)
- `Unicodedata.mirrored(ch: string): bool` — check if the character is mirrored in bidirectional text

```titrate
io::println(Unicodedata.category("A"));         // "Lu"
io::println(Unicodedata.category("5"));         // "Nd"
io::println(Unicodedata.name("A"));             // "LATIN CAPITAL LETTER A"
io::println(Unicodedata.lookup("LATIN CAPITAL LETTER A")); // "A"

io::println(Boolean.toString(Unicodedata.isLetter("A")));  // true
io::println(Boolean.toString(Unicodedata.isDigit("5")));   // true
io::println(Boolean.toString(Unicodedata.isSpace(" ")));   // true
io::println(Boolean.toString(Unicodedata.isUpper("A")));   // true
io::println(Boolean.toString(Unicodedata.isLower("a")));   // true

io::println(Integer.toString(Unicodedata.decimal("7")));   // 7
io::println(Double.toString(Unicodedata.numeric("½")));    // 0.5

// Normalization
let nfc: string = Unicodedata.normalize("NFC", "cafe\u0301");
io::println(nfc);  // "café" in composed form

io::println(Integer.toString(Unicodedata.combining("\u0301"))); // 230 (combining acute accent)
io::println(Boolean.toString(Unicodedata.mirrored("(")));       // false
```
