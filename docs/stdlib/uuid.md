# uuid

The `tt.uuid` module provides UUID (Universally Unique Identifier) generation and validation utilities. UUIDs are 128-bit identifiers that are unique across space and time without requiring a central coordination authority.

```titrate
import tt.uuid.Uuid;
```

## What Are UUIDs?

A **UUID** (Universally Unique Identifier) is a 128-bit value represented as a string of 32 hexadecimal digits displayed in five groups separated by hyphens:

```
xxxxxxxx-xxxx-Mxxx-Nxxx-xxxxxxxxxxxx
```

Where:
- `M` indicates the UUID version (1–5)
- `N` indicates the variant (typically `8`, `9`, `a`, or `b` for RFC 4122)

### Why Use UUIDs?

- **Globally unique**: No coordination needed between systems generating IDs
- **No collision risk**: The probability of generating duplicate UUIDs is negligibly small
- **Distributed-friendly**: Different machines can generate IDs independently
- **Merging data**: Datasets with UUID keys can be merged without key conflicts
- **Security**: UUIDs are unpredictable, making them unsuitable for enumeration attacks

### UUID Versions

The UUID specification defines several versions, each with different generation strategies:

| Version | Name | Description |
|---------|------|-------------|
| 1 | Time-based | Based on timestamp and MAC address |
| 2 | DCE Security | Based on timestamp, with POSIX UID/GID |
| 3 | MD5 Hash | Based on namespace and name, using MD5 |
| 4 | Random | Based on random or pseudo-random numbers |
| 5 | SHA-1 Hash | Based on namespace and name, using SHA-1 |

Titrate's `Uuid` module generates **version 4** (random) UUIDs, which are the most commonly used.

## Functions

### `uuid4(): string`

Generates a version 4 (random) UUID string. The UUID follows RFC 4122 format with version and variant bits set correctly:

- Format: `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx`
- The `4` at position 13 indicates version 4
- `y` is one of `8`, `9`, `a`, or `b` (RFC 4122 variant)

```titrate
let id: string = Uuid.uuid4();
io::println(id);
// Example output: "a1b2c3d4-e5f6-4a7b-8c9d-0e1f2a3b4c5d"
```

The version 4 UUID uses random bits for all positions except:
- Bits at position 48–51 (the `4` in the third group) are set to `0100` (version 4)
- Bits at position 64–65 (the high bits of `y`) are set to `10` (RFC 4122 variant)

This means there are 122 random bits, providing `2^122` possible UUIDs — approximately 5.3 × 10^36 unique values.

### `random(): string`

Generates a simple random UUID string. Unlike `uuid4()`, this does not set the version or variant bits. The result is 32 random hex characters formatted with dashes.

- Format: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
- No version bits are set
- Still passes `isValid()` because the format is correct

```titrate
let id: string = Uuid.random();
io::println(id);
// Example output: "f3e2d1c0-b9a8-7654-3210-fedcba987654"
```

Use `random()` when you need a unique identifier but do not require RFC 4122 compliance. Use `uuid4()` when interoperability with other systems is important.

### `isValid(uuid: string): bool`

Validates whether a string is a properly formatted UUID. The function checks:

1. The string is exactly 36 characters long
2. Hyphens appear at positions 8, 13, 18, and 23
3. All other characters are valid hexadecimal digits (0–9, a–f, A–F)

```titrate
let id: string = Uuid.uuid4();
io::println(Boolean.toString(Uuid.isValid(id)));     // true

io::println(Boolean.toString(Uuid.isValid("not-a-uuid"))); // false
io::println(Boolean.toString(Uuid.isValid("")));           // false
io::println(Boolean.toString(Uuid.isValid(null)));         // false

// Valid format but random content
io::println(Boolean.toString(Uuid.isValid("12345678-1234-1234-1234-123456789abc"))); // true

// Invalid: missing dashes
io::println(Boolean.toString(Uuid.isValid("12345678123412341234123456789abc"))); // false
```

### `parse(s: string)` (if available)

Parses a UUID string and extracts its components. Check your Titrate version for availability.

### `version()` (if available)

