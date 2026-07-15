# ImapLib

The `tt.net.Imaplib` module mirrors Python's `imaplib` module. It provides an `IMAP4` client that connects to a mail server over TCP (optionally TLS), authenticates, lists and selects mailboxes, searches for messages, fetches message bodies, mutates flags with `STORE`, and cleanly logs out via `LOGOUT`.

## Import

```titrate
import tt::net::Imaplib;
```

## Constants — IMAP response codes

- `OK: string = "OK"`
- `NO: string = "NO"`
- `BAD: string = "BAD"`

## Constants — mailbox flags

- `FLAG_SEEN: string = "\\Seen"`
- `FLAG_ANSWERED: string = "\\Answered"`
- `FLAG_FLAGGED: string = "\\Flagged"`
- `FLAG_DELETED: string = "\\Deleted"`
- `FLAG_DRAFT: string = "\\Draft"`
- `FLAG_RECENT: string = "\\Recent"`

## Constants — search keywords

- `SEARCH_ALL: string = "ALL"`
- `SEARCH_UNSEEN: string = "UNSEEN"`
- `SEARCH_SEEN: string = "SEEN"`
- `SEARCH_ANSWERED: string = "ANSWERED"`
- `SEARCH_DELETED: string = "DELETED"`
- `SEARCH_FLAGGED: string = "FLAGGED"`
- `SEARCH_RECENT: string = "RECENT"`

## ImapResponse

Result of an IMAP command containing the status tag, the human-readable line, and parsed data.

- `ImapResponse.init()` — fields default to empty
- `status: string` — `OK`, `NO`, or `BAD`
- `line: string` — human-readable response line
- `data: ArrayList<string>` — untagged response and continuation lines
- `tag: string` — the command tag this response answers
- `isOk(): bool` — `true` if `status == OK`
- `toString(): string`

## MailboxInfo

A mailbox selection result returned by `select()` and `examine()`.

- `MailboxInfo.init()` — fields default to empty
- `mailbox: string`
- `exists: int` — number of messages in the mailbox
- `recent: int` — number of recent messages
- `uidValidity: long` — UIDVALIDITY value sent by the server
- `flags: ArrayList<string>` — list of `\`-prefixed flags
- `toString(): string`

## FetchedMessage

A fetched message envelope/section returned by `fetch()` and `fetchRfc822()`.

- `FetchedMessage.init()` — fields default to empty
- `uid: string`
- `messageId: string`
- `flags: string` — raw flag list from the FETCH response
- `body: string` — message body or section
- `headers: HashMap<string, string>` — parsed headers
- `toString(): string`

## IMAP4

`IMAP4` is the client class. The connection state moves `LOGOUT → NONAUTHENTICATED → AUTHENTICATED → SELECTED → AUTHENTICATED → LOGOUT`.

- `IMAP4.init(host: string, port: int)` — TLS is auto-enabled when `port == 993`
- `host: string` / `port: int` / `useTls: bool` / `debug: bool` / `state: string`

### Connection

- `open(): ImapResponse` — open the connection and read the server greeting
- `close(): ImapResponse` — close the selected mailbox (deselects, expunges flagged messages)
- `logout(): ImapResponse` — send `LOGOUT` and close the transport
- `noop(): ImapResponse` — send a `NOOP` keepalive
- `check(): ImapResponse` — send a `CHECK` checkpoint
- `isConnected(): bool`
- `getState(): string`
- `getCurrentMailbox(): string`

### Authentication

- `login(username: string, password: string): ImapResponse` — plain login; state moves to `AUTHENTICATED` on success
- `authenticate(mechanism: string, initialResponse: string): ImapResponse` — SASL authentication

### Mailbox management

- `list(reference: string, pattern: string): ImapResponse`
- `subscribe(mailbox: string): ImapResponse`
- `unsubscribe(mailbox: string): ImapResponse`
- `select(mailbox: string): MailboxInfo` — select a mailbox; state moves to `SELECTED` on success; returns counts and flags
- `examine(mailbox: string): MailboxInfo` — examine a mailbox without selecting it for modification
- `create(mailbox: string): ImapResponse`
- `delete(mailbox: string): ImapResponse`
- `rename(oldName: string, newName: string): ImapResponse`

### Searching and fetching

- `search(charset: string, criteria: ArrayList<string>): ArrayList<string>` — search the selected mailbox; returns message sequence numbers
- `fetch(sequenceSet: string, items: string): ArrayList<FetchedMessage>` — fetch one or more message items by sequence set
- `fetchRfc822(messageId: string): FetchedMessage` — fetch the full RFC822 body for a single message id

### Mutating messages

- `store(sequenceSet: string, action: string, flags: ArrayList<string>): ImapResponse` — alter flags; `action` is `+FLAGS`, `-FLAGS`, or `FLAGS`
- `expunge(): ImapResponse` — permanently remove messages flagged as `\Deleted`
- `copy(sequenceSet: string, mailbox: string): ImapResponse` — copy messages from the selected mailbox to another
- `append(mailbox: string, flags: ArrayList<string>, datetime: string, message: string): ImapResponse` — append a message to a mailbox

### Low-level

- `execute(command: string): ImapResponse` — send a single tagged command and return the final response

## Module functions

### withImap

Run a callback inside a fresh `IMAP4` session that auto-logs out.

- Parameters
  - `host: string` — server host
  - `port: int` — server port
  - `username: string`
  - `password: string`
  - `callback: fn(IMAP4): void`
- Returns: `void`

```titrate
withImap("imap.example.com", 993, "alice", "secret", fn(client: IMAP4): void {
    let info: MailboxInfo = client.select("INBOX");
    io::println("INBOX has " + Integer.toString(info.exists) + " messages");
});
```

### flagsOf

Pass through a list of flag strings (helper for building flag arrays).

- Parameters
  - `flagList: ArrayList<string>`
- Returns: `ArrayList<string>` — same list

## Notes

- All network operations go through native functions `Imap_connect`, `Imap_readLine`, `Imap_write`, and `Imap_close`.
- The `execute` method assigns sequential tags of the form `A0001`, `A0002`, etc.
- Untagged response lines (those starting with `* `) are collected into `ImapResponse.data`.
- `select()` parses `EXISTS`, `RECENT`, `FLAGS`, and `UIDVALIDITY` from the untagged data; `examine()` parses only `EXISTS` and `RECENT`.
- `append()` first sends the APPEND command, then — if the server returns a continuation request — sends the literal message body and reads the final response.
