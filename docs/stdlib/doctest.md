# Doctest

The `tt.test.Doctest` module mirrors Python's `doctest` module. It extracts interactive examples from docstrings and runs them as tests, comparing the actual output against the expected output. Examples have the CPython form:

```
>>> fib(10)
55
```

## Import

```titrate
import tt::test::Doctest;
```

## Constants — option flags

- `ELLIPSIS: int = 1` — `...` matches any substring of the actual output
- `NORMALIZE_WHITESPACE: int = 2` — collapse runs of whitespace
- `IGNORE_EXCEPTION_DETAIL: int = 4` — ignore exception-type details in traceback matches
- `DONT_ACCEPT_BLANKLINE: int = 8` — disable `<BLANKLINE>` marker
- `REPORT_UDIFF: int = 16` — show unified diffs on failure
- `REPORT_CDIFF: int = 32` — show context diffs
- `REPORT_NDIFF: int = 64` — show ndiff (word-level)
- `REPORT_ONLY_FIRST_FAILURE: int = 128`

## Example

`Example` is a single `>>>` block from a docstring.

- `Example.init(source: string, expected: string, options: int, line: int)`
- `source(): string` — the input line(s)
- `expected(): string` — expected output (after blank lines)
- `options(): int` — option flags specific to this example
- `line(): int` — source line number

## DocTest

`DocTest` is a collection of examples from one docstring.

- `DocTest.init(name: string, filename: string, examples: ArrayList<Example>)`
- `name(): string` — function/class/module name
- `filename(): string`
- `examples(): ArrayList<Example>`
- `size(): int` — number of examples

## TestResult

`TestResult` summarizes the outcome of running a `DocTest`.

- `TestResult.init(name: string, attempted: int, failed: int)`
- `name(): string`
- `attempted(): int` — examples that were run
- `failed(): int` — examples that did not match
- `passed(): int` — `attempted - failed`
- `isSuccess(): bool` — `failed == 0`
- `toString(): string`

## DocTestRunner

`DocTestRunner` runs `DocTest`s and accumulates results.

- `DocTestRunner.init(verbose: bool, optionflags: int)`
- `run(test: DocTest): TestResult`
- `summarize(): (int, int)` — `(attempted, failed)` totals across all runs
- `reset(): void`

## Functions

### register_optionflag

Register a new option-flag name and return its bitmask. Useful for custom extensions.

**Parameters:** `name: string`
**Returns:** `int`

### findExamples

Extract `Example`s from a docstring.

**Parameters:** `docstring: string`, `name: string`, `filename: string`
**Returns:** `ArrayList<Example>`

```titrate
let exs: ArrayList<Example> = findExamples(
    "Compute a sum.\n>>> sum(1, 2)\n3\n",
    "sum",
    "demo.tr"
);
io::println(Integer.toString(exs.size()));  // 1
```

### testmod

Run all doctests in the current module and return the total results.

**Parameters:** `mod: Variant` (optional, defaults to the calling module), `verbose: bool` (optional, default `false`)
**Returns:** `(int, int)` — `(attempted, failed)`

### testfile

Read `path`, extract doctests, run them, and return the totals.

**Parameters:** `path: string`, `verbose: bool` (optional)
**Returns:** `(int, int)`

```titrate
let (a, f): (int, int) = testfile("docs/examples.tr");
if (f > 0) {
    io::println("Doctest failures: " + Integer.toString(f));
}
```

### runExample

Run a single `Example` and return its output as a string. Returns `null` if the example raised an exception.

**Parameters:** `example: Example`, `globals: HashMap<string, Variant>`
**Returns:** `string`

### run_docstring_examples

Run every example in `docstring` against the given `globals`, ignoring failures (this is useful for interactive use).

**Parameters:** `docstring: string`, `globals: HashMap<string, Variant>`, `name: string`, `verbose: bool`
**Returns:** `void`

### normalizeOutput

Apply the given option flags to normalize an output string before comparison.

**Parameters:** `s: string`, `options: int`
**Returns:** `string`

```titrate
let actual: string = "1   2   3";
let expected: string = "1 2 3";
io::println(Boolean.toString(normalizeOutput(actual, NORMALIZE_WHITESPACE) == expected));  // true
```

## Output comparison

Comparison follows CPython's rules:

1. If `ELLIPSIS` is set, every `...` token in the expected output matches any substring.
2. If `NORMALIZE_WHITESPACE` is set, all runs of whitespace in both strings are collapsed to single spaces and leading/trailing whitespace is stripped.
3. The expected output ends with a trailing newline if and only if the actual output does. The trailing newline is implicit in the docstring form.
4. If the source raises an exception, the expected output is matched as a `Traceback` block. `IGNORE_EXCEPTION_DETAIL` makes the comparison accept any exception type with the same message.

## Notes

- Examples are extracted only from string literals that are the first statement of a function/class/module (i.e., the docstring). Other string literals are not scanned.
- A docstring may contain multiple examples; each starts with a line beginning `>>> ` and continues with `... ` for multi-line inputs.
- When run via `testmod()`, the function being documented must be importable from the current module.
