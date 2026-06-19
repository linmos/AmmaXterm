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

// --- PuTTY session import ---
//
// PuTTY stores each saved session under the registry path
// `HKCU\Software\SimonTatham\PuTTY\Sessions\<name>`, where `<name>` is
// percent-encoded. The same data is recoverable from a `regedit` `.reg` export,
// which is what `parse_putty_reg` reads (pure + testable); `read_putty_registry`
// reads the live registry on Windows.

/// Decode PuTTY's `%XX` percent-encoding used in registry session key names
/// (e.g. spaces stored as `%20`). Unknown/short sequences are kept verbatim.
fn unescape_putty_name(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Build a PuTTY-derived candidate, or `None` when it isn't a connectable SSH
/// session (no host, or a non-SSH protocol like telnet/serial).
fn putty_site(
    name: String,
    host: String,
    port: u16,
    username: String,
    keyfile: String,
    protocol: &str,
) -> Option<ImportedSite> {
    if host.trim().is_empty() {
        return None; // "Default Settings" and the like carry no host
    }
    if !protocol.is_empty() && !protocol.eq_ignore_ascii_case("ssh") {
        return None; // telnet / serial / rlogin sessions are not importable here
    }
    let auth = if keyfile.is_empty() {
        AuthMethod::Password
    } else {
        AuthMethod::PublicKey { key_path: keyfile }
    };
    Some(ImportedSite {
        name,
        host,
        port: if port == 0 { 22 } else { port },
        username,
        auth,
        group: Some("putty".to_string()),
        tags: vec!["imported".to_string()],
    })
}

#[derive(Default)]
struct PuttyBlock {
    name: String,
    hostname: String,
    port: u16,
    username: String,
    protocol: String,
    keyfile: String,
}

impl PuttyBlock {
    fn finish(self, out: &mut Vec<ImportedSite>) {
        if let Some(site) = putty_site(
            self.name,
            self.hostname,
            self.port,
            self.username,
            self.keyfile,
            &self.protocol,
        ) {
            out.push(site);
        }
    }
}

/// Parse a `.reg` export (`"Key"="val"` / `"Key"=dword:hhhhhhhh`) on the current
/// block, returning the lowercased key and a raw value token.
fn parse_reg_line(line: &str) -> Option<(String, &str)> {
    let line = line.trim();
    if !line.starts_with('"') {
        return None;
    }
    let rest = &line[1..];
    let end = rest.find('"')?;
    let key = rest[..end].to_ascii_lowercase();
    let after = rest[end + 1..].trim_start();
    let value = after.strip_prefix('=')?.trim();
    Some((key, value))
}

/// Decode a `.reg` string value: strips the surrounding quotes and unescapes
/// `\\` and `\"`. Non-string tokens yield `None`.
fn reg_string(token: &str) -> Option<String> {
    let inner = token.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some(other) => out.push(other),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

/// Decode a `.reg` `dword:hhhhhhhh` value.
fn reg_dword(token: &str) -> Option<u32> {
    let hex = token.strip_prefix("dword:")?;
    u32::from_str_radix(hex.trim(), 16).ok()
}

/// Parse a Windows `.reg` export of PuTTY sessions into candidates (SM-7).
pub fn parse_putty_reg(text: &str) -> Vec<ImportedSite> {
    let mut out = Vec::new();
    let mut cur: Option<PuttyBlock> = None;

    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            if let Some(block) = cur.take() {
                block.finish(&mut out);
            }
            let inner = line.trim_start_matches('[').trim_end_matches(']');
            // Only `…\Sessions\<name>` keys are sessions.
            cur = inner.rfind("\\Sessions\\").and_then(|idx| {
                let name = unescape_putty_name(&inner[idx + "\\Sessions\\".len()..]);
                (!name.is_empty()).then(|| PuttyBlock {
                    name,
                    ..Default::default()
                })
            });
        } else if let Some(block) = cur.as_mut() {
            let Some((key, value)) = parse_reg_line(line) else {
                continue;
            };
            match key.as_str() {
                "hostname" => block.hostname = reg_string(value).unwrap_or_default(),
                "portnumber" => block.port = reg_dword(value).unwrap_or(0) as u16,
                "username" => block.username = reg_string(value).unwrap_or_default(),
                "protocol" => block.protocol = reg_string(value).unwrap_or_default(),
                "publickeyfile" => block.keyfile = reg_string(value).unwrap_or_default(),
                _ => {}
            }
        }
    }
    if let Some(block) = cur.take() {
        block.finish(&mut out);
    }
    out.sort_by_key(|s| s.name.to_lowercase());
    out
}

