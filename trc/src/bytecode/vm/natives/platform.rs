// Titrate Alpha 0.2 – bytecode virtual machine: platform-specific natives
// Precision in every step – richie-rich90454, 2026
//
// Native support for the platform-specific stdlib modules:
//   - Windows: tt::sys::WinReg (registry), tt::sys::WinSound (beep/PCM)
//   - Unix:    tt::sys::Fcntl, tt::sys::Termios, tt::sys::Pty,
//              tt::sys::Syslog, tt::sys::Resource
//
// Each function is compiled on every platform so the lookup table can
// reference it unconditionally. On Windows the registry and sound
// functions are real (winreg / winapi crates). Unix-only functions return
// an error on other platforms, and vice versa — the same contract as
// Python, which raises on unavailable platform APIs.

use super::super::super::value::Value;

// ===========================================================================
// Shared key-handle registry (Windows)
// ===========================================================================

#[cfg(windows)]
mod winreg_impl {
    use super::Value;
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicI64, Ordering};
    use std::sync::{LazyLock, Mutex as StdMutex};

    static KEY_REGISTRY: LazyLock<StdMutex<HashMap<i64, winreg::RegKey>>> =
        LazyLock::new(|| StdMutex::new(HashMap::new()));
    static KEY_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

    pub(super) fn alloc_key(key: winreg::RegKey) -> i64 {
        let handle = KEY_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
        KEY_REGISTRY.lock().unwrap().insert(handle, key);
        handle
    }

    /// Predefined root HKEY for values 0x80000000..=0x80000006
    /// (HKEY_CLASSES_ROOT .. HKEY_DYN_DATA).
    pub(super) fn root_key(value: i64) -> Option<winreg::RegKey> {
        use winreg::enums::*;
        let hkey = match value as u64 {
            0x8000_0000 => HKEY_CLASSES_ROOT,
            0x8000_0001 => HKEY_CURRENT_USER,
            0x8000_0002 => HKEY_LOCAL_MACHINE,
            0x8000_0003 => HKEY_USERS,
            0x8000_0004 => HKEY_PERFORMANCE_DATA,
            0x8000_0005 => HKEY_CURRENT_CONFIG,
            0x8000_0006 => HKEY_DYN_DATA,
            _ => return None,
        };
        Some(winreg::RegKey::predef(hkey))
    }

    pub(super) fn with_open_key<T>(
        handle: i64,
        f: impl FnOnce(&winreg::RegKey) -> Result<T, String>,
    ) -> Result<T, String> {
        let registry = KEY_REGISTRY.lock().unwrap();
        let key = registry
            .get(&handle)
            .ok_or_else(|| "WinReg: invalid key handle".to_string())?;
        f(key)
    }

    pub(super) fn drop_key(handle: i64) -> bool {
        KEY_REGISTRY.lock().unwrap().remove(&handle).is_some()
    }

    fn arg_long(args: &[Value], idx: usize, what: &str) -> Result<i64, String> {
        match args.get(idx) {
            Some(Value::Long(h)) => Ok(*h),
            Some(Value::Int(h)) => Ok(*h as i64),
            _ => Err(format!("WinReg: {} expected an Int/Long argument", what)),
        }
    }

    fn arg_string(args: &[Value], idx: usize, what: &str) -> Result<String, String> {
        match args.get(idx) {
            Some(Value::String(s)) => Ok(s.as_str().to_string()),
            _ => Err(format!("WinReg: {} expected a String argument", what)),
        }
    }

    fn arg_int(args: &[Value], idx: usize, what: &str) -> Result<i64, String> {
        arg_long(args, idx, what)
    }

    /// Decode a raw registry value to a Titrate string. DWORD/QWORD become
    /// decimal strings, SZ/EXPAND_SZ become text, MULTI_SZ lines are joined
    /// with "\n", anything else is carried as Latin-1 bytes.
    pub(super) fn decode_value(vtype: u32, bytes: &[u8]) -> String {
        if vtype == (winreg::enums::REG_DWORD as u32) || vtype == (winreg::enums::REG_DWORD_BIG_ENDIAN as u32) {
            if bytes.len() >= 4 {
                let mut arr = [0u8; 4];
                arr.copy_from_slice(&bytes[..4]);
                let n = if vtype == (winreg::enums::REG_DWORD_BIG_ENDIAN as u32) {
                    u32::from_be_bytes(arr)
                } else {
                    u32::from_le_bytes(arr)
                };
                return n.to_string();
            }
            return String::new();
        }
        if vtype == (winreg::enums::REG_QWORD as u32) {
            if bytes.len() >= 8 {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&bytes[..8]);
                return u64::from_le_bytes(arr).to_string();
            }
            return String::new();
        }
        if vtype == (winreg::enums::REG_SZ as u32) || vtype == (winreg::enums::REG_EXPAND_SZ as u32) {
            let units: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            return String::from_utf16_lossy(&units)
                .trim_end_matches('\0')
                .to_string();
        }
        if vtype == (winreg::enums::REG_MULTI_SZ as u32) {
            let units: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let text = String::from_utf16_lossy(&units);
            return text
                .trim_end_matches('\0')
                .split('\0')
                .collect::<Vec<_>>()
                .join("\n");
        }
        bytes.iter().map(|&b| b as char).collect()
    }

    /// Encode a Titrate string argument to raw registry bytes for the given
    /// REG_* type. Returns (vtype, bytes).
    pub(super) fn encode_value(vtype: u32, value: &str) -> Result<(u32, Vec<u8>), String> {
        if vtype == (winreg::enums::REG_DWORD as u32) || vtype == (winreg::enums::REG_DWORD_BIG_ENDIAN as u32) {
            let n: u32 = value
                .trim()
                .parse()
                .map_err(|_| "WinReg: DWORD value must be a decimal integer".to_string())?;
            if vtype == (winreg::enums::REG_DWORD_BIG_ENDIAN as u32) {
                return Ok((vtype, n.to_be_bytes().to_vec()));
            }
            return Ok((vtype, n.to_le_bytes().to_vec()));
        }
        if vtype == (winreg::enums::REG_QWORD as u32) {
            let n: u64 = value
                .trim()
                .parse()
                .map_err(|_| "WinReg: QWORD value must be a decimal integer".to_string())?;
            return Ok((vtype, n.to_le_bytes().to_vec()));
        }
        if vtype == (winreg::enums::REG_SZ as u32) || vtype == (winreg::enums::REG_EXPAND_SZ as u32) {
            let mut units: Vec<u8> = Vec::with_capacity(value.len() * 2 + 2);
            for u in value.encode_utf16() {
                units.extend_from_slice(&u.to_le_bytes());
            }
            units.extend_from_slice(&[0, 0]);
            return Ok((vtype, units));
        }
        if vtype == (winreg::enums::REG_MULTI_SZ as u32) {
            let mut units: Vec<u8> = Vec::new();
            for line in value.split('\n') {
                for u in line.encode_utf16() {
                    units.extend_from_slice(&u.to_le_bytes());
                }
                units.extend_from_slice(&[0, 0]);
            }
            units.extend_from_slice(&[0, 0]);
            return Ok((vtype, units));
        }
        if vtype == (winreg::enums::REG_BINARY as u32) || vtype == (winreg::enums::REG_NONE as u32) {
            return Ok((vtype, value.chars().map(|c| c as u8).collect()));
        }
        Err(format!("WinReg: unsupported value type {}", vtype))
    }

    /// Map a REG_* integer to the winreg enum. Unknown codes are an
    /// explicit error, never silently coerced.
    fn winreg_type_from_u32(vtype: u32) -> Result<winreg::enums::RegType, String> {
        use winreg::enums::*;
        // Discriminants are the documented REG_* codes.
        if vtype == REG_NONE as u32 {
            return Ok(REG_NONE);
        }
        if vtype == REG_SZ as u32 {
            return Ok(REG_SZ);
        }
        if vtype == REG_EXPAND_SZ as u32 {
            return Ok(REG_EXPAND_SZ);
        }
        if vtype == REG_BINARY as u32 {
            return Ok(REG_BINARY);
        }
        if vtype == REG_DWORD as u32 {
            return Ok(REG_DWORD);
        }
        if vtype == REG_DWORD_BIG_ENDIAN as u32 {
            return Ok(REG_DWORD_BIG_ENDIAN);
        }
        if vtype == REG_LINK as u32 {
            return Ok(REG_LINK);
        }
        if vtype == REG_MULTI_SZ as u32 {
            return Ok(REG_MULTI_SZ);
        }
        if vtype == REG_QWORD as u32 {
            return Ok(REG_QWORD);
        }
        Err(format!("WinReg: unsupported value type {}", vtype))
    }

    /// Read the discriminant of a RegType without moving out of a borrow.
    fn regtype_to_u32(vtype: &winreg::enums::RegType) -> u32 {
        use winreg::enums::*;
        if *vtype == REG_NONE {
            return REG_NONE as u32;
        }
        if *vtype == REG_SZ {
            return REG_SZ as u32;
        }
        if *vtype == REG_EXPAND_SZ {
            return REG_EXPAND_SZ as u32;
        }
        if *vtype == REG_BINARY {
            return REG_BINARY as u32;
        }
        if *vtype == REG_DWORD {
            return REG_DWORD as u32;
        }
        if *vtype == REG_DWORD_BIG_ENDIAN {
            return REG_DWORD_BIG_ENDIAN as u32;
        }
        if *vtype == REG_LINK {
            return REG_LINK as u32;
        }
        if *vtype == REG_MULTI_SZ {
            return REG_MULTI_SZ as u32;
        }
        if *vtype == REG_QWORD {
            return REG_QWORD as u32;
        }
        // Resource-list kinds have no Titrate codec; surface the raw code.
        0xFFFF_FFFF
    }

    /// Open a key by root-or-handle + subkey path with the given access.
    pub(super) fn open_path(
        root_or_handle: i64,
        sub: &str,
        access: u32,
    ) -> Result<winreg::RegKey, String> {
        if let Some(root) = root_key(root_or_handle) {
            return root
                .open_subkey_with_flags(sub, access)
                .map_err(|e| format!("WinReg_OpenKey: cannot open '{}': {}", sub, e));
        }
        let registry = KEY_REGISTRY.lock().unwrap();
        let parent = registry
            .get(&root_or_handle)
            .ok_or_else(|| "WinReg: invalid key handle".to_string())?;
        parent
            .open_subkey_with_flags(sub, access)
            .map_err(|e| format!("WinReg_OpenKey: cannot open '{}': {}", sub, e))
    }

    pub(super) fn do_open_key(args: &[Value]) -> Result<Value, String> {
        if args.len() < 2 {
            return Err("WinReg_OpenKey: expected (rootOrHandle, subKey[, reserved[, access]])".to_string());
        }
        let root = arg_long(args, 0, "OpenKey key")?;
        let sub = arg_string(args, 1, "OpenKey subKey")?;
        let access = if args.len() >= 4 {
            arg_int(args, 3, "OpenKey access")? as u32
        } else {
            winreg::enums::KEY_READ
        };
        let key = open_path(root, &sub, access)?;
        Ok(Value::Long(alloc_key(key)))
    }

    pub(super) fn do_create_key(args: &[Value]) -> Result<Value, String> {
        if args.len() < 2 {
            return Err("WinReg_CreateKey: expected (rootOrHandle, subKey)".to_string());
        }
        let root = arg_long(args, 0, "CreateKey key")?;
        let sub = arg_string(args, 1, "CreateKey subKey")?;
        if let Some(predef) = root_key(root) {
            let (key, _) = predef
                .create_subkey(&sub)
                .map_err(|e| format!("WinReg_CreateKey: cannot create '{}': {}", sub, e))?;
            return Ok(Value::Long(alloc_key(key)));
        }
        let registry = KEY_REGISTRY.lock().unwrap();
        let parent = registry
            .get(&root)
            .ok_or_else(|| "WinReg: invalid key handle".to_string())?;
        let (key, _) = parent
            .create_subkey(&sub)
            .map_err(|e| format!("WinReg_CreateKey: cannot create '{}': {}", sub, e))?;
        drop(registry);
        Ok(Value::Long(alloc_key(key)))
    }

    pub(super) fn do_close_key(args: &[Value]) -> Result<Value, String> {
        let handle = arg_long(args, 0, "CloseKey handle")?;
        if !drop_key(handle) {
            return Err("WinReg_CloseKey: invalid key handle".to_string());
        }
        Ok(Value::Null)
    }

    /// QueryValue(handle, valueName) -> "type|data" string.
    pub(super) fn do_query_value(args: &[Value]) -> Result<Value, String> {
        if args.len() < 2 {
            return Err("WinReg_QueryValue: expected (handle, valueName)".to_string());
        }
        let handle = arg_long(args, 0, "QueryValue handle")?;
        let name = arg_string(args, 1, "QueryValue valueName")?;
        with_open_key(handle, |key| {
            let rv = key
                .get_raw_value(&name)
                .map_err(|e| format!("WinReg_QueryValue: cannot read '{}': {}", name, e))?;
            Ok(Value::String(Rc::new(format!(
                "{}|{}",
                regtype_to_u32(&rv.vtype),
                decode_value(regtype_to_u32(&rv.vtype), &rv.bytes)
            ))))
        })
    }

    /// SetValue(rootOrHandle, subKey, type, value) -> Null (sets the default
    /// value of subKey, matching Python's winreg.SetValue).
    pub(super) fn do_set_value(args: &[Value]) -> Result<Value, String> {
        if args.len() < 4 {
            return Err("WinReg_SetValue: expected (key, subKey, type, value)".to_string());
        }
        let root = arg_long(args, 0, "SetValue key")?;
        let sub = arg_string(args, 1, "SetValue subKey")?;
        let vtype = arg_int(args, 2, "SetValue type")? as u32;
        let value = arg_string(args, 3, "SetValue value")?;
        let (vtype, bytes) = encode_value(vtype, &value)?;
        let key = open_path(root, &sub, winreg::enums::KEY_SET_VALUE)?;
        key.set_raw_value("", &winreg::RegValue { vtype: winreg_type_from_u32(vtype)?, bytes })
            .map_err(|e| format!("WinReg_SetValue: cannot write: {}", e))?;
        Ok(Value::Null)
    }

    /// SetValueEx(handle, valueName, reserved, type, value) -> Null.
    pub(super) fn do_set_value_ex(args: &[Value]) -> Result<Value, String> {
        if args.len() < 5 {
            return Err("WinReg_SetValueEx: expected (handle, valueName, reserved, type, value)".to_string());
        }
        let handle = arg_long(args, 0, "SetValueEx handle")?;
        let name = arg_string(args, 1, "SetValueEx valueName")?;
        let vtype = arg_int(args, 3, "SetValueEx type")? as u32;
        let value = arg_string(args, 4, "SetValueEx value")?;
        let (vtype, bytes) = encode_value(vtype, &value)?;
        with_open_key(handle, |key| {
            key.set_raw_value(&name, &winreg::RegValue { vtype: winreg_type_from_u32(vtype)?, bytes })
                .map_err(|e| format!("WinReg_SetValueEx: cannot write '{}': {}", name, e))?;
            Ok(Value::Null)
        })
    }

    /// EnumKey(handle, index) -> String subkey name. Out of range is an
    /// error so callers can terminate enumeration.
    pub(super) fn do_enum_key(args: &[Value]) -> Result<Value, String> {
        if args.len() < 2 {
            return Err("WinReg_EnumKey: expected (handle, index)".to_string());
        }
        let handle = arg_long(args, 0, "EnumKey handle")?;
        let index = arg_int(args, 1, "EnumKey index")? as u32;
        with_open_key(handle, |key| {
            key.enum_keys()
                .nth(index as usize)
                .ok_or_else(|| "WinReg_EnumKey: no more subkeys".to_string())?
                .map(|name| Value::String(Rc::new(name)))
                .map_err(|e| format!("WinReg_EnumKey: cannot enumerate: {}", e))
        })
    }

    /// EnumValue(handle, index) -> "name|type|data". Out of range is an error.
    pub(super) fn do_enum_value(args: &[Value]) -> Result<Value, String> {
        if args.len() < 2 {
            return Err("WinReg_EnumValue: expected (handle, index)".to_string());
        }
        let handle = arg_long(args, 0, "EnumValue handle")?;
        let index = arg_int(args, 1, "EnumValue index")? as u32;
        with_open_key(handle, |key| {
            let (name, rv) = key
                .enum_values()
                .nth(index as usize)
                .ok_or_else(|| "WinReg_EnumValue: no more values".to_string())?
                .map_err(|e| format!("WinReg_EnumValue: cannot enumerate: {}", e))?;
            Ok(Value::String(Rc::new(format!(
                "{}|{}|{}",
                name,
                regtype_to_u32(&rv.vtype),
                decode_value(regtype_to_u32(&rv.vtype), &rv.bytes)
            ))))
        })
    }

    /// DeleteKey(rootOrHandle, subKey) -> Null (removes the whole subtree).
    pub(super) fn do_delete_key(args: &[Value]) -> Result<Value, String> {
        if args.len() < 2 {
            return Err("WinReg_DeleteKey: expected (key, subKey)".to_string());
        }
        let root = arg_long(args, 0, "DeleteKey key")?;
        let sub = arg_string(args, 1, "DeleteKey subKey")?;
        if let Some(predef) = root_key(root) {
            return predef
                .delete_subkey_all(&sub)
                .map(|_| Value::Null)
                .map_err(|e| format!("WinReg_DeleteKey: cannot delete '{}': {}", sub, e));
        }
        let registry = KEY_REGISTRY.lock().unwrap();
        let parent = registry
            .get(&root)
            .ok_or_else(|| "WinReg: invalid key handle".to_string())?;
        parent
            .delete_subkey_all(&sub)
            .map(|_| Value::Null)
            .map_err(|e| format!("WinReg_DeleteKey: cannot delete '{}': {}", sub, e))
    }

    /// DeleteValue(handle, valueName) -> Null.
    pub(super) fn do_delete_value(args: &[Value]) -> Result<Value, String> {
        if args.len() < 2 {
            return Err("WinReg_DeleteValue: expected (handle, valueName)".to_string());
        }
        let handle = arg_long(args, 0, "DeleteValue handle")?;
        let name = arg_string(args, 1, "DeleteValue valueName")?;
        with_open_key(handle, |key| {
            key.delete_value(&name)
                .map(|_| Value::Null)
                .map_err(|e| format!("WinReg_DeleteValue: cannot delete '{}': {}", name, e))
        })
    }
}

