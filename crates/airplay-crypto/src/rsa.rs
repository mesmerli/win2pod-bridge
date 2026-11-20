//! RSA-OAEP encryption for RAOP (AirPlay 1) key exchange.
//!
//! The AirPlay 1 protocol encrypts the random AES session key with the
//! receiver's well-known 2048-bit RSA public key using RSA-OAEP (SHA-1).
//! This is the same key embedded in all Apple AirPlay receivers and used
//! by shairport-sync.

use airplay_core::error::CryptoError;
use base64::Engine;
use num_bigint::BigUint;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::traits::{PrivateKeyParts, PublicKeyParts};
use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};

/// Apple's well-known 2048-bit RSA public key (DER-encoded PKCS#1 modulus + exponent).
///
/// This is the same key used by shairport-sync and all RAOP implementations.
/// Modulus is 2048 bits (256 bytes), public exponent is 65537.
const APPLE_RSA_MODULUS: &[u8] = &[
    0xe7, 0xd7, 0x44, 0xf2, 0xa2, 0xe2, 0x78, 0x8b, 0x6c, 0x1f, 0x55, 0xa0, 0x8e, 0xb7, 0x05,
    0x44, 0xa8, 0xfa, 0x79, 0x45, 0xaa, 0x8b, 0xe6, 0xc6, 0x2c, 0xe5, 0xf5, 0x1c, 0xbd, 0xd4,
    0xdc, 0x68, 0x42, 0xfe, 0x3d, 0x10, 0x83, 0xdd, 0x2e, 0xde, 0xc1, 0xbf, 0xd4, 0x25, 0x2d,
    0xc0, 0x2e, 0x6f, 0x39, 0x8b, 0xdf, 0x0e, 0x61, 0x48, 0xea, 0x84, 0x85, 0x5e, 0x2e, 0x44,
    0x2d, 0xa6, 0xd6, 0x26, 0x64, 0xf6, 0x74, 0xa1, 0xf3, 0x04, 0x92, 0x9a, 0xde, 0x4f, 0x68,
    0x93, 0xef, 0x2d, 0xf6, 0xe7, 0x11, 0xa8, 0xc7, 0x7a, 0x0d, 0x91, 0xc9, 0xd9, 0x80, 0x82,
    0x2e, 0x50, 0xd1, 0x29, 0x22, 0xaf, 0xea, 0x40, 0xea, 0x9f, 0x0e, 0x14, 0xc0, 0xf7, 0x69,
    0x38, 0xc5, 0xf3, 0x88, 0x2f, 0xc0, 0x32, 0x3d, 0xd9, 0xfe, 0x55, 0x15, 0x5f, 0x51, 0xbb,
    0x59, 0x21, 0xc2, 0x01, 0x62, 0x9f, 0xd7, 0x33, 0x52, 0xd5, 0xe2, 0xef, 0xaa, 0xbf, 0x9b,
    0xa0, 0x48, 0xd7, 0xb8, 0x13, 0xa2, 0xb6, 0x76, 0x7f, 0x6c, 0x3c, 0xcf, 0x1e, 0xb4, 0xce,
    0x67, 0x3d, 0x03, 0x7b, 0x0d, 0x2e, 0xa3, 0x0c, 0x5f, 0xff, 0xeb, 0x06, 0xf8, 0xd0, 0x8a,
    0xdd, 0xe4, 0x09, 0x57, 0x1a, 0x9c, 0x68, 0x9f, 0xef, 0x10, 0x72, 0x88, 0x55, 0xdd, 0x8c,
    0xfb, 0x9a, 0x8b, 0xef, 0x5c, 0x89, 0x43, 0xef, 0x3b, 0x5f, 0xaa, 0x15, 0xdd, 0xe6, 0x98,
    0xbe, 0xdd, 0xf3, 0x59, 0x96, 0x03, 0xeb, 0x3e, 0x6f, 0x61, 0x37, 0x2b, 0xb6, 0x28, 0xf6,
    0x55, 0x9f, 0x59, 0x9a, 0x78, 0xbf, 0x50, 0x06, 0x87, 0xaa, 0x7f, 0x49, 0x76, 0xc0, 0x56,
    0x2d, 0x41, 0x29, 0x56, 0xf8, 0x98, 0x9e, 0x18, 0xa6, 0x35, 0x5b, 0xd8, 0x15, 0x97, 0x82,
    0x5e, 0x0f, 0xc8, 0x75, 0x34, 0x3e, 0xc7, 0x82, 0x11, 0x76, 0x25, 0xcd, 0xbf, 0x98, 0x44,
    0x7b,
];

