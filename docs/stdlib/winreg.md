# Winreg

The `tt.os.Winreg` module provides a Windows registry interface with `OpenKey`, `CreateKey`, `CloseKey`, `SetValue`, `SetValueEx`, `QueryValue`, `QueryValueEx`, `EnumKey`, `EnumValue`, `DeleteKey`, `DeleteValue`, `FlushKey`, `LoadKey`, `SaveKey`, `RestoreKey`, `ReplaceKey`, and `ExpandEnvironmentStrings`, plus the `HKEY_*`, `KEY_*`, and `REG_*` constants. It mirrors Python's `winreg` module. On non-Windows systems the registry does not exist, so mutators become graceful no-ops returning `-1`, getters return `null`/empty, and `ExpandEnvironmentStrings()` continues to work as a pure string operation.

## Import

```titrate
import tt::os::Winreg;
```

## Constants

All constants are zero-argument functions returning the platform's numeric value (loaded from `os/winreg.json`). HKEY handles are `long` because they exceed `int32` range.

### Predefined root keys

- `HKEY_CLASSES_ROOT(): long`
- `HKEY_CURRENT_USER(): long`
- `HKEY_LOCAL_MACHINE(): long`
- `HKEY_USERS(): long`
- `HKEY_PERFORMANCE_DATA(): long`
- `HKEY_CURRENT_CONFIG(): long`
- `HKEY_DYN_DATA(): long`

### `KEY_*` access rights

- `KEY_ALL_ACCESS(): int`, `KEY_CREATE_LINK(): int`, `KEY_CREATE_SUB_KEY(): int`, `KEY_ENUMERATE_SUB_KEYS(): int`, `KEY_EXECUTE(): int`, `KEY_NOTIFY(): int`, `KEY_QUERY_VALUE(): int`, `KEY_READ(): int`, `KEY_SET_VALUE(): int`, `KEY_WOW64_32KEY(): int`, `KEY_WOW64_64KEY(): int`, `KEY_WOW64_RES(): int`, `KEY_WRITE(): int`

### `REG_*` value types

- `REG_NONE(): int`, `REG_SZ(): int`, `REG_EXPAND_SZ(): int`, `REG_BINARY(): int`, `REG_DWORD(): int`, `REG_DWORD_LITTLE_ENDIAN(): int`, `REG_DWORD_BIG_ENDIAN(): int`, `REG_LINK(): int`, `REG_MULTI_SZ(): int`, `REG_RESOURCE_LIST(): int`, `REG_FULL_RESOURCE_DESCRIPTOR(): int`, `REG_RESOURCE_REQUIREMENTS_LIST(): int`, `REG_QWORD(): int`, `REG_QWORD_LITTLE_ENDIAN(): int`

## Classes

### RegistryValue

A typed registry value returned by `QueryValueEx()` and accepted by `SetValueEx()`.

**Fields:**
- `value: string`
- `type: int` — one of `REG_*`

**Constructor:** `RegistryValue(value: string, type: int)`

### RegistryEnumResult

Result of `EnumValue()`: the value's name, data, and type.

**Fields:**
- `name: string`
- `value: string`
- `type: int`

**Constructor:** `RegistryEnumResult(name: string, value: string, type: int)`

## Functions

### OpenKey / OpenKeyEx

Open the subkey `subKey` of the predefined root `key`. Returns an opaque `long` handle or `-1L` on error.

**Overloads:**
- `OpenKey(key: long, subKey: string, reserved: int, access: int): long`
- `OpenKey(key: long, subKey: string): long` — defaults `reserved=0`, `access=KEY_READ`
- `OpenKeyEx(...)` — same overloads as `OpenKey`

### CloseKey

Close a previously opened registry key handle.

**Parameters:** `handle: long`
**Returns:** `void`

### CreateKey / CreateKeyEx

Create or open the subkey `subKey` of `key`. Returns a `long` handle, or `-1L` on error.

**Overloads:**
- `CreateKey(key: long, subKey: string): long`
- `CreateKeyEx(key: long, subKey: string, reserved: int, access: int): long`

### DeleteKey

Delete the subkey `subKey` and all its values. Returns `0` on success, `-1` on error.

**Parameters:** `key: long`, `subKey: string`
**Returns:** `int`

### DeleteValue

Delete the named value from the key identified by `handle`.

**Parameters:** `handle: long`, `value: string`
**Returns:** `int`

### QueryValue

Retrieve the unnamed (default) value for `subKey` of `key`. Returns the value as a string, or `null` if it does not exist.

**Parameters:** `key: long`, `subKey: string`
**Returns:** `string`

### QueryValueEx

Retrieve the named value `valueName` from the key identified by `handle`. Returns a `RegistryValue`, or `null` if absent.

**Parameters:** `handle: long`, `valueName: string`
**Returns:** `RegistryValue`

### SetValue / SetValueEx

Associate a value with a key. `SetValue` sets the unnamed value of `subKey`; `SetValueEx` sets the named `valueName` on `handle`.

**Parameters:**
- `SetValue(key: long, subKey: string, type: int, value: string): int`
- `SetValueEx(handle: long, valueName: string, reserved: int, type: int, value: string): int`

**Returns:** `0` on success, `-1` on error

### EnumKey

Return the name of the subkey at position `index` of `handle`, or `null` if there are no more.

**Parameters:** `handle: long`, `index: int`
**Returns:** `string`

### EnumValue

Return a `RegistryEnumResult` describing the value at position `index` of `handle`, or `null` if there are no more.

**Parameters:** `handle: long`, `index: int`
**Returns:** `RegistryEnumResult`

### FlushKey

Write all attributes of `handle` to the registry.

**Parameters:** `handle: long`
**Returns:** `int` — `0` on success, `-1` on error

### ExpandEnvironmentStrings

Expand environment-variable references of the form `%NAME%` in `s` using the current process environment. This is a pure string operation and works on every platform.

**Parameters:** `s: string`
**Returns:** `string`

```titrate
let expanded = ExpandEnvironmentStrings("Path: %PATH%");
```

### LoadKey / SaveKey / RestoreKey / ReplaceKey

Hive-loading functions that require `SE_RESTORE_NAME`/`SE_BACKUP_NAME` privileges. No-ops returning `-1` on non-Windows platforms.

- `LoadKey(key: long, subKey: string, fileName: string): int`
- `SaveKey(handle: long, fileName: string): int`
- `RestoreKey(handle: long, fileName: string, flags: int): int`
- `ReplaceKey(key: long, subKey: string, newFile: string, oldFile: string): int`
