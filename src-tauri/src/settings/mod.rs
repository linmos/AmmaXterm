//! Global application settings (`settings.json`): terminal appearance and
//! connection defaults (TM-11, ST-1, ST-2). Like `sites.json` it carries a
//! schema version and is persisted atomically on every change.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

const SCHEMA_VERSION: u32 = 1;

fn default_theme() -> String {
    "dark".to_string()
}
fn default_font_family() -> String {
    "Consolas, \"Cascadia Mono\", \"DejaVu Sans Mono\", monospace".to_string()
}
fn default_font_size() -> u16 {
    14
}
fn default_scrollback() -> u32 {
    5000
}
fn default_keepalive() -> u32 {
    30
}

/// User-facing global defaults. Field defaults keep older/partial files valid.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default = "default_schema")]
    pub schema_version: u32,
    /// Named theme preset resolved to colors on the frontend ("dark" | "light").
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_font_size")]
    pub font_size: u16,
    #[serde(default = "default_scrollback")]
    pub scrollback: u32,
    /// SSH keepalive interval in seconds; 0 disables (TM-8, applied at connect).
    #[serde(default = "default_keepalive")]
    pub keepalive_secs: u32,
}

fn default_schema() -> u32 {
    SCHEMA_VERSION
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            theme: default_theme(),
            font_family: default_font_family(),
            font_size: default_font_size(),
            scrollback: default_scrollback(),
            keepalive_secs: default_keepalive(),
        }
    }
}

/// In-memory settings, persisted to `settings.json` on every change.
pub struct SettingsStore {
    path: PathBuf,
    settings: Mutex<Settings>,
}

impl SettingsStore {
    /// Load from `path` (missing/corrupt → defaults).
    pub fn load(path: PathBuf) -> Self {
        let settings = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
            .unwrap_or_default();
        Self {
            path,
            settings: Mutex::new(settings),
        }
    }

    pub fn get(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    pub fn set(&self, mut next: Settings) -> AppResult<Settings> {
        next.schema_version = SCHEMA_VERSION;
        let mut guard = self.settings.lock().unwrap();
        let previous = guard.clone();
        *guard = next;
        if let Err(e) = self.persist(&guard) {
            *guard = previous; // roll back on write failure
            return Err(e);
        }
        Ok(guard.clone())
    }

    fn persist(&self, settings: &Settings) -> AppResult<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(settings)
            .map_err(|e| AppError::Other(format!("serialize settings: {e}")))?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}
