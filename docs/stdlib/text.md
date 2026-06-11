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

- `Textwrap.wrap(text: String, width: int): ArrayList<String>` — wrap text to the given width, returning a list of lines
- `Textwrap.fill(text: String, width: int): String` — wrap text and join lines with newlines
- `Textwrap.dedent(text: String): String` — remove common leading whitespace from all lines
- `Textwrap.indent(text: String, prefix: String): String` — add prefix to every non-empty line
- `Textwrap.shorten(text: String, width: int): String` — truncate text to at most `width` characters, adding `...` if truncated

```titrate
let lines = Textwrap::wrap("The quick brown fox jumps over the lazy dog", 15);
let filled = Textwrap::fill("hello world foo bar", 10);
let dedented = Textwrap::dedent("    line1\n    line2");
let shortened = Textwrap::shorten("A very long string here", 12);
```

## StringUtils

String constants and word-level utilities.

- `StringUtils.asciiLetters(): String` — `"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"`
- `StringUtils.asciiLowercase(): String` — `"abcdefghijklmnopqrstuvwxyz"`
- `StringUtils.asciiUppercase(): String` — `"ABCDEFGHIJKLMNOPQRSTUVWXYZ"`
- `StringUtils.digits(): String` — `"0123456789"`
- `StringUtils.hexdigits(): String` — `"0123456789abcdefABCDEF"`
- `StringUtils.octdigits(): String` — `"01234567"`
- `StringUtils.punctuation(): String` — ASCII punctuation characters
- `StringUtils.whitespace(): String` — whitespace characters
- `StringUtils.printable(): String` — digits + letters + punctuation + whitespace
- `StringUtils.capwords(s: String): String` — capitalize each word

```titrate
let letters = StringUtils::asciiLetters();
let digits = StringUtils::digits();
let title = StringUtils::capwords("hello world foo");
```

## Template

String substitution using `$variable` placeholders.

- `Template(template: String)` — create a template with `$var` and `${var}` placeholders
- `substitute(map: HashMap<String, String>): String` — substitute variables; missing keys are skipped
- `safeSubstitute(map: HashMap<String, String>): String` — substitute variables; missing keys are left as-is

```titrate
let t = new Template("Hello, $name! Welcome to ${place}.");
let vars = new HashMap<String, String>();
vars.put("name", "Alice");
vars.put("place", "Wonderland");
let result = t.substitute(vars);
```

## Html

HTML escaping and unescaping.

- `Html.escape(s: String): String` — replace `&`, `<`, `>`, `"`, `'` with HTML entities
- `Html.unescape(s: String): String` — replace HTML entities (named and numeric) with characters

```titrate
let safe = Html::escape("<script>alert('xss')</script>");
let raw = Html::unescape("&lt;div&gt;hello&lt;/div&gt;");
```

## Decimal

Arbitrary-precision decimal arithmetic using string-based representation.

- `Decimal(value: String)` — construct from string (e.g. `"3.14"`, `"-0.001"`)
- `Decimal.fromString(s: String): Decimal` — factory method
- `add(other: Decimal): Decimal` — addition
- `sub(other: Decimal): Decimal` — subtraction
- `mul(other: Decimal): Decimal` — multiplication
- `div(other: Decimal): Decimal` — division (20-digit precision)
- `compareTo(other: Decimal): int` — comparison (`-1`, `0`, `1`)
- `equals(other: Decimal): bool` — equality check
- `toString(): String` — convert to string representation
- `toDouble(): double` — convert to double

```titrate
let a = new Decimal("0.1");
let b = new Decimal("0.2");
let sum = a.add(b);
io::println(sum.toString());  // "0.3"
let product = a.mul(new Decimal("3.0"));
let cmp = a.compareTo(b);
```
