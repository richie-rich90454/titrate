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

## HKDF

- `HKDF.extract(salt: string, inputKeyMaterial: string): string` — HKDF-Extract
- `HKDF.expand(prk: string, info: string, length: int): string` — HKDF-Expand
- `HKDF.deriveKey(salt: string, inputKeyMaterial: string, info: string, length: int): string` — full HKDF

## PBKDF2

- `PBKDF2.deriveKey(password: string, salt: string, iterations: int, keyLength: int): string` — PBKDF2 key derivation

## scrypt

- `Scrypt.deriveKey(password: string, salt: string, n: int, r: int, p: int, keyLength: int): string` — scrypt key derivation

## Argon2

- `Argon2.hash(password: string, salt: string, iterations: int, memory: int, parallelism: int): string` — Argon2id hash
- `Argon2.verify(hash: string, password: string): bool` — verify Argon2 hash
