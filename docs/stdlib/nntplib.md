# NntpLib

The `tt.net.Nntplib` module mirrors Python's `nntplib` module. It provides an `NNTP` client implementing RFC 3977 over TCP (optionally TLS), with `AUTHINFO` authentication, `GROUP` selection, `XOVER` overview retrieval, `ARTICLE`/`HEAD`/`BODY` retrieval, `POST` of new articles, and `QUIT`.

## Import

```titrate
import tt::net::Nntplib;
```

## Constants — NNTP response codes (RFC 3977 / RFC 977)

- `NNTP_INFO_HELP: int = 100`
- `NNTP_INFO_VERSION: int = 101`
- `NNTP_INFO_CAPABILITIES: int = 101`
- `NNTP_POSTING_ALLOWED: int = 200` — server greeting, posting allowed
- `NNTP_POSTING_NOT_ALLOWED: int = 201` — server greeting, posting prohibited
- `NNTP_GROUP_SELECTED: int = 211` — group selected
- `NNTP_GROUP_LIST: int = 215` — list of newsgroups follows
- `NNTP_ARTICLE_ALL: int = 220` — article follows (head + body)
- `NNTP_ARTICLE_HEAD: int = 221` — head follows
- `NNTP_ARTICLE_BODY: int = 222` — body follows
- `NNTP_OVERVIEW: int = 224` — overview follows
- `NNTP_NEW_GROUPS: int = 231` — new newsgroups follow
- `NNTP_ARTICLE_POSTED: int = 240` — article posted successfully
- `NNTP_SEND_POST: int = 340` — send article to be posted
- `NNTP_AUTH_NEED: int = 480` — authentication required
- `NNTP_AUTH_CONTINUE: int = 381` — continue with authentication
- `NNTP_AUTH_OK: int = 281` — authentication accepted
- `NNTP_AUTH_BAD: int = 482` — authentication rejected
- `NNTP_NOT_PERMITTED: int = 440` — posting not permitted

## NntpResponse

A response from the NNTP server.

- `NntpResponse.init()` — fields default to empty
- `code: int` — three-digit response code
- `message: string` — text after the code on the first line
- `lines: ArrayList<string>` — multi-line payload (when the 4th char of the first line is `-`)
- `isOk(): bool` — `true` if `code` is in `[100, 400)`
- `isInformation(): bool` — `true` if `code` is in `[100, 200)`
- `isOkComplete(): bool` — `true` if `code` is in `[200, 300)`
- `toString(): string`

## GroupInfo

A newsgroup selection result.

- `GroupInfo.init()` — fields default to empty
- `name: string`
- `count: long` — estimated number of articles
- `first: long` — first article number
- `last: long` — last article number
- `toString(): string`

## OverviewEntry

A single `XOVER` record.

- `OverviewEntry.init()` — fields default to empty
- `articleNumber: long`
- `subject: string`
- `from: string`
- `date: string`
- `messageId: string`
- `references: string`
- `bytes: long`
- `lines: long`

## NntpArticle

An article returned by `article()` / `head()` / `body()`.

- `NntpArticle.init()` — fields default to empty
- `number: long` — article number
- `messageId: string`
- `headers: ArrayList<string>` — raw header lines
- `body: ArrayList<string>` — body lines
- `headerMap: HashMap<string, string>` — parsed `Key: Value` header map

## NNTP

`NNTP` is the client class.

- `NNTP.init(host: string, port: int)` — TLS is auto-enabled when `port == 563`
- `host: string` / `port: int` / `useTls: bool` / `debug: bool` / `postingAllowed: string`

### Connection

- `open(): string` — open the connection and read the greeting; sets `postingAllowed` based on whether the greeting starts with `200`
- `close(): void` — force close without `QUIT`
- `quit(): NntpResponse` — send `QUIT` and close the transport
- `isConnected(): bool`
- `getWelcome(): string`
- `getCurrentGroup(): string`

### Authentication

- `login(username: string, password: string): NntpResponse` — `AUTHINFO USER` then `AUTHINFO PASS` (skipped if the user command already returned `281`)

### Group and listing

- `group(groupName: string): GroupInfo` — select a newsgroup; returns counts and article number range
- `list(pattern: string): ArrayList<string>` — list all newsgroups matching the pattern; pass empty string for unfiltered

### Retrieval

- `overview(rangeSpec: string): ArrayList<OverviewEntry>` — retrieve the overview (`XOVER`) for an article range
- `xover(rangeSpec: string): ArrayList<OverviewEntry>` — alias for `overview`
- `article(spec: string): NntpArticle` — retrieve a full article by number or message-id
- `head(spec: string): NntpArticle` — retrieve only the headers of an article
- `body(spec: string): NntpArticle` — retrieve only the body of an article
- `next(): NntpArticle` — retrieve the next article in the current group
- `last(): NntpArticle` — retrieve the last article in the current group
- `stat(spec: string): NntpResponse` — send `STAT` (no body retrieval)

### Posting and server commands

- `post(text: string): NntpResponse` — post an article. The `text` is the full message including headers and a blank line separator. Returns the final response after the dot-terminated body is sent.
- `help(): ArrayList<string>` — send `HELP`
- `capabilities(): ArrayList<string>` — query server capabilities
- `sendCommand(command: string): NntpResponse` — send a raw command and read the response

## Module functions

### withNntp

Run a callback inside a fresh `NNTP` session that auto-quits. Authentication is skipped if `username` is empty.

- Parameters
  - `host: string`
  - `port: int`
  - `username: string`
  - `password: string`
  - `callback: fn(NNTP): void`
- Returns: `void`

```titrate
withNntp("news.example.com", 119, "", "", fn(client: NNTP): void {
    let groups: ArrayList<string> = client.list("");
    io::println("Server has " + Integer.toString(groups.size()) + " groups");
    let info: GroupInfo = client.group("comp.lang.python");
    io::println("Group has " + Long.toString(info.count) + " articles");
    let entries: ArrayList<OverviewEntry> = client.overview(Long.toString(info.first) + "-" + Long.toString(info.first + 5));
    var i: int = 0;
    while (i < entries.size()) {
        io::println(entries.get(i).subject);
        i = i + 1;
    }
});
```

## Notes

- All network operations go through native functions `Nntp_connect`, `Nntp_readLine`, `Nntp_write`, and `Nntp_close`.
- The internal `readResponse` distinguishes single-line and multi-line responses by the 4th character of the first line: a `-` indicates continuation lines until a lone `.`. Leading dots are de-stuffed per RFC 3977.
- `article()`/`head()`/`body()` parse the first payload line as `<number> <message-id>`, then split the remaining lines at the first empty line into headers and body.
- `XOVER` lines are tab-separated; the parser expects at least 7 fields (articleNumber, subject, from, date, messageId, references, bytes) and an optional 8th (lines).
- `post()` waits for the `340 Send article to be posted` code before sending the body; otherwise it returns the initial response unchanged.
