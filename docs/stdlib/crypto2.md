# crypto2

The `tt.crypto2` module provides advanced cryptographic operations including symmetric encryption (AES), asymmetric encryption (RSA), elliptic curve cryptography (ECDSA), and key derivation functions (KDF). These primitives enable secure communication, digital signatures, key exchange, and password hashing.

```titrate
import tt.crypto2.AES;
import tt.crypto2.RSA;
import tt.crypto2.ECDSA;
import tt.crypto2.KDF;
```

## AES

AES symmetric encryption supporting 128/192/256-bit keys, ECB/CBC/CTR/GCM modes, and PKCS7 padding.

- `fn init(key: string, mode: string)` — create an AES cipher with the given key and mode (`"ecb"`, `"cbc"`, `"ctr"`, `"gcm"`). Key length determines AES variant: 16 bytes = AES-128, 24 bytes = AES-192, 32 bytes = AES-256.
- `encrypt(plaintext: string): string` — encrypt plaintext, returns Base64-encoded ciphertext
- `decrypt(ciphertext: string): string` — decrypt Base64-encoded ciphertext, returns plaintext
- `encryptWithIV(plaintext: string, iv: string): string` — encrypt with a specific IV (CBC/CTR/GCM modes)
- `decryptWithIV(ciphertext: string, iv: string): string` — decrypt with a specific IV
- `generateIV(): string` — generate a random 16-byte IV, returned as Base64
- `setPadding(padding: string): void` — set padding scheme (`"pkcs7"`, `"none"`)

```titrate
let key: string = "0123456789abcdef";  // 16 bytes = AES-128
let aes: AES = new AES(key, "cbc");
let iv: string = aes.generateIV();
let encrypted: string = aes.encryptWithIV("secret message", iv);
let decrypted: string = aes.decryptWithIV(encrypted, iv);
```

### GCM Mode

GCM mode provides authenticated encryption with an authentication tag.

- `encryptGCM(plaintext: string, iv: string, aad: string): string` — encrypt with additional authenticated data; returns Base64 ciphertext + tag
- `decryptGCM(ciphertext: string, iv: string, aad: string): string` — decrypt and verify authentication tag

```titrate
let aesGcm: AES = new AES("0123456789abcdef01234567", "gcm");
let iv: string = aesGcm.generateIV();
let aad: string = "header-data";
let enc: string = aesGcm.encryptGCM("authenticated message", iv, aad);
let dec: string = aesGcm.decryptGCM(enc, iv, aad);
```

## RSA

RSA asymmetric encryption supporting key generation, encryption/decryption, OAEP and PKCS1 v1.5 padding, and digital signatures.

- `fn init(keySize: int)` — create an RSA instance and generate a new key pair (valid sizes: 2048, 3072, 4096)
- `fn initFromKeys(publicKey: string, privateKey: string)` — create an RSA instance from existing PEM-encoded keys
- `publicKey(): string` — get the PEM-encoded public key
- `privateKey(): string` — get the PEM-encoded private key
- `encrypt(plaintext: string): string` — encrypt with public key using PKCS1 v1.5 padding, returns Base64 ciphertext
- `decrypt(ciphertext: string): string` — decrypt with private key using PKCS1 v1.5 padding
- `encryptOAEP(plaintext: string, hash: string): string` — encrypt with OAEP padding; hash is `"sha1"`, `"sha256"`, or `"sha512"`
- `decryptOAEP(ciphertext: string, hash: string): string` — decrypt with OAEP padding
- `sign(message: string, hash: string): string` — sign a message with the private key; returns Base64 signature
- `verify(message: string, signature: string, hash: string): bool` — verify a signature with the public key

```titrate
let rsa: RSA = new RSA(2048);
let pubKey: string = rsa.publicKey();
let privKey: string = rsa.privateKey();

let encrypted: string = rsa.encryptOAEP("secret data", "sha256");
let decrypted: string = rsa.decryptOAEP(encrypted, "sha256");

let signature: string = rsa.sign("important message", "sha256");
let valid: bool = rsa.verify("important message", signature, "sha256");
```

