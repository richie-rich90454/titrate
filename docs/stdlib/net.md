---
title: net
description: Networking primitives for Titrate — TCP, UDP, HTTP, URL building, and DNS.
---

# net

The `tt.net` module provides networking primitives: TCP clients and servers, UDP sockets, HTTP clients, URL building, and DNS lookups.

```titrate
import tt::net::TcpClient;
import tt::net::TcpServer;
import tt::net::HttpClient;
import tt::net::UdpSocket;
import tt::net::UrlBuilder;
import tt::net::Dns;
```

## TcpClient

- `fn init()`
- `connect(host: string, port: int): bool`
- `send(data: string): int`
- `receive(maxBytes: int): string`
- `close(): void`
- `isConnected(): bool`
- `setTimeout(ms: int): void`
- `getLocalAddress(): string`
- `getRemoteAddress(): string`
- `setHandle(h: long): void`

```titrate
let client: TcpClient = new TcpClient();
if (client.connect("example.com", 80)) {
    client.send("GET / HTTP/1.0\r\n\r\n");
    let response: string = client.receive(4096);
    io::println(response);
    client.close();
}
```

## TcpServer

- `fn init()`
- `bind(port: int): bool`
- `accept(): TcpClient`
- `close(): void`
- `isBound(): bool`
- `getLocalPort(): int`
- `setTimeout(ms: int): void`

```titrate
let server: TcpServer = new TcpServer();
if (server.bind(8080)) {
    io::println("Listening on port " + Integer.toString(server.getLocalPort()));
    let client: TcpClient = server.accept();
    client.send("Hello\n");
    client.close();
}
```

## HttpClient

- `fn init()`
- `setHeader(key: string, value: string): void`
- `setTimeout(ms: int): void`
- `setFollowRedirects(enabled: bool): void`
- `setUserAgent(agent: string): void`
- `get(url: string): HttpResponse`
- `post(url: string, body: string, contentType: string): string`
- `put(url: string, body: string): string`
- `delete(url: string): string`
- `patch(url: string, body: string): string`
- `head(url: string): HttpResponse`

### HttpResponse

- `fn init(statusCode: int, headers: HashMap<string, string>, body: string)`
- `getStatusCode(): int`
- `getReason(): string`
- `setReason(reason: string): void`
- `getHeaders(): HashMap<string, string>`
- `getBody(): string`
- `toString(): string`

```titrate
let http: HttpClient = new HttpClient();
http.setUserAgent("Titrate/1.0");
let resp: HttpResponse = http.get("https://api.example.com/data");
if (resp.getStatusCode() == 200) {
    io::println(resp.getBody());
}
```

## UdpSocket

- `fn init()`
- `bind(host: string, port: int): void`
- `sendTo(data: string, host: string, port: int): int`
- `receiveFrom(bufferSize: int): Pair<string, Pair<string, int>>`
- `close(): void`
- `setTimeout(ms: int): void`
- `isClosed(): bool`

```titrate
let udp: UdpSocket = new UdpSocket();
udp.bind("0.0.0.0", 9999);
udp.sendTo("hello", "127.0.0.1", 9999);
let result: Pair<string, Pair<string, int>> = udp.receiveFrom(1024);
io::println(result.first);
```

## UrlBuilder

Fluent URL construction and parsing.

- `fn init()`
- `withScheme(scheme: string): Url`
- `withHost(host: string): Url`
- `withPort(port: int): Url`
- `withPath(path: string): Url`
- `withQuery(key: string, value: string): Url`
- `query(key: string, value: string): Url`
- `withFragment(fragment: string): Url`
- `withUserInfo(user: string): Url`
- `build(): string`
- `toString(): string`

### Module Functions

- `UrlBuilder.parseUrl(url: string): Url`
- `UrlBuilder.urlJoin(base: string, relative: string): string`
- `UrlBuilder.urlNormalize(url: string): string`
- `UrlBuilder.urlEncode(s: string): string`
- `UrlBuilder.urlDecode(s: string): string`

```titrate
let url: string = UrlBuilder.parseUrl("https://example.com")
    .withPath("/api/v1/users")
    .withQuery("page", "1")
    .withQuery("limit", "50")
    .build();
io::println(url);
```

## Dns

- `Dns.dnsLookup(hostname: string, recordType: string): ArrayList<DnsRecord>`
- `Dns.lookupMX(domain: string): ArrayList<DnsRecord>`
- `Dns.reverseDns(ip: string): string`

### DnsRecord

- `fn init(name: string, recordType: string, value: string, ttl: int)`
- `toString(): string`

### DnsCache

- `fn init(defaultTtl: int)`
- `lookup(hostname: string, recordType: string): ArrayList<DnsRecord>`
- `flush(): void`

```titrate
let records: ArrayList<DnsRecord> = Dns.dnsLookup("example.com", "A");
for (record in records) {
    io::println(record.toString());
}
```
