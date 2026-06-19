//! Master-key-encrypted local credential vault (AK-4), for when no OS keychain
//! is available. Secrets are kept decrypted in memory only while unlocked; on
//! disk they are AES-256-GCM ciphertext keyed by an Argon2id-derived key.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use argon2::Argon2;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

const SCHEMA_VERSION: u32 = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

/// On-disk vault representation (all binary fields base64-encoded).
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VaultFile {
    schema_version: u32,
    salt: String,
    nonce: String,
    ciphertext: String,
}

/// An unlocked vault: the derived key, the KDF salt (reused on re-save), and the
/// decrypted secret map.
pub struct Vault {
    path: PathBuf,
    key: [u8; KEY_LEN],
    salt: [u8; SALT_LEN],
    secrets: HashMap<String, String>,
}

/// Tauri-managed lock state: `Some` once the vault is unlocked this session.
#[derive(Default)]
pub struct VaultState(pub Mutex<Option<Vault>>);

fn derive_key(password: &str, salt: &[u8]) -> AppResult<[u8; KEY_LEN]> {
    let mut key = [0u8; KEY_LEN];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| AppError::Other(format!("key derivation failed: {e}")))?;
    Ok(key)
}

impl Vault {
    pub fn exists(path: &Path) -> bool {
        path.exists()
    }

    /// Create a new empty vault encrypted with `password`, and persist it.
    pub fn create(path: PathBuf, password: &str) -> AppResult<Vault> {
        let mut salt = [0u8; SALT_LEN];
        getrandom::fill(&mut salt).map_err(|e| AppError::Other(format!("rng: {e}")))?;
        let key = derive_key(password, &salt)?;
        let vault = Vault {
            path,
            key,
            salt,
            secrets: HashMap::new(),
        };
        vault.persist()?;
        Ok(vault)
    }

    /// Unlock an existing vault; a wrong password surfaces as a decryption error.
    pub fn unlock(path: PathBuf, password: &str) -> AppResult<Vault> {
        let text = std::fs::read_to_string(&path)
            .map_err(|e| AppError::Other(format!("cannot read vault: {e}")))?;
        let file: VaultFile = serde_json::from_str(&text)
            .map_err(|e| AppError::Other(format!("invalid vault file: {e}")))?;
        let salt_v = B64
            .decode(&file.salt)
            .map_err(|e| AppError::Other(e.to_string()))?;
        let nonce_v = B64
            .decode(&file.nonce)
            .map_err(|e| AppError::Other(e.to_string()))?;
        let ct = B64
            .decode(&file.ciphertext)
            .map_err(|e| AppError::Other(e.to_string()))?;
        if salt_v.len() != SALT_LEN || nonce_v.len() != NONCE_LEN {
            return Err(AppError::Other("corrupt vault header".into()));
        }
        let mut salt = [0u8; SALT_LEN];
        salt.copy_from_slice(&salt_v);
        let key = derive_key(password, &salt)?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
        let plaintext = cipher
            .decrypt(Nonce::from_slice(&nonce_v), ct.as_ref())
            .map_err(|_| AppError::Other("wrong master password or corrupt vault".into()))?;
        let secrets: HashMap<String, String> = serde_json::from_slice(&plaintext)
            .map_err(|e| AppError::Other(format!("decode vault contents: {e}")))?;
        Ok(Vault {
            path,
            key,
            salt,
            secrets,
        })
    }

    fn persist(&self) -> AppResult<()> {
        let plaintext = serde_json::to_vec(&self.secrets)
            .map_err(|e| AppError::Other(format!("serialize vault: {e}")))?;
        let mut nonce = [0u8; NONCE_LEN];
        getrandom::fill(&mut nonce).map_err(|e| AppError::Other(format!("rng: {e}")))?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        let ct = cipher
            .encrypt(Nonce::from_slice(&nonce), plaintext.as_ref())
            .map_err(|e| AppError::Other(format!("encrypt vault: {e}")))?;
        let file = VaultFile {
            schema_version: SCHEMA_VERSION,
            salt: B64.encode(self.salt),
            nonce: B64.encode(nonce),
            ciphertext: B64.encode(ct),
        };
        let json = serde_json::to_string_pretty(&file)
            .map_err(|e| AppError::Other(format!("serialize vault file: {e}")))?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    pub fn set(&mut self, key: &str, value: &str) -> AppResult<()> {
        self.secrets.insert(key.to_string(), value.to_string());
        self.persist()
    }
    pub fn get(&self, key: &str) -> Option<String> {
        self.secrets.get(key).cloned()
    }
    pub fn delete(&mut self, key: &str) -> AppResult<()> {
        self.secrets.remove(key);
        self.persist()
    }
    pub fn keys(&self) -> Vec<String> {
        let mut k: Vec<String> = self.secrets.keys().cloned().collect();
        k.sort();
        k
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_wrong_password() {
        let path =
            std::env::temp_dir().join(format!("ammax_vault_test_{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let mut v = Vault::create(path.clone(), "correct horse").expect("create");
        v.set("site:1:password", "s3cret").expect("set");
        assert_eq!(v.get("site:1:password").as_deref(), Some("s3cret"));
        drop(v);

        // Re-unlock with the right password.
        let v2 = Vault::unlock(path.clone(), "correct horse").expect("unlock");
        assert_eq!(v2.get("site:1:password").as_deref(), Some("s3cret"));
        assert_eq!(v2.keys(), vec!["site:1:password".to_string()]);

        // Wrong password must fail (AES-GCM auth tag mismatch).
        assert!(Vault::unlock(path.clone(), "wrong").is_err());

        let _ = std::fs::remove_file(&path);
    }
}
