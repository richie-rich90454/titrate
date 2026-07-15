# GetPass

The `tt.io.Getpass` module provides secure password prompts and login-name lookup. It mirrors Python's `getpass` module, exposing `getpass` (which prompts to stderr and reads a line without local echo when a TTY is available) and `getuser` (which returns the current login name from environment variables).

## Import

```titrate
import tt::io::Getpass;
```

## Functions

### getpass

- `Getpass.getpass(prompt: string): string` — prompt for a password without echoing it. `prompt` is written to stderr before reading; if `prompt` is empty, `"Password: "` is used. On platforms without a raw-mode TTY native, this falls back to reading a line from stdin (which may echo). Returns the entered line, or `""` if input is unavailable.

```titrate
let password: string = Getpass.getpass("Enter password: ");
io::println("Received " + Integer.toString(String.length(password)) + " characters");
```

### getuser

- `Getpass.getuser(): string` — return the current user's login name by checking the `LOGNAME`, `USER`, `LNAME`, and `USERNAME` environment variables in order. Returns `"unknown"` if none are set.

```titrate
let user: string = Getpass.getuser();
io::println("Hello, " + user);
```

## Usage Example

```titrate
import tt::io::Getpass;

public fn main(): void {
    let user: string = Getpass.getuser();
    io::println("Signing in as " + user);
    let password: string = Getpass.getpass("Password for " + user + ": ");
    if (String.length(password) == 0) {
        io::println("Empty password, aborting");
        return;
    }
    io::println("Authenticated");
}
```
