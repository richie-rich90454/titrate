// Titrate Alpha 0.4 – bytecode virtual machine: cryptoext native function stubs
// Precision in every step – richie-rich90454, 2026
//
// Stub implementations for extended cryptographic natives: Ed25519,
// Curve25519, ChaCha20-Poly1305, and HKDF. The Titrate crypto::CryptoExt
// module wraps these calls in user code; when the native runtime is
// unavailable, callers should catch the thrown error and degrade gracefully.
//
// Registering the stubs is also required to prevent infinite recursion:
// without an entry in the native table, the VM's STATIC_CALL fallback
// resolves `CryptoExt_ed25519Sign(...)` back to the module-level
// `ed25519Sign` function (matched by the `.CryptoExt.ed25519Sign` suffix),
// which would call `CryptoExt_ed25519Sign` again indefinitely.

use super::super::super::value::Value;

pub(crate) fn native_cryptoext_ed25519_sign(_args: &[Value]) -> Result<Value, String> {
    Err("CryptoExt_ed25519Sign: native crypto runtime not available".to_string())
}

pub(crate) fn native_cryptoext_ed25519_verify(_args: &[Value]) -> Result<Value, String> {
    Err("CryptoExt_ed25519Verify: native crypto runtime not available".to_string())
}

pub(crate) fn native_cryptoext_curve25519_key_exchange(_args: &[Value]) -> Result<Value, String> {
    Err("CryptoExt_curve25519KeyExchange: native crypto runtime not available".to_string())
}

pub(crate) fn native_cryptoext_chacha20_poly1305_encrypt(_args: &[Value]) -> Result<Value, String> {
    Err("CryptoExt_chacha20Poly1305Encrypt: native crypto runtime not available".to_string())
}

pub(crate) fn native_cryptoext_chacha20_poly1305_decrypt(_args: &[Value]) -> Result<Value, String> {
    Err("CryptoExt_chacha20Poly1305Decrypt: native crypto runtime not available".to_string())
}

pub(crate) fn native_cryptoext_hkdf_extract(_args: &[Value]) -> Result<Value, String> {
    Err("CryptoExt_hkdfExtract: native crypto runtime not available".to_string())
}

pub(crate) fn native_cryptoext_hkdf_expand(_args: &[Value]) -> Result<Value, String> {
    Err("CryptoExt_hkdfExpand: native crypto runtime not available".to_string())
}
