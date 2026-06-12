# operator

The `tt.operator` module provides functional equivalents of built-in operators, mirroring Python's `operator` module. Instead of writing `a + b`, you can write `Operator.add(a, b)`. This is particularly useful when passing operators as higher-order functions to `map`, `filter`, `reduce`, and other functional patterns.

```titrate
import tt.operator.Operator;
```

## Why Use the Operator Module?

Titrate's built-in operators (`+`, `-`, `==`, etc.) are syntax — they cannot be passed as arguments to functions. The `Operator` module wraps each operator in a function, enabling:

- **Higher-order programming**: Pass operators as function arguments
- **Dynamic dispatch**: Select an operator at runtime
- **Functional patterns**: Use operators with `map`, `filter`, and `reduce`
- **Table-driven logic**: Store operators in data structures

```titrate
// Without Operator module — need a lambda every time
let sum: int = list.reduce(fn(a: int, b: int): int { return a + b; });

// With Operator module — pass the function directly
let sum: int = list.reduce(Operator.add);
```

## Arithmetic Operators

### `add(a: int, b: int): int`

Returns the sum of `a` and `b`.

```titrate
let result: int = Operator.add(3, 4);    // 7
let sum: int = Operator.add(100, -25);   // 75
```

### `sub(a: int, b: int): int`

Returns the difference of `a` and `b`.

```titrate
let result: int = Operator.sub(10, 3);   // 7
let diff: int = Operator.sub(5, 12);     // -7
```

### `mul(a: int, b: int): int`

Returns the product of `a` and `b`.

```titrate
let result: int = Operator.mul(6, 7);    // 42
let product: int = Operator.mul(-3, 8);  // -24
```

### `truediv(a: double, b: double): double`

Returns the true division of `a` by `b` (floating-point result).

```titrate
let result: double = Operator.truediv(10.0, 3.0);  // 3.333...
let half: double = Operator.truediv(7.0, 2.0);     // 3.5
```

### `mod(a: int, b: int): int`

Returns the remainder of `a` divided by `b`.

```titrate
let result: int = Operator.mod(10, 3);   // 1
let rem: int = Operator.mod(17, 5);      // 2
```

### `neg(a: double): double`

Returns the negation of `a`.

```titrate
let result: double = Operator.neg(3.14);   // -3.14
let positive: double = Operator.neg(-5.0); // 5.0
```

### `pos(a: double): double`

Returns the unary positive of `a` (identity operation).

```titrate
let result: double = Operator.pos(3.14);  // 3.14
```

### `abs(a: double): double`

Returns the absolute value of `a`.

```titrate
let result: double = Operator.abs(-3.14);  // 3.14
let same: double = Operator.abs(5.0);      // 5.0
```

## Comparison Operators

### `eq<T>(a: T, b: T): bool`

Returns `true` if `a` equals `b`.

```titrate
let result: bool = Operator.eq(42, 42);       // true
let diff: bool = Operator.eq("hello", "hi");  // false
```

### `ne<T>(a: T, b: T): bool`

Returns `true` if `a` does not equal `b`.

```titrate
let result: bool = Operator.ne(1, 2);   // true
let same: bool = Operator.ne(5, 5);     // false
```

### `lt(a: int, b: int): bool`

Returns `true` if `a` is less than `b`.

```titrate
let result: bool = Operator.lt(3, 5);   // true
let nope: bool = Operator.lt(5, 3);     // false
```

### `le(a: int, b: int): bool`

Returns `true` if `a` is less than or equal to `b`.

```titrate
let result: bool = Operator.le(3, 5);   // true
let equal: bool = Operator.le(5, 5);    // true
let nope: bool = Operator.le(6, 5);     // false
```

### `gt(a: int, b: int): bool`

Returns `true` if `a` is greater than `b`.

```titrate
let result: bool = Operator.gt(10, 5);  // true
let nope: bool = Operator.gt(5, 10);    // false
```

### `ge(a: int, b: int): bool`

Returns `true` if `a` is greater than or equal to `b`.

```titrate
let result: bool = Operator.ge(10, 5);  // true
let equal: bool = Operator.ge(5, 5);    // true
let nope: bool = Operator.ge(4, 5);     // false
```

