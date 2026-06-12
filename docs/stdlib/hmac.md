# hmac

The `tt.crypto` module provides HMAC (Hash-based Message Authentication Code) computation. Hmac supports common digest algorithms for generating keyed-hash message authentication codes.

```titrate
import tt::crypto::Hmac;
```

## Hmac

HMAC computation for message authentication.

- `Hmac.sha256(key: string, data: string): string` — HMAC-SHA256 hex digest
- `Hmac.sha512(key: string, data: string): string` — HMAC-SHA512 hex digest
- `Hmac.md5(key: string, data: string): string` — HMAC-MD5 hex digest
- `Hmac.digest(key: string, data: string, algorithm: string): string` — HMAC with named algorithm (`"sha256"`, `"sha512"`, `"md5"`)

```titrate
let sig256: string = Hmac.sha256("secret-key", "message data");
let sig512: string = Hmac.sha512("secret-key", "message data");
let sigMd5: string = Hmac.md5("secret-key", "message data");
let sig: string = Hmac.digest("secret-key", "message data", "sha256");
```