// ===========================================================================
// Windows Registry (WinReg_*)
// ===========================================================================

/// WinReg_OpenKey(parentHandle, subKey, [reserved], [access]) -> Long handle
pub(crate) fn native_winreg_open_key(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        winreg_impl::do_open_key(args)
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinReg_OpenKey: Windows registry is only available on Windows".to_string())
    }
}

/// WinReg_CloseKey(handle) -> Null
pub(crate) fn native_winreg_close_key(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        winreg_impl::do_close_key(args)
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinReg_CloseKey: Windows registry is only available on Windows".to_string())
    }
}

/// WinReg_QueryValue(handle, valueName) -> "type|data" String
pub(crate) fn native_winreg_query_value(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        winreg_impl::do_query_value(args)
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinReg_QueryValue: Windows registry is only available on Windows".to_string())
    }
}

/// WinReg_SetValue(handle, subKey, [reserved], [type], value) -> Null
///
/// With 5 arguments this is the SetValueEx form
/// (handle, valueName, reserved, type, value); otherwise it is the
/// SetValue form (key, subKey, type, value).
pub(crate) fn native_winreg_set_value(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        if args.len() >= 5 {
            return winreg_impl::do_set_value_ex(args);
        }
        winreg_impl::do_set_value(args)
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinReg_SetValue: Windows registry is only available on Windows".to_string())
    }
}

