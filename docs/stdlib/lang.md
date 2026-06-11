# lang

The `tt.lang` module provides core language types and utilities. It includes wrapper classes for primitive types, the `Result` type for error handling, and the `Iterator`/`Iterable` interfaces that underpin the language's iteration protocol.

```titrate
import tt.lang.String;
import tt.lang.Integer;
import tt.lang.Double;
import tt.lang.Boolean;
import tt.lang.Character;
import tt.lang.Result;
import tt.lang.Iterator;
import tt.lang.Iterable;
```

## String

Static utility methods for string manipulation.

- `String.length(s: String): int` — return the length of the string
- `String.concat(a: String, b: String): String` — concatenate two strings
- `String.toUpperCase(s: String): String` — convert to uppercase
- `String.toLowerCase(s: String): String` — convert to lowercase
- `String.replace(s: String, target: String, replacement: String): String` — replace all occurrences
- `String.trim(s: String): String` — strip leading and trailing whitespace
- `String.split(s: String, delim: String): ArrayList<String>` — split by delimiter
- `String.join(parts: ArrayList<String>, delim: String): String` — join with delimiter
- `String.indexOf(s: String, sub: String): int` — find first occurrence, or `-1`
- `String.lastIndexOf(s: String, sub: String): int` — find last occurrence, or `-1`
- `String.contains(s: String, sub: String): bool` — check if substring exists
- `String.startsWith(s: String, prefix: String): bool` — check prefix
- `String.endsWith(s: String, suffix: String): bool` — check suffix
- `String.substring(s: String, start: int, end: int): String` — extract substring
- `String.format(template: String, args: ArrayList<String>): String` — format with `{}` placeholders
- `String.isEmpty(s: String): bool` — check if length is zero
- `String.isBlank(s: String): bool` — check if empty or all whitespace
- `String.repeat(s: String, count: int): String` — repeat the string
- `String.reverse(s: String): String` — reverse the string
- `String.toCharArray(s: String): ArrayList<String>` — split into individual characters
- `String.compareTo(a: String, b: String): int` — lexicographic comparison

```titrate
let parts = String::split("a,b,c", ",");
let joined = String::join(parts, "-");
let upper = String::toUpperCase("hello");
```

## Integer

Static utility methods for the `int` type.

- `Integer.parseInt(s: String): int` — parse a string to an integer
- `Integer.parseOr(s: String, defaultValue: int): int` — parse with fallback
- `Integer.toString(value: int): String` — convert to string
- `Integer.toHexString(n: int): String` — convert to hexadecimal string
- `Integer.toBinaryString(n: int): String` — convert to binary string
- `Integer.toOctalString(n: int): String` — convert to octal string
- `Integer.bitCount(n: int): int` — count one-bits
- `Integer.highestOneBit(n: int): int` — highest one-bit position
- `Integer.lowestOneBit(n: int): int` — lowest one-bit position
- `Integer.numberOfLeadingZeros(n: int): int` — count leading zero bits
- `Integer.numberOfTrailingZeros(n: int): int` — count trailing zero bits
- `Integer.signum(n: int): int` — sign function (`-1`, `0`, or `1`)
- `Integer.reverse(n: int): int` — reverse bit order
- `Integer.rotateLeft(n: int, dist: int): int` — rotate bits left
- `Integer.rotateRight(n: int, dist: int): int` — rotate bits right
- `Integer.compare(a: int, b: int): int` — compare two integers
- `Integer.max(a: int, b: int): int` — larger of two
- `Integer.min(a: int, b: int): int` — smaller of two
- `Integer.MAX_VALUE(): int` — maximum `int` value (`2147483647`)
- `Integer.MIN_VALUE(): int` — minimum `int` value (`-2147483648`)

```titrate
let n = Integer::parseInt("42");
let hex = Integer::toHexString(255);
let bits = Integer::bitCount(15);
```

## Double

Static utility methods for the `double` type.

- `Double.parseDouble(s: String): double` — parse a string to a double
- `Double.parseOr(s: String, defaultValue: double): double` — parse with fallback
- `Double.toString(value: double): String` — convert to string
- `Double.isNaN(value: double): bool` — check for NaN
- `Double.isInfinite(value: double): bool` — check for infinity
- `Double.isFinite(d: double): bool` — check for finite value
- `Double.toHexString(d: double): String` — convert to hexadecimal float representation
- `Double.compare(a: double, b: double): int` — compare two doubles
- `Double.max(a: double, b: double): double` — larger of two
- `Double.min(a: double, b: double): double` — smaller of two
- `Double.MAX_VALUE(): double` — maximum finite `double`
- `Double.MIN_VALUE(): double` — smallest positive `double`
- `Double.EPSILON(): double` — machine epsilon
- `Double.POSITIVE_INFINITY(): double` — positive infinity
- `Double.NEGATIVE_INFINITY(): double` — negative infinity
- `Double.NaN(): double` — not-a-number

