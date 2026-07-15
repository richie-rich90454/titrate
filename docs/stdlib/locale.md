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

## Locale facets and global state (C++ `<locale>` parity, Phase 1-2)

### locale::global / locale::classic

- `Locale.global(loc: Locale): Locale` — set the global locale and return the previous one (`std::locale::global`)
- `Locale.classic(): Locale` — return the classic "C" locale (`std::locale::classic`)

```titrate
let prev: Locale = Locale.global(new Locale("de_DE"));
// ... do locale-sensitive work ...
Locale.global(prev);  // restore
let c: Locale = Locale.classic();  // the "C" locale
```

### Facets

A facet encapsulates one category of locale-dependent behavior. Each facet is named after its C++ `<locale>` counterpart.

- `Ctype` — character classification (`isalpha`, `isdigit`, `tolower`, `toupper`) for a locale
- `NumPut` — format numbers for output according to locale rules
- `NumGet` — parse numbers from input according to locale rules
- `TimePut` — format dates and times for a locale
- `TimeGet` — parse dates and times for a locale
- `MoneyPut` — format currency for output
- `MoneyGet` — parse currency from input
- `Messages` — message catalog lookup (gettext-style)
- `Collate` — locale-aware string comparison and hashing
- `Codecvt` — character-set conversion between encodings (e.g. UTF-8 ↔ UTF-16)

```titrate
let loc: Locale = new Locale("de_DE");
let ctype: Ctype = loc.getFacet<Ctype>("ctype");
io::println(Boolean.toString(ctype.isalpha('ä')));  // true

let coll: Collate = loc.getFacet<Collate>("collate");
let cmp: int = coll.compare("straße", "strasse");  // locale-dependent ordering

let msgs: Messages = loc.getFacet<Messages>("messages");
io::println(msgs.get("greeting", "default"));  // catalog lookup or "default"
```

Facets are retrieved via `Locale.getFacet<T>(name: string): T`. The available facet names are `"ctype"`, `"num_put"`, `"num_get"`, `"time_put"`, `"time_get"`, `"money_put"`, `"money_get"`, `"messages"`, `"collate"`, and `"codecvt"`.