/// WinReg_EnumKey(handle, index) -> String
pub(crate) fn native_winreg_enum_key(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        winreg_impl::do_enum_key(args)
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinReg_EnumKey: Windows registry is only available on Windows".to_string())
    }
}

/// WinReg_EnumValue(handle, index) -> "name|type|data" String
pub(crate) fn native_winreg_enum_value(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        winreg_impl::do_enum_value(args)
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinReg_EnumValue: Windows registry is only available on Windows".to_string())
    }
}

/// WinReg_CreateKey(parentHandle, subKey, [reserved], [access]) -> Long handle
pub(crate) fn native_winreg_create_key(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        winreg_impl::do_create_key(args)
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinReg_CreateKey: Windows registry is only available on Windows".to_string())
    }
}

/// WinReg_DeleteKey(handle, subKey) -> Null
pub(crate) fn native_winreg_delete_key(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        winreg_impl::do_delete_key(args)
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinReg_DeleteKey: Windows registry is only available on Windows".to_string())
    }
}

/// WinReg_DeleteValue(handle, valueName) -> Null
pub(crate) fn native_winreg_delete_value(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        winreg_impl::do_delete_value(args)
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinReg_DeleteValue: Windows registry is only available on Windows".to_string())
    }
}

// ===========================================================================
// Windows Sound (WinSound_*)
// ===========================================================================

