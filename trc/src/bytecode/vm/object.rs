// Titrate Alpha 0.2 – bytecode virtual machine: object instantiation
// Precision in every step – richie-rich90454, 2026

use super::super::frame::{Frame, FunctionDef};
use super::super::chunk::Chunk;
use super::super::value::Value;
use super::natives::lookup_builtin_native;
use super::Vm;
use std::collections::HashMap;
use std::rc::Rc;

impl Vm {
    pub(super) fn exec_new(&mut self, class_idx: u16, arg_count: u8) -> Result<(), String> {
        let ci = class_idx as usize;
        if ci >= self.classes.len() {
            return Err(format!("NEW: class index {} out of range", class_idx));
        }

        // Clone all needed data upfront to avoid borrow conflicts
        let class_name = self.classes[ci].name.clone();

        // Handle built-in pseudo-classes
        match class_name.as_str() {
            n if n.starts_with("ArrayList") => {
                let mut fields = HashMap::new();
                fields.insert("_elements".to_string(), Value::Array { elements: vec![] });
                let instance = Value::ClassInstance {
                    class_name: class_name.clone(),
                    fields: Rc::new(std::cell::RefCell::new(fields)),
                    vtable: HashMap::new(),
                };
                self.push(instance);
                return Ok(());
            }
            n if n.starts_with("HashMap") => {
                let mut fields = HashMap::new();
                fields.insert("_keys".to_string(), Value::Array { elements: vec![] });
                fields.insert("_values".to_string(), Value::Array { elements: vec![] });
                let instance = Value::ClassInstance {
                    class_name: class_name.clone(),
                    fields: Rc::new(std::cell::RefCell::new(fields)),
                    vtable: HashMap::new(),
                };
                self.push(instance);
                return Ok(());
            }
            _ => {}
        }

        // Find the constructor matching the arg count.
        // The class may have multiple constructors (overloaded by arity).
        // We search the function table for a function named "<class_name>.<init>"
        // with arity matching arg_count.  If none matches, fall back to the
        // class's default constructor field.
        let ctor_pattern = format!("{}.<init>", class_name);
        let constructor = self.functions.iter().enumerate()
            .find(|(_, f)| f.name == ctor_pattern && f.arity == arg_count as usize)
            .map(|(i, _)| i as u16)
            .or(self.classes[ci].constructor);

        let field_inits: Vec<(String, Chunk)> = self.classes[ci].field_inits.clone();
        let field_names: Vec<String> = self.classes[ci].fields.iter().map(|f| f.name.clone()).collect();
        let vtable = self.classes[ci].methods.clone();

        // Build default fields (all Null initially)
        let mut fields = HashMap::new();
        for name in &field_names {
            fields.insert(name.clone(), Value::Null);
        }

        let instance = Value::ClassInstance {
            class_name,
            fields: Rc::new(std::cell::RefCell::new(fields)),
            vtable,
        };

        // Push instance onto the stack
        self.push(instance.clone());

        // Run field initializers
        // Each field_init is a (name, Chunk) pair that computes the initial value.
        // We execute each chunk and set the field.
        for (field_name, init_chunk) in field_inits {
            // Execute the init chunk by creating a temporary function/frame
            let temp_func_idx = self.functions.len() as u16;
            self.functions.push(FunctionDef {
                name: format!("<init_{}>", field_name),
                arity: 0,
                chunk: init_chunk,
                is_method: false,
                is_constructor: false,
                local_count: 0,
                param_types: Vec::new(),
            });
            self.frames.push(Frame::new(temp_func_idx, self.stack.len()));
            // Run the init chunk
            while self.frames.last().is_some_and(|f| f.function_index == temp_func_idx) {
                self.step()?;
            }
            // The init chunk should have left a value on the stack
            let init_val = self.pop();
            // Set the field on the instance
            if let Value::ClassInstance { fields, .. } = &instance {
                fields.borrow_mut().insert(field_name, init_val);
            }
            // Remove the temporary function
            self.functions.pop();
        }

        // If class has a constructor, call it
        if let Some(ctor_idx) = constructor {
            // The stack is: [..., arg0, arg1, ..., instance]
            // We need:      [..., instance, arg0, arg1, ...]
            // Pop the instance, then insert it before the arguments.
            let instance_val = self.pop();
            // Use the actual number of args passed (from the NEW opcode)
            // rather than the constructor's arity, since they should match
            // but arg_count is the authoritative source.
            let num_args = arg_count as usize;
            let arg_start = self.stack.len() - num_args;
            self.stack.insert(arg_start, instance_val.clone());
            // Now base points to the instance (which is "this")
            let base = arg_start;
            // Pre-allocate local slots for the constructor
            let local_count = self.functions[ctor_idx as usize].local_count;
            let needed = base + local_count;
            while self.stack.len() < needed {
                self.stack.push(Value::Null);
            }
            self.frames.push(Frame::new(ctor_idx, base));
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Static calls
    // -----------------------------------------------------------------------

    pub(super) fn exec_static_call(
        &mut self,
        class_name_idx: u16,
        method_name_idx: u16,
        arg_count: u8,
    ) -> Result<(), String> {
        let (class_name, method_name) = {
            let frame = self.current_frame();
            let chunk = &self.functions[frame.function_index as usize].chunk;
            (
                chunk.strings[class_name_idx as usize].clone(),
                chunk.strings[method_name_idx as usize].clone(),
            )
        };

        if self.exec_static_part_a(&class_name, &method_name, arg_count)? {
            return Ok(());
        }
        if self.exec_static_part_b(&class_name, &method_name, arg_count)? {
            return Ok(());
        }
                // Try native function lookup (ClassName_method) FIRST.
                // The compiler converts calls like Hash_md5(x) into STATIC_CALL
                // with class_name="Hash", method_name="md5".  If a native
                // function named "Hash_md5" exists, it should be preferred over
                // a class method to avoid infinite recursion (the class method
                // itself calls Hash_md5, which would loop).
                let native_name = format!("{}_{}", class_name, method_name);
                if let Some(&native_idx) = self.native_names.get(&native_name) {
                    let arg_start = self.stack.len() - arg_count as usize;
                    let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                    let func = self.natives[native_idx as usize];
                    let result = func(&args)?;

                    // Special handling for Thread_spawn: when invoked via
                    // STATIC_CALL ("Thread", "spawn"), the native_thread_spawn
                    // function only spawns a no-op OS thread. Because Value
                    // contains Rc<> (not Send-safe), the closure argument must
                    // be executed synchronously on the calling thread. The
                    // call_native_fn path already does this; mirror the same
                    // behaviour here so STATIC_CALL Thread.spawn also runs the
                    // task closure before returning.
                    if native_idx == self.thread_spawn_idx {
                        if let Some(closure) = args.first() {
                            if matches!(closure, Value::Closure { .. }) {
                                let _ = self.call_closure_with_args(closure, &[]);
                                let _ = self.pop();
                            }
                        }
                    }

                    self.push(result);
                    return Ok(());
                }

                // Also try lookup_builtin_native in case it hasn't been registered yet
                if let Some(func) = lookup_builtin_native(&native_name) {
                    let arg_start = self.stack.len() - arg_count as usize;
                    let args: Vec<Value> = self.stack.drain(arg_start..).collect();
                    let result = func(&args)?;
                    self.push(result);
                    return Ok(());
                }

                // Prefer module-level functions over instance methods when both
                // exist.  A STATIC_CALL like Shlex.split("hello world") should
                // resolve to the top-level `split` function in the Shlex module,
                // not the instance method `split` on the Shlex class (which
                // would require a receiver).  Search the function table by
                // mangled name first.
                // Module-level functions like String.toString are compiled with
                // mangled names such as "tt.lang.String.toString".  When the
                // compiler emits a STATIC_CALL with class_name="String" and
                // method_name="toString", we search for a function whose name
                // ends with ".String.toString".  When class_name is empty
                // (bare calls like quickSort()), search by method_name suffix.
                let suffix = if class_name.is_empty() {
                    format!(".{}", method_name)
                } else {
                    format!(".{}.{}", class_name, method_name)
                };
                let exact = format!("tt.lang.{}.{}", class_name, method_name);
                let mut found_idx: Option<u16> = None;
                // Prefer exact tt.lang match first (non-method functions only,
                // since instance methods share the same mangled name pattern
                // but require a `this` receiver). When multiple overloads share
                // the same mangled name, disambiguate by arity (and, if
                // available, by parameter type names) so the correct body is
                // invoked for the given call site.
                for (i, f) in self.functions.iter().enumerate() {
                    if f.name == exact && !f.is_method && f.arity == arg_count as usize {
                        found_idx = Some(i as u16);
                        break;
                    }
                }
                // Exact-name fallback ignoring arity (legacy path) — only used
                // when no arity-matched overload exists.
                if found_idx.is_none() {
                    for (i, f) in self.functions.iter().enumerate() {
                        if f.name == exact && !f.is_method {
                            found_idx = Some(i as u16);
                            break;
                        }
                    }
                }
                // Then fall back to suffix matching (again, non-method only).
                // Apply the same arity-based disambiguation.
                if found_idx.is_none() {
                    for (i, f) in self.functions.iter().enumerate() {
                        if f.name.ends_with(&suffix) && !f.is_method && f.arity == arg_count as usize {
                            found_idx = Some(i as u16);
                            break;
                        }
                    }
                }
                if found_idx.is_none() {
                    for (i, f) in self.functions.iter().enumerate() {
                        if f.name.ends_with(&suffix) && !f.is_method {
                            found_idx = Some(i as u16);
                            break;
                        }
                    }
                }
                if let Some(func_idx) = found_idx {
                    let base = self.stack.len() - arg_count as usize;
                    self.frames.push(Frame::new(func_idx, base));
                    return Ok(());
                }

                // No module-level function found.  Try to find the class and
                // its method as a fallback.  This allows calling instance
                // methods statically (with a null `this` placeholder) for
                // methods that don't actually use `this` (e.g., Hash.md5
                // which just delegates to a native function).
                // Class names in the VM are mangled (e.g., "tt.crypto.Hash"),
                // but STATIC_CALL operands use the simple name (e.g., "Hash").
                // Try exact match first, then suffix match.
                let class_def_idx = self.classes.iter().position(|c| c.name == class_name)
                    .or_else(|| {
                        let suffix = format!(".{}", class_name);
                        self.classes.iter().position(|c| c.name.ends_with(&suffix))
                    });
                if let Some(ci) = class_def_idx {
                    let cd = &self.classes[ci];
                    if let Some(indices) = cd.methods.get(&method_name) {
                        // Pick the overload whose arity matches the call site.
                        // If none matches, fall back to the first overload.
                        let func_idx = indices.iter().copied().find(|&idx| {
                            self.functions[idx as usize].arity == arg_count as usize
                        }).or_else(|| indices.first().copied());
                        if let Some(func_idx) = func_idx {
                            // Instance methods expect `this` in slot 0. Static calls
                            // don't have a receiver, so push a null placeholder before
                            // the arguments to serve as `this`. This allows calling
                            // instance methods that don't use `this` (e.g., Hash.md5
                            // which just delegates to a native function).
                            let arg_start = self.stack.len() - arg_count as usize;
                            self.stack.insert(arg_start, Value::Null);
                            let base = arg_start;
                            let frame = Frame::new(func_idx, base);
                            let local_count = self.functions[func_idx as usize].local_count;
                            let needed = base + local_count;
                            while self.stack.len() < needed {
                                self.stack.push(Value::Null);
                            }
                            self.frames.push(frame);
                            return Ok(());
                        }
                    }
                }

                Err(format!(
                    "Unknown static call: {}.{}",
                    class_name, method_name
                ))
    }
}

// -----------------------------------------------------------------------
// Free helper functions for static calls
// -----------------------------------------------------------------------

/// Simple HMAC implementation using sha2.
/// Returns hex-encoded digest.
pub(super) fn simple_hmac(key: &[u8], msg: &[u8], algo: &str) -> String {
    use sha2::{Sha256, Sha512, Digest};
    match algo {
        "sha512" => {
            let block_size = 128usize;
            let k = if key.len() > block_size {
                let mut h = Sha512::new();
                h.update(key);
                h.finalize().to_vec()
            } else {
                key.to_vec()
            };
            let mut k_padded = vec![0u8; block_size];
            k_padded[..k.len()].copy_from_slice(&k);
            let mut ipad = vec![0x36u8; block_size];
            let mut opad = vec![0x5cu8; block_size];
            for i in 0..block_size {
                ipad[i] ^= k_padded[i];
                opad[i] ^= k_padded[i];
            }
            let mut inner = Sha512::new();
            inner.update(&ipad);
            inner.update(msg);
            let inner_hash = inner.finalize();
            let mut outer = Sha512::new();
            outer.update(&opad);
            outer.update(inner_hash);
            let result = outer.finalize();
            hex_encode(&result)
        }
        _ => {
            let block_size = 64usize;
            let k = if key.len() > block_size {
                let mut h = Sha256::new();
                h.update(key);
                h.finalize().to_vec()
            } else {
                key.to_vec()
            };
            let mut k_padded = vec![0u8; block_size];
            k_padded[..k.len()].copy_from_slice(&k);
            let mut ipad = vec![0x36u8; block_size];
            let mut opad = vec![0x5cu8; block_size];
            for i in 0..block_size {
                ipad[i] ^= k_padded[i];
                opad[i] ^= k_padded[i];
            }
            let mut inner = Sha256::new();
            inner.update(&ipad);
            inner.update(msg);
            let inner_hash = inner.finalize();
            let mut outer = Sha256::new();
            outer.update(&opad);
            outer.update(inner_hash);
            let result = outer.finalize();
            hex_encode(&result)
        }
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// Parse a struct format string like "<3b2i" into (little_endian, fields).
// fields is a Vec of (format_char, repeat_count).
// Byte order prefixes: @ = native, < = little, > ! = big.
pub(super) fn parse_struct_format(format: &str) -> (bool, Vec<(char, usize)>) {
    let mut chars = format.chars().peekable();
    let mut little_endian = cfg!(target_endian = "little");
    if let Some(&first) = chars.peek() {
        match first {
            '@' | '=' => {
                chars.next();
                little_endian = cfg!(target_endian = "little");
            }
            '<' => {
                chars.next();
                little_endian = true;
            }
            '>' | '!' => {
                chars.next();
                little_endian = false;
            }
            _ => {}
        }
    }
    let mut fields: Vec<(char, usize)> = Vec::new();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            let mut count: usize = c.to_digit(10).unwrap() as usize;
            while let Some(&next) = chars.peek() {
                if next.is_ascii_digit() {
                    count = count * 10 + next.to_digit(10).unwrap() as usize;
                    chars.next();
                } else {
                    break;
                }
            }
            if let Some(fc) = chars.next() {
                fields.push((fc, count));
            }
        } else {
            fields.push((c, 1));
        }
    }
    (little_endian, fields)
}

pub(super) fn value_to_i64(v: &Value) -> Option<i64> {
    match v {
        Value::Byte(b) => Some(*b as i64),
        Value::Short(s) => Some(*s as i64),
        Value::Int(i) => Some(*i as i64),
        Value::Long(i) => Some(*i),
        Value::Vast(i) => Some(*i as i64),
        Value::Uvast(u) => Some(*u as i64),
        Value::Bool(b) => Some(if *b { 1 } else { 0 }),
        Value::Double(d) => Some(*d as i64),
        Value::Float(f) => Some(*f as i64),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

pub(super) fn value_to_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Byte(b) => Some(*b as f64),
        Value::Short(s) => Some(*s as f64),
        Value::Int(i) => Some(*i as f64),
        Value::Long(i) => Some(*i as f64),
        Value::Vast(i) => Some(*i as f64),
        Value::Uvast(u) => Some(*u as f64),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        Value::Double(d) => Some(*d),
        Value::Float(f) => Some(*f as f64),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}
 
// ---------------------------------------------------------------------------
// ustar tar archive codec (backing Tar_build / Tar_parse)
//
// Binary data travels in Latin-1 strings (each char = one byte), the same
// convention as the zlib natives. Tar_build consumes the stdlib
// serialization "name|size|mode|mtime|isDir|isLink|linkTarget|data;..."
// and emits a real ustar archive; Tar_parse does the inverse.
 
fn tar_bytes(s: &str) -> Vec<u8> {
    s.chars().map(|c| c as u8).collect()
}
 
fn tar_latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}
 
fn tar_octal(field: &[u8]) -> Result<u64, String> {
    // Skip leading NULs/spaces (both appear in the wild), then read
    // octal digits up to the first NUL/space.
    let mut digits: Vec<u8> = Vec::new();
    let mut started = false;
    for &b in field {
        if b == 0 || b == b' ' {
            if started {
                break;
            }
            continue;
        }
        started = true;
        digits.push(b);
    }
    if digits.is_empty() {
        return Ok(0);
    }
    let s = String::from_utf8_lossy(&digits);
    u64::from_str_radix(s.trim(), 8)
        .map_err(|_| format!("Tar: invalid octal field {:?}", s))
}
 
fn tar_name(field: &[u8]) -> String {
    let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).into_owned()
}
 
fn tar_write_octal(buf: &mut [u8], value: u64) {
    // POSIX ustar: ASCII '0'-padded digits, NUL-terminated.
    for b in buf.iter_mut() {
        *b = b'0';
    }
    let s = format!("{:o}", value);
    let digits = s.as_bytes();
    let start = buf.len().saturating_sub(digits.len() + 1);
    buf[start..start + digits.len()].copy_from_slice(digits);
    buf[buf.len() - 1] = 0;
}
 
pub(super) fn tar_build_archive(serialized: &str) -> Result<String, String> {
    let mut out: Vec<u8> = Vec::new();
    if !serialized.is_empty() {
        for part in serialized.split(';') {
            if part.is_empty() {
                continue;
            }
            let f: Vec<&str> = part.split('|').collect();
            if f.len() < 7 {
                return Err("Tar_build: entry needs 7+ fields".to_string());
            }
            let name = f[0];
            let size: u64 = f[1]
                .parse()
                .map_err(|_| format!("Tar_build: bad size {:?}", f[1]))?;
            let mode: u64 = f[2]
                .parse()
                .map_err(|_| format!("Tar_build: bad mode {:?}", f[2]))?;
            let mtime: u64 = f[3]
                .parse()
                .map_err(|_| format!("Tar_build: bad mtime {:?}", f[3]))?;
            let is_dir = f[4] == "1";
            let is_link = f[5] == "1";
            let link_target = f[6];
            let data_field = if f.len() >= 8 { f[7] } else { "" };
            let name_bytes = name.as_bytes();
            if name_bytes.len() > 100 {
                return Err(format!(
                    "Tar_build: name {:?} exceeds 100-byte ustar limit",
                    name
                ));
            }
            if link_target.len() > 100 {
                return Err("Tar_build: link target exceeds 100-byte ustar limit".to_string());
            }
            let data = tar_bytes(data_field);
            if !is_dir && !is_link && data.len() as u64 != size {
                return Err(format!(
                    "Tar_build: entry {:?} declares size {} but carries {} bytes",
                    name,
                    size,
                    data.len()
                ));
            }
            let mut hdr = [0u8; 512];
            hdr[0..name_bytes.len()].copy_from_slice(name_bytes);
            tar_write_octal(&mut hdr[100..108], mode);
            tar_write_octal(&mut hdr[124..136], if is_dir || is_link { 0 } else { size });
            tar_write_octal(&mut hdr[136..148], mtime);
            hdr[156] = if is_dir {
                b'5'
            } else if is_link {
                b'2'
            } else {
                b'0'
            };
            let lt = link_target.as_bytes();
            hdr[157..157 + lt.len()].copy_from_slice(lt);
            hdr[257..262].copy_from_slice(b"ustar");
            hdr[263..265].copy_from_slice(b"00");
            let sum: u64 = hdr.iter().map(|&b| b as u64).collect::<Vec<_>>().iter().sum::<u64>() + 8 * 32;
            // Checksum field is 6 octal digits, NUL, space.
            let cs = format!("{:06o}\0 ", sum);
            hdr[148..156].copy_from_slice(cs.as_bytes());
            out.extend_from_slice(&hdr);
            if !is_dir && !is_link {
                out.extend_from_slice(&data);
                let pad = (512 - data.len() % 512) % 512;
                out.extend(std::iter::repeat_n(0u8, pad));
            }
        }
    }
    out.extend(std::iter::repeat_n(0u8, 1024));
    Ok(tar_latin1(&out))
}
 
pub(super) fn tar_parse_archive(data: &str) -> Result<String, String> {
    let bytes = tar_bytes(data);
    if !bytes.len().is_multiple_of(512) {
        return Err("Tar_parse: archive length is not a multiple of 512".to_string());
    }
    let mut parts: Vec<String> = Vec::new();
    let mut off = 0usize;
    while off + 512 <= bytes.len() {
        let hdr = &bytes[off..off + 512];
        off += 512;
        if hdr.iter().all(|&b| b == 0) {
            break;
        }
        let stored = tar_octal(&hdr[148..156])?;
        let mut tmp = [0u8; 512];
        tmp.copy_from_slice(hdr);
        for b in tmp[148..156].iter_mut() {
            *b = b' ';
        }
        let actual: u64 = tmp.iter().map(|&b| b as u64).sum();
        if stored != actual {
            return Err("Tar_parse: header checksum mismatch".to_string());
        }
        if &hdr[257..262] != b"ustar" && hdr[257..262].iter().any(|&b| b != 0) {
            return Err("Tar_parse: not a ustar archive".to_string());
        }
        let name = tar_name(&hdr[0..100]);
        let mode = tar_octal(&hdr[100..108])?;
        let size = tar_octal(&hdr[124..136])?;
        let mtime = tar_octal(&hdr[136..148])?;
        let typeflag = hdr[156];
        let link_target = tar_name(&hdr[157..257]);
        let (is_dir, is_link) = match typeflag {
            b'5' => (true, false),
            b'2' => (false, true),
            b'0' | 0 => (name.ends_with('/'), false),
            _ => {
                return Err(format!(
                    "Tar_parse: unsupported typeflag {:?}",
                    typeflag as char
                ))
            }
        };
        let blocks = size.div_ceil(512) as usize;
        if off + blocks * 512 > bytes.len() {
            return Err("Tar_parse: truncated entry data".to_string());
        }
        let payload = if is_dir || is_link {
            String::new()
        } else {
            tar_latin1(&bytes[off..off + size as usize])
        };
        off += blocks * 512;
        parts.push(format!(
            "{}|{}|{}|{}|{}|{}|{}|{}",
            name,
            size,
            mode,
            mtime,
            if is_dir { "1" } else { "0" },
            if is_link { "1" } else { "0" },
            link_target,
            payload
        ));
    }
    Ok(parts.join(";"))
}

#[cfg(test)]
mod tar_codec_tests {
    use super::{tar_build_archive, tar_parse_archive};

    #[test]
    fn tar_build_parse_roundtrip() {
        let serialized = "hello.txt|11|420|0|0|0||Hello, tar!";
        let archive = tar_build_archive(serialized).expect("build");
        assert_eq!(archive.len(), 2048);
        let back = tar_parse_archive(&archive).expect("parse");
        assert_eq!(back, serialized);
    }

    #[test]
    fn tar_build_parse_multi_entry() {
        let serialized = "a.txt|3|420|0|0|0||abc;docs|0|493|0|1|0||;link|0|0|0|0|1|a.txt|";
        let archive = tar_build_archive(serialized).expect("build");
        let back = tar_parse_archive(&archive).expect("parse");
        assert_eq!(back, serialized);
    }
}
