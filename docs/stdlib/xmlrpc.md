# XmlRpc

The `tt.net.XmlRpc` module mirrors Python's `xmlrpc.client` and `xmlrpc.server` modules. It provides a `ServerProxy` client, a `SimpleXMLRPCServer` server, and `Marshaller` / `Unmarshaller` helpers that translate Titrate `JsonValue` values to and from XML-RPC documents.

## Import

```titrate
import tt::net::XmlRpc;
```

## XmlRpcFault

An XML-RPC fault returned by the server.

- `XmlRpcFault.init(code: int, message: string)`
- `faultCode: int`
- `faultString: string`
- `toString(): string`

## XmlRpcRequest

A marshalled XML-RPC request envelope.

- `XmlRpcRequest.init(methodName: string)`
- `methodName: string`
- `params: ArrayList<JsonValue>`
- `addParam(value: JsonValue): void` — append a parameter
- `toXml(): string` — serialize the request to an XML-RPC `methodCall` document

## XmlRpcResponse

The result of an XML-RPC call.

- `XmlRpcResponse.init()` — `result` defaults to `JsonValue.ofNull()`, `fault` to `null`, `isFault` to `false`
- `result: JsonValue` — the single return value (valid when `isFault == false`)
- `fault: XmlRpcFault` — the parsed fault (valid when `isFault == true`)
- `isFault: bool`

## Marshaller

Encodes Titrate `JsonValue` values into XML-RPC `<value>` element bodies.

- `Marshaller.marshal(value: JsonValue): string` — marshal a single value to its XML representation
- `Marshaller.marshalValue(value: JsonValue): string` — public alias for `marshal`

## Unmarshaller

Decodes XML-RPC response XML back into Titrate `JsonValue` values.

- `Unmarshaller.parse(xml: string): XmlRpcResponse` — parse an XML-RPC `<methodResponse>` document

## ServerProxy

Client proxy that dispatches method calls to a remote XML-RPC endpoint.

- `ServerProxy.init(url: string)`
- `url: string` / `debug: bool` / `headers: HashMap<string, string>`
- `setHeader(name: string, value: string): void`
- `call(methodName: string, params: ArrayList<JsonValue>): XmlRpcResponse` — invoke a remote method by name with the given parameter list
- `callValue(methodName: string, params: ArrayList<JsonValue>): JsonValue` — convenience: invoke and return the result `JsonValue`, throwing on fault

## RegisteredMethod

A registered server-side method binding.

- `RegisteredMethod.init(name: string, handler: fn(ArrayList<JsonValue>): JsonValue)`
- `name: string`
- `handler: fn(ArrayList<JsonValue>): JsonValue`

## SimpleXMLRPCServer

A minimal XML-RPC server that dispatches registered methods.

- `SimpleXMLRPCServer.init(host: string, port: int)`
- `host: string` / `port: int` / `debug: bool`
- `registerFunction(name: string, handler: fn(ArrayList<JsonValue>): JsonValue): void` — register a function under a method name
- `registerMultiple(handlers: ArrayList<RegisteredMethod>): void` — register multiple functions in one call
- `registerIntrospectionFunctions(): void` — register the `system.listMethods` introspection helper
- `dispatch(requestXml: string): string` — dispatch a parsed request to its registered handler and build a response XML document
- `handleRequest(requestXml: string): string` — allow the transport layer to invoke `dispatch` for an incoming request
- `serveForever(): void` — begin serving requests; blocks until `stop()` is called
- `stop(): void` — stop the server loop
- `isRunning(): bool`

## Module functions

### marshal

Marshal a single `JsonValue` to its XML representation. Mirrors Python's `xmlrpc.client.Marshaller.dumps`.

- Parameters
  - `value: JsonValue`
- Returns: `string`

### unmarshal

Parse an XML-RPC `<methodResponse>` document. Mirrors Python's `xmlrpc.client.Unmarshaller`.

- Parameters
  - `xml: string`
- Returns: `XmlRpcResponse`

### callRemote

Convenience: create a `ServerProxy` and call a single method, returning the result `JsonValue` (throws on fault).

- Parameters
  - `url: string`
  - `methodName: string`
  - `params: ArrayList<JsonValue>`
- Returns: `JsonValue`

```titrate
let params: ArrayList<JsonValue> = new ArrayList<JsonValue>();
params.add(JsonValue.ofNum(7.0));
params.add(JsonValue.ofNum(5.0));
let result: JsonValue = callRemote("http://example.com/rpc", "Math.add", params);
io::println("Result: " + result.toString());
```

### escapeXml

Escape the five XML special characters (`& < > " '`).

- Parameters
  - `s: string`
- Returns: `string`

### unescapeXml

Reverse the five XML special-character escapes.

- Parameters
  - `s: string`
- Returns: `string`

## Type mapping

The marshaller and unmarshaller translate Titrate `JsonValue` variants to and from XML-RPC type tags:

| Titrate `JsonValue` | XML-RPC tag |
| --- | --- |
| `null` | `<nil/>` |
| `bool` | `<boolean>0|1</boolean>` |
| `int` (whole number) | `<int>...</int>` |
| `double` (non-whole) | `<double>...</double>` |
| `string` | `<string>...</string>` |
| `array` | `<array><data><value>...</value></data></array>` |
| `object` | `<struct><member><name>...</name><value>...</value></member>...</struct>` |

On parse, `<i4>` is accepted as a synonym for `<int>`. A `<value>` with no type tag is treated as a string.

## Notes

- Network requests go through the native function `XmlRpc_post(url, body, headers)`; the server loop uses `XmlRpc_serve(host, port, server)` and `XmlRpc_stop(host, port)`.
- `ServerProxy.call` builds an `XmlRpcRequest`, serialises it with `toXml()`, posts it via `XmlRpc_post`, then parses the response with an `Unmarshaller`.
- `SimpleXMLRPCServer.dispatch` produces a `<fault>` response with `faultCode = 1` for an unknown method, or `faultCode = 2` if the handler throws an exception.
- `registerIntrospectionFunctions` snapshots the method names registered so far and binds them to a fresh `system.listMethods` handler.
- XML strings are escaped/unescaped with exactly five replacements (not a lookup table): `&`, `<`, `>`, `"`, `'`.
