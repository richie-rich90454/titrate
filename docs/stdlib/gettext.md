# Gettext

The `tt.i18n.Locale` module provides GNU gettext-style internationalization. It mirrors Python's `gettext` module via the `GettextCatalog` class (which parses `.mo` files and performs message lookups) and the module-level `bindtextdomain`/`textdomain`/`gettext`/`dgettext`/`ngettext`/`dngettext` functions. Plural-form selection follows the catalog's recorded plural count; without a translation, `n == 1` selects the singular and otherwise the plural.

## Import

```titrate
import tt::i18n::Locale;
```

## Classes

### GettextCatalog

A GNU `.mo` catalog: maps `msgid` to `msgstr`, plus optional plural-form translations.

**Fields:**
- `translations: HashMap<string, string>` — singular-message map
- `pluralTranslations: HashMap<string, ArrayList<string>>` — plural-message map (msgid to list of plural forms)
- `charset: string` — catalog charset (default `"utf-8"`)
- `domain: string` — catalog domain name
- `path: string` — file path the catalog was loaded from

**Constructors:**
- `init(domain: string)`

**Methods:**
- `gettext(msgid: string): string` — look up a singular message; returns `msgid` if no translation exists
- `ngettext(singular: string, plural: string, n: int): string` — look up a plural message; `n` selects which plural form to use. Falls back to `singular` (if `n == 1`) or `plural` (otherwise) when no translation exists.
- `loadFile(path: string): bool` — load this catalog from a `.mo` file at `path`. Returns `true` on success.

```titrate
let cat: GettextCatalog = new GettextCatalog("myapp");
cat.loadFile("locale/fr/LC_MESSAGES/myapp.mo");
io::println(cat.gettext("Hello"));  // French translation, or "Hello"
```

## Functions

### bindtextdomain

- `Locale.bindtextdomain(domain: string, dir: string): string` — bind `domain` to the directory containing its `.mo` catalogs. Returns `dir`. Invalidates any cached catalog so the next lookup reloads it.

### textdomain

- `Locale.textdomain(domain: string): string` — set the current text domain. Returns the previously active domain.

### getTextDomain

- `Locale.getTextDomain(): string` — return the current text domain (default `"messages"`).

### gettext

- `Locale.gettext(msgid: string): string` — look up `msgid` in the current domain. Returns `msgid` if no translation exists.

```titrate
Locale.bindtextdomain("myapp", "locale");
Locale.textdomain("myapp");
io::println(Locale.gettext("Hello"));
```

### ngettext

- `Locale.ngettext(singular: string, plural: string, n: int): string` — look up a plural message in the current domain. Falls back to `singular` (if `n == 1`) or `plural` (otherwise) when no translation exists.

### dgettext

- `Locale.dgettext(domain: string, msgid: string): string` — look up `msgid` in the specified `domain`. Returns `msgid` if no translation exists.

### dngettext

- `Locale.dngettext(domain: string, singular: string, plural: string, n: int): string` — look up a plural message in the specified `domain`.

### bind_textdomain_codeset

- `Locale.bind_textdomain_codeset(domain: string, codeset: string): string` — bind a codeset to a domain. Returns `codeset`. (No-op stub for API parity.)

## Usage Example

```titrate
import tt::i18n::Locale;

public fn main(): void {
    Locale.bindtextdomain("myapp", "locale");
    Locale.textdomain("myapp");
    io::println(Locale.gettext("Hello, world!"));
    let count: int = 3;
    let msg: string = Locale.ngettext("You have 1 message", "You have %d messages", count);
    io::println(msg);
}
```
