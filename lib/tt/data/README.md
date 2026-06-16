# Titrate Stdlib Data Files

This directory contains external data files used by the Titrate standard library.
All data is loaded at runtime via `DataFile.load()`, `DataFile.loadCsv()`, or `DataFile.loadText()`.

## Convention

- All data files are organized by module: `data/<module>/<file>.json`
- Data is loaded via `DataFile.load("<module>/<file>.json")`
- No .tr file shall contain more than 5 hardcoded literal values used as reference data
- All reference data (lookup tables, name mappings, constants) must be in external data files

## Required _meta Object

Every JSON data file MUST include a `_meta` object with the following fields:

```json
{
  "_meta": {
    "source": "CODATA 2018",
    "version": "2018",
    "description": "Fundamental physical constants"
  },
  "data": { ... }
}
```

- `source`: The authoritative source of the data (e.g., "CODATA 2018", "Unicode 15.0", "IUPAC 2021")
- `version`: The version or edition of the source data
- `description`: A brief human-readable description of the data file

## Directory Structure

```
data/
  bio/           - Bioinformatics data (scoring matrices, codon tables, restriction enzymes)
  chem/          - Chemistry data (periodic table, orbital exponents, force field params)
  color/         - Color data (named colors, luminance coefficients)
  crypto/        - Cryptography data (hash algorithm mappings)
  datetime/      - Date/time data (timezones, holidays)
  encoding/      - Encoding data (Base64 alphabets)
  hft/           - HFT data (FIX dictionaries)
  html/          - HTML data (entity references)
  lang/          - Language data (error codes)
  locale/        - Locale data (CLDR, month names)
  math/          - Math data (Bessel coefficients, Lanczos coefficients, erf coefficients)
  materials/     - Materials science data (space groups, scattering factors)
  nlp/           - NLP data (stop words, sentiment lexicons)
  physics/       - Physics data (constants)
  schemas/       - JSON Schema files for data validation
  unicode/       - Unicode data (decomposition, composition, combining classes)
  units/         - Unit conversion data
  uuid/          - UUID data (namespace UUIDs)
```

## Validation

Data files can be validated against their schemas using:
```
DataFile.validate("chem/periodic_table.json", "schemas/periodic_table_schema.json")
```