## Logical Operators

### `not(a: bool): bool`

Returns the logical negation of `a`.

```titrate
let result: bool = Operator.not(true);   // false
let inverse: bool = Operator.not(false); // true
```

### `and(a: bool, b: bool): bool`

Returns the logical AND of `a` and `b`.

```titrate
let result: bool = Operator.and(true, true);    // true
let nope: bool = Operator.and(true, false);     // false
let nope2: bool = Operator.and(false, false);   // false
```

### `or(a: bool, b: bool): bool`

Returns the logical OR of `a` and `b`.

```titrate
let result: bool = Operator.or(true, false);    // true
let both: bool = Operator.or(true, true);       // true
let nope: bool = Operator.or(false, false);     // false
```

## Accessor Helpers

### `itemGetter<T>(arr: ArrayList<T>, index: int): T`

Returns the element at the given index in an `ArrayList`. This is the functional equivalent of `arr.get(index)`.

```titrate
import tt.util.ArrayList;

let list: ArrayList<string> = new ArrayList<string>();
list.add("apple");
list.add("banana");
list.add("cherry");

let item: string = Operator.itemGetter(list, 1);
io::println(item);  // "banana"
```

### `attrGetter<T>(obj: T, attr: string): Variant`

Attempts to access an attribute on an object by name. Since Titrate does not support dynamic attribute access, this function returns `null` as a fallback. It is provided for API compatibility with patterns that expect attribute accessors.

```titrate
let result: Variant = Operator.attrGetter(someObj, "name");  // returns null
```

## Built-in Operators Reference

While the `Operator` module provides functional wrappers, Titrate also supports a rich set of built-in operators directly in the language syntax.

### Arithmetic Operators

| Operator | Name | Example | Result |
|----------|------|---------|--------|
| `+` | Addition | `3 + 4` | `7` |
| `-` | Subtraction | `10 - 3` | `7` |
| `*` | Multiplication | `6 * 7` | `42` |
| `/` | Division | `10 / 3` | `3` (int) or `3.333...` (double) |
| `%` | Modulo | `10 % 3` | `1` |

### Comparison Operators

| Operator | Name | Example | Result |
|----------|------|---------|--------|
| `==` | Equal | `5 == 5` | `true` |
| `!=` | Not equal | `5 != 3` | `true` |
| `<` | Less than | `3 < 5` | `true` |
| `>` | Greater than | `5 > 3` | `true` |
| `<=` | Less than or equal | `5 <= 5` | `true` |
| `>=` | Greater than or equal | `5 >= 3` | `true` |

### Logical Operators

| Operator | Name | Example | Result |
|----------|------|---------|--------|
| `&&` | Logical AND | `true && false` | `false` |
| `\|\|` | Logical OR | `true \|\| false` | `true` |
| `!` | Logical NOT | `!true` | `false` |

### Bitwise Operators

| Operator | Name | Example | Result |
|----------|------|---------|--------|
| `&` | Bitwise AND | `0xFF & 0x0F` | `0x0F` |
| `\|` | Bitwise OR | `0xF0 \| 0x0F` | `0xFF` |
| `^` | Bitwise XOR | `0xFF ^ 0x0F` | `0xF0` |
| `~` | Bitwise NOT | `~0x0F` | `0xFFFFFFF0` |
| `<<` | Left shift | `1 << 4` | `16` |
| `>>` | Right shift | `16 >> 2` | `4` |

### Assignment Operators

| Operator | Name | Example | Equivalent |
|----------|------|---------|------------|
| `=` | Assignment | `x = 5` | — |
| `+=` | Add and assign | `x += 3` | `x = x + 3` |
| `-=` | Subtract and assign | `x -= 2` | `x = x - 2` |
| `*=` | Multiply and assign | `x *= 4` | `x = x * 4` |
| `/=` | Divide and assign | `x /= 2` | `x = x / 2` |
| `%=` | Modulo and assign | `x %= 3` | `x = x % 3` |
| `&=` | AND and assign | `x &= 0xFF` | `x = x & 0xFF` |
| `\|=` | OR and assign | `x \|= 0x0F` | `x = x \| 0x0F` |
| `^=` | XOR and assign | `x ^= 0x0F` | `x = x ^ 0x0F` |
| `<<=` | Left shift and assign | `x <<= 2` | `x = x << 2` |
| `>>=` | Right shift and assign | `x >>= 1` | `x = x >> 1` |

