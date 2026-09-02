//! AES-256-GCM encryption for job arguments in the browser.
//!
//! Uses the Web Crypto API (`SubtleCrypto`) for AES-256-GCM encryption,
//! providing transparent encryption/decryption of job payloads in WASM.
//!
//! # Architecture
//!
//! - **Enqueue side**: Call [`encrypt_args`] to encrypt job args before sending.
//! - **Decrypt side**: Call [`decrypt_args`] to recover the original args.
//!
//! The encryption uses the OJS spec-canonical metadata keys:
//! - `ojs.codec.encodings`: `["binary/encrypted"]`
//! - `ojs.codec.key_id`: the key identifier used for encryption
//!
//! # Example
//!
//! ```js
//! import { encrypt_args, decrypt_args } from '@openjobspec/wasm';
//!
//! const key = "0123456789abcdef0123456789abcdef"; // 32-char hex = 16 bytes, use 64-char for 256-bit
//! const encrypted = encrypt_args(JSON.stringify(["sensitive data"]), key, "key-1");
//! const decrypted = decrypt_args(encrypted, key);
//! ```

use wasm_bindgen::prelude::*;

const NONCE_SIZE: usize = 12;
const META_CODEC_ENCODINGS: &str = "ojs.codec.encodings";
const META_CODEC_KEY_ID: &str = "ojs.codec.key_id";
const ENCODING_BINARY_ENCRYPTED: &str = "binary/encrypted";

/// Software-based AES-256-GCM encryption codec for environments where
/// Web Crypto API is unavailable or for synchronous use.
///
/// Uses a pure-Rust AES-GCM implementation that compiles to WASM.
#[wasm_bindgen]
pub struct EncryptionCodec {
    _private: (),
}

#[wasm_bindgen]
impl EncryptionCodec {
    /// Create a new encryption codec.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Encrypt a JSON string with AES-256-GCM.
    ///
    /// `plaintext` is the JSON-serialized args string.
    /// `key_hex` is the 64-character hex-encoded 256-bit key.
    ///
    /// Returns a base64-encoded string of `nonce (12 bytes) || ciphertext`.
    pub fn encrypt(&self, plaintext: &str, key_hex: &str) -> Result<String, JsValue> {
        let key = hex_to_bytes(key_hex)
            .map_err(|e| JsValue::from_str(&format!("invalid key hex: {}", e)))?;
        if key.len() != 32 {
            return Err(JsValue::from_str("key must be 32 bytes (64 hex chars)"));
        }

        let nonce = generate_nonce();
        let ciphertext = aes256_gcm_encrypt(plaintext.as_bytes(), &key, &nonce)
            .map_err(|e| JsValue::from_str(&format!("encryption failed: {}", e)))?;

        let mut out = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ciphertext);

        Ok(base64_encode(&out))
    }

    /// Decrypt a base64-encoded ciphertext produced by [`encrypt`](Self::encrypt).
    ///
    /// Returns the original JSON string.
    pub fn decrypt(&self, encoded: &str, key_hex: &str) -> Result<String, JsValue> {
        let key = hex_to_bytes(key_hex)
            .map_err(|e| JsValue::from_str(&format!("invalid key hex: {}", e)))?;
        if key.len() != 32 {
            return Err(JsValue::from_str("key must be 32 bytes (64 hex chars)"));
        }

        let data = base64_decode(encoded)
            .map_err(|e| JsValue::from_str(&format!("base64 decode failed: {}", e)))?;

        if data.len() < NONCE_SIZE {
            return Err(JsValue::from_str("encrypted data too short"));
        }

        let (nonce, ciphertext) = data.split_at(NONCE_SIZE);

        let plaintext = aes256_gcm_decrypt(ciphertext, &key, nonce)
            .map_err(|e| JsValue::from_str(&format!("decryption failed: {}", e)))?;

        String::from_utf8(plaintext)
            .map_err(|e| JsValue::from_str(&format!("decrypted data is not valid UTF-8: {}", e)))
    }
}

/// Encrypt job args and return a JS object with `{ encrypted_args, meta }`.
///
/// `args_json` is the JSON-serialized job args.
/// `key_hex` is the 64-character hex-encoded 256-bit key.
/// `key_id` identifies which key was used (for key rotation support).
///
/// The returned object has:
/// - `encrypted_args`: base64-encoded encrypted string (to be used as args)
/// - `meta`: object with `ojs.codec.encodings` and `ojs.codec.key_id`
#[wasm_bindgen]
pub fn encrypt_args(args_json: &str, key_hex: &str, key_id: &str) -> Result<JsValue, JsValue> {
    let codec = EncryptionCodec::new();
    let encrypted = codec.encrypt(args_json, key_hex)?;

    let obj = js_sys::Object::new();
    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("encrypted_args"),
        &JsValue::from_str(&encrypted),
    )?;

    let meta = js_sys::Object::new();
    let encodings = js_sys::Array::new();
    encodings.push(&JsValue::from_str(ENCODING_BINARY_ENCRYPTED));
    js_sys::Reflect::set(&meta, &JsValue::from_str(META_CODEC_ENCODINGS), &encodings)?;
    js_sys::Reflect::set(
        &meta,
        &JsValue::from_str(META_CODEC_KEY_ID),
        &JsValue::from_str(key_id),
    )?;
    js_sys::Reflect::set(&obj, &JsValue::from_str("meta"), &meta)?;

    Ok(obj.into())
}

