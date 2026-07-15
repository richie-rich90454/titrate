# WsgiRef

The `tt.net.WsgiRef` module mirrors Python's `wsgiref` package. It provides a minimal WSGI reference server (`WSGIServer`), an HTTP request handler (`WSGIRequestHandler`) that parses the request, builds the WSGI `environ`, invokes the application, and writes the response back to the client, plus the `StartResponse` callable and the `make_server` / `serve_forever` convenience helpers.

A WSGI application is a function `fn(HashMap<string, string>, StartResponse): ArrayList<string>` — it takes the environ and a `start_response` callable, calls `start_response.call(status, headers)`, and returns the response body as a list of string chunks.

## Import

```titrate
import tt::net::WsgiRef;
```

## WsgiResponseHeader

A single HTTP response header as returned by the WSGI application via `start_response`.

- `WsgiResponseHeader.init(name: string, value: string)`
- `name: string`
- `value: string`

## StartResponse

Callable that the WSGI application invokes to begin the HTTP response. Mirrors Python's `start_response(status, headers)`.

- `StartResponse.init()` — `status` defaults to empty, `headers` to an empty list, `called` to `false`
- `status: string` — e.g. `"200 OK"`
- `headers: ArrayList<WsgiResponseHeader>`
- `called: bool` — whether `call` has been invoked
- `call(status: string, headers: ArrayList<WsgiResponseHeader>): void` — set the response status line and header list
- `getStatus(): string`
- `getHeaders(): ArrayList<WsgiResponseHeader>`
- `isCalled(): bool`

## WsgiRequest

Parsed HTTP request: request line components, headers, and body.

- `WsgiRequest.init()` — defaults to `GET / HTTP/1.1`
- `method: string`
- `path: string`
- `query: string`
- `version: string`
- `headers: HashMap<string, string>`
- `body: string`

## WSGIRequestHandler

Handles a single HTTP request: parse, build environ, invoke the WSGI app, and write the response back to the client.

- `WSGIRequestHandler.init(server: WSGIServer, client: TcpClient)`
- `server: WSGIServer`
- `client: TcpClient`
- `request: WsgiRequest`
- `receivedBuffer: string`
- `readRequest(): bool` — read the entire request (headers + optional body) from the client; returns `false` on EOF
- `parseRequest(): bool` — parse the buffered HTTP request into method, path, query, version, headers, and body
- `buildEnviron(): HashMap<string, string>` — build the WSGI environ dict from the parsed request
- `handle(): void` — drive the WSGI application: build environ, call `start_response`, then write the full HTTP response back to the client
- `writeResponse(startResponse: StartResponse, body: ArrayList<string>): void` — write a successful response to the client
- `writeError(code: int, reason: string, message: string): void` — write an error response with the given status code and message

### WSGI environ keys

`buildEnviron()` populates these standard WSGI keys:

- `REQUEST_METHOD`, `PATH_INFO`, `QUERY_STRING`, `SERVER_NAME`, `SERVER_PORT`, `SERVER_PROTOCOL`
- `wsgi.version` (`"1.0"`), `wsgi.url_scheme` (`"http"`), `wsgi.input` (the body), `wsgi.errors` (`""`), `wsgi.multithread` (`"false"`), `wsgi.multiprocess` (`"false"`), `wsgi.run_once` (`"false"`)
- Each request header is copied as `HTTP_<NAME>` with dashes replaced by underscores and the name uppercased.
- `Content-Type` and `Content-Length` additionally get non-`HTTP_` keys (`CONTENT_TYPE`, `CONTENT_LENGTH`).

## WSGIServer

WSGI reference server. Holds the application and accepts connections, dispatching each to a fresh `WSGIRequestHandler`.

- `WSGIServer.init(host: string, port: int, app: fn(HashMap<string, string>, StartResponse): ArrayList<string>)`
- `host: string` / `port: int`
- `application: fn(HashMap<string, string>, StartResponse): ArrayList<string>`
- `serveForever(): void` — bind and start accepting connections; blocks until `stop()` is called (throws on bind failure)
- `stop(): void` — stop accepting connections and close the listening socket
- `handleRequest(environ: HashMap<string, string>): ArrayList<string>` — handle a single request programmatically (useful for testing without opening a real socket); returns the response body chunks
- `isRunning(): bool`

## Module functions

### make_server

Create a `WSGIServer` bound to `host:port` serving `app`.

- Parameters
  - `host: string`
  - `port: int`
  - `app: fn(HashMap<string, string>, StartResponse): ArrayList<string>`
- Returns: `WSGIServer`

### serve_forever

Run a server's accept loop. Mirrors Python's `serve_forever()`.

- Parameters
  - `server: WSGIServer`
- Returns: `void`

```titrate
let app: fn(HashMap<string, string>, StartResponse): ArrayList<string> = fn(environ: HashMap<string, string>, startResponse: StartResponse): ArrayList<string> {
    let headers: ArrayList<WsgiResponseHeader> = new ArrayList<WsgiResponseHeader>();
    headers.add(new WsgiResponseHeader("Content-Type", "text/plain"));
    startResponse.call("200 OK", headers);
    let body: ArrayList<string> = new ArrayList<string>();
    body.add("Hello from WSGI!");
    return body;
};
let server: WSGIServer = make_server("127.0.0.1", 8080, app);
io::println("Serving on http://127.0.0.1:8080/");
serve_forever(server);
```

## Notes

- The server is single-threaded: each accepted connection is fully processed (`handle()` → `readRequest` → `parseRequest` → `buildEnviron` → application → `writeResponse`) before the next connection is accepted.
- `readRequest` accumulates bytes until it sees the `\r\n\r\n` blank line that separates headers from the body, then reads the body up to `Content-Length` if present.
- If the application throws, `handle()` writes a `500 Internal Server Error` response with the exception message as the body.
- If the application returns without calling `start_response`, `handle()` writes a `500 Internal Server Error` with the message `"start_response not called"`.
- `writeResponse` adds a `Content-Length` header computed from the body chunks if the application did not already provide one.
- All socket operations go through `tt.net.TcpServer` and `tt.net.TcpClient`.
