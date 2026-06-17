# crypto

The `tt.crypto` and `tt.encoding` modules provide cryptographic hashing, HMAC, encoding, and decoding utilities. Hash offers common digest algorithms, Hmac provides keyed-hash message authentication, while Base64, Hex, and Url handle standard encoding schemes.

```titrate
import tt.crypto.Hash;
import tt.crypto.Hmac;
import tt.encoding.Base64;
import tt.encoding.Hex;
import tt.encoding.Url;
```

## Hash

- `Hash.md5(input: string): string` — compute MD5 digest
- `Hash.sha1(input: string): string` — compute SHA-1 digest
- `Hash.sha256(input: string): string` — compute SHA-256 digest
- `Hash.sha384(input: string): string` — compute SHA-384 digest
- `Hash.sha512(input: string): string` — compute SHA-512 digest
- `Hash.hmac(key: string, data: string): string` — compute HMAC-SHA256
- `Hash.crc32(data: string): string` — compute CRC-32 checksum
- `Hash.hashBytes(algorithm: string, data: string): string` — hash using the named algorithm (`"md5"`, `"sha1"`, `"sha256"`, `"sha384"`, `"sha512"`)

```titrate
let digest = Hash.sha256("hello world");
let mac = Hash.hmac("secret-key", "message");
let checksum = Hash.crc32("some data");
```

## Hmac

Keyed-hash message authentication codes (HMAC) for verifying both data integrity and authenticity.

- `Hmac.sha256(key: string, data: string): string` — HMAC-SHA256 hex digest
- `Hmac.sha512(key: string, data: string): string` — HMAC-SHA512 hex digest
- `Hmac.md5(key: string, data: string): string` — HMAC-MD5 hex digest
- `Hmac.digest(key: string, data: string, algorithm: string): string` — HMAC with named algorithm (`"sha256"`, `"sha512"`, `"md5"`)

```titrate
let mac256 = Hmac.sha256("secret-key", "message");
let mac512 = Hmac.sha512("secret-key", "message");
let custom = Hmac.digest("key", "data", "sha256");
```

## Base64

- `Base64.encode(input: string): string` — encode a string to Base64
- `Base64.decode(input: string): string` — decode a Base64 string
- `Base64.encodeUrlSafe(data: string): string` — URL-safe Base64 encoding
- `Base64.decodeUrlSafe(data: string): string` — URL-safe Base64 decoding
- `Base64.encodeWithoutPadding(data: string): string` — encode without trailing `=` padding
- `Base64.encodeBytes(bytes: ArrayList<int>): string` — encode raw bytes to Base64
- `Base64.decodeToBytes(data: string): ArrayList<int>` — decode Base64 to raw bytes

```titrate
let encoded = Base64.encode("hello");
let decoded = Base64.decode(encoded);
let safe = Base64.encodeUrlSafe("data?param=value");
```

## Hex

- `Hex.encode(input: string): string` — encode a string to hexadecimal
- `Hex.decode(input: string): string` — decode a hexadecimal string
- `Hex.encodeUpperCase(data: string): string` — encode with uppercase hex digits
- `Hex.encodeLowerCase(data: string): string` — encode with lowercase hex digits
- `Hex.encodeBytes(bytes: ArrayList<int>): string` — encode raw bytes to hex
- `Hex.decodeToBytes(data: string): ArrayList<int>` — decode hex to raw bytes

```titrate
let hex = Hex.encode("ABC");
let upper = Hex.encodeUpperCase("ABC");
let bytes = Hex.decodeToBytes("48656c6c6f");
```

## Url

- `Url.encode(input: string): string` — percent-encode a URL string
- `Url.decode(input: string): string` — decode a percent-encoded URL
- `Url.encodeComponent(s: string): string` — encode a URL component (encodes more characters)
- `Url.decodeComponent(s: string): string` — decode a percent-encoded URL component
- `Url.parseQueryString(qs: string): HashMap<string, string>` — parse a query string into a map
- `Url.buildQueryString(map: HashMap<string, string>): string` — build a query string from a map

```titrate
let encoded = Url.encodeComponent("hello world&foo=bar");
let params = Url.parseQueryString("?name=alice&age=30");
let qs = Url.buildQueryString(params);
```

## Ed25519 Signatures

- `Ed25519.generateKeyPair(): (string, string)` — generate Ed25519 key pair
- `Ed25519.sign(privateKey: string, message: string): string` — sign message
- `Ed25519.verify(publicKey: string, message: string, signature: string): bool` — verify signature

## Curve25519 Key Exchange

- `Curve25519.generateKeyPair(): (string, string)` — generate X25519 key pair
- `Curve25519.computeSharedSecret(privateKey: string, publicKey: string): string` — compute shared secret

## ChaCha20-Poly1305

- `ChaCha20Poly1305.encrypt(key: string, nonce: string, plaintext: string): string` — AEAD encrypt
- `ChaCha20Poly1305.decrypt(key: string, nonce: string, ciphertext: string): string` — AEAD decrypt

## HKDF

- `HKDF.extract(salt: string, inputKeyMaterial: string): string` — HKDF-Extract
- `HKDF.expand(prk: string, info: string, length: int): string` — HKDF-Expand
- `HKDF.deriveKey(salt: string, inputKeyMaterial: string, info: string, length: int): string` — full HKDF

## Constant-Time Comparison

- `Crypto.constantTimeEquals(a: string, b: string): bool` — constant-time string comparison
