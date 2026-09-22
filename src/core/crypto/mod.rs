use aes::Aes256;
use anyhow::{anyhow, Result};
use base64::Engine;
use cbc::{Decryptor, Encryptor};
use aes::cipher::{block_padding::Pkcs7, BlockEncryptMut, BlockDecryptMut, KeyIvInit};
use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use rand::{thread_rng, Rng};
use sha2::Sha256;

type Aes256CbcEnc = Encryptor<Aes256>;
type Aes256CbcDec = Decryptor<Aes256>;

/// Derives a 32-byte key and a 16-byte IV using OpenSSL's EVP_BytesToKey algorithm (MD5).
/// This perfectly matches `crypto-js` AES implementation.
pub fn evp_bytes_to_key(password: &str, salt: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut derived_key = Vec::new();
    let mut block = Vec::new();

    // We need 32 bytes for key + 16 bytes for IV = 48 bytes total
    while derived_key.len() < 48 {
        let mut hasher = Md5::new();
        hasher.update(&block);
        hasher.update(password.as_bytes());
        hasher.update(salt);
        block = hasher.finalize().to_vec();
        derived_key.extend_from_slice(&block);
    }

    let key = derived_key[0..32].to_vec();
    let iv = derived_key[32..48].to_vec();

    (key, iv)
}

/// Encrypts plaintext using AES-256-CBC, outputting a Base64 string that starts with `Salted__`.
/// Fully compatible with `CryptoJS.AES.encrypt().toString()`.
pub fn encrypt_aes(plaintext: &str, passphrase: &str) -> Result<String> {
    let mut salt = [0u8; 8];
    thread_rng().fill(&mut salt);

    let (key, iv) = evp_bytes_to_key(passphrase, &salt);

    let cipher = Aes256CbcEnc::new(key.as_slice().into(), iv.as_slice().into());
    let pt_len = plaintext.len();
    let mut buf = vec![0u8; pt_len + 32];
    buf[..pt_len].copy_from_slice(plaintext.as_bytes());
    let ciphertext = cipher.encrypt_padded_mut::<Pkcs7>(&mut buf, pt_len)
        .map_err(|_| anyhow!("Encryption padding failed"))?;

    let mut result = Vec::new();
    result.extend_from_slice(b"Salted__");
    result.extend_from_slice(&salt);
    result.extend_from_slice(ciphertext);

    Ok(base64::engine::general_purpose::STANDARD.encode(result))
}

/// Decrypts a `Salted__` Base64 AES-256-CBC string.
/// Fully compatible with `CryptoJS.AES.decrypt()`.
pub fn decrypt_aes(encrypted_base64: &str, passphrase: &str) -> Result<String> {
    let encrypted_bytes = base64::engine::general_purpose::STANDARD.decode(encrypted_base64)?;

    if encrypted_bytes.len() < 16 || &encrypted_bytes[0..8] != b"Salted__" {
        return Err(anyhow!("Invalid encrypted data format or missing Salted__ prefix"));
    }

    let salt = &encrypted_bytes[8..16];
    let ciphertext = &encrypted_bytes[16..];

    let (key, iv) = evp_bytes_to_key(passphrase, salt);

    let cipher = Aes256CbcDec::new(key.as_slice().into(), iv.as_slice().into());
    let _ct_len = ciphertext.len();
    let mut buf = ciphertext.to_vec();
    let decrypted = cipher.decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|_| anyhow!("Decryption failed (incorrect padding or passphrase)"))?;

    let plaintext = String::from_utf8(decrypted.to_vec())?;
    Ok(plaintext)
}

/// Generates an HMAC-SHA256 signature for the given encrypted string using the hashed passphrase.
pub fn generate_hmac(encrypted_value: &str, passphrase: &str) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(passphrase.as_bytes());
    let key = hasher.finalize();

    let mut mac = Hmac::<Sha256>::new_from_slice(&key)?;
    mac.update(encrypted_value.as_bytes());
    let result = mac.finalize();
    Ok(hex::encode(result.into_bytes()))
}

/// Encrypts a secret exactly how `automa-cli` EncryptSecretCommand does it.
/// Returns: `hmac_hex(64 chars) + aes_base64`
pub fn encrypt_secret(plaintext: &str, passphrase: &str) -> Result<String> {
    let encrypted_value = encrypt_aes(plaintext, passphrase)?;
    let hmac = generate_hmac(&encrypted_value, passphrase)?;
    Ok(format!("{}{}", hmac, encrypted_value))
}

/// Decrypts a secret that was encrypted using `encrypt_secret`.
/// Verifies the HMAC signature before decrypting to ensure integrity.
pub fn decrypt_secret(secret_payload: &str, passphrase: &str) -> Result<String> {
    if secret_payload.len() < 64 {
        return Err(anyhow!("Invalid secret payload: too short. Expected HMAC signature prefix."));
    }

    let hmac_signature = &secret_payload[0..64];
    let encrypted_value = &secret_payload[64..];

    let expected_hmac = generate_hmac(encrypted_value, passphrase)?;
    if hmac_signature != expected_hmac {
        return Err(anyhow!("Integrity check failed: HMAC signature does not match!"));
    }

    decrypt_aes(encrypted_value, passphrase)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evp_bytes_to_key() {
        let (key, iv) = evp_bytes_to_key("secret", b"salt1234");
        assert_eq!(key.len(), 32);
        assert_eq!(iv.len(), 16);
    }

    #[test]
    fn test_encryption_decryption_cycle() {
        let plaintext = "Hello, Automa!";
        let passphrase = "my-strong-password";

        let encrypted = encrypt_secret(plaintext, passphrase).expect("Encryption failed");
        assert!(encrypted.len() > 64);

        let decrypted = decrypt_secret(&encrypted, passphrase).expect("Decryption failed");
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_hmac_integrity() {
        let plaintext = "Important Token";
        let passphrase = "safe";

        let mut encrypted = encrypt_secret(plaintext, passphrase).unwrap();
        // Tamper with the encrypted portion
        encrypted.push('x');

        let result = decrypt_secret(&encrypted, passphrase);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Integrity check failed: HMAC signature does not match!");
    }

    #[test]
    fn test_decrypt_secret_too_short_err() {
        let result = decrypt_secret("short_payload", "pass");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn test_decrypt_aes_invalid_base64_err() {
        let result = decrypt_aes("!!!not-valid-base64!!!", "pass");
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_aes_missing_salted_prefix_err() {
        let invalid_prefix_base64 = base64::engine::general_purpose::STANDARD.encode(b"NotSaltedData123456");
        let result = decrypt_aes(&invalid_prefix_base64, "pass");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing Salted__ prefix"));
    }

    #[test]
    fn test_encryption_utf8_multibyte_and_special_chars() {
        let text = "🔒 Mật khẩu tiếng Việt có dấu: 123@#%&*!() \n\t 🚀 ~ ` ^ \0 nullbyte";
        let pass = "mật_khẩu_chủ_123";

        let enc = encrypt_secret(text, pass).unwrap();
        let dec = decrypt_secret(&enc, pass).unwrap();
        assert_eq!(text, dec);
    }
}
