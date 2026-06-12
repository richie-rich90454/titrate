// Titrate Alpha 0.2 – bytecode virtual machine: mmap native function stubs
// Precision in every step – richie-rich90454, 2026
//
// These are stub implementations. To enable real memory-mapped file support,
// add the `memmap2` dependency to Cargo.toml.

use super::super::super::value::Value;

pub(crate) fn native_mmap_open(_args: &[Value]) -> Result<Value, String> {
    Err("Mmap not available: add memmap2 dependency".to_string())
}

pub(crate) fn native_mmap_close(_args: &[Value]) -> Result<Value, String> {
    Err("Mmap not available: add memmap2 dependency".to_string())
}

pub(crate) fn native_mmap_get(_args: &[Value]) -> Result<Value, String> {
    Err("Mmap not available: add memmap2 dependency".to_string())
}

pub(crate) fn native_mmap_set(_args: &[Value]) -> Result<Value, String> {
    Err("Mmap not available: add memmap2 dependency".to_string())
}

pub(crate) fn native_mmap_size(_args: &[Value]) -> Result<Value, String> {
    Err("Mmap not available: add memmap2 dependency".to_string())
}

pub(crate) fn native_mmap_flush(_args: &[Value]) -> Result<Value, String> {
    Err("Mmap not available: add memmap2 dependency".to_string())
}
