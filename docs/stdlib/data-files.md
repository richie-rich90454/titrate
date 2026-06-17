# data-files

The `tt.lang` module provides the `DataFile` utility for loading, caching, and managing reference data files stored under `lib/tt/data/`. This infrastructure enforces a separation between code and data, ensuring that `.tr` source files do not become cluttered with large literal lookup tables.

```titrate
import tt.lang.DataFile;
```

## DataFile

A data file loader that reads JSON, CSV/TSV, and plain text files from the `lib/tt/data/` directory tree. Loaded data is cached in a `HashMap` so that repeated calls for the same path return the cached result without re-reading the file.

- `DataFile.load(path: string): JsonValue` — load a JSON file from `lib/tt/data/`, parse it, and cache the result. Returns the parsed `JsonValue`. Subsequent calls for the same path return the cached value.
- `DataFile.loadCsv(path: string): ArrayList<ArrayList<string>>` — load a CSV or TSV file. Returns a two-dimensional array of strings (rows × columns). Automatically detects comma vs tab delimiter.
- `DataFile.loadText(path: string): string` — load a raw text file and return its contents as a string.
- `DataFile.unload(path: string): void` — remove a file from the cache, freeing memory.
- `DataFile.reload(path: string): JsonValue` — force re-read a file from disk, updating the cache. Returns the freshly parsed value.
- `DataFile.dataDir(): string` — return the absolute path to the `lib/tt/data/` directory.
- `DataFile.exists(path: string): bool` — check whether a data file exists without loading it.
- `DataFile.list(module: string): ArrayList<string>` — list all data files within a module subdirectory (e.g., `"chem"` returns files under `lib/tt/data/chem/`).
- `DataFile.meta(path: string): JsonValue` — return the `_meta` object from a JSON data file. Equivalent to `DataFile.load(path).get("_meta")` but does not load the full file into the working cache.
- `DataFile.validate(path: string, schemaPath: string): bool` — validate a data file against a JSON Schema file. Both paths are relative to `lib/tt/data/`.

```titrate
// Load a JSON data file
let elements = DataFile.load("chem/elements.json");
let hydrogen = elements.get(0);
io::println(hydrogen.get("symbol").asString()); // "H"

// Load a CSV file
let rows = DataFile.loadCsv("locale/countries.csv");
for (row in rows) {
    io::println(row.get(0) + ": " + row.get(1));
}

// Load a plain text file
let stopwords = DataFile.loadText("nlp/stopwords_en.txt");

// Check existence before loading
if (DataFile.exists("unicode/blocks.json")) {
    let blocks = DataFile.load("unicode/blocks.json");
}
```

### Caching Behavior

All `load` calls cache their results. Use `reload` to force a fresh read:

```titrate
let data1 = DataFile.load("math/constants.json"); // reads from disk
let data2 = DataFile.load("math/constants.json"); // returns cached value
let data3 = DataFile.reload("math/constants.json"); // re-reads from disk

DataFile.unload("math/constants.json"); // remove from cache
```

### Listing Available Files

```titrate
let chemFiles = DataFile.list("chem");
for (f in chemFiles) {
    io::println(f);
}
// elements.json
// isotopes.json
// ...

let allModules = DataFile.list("");
for (m in allModules) {
    io::println(m);
}
```

### Validation Against JSON Schema

```titrate
let valid = DataFile.validate("chem/elements.json", "chem/schemas/element_schema.json");
if (!valid) {
    io::println("Data file failed schema validation");
}
```

## Data File Convention

All reference data files reside under `lib/tt/data/<module>/` and are loaded using a path relative to the data directory:

```
lib/tt/data/
├── chem/
│   ├── elements.json
│   ├── isotopes.json
│   └── ...
├── unicode/
│   ├── blocks.json
│   ├── categories.json
│   └── ...
├── html/
│   └── entities.json
├── datetime/
│   └── timezones.json
├── locale/
│   ├── countries.csv
│   └── languages.json
├── math/
│   └── constants.json
├── physics/
│   └── constants.json
├── encoding/
│   └── codepages.json
├── uuid/
│   └── namespaces.json
├── crypto/
│   └── curves.json
├── lang/
│   └── keywords.json
├── color/
│   └── named_colors.json
├── bio/
│   └── amino_acids.json
├── nlp/
│   ├── stopwords_en.txt
│   └── stopwords_zh.txt
├── hft/
│   └── exchanges.json
├── materials/
│   └── properties.json
└── units/
    └── definitions.json
```

Load a file by its module-relative path:

```titrate
let elements = DataFile.load("chem/elements.json");
let constants = DataFile.load("physics/constants.json");
let colors = DataFile.load("color/named_colors.json");
```

## _meta Convention

Every JSON data file **must** include a `_meta` object as a top-level key. This object provides provenance and versioning information for the data:

```titrate
{
    "_meta": {
        "source": "NIST CODATA 2022",
        "version": "1.3.0",
        "description": "Fundamental physical constants"
    },
    "items": [
        {"name": "speed of light", "value": 299792458, "unit": "m/s"},
        {"name": "Planck constant", "value": 6.62607015e-34, "unit": "J·s"}
    ]
}
```

