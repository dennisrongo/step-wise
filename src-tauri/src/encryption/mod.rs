// At-rest encryption for the OAuth refresh token. AES-256-GCM with an Argon2id
// key derived from a machine-bound id + per-secret random salt. The token never
// touches disk in plaintext (unlike the reference `.gh-tokens.json`).
use aes_gcm::aead::{generic_array::GenericArray, Aead};
use aes_gcm::{Aes256Gcm, KeyInit};
use argon2::Argon2;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::platform;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedSecret {
    pub ciphertext: String,
    pub nonce: String,
    pub salt: String,
}

#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error("key derivation failed: {0}")]
    Kdf(String),
    #[error("cipher failure: {0}")]
    Cipher(String),
    #[error("base64 decode: {0}")]
    Decode(#[from] base64::DecodeError),
    #[error("utf-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

fn derive_key(salt: &[u8]) -> Result<[u8; 32], EncryptionError> {
    let id = platform::machine_id();
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(id.as_bytes(), salt, &mut key)
        .map_err(|e| EncryptionError::Kdf(e.to_string()))?;
    Ok(key)
}

pub fn encrypt(plaintext: &str) -> Result<EncryptedSecret, EncryptionError> {
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce);

    let key = derive_key(&salt)?;
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
    let ciphertext = cipher
        .encrypt(GenericArray::from_slice(&nonce), plaintext.as_bytes())
        .map_err(|e| EncryptionError::Cipher(e.to_string()))?;

    Ok(EncryptedSecret {
        ciphertext: B64.encode(ciphertext),
        nonce: B64.encode(nonce),
        salt: B64.encode(salt),
    })
}

pub fn decrypt(secret: &EncryptedSecret) -> Result<String, EncryptionError> {
    let salt = B64.decode(&secret.salt)?;
    let nonce = B64.decode(&secret.nonce)?;
    let ciphertext = B64.decode(&secret.ciphertext)?;

    let key = derive_key(&salt)?;
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
    let plaintext = cipher
        .decrypt(GenericArray::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|e| EncryptionError::Cipher(e.to_string()))?;

    Ok(String::from_utf8(plaintext)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let secret = "1//refresh-token-abc123";
        let enc = encrypt(secret).expect("encrypt");
        assert_ne!(enc.ciphertext, secret);
        let dec = decrypt(&enc).expect("decrypt");
        assert_eq!(dec, secret);
    }

    #[test]
    fn distinct_nonces_per_call() {
        let a = encrypt("same").unwrap();
        let b = encrypt("same").unwrap();
        assert_ne!(a.nonce, b.nonce, "each encryption must use a fresh nonce");
    }
}