/// Read PuTTY sessions directly from the Windows registry (SM-7). Returns an
/// empty list when PuTTY isn't installed / has no saved sessions.
#[cfg(windows)]
pub fn read_putty_registry() -> AppResult<Vec<ImportedSite>> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let sessions = match hkcu.open_subkey("Software\\SimonTatham\\PuTTY\\Sessions") {
        Ok(k) => k,
        Err(_) => return Ok(Vec::new()),
    };

    let mut out = Vec::new();
    for raw_name in sessions.enum_keys().flatten() {
        let Ok(key) = sessions.open_subkey(&raw_name) else {
            continue;
        };
        let protocol: String = key.get_value("Protocol").unwrap_or_default();
        let hostname: String = key.get_value("HostName").unwrap_or_default();
        let port: u32 = key.get_value("PortNumber").unwrap_or(22);
        let username: String = key.get_value("UserName").unwrap_or_default();
        let keyfile: String = key.get_value("PublicKeyFile").unwrap_or_default();
        if let Some(site) = putty_site(
            unescape_putty_name(&raw_name),
            hostname,
            port as u16,
            username,
            keyfile,
            &protocol,
        ) {
            out.push(site);
        }
    }
    out.sort_by_key(|s| s.name.to_lowercase());
    Ok(out)
}

/// Non-Windows builds have no PuTTY registry; reading one is unsupported.
#[cfg(not(windows))]
pub fn read_putty_registry() -> AppResult<Vec<ImportedSite>> {
    Err(AppError::Other(
        "PuTTY registry import is only available on Windows".into(),
    ))
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

    #[test]
    fn parses_putty_reg_export() {
        let reg = "\
Windows Registry Editor Version 5.00

[HKEY_CURRENT_USER\\Software\\SimonTatham\\PuTTY\\Sessions\\Prod%20Web]
\"HostName\"=\"10.0.0.5\"
\"PortNumber\"=dword:00000016
\"UserName\"=\"deploy\"
\"Protocol\"=\"ssh\"
\"PublicKeyFile\"=\"C:\\\\keys\\\\id.ppk\"

[HKEY_CURRENT_USER\\Software\\SimonTatham\\PuTTY\\Sessions\\Default%20Settings]
\"HostName\"=\"\"
\"Protocol\"=\"ssh\"

[HKEY_CURRENT_USER\\Software\\SimonTatham\\PuTTY\\Sessions\\serial-box]
\"HostName\"=\"COM3\"
\"Protocol\"=\"serial\"

[HKEY_CURRENT_USER\\Software\\SimonTatham\\PuTTY\\Sessions\\db]
\"HostName\"=\"db.internal\"
\"Protocol\"=\"ssh\"
";
        let sites = parse_putty_reg(reg);
        // "Prod Web" + "db"; "Default Settings" (no host) and serial are skipped.
        assert_eq!(sites.len(), 2);

        // Sorted by name: "db" before "Prod Web".
        let db = &sites[0];
        assert_eq!(db.name, "db");
        assert_eq!(db.host, "db.internal");
        assert_eq!(db.port, 22);
        assert!(matches!(db.auth, AuthMethod::Password));

        let web = &sites[1];
        assert_eq!(web.name, "Prod Web"); // %20 decoded
        assert_eq!(web.host, "10.0.0.5");
        assert_eq!(web.port, 22); // 0x16
        assert_eq!(web.username, "deploy");
        match &web.auth {
            AuthMethod::PublicKey { key_path } => assert_eq!(key_path, "C:\\keys\\id.ppk"),
            _ => panic!("expected public-key auth from PublicKeyFile"),
        }
    }
}