```titrate
let x = Double::parseDouble("3.14");
let finite = Double::isFinite(x);
let eps = Double::EPSILON();
```

## Boolean

Static utility methods for the `bool` type.

- `Boolean.toString(value: bool): String` — convert to `"true"` or `"false"`
- `Boolean.parseBoolean(s: String): bool` — parse `"true"` to `true` (case-sensitive)
- `Boolean.logicalAnd(a: bool, b: bool): bool` — logical AND
- `Boolean.logicalOr(a: bool, b: bool): bool` — logical OR
- `Boolean.logicalNot(a: bool): bool` — logical NOT

```titrate
let s = Boolean::toString(true);
let b = Boolean::parseBoolean("true");
```

## Character

Static utility methods for character classification and conversion.

- `Character.isDigit(c: char): bool` — check if digit
- `Character.isLetter(c: char): bool` — check if letter
- `Character.isAlphabetic(c: char): bool` — check if alphabetic
- `Character.isWhitespace(c: char): bool` — check if whitespace
- `Character.isUpperCase(c: char): bool` — check if uppercase
- `Character.isLowerCase(c: char): bool` — check if lowercase
- `Character.isISOControl(c: char): bool` — check if ISO control character
- `Character.toUpperCase(c: char): char` — convert to uppercase
- `Character.toLowerCase(c: char): char` — convert to lowercase
- `Character.getNumericValue(c: char): int` — numeric value of digit or letter character
- `Character.toString(value: char): String` — convert to string

```titrate
let yes = Character::isDigit("5");
let upper = Character::toUpperCase("a");
let val = Character::getNumericValue("f");
```

## Result

A discriminated union for error handling, representing either a success (`ok`) or failure (`err`).

- `Result<T, E>.ok(value: T): Result<T, E>` — create a success result
- `Result<T, E>.err(value: E): Result<T, E>` — create an error result
- `isOk(): bool` — true if success
- `isErr(): bool` — true if failure
- `unwrap(): T` — return the success value (null if error)
- `unwrapOr(defaultValue: T): T` — return success value or default
- `unwrapErr(): E` — return the error value
- `expect(msg: String): T` — return success value or null with message
- `map(mapper: fn(T): T): Result<T, E>` — transform the success value
- `mapErr(mapper: fn(E): E): Result<T, E>` — transform the error value
- `andThen(mapper: fn(T): Result<T, E>): Result<T, E>` — chain on success
- `orElse(mapper: fn(E): Result<T, E>): Result<T, E>` — chain on error
- `flatten(): Result<T, E>` — flatten a nested Result
- `isOkAnd(fn: fn(T): bool): bool` — check success value with predicate
- `isErrAnd(fn: fn(E): bool): bool` — check error value with predicate
- `inspect(fn: fn(T): void): Result<T, E>` — inspect success value
- `inspectErr(fn: fn(E): void): Result<T, E>` — inspect error value

```titrate
let r = Result<int, String>::ok(42);
if (r.isOk()) {
    io::println(r.unwrap().toString());
}
let e = Result<int, String>::err("not found");
let fallback = e.unwrapOr(0);
```

## Iterator

The core iteration interface. All iterable types implement this.

- `next(): E` — return the next element
- `hasNext(): bool` — check if more elements exist
- `forEachRemaining(fn: fn(E): void)` — consume remaining elements
- `map<R>(fn: fn(E): R): Iterator<R>` — transform elements
- `filter(fn: fn(E): bool): Iterator<E>` — keep matching elements
- `reduce(init: E, fn: fn(E, E): E): E` — fold into a single value
- `collect(): ArrayList<E>` — consume into an ArrayList
- `count(): int` — count remaining elements
- `take(n: int): Iterator<E>` — take at most `n` elements
- `skip(n: int): Iterator<E>` — skip the first `n` elements
- `zip(other: Iterator<E>): Iterator<E>` — pair with another iterator
- `chain(other: Iterator<E>): Iterator<E>` — concatenate with another iterator
- `enumerate(): Iterator<(int, E)>` — pair each element with its index
- `any(fn: fn(E): bool): bool` — true if any element matches
- `all(fn: fn(E): bool): bool` — true if all elements match
- `find(fn: fn(E): bool): E` — first matching element
- `max(): E` — largest element
- `min(): E` — smallest element
- `nth(n: int): E` — nth element
- `last(): E` — last element

```titrate
let iter = [1, 2, 3, 4, 5].iterator();
let doubled = iter.map(fn(n: int): int => n * 2);
let sum = doubled.reduce(0, fn(a: int, b: int): int => a + b);
```

## Iterable

The interface for types that can produce an iterator.

- `iterator(): Iterator<E>` — return an iterator over the elements

```titrate
// Implemented by ArrayList, HashMap, Set, etc.
let list = new ArrayList<int>();
list.add(1);
list.add(2);
let iter = list.iterator();
```