The `_meta` object must contain:

| Field | Type | Description |
|-------|------|-------------|
| `source` | `string` | Origin or authority for the data (e.g., `"NIST"`, `"Unicode 15.0"`, `"IANA"`) |
| `version` | `string` | Semantic version of the data file (not the library version) |
| `description` | `string` | Brief human-readable description of the file's contents |

Access the `_meta` object:

```titrate
let meta = DataFile.meta("physics/constants.json");
io::println("Source: " + meta.get("source").asString());
io::println("Version: " + meta.get("version").asString());
```

## 5-Literal Rule

No `.tr` source file shall contain more than **5 literal numeric or string values** used as reference data. When a module needs lookup tables, constant maps, or enumerated data sets, the data must be externalized to a file under `lib/tt/data/` and loaded via `DataFile`.

**Violations of the 5-literal rule:**

```titrate
// WRONG — too many literal reference values in source code
let ELEMENT_SYMBOLS = ["H", "He", "Li", "Be", "B", "C", "N", "O", "F", "Ne"];
let ATOMIC_NUMBERS = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
```

**Correct approach — externalize to a data file:**

```titrate
// In lib/tt/data/chem/elements.json:
// {"_meta": {...}, "items": [{"symbol": "H", "number": 1}, ...]}

// In source code:
let elements = DataFile.load("chem/elements.json");
let symbol = elements.get(0).get("symbol").asString(); // "H"
```

The rule applies specifically to **reference data** — values that serve as lookup tables, constant maps, or enumerated datasets. It does not apply to:

- Single configuration values (e.g., `const MAX_RETRIES: int = 3`)
- Error message strings
- Format strings
- Test assertions
- Enum-like class field declarations

## Available Data Files

The following data directories and their contents are available:

### chem

Chemical element data, isotope tables, and periodic table information.

| File | Format | Description |
|------|--------|-------------|
| `elements.json` | JSON | Chemical elements with symbol, name, atomic number, mass, category |
| `isotopes.json` | JSON | Isotope data with mass numbers and abundances |

### unicode

Unicode character database subsets.

| File | Format | Description |
|------|--------|-------------|
| `blocks.json` | JSON | Unicode block ranges and names |
| `categories.json` | JSON | General category mappings |

### html

HTML entity definitions.

| File | Format | Description |
|------|--------|-------------|
| `entities.json` | JSON | Named HTML entities and their Unicode code points |

### datetime

Date and time reference data.

| File | Format | Description |
|------|--------|-------------|
| `timezones.json` | JSON | IANA timezone identifiers and UTC offsets |

### locale

Localization data for internationalization.

| File | Format | Description |
|------|--------|-------------|
| `countries.csv` | CSV | ISO 3166 country codes and names |
| `languages.json` | JSON | ISO 639 language codes and names |

### math

Mathematical constants and tables.

| File | Format | Description |
|------|--------|-------------|
| `constants.json` | JSON | Mathematical constants (π, e, φ, etc.) |

### physics

Physical constants and measurement data.

| File | Format | Description |
|------|--------|-------------|
| `constants.json` | JSON | Fundamental physical constants (CODATA values) |

### encoding

Character encoding mappings.

| File | Format | Description |
|------|--------|-------------|
| `codepages.json` | JSON | Legacy code page mappings |

### uuid

UUID namespace definitions.

| File | Format | Description |
|------|--------|-------------|
| `namespaces.json` | JSON | Well-known UUID namespaces (DNS, URL, OID, X.500) |

### crypto

Cryptographic algorithm parameters.

| File | Format | Description |
|------|--------|-------------|
| `curves.json` | JSON | Elliptic curve parameters and identifiers |

### lang

Language keyword and syntax reference data.

| File | Format | Description |
|------|--------|-------------|
| `keywords.json` | JSON | Reserved keywords and their categories |

### color

Color name and value definitions.

| File | Format | Description |
|------|--------|-------------|
| `named_colors.json` | JSON | CSS/HTML named colors with RGB values |

### bio

Biological sequence and taxonomy data.

| File | Format | Description |
|------|--------|-------------|
| `amino_acids.json` | JSON | Amino acid codes, names, and properties |

### nlp

Natural language processing reference data.

| File | Format | Description |
|------|--------|-------------|
| `stopwords_en.txt` | Text | English stop words (one per line) |
| `stopwords_zh.txt` | Text | Chinese stop words (one per line) |

### hft

High-frequency trading reference data.

| File | Format | Description |
|------|--------|-------------|
| `exchanges.json` | JSON | Exchange identifiers, MIC codes, and trading hours |

### materials

Material science property data.

| File | Format | Description |
|------|--------|-------------|
| `properties.json` | JSON | Material properties (density, conductivity, etc.) |

### units

Unit of measure definitions and conversions.

| File | Format | Description |
|------|--------|-------------|
| `definitions.json` | JSON | Unit definitions, symbols, and conversion factors |
