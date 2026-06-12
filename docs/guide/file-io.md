# File I/O

Reading and writing files is one of the first things you'll need when building real programs — whether you're loading configuration, processing data, or saving results. Titrate makes file operations straightforward with the `File` class and built-in I/O functions, using `Result` types so you never forget to handle errors.

## Reading a File

`File.readFile` reads the entire contents of a file as a string. It returns a `Result<string, string>` to handle potential errors:

```titrate
public fn main(): void {
    switch File.readFile("data.txt") {
        case Ok(content) => io::println(content);
        case Err(e) => io::println("Failed to read: " + e);
    }
}
```

::: tip Why `Result` instead of exceptions?
Titrate uses `Result` types for file operations because it forces you to think about what happens when things go wrong. No more surprise crashes because a file was missing — the type system reminds you to handle both cases.
:::

## Writing a File

`File.writeFile` writes a string to a file, creating it if it does not exist or overwriting it if it does. It returns `Result<void, string>`:

```titrate
public fn main(): void {
    switch File.writeFile("output.txt", "Hello, file!") {
        case Ok(_) => io::println("Written successfully");
        case Err(e) => io::println("Failed to write: " + e);
    }
}
```

## Reading Lines

`File.readLines` reads a file and returns an array of strings, one element per line:

```titrate
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
public fn main(): void {
    let lines = File.readLines("data.csv");
    for (line in lines) {
        let fields = String.split(line, ",");
        io::println(fields[0]);
    }
}
```

## Common File I/O Patterns

### Reading with Error Recovery

When a file might not exist, you can provide a fallback value:

```titrate
fn readConfig(path: string): string {
    switch File.readFile(path) {
        case Ok(content) => return content;
        case Err(_) => return "";
    }
}

let config = readConfig("config.txt");
if (String.length(config) == 0) {
    io::println("Using default configuration");
}
```

### Processing a File Line by Line

A common pattern: read lines, parse each one, and collect results:

```titrate
public fn main(): void {
    let lines = File.readLines("scores.txt");
    var total: int = 0;
    var count: int = 0;
    for (line in lines) {
        let score = Integer.parseInt(line);
        switch score {
            case Ok(n) => {
                total = total + n;
                count = count + 1;
            }
            case Err(_) => {
                io::println("Skipping invalid line: " + line);
            }
        }
    }
    if (count > 0) {
        let avg: int = total / count;
        io::println("Average score: " + Integer.toString(avg));
    }
}
```

### Writing Multiple Lines

Build up content in memory, then write it all at once:

```titrate
public fn main(): void {
    var output: string = "";
    for (i in 1..=10) {
        output = output + "Line " + Integer.toString(i) + "\n";
    }
    switch File.writeFile("output.txt", output) {
        case Ok(_) => io::println("File written");
        case Err(e) => io::println("Error: " + e);
    }
}
```

### Chaining File Operations

Read a file, transform it, and write the result — all with proper error handling:

```titrate
fn processFile(inputPath: string, outputPath: string): void {
    switch File.readFile(inputPath) {
        case Ok(content) => {
            let upper = String.toUpperCase(content);
            switch File.writeFile(outputPath, upper) {
                case Ok(_) => io::println("Processed successfully");
                case Err(e) => io::println("Write error: " + e);
            }
        }
        case Err(e) => {
            io::println("Read error: " + e);
        }
    }
}
```

## Print Functions

The `tt::io` module provides standalone print functions available without import:

- `io::println(s: string): void` — print a string followed by a newline
- `io::print(s: string): void` — print a string without a trailing newline

```titrate
public fn main(): void {
    io::print("Loading...");
    io::println("done");
}
```

## Working Directory

Relative paths in file operations are resolved against the **current working directory** of the process. When running with `pipette run`, the working directory is the project root (where `Titrate.toml` lives).

```titrate
// Resolves to <project_root>/data/input.txt
let content = File.readFile("data/input.txt");
```

For predictable behavior, prefer using relative paths from the project root rather than absolute paths.

## Try It Yourself

Write a program that reads a file containing one number per line, doubles each number, and writes the results to a new file.

For example, if `numbers.txt` contains:
```
3
7
15
```

Then `doubled.txt` should contain:
```
6
14
30
```

```titrate
public fn main(): void {
    // Read numbers.txt
    // Double each number
    // Write results to doubled.txt
    // Handle errors gracefully!
}
```

<details>
<summary>Show solution</summary>

```titrate
public fn main(): void {
    switch File.readFile("numbers.txt") {
        case Ok(content) => {
            let lines = String.split(content, "\n");
            var output: string = "";
            for (line in lines) {
                if (String.length(line) > 0) {
                    let parsed = Integer.parseInt(line);
                    switch parsed {
                        case Ok(n) => {
                            output = output + Integer.toString(n * 2) + "\n";
                        }
                        case Err(_) => {
                            io::println("Skipping invalid line: " + line);
                        }
                    }
                }
            }
            switch File.writeFile("doubled.txt", output) {
                case Ok(_) => io::println("Wrote doubled.txt");
                case Err(e) => io::println("Write error: " + e);
            }
        }
        case Err(e) => {
            io::println("Read error: " + e);
        }
    }
}
```

</details>
