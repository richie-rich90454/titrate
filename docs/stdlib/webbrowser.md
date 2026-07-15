# WebBrowser

The `tt.net.Webbrowser` module provides primitives for displaying HTML documents to a user by launching the platform default browser. It mirrors Python's `webbrowser` module, exposing `open`, `open_new`, `open_new_tab`, plus `register`/`get` for custom browser controllers. Platform defaults (`cmd /c start` on Windows, `xdg-open` on Linux) are registered automatically.

## Import

```titrate
import tt::net::Webbrowser;
```

## Classes

### GenericBrowser

A browser controller that invokes a configured platform command to open URLs.

**Fields:**
- `name: string` — browser type name
- `command: ArrayList<string>` — argv-style command list; an empty string or `"%s"` element is replaced with the quoted URL
- `background: bool` — whether the browser launches in the background

**Constructors:**
- `GenericBrowser(name: string, command: ArrayList<string>)`

**Methods:**
- `open(url: string, new: int): bool` — open `url`; `new` controls window behavior (`0` = same, `1` = new window, `2` = new tab). Returns `true` on success.
- `open_new(url: string): bool` — equivalent to `open(url, 1)`
- `open_new_tab(url: string): bool` — equivalent to `open(url, 2)`
- `getName(): string` — return the browser type name

```titrate
let cmd = new ArrayList<string>();
cmd.add("xdg-open");
let browser = new GenericBrowser("xdg-open", cmd);
let ok: bool = browser.open("https://example.com", 0);
```

## Functions

### open

- `Webbrowser.open(url: string, new: int, autoraise: bool): bool` — open `url` in the default browser. Returns `true` on success.

```titrate
Webbrowser.open("https://example.com", 0, true);
```

### open_new

- `Webbrowser.open_new(url: string): bool` — open `url` in a new browser window

### open_new_tab

- `Webbrowser.open_new_tab(url: string): bool` — open `url` in a new browser tab

### get

- `Webbrowser.get(name: string): GenericBrowser` — return the registered browser controller for `name`, or the default browser if `name` is empty. Returns `null` if no controller matches.

```titrate
let browser: GenericBrowser = Webbrowser.get("xdg-open");
if (browser != null) {
    browser.open_new("https://example.com");
}
```

### register

- `Webbrowser.register(name: string, controller: GenericBrowser, preferred: bool): void` — register a browser controller under `name`. If `preferred` is `true` (or no default has been set yet), the controller becomes the default.

```titrate
let cmd = new ArrayList<string>();
cmd.add("firefox");
let ff = new GenericBrowser("firefox", cmd);
Webbrowser.register("firefox", ff, false);
```

## Usage Example

```titrate
import tt::net::Webbrowser;

public fn main(): void {
    if (!Webbrowser.open("https://example.com", 0, true)) {
        io::println("Failed to open browser");
    }
    Webbrowser.open_new_tab("https://example.com/docs");
}
```
