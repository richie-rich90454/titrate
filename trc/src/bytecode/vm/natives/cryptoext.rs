// Titrate Alpha 0.4 – bytecode virtual machine: cryptoext native functions
// Precision in every step – richie-rich90454, 2026
//
// Real extended cryptography, all pure-Rust:
// - Ed25519 sign/verify via `ed25519-dalek` (RFC 8032)
// - X25519 key exchange via `x25519-dalek` (RFC 7748)
// - ChaCha20-Poly1305 AEAD via `chacha20poly1305` (RFC 8439)
// - HKDF-SHA256 extract/expand via `hkdf` (RFC 5869)
//
// Keys, messages, salts, nonces, and signatures are exchanged as Latin-1
// strings (each byte → one char), preserving all 256 byte values without
// UTF-8 corruption (same convention as zlib.rs).

use super::super::super::value::Value;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::ChaCha20Poly1305;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hkdf::Hkdf;
use sha2::Sha256;
use std::rc::Rc;
use x25519_dalek::{PublicKey, StaticSecret};

/// Convert a byte slice to a Latin-1 string (each byte becomes one char).
fn bytes_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

/// Convert a Latin-1 string back to a byte vec (each char's lower 8 bits).
fn string_to_bytes(s: &str) -> Vec<u8> {
    s.chars().map(|c| c as u8).collect()
}

fn arg_bytes(args: &[Value], idx: usize, what: &str, name: &str) -> Result<Vec<u8>, String> {
    match args.get(idx) {
        Some(Value::String(s)) => Ok(string_to_bytes(s)),
        Some(other) => Err(format!("{}: {} must be a string, got {}", what, name, other.type_name())),
        None => Err(format!("{}: expected {} argument {}", what, name, idx)),
    }
}

fn fixed_bytes(bytes: Vec<u8>, what: &str, name: &str, len: usize) -> Result<Vec<u8>, String> {
    if bytes.len() == len {
        Ok(bytes)
    } else {
        Err(format!("{}: {} must be {} bytes (got {})", what, name, len, bytes.len()))
    }
}

/// Accept a 32-byte seed, or a 64-byte libsodium-style secret key (seed || pubkey).
fn signing_seed(bytes: Vec<u8>, what: &str) -> Result<[u8; 32], String> {
    match bytes.len() {
        32 => {
            let arr: [u8; 32] = bytes.try_into().expect("len checked");
            Ok(arr)
        }
        64 => {
            let arr: [u8; 32] = bytes[..32].to_vec().try_into().expect("len checked");
            Ok(arr)
        }
        n => Err(format!("{}: privateKey must be 32 bytes (got {})", what, n)),
    }
}

pub(crate) fn native_cryptoext_ed25519_sign(args: &[Value]) -> Result<Value, String> {
    let key = signing_seed(arg_bytes(args, 0, "CryptoExt_ed25519Sign", "privateKey")?, "CryptoExt_ed25519Sign")?;
    let message = arg_bytes(args, 1, "CryptoExt_ed25519Sign", "message")?;
    let signature = SigningKey::from_bytes(&key).sign(&message);
    Ok(Value::String(Rc::new(bytes_to_string(&signature.to_bytes()))))
}

pub(crate) fn native_cryptoext_ed25519_verify(args: &[Value]) -> Result<Value, String> {
    let key = fixed_bytes(
        arg_bytes(args, 0, "CryptoExt_ed25519Verify", "publicKey")?,
        "CryptoExt_ed25519Verify",
        "publicKey",
        32,
    )?;
    let message = arg_bytes(args, 1, "CryptoExt_ed25519Verify", "message")?;
    let sig = fixed_bytes(
        arg_bytes(args, 2, "CryptoExt_ed25519Verify", "signature")?,
        "CryptoExt_ed25519Verify",
        "signature",
        64,
    )?;
    let key: [u8; 32] = key.try_into().expect("len checked");
    let verifying = VerifyingKey::from_bytes(&key)        .map_err(|e| format!("CryptoExt_ed25519Verify: invalid public key: {}", e))?;
    let signature = Signature::from_slice(&sig)
        .map_err(|e| format!("CryptoExt_ed25519Verify: invalid signature: {}", e))?;
    Ok(Value::Bool(verifying.verify(&message, &signature).is_ok()))
}

