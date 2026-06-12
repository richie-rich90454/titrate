# contextlib

The `tt.contextlib` module provides context manager utilities for resource management. Context managers ensure that setup and teardown logic — like opening and closing files, acquiring and releasing locks, or redirecting and restoring output — is always executed, even when errors occur.

```titrate
import tt.contextlib.Contextlib;
```

## What Are Context Managers?

A **context manager** is a programming pattern that guarantees cleanup code runs after a block of code completes, regardless of whether the block exits normally or throws an error. This is similar to `try/finally` in other languages, but more concise and less error-prone.

Without context managers, resource cleanup is verbose and easy to forget:

```titrate
// Without context managers — verbose and error-prone
let file: File = File.open("data.txt");
// ... use file ...
file.close();  // What if an error occurs above? This never runs!
```

With context managers, cleanup is automatic:

```titrate
// With context managers — automatic cleanup
Contextlib.closing(file, fn(): void {
    // ... use file ...
    // file.close() is called automatically, even on error
});
```

## The `with` Statement

Titrate provides the `with` keyword as built-in syntax for context managers. The `with` statement ensures that the resource's cleanup logic runs when the block exits:

```titrate
with (resource) {
    // use resource
}
// resource.close() is called automatically
```

The `with` statement is syntactic sugar for the `closing()` function. Both forms are equivalent:

```titrate
// Using with statement
with (file) {
    // work with file
}

// Equivalent using closing()
Contextlib.closing(file, fn(): void {
    // work with file
});
```

## Contextlib Functions

All functions in the `Contextlib` module are top-level static functions.

### `suppress(block: fn(): void): void`

Runs a block of code and silently swallows any error that occurs. This is useful when you want to attempt an operation that may fail, but don't care about the error.

**When to use:**
- Optional operations that shouldn't crash the program
- Cleanup code that might fail but shouldn't mask the original error
- Best-effort operations where failure is acceptable

```titrate
// Suppress a potentially failing operation
Contextlib.suppress(fn(): void {
    let result: Result<File, string> = File.openRead("optional_config.txt");
    // If the file doesn't exist, the error is silently ignored
});

// Useful for optional cleanup steps
public fn safeDelete(path: string): void {
    Contextlib.suppress(fn(): void {
        File.delete(path);
        // If deletion fails (e.g., file locked), just continue
    });
}
```

**Caution**: Overusing `suppress` can hide real bugs. Only suppress errors when you genuinely don't care about the outcome. If you need to know whether an operation succeeded, use `Result<T, E>` instead.

### `redirectStdout(block: fn(): void): void`

Runs a block with stdout redirection. All output produced by `io::println` and `io::print` within the block is redirected via a VM runtime hook. This is useful for capturing output for testing or logging.

**When to use:**
- Capturing output in tests
- Redirecting logs to a file
- Suppressing noisy output temporarily

```titrate
// Redirect output during a specific operation
Contextlib.redirectStdout(fn(): void {
    io::println("This output is redirected");
    // The VM runtime hook determines where this goes
});

// Useful in testing: capture output for verification
public fn testGreeting(): void {
    Contextlib.redirectStdout(fn(): void {
        greetUser("Alice");
        // Output is captured by the test framework
    });
    // Assert on captured output
}
```

### `closing<T>(resource: T, block: fn(): void): void`

Runs a block of code and ensures that `resource.close()` is called afterward, even if the block throws an error. This is the programmatic equivalent of the `with` statement.

**When to use:**
- File handling — ensure files are closed after reading/writing
- Network connections — ensure sockets are closed
- Database connections — ensure connections are returned to the pool
- Any resource with a `close()` method

```titrate
// Ensure a file is closed after use
let file: File = File.open("data.txt");
Contextlib.closing(file, fn(): void {
    let content: string = file.readAll();
    io::println(content);
    // Even if readAll() throws, file.close() is still called
});
// file.close() has been called
```

## Creating Custom Context Managers

You can create custom context managers by defining classes with a `close()` method and using them with `Contextlib.closing()` or the `with` statement.

### Basic Custom Context Manager

```titrate
public class Timer {
    public string label;
    public double startTime;

    public fn init(label: string) {
        this.label = label;
        this.startTime = 0.0;
    }

    public fn start(): void {
        this.startTime = System.currentTimeMillis();
        io::println("[" + this.label + "] started");
    }

    public fn close(): void {
        let elapsed: double = System.currentTimeMillis() - this.startTime;
        io::println("[" + this.label + "] elapsed: " + Double.toString(elapsed) + "ms");
    }
}

// Usage
let timer: Timer = new Timer("query");
timer.start();
Contextlib.closing(timer, fn(): void {
    // perform the timed operation
    executeQuery();
});
// timer.close() is called automatically, printing elapsed time
```

### Resource Pool Context Manager

