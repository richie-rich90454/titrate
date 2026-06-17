# text

The `tt.textwrap`, `tt.string`, `tt.html`, and `tt.decimal` modules provide text processing utilities: wrapping and indentation, string constants and templates, HTML escaping, and arbitrary-precision decimal arithmetic.

```titrate
import tt.textwrap.Textwrap;
import tt.string.StringUtils;
import tt.string.Template;
import tt.html.Html;
import tt.decimal.Decimal;
```

## Textwrap

Text wrapping and filling utilities.

- `Textwrap.wrap(text: string, width: int): ArrayList<string>` — wrap text to the given width, returning a list of lines
- `Textwrap.fill(text: string, width: int): string` — wrap text and join lines with newlines
- `Textwrap.dedent(text: string): string` — remove common leading whitespace from all lines
- `Textwrap.indent(text: string, prefix: string): string` — add prefix to every non-empty line
- `Textwrap.shorten(text: string, width: int): string` — truncate text to at most `width` characters, adding `...` if truncated

```titrate
let lines = Textwrap.wrap("The quick brown fox jumps over the lazy dog", 15);
let filled = Textwrap.fill("hello world foo bar", 10);
let dedented = Textwrap.dedent("    line1\n    line2");
let shortened = Textwrap.shorten("A very long string here", 12);
```

## StringUtils

String constants and word-level utilities.

- `StringUtils.asciiLetters(): string` — `"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"`
- `StringUtils.asciiLowercase(): string` — `"abcdefghijklmnopqrstuvwxyz"`
- `StringUtils.asciiUppercase(): string` — `"ABCDEFGHIJKLMNOPQRSTUVWXYZ"`
- `StringUtils.digits(): string` — `"0123456789"`
- `StringUtils.hexdigits(): string` — `"0123456789abcdefABCDEF"`
- `StringUtils.octdigits(): string` — `"01234567"`
- `StringUtils.punctuation(): string` — ASCII punctuation characters
- `StringUtils.whitespace(): string` — whitespace characters
- `StringUtils.printable(): string` — digits + letters + punctuation + whitespace
- `StringUtils.capwords(s: string): string` — capitalize each word

```titrate
let letters = StringUtils.asciiLetters();
let digits = StringUtils.digits();
let title = StringUtils.capwords("hello world foo");
```

## Template

String substitution using `$variable` placeholders.

- `Template(template: string)` — create a template with `$var` and `${var}` placeholders
- `substitute(map: HashMap<string, string>): string` — substitute variables; missing keys are skipped
- `safeSubstitute(map: HashMap<string, string>): string` — substitute variables; missing keys are left as-is

```titrate
let t = new Template("Hello, $name! Welcome to ${place}.");
let vars = new HashMap<string, string>();
vars.put("name", "Alice");
vars.put("place", "Wonderland");
let result = t.substitute(vars);
```

## Html

HTML escaping and unescaping.

- `Html.escape(s: string): string` — replace `&`, `<`, `>`, `"`, `'` with HTML entities
- `Html.unescape(s: string): string` — replace HTML entities (named and numeric) with characters

```titrate
let safe = Html.escape("<script>alert('xss')</script>");
let raw = Html.unescape("&lt;div&gt;hello&lt;/div&gt;");
```

## Decimal

Arbitrary-precision decimal arithmetic using string-based representation.

- `Decimal(value: string)` — construct from string (e.g. `"3.14"`, `"-0.001"`)
- `Decimal.fromString(s: string): Decimal` — factory method
- `add(other: Decimal): Decimal` — addition
- `sub(other: Decimal): Decimal` — subtraction
- `mul(other: Decimal): Decimal` — multiplication
- `div(other: Decimal): Decimal` — division (20-digit precision)
- `compareTo(other: Decimal): int` — comparison (`-1`, `0`, `1`)
- `equals(other: Decimal): bool` — equality check
- `toString(): string` — convert to string representation
- `toDouble(): double` — convert to double

```titrate
let a = new Decimal("0.1");
let b = new Decimal("0.2");
let sum = a.add(b);
io::println(sum.toString());  // "0.3"
let product = a.mul(new Decimal("3.0"));
let cmp = a.compareTo(b);
```

## Deepened Difflib

- `Difflib.unifiedDiff(a: ArrayList<string>, b: ArrayList<string>, fromFile: string, toFile: string): ArrayList<string>` — unified diff
- `Difflib.contextDiff(a: ArrayList<string>, b: ArrayList<string>): ArrayList<string>` — context diff
- `Difflib.ndiff(a: ArrayList<string>, b: ArrayList<string>): ArrayList<string>` — delta in ndiff format
- `Difflib.restore(delta: ArrayList<string>, which: int): ArrayList<string>` — restore from delta

## Deepened Shlex

- `Shlex.split(s: string): ArrayList<string>` — shell-like splitting
- `Shlex.quote(s: string): string` — shell-escape a string
- `Shlex.join(splitCommand: ArrayList<string>): string` — join split command

## Deepened Unicodedata

- `Unicodedata.normalize(form: string, s: string): string` — Unicode normalization (NFC, NFD, NFKC, NFKD)
- `Unicodedata.category(c: string): string` — Unicode category
- `Unicodedata.bidirectional(c: string): string` — bidirectional class
- `Unicodedata.combining(c: string): int` — combining class value
- `Unicodedata.decomposition(c: string): string` — decomposition mapping
- `Unicodedata.mirrored(c: string): int` — mirrored property
- `Unicodedata.numeric(c: string): double` — numeric value
- Data loaded from `data/unicode/decomposition.json`, `data/unicode/combining_class.json`