/// WinSound_Beep(frequency: Int, duration: Int) -> Bool
pub(crate) fn native_winsound_beep(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        let freq = match args.first() {
            Some(Value::Long(f)) => *f as u32,
            Some(Value::Int(f)) => *f as u32,
            _ => return Err("WinSound_Beep: expected an Int/Long frequency".to_string()),
        };
        let dur = match args.get(1) {
            Some(Value::Long(d)) => *d as u32,
            Some(Value::Int(d)) => *d as u32,
            _ => return Err("WinSound_Beep: expected an Int/Long duration".to_string()),
        };
        // SAFETY: Beep takes two DWORDs by value; no pointers cross the FFI.
        let ok = unsafe { winapi::um::utilapiset::Beep(freq, dur) };
        Ok(Value::Bool(ok != 0))
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinSound_Beep: Windows sound is only available on Windows".to_string())
    }
}

/// WinSound_PlaySound(sound: String, flags: Int) -> Bool
pub(crate) fn native_winsound_play_sound(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let sound = match args.first() {
            Some(Value::String(s)) => s.as_str().to_string(),
            _ => return Err("WinSound_PlaySound: expected a String sound".to_string()),
        };
        let flags = match args.get(1) {
            Some(Value::Long(f)) => *f as u32,
            Some(Value::Int(f)) => *f as u32,
            _ => return Err("WinSound_PlaySound: expected an Int/Long flags".to_string()),
        };
        let wide: Vec<u16> = std::ffi::OsStr::new(&sound)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: wide is NUL-terminated and alive for the call.
        let ok = unsafe {
            winapi::um::playsoundapi::PlaySoundW(
                wide.as_ptr(),
                std::ptr::null_mut(),
                flags,
            )
        };
        Ok(Value::Bool(ok != 0))
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinSound_PlaySound: Windows sound is only available on Windows".to_string())
    }
}

