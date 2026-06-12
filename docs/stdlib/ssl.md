# ssl

The `tt.net` module provides TLS/SSL secure connections. SslContext manages SSL configuration and creates encrypted connections, while SslConnection handles the actual secure data transfer.

```titrate
import tt::net::SslContext;
import tt::net::SslConnection;
```

## SslContext

SSL/TLS context for establishing secure connections.

- `fn init()` — create SSL context
- `connect(host: string, port: int): SslConnection` — establish TLS connection
- `close(): void` — close context

```titrate
let ctx = new SslContext();
let conn: SslConnection = ctx.connect("example.com", 443);
conn.send("GET / HTTP/1.1\r\nHost: example.com\r\n\r\n");
let response: string = conn.recv(4096);
conn.close();
ctx.close();
```

## SslConnection

An active TLS/SSL connection for sending and receiving encrypted data.

- `send(data: string): int` — send encrypted data
- `recv(bufferSize: int): string` — receive encrypted data
- `close(): void` — close connection
- `getPeerCertificate(): string` — get peer certificate info

```titrate
let ctx = new SslContext();
let conn: SslConnection = ctx.connect("api.example.com", 443);
let bytesSent: int = conn.send("Hello, secure world!");
let data: string = conn.recv(1024);
let cert: string = conn.getPeerCertificate();
conn.close();
ctx.close();
```