const APPLE_RSA_EXPONENT: u32 = 65537;

/// Encrypt a 16-byte AES key using RSA-OAEP (SHA-1) with Apple's public key.
///
/// Returns a 256-byte (2048-bit) RSA ciphertext suitable for the SDP
/// `a=rsaaeskey:` attribute (after Base64 encoding).
pub fn encrypt_aes_key(aes_key: &[u8; 16]) -> Result<Vec<u8>, CryptoError> {
    let n = rsa::BigUint::from_bytes_be(APPLE_RSA_MODULUS);
    let e = rsa::BigUint::from(APPLE_RSA_EXPONENT);

    let public_key = RsaPublicKey::new(n, e)
        .map_err(|e| CryptoError::Encryption(format!("Invalid RSA key: {}", e)))?;

    let padding = Oaep::new::<sha1::Sha1>();
    let mut rng = rand::thread_rng();

    let ciphertext = public_key
        .encrypt(&mut rng, padding, aes_key)
        .map_err(|e| CryptoError::Encryption(format!("RSA encryption failed: {}", e)))?;

    Ok(ciphertext)
}

/// Apple's AirPort Express 2048-bit RSA private key in PKCS#1 DER format.
///
/// This key was extracted from AirPort Express hardware and is used by all RAOP
/// receivers (such as shairport-sync) to decrypt the session AES key transmitted
/// in the SDP `a=rsaaeskey:` parameter.
pub const AIRPORT_RSA_PRIVATE_KEY_DER: &[u8] = include_bytes!("airport_private_key.der");

/// Decrypt a 256-byte RSA-OAEP (SHA-1) ciphertext into a 16-byte AES session key.
///
/// Used by RAOP audio receivers to decrypt the incoming `a=rsaaeskey:` SDP parameter.
pub fn decrypt_aes_key(ciphertext: &[u8]) -> Result<[u8; 16], CryptoError> {
    let private_key = RsaPrivateKey::from_pkcs1_der(AIRPORT_RSA_PRIVATE_KEY_DER)
        .map_err(|e| CryptoError::Decryption(format!("Invalid RSA private key: {}", e)))?;

    let padding = Oaep::new::<sha1::Sha1>();
    let decrypted = private_key
        .decrypt(padding, ciphertext)
        .map_err(|e| CryptoError::Decryption(format!("RSA decryption failed: {}", e)))?;

    if decrypted.len() != 16 {
        return Err(CryptoError::Decryption(format!(
            "Expected 16-byte AES key, got {} bytes",
            decrypted.len()
        )));
    }

    let mut key = [0u8; 16];
    key.copy_from_slice(&decrypted);
    Ok(key)
}

