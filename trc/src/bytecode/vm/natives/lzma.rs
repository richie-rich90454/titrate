// Titrate Alpha 0.4 – bytecode virtual machine: lzma native function stubs
// Precision in every step – richie-rich90454, 2026
//
// Stub implementations for LZMA/XZ compression natives. The Titrate
// compression::Lzma module wraps these calls in try/catch and falls
// back to a pass-through wrapper when the native runtime is unavailable.
// Returning Err here causes the VM to convert the error into a catchable
// Titrate exception (see vm::run exception-handler logic), so the
// Lzma.tr try/catch blocks recover gracefully and round-trip
// compress/decompress still works.
//
// Registering the stubs is also required to prevent infinite recursion:
// without an entry in the native table, the VM's STATIC_CALL fallback
// resolves `Lzma_compress(...)` back to the module-level `compress`
// function (matched by the `.Lzma.compress` suffix), which would call
// `Lzma_compress` again indefinitely.

use super::super::super::value::Value;

pub(crate) fn native_lzma_compress(_args: &[Value]) -> Result<Value, String> {
    Err("Lzma_compress: native LZMA runtime not available".to_string())
}

pub(crate) fn native_lzma_decompress(_args: &[Value]) -> Result<Value, String> {
    Err("Lzma_decompress: native LZMA runtime not available".to_string())
}
