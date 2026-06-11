# testing

The `tt.assay` module provides a testing framework with test suites, assertions, and a test runner.

```titrate
import tt.assay.Assay;
import tt.assay.TestRunner;
```

## TestSuite

A named collection of test cases with pass/fail tracking and assertion methods.

- `TestSuite(name: String)` — create a named test suite
- `addTest(name: String, testFn: fn() => void): void` — register a test
- `assertEqual<T>(actual: T, expected: T, msg: String): void` — assert equality
- `assertNotEqual<T>(actual: T, expected: T, msg: String): void` — assert inequality
- `assertTrue(condition: bool, msg: String): void` — assert true
- `assertFalse(condition: bool, msg: String): void` — assert false
- `assertThrows(fn: fn() => void, msg: String): void` — assert function throws
- `assertApproxEqual(actual: double, expected: double, tolerance: double, msg: String): void` — assert approximate equality
- `assertGreaterThan(actual: double, expected: double, msg: String): void`
- `assertLessThan(actual: double, expected: double, msg: String): void`
- `assertContains<T>(collection: ArrayList<T>, item: T, msg: String): void` — assert collection contains item
- `assertNull<T>(value: T, msg: String): void` — assert null
- `assertNotNull<T>(value: T, msg: String): void` — assert not null
- `beforeEach(fn: fn() => void): void` — run before each test
- `afterEach(fn: fn() => void): void` — run after each test
- `skip(msg: String): void` — skip with message
- `timeout(ms: int, fn: fn() => void): void` — run with timeout
- `fail(msg: String): void` — record a failure
- `run(): void` — run all tests and print results
- `summary(): String` — summary string

```titrate
let suite = new TestSuite("Math tests");
suite.addTest("addition", fn() => void {
    suite.assertEqual(1 + 1, 2, "basic addition");
});
suite.addTest("approximate", fn() => void {
    suite.assertApproxEqual(0.1 + 0.2, 0.3, 1e-9, "float addition");
});
suite.run();
```

## TestRunner

Runs multiple test suites and reports overall results.

- `TestRunner()` — create a new runner
- `addSuite(suite: TestSuite): void` — register a test suite
- `runAll(): void` — run all suites and print summary
- `filterByName(pattern: String): void` — filter suites by name
- `runSingle(name: String): void` — run a single test by name
- `listTests(): void` — list all registered tests
- `discoverAndRun(pattern: String): void` — discover test files and run
- `exitCode(): int` — 0 if all passed, 1 otherwise

```titrate
let runner = new TestRunner();
runner.addSuite(suite1);
runner.addSuite(suite2);
runner.runAll();
if (runner.exitCode() != 0) {
    Sys.exit(1);
}
```
