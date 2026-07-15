# Trace

The `tt.tooling.Trace` module mirrors Python's `trace` module. It traces statement execution counts: as Titrate runs the traced code, every executed source line is recorded; the result is a per-file/per-line counter that can be used for coverage analysis and debugging.

## Import

```titrate
import tt::tooling.Trace;
```

## Trace

`Trace` is the main class. Construct one, run code under it, and inspect the per-line execution counts.

### Construction

- `Trace.init(count: bool, trace: bool, countDirs: ArrayList<string>)`
  - `count` — whether to record per-line execution counts
  - `trace` — whether to print each line as it executes
  - `countDirs` — list of source directories whose `.tr` files should be counted

### Running code

#### run

Execute `cmd` (a source string) in the global namespace.

**Parameters:** `cmd: string`
**Returns:** `void`

```titrate
let t: Trace = new Trace(true, false, new ArrayList<string>());
t.run("let x = 1 + 2; io::println(x);");
```

#### runctx

Execute `cmd` with the given `globals` and `locals` maps.

**Parameters:** `cmd: string`, `globals: HashMap<string, Variant>`, `locals: HashMap<string, Variant>`
**Returns:** `void`

#### runfunc

Call `fn` with the given `args` under the tracer.

**Parameters:** `fn: Variant`, `args: ArrayList<Variant>`
**Returns:** `Variant` (the function's return value)

### Results

#### results

Return a `Coverage` object containing the recorded per-line counts.

**Returns:** `Coverage`

#### callerResults

Return a `CallerCoverage` object that additionally records which function called each line.

**Returns:** `CallerCoverage`

#### clear

Reset all recorded counts.

**Returns:** `void`

### Output

#### formatResults

Format the coverage results as a human-readable string.

**Parameters:** `coverage: Coverage` (optional, defaults to `results()`)
**Returns:** `string`

#### writeResultsFile

Write the coverage results to `cover.tr`-style output at `path`.

**Parameters:** `path: string`, `coverage: Coverage` (optional)
**Returns:** `void`

## Coverage

`Coverage` records per-file per-line execution counts.

- `Coverage.init()`
- `record(filename: string, line: int): void` — increment the count for `(filename, line)`
- `count(filename: string, line: int): int` — the number of times `line` was executed
- `lines(filename: string): ArrayList<int>` — sorted list of executed line numbers
- `files(): ArrayList<string>` — sorted list of files that were traced
- `merge(other: Coverage): void` — merge another coverage's counts into this one
- `toString(): string`

## CallerCoverage

`CallerCoverage` extends `Coverage` with caller tracking.

- `CallerCoverage.init()`
- `callers(filename: string, line: int): HashMap<string, int>` — map from "caller:line" to call count
- `recordCall(filename: string, line: int, caller: string): void`

## Command-line behaviour

When run as a script (`tr lib/tt/tooling/Trace.tr --count --cover demo.tr`), the module mirrors the `python -m trace` CLI:

- `--count` — write execution counts to `cover.tr`
- `--trace` — print each line as it executes
- `--listfuncs` — list all functions that were called
- `--trackcalls` — record caller relationships
- `--file=OUT` — write results to `OUT` instead of `cover.tr`

```titrate
let t: Trace = new Trace(true, true, new ArrayList<string>());
t.run("fn fib(n: int): int { if (n < 2) { return n; } return fib(n-1) + fib(n-2); } fib(5);");
io::println(t.formatResults());
```
