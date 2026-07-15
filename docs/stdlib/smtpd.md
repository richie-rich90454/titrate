# Smtpd

The `tt.net.Smtpd` module provides building blocks for SMTP server implementations. It mirrors Python's `smtpd` module, exposing `SMTPServer` (the base server with an accept loop), `SMTPChannel` (per-connection protocol driver), `DebuggingServer` (prints received messages to stdout), and `PureProxy` (forwards messages to a remote SMTP server). Subclasses override `processMessage` to handle delivered mail.

## Import

```titrate
import tt::net::Smtpd;
```

## Classes

### SMTPChannel

Represents a single client connection driving the SMTP protocol. The channel reads commands via `TcpClient.readLine`, dispatches them through `handleCommand`, and accumulates DATA payloads via `handleData` until the terminating `.` line.

**Fields:**
- `server: SMTPServer` — owning server
- `client: TcpClient` — underlying client socket
- `receivedLines: string` — buffer of raw received lines
- `mailFrom: string` — `MAIL FROM` address for the current transaction
- `rcptTos: ArrayList<string>` — `RCPT TO` recipients for the current transaction
- `mailData: string` — accumulated message body
- `state: string` — current protocol state (`"init"`, `"command"`, or `"data"`)
- `closed: bool` — whether the channel has been closed

**Constructors:**
- `init(server: SMTPServer, client: TcpClient)`

**Methods:**
- `send(code: int, message: string): void` — send a `"<code> <message>\r\n"` response line to the client
- `greet(): void` — send a `220 <hostname> Titrate SMTP` greeting and enter `"command"` state
- `handleCommand(line: string): void` — dispatch a single SMTP command (`HELO`/`EHLO`, `MAIL FROM`, `RCPT TO`, `DATA`, `QUIT`, `RSET`, `NOOP`); unrecognized commands receive a `502` response
- `handleData(line: string): void` — accumulate message body; on receipt of a single `.` line, invokes `server.processMessage` and resets the transaction
- `close(): void` — close the underlying client socket

```titrate
// Channels are typically created by SMTPServer.start, not directly.
```

### SMTPServer

Base class for SMTP servers. Binds a `TcpServer`, accepts connections, and dispatches each to a fresh `SMTPChannel`. Subclasses override `processMessage` to handle delivered mail.

**Fields:**
- `hostname: string` — advertised server hostname
- `port: int` — listening port
- `_socket: TcpServer` — underlying listener
- `_channels: ArrayList<SMTPChannel>` — active channels
- `_running: bool` — whether the accept loop is active

**Constructors:**
- `init(host: string, port: int)`

**Methods:**
- `start(): bool` — bind, then accept connections in a loop, greeting each and dispatching via `handleChannel`. Returns `true` if the loop ran to completion.
- `handleChannel(channel: SMTPChannel): void` — read lines from `channel.client` and dispatch to `handleCommand`/`handleData` until `channel.closed`. Override to customize the per-connection loop (e.g. async I/O).
- `stop(): void` — stop accepting connections and close the listener
- `processMessage(mailfrom: string, rcpttos: ArrayList<string>, data: string): void` — **override point** called when a message has been fully received. The base implementation is a no-op.

```titrate
public class MyServer extends SMTPServer {
    public fn init(host: string, port: int) {
        super.init(host, port);
    }

    public fn processMessage(mailfrom: string, rcpttos: ArrayList<string>, data: string): void {
        io::println("From: " + mailfrom);
        io::println("Data length: " + Integer.toString(String.length(data)));
    }
}

public fn main(): void {
    let server = new MyServer("localhost", 2525);
    server.start();
}
```

### DebuggingServer

An `SMTPServer` that prints every received message to stdout.

**Constructors:**
- `init(host: string, port: int)`

**Override:**
- `processMessage(mailfrom: string, rcpttos: ArrayList<string>, data: string): void` — prints a `MESSAGE` banner, the sender, each recipient, and the message body

```titrate
let server = new DebuggingServer("localhost", 8025);
server.start();
```

### PureProxy

An `SMTPServer` that forwards received messages to another SMTP server via a `SmtpClient`.

**Fields:**
- `remoteHost: string` — upstream SMTP host
- `remotePort: int` — upstream SMTP port

**Constructors:**
- `init(host: string, port: int, remoteHost: string, remotePort: int)`

**Override:**
- `processMessage(mailfrom: string, rcpttos: ArrayList<string>, data: string): void` — opens a `SmtpClient` to `(remoteHost, remotePort)`, connects, and sends the message

```titrate
let proxy = new PureProxy("0.0.0.0", 25, "smtp.upstream.example", 25);
proxy.start();
```

## Usage Example

```titrate
import tt::net::Smtpd;
import tt::util::ArrayList;

public class LoggingServer extends SMTPServer {
    public fn init(host: string, port: int) {
        super.init(host, port);
    }

    public fn processMessage(mailfrom: string, rcpttos: ArrayList<string>, data: string): void {
        io::println("Received mail from " + mailfrom);
        var i: int = 0;
        while (i < rcpttos.size()) {
            io::println("  -> " + rcpttos.get(i));
            i = i + 1;
        }
    }
}

public fn main(): void {
    let server = new LoggingServer("localhost", 2525);
    server.start();
}
```