pub(crate) fn native_cryptoext_curve25519_key_exchange(args: &[Value]) -> Result<Value, String> {
    let private = fixed_bytes(
        arg_bytes(args, 0, "CryptoExt_curve25519KeyExchange", "privateKey")?,
        "CryptoExt_curve25519KeyExchange",
        "privateKey",
        32,
    )?;
    let public = fixed_bytes(
        arg_bytes(args, 1, "CryptoExt_curve25519KeyExchange", "publicKey")?,
        "CryptoExt_curve25519KeyExchange",
        "publicKey",
        32,
    )?;
    let secret_bytes: [u8; 32] = private.try_into().expect("len checked");
    let peer_bytes: [u8; 32] = public.try_into().expect("len checked");
    let secret = StaticSecret::from(secret_bytes);
    let peer = PublicKey::from(peer_bytes);
    let shared = secret.diffie_hellman(&peer);
    Ok(Value::String(Rc::new(bytes_to_string(shared.as_bytes()))))
}

pub(crate) fn native_cryptoext_chacha20_poly1305_encrypt(args: &[Value]) -> Result<Value, String> {
    let key = fixed_bytes(
        arg_bytes(args, 0, "CryptoExt_chacha20Poly1305Encrypt", "key")?,
        "CryptoExt_chacha20Poly1305Encrypt",
        "key",
        32,
    )?;
    let nonce = fixed_bytes(
        arg_bytes(args, 1, "CryptoExt_chacha20Poly1305Encrypt", "nonce")?,
        "CryptoExt_chacha20Poly1305Encrypt",
        "nonce",
        12,
    )?;
    let plaintext = arg_bytes(args, 2, "CryptoExt_chacha20Poly1305Encrypt", "plaintext")?;
    let cipher = ChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&key));
    let ciphertext = cipher
        .encrypt(chacha20poly1305::Nonce::from_slice(&nonce), plaintext.as_slice())
        .map_err(|e| format!("CryptoExt_chacha20Poly1305Encrypt: {}", e))?;
    Ok(Value::String(Rc::new(bytes_to_string(&ciphertext))))
}

pub(crate) fn native_cryptoext_chacha20_poly1305_decrypt(args: &[Value]) -> Result<Value, String> {
    let key = fixed_bytes(
        arg_bytes(args, 0, "CryptoExt_chacha20Poly1305Decrypt", "key")?,
        "CryptoExt_chacha20Poly1305Decrypt",
        "key",
        32,
    )?;
    let nonce = fixed_bytes(
        arg_bytes(args, 1, "CryptoExt_chacha20Poly1305Decrypt", "nonce")?,
        "CryptoExt_chacha20Poly1305Decrypt",
        "nonce",
        12,
    )?;
    let ciphertext = arg_bytes(args, 2, "CryptoExt_chacha20Poly1305Decrypt", "ciphertext")?;
    let cipher = ChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&key));
    let plaintext = cipher
        .decrypt(chacha20poly1305::Nonce::from_slice(&nonce), ciphertext.as_slice())
        .map_err(|_| {
            "CryptoExt_chacha20Poly1305Decrypt: decryption failed (authentication error)".to_string()
        })?;
    match String::from_utf8(plaintext) {
        Ok(s) => Ok(Value::String(Rc::new(s))),
        Err(e) => Ok(Value::String(Rc::new(bytes_to_string(e.as_bytes())))),
    }
}