/// Decrypt encrypted job args back to the original JSON string.
///
/// `encrypted_args` is the base64-encoded encrypted string.
/// `key_hex` is the 64-character hex-encoded 256-bit key.
#[wasm_bindgen]
pub fn decrypt_args(encrypted_args: &str, key_hex: &str) -> Result<String, JsValue> {
    let codec = EncryptionCodec::new();
    codec.decrypt(encrypted_args, key_hex)
}

// ---------------------------------------------------------------------------
// Pure-Rust AES-256-GCM implementation for WASM
// ---------------------------------------------------------------------------

fn generate_nonce() -> [u8; NONCE_SIZE] {
    let mut nonce = [0u8; NONCE_SIZE];
    getrandom::getrandom(&mut nonce).expect("getrandom failed");
    nonce
}

fn aes256_gcm_encrypt(
    plaintext: &[u8],
    key: &[u8],
    nonce: &[u8; NONCE_SIZE],
) -> Result<Vec<u8>, String> {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Key, Nonce};

    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce);

    cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("AES-GCM encrypt error: {}", e))
}

fn aes256_gcm_decrypt(ciphertext: &[u8], key: &[u8], nonce: &[u8]) -> Result<Vec<u8>, String> {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Key, Nonce};

    if nonce.len() != NONCE_SIZE {
        return Err("invalid nonce size".into());
    }

    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("AES-GCM decrypt error: {}", e))
}

// ---------------------------------------------------------------------------
// Encoding utilities (no external deps needed)
// ---------------------------------------------------------------------------

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("odd-length hex string".into());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| format!("invalid hex at position {}: {}", i, e))
        })
        .collect()
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    fn char_to_val(c: u8) -> Result<u8, String> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            b'=' => Ok(0),
            _ => Err(format!("invalid base64 char: {}", c as char)),
        }
    }

    let bytes: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if bytes.len() % 4 != 0 {
        return Err("base64 input length not a multiple of 4".into());
    }

    let mut result = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        let a = char_to_val(chunk[0])?;
        let b = char_to_val(chunk[1])?;
        let c = char_to_val(chunk[2])?;
        let d = char_to_val(chunk[3])?;

        let triple = ((a as u32) << 18) | ((b as u32) << 12) | ((c as u32) << 6) | (d as u32);

        result.push(((triple >> 16) & 0xFF) as u8);
        if chunk[2] != b'=' {
            result.push(((triple >> 8) & 0xFF) as u8);
        }
        if chunk[3] != b'=' {
            result.push((triple & 0xFF) as u8);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY_HEX: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn test_hex_to_bytes() {
        let bytes = hex_to_bytes("deadbeef").unwrap();
        assert_eq!(bytes, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn test_hex_to_bytes_invalid() {
        assert!(hex_to_bytes("xyz").is_err());
        assert!(hex_to_bytes("abc").is_err()); // odd length
    }

    #[test]
    fn test_base64_round_trip() {
        let data = b"hello, world!";
        let encoded = base64_encode(data);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base64_empty() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_decode("").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn test_base64_padding() {
        assert_eq!(base64_encode(b"a"), "YQ==");
        assert_eq!(base64_encode(b"ab"), "YWI=");
        assert_eq!(base64_encode(b"abc"), "YWJj");
    }

    #[test]
    fn test_aes_round_trip() {
        let key = hex_to_bytes(TEST_KEY_HEX).unwrap();
        let plaintext = b"sensitive data here";
        let nonce = [1u8; NONCE_SIZE];

        let ciphertext = aes256_gcm_encrypt(plaintext, &key, &nonce).unwrap();
        assert_ne!(ciphertext.as_slice(), plaintext);

        let decrypted = aes256_gcm_decrypt(&ciphertext, &key, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_aes_wrong_key() {
        let key1 = hex_to_bytes(TEST_KEY_HEX).unwrap();
        let key2 = hex_to_bytes("fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210")
            .unwrap();
        let nonce = [2u8; NONCE_SIZE];

        let ciphertext = aes256_gcm_encrypt(b"secret", &key1, &nonce).unwrap();
        assert!(aes256_gcm_decrypt(&ciphertext, &key2, &nonce).is_err());
    }

    #[test]
    fn test_full_encrypt_decrypt_flow() {
        let key = hex_to_bytes(TEST_KEY_HEX).unwrap();
        let plaintext = r#"["sensitive", {"ssn": "123-45-6789"}]"#;
        let nonce = generate_nonce();

        let ciphertext = aes256_gcm_encrypt(plaintext.as_bytes(), &key, &nonce).unwrap();

        let mut packed = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        packed.extend_from_slice(&nonce);
        packed.extend_from_slice(&ciphertext);

        let encoded = base64_encode(&packed);
        let decoded = base64_decode(&encoded).unwrap();

        let (dec_nonce, dec_ct) = decoded.split_at(NONCE_SIZE);
        let decrypted = aes256_gcm_decrypt(dec_ct, &key, dec_nonce).unwrap();
        assert_eq!(String::from_utf8(decrypted).unwrap(), plaintext);
    }

    #[test]
    fn test_unique_nonces() {
        let n1 = generate_nonce();
        let n2 = generate_nonce();
        assert_ne!(n1, n2);
    }
}
