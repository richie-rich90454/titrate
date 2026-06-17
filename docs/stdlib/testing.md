# testing

The `tt.assay` module provides a testing framework with test suites, assertions, and a test runner.

```titrate
import tt.assay.Assay;
import tt.assay.TestRunner;
```

## TestSuite

A named collection of test cases with pass/fail tracking and assertion methods.

- `fn init(name: string)` — create a named test suite
- `addTest(name: string, testFn: fn(): void): void` — register a test
- `assertEqual<T>(actual: T, expected: T, msg: string): void` — assert equality
- `assertNotEqual<T>(actual: T, expected: T, msg: string): void` — assert inequality
- `assertTrue(condition: bool, msg: string): void` — assert true
- `assertFalse(condition: bool, msg: string): void` — assert false
- `assertThrows(fn: fn(): void, msg: string): void` — assert function throws
- `assertApproxEqual(actual: double, expected: double, tolerance: double, msg: string): void` — assert approximate equality
- `assertGreaterThan(actual: double, expected: double, msg: string): void`
- `assertLessThan(actual: double, expected: double, msg: string): void`
- `assertContains<T>(collection: ArrayList<T>, item: T, msg: string): void` — assert collection contains item
- `assertNull<T>(value: T, msg: string): void` — assert null
- `assertNotNull<T>(value: T, msg: string): void` — assert not null
- `beforeEach(fn: fn(): void): void` — run before each test
- `afterEach(fn: fn(): void): void` — run after each test
- `skip(msg: string): void` — skip with message
- `timeout(ms: int, fn: fn(): void): void` — run with timeout
- `fail(msg: string): void` — record a failure
- `run(): void` — run all tests and print results
- `summary(): string` — summary string

```titrate
let suite = new TestSuite("Math tests");
suite.addTest("addition", fn(): void {
    suite.assertEqual(1 + 1, 2, "basic addition");
});
suite.addTest("approximate", fn(): void {
    suite.assertApproxEqual(0.1 + 0.2, 0.3, 1e-9, "float addition");
});
suite.run();
```

## TestRunner

Runs multiple test suites and reports overall results.

- `fn init()` — create a new runner
- `addSuite(suite: TestSuite): void` — register a test suite
- `runAll(): void` — run all suites and print summary
- `filterByName(pattern: string): void` — filter suites by name
- `runSingle(name: string): void` — run a single test by name
- `listTests(): void` — list all registered tests
- `discoverAndRun(pattern: string): void` — discover test files and run
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

## Test Fixtures

- `TestFixture.setUp(fn: fn(): void): void` — setup before each test
- `TestFixture.tearDown(fn: fn(): void): void` — cleanup after each test
- `TestFixture.setUpClass(fn: fn(): void): void` — setup before all tests
- `TestFixture.tearDownClass(fn: fn(): void): void` — cleanup after all tests

## Parameterized Tests

- `Testing.parameterize(test: fn(Variant): void, params: ArrayList<Variant>): void` — run test with multiple parameters

## Test Skipping

- `Testing.skip(reason: string): void` — skip current test
- `Testing.skipIf(condition: bool, reason: string): void` — conditional skip

## Expected Failures

- `Testing.expectedFailure(test: fn(): void): void` — mark test as expected to fail

## Test Suites

- `TestSuite.init(name: string)` — create named test suite
- `TestSuite.addTest(test: fn(): void): void` — add test to suite
- `TestSuite.addSuite(suite: TestSuite): void` — add nested suite
- `TestSuite.run(): TestResult` — run all tests

## Benchmark Mode

- `Testing.benchmark(test: fn(): void, iterations: int): BenchmarkResult` — benchmark a test
- `BenchmarkResult.meanTime(): double` — mean execution time (ms)
- `BenchmarkResult.minTime(): double` — minimum execution time (ms)
- `BenchmarkResult.maxTime(): double` — maximum execution time (ms)
- `BenchmarkResult.stdDev(): double` — standard deviation