pub(crate) fn native_cryptoext_hkdf_extract(args: &[Value]) -> Result<Value, String> {
    let salt = arg_bytes(args, 0, "CryptoExt_hkdfExtract", "salt")?;
    let ikm = arg_bytes(args, 1, "CryptoExt_hkdfExtract", "inputKey")?;
    let (prk, _) = Hkdf::<Sha256>::extract(Some(&salt), &ikm);
    Ok(Value::String(Rc::new(bytes_to_string(&prk[..]))))
}

pub(crate) fn native_cryptoext_hkdf_expand(args: &[Value]) -> Result<Value, String> {
    let prk = arg_bytes(args, 0, "CryptoExt_hkdfExpand", "prk")?;
    let info = arg_bytes(args, 1, "CryptoExt_hkdfExpand", "info")?;
    let length = match args.get(2) {
        Some(v) => v.to_i64().ok_or_else(|| {
            format!("CryptoExt_hkdfExpand: length must be an integer, got {}", v.type_name())
        })?,
        None => return Err("CryptoExt_hkdfExpand: expected a length argument 2".to_string()),
    };
    if !(1..=255 * 32).contains(&length) {
        return Err(format!(
            "CryptoExt_hkdfExpand: length must be 1..=8160 for HKDF-SHA256 (got {})",
            length
        ));
    }
    let hk = Hkdf::<Sha256>::from_prk(&prk)
        .map_err(|_| "CryptoExt_hkdfExpand: prk must be at least 32 bytes for HKDF-SHA256".to_string())?;
    let mut okm = vec![0u8; length as usize];
    hk.expand(&info, &mut okm)
        .map_err(|e| format!("CryptoExt_hkdfExpand: {}", e))?;
    Ok(Value::String(Rc::new(bytes_to_string(&okm))))
}

#[cfg(test)]
mod cryptoext_native_tests {
    use super::*;

    fn latin(bytes: &[u8]) -> Value {
        Value::String(Rc::new(bytes.iter().map(|&b| b as char).collect()))
    }

    fn as_bytes(v: &Value) -> Vec<u8> {
        match v {
            Value::String(s) => string_to_bytes(s),
            other => panic!("expected string, got {}", other.type_name()),
        }
    }

    fn hex(s: &str) -> Vec<u8> {
        assert!(s.len() % 2 == 0);
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
            .collect()
    }

    const SEED_A: [u8; 32] = [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
        0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C,
        0x1D, 0x1E, 0x1F, 0x20,
    ];
    const SEED_B: [u8; 32] = [
        0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xAB, 0xAC, 0xAD, 0xAE,
        0xAF, 0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xBB, 0xBC,
        0xBD, 0xBE, 0xBF, 0xC0,
    ];

    #[test]
    fn ed25519_sign_verify_roundtrip() {
        let msg = b"titrate signs this";
        let sig = native_cryptoext_ed25519_sign(&[latin(&SEED_A), latin(msg)]).expect("sign");
        assert_eq!(as_bytes(&sig).len(), 64);
        let pubkey = ed25519_dalek::SigningKey::from_bytes(&SEED_A).verifying_key();
        let ok = native_cryptoext_ed25519_verify(&[
            latin(pubkey.as_bytes()),
            latin(msg),
            sig.clone(),
        ])
        .expect("verify");
        assert_eq!(ok, Value::Bool(true));
    }

    #[test]
    fn ed25519_deterministic() {
        let a = native_cryptoext_ed25519_sign(&[latin(&SEED_A), latin(b"m")]).expect("s");
        let b = native_cryptoext_ed25519_sign(&[latin(&SEED_A), latin(b"m")]).expect("s");
        assert_eq!(a, b);
    }

    #[test]
    fn ed25519_tamper_fails() {
        let sig = native_cryptoext_ed25519_sign(&[latin(&SEED_A), latin(b"original")]).expect("s");
        let pubkey = ed25519_dalek::SigningKey::from_bytes(&SEED_A).verifying_key();
        let bad_msg = native_cryptoext_ed25519_verify(&[
            latin(pubkey.as_bytes()),
            latin(b"tampered"),
            sig,
        ])
        .expect("verify");
        assert_eq!(bad_msg, Value::Bool(false));
    }

