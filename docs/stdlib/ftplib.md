# FtpLib

The `tt.net.Ftplib` module mirrors Python's `ftplib` module. It provides an `FTP` client implementing RFC 959 with passive (`PASV`) data transfers, supporting `USER`/`PASS` login, `CWD`/`PWD`/`MKD`/`RMD`/`DELE`/`RNFR`+`RNTO` directory operations, `NLST`/`LIST` directory listings, `RETR`/`STOR` file transfers in both ASCII and binary mode, and `SIZE`/`MDTM` queries.

## Import

```titrate
import tt::net::Ftplib;
```

## Constants — transfer mode

- `FTP_ASCII: string = "A"`
- `FTP_BINARY: string = "I"`

## Constants — FTP response codes (RFC 959)

- `FTP_READY_NEW: int = 120` — service ready in n minutes
- `FTP_OK: int = 200` — command okay
- `FTP_SYSTEM: int = 215` — system type
- `FTP_SERVICE_READY: int = 220` — service ready for new user
- `FTP_SERVICE_CLOSING: int = 221` — service closing control connection
- `FTP_DATA_OPEN: int = 125` — data connection already open; transfer starting
- `FTP_DATA_OPEN_ALREADY: int = 225` — data connection open; no transfer in progress
- `FTP_DATA_CLOSE: int = 226` — closing data connection; transfer complete
- `FTP_PATHNAME: int = 257` — pathname created / current directory
- `FTP_USER_LOGIN_OK: int = 230` — user logged in
- `FTP_USER_NEED_PASSWORD: int = 331` — user name okay, need password
- `FTP_USER_NEED_ACCOUNT: int = 332` — need account for login
- `FTP_FILE_OK: int = 150` — file status okay; about to open data connection
- `FTP_FILE_UNAVAILABLE: int = 550` — file unavailable / not found

## FtpResponse

A response from the FTP control connection.

- `FtpResponse.init()` — fields default to empty
- `code: int` — three-digit response code
- `message: string` — final message text after the code
- `lines: ArrayList<string>` — continuation lines (when the 4th char of the first line is `-`)
- `isPositiveCompletion(): bool` — `true` if `code` is in `[200, 300)`
- `isPositiveIntermediate(): bool` — `true` if `code` is in `[300, 400)`
- `toString(): string`

## FTP

`FTP` is the client class. Both active and passive data transfers are implemented; only passive mode is enabled by default.

- `FTP.init(host: string, port: int)`
- `host: string` / `port: int` / `timeout: int` / `debug: bool` / `encoding: string`

### Connection

- `connect(): FtpResponse` — connect with the configured `timeout` and read the greeting
- `connectWithTimeout(timeoutMs: int): FtpResponse` — connect with an explicit timeout
- `close(): void` — close without sending `QUIT`
- `quit(): FtpResponse` — send `QUIT` and close the control connection
- `isConnected(): bool`
- `getWelcome(): string`
- `setPasv(mode: bool): void` — enable/disable passive mode for subsequent transfers

### Authentication

- `login(username: string, password: string): FtpResponse` — defaults to anonymous if `username` is empty; returns immediately if the server accepts with `230`

### File type

- `setType(type: string): FtpResponse` — set the transfer type (`FTP_ASCII` or `FTP_BINARY`)

### Directory operations

- `cwd(dirname: string): FtpResponse` — change working directory
- `pwd(): string` — print working directory (extracted from the quoted path in the `257` response)
- `cdup(): FtpResponse` — change to parent directory
- `mkd(dirname: string): FtpResponse` — make a directory
- `rmd(dirname: string): FtpResponse` — remove a directory
- `delete(filename: string): FtpResponse` — delete a file
- `rename(fromname: string, toname: string): FtpResponse` — rename via `RNFR` then `RNTO`

### Listing

- `nlst(dirname: string): ArrayList<string>` — list directory contents (short names) over the data connection
- `dir(dirname: string): ArrayList<string>` — list directory contents (long form) as lines

### File transfer

- `retrbinary(filename: string): string` — retrieve a file in binary mode, returning its content
- `retrlines(filename: string): ArrayList<string>` — retrieve a file in ASCII mode, returning its lines
- `storbinary(filename: string, data: string): FtpResponse` — store a binary file on the server
- `storlines(filename: string, data: string): FtpResponse` — store an ASCII file on the server

### File queries

- `size(filename: string): long` — get the size of a file (binary mode); returns `-1` on failure
- `mdtm(filename: string): string` — get the modification time of a file (`MDTM`)
- `void_(): FtpResponse` — `NOOP` keepalive

### Low-level

- `sendCommand(command: string): FtpResponse` — send a control command and read the response

## Module functions

### withFtp

Run a callback inside a fresh `FTP` session that auto-quits.

- Parameters
  - `host: string`
  - `port: int`
  - `username: string`
  - `password: string`
  - `callback: fn(FTP): void`
- Returns: `void`

```titrate
withFtp("ftp.example.com", 21, "anonymous", "alice@example.com", fn(client: FTP): void {
    let cwd: string = client.pwd();
    io::println("Remote cwd: " + cwd);
    let entries: ArrayList<string> = client.nlst("");
    var i: int = 0;
    while (i < entries.size()) {
        io::println(entries.get(i));
        i = i + 1;
    }
    let body: string = client.retrbinary("README.txt");
    io::println("README is " + Integer.toString(String.length(body)) + " bytes");
});
```

## Notes

- All network operations go through native functions `Ftp_connect`, `Ftp_readLine`, `Ftp_write`, `Ftp_readData`, `Ftp_writeData`, and `Ftp_close`.
- The internal `readResponse` recognises multi-line responses: when the 4th character of the first line is `-`, continuation lines are accumulated until a line starting with `<code> ` arrives.
- `pwd()` extracts the first double-quoted path from the `257` response message.
- Passive mode parses the `PASV` reply tuple `(h1,h2,h3,h4,p1,p2)` and opens a fresh TCP data connection to `h1.h2.h3.h4:p1*256+p2`.
- `nlst()` and `dir()` strip a trailing empty line produced by the final `CRLF`.
- Active mode (`PORT`/`EPRT`) is not implemented; calling any data command with passive mode disabled throws.
