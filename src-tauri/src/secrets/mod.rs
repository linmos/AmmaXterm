//! Secret storage backed by the OS keychain via the `keyring` crate (AK-1).
//!
//! Passwords and key passphrases are keyed by site id and stored in the OS
//! credential store (Windows Credential Manager / macOS Keychain / Linux Secret
//! Service). Nothing secret is written to disk in plaintext.

use keyring::Entry;

use crate::error::{AppError, AppResult};

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