/// WinSound_MessageBeep(type: Int) -> Bool
pub(crate) fn native_winsound_message_beep(args: &[Value]) -> Result<Value, String> {
    #[cfg(windows)]
    {
        let kind = match args.first() {
            Some(Value::Long(t)) => *t as u32,
            Some(Value::Int(t)) => *t as u32,
            _ => return Err("WinSound_MessageBeep: expected an Int/Long type".to_string()),
        };
        // SAFETY: MessageBeep takes one UINT by value.
        let ok = unsafe { winapi::um::winuser::MessageBeep(kind) };
        Ok(Value::Bool(ok != 0))
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        Err("WinSound_MessageBeep: Windows sound is only available on Windows".to_string())
    }
}

// ===========================================================================
// Unix File Control (Fcntl_*)
// ===========================================================================

/// Fcntl_fcntl(fd: Int, cmd: Int, [arg]) -> Int
pub(crate) fn native_fcntl_fcntl(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Int(0))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Fcntl_fcntl: Unix file control is only available on Unix".to_string())
    }
}

/// Fcntl_ioctl(fd: Int, request: Int, [arg]) -> Int
pub(crate) fn native_fcntl_ioctl(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Int(0))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Fcntl_ioctl: Unix file control is only available on Unix".to_string())
    }
}