```titrate
public class ConnectionPool {
    public ArrayList<Connection> available;
    public ArrayList<Connection> inUse;

    public fn init(size: int) {
        this.available = new ArrayList<Connection>();
        this.inUse = new ArrayList<Connection>();
        var i: int = 0;
        while (i < size) {
            this.available.add(new Connection());
            i = i + 1;
        }
    }

    public fn acquire(): Connection {
        let conn: Connection = this.available.get(0);
        this.available.remove(0);
        this.inUse.add(conn);
        return conn;
    }

    public fn close(): void {
        // Return all in-use connections to the pool
        for (conn in this.inUse) {
            this.available.add(conn);
        }
        this.inUse.clear();
        io::println("All connections returned to pool");
    }
}
```

## Nesting Context Managers

When you need to manage multiple resources simultaneously, you can nest context manager calls. Each resource is guaranteed to be cleaned up in reverse order (innermost first):

```titrate
// Nested context managers for multiple resources
let input: File = File.open("input.txt");
Contextlib.closing(input, fn(): void {
    let output: File = File.openWrite("output.txt");
    Contextlib.closing(output, fn(): void {
        // Both input and output are open
        let line: string = input.readLine();
        output.write(line);
    });
    // output.close() called here
});
// input.close() called here
```

### Nesting with `suppress`

You can combine different contextlib functions for complex resource management:

```titrate
// Suppress errors in a nested context
let file: File = File.open("data.txt");
Contextlib.closing(file, fn(): void {
    Contextlib.suppress(fn(): void {
        // Try to write a backup, but don't fail if it doesn't work
        let backup: File = File.openWrite("data.txt.bak");
        backup.write(file.readAll());
        backup.close();
    });
    // Continue with primary operation even if backup failed
    processData(file);
});
```

## Common Patterns

### File Handling

The most common use of context managers is ensuring files are properly closed:

```titrate
// Reading a file safely
public fn readFile(path: string): string {
    let file: File = File.open(path);
    var content: string = "";
    Contextlib.closing(file, fn(): void {
        content = file.readAll();
    });
    return content;
}

// Writing a file safely
public fn writeFile(path: string, data: string): void {
    let file: File = File.openWrite(path);
    Contextlib.closing(file, fn(): void {
        file.write(data);
    });
}
```

### Database Connections

```titrate
public fn queryUser(id: int): User {
    let conn: DbConnection = Database.connect("localhost", 5432);
    var user: User = new User();
    Contextlib.closing(conn, fn(): void {
        let result: Result<string, string> = conn.query("SELECT * FROM users WHERE id = " + Integer.toString(id));
        if (result.isOk()) {
            user = parseUser(result.unwrap());
        }
    });
    return user;
}
```

### Lock Management

```titrate
public fn synchronizedUpdate(lock: Lock, data: SharedData): void {
    lock.acquire();
    Contextlib.closing(lock, fn(): void {
        data.increment();
        data.save();
    });
    // lock.close() (which releases the lock) is called automatically
}
```

## Error Handling Within Context Managers

Context managers guarantee that cleanup code runs even when errors occur. This is the key advantage over manual resource management:

```titrate
// Without context manager — cleanup is skipped on error
let file: File = File.open("data.txt");
let content: string = file.readAll();  // What if this throws?
file.close();  // Never reached if readAll() throws!

// With context manager — cleanup always runs
let file: File = File.open("data.txt");
Contextlib.closing(file, fn(): void {
    let content: string = file.readAll();  // If this throws...
    processContent(content);
});
// ...file.close() is still called
```

### Error Propagation

Errors within the block are not caught (except by `suppress`). They propagate to the caller after cleanup:

```titrate
public fn processFile(path: string): void {
    let file: File = File.open(path);
    Contextlib.closing(file, fn(): void {
        let content: string = file.readAll();
        if (String.length(content) == 0) {
            // This error propagates to the caller,
            // but file.close() is still called first
            throw "File is empty";
        }
        processContent(content);
    });
}
```

### Combining `suppress` with `closing`

Use `suppress` inside `closing` to handle optional operations within a managed resource:

```titrate
let file: File = File.open("config.txt");
Contextlib.closing(file, fn(): void {
    Contextlib.suppress(fn(): void {
        // Try to read an optional section — don't fail if missing
        let section: string = file.findSection("advanced");
        applyAdvancedSettings(section);
    });
    // Continue with required sections
    let mainConfig: string = file.findSection("main");
    applySettings(mainConfig);
});
```

## Function Reference

| Function | Signature | Description |
|----------|-----------|-------------|
| `suppress` | `(block: fn(): void): void` | Run a block, silently swallowing any thrown error |
| `redirectStdout` | `(block: fn(): void): void` | Run a block with stdout redirection (VM runtime hook) |
| `closing` | `<T>(resource: T, block: fn(): void): void` | Run a block, then call `resource.close()` automatically |
