# crypto

The `tt.crypto` and `tt.encoding` modules provide cryptographic hashing, encoding, and decoding utilities. Hash offers common digest algorithms, while Base64, Hex, and Url handle standard encoding schemes.

```titrate
import tt.crypto.Hash;
import tt.encoding.Base64;
import tt.encoding.Hex;
import tt.encoding.Url;
```

## Hash

- `Hash.md5(input: String): String` — compute MD5 digest
- `Hash.sha1(input: String): String` — compute SHA-1 digest
- `Hash.sha256(input: String): String` — compute SHA-256 digest
- `Hash.sha384(input: String): String` — compute SHA-384 digest
- `Hash.sha512(input: String): String` — compute SHA-512 digest
- `Hash.hmac(key: String, data: String): String` — compute HMAC-SHA256
- `Hash.crc32(data: String): String` — compute CRC-32 checksum
- `Hash.hashBytes(algorithm: String, data: String): String` — hash using the named algorithm (`"md5"`, `"sha1"`, `"sha256"`, `"sha384"`, `"sha512"`)

```titrate
let digest = Hash::sha256("hello world");
let mac = Hash::hmac("secret-key", "message");
let checksum = Hash::crc32("some data");
```

## Base64

- `Base64.encode(input: String): String` — encode a string to Base64
- `Base64.decode(input: String): String` — decode a Base64 string
- `Base64.encodeUrlSafe(data: String): String` — URL-safe Base64 encoding
- `Base64.decodeUrlSafe(data: String): String` — URL-safe Base64 decoding
- `Base64.encodeWithoutPadding(data: String): String` — encode without trailing `=` padding
- `Base64.encodeBytes(bytes: ArrayList<int>): String` — encode raw bytes to Base64
- `Base64.decodeToBytes(data: String): ArrayList<int>` — decode Base64 to raw bytes

```titrate
let encoded = Base64::encode("hello");
let decoded = Base64::decode(encoded);
let safe = Base64::encodeUrlSafe("data?param=value");
```

## Hex

- `Hex.encode(input: String): String` — encode a string to hexadecimal
- `Hex.decode(input: String): String` — decode a hexadecimal string
- `Hex.encodeUpperCase(data: String): String` — encode with uppercase hex digits
- `Hex.encodeLowerCase(data: String): String` — encode with lowercase hex digits
- `Hex.encodeBytes(bytes: ArrayList<int>): String` — encode raw bytes to hex
- `Hex.decodeToBytes(data: String): ArrayList<int>` — decode hex to raw bytes

```titrate
let hex = Hex::encode("ABC");
let upper = Hex::encodeUpperCase("ABC");
let bytes = Hex::decodeToBytes("48656c6c6f");
```

## Url

- `Url.encode(input: String): String` — percent-encode a URL string
- `Url.decode(input: String): String` — decode a percent-encoded URL
- `Url.encodeComponent(s: String): String` — encode a URL component (encodes more characters)
- `Url.decodeComponent(s: String): String` — decode a percent-encoded URL component
- `Url.parseQueryString(qs: String): HashMap<String, String>` — parse a query string into a map
- `Url.buildQueryString(map: HashMap<String, String>): String` — build a query string from a map

```titrate
let encoded = Url::encodeComponent("hello world&foo=bar");
let params = Url::parseQueryString("?name=alice&age=30");
let qs = Url::buildQueryString(params);
```
