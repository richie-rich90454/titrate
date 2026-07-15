# Errno

The `tt.lang.Errno` module mirrors C's `<cerrno>` header. It exposes the thread-local `errno` value, the standard `E*` symbolic constants (loaded from `data/lang/errno.json`), and the `strerror`/`perror` helpers.

## Import

```titrate
import tt::lang::Errno;
```

## Constants

All `E*` constants are exposed as zero-argument functions returning the integer value defined by the platform (loaded from `data/lang/errno.json`):

- `EPERM(): int` — operation not permitted
- `ENOENT(): int` — no such file or directory
- `ESRCH(): int` — no such process
- `EINTR(): int` — interrupted system call
- `EIO(): int` — I/O error
- `ENXIO(): int` — no such device or address
- `E2BIG(): int` — argument list too long
- `ENOEXEC(): int` — exec format error
- `EBADF(): int` — bad file descriptor
- `ECHILD(): int` — no child processes
- `EAGAIN(): int` — resource temporarily unavailable
- `ENOMEM(): int` — cannot allocate memory
- `EACCES(): int` — permission denied
- `EFAULT(): int` — bad address
- `EBUSY(): int` — device or resource busy
- `EEXIST(): int` — file exists
- `EXDEV(): int` — invalid cross-device link
- `ENODEV(): int` — no such device
- `ENOTDIR(): int` — not a directory
- `EISDIR(): int` — is a directory
- `EINVAL(): int` — invalid argument
- `ENFILE(): int` — too many open files in system
- `EMFILE(): int` — too many open files
- `ENOTTY(): int` — inappropriate ioctl for device
- `ETXTBSY(): int` — text file busy
- `EFBIG(): int` — file too large
- `ENOSPC(): int` — no space left on device
- `ESPIPE(): int` — illegal seek
- `EROFS(): int` — read-only file system
- `EMLINK(): int` — too many links
- `EPIPE(): int` — broken pipe
- `EDOM(): int` — mathematics argument out of domain of function
- `ERANGE(): int` — mathematics result not representable
- `EDEADLK(): int` — resource deadlock avoided
- `ENAMETOOLONG(): int` — filename too long
- `ENOLCK(): int` — no locks available
- `ENOSYS(): int` — function not implemented
- `ENOTEMPTY(): int` — directory not empty
- `ELOOP(): int` — too many levels of symbolic links
- `ENOMSG(): int` — no message of desired type
- `EIDRM(): int` — identifier removed
- `EOVERFLOW(): int` — value too large for defined data type

The full set is enumerated in `data/lang/errno.json`; use `listNames()` to enumerate them at runtime.

## Functions

### errno

Return the current thread-local `errno` value.

**Returns:** `int`

```titrate
let e: int = errno();
if (e == ENOENT()) {
    io::println("file not found");
}
```

### setErrno

Set the thread-local `errno` value.

**Parameters:** `value: int`
**Returns:** `void`

### errnoValue

Alias for `errno()`.

### clearErrno

Reset `errno` to 0 (no error).

**Returns:** `void`

```titrate
clearErrno();
```

### strerror

Return a human-readable description of the given error number.

**Parameters:** `err: int`
**Returns:** `string`

```titrate
io::println(strerror(EACCES()));  // "Permission denied"
```

### perror

Print `<prefix>: <strerror(errno)>` to standard error, followed by a newline. If `prefix` is empty, only the error description is printed.

**Parameters:** `prefix: string`
**Returns:** `void`

```titrate
perror("open failed");
// prints: open failed: No such file or directory
```

### listNames

Return the names of all known `E*` constants.

**Returns:** `ArrayList<string>`

```titrate
let names: ArrayList<string> = listNames();
io::println(Integer.toString(names.size()));  // ~40+
```
