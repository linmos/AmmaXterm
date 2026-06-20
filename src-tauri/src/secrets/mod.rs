//! Secret storage backed by the OS keychain via the `keyring` crate (AK-1).
//!
//! Passwords and key passphrases are keyed by site id and stored in the OS
//! credential store (Windows Credential Manager / macOS Keychain / Linux Secret
//! Service). Nothing secret is written to disk in plaintext.
//!
//! When no OS keychain is available (e.g. a headless Linux without Secret
//! Service), the `*_pref` functions fall back to the encrypted local vault
//! (AK-4), which must be unlocked first. The keychain is always preferred.

use keyring::Entry;

use crate::error::{AppError, AppResult};
use crate::vault::VaultState;

const SERVICE: &str = "com.ammaxterm.app";

#[derive(Clone, Copy)]
pub enum SecretKind {
    Password,
    Passphrase,
}

impl SecretKind {
    fn prefix(self) -> &'static str {
        match self {
            SecretKind::Password => "password",
            SecretKind::Passphrase => "passphrase",
        }
    }
}

fn entry(kind: SecretKind, site_id: &str) -> AppResult<Entry> {
    Entry::new(SERVICE, &format!("{}:{}", kind.prefix(), site_id))
        .map_err(|e| AppError::Other(format!("keychain entry: {e}")))
}

pub fn set(kind: SecretKind, site_id: &str, value: &str) -> AppResult<()> {
    entry(kind, site_id)?
        .set_password(value)
        .map_err(|e| AppError::Other(format!("keychain set: {e}")))
}

pub fn get(kind: SecretKind, site_id: &str) -> AppResult<Option<String>> {
    match entry(kind, site_id)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Other(format!("keychain get: {e}"))),
    }
}

pub fn delete(kind: SecretKind, site_id: &str) -> AppResult<()> {
    match entry(kind, site_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Other(format!("keychain delete: {e}"))),
    }
}

/// Remove every secret for a site (called when the site is deleted).
pub fn delete_all(site_id: &str) -> AppResult<()> {
    delete(SecretKind::Password, site_id)?;
    delete(SecretKind::Passphrase, site_id)?;
    Ok(())
}

// --- Keychain-preferred / vault-fallback wrappers (AK-1 + AK-4) ---

/// Vault entry name for a site secret (mirrors the keychain account format).
fn vault_key(kind: SecretKind, site_id: &str) -> String {
    format!("{}:{}", kind.prefix(), site_id)
}

/// Store a secret, preferring the OS keychain and falling back to the encrypted
/// vault when the keychain is unavailable. The vault must be unlocked for the
/// fallback to succeed.
pub fn set_pref(kind: SecretKind, site_id: &str, value: &str, vault: &VaultState) -> AppResult<()> {
    if set(kind, site_id, value).is_ok() {
        return Ok(());
    }
    let mut guard = vault.0.lock().unwrap();
    let v = guard.as_mut().ok_or_else(|| {
        AppError::Other(
            "no OS keychain is available and the vault is locked — unlock the vault to save \
             credentials"
                .into(),
        )
    })?;
    v.set(&vault_key(kind, site_id), value)
}

/// Fetch a secret, preferring the OS keychain and falling back to the vault.
/// Returns `Ok(None)` when neither source holds it.
pub fn get_pref(kind: SecretKind, site_id: &str, vault: &VaultState) -> AppResult<Option<String>> {
    // 1) OS keychain (swallow a backend-unavailable error → try the vault).
    if let Ok(Some(v)) = get(kind, site_id) {
        return Ok(Some(v));
    }
    // 2) Encrypted vault, if unlocked.
    let guard = vault.0.lock().unwrap();
    Ok(guard
        .as_ref()
        .and_then(|v| v.get(&vault_key(kind, site_id))))
}

/// Copy every secret from one site to another (keychain-preferred, vault
/// fallback). Used when duplicating a site so the clone keeps its credentials.
/// Best-effort per kind: a missing source secret is simply skipped.
pub fn copy_all_pref(from: &str, to: &str, vault: &VaultState) -> AppResult<()> {
    for kind in [SecretKind::Password, SecretKind::Passphrase] {
        if let Some(value) = get_pref(kind, from, vault)? {
            set_pref(kind, to, &value, vault)?;
        }
    }
    Ok(())
}

/// Remove every secret for a site from both the keychain and the vault
/// (best-effort; called when a site is deleted).
pub fn delete_all_pref(site_id: &str, vault: &VaultState) -> AppResult<()> {
    let _ = delete_all(site_id);
    let mut guard = vault.0.lock().unwrap();
    if let Some(v) = guard.as_mut() {
        let _ = v.delete(&vault_key(SecretKind::Password, site_id));
        let _ = v.delete(&vault_key(SecretKind::Passphrase, site_id));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Touches the real OS keychain; run explicitly:
    //   cargo test --manifest-path src-tauri/Cargo.toml --lib secrets::tests -- --ignored --nocapture
    #[test]
    #[ignore = "touches the OS keychain"]
    fn keyring_round_trip() {
        let id = "ammax-selftest-site";
        let _ = delete_all(id);
        set(SecretKind::Password, id, "s3cr3t-pw").unwrap();
        set(SecretKind::Passphrase, id, "s3cr3t-pp").unwrap();
        assert_eq!(
            get(SecretKind::Password, id).unwrap().as_deref(),
            Some("s3cr3t-pw")
        );
        assert_eq!(
            get(SecretKind::Passphrase, id).unwrap().as_deref(),
            Some("s3cr3t-pp")
        );
        delete_all(id).unwrap();
        assert_eq!(get(SecretKind::Password, id).unwrap(), None);
        assert_eq!(get(SecretKind::Passphrase, id).unwrap(), None);
        println!("[keyring] round-trip OK against the OS keychain");
    }
}
