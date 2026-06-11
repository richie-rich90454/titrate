# networking

The `tt.net` module provides TCP client/server and HTTP client functionality backed by VM built-ins.

```titrate
import tt.net.TcpClient;
import tt.net.TcpServer;
import tt.net.HttpClient;
```

## TcpClient

TCP client for connecting to remote hosts.

- `TcpClient()` — create a new client
- `connect(host: String, port: int): bool` — connect to a host; returns true on success
- `send(data: String): int` — send data; returns bytes sent
- `receive(maxBytes: int): String` — receive up to maxBytes
- `close(): void` — close the connection
- `isConnected(): bool` — check if connected
- `setTimeout(ms: int): void` — set socket timeout
- `getLocalAddress(): String` — local address string
- `getRemoteAddress(): String` — remote address string

```titrate
let client = new TcpClient();
if (client.connect("example.com", 80)) {
    client.send("GET / HTTP/1.1\r\nHost: example.com\r\n\r\n");
    String response = client.receive(4096);
    client.close();
}
```

## TcpServer

TCP server for accepting incoming connections.

- `TcpServer()` — create a new server
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
String request = client.receive(1024);
client.send("Hello from server");
client.close();
server.close();
```

## HttpClient

HTTP client for making web requests backed by VM built-ins.

- `HttpClient()` — create a new client
- `setHeader(key: String, value: String): void` — set a default header
- `get(url: String): String` — HTTP GET; returns response body
- `post(url: String, body: String, contentType: String): String` — HTTP POST
- `put(url: String, body: String): String` — HTTP PUT
- `delete(url: String): String` — HTTP DELETE
- `patch(url: String, body: String): String` — HTTP PATCH
- `head(url: String): HttpResponse` — HTTP HEAD; returns response with status and headers

### HttpResponse

- `getStatusCode(): int` — HTTP status code
- `getHeaders(): HashMap<String, String>` — response headers
- `getBody(): String` — response body

```titrate
let http = new HttpClient();
http.setHeader("Accept", "application/json");
String body = http.get("https://api.example.com/data");
```