Returns the version number of a UUID. Check your Titrate version for availability.

## Using UUIDs as Map Keys

UUIDs make excellent keys for `HashMap` because they are:
- **Unique**: No two UUIDs will collide
- **Hashable**: Their string representation produces well-distributed hash codes
- **Comparable**: String comparison works correctly for UUIDs

```titrate
import tt.uuid.Uuid;
import tt.collections.HashMap;

public fn main(): void {
    let users: HashMap<string, string> = new HashMap<string, string>();

    // Generate unique IDs for each user
    let id1: string = Uuid.uuid4();
    let id2: string = Uuid.uuid4();

    HashMap.put(users, id1, "Alice");
    HashMap.put(users, id2, "Bob");

    // Look up by UUID
    io::println(HashMap.get(users, id1));  // "Alice"
    io::println(HashMap.get(users, id2));  // "Bob"
}
```

### UUID Keys vs Integer Keys

| Aspect | UUID Keys | Integer Keys |
|--------|-----------|-------------|
| Uniqueness | Globally unique | Unique within a table |
| Predictability | Unpredictable | Sequential/predictable |
| Merging | No conflicts | Conflicts likely |
| Size | 36 chars (string) | 4–8 bytes |
| Performance | Slightly slower (string hash) | Faster (integer hash) |
| Security | No enumeration risk | Easy to enumerate |

## UUID vs Auto-Increment IDs

Choosing between UUIDs and auto-incrementing integers depends on your use case:

### When to Use UUIDs

```titrate
// Distributed systems — multiple servers generating IDs
public fn createOrder(): Order {
    let orderId: string = Uuid.uuid4();
    let order: Order = new Order(orderId);
    return order;
}
```

- **Distributed systems**: Multiple servers can generate IDs without coordination
- **Public-facing IDs**: UUIDs cannot be guessed or enumerated
- **Data merging**: Datasets from different sources can be merged safely
- **Offline generation**: IDs can be created without a database connection

### When to Use Auto-Increment IDs

- **Single-database systems**: Simpler and faster
- **Human-readable references**: "Order #42" is easier to communicate than a UUID
- **Storage efficiency**: Integers take less space than UUID strings
- **Indexing performance**: Integer indexes are faster than string indexes

## Common Use Cases

### Generating Unique Identifiers

```titrate
// Create unique session IDs
public fn createSession(userId: string): Session {
    let sessionId: string = Uuid.uuid4();
    let session: Session = new Session(sessionId, userId);
    return session;
}
```

### Generating API Request IDs

```titrate
// Track API requests with unique IDs
public fn handleRequest(req: Request): Response {
    let requestId: string = Uuid.uuid4();
    log("Request " + requestId + " started");

    let response: Response = processRequest(req);

    log("Request " + requestId + " completed");
    return response;
}
```

### Creating Unique File Names

```titrate
// Avoid file name collisions with UUID prefixes
public fn saveUpload(data: string, extension: string): string {
    let id: string = Uuid.uuid4();
    let filename: string = id + "." + extension;
    File.write(filename, data);
    return filename;
}
```

### Validating User-Provided UUIDs

```titrate
// Validate UUID format before using it
public fn findUser(id: string): Result<User, string> {
    if (!Uuid.isValid(id)) {
        return err("Invalid UUID format");
    }
    let user: User = database.findUserById(id);
    if (user == null) {
        return err("User not found");
    }
    return ok(user);
}
```

### Generating Multiple UUIDs

```titrate
// Batch-generate UUIDs for a collection of items
public fn assignIds(items: ArrayList<Item>): void {
    var i: int = 0;
    while (i < items.size()) {
        let item: Item = items.get(i);
        item.id = Uuid.uuid4();
        i = i + 1;
    }
}
```

## Function Reference

| Function | Signature | Description |
|----------|-----------|-------------|
| `uuid4` | `(): string` | Generate a version 4 (random) UUID with RFC 4122 format |
| `random` | `(): string` | Generate a random UUID without version/variant bits |
| `isValid` | `(uuid: string): bool` | Validate whether a string is a properly formatted UUID |
