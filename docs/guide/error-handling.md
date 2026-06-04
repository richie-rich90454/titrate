# Error Handling

## Result Type

```titrate
let result: Result<int, string> = Ok(42);
```

## Error Propagation with `?`

```titrate
fn try_parse(s: string): Result<int, string> {
    let value: Result<int, string> = Integer.parseInt(s);
    let n: int = value?;  // returns Err early if value is Err
    return Ok(n * 2);
}
```

## ok and err Constructors

```titrate
let good: Result<int, string> = Ok(42);
let bad: Result<int, string> = Err("parse failed");
```