    #[test]
    fn ed25519_wrong_key_fails() {
        let sig = native_cryptoext_ed25519_sign(&[latin(&SEED_A), latin(b"m")]).expect("s");
        let other = ed25519_dalek::SigningKey::from_bytes(&SEED_B).verifying_key();
        let ok = native_cryptoext_ed25519_verify(&[latin(other.as_bytes()), latin(b"m"), sig])
            .expect("verify");
        assert_eq!(ok, Value::Bool(false));
    }

    #[test]
    fn ed25519_bad_sizes_error() {
        assert!(native_cryptoext_ed25519_sign(&[latin(&[1u8; 31]), latin(b"m")]).is_err());
        assert!(native_cryptoext_ed25519_sign(&[latin(&SEED_A)]).is_err());
        let sig = native_cryptoext_ed25519_sign(&[latin(&SEED_A), latin(b"m")]).expect("s");
        let pubkey = ed25519_dalek::SigningKey::from_bytes(&SEED_A).verifying_key();
        assert!(native_cryptoext_ed25519_verify(&[
            latin(&[9u8; 31]),
            latin(b"m"),
            sig.clone()
        ])
        .is_err());
        assert!(native_cryptoext_ed25519_verify(&[
            latin(pubkey.as_bytes()),
            latin(b"m"),
            latin(&[9u8; 63])
        ])
        .is_err());
    }

    #[test]
    fn ed25519_libsodium_secret_accepted() {
        let signing = ed25519_dalek::SigningKey::from_bytes(&SEED_A);
        let mut sk64 = signing.to_bytes().to_vec();
        sk64.extend_from_slice(signing.verifying_key().as_bytes());
        let sig = native_cryptoext_ed25519_sign(&[latin(&sk64), latin(b"m")]).expect("sign");
        assert_eq!(as_bytes(&sig).len(), 64);
    }

    fn x25519_pub(seed: &[u8; 32]) -> Vec<u8> {
        x25519_dalek::PublicKey::from(&x25519_dalek::StaticSecret::from(*seed))
            .as_bytes()
            .to_vec()
    }

    #[test]
    fn x25519_dh_symmetry() {
        let pub_a = x25519_pub(&SEED_A);
        let pub_b = x25519_pub(&SEED_B);
        let s1 = native_cryptoext_curve25519_key_exchange(&[latin(&SEED_A), latin(&pub_b)])
            .expect("dh");
        let s2 = native_cryptoext_curve25519_key_exchange(&[latin(&SEED_B), latin(&pub_a)])
            .expect("dh");
        assert_eq!(as_bytes(&s1).len(), 32);
        assert_eq!(s1, s2);
    }

    #[test]
    fn x25519_bad_sizes_error() {
        let pub_a = x25519_pub(&SEED_A);
        assert!(native_cryptoext_curve25519_key_exchange(&[latin(&[1u8; 31]), latin(&pub_a)])
            .is_err());
        assert!(native_cryptoext_curve25519_key_exchange(&[latin(&SEED_A), latin(&[1u8; 31])])
            .is_err());
    }

    const CHACHA_KEY: [u8; 32] = [0x42; 32];
    const CHACHA_NONCE: [u8; 12] = [0x24; 12];

    #[test]
    fn chacha_roundtrip() {
        let pt = b"chacha20-poly1305 secret payload";
        let ct = native_cryptoext_chacha20_poly1305_encrypt(&[
            latin(&CHACHA_KEY),
            latin(&CHACHA_NONCE),
            latin(pt),
        ])
        .expect("enc");
        // Ciphertext is plaintext + 16-byte tag.
        assert_eq!(as_bytes(&ct).len(), pt.len() + 16);
        let back = native_cryptoext_chacha20_poly1305_decrypt(&[
            latin(&CHACHA_KEY),
            latin(&CHACHA_NONCE),
            ct,
        ])
        .expect("dec");
        assert_eq!(as_bytes(&back), pt);
    }

