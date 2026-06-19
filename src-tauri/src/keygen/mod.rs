//! SSH key generation (AK-3): Ed25519 / RSA keypairs with an OS-backed CSPRNG.

use std::convert::Infallible;

use serde::Serialize;
use ssh_key::rand_core::{TryCryptoRng, TryRng};
use ssh_key::{Algorithm, HashAlg, LineEnding, PrivateKey};

use crate::error::{AppError, AppResult};

/// OS CSPRNG adapter for rand_core 0.10: implementing the fallible base traits
/// with an `Infallible` error yields `Rng`/`CryptoRng` via rand_core's blanket
/// impls. Randomness comes from the OS (`getrandom`); a failure there is fatal.
struct OsRng;

impl TryRng for OsRng {
    type Error = Infallible;
    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        Ok(getrandom::u32().expect("OS RNG unavailable"))
    }
    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        Ok(getrandom::u64().expect("OS RNG unavailable"))
    }
    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Infallible> {
        getrandom::fill(dst).expect("OS RNG unavailable");
        Ok(())
    }
}
impl TryCryptoRng for OsRng {}

/// A freshly generated keypair, in OpenSSH text format.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedKey {
    pub public_key: String,
    pub private_key: String,
    pub fingerprint: String,
}

/// Generate an Ed25519 or RSA keypair. RSA uses ssh-key's default key size.
/// CPU-bound (RSA especially) — call off the async runtime.
pub fn generate(algorithm: &str, comment: &str) -> AppResult<GeneratedKey> {
    let alg = match algorithm {
        "ed25519" => Algorithm::Ed25519,
        "rsa" => Algorithm::Rsa { hash: None },
        other => return Err(AppError::Other(format!("unsupported key type: {other}"))),
    };
    let mut key = PrivateKey::random(&mut OsRng, alg)
        .map_err(|e| AppError::Other(format!("key generation failed: {e}")))?;
    if !comment.is_empty() {
        key.set_comment(comment);
    }
    let private_key = key
        .to_openssh(LineEnding::LF)
        .map_err(|e| AppError::Other(format!("encode private key: {e}")))?
        .to_string();
    let public = key.public_key();
    let public_key = public
        .to_openssh()
        .map_err(|e| AppError::Other(format!("encode public key: {e}")))?;
    let fingerprint = public.fingerprint(HashAlg::Sha256).to_string();
    Ok(GeneratedKey {
        public_key,
        private_key,
        fingerprint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_ed25519() {
        let k = generate("ed25519", "test@ammax").expect("ed25519 keygen");
        assert!(k.public_key.starts_with("ssh-ed25519 "));
        assert!(k.private_key.contains("BEGIN OPENSSH PRIVATE KEY"));
        assert!(k.fingerprint.starts_with("SHA256:"));
        assert!(k.public_key.trim_end().ends_with("test@ammax"));
    }

    #[test]
    fn rejects_unknown_algorithm() {
        assert!(generate("dsa", "").is_err());
    }
}
