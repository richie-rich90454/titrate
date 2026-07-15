# TelnetLib

The `tt.net.Telnetlib` module mirrors Python's `telnetlib` module. It provides a `Telnet` client implementing RFC 854 negotiation and streaming reads, with `read_until`, `expect`, and `interact` helpers.

## Import

```titrate
import tt::net::Telnetlib;
```

## Constants — Telnet commands (RFC 854 / RFC 855)

- `IAC: int = 255` — interpret as command
- `DONT: int = 254`
- `DO: int = 253`
- `WONT: int = 252`
- `WILL: int = 251`
- `SB: int = 250` — subnegotiation begin
- `GA: int = 249` — go ahead
- `EL: int = 248` — erase line
- `EC: int = 247` — erase character
- `AYT: int = 246` — are you there
- `AO: int = 245` — abort output
- `IP: int = 244` — interrupt process
- `BREAK: int = 243`
- `DM: int = 242` — data mark
- `NOP: int = 241` — no operation
- `SE: int = 240` — subnegotiation end
- `EOR: int = 239` — end of record

## Constants — common Telnet options

- `OPT_BINARY: int = 0`
- `OPT_ECHO: int = 1`
- `OPT_SGA: int = 3` — suppress go ahead
- `OPT_TTYPE: int = 24` — terminal type
- `OPT_NAWS: int = 31` — negotiate about window size
- `OPT_TSPEED: int = 32` — terminal speed
- `OPT_LINEMODE: int = 34`
- `OPT_ENVIRON: int = 36`
- `OPT_NEW_ENVIRON: int = 39`

## Errors

### TelnetEof

Raised when EOF is encountered on a Telnet read.

- `TelnetEof.init(message: string)` — `message` is set to `"TelnetEof: <message>"`
- `toString(): string`

### OptionRefused

Raised when a negotiation option is refused.

- `OptionRefused.init(option: string)` — `message` is set to `"OptionRefused: <option>"`
- `toString(): string`

## ExpectMatch

A single matched `expect` result.

- `ExpectMatch.init(index: int, match: string, text: string)`
- `index: int` — index of the matched pattern, or `-1` on EOF
- `match: string` — the matched pattern
- `text: string` — the full text accumulated up to the match
- `toString(): string`

## Telnet

`Telnet` is the client class implementing RFC 854 negotiation and streaming reads.

- `Telnet.init(host: string, port: int)`
- `host: string` / `port: int` / `timeout: int` / `connected: bool` / `debug: bool`

### Connection

- `open(): void` — open the connection (throws on failure)
- `setTimeout(ms: int): void` — set the socket timeout in milliseconds
- `close(): void` — close the connection and release the transport
- `eofReached(): bool` — whether EOF has been reached

### Writing

- `write(data: string): void` — write raw data to the transport
- `writeLine(line: string): void` — write a line followed by `\r\n`

### Reading

- `readUntil(terminator: string): string` — read until the terminator is found, EOF, or timeout; returns the bytes read including terminator
- `readAll(): string` — read all available data until EOF, returning the assembled text
- `readSome(maxBytes: int): string` — read up to `maxBytes` without blocking semantics

```titrate
let t: Telnet = new Telnet("example.com", 23);
t.open();
let banner: string = t.readUntil("login: ");
io::println(banner);
t.writeLine("alice");
t.close();
```

### Expect

- `expect(patterns: ArrayList<string>): ExpectMatch` — wait until one of the listed substrings appears in the stream; returns an `ExpectMatch` with `index = -1` on EOF

### Negotiation

- `setOptionHandler(handler: fn(int, int): bool): void` — register a handler invoked for each negotiation option pair `(command, option)`; the handler returns `true` to accept, `false` to refuse
- `sendNegotiation(command: int, option: int): void` — send a Telnet negotiation command (`IAC + command + option`)
- `areYouThere(): string` — send an Are You There probe and read until `"yes"`

### Interact

- `interact(inputFn: fn(): string, outputFn: fn(string): void): void` — interactive loop reading from `inputFn` and writing to the transport; the loop exits when `inputFn` returns `"quit"` or `"exit"`

## Notes

- The byte stream is processed by an internal `processInput` helper that strips `IAC` negotiation sequences, including escaped `IAC` bytes (`IAC IAC` → `IAC`), `WILL`/`WONT`/`DO`/`DONT` option triples, and `SB ... SE` subnegotiation blocks.
- All network operations go through native functions `Telnet_connect`, `Telnet_read`, `Telnet_write`, `Telnet_setTimeout`, and `Telnet_close`.
- `readUntil` reads one byte at a time so it can stop as soon as the terminator appears.
- The `interact` loop calls `inputFn` for input and `outputFn` for output; tests typically replace these with stub functions.