    #[test]
    fn chacha_wrong_key_or_nonce_fails() {
        let ct = native_cryptoext_chacha20_poly1305_encrypt(&[
            latin(&CHACHA_KEY),
            latin(&CHACHA_NONCE),
            latin(b"data"),
        ])
        .expect("enc");
        let mut bad_key = CHACHA_KEY;
        bad_key[0] ^= 0xFF;
        assert!(native_cryptoext_chacha20_poly1305_decrypt(&[
            latin(&bad_key),
            latin(&CHACHA_NONCE),
            ct.clone()
        ])
        .is_err());
        let mut bad_nonce = CHACHA_NONCE;
        bad_nonce[0] ^= 0xFF;
        assert!(native_cryptoext_chacha20_poly1305_decrypt(&[
            latin(&CHACHA_KEY),
            latin(&bad_nonce),
            ct.clone()
        ])
        .is_err());
        let mut bad_ct = as_bytes(&ct);
        bad_ct[0] ^= 0xFF;
        assert!(native_cryptoext_chacha20_poly1305_decrypt(&[
            latin(&CHACHA_KEY),
            latin(&CHACHA_NONCE),
            latin(&bad_ct)
        ])
        .is_err());
    }

    #[test]
    fn chacha_bad_sizes_error() {
        assert!(native_cryptoext_chacha20_poly1305_encrypt(&[
            latin(&[1u8; 31]),
            latin(&CHACHA_NONCE),
            latin(b"x")
        ])
        .is_err());
        assert!(native_cryptoext_chacha20_poly1305_encrypt(&[
            latin(&CHACHA_KEY),
            latin(&[1u8; 11]),
            latin(b"x")
        ])
        .is_err());
    }

    #[test]
    fn hkdf_rfc5869_case1() {
        // RFC 5869 Appendix A Test Case 1 (SHA-256).
        let ikm = hex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
        let salt = hex("000102030405060708090a0b0c");
        let info = hex("f0f1f2f3f4f5f6f7f8f9");
        let prk = native_cryptoext_hkdf_extract(&[latin(&salt), latin(&ikm)]).expect("extract");
        assert_eq!(
            as_bytes(&prk),
            hex("077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5")
        );
        let okm =
            native_cryptoext_hkdf_expand(&[prk, latin(&info), Value::Int(42)]).expect("expand");
        assert_eq!(
            as_bytes(&okm),
            hex("3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865")
        );
    }

    #[test]
    fn hkdf_expand_length_rules() {
        let prk = native_cryptoext_hkdf_extract(&[latin(b"salt"), latin(b"ikm")]).expect("e");
        assert_eq!(as_bytes(&native_cryptoext_hkdf_expand(&[prk.clone(), latin(b"i"), Value::Int(1)]).expect("e")).len(), 1);
        assert_eq!(as_bytes(&native_cryptoext_hkdf_expand(&[prk.clone(), latin(b"i"), Value::Int(100)]).expect("e")).len(), 100);
        assert!(native_cryptoext_hkdf_expand(&[prk.clone(), latin(b"i"), Value::Int(0)]).is_err());
        assert!(native_cryptoext_hkdf_expand(&[prk, latin(b"i"), Value::Int(8161)]).is_err());
    }

    #[test]
    fn hkdf_short_prk_rejected() {
        assert!(native_cryptoext_hkdf_expand(&[latin(b"too short"), latin(b"i"), Value::Int(32)])
            .is_err());
    }

    #[test]
    fn hkdf_deterministic() {
        let a = native_cryptoext_hkdf_extract(&[latin(b"s"), latin(b"k")]).expect("e");
        let b = native_cryptoext_hkdf_extract(&[latin(b"s"), latin(b"k")]).expect("e");
        assert_eq!(a, b);
    }
}
