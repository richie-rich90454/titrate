# Netrc

The `tt.net.Netrc` module parses and renders `.netrc` files used to store credentials for FTP/HTTP clients. It mirrors Python's `netrc` module, exposing the `Netrc` class with `hosts`/`macros`/`authenticators` accessors, a `parse` method, and a `toText` serializer. The `NetrcAuthenticator` class holds the `(login, password, account)` tuple for a host.

## Import

```titrate
import tt::net::Netrc;
```

## Classes

### NetrcAuthenticator

Holds the credentials for a single host entry.

**Fields:**
- `login: string` — user name
- `password: string` — password
- `account: string` — account (rarely used)

**Constructors:**
- `init(login: string, password: string, account: string)`

**Methods:**
- `toString(): string` — returns `"Authenticator(login=<login>)"`

### Netrc

Parses a `.netrc` file and exposes hosts, the default authenticator, and macros.

**Constructors:**
- `init()` — creates an empty Netrc; call `parse(content)` to populate

**Methods:**
- `hosts(): HashMap<string, NetrcAuthenticator>` — return the map of host name to authenticator
- `defaultAuthenticator(): NetrcAuthenticator` — return the default authenticator (for hosts without an explicit entry), or `null`
- `macros(): HashMap<string, string>` — return the map of macro name to macro body
- `authenticators(host: string): NetrcAuthenticator` — look up the authenticator for `host`, falling back to the default entry if no explicit entry exists
- `parse(content: string): void` — parse the textual contents of a `.netrc` file into this object. Recognizes the `machine`, `default`, `login`, `password`, `account`, and `macdef` tokens; `#` introduces a comment to end of line.
- `toText(): string` — render this Netrc back into `.netrc` text format

```titrate
let content = "machine example.com\n    login alice\n    password secret\n";
let n: Netrc = new Netrc();
n.parse(content);
let auth: NetrcAuthenticator = n.authenticators("example.com");
io::println(auth.login);  // "alice"
```

## Functions

### netrc

- `Netrc.netrc(content: string): Netrc` — module-level convenience: parse a `.netrc` file's text into a new `Netrc` object

```titrate
let n: Netrc = Netrc.netrc("machine ftp.example.com login bob password hunter2");
```

## Usage Example

```titrate
import tt::net::Netrc;
import tt::io::File;

public fn main(): void {
    let content: string = "machine api.example.com\n    login alice\n    password hunter2\n";
    let n: Netrc = Netrc.netrc(content);
    let auth: NetrcAuthenticator = n.authenticators("api.example.com");
    if (auth != null) {
        io::println("Login as " + auth.login);
    }
    io::println("Hosts: " + Integer.toString(n.hosts().size()));
}
```
