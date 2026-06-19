//! Importing saved connections from external sources (SM-7) and reading
//! AmmaXterm backup files (SM-8 restore).
//!
//! Parsers here are deliberately lenient: an unrecognised or malformed entry is
//! skipped rather than failing the whole import. Nothing is written to the store
//! from this module — it only produces preview candidates that the frontend
//! confirms before calling `site_add`.

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::store::AuthMethod;

fn default_port() -> u16 {
    22
}

/// A connection candidate produced by an importer, shown for review before it is
/// turned into a real saved site. Mirrors the fields of `SiteInput` (no id, no
/// secrets).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedSite {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: AuthMethod,
    pub group: Option<String>,
    pub tags: Vec<String>,
}

/// Split a config line into a directive keyword and its value, accepting both
/// `Key value` and `Key=value` forms (OpenSSH allows either).
fn split_directive(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (key, val) = match line.find(['=', ' ', '\t']) {
        Some(i) => (
            line[..i].trim(),
            line[i + 1..].trim_start_matches(['=', ' ', '\t']),
        ),
        None => return None,
    };
    let val = val.trim().trim_matches('"');
    if key.is_empty() {
        return None;
    }
    Some((key.to_ascii_lowercase(), val.to_string()))
}

#[derive(Default)]
struct HostBlock {
    aliases: Vec<String>,
    hostname: Option<String>,
    user: Option<String>,
    port: Option<u16>,
    identity: Option<String>,
}

impl HostBlock {
    /// Emit one candidate per concrete (wildcard-free) alias in the block.
    fn finish(self, out: &mut Vec<ImportedSite>) {
        for alias in &self.aliases {
            if alias.contains(['*', '?', '!']) {
                continue; // pattern/default block, not a connectable host
            }
            let host = self.hostname.clone().unwrap_or_else(|| alias.clone());
            let auth = match &self.identity {
                Some(path) => AuthMethod::PublicKey {
                    key_path: path.clone(),
                },
                None => AuthMethod::Password,
            };
            out.push(ImportedSite {
                name: alias.clone(),
                host,
                port: self.port.unwrap_or(22),
                username: self.user.clone().unwrap_or_default(),
                auth,
                group: Some("ssh-config".to_string()),
                tags: vec!["imported".to_string()],
            });
        }
    }
}

/// Parse an OpenSSH `config` file into connection candidates (SM-7).
pub fn parse_openssh_config(text: &str) -> Vec<ImportedSite> {
    let mut out = Vec::new();
    let mut cur: Option<HostBlock> = None;

    for line in text.lines() {
        let Some((key, val)) = split_directive(line) else {
            continue;
        };
        if key == "host" {
            if let Some(block) = cur.take() {
                block.finish(&mut out);
            }
            let aliases = val.split_whitespace().map(str::to_string).collect();
            cur = Some(HostBlock {
                aliases,
                ..Default::default()
            });
        } else if let Some(block) = cur.as_mut() {
            match key.as_str() {
                "hostname" => block.hostname = Some(val),
                "user" => block.user = Some(val),
                "port" => block.port = val.parse().ok(),
                // Take the first IdentityFile only (OpenSSH allows several).
                "identityfile" if block.identity.is_none() => block.identity = Some(val),
                _ => {}
            }
        }
    }
    if let Some(block) = cur.take() {
        block.finish(&mut out);
    }
    out
}

/// Shape of an AmmaXterm `sites.json` / backup file (read-only, lenient).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupFile {
    #[serde(default)]
    sites: Vec<BackupSite>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupSite {
    name: String,
    host: String,
    #[serde(default = "default_port")]
    port: u16,
    username: String,
    #[serde(default)]
    auth: AuthMethod,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

/// Read an AmmaXterm backup file into connection candidates (SM-8 restore).
pub fn read_backup(text: &str) -> AppResult<Vec<ImportedSite>> {
    let file: BackupFile =
        serde_json::from_str(text).map_err(|e| AppError::Other(format!("invalid backup: {e}")))?;
    Ok(file
        .sites
        .into_iter()
        .map(|s| ImportedSite {
            name: s.name,
            host: s.host,
            port: s.port,
            username: s.username,
            auth: s.auth,
            group: s.group,
            tags: s.tags,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_config() {
        let cfg = "\
# a comment
Host web prod-web
    HostName 10.0.0.5
    User deploy
    Port 2222
    IdentityFile ~/.ssh/id_ed25519

Host *
    User nobody

Host db
    HostName db.internal
";
        let sites = parse_openssh_config(cfg);
        // 'web' + 'prod-web' (shared block) + 'db'; the '*' block is skipped.
        assert_eq!(sites.len(), 3);

        let web = &sites[0];
        assert_eq!(web.name, "web");
        assert_eq!(web.host, "10.0.0.5");
        assert_eq!(web.username, "deploy");
        assert_eq!(web.port, 2222);
        assert!(matches!(web.auth, AuthMethod::PublicKey { .. }));

        // Second alias of the same block inherits its settings.
        assert_eq!(sites[1].name, "prod-web");
        assert_eq!(sites[1].host, "10.0.0.5");

        // 'db' has no HostName override → falls back to the alias; no identity.
        let db = sites.iter().find(|s| s.name == "db").unwrap();
        assert_eq!(db.host, "db.internal");
        assert_eq!(db.port, 22);
        assert!(matches!(db.auth, AuthMethod::Password));
    }

    #[test]
    fn accepts_equals_form() {
        let sites = parse_openssh_config("Host x\n  HostName=1.2.3.4\n  Port=22\n");
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].host, "1.2.3.4");
    }
}
