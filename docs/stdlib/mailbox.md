# Mailbox

The `tt.mail.Mailbox` module provides a Python `mailbox` analog for reading and writing mailbox formats: the `Mailbox` base class, `Message`, and format-specific implementations `Mbox`, `Maildir`, `MH`, `Babyl`, and `MMDF`.

## Import

```titrate
import tt::mail::Mailbox;
```

## Class: Message

A single email message with headers and a body.

**Fields:**
- `headers: HashMap<string, string>`
- `body: string`
- `fromLine: string` — used by `mbox` for the leading `"From "` line

**Methods:**
- `init()`
- `get(name: string): string` — case-insensitive header lookup; `""` if absent
- `set(name: string, value: string): void` — set a header value
- `keys(): ArrayList<string>` — all header names
- `setBody(body: string): void`
- `asString(): string` — serialize to text (headers + blank line + body)

```titrate
let m: Message = new Message();
m.set("From", "alice@example.com");
m.set("Subject", "Hello");
m.setBody("Hi there.");
io::println(m.asString());
```

## Class: Mailbox

Base class for mailbox formats. Subclasses implement `load` and `flush`.

**Fields:**
- `path: string`
- `messages: ArrayList<Message>`

**Methods:**
- `init(path: string)`
- `add(msg: Message): void`
- `remove(index: int): void`
- `get(index: int): Message` — `null` if out of range
- `size(): int`
- `load(): void` — base no-op; overridden in subclasses
- `flush(): void` — base no-op; overridden in subclasses
- `keys(): ArrayList<int>` — iterate message indices

## Subclasses

### Mbox

Unix mbox format: messages separated by `"From "` lines. Loads from a single file on construction; `flush()` writes all messages back to that file.

**Constructor:** `Mbox(path: string)`

### Maildir

Qmail Maildir format: `cur/`, `new/`, `tmp/` directories. Loads messages from `new/` and `cur/`; `flush()` is a no-op (Maildir writes are per-message).

**Constructor:** `Maildir(path: string)`

### MH

MH format: one file per message in a numbered sequence. Loads numbered files from the directory; `flush()` writes each message back to `path/N`.

**Constructor:** `MH(path: string)`

### Babyl

Babyl format (Rmail): messages separated by form-feed (`\f`) markers.

**Constructor:** `Babyl(path: string)`

### MMDF

MMDF format: messages separated by lines of four `^A` (`\x01\x01\x01\x01`) characters.

**Constructor:** `MMDF(path: string)`

```titrate
let box: Mbox = new Mbox("inbox.mbox");
var i: int = 0;
while (i < box.size()) {
    let m: Message = box.get(i);
    io::println(m.get("Subject"));
    i = i + 1;
}
```