### Increment / Decrement

| Operator | Name | Example | Effect |
|----------|------|---------|--------|
| `++` | Increment | `x++` or `++x` | `x = x + 1` |
| `--` | Decrement | `x--` or `--x` | `x = x - 1` |

### Type Operators

| Operator | Name | Example | Result |
|----------|------|---------|--------|
| `as` | Type cast | `42 as double` | `42.0` |
| `is` | Type check | `obj is Circle` | `true` or `false` |

```titrate
// Type casting with 'as'
let x: int = 42;
let d: double = x as double;
let s: string = x as string;

// Type checking with 'is'
if (shape is Circle) {
    let c: Circle = shape as Circle;
    io::println(Double.toString(c.radius()));
}
```

### Member Access Operators

| Operator | Name | Example | Description |
|----------|------|---------|-------------|
| `.` | Dot access | `Math.sqrt(2.0)` | Method/field access (preferred) |
| `::` | Scope access | `import tt::math::Math` | Import paths only |

### Range Operators

| Operator | Name | Example | Description |
|----------|------|---------|-------------|
| `..` | Exclusive range | `0..5` | 0, 1, 2, 3, 4 |
| `..=` | Inclusive range | `0..=5` | 0, 1, 2, 3, 4, 5 |

### Ternary Operator

| Operator | Name | Example |
|----------|------|---------|
| `? :` | Conditional | `x > 0 ? x : -x` |

```titrate
let age: int = 20;
let label: string = age >= 18 ? "adult" : "minor";
```

## Operator Precedence

Operators are evaluated in the following order, from highest to lowest precedence:

| Precedence | Operators | Associativity |
|------------|-----------|---------------|
| 1 (highest) | `.` `::` | Left |
| 2 | `++` `--` (postfix) | Left |
| 3 | `++` `--` (prefix) `!` `~` `-` (unary) | Right |
| 4 | `as` `is` | Left |
| 5 | `*` `/` `%` | Left |
| 6 | `+` `-` | Left |
| 7 | `<<` `>>` | Left |
| 8 | `<` `>` `<=` `>=` | Left |
| 9 | `==` `!=` | Left |
| 10 | `&` | Left |
| 11 | `^` | Left |
| 12 | `\|` | Left |
| 13 | `&&` | Left |
| 14 | `\|\|` | Left |
| 15 | `? :` | Right |
| 16 (lowest) | `=` `+=` `-=` `*=` `/=` `%=` `&=` `\|=` `^=` `<<=` `>>=` | Right |

When in doubt, use parentheses to make the intended order explicit:

```titrate
let result: int = (2 + 3) * 4;  // 20, not 14
let valid: bool = (a > 0) && (b > 0);  // clear intent
```

## Using Operators with Higher-Order Functions

The primary use case for the `Operator` module is passing operators as function arguments:

### With Map

```titrate
import tt.operator.Operator;

// Double every element in a list
let numbers: ArrayList<int> = new ArrayList<int>();
numbers.add(1);
numbers.add(2);
numbers.add(3);

// Using a lambda
let doubled: ArrayList<int> = numbers.map(fn(x: int): int { return x * 2; });

// Using Operator module (if map accepts a function reference)
// This is more concise for simple operations
```

### With Reduce / Accumulate

```titrate
import tt.operator.Operator;

// Sum a list of numbers
let numbers: ArrayList<int> = new ArrayList<int>();
numbers.add(1);
numbers.add(2);
numbers.add(3);
numbers.add(4);

// Using Operator.add as the reducer
let total: int = numbers.reduce(Operator.add);  // 10
```

### With Sort / Filter