/// Fcntl_flock(fd: Int, operation: Int) -> Int
pub(crate) fn native_fcntl_flock(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Int(0))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Fcntl_flock: Unix file control is only available on Unix".to_string())
    }
}

/// Fcntl_lockf(fd: Int, operation: Int, length: Long) -> Int
pub(crate) fn native_fcntl_lockf(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Int(0))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Fcntl_lockf: Unix file control is only available on Unix".to_string())
    }
}

// ===========================================================================
// Unix Terminal Control (Termios_*)
// ===========================================================================

/// Termios_tcgetattr(fd: Int) -> Long handle
///
/// Returns a handle to a termios structure for the file descriptor. Stub
/// returns a fake handle; real implementation requires libc::tcgetattr.
pub(crate) fn native_termios_tcgetattr(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Long(1))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Termios_tcgetattr: Unix terminal control is only available on Unix".to_string())
    }
}

/// Termios_tcsetattr(fd: Int, when: Int, handle: Long) -> Null
pub(crate) fn native_termios_tcsetattr(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Termios_tcsetattr: Unix terminal control is only available on Unix".to_string())
    }
}

/// Termios_tcdrain(fd: Int) -> Null
pub(crate) fn native_termios_tcdrain(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Termios_tcdrain: Unix terminal control is only available on Unix".to_string())
    }
}