### Loading Existing Keys

```titrate
let pubPem: string = "...";  // PEM-encoded public key
let privPem: string = "..."; // PEM-encoded private key
let rsa2: RSA = RSA.initFromKeys(pubPem, privPem);
let enc: string = rsa2.encrypt("hello");
```

## ECDSA

Elliptic curve cryptography supporting ECDSA sign/verify, Curve25519 key exchange, and Ed25519 signatures.

- `fn init(curve: string)` — create an ECDSA instance with the named curve (`"secp256r1"`, `"secp384r1"`, `"secp521r1"`, `"curve25519"`, `"ed25519"`)
- `generateKeyPair(): void` — generate a new key pair on the selected curve
- `publicKey(): string` — get the public key as a hex string
- `privateKey(): string` — get the private key as a hex string
- `sign(message: string): string` — sign a message with the private key; returns hex-encoded signature
- `verify(message: string, signature: string): bool` — verify a signature with the public key
- `ecdhKeyExchange(otherPublicKey: string): string` — compute a shared secret using ECDH with another party's public key (Curve25519)

```titrate
let ecdsa: ECDSA = new ECDSA("secp256r1");
ecdsa.generateKeyPair();
let pub: string = ecdsa.publicKey();
let priv: string = ecdsa.privateKey();

let sig: string = ecdsa.sign("verify me");
let ok: bool = ecdsa.verify("verify me", sig);
```

### Curve25519 Key Exchange

```titrate
let alice: ECDSA = new ECDSA("curve25519");
alice.generateKeyPair();
let bob: ECDSA = new ECDSA("curve25519");
bob.generateKeyPair();

let sharedAlice: string = alice.ecdhKeyExchange(bob.publicKey());
let sharedBob: string = bob.ecdhKeyExchange(alice.publicKey());
// sharedAlice == sharedBob
```

### Ed25519 Signatures

```titrate
let ed: ECDSA = new ECDSA("ed25519");
ed.generateKeyPair();
let sig: string = ed.sign("ed25519 message");
let valid: bool = ed.verify("ed25519 message", sig);
```

## KDF

Key derivation functions for password hashing, key stretching, and key expansion.

- `pbkdf2(password: string, salt: string, iterations: int, keyLength: int, hash: string): string` — PBKDF2 derivation; hash is `"sha256"` or `"sha512"`; returns hex-derived key
- `scrypt(password: string, salt: string, n: int, r: int, p: int, keyLength: int): string` — scrypt derivation with cost parameters (n = CPU/memory cost, r = block size, p = parallelism)
- `argon2(password: string, salt: string, iterations: int, memory: int, parallelism: int, keyLength: int): string` — Argon2id derivation; memory in KiB
- `hkdf(hash: string, ikm: string, salt: string, info: string, keyLength: int): string` — HKDF expand; hash is `"sha256"` or `"sha512"`
- `generateSalt(length: int): string` — generate a cryptographically random salt of the given byte length, returned as hex
- `hashPassword(password: string): string` — hash a password with automatic salt and Argon2id; returns a stored-format string
- `verifyPassword(password: string, storedHash: string): bool` — verify a password against a stored hash

```titrate
let salt: string = KDF.generateSalt(16);

// PBKDF2
let derived: string = KDF.pbkdf2("password123", salt, 100000, 32, "sha256");

// scrypt
let key: string = KDF.scrypt("password123", salt, 16384, 8, 1, 32);

// Argon2id
let argonKey: string = KDF.argon2("password123", salt, 3, 65536, 4, 32);

// HKDF
let expanded: string = KDF.hkdf("sha256", "input-keying-material", salt, "context-info", 32);
```

### Password Hashing

```titrate
let stored: string = KDF.hashPassword("user-password");
let valid: bool = KDF.verifyPassword("user-password", stored);
```
