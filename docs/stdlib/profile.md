# Profile

The `tt.tooling.Profile` module mirrors Python's `profile` module. It provides a deterministic profiler that records per-function call counts, total time, and cumulative time. The output mirrors `profile.Profile` and `pstats.Stats` from CPython.

## Import

```titrate
import tt::tooling.Profile;
```

## ProfileStats

`ProfileStats` aggregates the per-function timings collected by a `Profile`.

- `ProfileStats.init()`
- `recordCall(funcName: string, totalTime: double, cumulativeTime: double): void`
- `functionCount(funcName: string): int` — number of times `funcName` was called
- `totalTime(funcName: string): double` — total time spent in `funcName` (microseconds)
- `cumulativeTime(funcName: string): double` — cumulative time including callees (microseconds)
- `functionNames(): ArrayList<string>` — sorted list of all called functions
- `merge(other: ProfileStats): void` — merge another stats object into this one
- `toString(): string`

## Profile

`Profile` is the main profiler class.

### Construction

- `Profile.init(timer: string)` — `timer` is `"wall"` (default) or `"cpu"`
- `Profile.init()` — equivalent to `new Profile("wall")`

### Lifecycle

- `enable(): void` — start collecting timings
- `disable(): void` — stop collecting timings
- `clear(): void` — reset all recorded statistics

### Running code

#### run

Execute `cmd` (a source string) under the profiler and return the resulting `ProfileStats`.

**Parameters:** `cmd: string`
**Returns:** `ProfileStats`

```titrate
let p: Profile = new Profile();
let stats: ProfileStats = p.run("fib(20);");
p.printStats(stats);
```

#### runctx

Execute `cmd` with the given `globals` and `locals` maps under the profiler.

**Parameters:** `cmd: string`, `globals: HashMap<string, Variant>`, `locals: HashMap<string, Variant>`
**Returns:** `ProfileStats`

#### runfunc

Call `fn` with the given `args` under the profiler and return its result and the collected stats.

**Parameters:** `fn: Variant`, `args: ArrayList<Variant>`
**Returns:** `(Variant, ProfileStats)` — `(returnValue, stats)`

### Results

#### results

Return the `ProfileStats` collected so far. Profiling must be disabled before calling this.

**Returns:** `ProfileStats`

#### printStats

Format `stats` as a human-readable table and return it as a string.

**Parameters:** `stats: ProfileStats` (optional, defaults to `results()`)
**Returns:** `string`

The output table has columns:

- `ncalls` — number of calls (with per-call recursion count)
- `tottime` — total time spent in the function (excluding callees)
- `percall` — `tottime / ncalls`
- `cumtime` — cumulative time (including callees)
- `percall` — `cumtime / ncalls`
- `filename:lineno(function)` — function identifier

### Sorting

#### sortStats

Sort the stats by the given key before printing. Valid keys are: `"calls"`, `"total"`, `"cumulative"`, `"file"`, `"name"`.

**Parameters:** `key: string`
**Returns:** `void`

#### stripDirs

Strip directory prefixes from filenames in the stats table. Convenient for short output.

**Returns:** `void`

## Module-level helpers

### run

Convenience function: construct a `Profile`, call `run(cmd)`, print the stats, and return the `ProfileStats`.

**Parameters:** `cmd: string`
**Returns:** `ProfileStats`

### runctx

Like `run` but with explicit globals and locals.

**Parameters:** `cmd: string`, `globals: HashMap<string, Variant>`, `locals: HashMap<string, Variant>`
**Returns:** `ProfileStats`

## Notes

- The profiler is deterministic — it does not sample. The wall-clock timer uses nanosecond precision; the CPU timer uses the OS's process CPU time.
- Recursion is tracked: a function that calls itself shows `(native / total)` call counts in the output, mirroring `pstats`.
- Profiling introduces non-trivial overhead (typically 2–5×); for production profiling prefer the statistical profiler in `tt.tooling.CProfile`.
- The output table is sortable in-place; sorting affects only the formatted string returned by `printStats`, not the underlying data.