/// Generate the `Apple-Response` header value for an incoming `Apple-Challenge`.
///
/// In Apple RAOP (AirPlay 1), the client challenges the receiver with a random nonce.
/// The receiver responds by signing:
/// `0x00 || 0x01 || 0xFF... || 0x00 || challenge || ip (4 or 16B) || mac (6B)`
/// using the AirPort Express RSA private key (s = m^d mod n), encoded in Base64 (unpadded).
pub fn generate_apple_response(
    challenge_b64: &str,
    ip: &[u8],
    mac: &[u8],
) -> Result<String, CryptoError> {
    let private_key = RsaPrivateKey::from_pkcs1_der(AIRPORT_RSA_PRIVATE_KEY_DER)
        .map_err(|e| CryptoError::Decryption(format!("Invalid RSA private key: {}", e)))?;

    let challenge = base64::engine::general_purpose::STANDARD
        .decode(challenge_b64.trim().trim_end_matches('='))
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(challenge_b64.trim()))
        .map_err(|e| CryptoError::Decryption(format!("Base64 decode challenge failed: {}", e)))?;

    let mut data = Vec::with_capacity(32);
    data.extend_from_slice(&challenge);
    data.extend_from_slice(ip);
    data.extend_from_slice(mac);
    if data.len() < 32 {
        data.resize(32, 0u8);
    }

    let input_len = data.len();
    if input_len > 256 - 3 {
        return Err(CryptoError::Decryption("Challenge data too long".to_string()));
    }

    let pad_len = 256 - input_len - 3;
    let mut buffer = Vec::with_capacity(256);
    buffer.push(0x00);
    buffer.push(0x01);
    buffer.extend(std::iter::repeat(0xFF).take(pad_len));
    buffer.push(0x00);
    buffer.extend_from_slice(&data);

    // RSA raw private key operation: s = m^d mod n
    let m = BigUint::from_bytes_be(&buffer);
    let d = BigUint::from_bytes_be(&private_key.d().to_bytes_be());
    let n = BigUint::from_bytes_be(&private_key.n().to_bytes_be());
    let s = m.modpow(&d, &n);

    let mut sig_bytes = s.to_bytes_be();
    if sig_bytes.len() < 256 {
        let mut padded = vec![0u8; 256 - sig_bytes.len()];
        padded.extend_from_slice(&sig_bytes);
        sig_bytes = padded;
    }

    Ok(base64::engine::general_purpose::STANDARD_NO_PAD.encode(&sig_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_and_decrypt_aes_key_roundtrip() {
        let original_key = [0x5A; 16];
        let encrypted = encrypt_aes_key(&original_key).expect("encryption should succeed");
        assert_eq!(encrypted.len(), 256);

        let decrypted = decrypt_aes_key(&encrypted).expect("decryption should succeed");
        assert_eq!(original_key, decrypted, "Decrypted AES key must match original key");
    }

    #[test]
    fn apple_response_verifies_with_public_key() {
        let challenge = "9OsoWRfxJXoQpz7n0dUqyg==";
        let ip = [192, 168, 1, 100];
        let mac = [0x02, 0x00, 0x00, 0x00, 0x00, 0x01];

        let response_b64 = generate_apple_response(challenge, &ip, &mac).unwrap();
        assert_eq!(response_b64.len(), 342); // 256 bytes in unpadded base64 is 342 chars

        // Verify with public key: s^e mod n
        let sig_bytes = base64::engine::general_purpose::STANDARD_NO_PAD
            .decode(&response_b64)
            .unwrap();
        assert_eq!(sig_bytes.len(), 256);

        let s = BigUint::from_bytes_be(&sig_bytes);
        let e = BigUint::from(APPLE_RSA_EXPONENT);
        let n = BigUint::from_bytes_be(APPLE_RSA_MODULUS);
        let recovered = s.modpow(&e, &n);

        let recovered_bytes = recovered.to_bytes_be();
        // PKCS#1 v1.5 padding starts with 0x01
        assert_eq!(recovered_bytes[0], 0x01);
        // Ends with challenge + ip + mac + padding
        let challenge_raw = base64::engine::general_purpose::STANDARD.decode("9OsoWRfxJXoQpz7n0dUqyg==").unwrap();
        let tail = &recovered_bytes[recovered_bytes.len() - 32..];
        assert_eq!(&tail[..challenge_raw.len()], &challenge_raw[..]);
        assert_eq!(&tail[challenge_raw.len()..challenge_raw.len() + 4], &ip);
        assert_eq!(&tail[challenge_raw.len() + 4..challenge_raw.len() + 10], &mac);
    }

    #[test]
    fn encrypt_aes_key_produces_256_bytes() {
        let aes_key = [0x42u8; 16];
        let ciphertext = encrypt_aes_key(&aes_key).unwrap();
        assert_eq!(ciphertext.len(), 256, "RSA-2048 output must be 256 bytes");
    }

    #[test]
    fn encrypt_aes_key_is_nondeterministic() {
        // OAEP uses random padding, so encrypting the same key twice should give different results
        let aes_key = [0x42u8; 16];
        let ct1 = encrypt_aes_key(&aes_key).unwrap();
        let ct2 = encrypt_aes_key(&aes_key).unwrap();
        assert_ne!(ct1, ct2, "OAEP output should be nondeterministic");
    }

    #[test]
    fn encrypt_different_keys_produces_different_output() {
        let key1 = [0x00u8; 16];
        let key2 = [0xFFu8; 16];
        let ct1 = encrypt_aes_key(&key1).unwrap();
        let ct2 = encrypt_aes_key(&key2).unwrap();
        assert_ne!(ct1, ct2);
    }
}
