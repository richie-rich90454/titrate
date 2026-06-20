# warnings

The `tt.sys.Warnings` module provides warning filtering and emission. Warnings are categorized (e.g. `Warning`, `UserWarning`, `DeprecationWarning`) and an ordered list of filters determines the action taken for each category: `warn` (print), `always` (always print), `ignore` (suppress), or `error` (print as an error).

```titrate
import tt.sys.Warnings;
```

## Constants

Warning category string constants:

- `WARNING: string` — `"Warning"`
- `USER_WARNING: string` — `"UserWarning"`
- `DEPRECATION_WARNING: string` — `"DeprecationWarning"`

## Top-level Functions

- `fn warn(message: string, category: string): void` — issue a warning message for the given category, applying the currently active filter action
- `fn filterWarnings(action: string, category: string): void` — append a filter mapping the category to an action (`warn`, `always`, `ignore`, or `error`)
- `fn simplefilter(action: string, category: string): void` — prepend a filter so it takes priority over previously registered filters; an empty category matches all warnings
- `fn resetFilters(): void` — remove all warning filters and restore the default action
- `fn getFilters(): ArrayList<HashMap<string, string>>` — return the current list of filters, each a map with `"action"` and `"category"` keys

```titrate
import tt.sys.Warnings;

Warnings.simplefilter("ignore", Warnings.USER_WARNING);
Warnings.filterWarnings("error", Warnings.DEPRECATION_WARNING);
Warnings.warn("This feature is deprecated", Warnings.DEPRECATION_WARNING);
// Error [DeprecationWarning]: This feature is deprecated
```
