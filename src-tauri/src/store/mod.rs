//! Saved-site model and local persistence (`sites.json`).
//!
//! Credentials are **not** stored here — a site only references an auth method;
//! secrets (passwords, key passphrases) live in the OS keychain (AK-1). The
//! schema carries a version field and keeps room for future sync (A4).

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::tunnel::TunnelSpec;

const SCHEMA_VERSION: u32 = 2;

fn default_port() -> u16 {
    22
}

/// Per-site overrides of the global terminal/connection defaults (SM-6). Every
/// field is optional: `None` means "inherit the global setting". Appearance
/// fields are applied by the frontend per tab; `keepalive_secs` is applied at
/// connect time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteOverrides {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scrollback: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keepalive_secs: Option<u32>,
}

/// How a site authenticates. Secret material is never stored inline.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum AuthMethod {
    /// Password (fetched from the keychain at connect time).
    #[default]
    Password,
    /// Public-key auth using a private key file (passphrase from the keychain).
    /// The enum's `rename_all` only renames variants, not struct-variant fields,
    /// so name the JSON field explicitly to match the camelCase frontend (and
    /// accept the legacy `key_path` spelling from older files).
    PublicKey {
        #[serde(rename = "keyPath", alias = "key_path")]
        key_path: String,
    },
    /// Keyboard-interactive.
    KeyboardInteractive,
    /// SSH agent.
    Agent,
}

/// A saved connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(default)]
    pub auth: AuthMethod,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Tunnels auto-established when this site connects (PF-4).
    #[serde(default)]
    pub tunnels: Vec<TunnelSpec>,
    /// ProxyJump chain (TM-9): ordered ids of saved sites to hop through,
    /// client → jump[0] → … → this site. Each jump uses its own saved auth.
    #[serde(default)]
    pub proxy_jump: Vec<String>,
    /// Per-site overrides of the global defaults (SM-6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overrides: Option<SiteOverrides>,
}

/// Create/update payload from the frontend (id is assigned by the store).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteInput {
    pub name: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    #[serde(default)]
    pub auth: AuthMethod,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub tunnels: Vec<TunnelSpec>,
    #[serde(default)]
    pub proxy_jump: Vec<String>,
    #[serde(default)]
    pub overrides: Option<SiteOverrides>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SitesFile {
    schema_version: u32,
    sites: Vec<Site>,
}

/// In-memory cache of sites, persisted to `sites.json` on every mutation.
pub struct SiteStore {
    path: PathBuf,
    sites: Mutex<Vec<Site>>,
}

impl SiteStore {
    /// Load sites from `path` (missing/corrupt file → empty list).
    pub fn load(path: PathBuf) -> Self {
        let sites = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<SitesFile>(&s).ok())
            .map(|f| f.sites)
            .unwrap_or_default();
        Self {
            path,
            sites: Mutex::new(sites),
        }
    }

    fn persist(&self, sites: &[Site]) -> AppResult<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = SitesFile {
            schema_version: SCHEMA_VERSION,
            sites: sites.to_vec(),
        };
        let json = serde_json::to_string_pretty(&file)
            .map_err(|e| AppError::Other(format!("serialize sites: {e}")))?;
        // Write to a temp file then rename for an atomic replace.
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    pub fn list(&self) -> Vec<Site> {
        self.sites.lock().unwrap().clone()
    }

    pub fn get(&self, id: &str) -> AppResult<Site> {
        self.sites
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.id == id)
            .cloned()
            .ok_or_else(|| AppError::Other(format!("site not found: {id}")))
    }

    pub fn add(&self, input: SiteInput) -> AppResult<Site> {
        let site = Site {
            id: Uuid::new_v4().to_string(),
            name: input.name,
            host: input.host,
            port: input.port,
            username: input.username,
            auth: input.auth,
            group: input.group,
            tags: input.tags,
            tunnels: input.tunnels,
            proxy_jump: input.proxy_jump,
            overrides: input.overrides,
        };
        let mut guard = self.sites.lock().unwrap();
        guard.push(site.clone());
        if let Err(e) = self.persist(&guard) {
            guard.pop(); // roll back in-memory change if the write failed
            return Err(e);
        }
        Ok(site)
    }

    pub fn update(&self, id: &str, input: SiteInput) -> AppResult<Site> {
        let mut guard = self.sites.lock().unwrap();
        let idx = guard
            .iter()
            .position(|s| s.id == id)
            .ok_or_else(|| AppError::Other(format!("site not found: {id}")))?;
        let previous = guard[idx].clone();
        guard[idx] = Site {
            id: previous.id.clone(),
            name: input.name,
            host: input.host,
            port: input.port,
            username: input.username,
            auth: input.auth,
            group: input.group,
            tags: input.tags,
            tunnels: input.tunnels,
            proxy_jump: input.proxy_jump,
            overrides: input.overrides,
        };
        let updated = guard[idx].clone();
        if let Err(e) = self.persist(&guard) {
            guard[idx] = previous; // roll back
            return Err(e);
        }
        Ok(updated)
    }

    /// Write the current sites to `path` as a backup file (SM-8). Secrets are
    /// never part of `Site`, so the export carries no credentials.
    pub fn export_to(&self, path: &std::path::Path) -> AppResult<()> {
        let file = SitesFile {
            schema_version: SCHEMA_VERSION,
            sites: self.list(),
        };
        let json = serde_json::to_string_pretty(&file)
            .map_err(|e| AppError::Other(format!("serialize sites: {e}")))?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> AppResult<()> {
        let mut guard = self.sites.lock().unwrap();
        let idx = guard
            .iter()
            .position(|s| s.id == id)
            .ok_or_else(|| AppError::Other(format!("site not found: {id}")))?;
        let removed = guard.remove(idx);
        if let Err(e) = self.persist(&guard) {
            guard.insert(idx, removed); // roll back
            return Err(e);
        }
        Ok(())
    }
}
