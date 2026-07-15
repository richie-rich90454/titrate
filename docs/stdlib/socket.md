# socket

The `tt.net` module provides raw socket APIs for TCP and UDP communication. Socket offers full control over TCP connections, while UdpSocket handles datagram-based communication.

```titrate
import tt::net::Socket;
import tt::net::UdpSocket;
```

## Socket

Raw TCP socket for client and server communication.

- `fn init()` — create unconnected socket
- `connect(host: string, port: int): void` — connect to remote host
- `bind(host: string, port: int): void` — bind to address
- `listen(backlog: int): void` — listen for connections
- `accept(): Socket` — accept incoming connection
- `send(data: string): int` — send data
- `recv(bufferSize: int): string` — receive data
- `close(): void` — close socket
- `setTimeout(ms: int): void` — set I/O timeout
- `setNoDelay(noDelay: bool): void` — set TCP_NODELAY
- `isConnected(): bool` — check connection state
- `isClosed(): bool` — check if closed

```titrate
let sock = new Socket();
sock.connect("example.com", 80);
sock.send("GET / HTTP/1.1\r\nHost: example.com\r\n\r\n");
let response: string = sock.recv(4096);
sock.close();
```

### TCP Server Example

```titrate
let server = new Socket();
server.bind("0.0.0.0", 8080);
server.listen(5);
let client: Socket = server.accept();
let request: string = client.recv(1024);
client.send("Hello from server");
client.close();
server.close();
```

## UdpSocket

UDP socket for datagram-based communication.

- `fn init()` — create unbound UDP socket
- `bind(host: string, port: int): void` — bind to address
- `sendTo(data: string, host: string, port: int): int` — send datagram
- `receiveFrom(bufferSize: int): Pair<string, Pair<string, int>>` — receive datagram with sender info
- `close(): void` — close socket
- `setTimeout(ms: int): void` — set I/O timeout
- `isClosed(): bool` — check if closed

```titrate
let udp = new UdpSocket();
udp.bind("0.0.0.0", 9090);
let sent: int = udp.sendTo("hello", "127.0.0.1", 9091);
let result: Pair<string, Pair<string, int>> = udp.receiveFrom(1024);
let data: string = result.first;
let senderAddr: string = result.second.first;
let senderPort: int = result.second.second;
udp.close();
```

## SocketServer framework (Phase 1-2 parity)

The `SocketServer` framework provides a high-level server abstraction that mirrors Python's `socketserver` module. It handles accept loops, request dispatching, and optional threading so you can focus on the request handler.

### BaseServer / TCPServer

- `BaseServer(serverAddress: string, port: int, handler: RequestHandler)` — abstract base class
- `TCPServer(host: string, port: int, handler: RequestHandler)` — synchronous TCP server; serves one request at a time
- `serveForever(): void` — accept and dispatch connections until `shutdown()` is called
- `shutdown(): void` — stop serving and return from `serveForever()`
- `serverClose(): void` — close the listening socket

### ThreadingTCPServer

- `ThreadingTCPServer(host: string, port: int, handler: RequestHandler)` — TCP server that spawns a new thread per request (mirrors `socketserver.ThreadingTCPServer`)

### UDPServer / ThreadingUDPServer

- `UDPServer(host: string, port: int, handler: RequestHandler)` — synchronous datagram server
- `ThreadingUDPServer(host: string, port: int, handler: RequestHandler)` — threaded datagram server

### RequestHandler

Subclass `StreamRequestHandler` (TCP) or `DatagramRequestHandler` (UDP) and override `handle(request: Variant): void` to process a single request.

```titrate
import tt.net.SocketServer;

public class EchoHandler extends SocketServer.StreamRequestHandler {
    public fn handle(request: Variant): void {
        let conn = request as Socket;
        let data: string = conn.recv(1024);
        conn.send("echo: " + data);
        conn.close();
    }
}

// Single-threaded server
let server = new SocketServer.TCPServer("0.0.0.0", 8080, new EchoHandler());
server.serveForever();

// Threaded server (one thread per connection)
let threaded = new SocketServer.ThreadingTCPServer("0.0.0.0", 8081, new EchoHandler());
threaded.serveForever();
```

**Mix-in classes** `ForkingMixIn` and `ThreadingMixIn` are also available for composing custom server classes, mirroring the Python mix-in pattern.
