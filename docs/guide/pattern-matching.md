# Pattern Matching

## Switch Statements

Titrate uses `switch` for pattern matching on enum values.

```titrate
enum HttpStatus {
    Ok(int),
    NotFound,
    ServerError(string),
}

switch status {
    case Ok(code) => io::println("success: " + Integer.toString(code));
    case NotFound => io::println("not found");
    case ServerError(msg) => io::println("error: " + msg);
}
```

## Wildcard Pattern

Use `_` to match any value:

```titrate
switch status {
    case Ok(_) => io::println("success");
    case NotFound => io::println("not found");
    case ServerError(_) => io::println("failure");
}
```
