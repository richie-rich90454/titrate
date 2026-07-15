# Email

The `tt.mail.Email` module mirrors Python's `email` package. It provides `Message`, `MimeMessage`, `MimeText`, `MimeBase`, and `MimeMultipart` classes for building RFC 2822 / MIME messages, plus header and address parsing helpers.

## Import

```titrate
import tt::mail::Email;
```

## Errors

### MessageError

Raised on malformed messages or invalid operations.

- `MessageError.init(msg: string)`
- `message: string`
- `toString(): string` — returns `"MessageError: <message>"`

## Header

`Header` represents a single RFC 2822 header field with optional parameters (used for `Content-Type` and `Content-Disposition`).

- `Header.init(name: string, value: string)`
- `name: string`
- `value: string`
- `params: HashMap<string, string>`
- `setParam(key: string, val: string): void`
- `getParam(key: string): string`
- `hasParam(key: string): bool`
- `toString(): string` — renders `Name: value; key="value"` form

## Message

`Message` is the base class for all email message parts. It holds an ordered list of headers, a text payload, and (for multipart messages) a list of sub-parts.

- `Message.init()`
- `setHeader(name: string, value: string): void` — replace or add a header
- `addHeader(name: string, value: string): void` — append a header (allowing duplicates)
- `getHeader(name: string): string` — first value, or `""`
- `getAllHeaders(name: string): ArrayList<string>` — all values for the given name
- `getHeaders(): ArrayList<Header>`
- `delHeader(name: string): bool` — remove the first matching header
- `setPayload(payload: string): void`
- `getPayload(): string`
- `setCharset(charset: string): void`
- `getCharset(): string`
- `attach(part: Message): void` — add a sub-part (makes the message multipart)
- `getParts(): ArrayList<Message>`
- `isMultipart(): bool`
- `setParam(name: string, value: string): void` — set a Content-Type parameter
- `getParam(name: string): string`
- `asString(): string` — render the message (and sub-parts) as RFC 2822 text
- `toString(): string`

## MimeBase

`MimeBase` is the base class for MIME parts. It adds a `maintype`/`subtype` pair and the `MIME-Version` header.

- `MimeBase.init(maintype: string, subtype: string)` — sets `Content-Type: <maintype>/<subtype>` and `MIME-Version: 1.0`
- `maintype: string`
- `subtype: string`
- `setContentType(maintype: string, subtype: string): void`
- `encodeBase64(): void` — base64-encode the payload and set `Content-Transfer-Encoding: base64`
- `encodeQuotedPrintable(): void` — quoted-printable-encode the payload

## MimeText

`MimeText` is a simple text part with maintype `text`.

- `MimeText.init(text: string)` — subtype defaults to `"plain"`, charset `utf-8`
- `initWithSubtype(text: string, subtype: string): void` — e.g. `"html"`

## MimeMultipart

`MimeMultipart` is a container for multiple MIME parts, separated by a boundary string.

- `MimeMultipart.init()` — subtype defaults to `"mixed"`
- `initWithSubtype(subtype: string): void` — e.g. `"alternative"`
- `boundary: string`
- `asString(): string` — render with `--<boundary>` delimiters between parts

## MimeMessage

`MimeMessage` is a top-level email message with standard headers.

- `MimeMessage.init()`
- `setFrom(addr: string): void` / `getFrom(): string`
- `setTo(addresses: ArrayList<string>): void` / `getTo(): string`
- `setCc(addresses: ArrayList<string>): void` / `getCc(): string`
- `setBcc(addresses: ArrayList<string>): void`
- `setSubject(subject: string): void` / `getSubject(): string`
- `setDate(dateStr: string): void` / `getDate(): string`
- `setBody(body: MimeBase): void` — attach a single part or copy the multipart's parts and `Content-Type`

```titrate
let m: MimeMessage = new MimeMessage();
m.setFrom("alice@example.com");
let to: ArrayList<string> = new ArrayList<string>();
to.add("bob@example.com");
m.setTo(to);
m.setSubject("Hello");
let body: MimeText = new MimeText("Hi from Titrate!");
m.setBody(body);
io::println(m.asString());
```

## Functions

### parseHeaderLine

Parse a single `Name: value; param=...` header line into a `Header`.

**Parameters:** `line: string`
**Returns:** `Header`

### parseHeaderParams

Parse `key=value; key=value` parameters into the given `Header`.

**Parameters:** `s: string`, `h: Header`
**Returns:** `void`

### parseHeaders

Parse a block of headers (separated by `\r\n` or `\n`) into a list of `Header` objects, handling header folding (continuation lines starting with whitespace).

**Parameters:** `raw: string`
**Returns:** `ArrayList<Header>`

### formatAddress

Format an address as `"Name <addr@example.com>"` if a display name is given, otherwise return the bare address.

**Parameters:** `displayName: string`, `addr: string`
**Returns:** `string`

### parseAddress

Parse a single address string `"Name <addr>"` or `"addr"` into a `(name, addr)` pair returned as a 2-element `ArrayList<string>`.

**Parameters:** `s: string`
**Returns:** `ArrayList<string>`

### parseAddressList

Parse a comma-separated list of addresses into a list of `(name, addr)` pairs.

**Parameters:** `s: string`
**Returns:** `ArrayList<ArrayList<string>>`

### joinAddresses

Join a list of addresses into a single comma-separated string.

**Parameters:** `addresses: ArrayList<string>`
**Returns:** `string`

### messageFromString

Parse a raw RFC 2822 message string into a `Message` object. Headers are parsed and the remaining text becomes the payload.

**Parameters:** `raw: string`
**Returns:** `Message`

### encodeQuotedPrintable

Encode a string using quoted-printable encoding (RFC 2045). Printable ASCII (33-126 except `=`) and tabs/spaces are passed through; all other bytes become `=XX` hex escapes. Lines are wrapped at 76 characters.

**Parameters:** `s: string`
**Returns:** `string`

### decodeQuotedPrintable

Decode a quoted-printable string back to its original form.

**Parameters:** `s: string`
**Returns:** `string`

## Convenience builders

### messageFromText

Build a simple `text/plain` message.

**Parameters:** `from: string`, `to: string`, `subject: string`, `body: string`
**Returns:** `MimeMessage`

```titrate
let m: MimeMessage = messageFromText(
    "alice@example.com",
    "bob@example.com",
    "Hello",
    "Hi from Titrate!"
);
```

### messageFromHtml

Build a `multipart/alternative` message with plain and HTML bodies.

**Parameters:** `from: string`, `to: string`, `subject: string`, `plainBody: string`, `htmlBody: string`
**Returns:** `MimeMessage`

### messageWithAttachment

Build a `multipart/mixed` message with a text body and a base64-encoded attachment.

**Parameters:** `from: string`, `to: string`, `subject: string`, `body: string`, `filename: string`, `attachmentData: string`
**Returns:** `MimeMessage`

## Notes

- Headers are stored in insertion order; `setHeader` replaces the first matching header while `addHeader` appends.
- A `Message` becomes multipart as soon as `attach()` is called.
- `MimeMultipart.asString()` renders the headers, then each part preceded by `--<boundary>` and a final `--<boundary>--`.
- Header folding (continuation lines starting with whitespace) is honored when parsing.
