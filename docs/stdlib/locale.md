# locale

The `tt.i18n` module provides `Locale` — locale-aware number and currency formatting.

```titrate
import tt.i18n.Locale;
```

## Locale

Represents a locale for formatting numbers and currency values according to regional conventions.

- `fn init(name: string)` — create a Locale from a locale identifier (e.g. `"en_US"`, `"de_DE"`, `"ja_JP"`)
- `formatNumber(value: double, decimals: int): string` — format a number with the locale's grouping separator and decimal separator
- `formatCurrency(value: double): string` — format a value as currency using the locale's conventions
- `toString(): string` — return the locale identifier string

```titrate
let us: Locale = new Locale("en_US");
io::println(us.formatNumber(1234567.89, 2));  // "1,234,567.89"
io::println(us.formatCurrency(42.5));          // "$42.50"

let de: Locale = new Locale("de_DE");
io::println(de.formatNumber(1234567.89, 2));  // "1.234.567,89"
io::println(de.formatCurrency(42.5));          // "42,50 €"

let jp: Locale = new Locale("ja_JP");
io::println(jp.formatNumber(1234567, 0));     // "1,234,567"
```

## Free Functions

- `defaultLocale(): Locale` — return the system's default locale

```titrate
let loc: Locale = defaultLocale();
io::println(loc.toString());  // e.g. "en_US"
io::println(loc.formatNumber(1000.5, 2));
```
