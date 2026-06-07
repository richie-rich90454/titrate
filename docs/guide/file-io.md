# File I/O

Titrate provides file operations through the `tt::io` module. All file functions use the `File` class and standalone I/O functions.

## Reading a File

`File.readFile` reads the entire contents of a file as a string. It returns a `Result<string>` to handle potential errors:

```titrate
import tt::io::File;

public fn main(): void {
    match File.readFile("data.txt") {
        Ok(content) => io::println(content),
        Err(e) => io::println("Failed to read: " + e),
    }
}
```

## Writing a File

`File.writeFile` writes a string to a file, creating it if it does not exist or overwriting it if it does. It returns `Result<void>`:

```titrate
import tt::io::File;

public fn main(): void {
    match File.writeFile("output.txt", "Hello, file!") {
        Ok(_) => io::println("Written successfully"),
        Err(e) => io::println("Failed to write: " + e),
    }
}
```

## Reading Lines

`File.readLines` reads a file and returns an `array&lt;string&gt;`, one element per line:

```titrate
import tt::io::File;

public fn main(): void {
    let lines = File.readLines("data.txt");
    for (line in lines) {
        io::println(line);
    }
}
```

## Splitting Strings

Use `String.split` to break a string into an array on a delimiter:

```titrate
let csv = "one,two,three";
let parts = String.split(csv, ",");
// parts is ["one", "two", "three"]
```

This is especially useful when processing line-based file input:

```titrate
import tt::io::File;

public fn main(): void {
    let lines = File.readLines("data.csv");
    for (line in lines) {
        let fields = String.split(line, ",");
        io::println(fields[0]);
    }
}
```

## Print Functions

The `tt::io` module also provides standalone print functions:

- `println(s: string): void` — print a string followed by a newline
- `print(s: string): void` — print a string without a trailing newline

```titrate
import tt::io::{println, print};

public fn main(): void {
    print("Loading...");
    println("done");
}
```

## Working Directory

Relative paths in file operations are resolved against the **current working directory** of the process. When running with `pipette run`, the working directory is the project root (where `Titrate.toml` lives).

```titrate
// Resolves to <project_root>/data/input.txt
let content = File.readFile("data/input.txt");
```

For predictable behavior, prefer using relative paths from the project root rather than absolute paths.
