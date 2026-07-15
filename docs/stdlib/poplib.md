# PopLib

The `tt.net.Poplib` module mirrors Python's `poplib` module. It provides a `POP3` client implementing RFC 1939 over TCP (optionally TLS), with `USER`/`PASS` and `APOP` authentication, mailbox status, message listing and retrieval, deletion, top-bytes preview, unique-id listing, and `QUIT`.

## Import

```titrate
import tt::net::Poplib;
```

## Constants — POP3 status indicators

- `POP_OK: string = "+OK"`
- `POP_ERR: string = "-ERR"`

## PopResponse

A response from the POP3 server, splitting the status from the body lines.

- `PopResponse.init()` — fields default to empty
- `ok: bool` — `true` if the status line began with `+OK`
- `status: string` — `+OK` or `-ERR`
- `message: string` — the first line after the status word
- `lines: ArrayList<string>` — multi-line payload (for `RETR`, `TOP`, multi-`LIST`, multi-`UIDL`)
- `toString(): string`

## PopStat

A mailbox status result (number of messages and total size).

- `PopStat.init(count: int, size: long)`
- `count: int` — number of messages
- `size: long` — total size in octets
- `toString(): string`

## PopListEntry

A message listing entry.

- `PopListEntry.init(number: int, size: long)`
- `number: int` — message sequence number
- `size: long` — message size in octets
- `toString(): string`

## POP3

`POP3` is the client class.

- `POP3.init(host: string, port: int)` — TLS is auto-enabled when `port == 995`
- `host: string` / `port: int` / `useTls: bool` / `debug: bool`

### Connection

- `open(): string` — open the connection and read the server greeting (returned to caller)
- `close(): void` — close the transport without sending `QUIT` (rolls back deletions)
- `quit(): PopResponse` — send `QUIT` (committing deletions) and close the transport
- `isConnected(): bool`
- `getWelcome(): string`

### Authentication

- `user(username: string): PopResponse` — send the `USER` command identifying the mailbox
- `pass(password: string): PopResponse` — send the `PASS` command with the cleartext password
- `login(username: string, password: string): PopResponse` — convenience: `USER` followed by `PASS`
- `apop(username: string, password: string): PopResponse` — authenticated login using `APOP` (MD5 digest of timestamp+password); throws if the server did not send an APOP timestamp

### Mailbox status

- `stat(): PopStat` — get the mailbox status: number of messages and total size
- `list(which: int): ArrayList<PopListEntry>` — list message sizes; pass `which = -1` to list all messages

### Retrieval

- `retr(which: int): ArrayList<string>` — retrieve a message by number, returning its raw lines
- `retrAsString(which: int): string` — retrieve a message body as a single string with newlines
- `top(which: int, numLines: int): ArrayList<string>` — get headers and the first `numLines` of body without marking as seen

### Mutating messages

- `dele(which: int): PopResponse` — mark a message for deletion
- `rset(): PopResponse` — reset all marked deletions
- `noop(): PopResponse` — keepalive

### Unique IDs

- `uidl(which: int): HashMap<int, string>` — get a unique identifier for a message (or all messages if `which == -1`)

## Module functions

### withPop3

Run a callback inside a fresh `POP3` session that auto-quits.

- Parameters
  - `host: string`
  - `port: int`
  - `username: string`
  - `password: string`
  - `callback: fn(POP3): void`
- Returns: `void`

```titrate
withPop3("pop.example.com", 995, "alice", "secret", fn(client: POP3): void {
    let stat: PopStat = client.stat();
    io::println("Inbox has " + Integer.toString(stat.count) + " messages");
    let entries: ArrayList<PopListEntry> = client.list(-1);
    var i: int = 0;
    while (i < entries.size()) {
        io::println(Integer.toString(entries.get(i).number) + ": " + Long.toString(entries.get(i).size));
        i = i + 1;
    }
});
```

## Notes

- All network operations go through native functions `Pop3_connect`, `Pop3_readLine`, `Pop3_write`, `Pop3_close`, and `Pop3_md5Hex`.
- The internal `sendCommand` automatically detects multi-line commands (`RETR`, `TOP`, bare `LIST`, bare `UIDL`) and reads the dot-terminated payload, de-stuffing leading dots per RFC 1939.
- `LIST n` and `UIDL n` (with a numeric argument) are single-line responses and are parsed from the first response line.
- `apop` requires the server greeting to contain a `<...@...>` timestamp; otherwise it throws.
