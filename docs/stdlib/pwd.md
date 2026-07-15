# Pwd

The `tt.os.Pwd` module provides a Unix password database interface. It mirrors Python's `pwd` module. On Unix the implementation reads `/etc/passwd` directly; on Windows there is no password database, so the getters return `null` and `getpwall()` returns an empty list.

## Import

```titrate
import tt::os::Pwd;
```

## Class: StructPasswd

Mirrors Python's `pwd.struct_passwd`: a sequence-like record exposing the seven standard fields of a Unix passwd entry.

**Fields:**
- `pwName: string`
- `pwPasswd: string`
- `pwUid: int`
- `pwGid: int`
- `pwGecos: string`
- `pwDir: string`
- `pwShell: string`

**Methods:**
- `getAt(index: int): string` — tuple-style index access (`0..6` map to the seven fields; numeric fields returned as strings)
- `size(): int` — always `7`
- `toString(): string` — colon-joined form

```titrate
let e: StructPasswd = new StructPasswd("alice", "x", 1000, 1000, "Alice", "/home/alice", "/bin/sh");
io::println(e.toString());
// alice:x:1000:1000:Alice:/home/alice:/bin/sh
```

## Functions

### getpwnam

Return the password database entry for the user `name`, or `null` if no such user exists or the platform does not expose a passwd database.

**Parameters:** `name: string`
**Returns:** `StructPasswd`

```titrate
let e = getpwnam("root");
if (e != null) {
    io::println(e.pwUid);  // 0
}
```

### getpwuid

Return the password database entry for the numeric user ID `uid`, or `null` if no such user exists.

**Parameters:** `uid: int`
**Returns:** `StructPasswd`

```titrate
let e = getpwuid(0);
if (e != null) {
    io::println(e.pwName);  // root
}
```

### getpwall

Return all password database entries. On Windows this is the empty list.

**Returns:** `ArrayList<StructPasswd>`

```titrate
let all = getpwall();
var i: int = 0;
while (i < all.size()) {
    io::println(all.get(i).pwName);
    i = i + 1;
}
```

### currentUser

Return the `StructPasswd` for the current user, falling back to a synthesised entry populated from environment variables if `/etc/passwd` does not contain a matching record (e.g. on Windows).

**Returns:** `StructPasswd`

```titrate
let me = currentUser();
io::println(me.pwName);
io::println(me.pwDir);
```