```titrate
import tt.operator.Operator;

// Filter elements greater than a threshold
let values: ArrayList<int> = new ArrayList<int>();
values.add(5);
values.add(15);
values.add(3);
values.add(20);

// Using Operator.gt in a comparison
let big: ArrayList<int> = values.filter(fn(x: int): bool { return Operator.gt(x, 10); });
// big contains [15, 20]
```

## Operator Overloading

Titrate supports operator overloading through the `fn operator<op>` syntax in classes. This allows your custom types to work with built-in operators:

```titrate
public class Vec2 {
    public double x;
    public double y;

    public fn init(x: double, y: double) {
        this.x = x;
        this.y = y;
    }

    public fn operator+(other: Vec2): Vec2 {
        return new Vec2(this.x + other.x, this.y + other.y);
    }

    public fn operator*(scalar: double): Vec2 {
        return new Vec2(this.x * scalar, this.y * scalar);
    }

    public fn operator==(other: Vec2): bool {
        return this.x == other.x && this.y == other.y;
    }
}
```

```titrate
let a: Vec2 = new Vec2(1.0, 2.0);
let b: Vec2 = new Vec2(3.0, 4.0);
let sum: Vec2 = a + b;           // Vec2(4.0, 6.0)
let scaled: Vec2 = a * 2.0;      // Vec2(2.0, 4.0)
let equal: bool = a == b;        // false
```

### Overloadable Operators

| Operator | Method Name | Example |
|----------|------------|---------|
| `+` | `operator+` | `fn operator+(other: T): T` |
| `-` | `operator-` | `fn operator-(other: T): T` |
| `*` | `operator*` | `fn operator*(other: T): T` |
| `/` | `operator/` | `fn operator/(other: T): T` |
| `%` | `operator%` | `fn operator%(other: T): T` |
| `==` | `operator==` | `fn operator==(other: T): bool` |
| `!=` | `operator!=` | `fn operator!=(other: T): bool` |
| `<` | `operator<` | `fn operator<(other: T): bool` |
| `>` | `operator>` | `fn operator>(other: T): bool` |
| `<=` | `operator<=` | `fn operator<=(other: T): bool` |
| `>=` | `operator>=` | `fn operator>=(other: T): bool` |
| `&` | `operator&` | `fn operator&(other: T): T` |
| `\|` | `operator\|` | `fn operator\|(other: T): T` |
| `^` | `operator^` | `fn operator^(other: T): T` |
| `<<` | `operator<<` | `fn operator<<(other: T): T` |
| `>>` | `operator>>` | `fn operator>>(other: T): T` |

## Function Reference

| Function | Signature | Description |
|----------|-----------|-------------|
| `add` | `(a: int, b: int): int` | Addition (`a + b`) |
| `sub` | `(a: int, b: int): int` | Subtraction (`a - b`) |
| `mul` | `(a: int, b: int): int` | Multiplication (`a * b`) |
| `truediv` | `(a: double, b: double): double` | True division (`a / b`) |
| `mod` | `(a: int, b: int): int` | Modulo (`a % b`) |
| `neg` | `(a: double): double` | Negation (`-a`) |
| `pos` | `(a: double): double` | Unary positive (`+a`) |
| `abs` | `(a: double): double` | Absolute value (`|a|`) |
| `eq` | `<T>(a: T, b: T): bool` | Equality (`a == b`) |
| `ne` | `<T>(a: T, b: T): bool` | Inequality (`a != b`) |
| `lt` | `(a: int, b: int): bool` | Less than (`a < b`) |
| `le` | `(a: int, b: int): bool` | Less than or equal (`a <= b`) |
| `gt` | `(a: int, b: int): bool` | Greater than (`a > b`) |
| `ge` | `(a: int, b: int): bool` | Greater than or equal (`a >= b`) |
| `not` | `(a: bool): bool` | Logical NOT (`!a`) |
| `and` | `(a: bool, b: bool): bool` | Logical AND (`a && b`) |
| `or` | `(a: bool, b: bool): bool` | Logical OR (`a \|\| b`) |
| `itemGetter` | `<T>(arr: ArrayList<T>, index: int): T` | Get element by index |
| `attrGetter` | `<T>(obj: T, attr: string): Variant` | Attribute access (returns null) |
