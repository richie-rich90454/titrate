# networking

The `tt.net` module provides TCP client/server and HTTP client functionality backed by VM built-ins.

```titrate
import tt.net.TcpClient;
import tt.net.TcpServer;
import tt.net.HttpClient;
```

## TcpClient

TCP client for connecting to remote hosts.

- `fn init()` — create a new client
- `connect(host: string, port: int): bool` — connect to a host; returns true on success
- `send(data: string): int` — send data; returns bytes sent
- `receive(maxBytes: int): string` — receive up to maxBytes
- `close(): void` — close the connection
- `isConnected(): bool` — check if connected
- `setTimeout(ms: int): void` — set socket timeout
- `getLocalAddress(): string` — local address string
- `getRemoteAddress(): string` — remote address string

```titrate
let client = new TcpClient();
if (client.connect("example.com", 80)) {
    client.send("GET / HTTP/1.1\r\nHost: example.com\r\n\r\n");
    let response: string = client.receive(4096);
    client.close();
}
```

## TcpServer

TCP server for accepting incoming connections.

- `fn init()` — create a new server
- `bind(port: int): bool` — bind to a port; returns true on success
- `accept(): TcpClient` — wait for and accept a client connection
- `close(): void` — stop listening and close
- `isBound(): bool` — check if bound
- `getLocalPort(): int` — the port the server is listening on
- `setTimeout(ms: int): void` — set accept timeout

```titrate
let server = new TcpServer();
server.bind(8080);
let client = server.accept();
let request: string = client.receive(1024);
client.send("Hello from server");
client.close();
server.close();
```

## HttpClient

HTTP client for making web requests backed by VM built-ins.

- `fn init()` — create a new client
- `setHeader(key: string, value: string): void` — set a default header
- `get(url: string): string` — HTTP GET; returns response body
- `post(url: string, body: string, contentType: string): string` — HTTP POST
- `put(url: string, body: string): string` — HTTP PUT
- `delete(url: string): string` — HTTP DELETE
- `patch(url: string, body: string): string` — HTTP PATCH
- `head(url: string): HttpResponse` — HTTP HEAD; returns response with status and headers

### HttpResponse

- `getStatusCode(): int` — HTTP status code
- `getHeaders(): HashMap<string, string>` — response headers
- `getBody(): string` — response body

```titrate
let http = new HttpClient();
http.setHeader("Accept", "application/json");
let body: string = http.get("https://api.example.com/data");
```

## WebSocket

- `WebSocket.connect(url: string): WebSocket` — connect to WebSocket server
- `WebSocket.send(message: string): void` — send text message
- `WebSocket.receive(): string` — receive message
- `WebSocket.close(): void` — close connection
- `WebSocket.ping(): void` — send ping frame
- `WebSocket.onMessage(handler: fn(string): void): void` — register message handler
- `WebSocket.onClose(handler: fn(): void): void` — register close handler

## URL Builder

- `UrlBuilder.init()` — create URL builder
- `UrlBuilder.scheme(s: string): UrlBuilder` — set scheme
- `UrlBuilder.host(h: string): UrlBuilder` — set host
- `UrlBuilder.port(p: int): UrlBuilder` — set port
- `UrlBuilder.path(p: string): UrlBuilder` — set path
- `UrlBuilder.query(key: string, value: string): UrlBuilder` — add query parameter
- `UrlBuilder.fragment(f: string): UrlBuilder` — set fragment
- `UrlBuilder.build(): string` — build URL string
- `UrlBuilder.normalize(url: string): string` — normalize URL

## HTTP Utilities

- `HttpUtil.cookieJar(): CookieJar` — create cookie jar
- `HttpUtil.multipartFormData(): MultipartBuilder` — create multipart form builder
- `HttpUtil.cache(): HttpCache` — create HTTP cache
- `HttpUtil.retryWithBackoff(request: fn(): string, maxRetries: int, baseDelayMs: int): string` — retry HTTP request
- `HttpUtil.connectionPool(maxConnections: int): ConnectionPool` — create connection pool

## DNS

- `Dns.lookupA(hostname: string): ArrayList<string>` — A record lookup
- `Dns.lookupAAAA(hostname: string): ArrayList<string>` — AAAA record lookup
- `Dns.lookupMX(hostname: string): ArrayList<string>` — MX record lookup
- `Dns.lookupNS(hostname: string): ArrayList<string>` — NS record lookup
- `Dns.lookupTXT(hostname: string): ArrayList<string>` — TXT record lookup
- `Dns.reverseLookup(ip: string): string` — reverse DNS lookup

## SMTP

- `Smtp.connect(host: string, port: int): SmtpClient` — connect to SMTP server
- `Smtp.authenticate(username: string, password: string): void` — authenticate
- `Smtp.send(from: string, to: string, subject: string, body: string): void` — send email
- `Smtp.sendWithAttachment(from: string, to: string, subject: string, body: string, attachmentPath: string): void` — send with attachment
- `Smtp.sendHtml(from: string, to: string, subject: string, htmlBody: string): void` — send HTML email
- `Smtp.disconnect(): void` — disconnect
