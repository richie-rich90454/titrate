# Pattern Matching

## Switch Statements

Titrate uses `switch` for pattern matching on enum values.

```titrate
enum Result {
    Ok(int),
    Err(string),
}

switch result {
    case Ok(value) => io::println("success: " + Integer.toString(value));
    case Err(msg) => io::println("error: " + msg);
    default => io::println("unknown");
}
```

## Wildcard Pattern

Use `_` to match any value:

```titrate
switch result {
    case Ok(_) => io::println("success");
    case Err(_) => io::println("failure");
}
```
