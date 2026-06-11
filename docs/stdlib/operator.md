# operator

The `tt.operator` module provides functional equivalents of built-in operators, mirroring Python's `operator` module. Useful when passing operators as higher-order functions.

```titrate
import tt.operator.Operator;
```

## Operator

All methods are static.

### Arithmetic operators

- `add(a: int, b: int): int` — addition
- `sub(a: int, b: int): int` — subtraction
- `mul(a: int, b: int): int` — multiplication
- `truediv(a: double, b: double): double` — true division
- `mod(a: int, b: int): int` — modulo
- `neg(a: double): double` — negation
- `pos(a: double): double` — unary positive
- `abs(a: double): double` — absolute value

### Comparison operators

- `eq(a: Object, b: Object): bool` — equality
- `ne(a: Object, b: Object): bool` — inequality
- `lt(a: int, b: int): bool` — less than
- `le(a: int, b: int): bool` — less than or equal
- `gt(a: int, b: int): bool` — greater than
- `ge(a: int, b: int): bool` — greater than or equal

### Logical operators

- `not(a: bool): bool` — logical NOT
- `and(a: bool, b: bool): bool` — logical AND
- `or(a: bool, b: bool): bool` — logical OR

### Accessor helpers

- `itemGetter(arr: ArrayList<Object>, index: int): Object` — get element by index
- `attrGetter(obj: Object, attr: String): Object` — attribute access (returns null; dynamic attrs not supported)

```titrate
let sum = Operator::add(3, 4);         // 7
let cmp = Operator::gt(10, 5);         // true
let result = Operator::truediv(10.0, 3.0);  // 3.333...
```
