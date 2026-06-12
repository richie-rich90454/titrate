# assert

The `tt.assert` module provides static assertion methods for validating conditions at runtime. Throwing `AssertionError` on failure makes it easy to catch contract violations in tests and debug builds.

```titrate
import tt.assert.Assert;
```

## Assert

All methods are static. Each throws `"AssertionError: ..."` on failure.

- `assertTrue(condition: bool): void` — assert condition is true
- `assertTrue(condition: bool, message: string): void` — assert true with custom message
- `assertFalse(condition: bool): void` — assert condition is false
- `assertFalse(condition: bool, message: string): void` — assert false with custom message
- `assertEqual<T>(expected: T, actual: T): void` — assert equality
- `assertEqual<T>(expected: T, actual: T, message: string): void` — assert equality with custom message
- `assertNotEqual<T>(expected: T, actual: T): void` — assert inequality
- `assertNull<T>(value: T): void` — assert value is null
- `assertNotNull<T>(value: T): void` — assert value is not null
- `assertInRange(value: int, low: int, high: int): void` — assert int is in inclusive range `[low, high]`
- `assertInRange(value: double, low: double, high: double): void` — assert double is in inclusive range `[low, high]`
- `fail(message: string): void` — always fail with message

```titrate
Assert.assertTrue(1 + 1 == 2);
Assert.assertEqual("hello", "hello");
Assert.assertNotNull(42);
Assert.assertInRange(5, 1, 10);

// With custom message
Assert.assertTrue(x > 0, "x must be positive");
```
