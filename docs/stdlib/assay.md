---
title: assay
description: Titrate's built-in testing framework — suites, assertions, lifecycle hooks, and runners.
---

# assay

The `tt.assay` module is Titrate's built-in testing framework. It provides test suites, assertions, lifecycle hooks, filtering, and a test runner for organizing and executing tests.

```titrate
import tt::assay::Assay;
import tt::assay::TestRunner;
```

## TestSuite

A `TestSuite` groups related test cases and provides assertion helpers.

- `fn init(name: string)`
- `addTest(name: string, testFn: fn(): void): void`
- `beforeEach(fn: fn(): void): void`
- `afterEach(fn: fn(): void): void`
- `run(): void`
- `summary(): string`
- `skip(msg: string): void`
- `timeout(ms: int, fn: fn(): void): void`
- `fail(msg: string): void`

### Assertions

| Assertion | Description |
|-----------|-------------|
| `assertEqual<T>(actual: T, expected: T, msg: string): void` | Check equality |
| `assertNotEqual<T>(actual: T, expected: T, msg: string): void` | Check inequality |
| `assertTrue(condition: bool, msg: string): void` | Check truthy |
| `assertFalse(condition: bool, msg: string): void` | Check falsy |
| `assertThrows(fn: fn(): void, msg: string): void` | Check function throws |
| `assertApproxEqual(actual: double, expected: double, tolerance: double, msg: string): void` | Floating-point comparison |
| `assertGreaterThan(actual: double, expected: double, msg: string): void` | Numeric greater than |
| `assertLessThan(actual: double, expected: double, msg: string): void` | Numeric less than |
| `assertContains<T>(collection: ArrayList<T>, item: T, msg: string): void` | Membership check |
| `assertNull<T>(value: T, msg: string): void` | Check null |
| `assertNotNull<T>(value: T, msg: string): void` | Check non-null |

```titrate
let suite: Assay.TestSuite = new Assay.TestSuite("Math tests");

suite.beforeEach(fn(): void {
    io::println("Setting up test");
});

suite.addTest("addition", fn(): void {
    suite.assertEqual(2 + 2, 4, "basic addition");
});

suite.addTest("approximation", fn(): void {
    suite.assertApproxEqual(0.1 + 0.2, 0.3, 0.0001, "float addition");
});

suite.run();
io::println(suite.summary());
```

## TestCase

A single named test case.

- `fn init(name: string, fn: fn(): void)`

## TestRunner

Runs multiple suites and reports results.

- `fn init()`
- `addSuite(suite: TestSuite): void`
- `runAll(): void`
- `filterByName(pattern: string): void`
- `exitCode(): int`
- `listTests(): void`
- `runSingle(name: string): void`
- `discoverAndRun(pattern: string): void`

```titrate
let runner: TestRunner = new TestRunner();

let mathSuite: Assay.TestSuite = new Assay.TestSuite("Math");
mathSuite.addTest("multiply", fn(): void {
    mathSuite.assertEqual(3 * 4, 12, "multiplication");
});

let stringSuite: Assay.TestSuite = new Assay.TestSuite("Strings");
stringSuite.addTest("length", fn(): void {
    stringSuite.assertEqual(String.length("hello"), 5, "string length");
});

runner.addSuite(mathSuite);
runner.addSuite(stringSuite);
runner.runAll();

Sys.exit(runner.exitCode());
```

## Organizing Tests

Group tests by feature in separate suites, then collect them in a `TestRunner`. Use `beforeEach` and `afterEach` for setup and teardown. Use `filterByName` to run only matching tests during development.

## Mocking and extended assertions (Phase 1-2 parity)

### Mock / Patch

The `Mock` module mirrors Python's `unittest.mock` for replacing dependencies during tests.

- `Mock.create(spec: Variant): Mock` — create a mock object that records calls and return values
- `Mock.patch(target: fn(): void, name: string, replacement: Variant): void` — temporarily replace a function/method within a `target` block, restoring the original afterwards
- `Mock.callCount(m: Mock): int` — number of times the mock was called
- `Mock.callArgs(m: Mock, index: int): ArrayList<Variant>` — arguments passed on the `index`-th call
- `Mock.returnValue(m: Mock, value: Variant): void` — configure the mock's return value

```titrate
import tt.assay.Mock;

let m = Mock.create(null);
Mock.returnValue(m, "fake");

Mock.patch(fn(): void {
    // Inside this block, the patched name is replaced
    doWork(m);
}, "fetchValue", m);
```

### Regex, warning, and log assertions

These extended assertions round out `unittest.TestCase` parity.

| Assertion | Description |
|-----------|-------------|
| `assertRaisesRegex(fn: fn(): void, pattern: string, msg: string): void` | Check that the function throws, and the message matches `pattern` |
| `assertWarns(fn: fn(): void, msg: string): void` | Check that the function emits a warning |
| `assertWarnsRegex(fn: fn(): void, pattern: string, msg: string): void` | Check that a warning matching `pattern` is emitted |
| `assertLogs(logger: string, level: string, fn: fn(): void): void` | Check that the function emits a log record at-or-above `level` |
| `assertNoLogs(fn: fn(): void): void` | Check that the function emits no log records |

### Conditional skips

| Method | Description |
|---------|-------------|
| `skipUnless(condition: bool, reason: string): void` | Skip the current test unless `condition` is true |
| `skipIf(condition: bool, reason: string): void` | Skip the current test if `condition` is true |

```titrate
let suite: Assay.TestSuite = new Assay.TestSuite("Regex and skips");

suite.addTest("div-by-zero message", fn(): void {
    suite.assertRaisesRegex(
        fn(): void { throw "division by zero attempted"; },
        "division by zero",
        "should throw a div-by-zero error"
    );
});

// Skip on Windows
suite.addTest("posix-only", fn(): void {
    suite.skipUnless(Os.getEnv("OS") != "Windows_NT", "requires POSIX");
    // ... POSIX-only assertions ...
});
```