/// Termios_tcflush(fd: Int, queue: Int) -> Null
pub(crate) fn native_termios_tcflush(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Termios_tcflush: Unix terminal control is only available on Unix".to_string())
    }
}

/// Termios_tcsendbreak(fd: Int, duration: Int) -> Null
pub(crate) fn native_termios_tcsendbreak(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Termios_tcsendbreak: Unix terminal control is only available on Unix".to_string())
    }
}

// ===========================================================================
// Unix Pseudo-Terminal (Pty_*)
// ===========================================================================

/// Pty_openpty() -> (masterFd: Long, slaveFd: Long)
///
/// Returns a tuple of (master_fd, slave_fd). Stub returns fake descriptors.
pub(crate) fn native_pty_openpty(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Tuple {
            elements: vec![Value::Long(3), Value::Long(4)],
        })
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Pty_openpty: Unix pseudo-terminal is only available on Unix".to_string())
    }
}

/// Pty_fork() -> Int pid
///
/// Stub returns a fake pid of 1 (parent side).
pub(crate) fn native_pty_fork(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Int(1))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Pty_fork: Unix pseudo-terminal is only available on Unix".to_string())
    }
}

/// Pty_spawn(argv: Array) -> Int pid
///
/// Stub returns a fake pid of 1.
pub(crate) fn native_pty_spawn(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Int(1))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Pty_spawn: Unix pseudo-terminal is only available on Unix".to_string())
    }
}

// ===========================================================================
// Unix Syslog (Syslog_*)
// ===========================================================================

/// Syslog_openlog(ident: String, option: Int, facility: Int) -> Null
pub(crate) fn native_syslog_openlog(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Syslog_openlog: Unix syslog is only available on Unix".to_string())
    }
}

/// Syslog_syslog(priority: Int, message: String) -> Null
pub(crate) fn native_syslog_syslog(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Syslog_syslog: Unix syslog is only available on Unix".to_string())
    }
}

/// Syslog_closelog() -> Null
pub(crate) fn native_syslog_closelog(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Syslog_closelog: Unix syslog is only available on Unix".to_string())
    }
}

/// Syslog_setlogmask(mask: Int) -> Int
///
/// Returns the previous log mask. Stub returns 0.
pub(crate) fn native_syslog_setlogmask(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Int(0))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Syslog_setlogmask: Unix syslog is only available on Unix".to_string())
    }
}

// ===========================================================================
// Unix Resource Limits (Resource_*)
// ===========================================================================

/// Resource_getrlimit(resource: Int) -> (soft: Long, hard: Long)
///
/// Returns a tuple of (soft_limit, hard_limit). Stub returns (-1, -1) to
/// represent RLIM_INFINITY.
pub(crate) fn native_resource_getrlimit(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Tuple {
            elements: vec![Value::Long(-1), Value::Long(-1)],
        })
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Resource_getrlimit: Unix resource limits are only available on Unix".to_string())
    }
}

/// Resource_setrlimit(resource: Int, soft: Long, hard: Long) -> Null
pub(crate) fn native_resource_setrlimit(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Null)
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Resource_setrlimit: Unix resource limits are only available on Unix".to_string())
    }
}

/// Resource_getrusage(who: Int) -> Long handle
///
/// Returns a handle to an rusage structure. Stub returns a fake handle.
pub(crate) fn native_resource_getrusage(args: &[Value]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let _ = args;
        Ok(Value::Long(1))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Resource_getrusage: Unix resource limits are only available on Unix".to_string())
    }
}
