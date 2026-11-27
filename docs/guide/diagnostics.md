# Diagnostics Output Formats

`trc` reports compiler errors in one of two formats: human-readable **text**
(the default) and machine-readable **JSON lines**. The format is selected with
the `--diagnostics-format` flag.

```
trc <file.tr> [--diagnostics-format=json|text] [other options]
```

| Value | Behavior |
|-------|----------|
| `text` | Default. Canonical `error[E0XXX]: <message>` output with source location and caret. |
| `json` | One JSON value per line, written to stderr. No source excerpt or caret. |

The flag only changes how errors from the lexer, parser, and analyzer are
rendered. It does not change which diagnostics are produced, exit codes, or
any other output (module-resolution, native-backend, and runtime errors, plus
warnings, remain plain text).

## JSON lines

With `--diagnostics-format=json`, every failing compile phase writes a single
line to stderr:

- Lexer and parser errors emit one JSON **object** per line.
- Analyzer (semantic) errors emit one JSON **array** of objects per line, so a
  program with several semantic errors still produces exactly one line.

Successful compiles and program output are unchanged. A clean run writes no
diagnostic lines.

## Schema

Each diagnostic object has a fixed key order:

```json
{"code":"E2000","message":"undeclared identifier: 'missingName'","line":1,"column":32,"suggestions":[]}
```

| Key | Type | Description |
|-----|------|-------------|
| `code` | string | Stable error code from the catalog in `trc/src/errors.rs` (`E0xxx` lexer, `E1xxx` parser, `E2xxx` semantic). |
| `message` | string | Primary message with any trailing ` at L:C` suffix removed. |
| `line` | number or `null` | 1-based source line, or `null` when unknown. |
| `column` | number or `null` | 1-based source column, or `null` when unknown. |
| `suggestions` | array | Zero or more suggested fixes (see below). |

A suggestion object has the shape:

```json
{"message":"a similar name exists in scope","replacement":"missingName"}
```

| Key | Type | Description |
|-----|------|-------------|
| `message` | string | Human-readable hint (the text after `help:` in text mode). |
| `replacement` | string or `null` | Machine-applicable replacement text, or `null` when the hint has no exact edit. |

JSON strings use standard escaping (`\"`, `\\`, `\n`, `\r`, `\t`, `\b`, `\f`,
and `\u00xx` for other control characters). Non-ASCII characters are emitted
verbatim as UTF-8.

## Examples

Lexer error:

```console
$ trc bad.tr --diagnostics-format=json
{"code":"E0001","message":"Unterminated string","line":1,"column":9,"suggestions":[]}
```

Parser error:

```console
$ trc bad.tr --diagnostics-format=json
{"code":"E1000","message":"Expected ;, found EOF","line":1,"column":11,"suggestions":[]}
```

Semantic errors (one array line, even for multiple errors):

```console
$ trc bad.tr --diagnostics-format=json
[{"code":"E2000","message":"undeclared identifier: 'missingName'","line":1,"column":32,"suggestions":[]}]
```

The same errors in the default text format:

```console
$ trc bad.tr
error[E2000]: undeclared identifier: 'missingName'
  --> bad.tr:1:32
```
