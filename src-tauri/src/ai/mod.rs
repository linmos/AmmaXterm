//! AI assistant backend (multi-provider, BYO key).
//!
//! Streams chat completions from Claude (Anthropic Messages API), any
//! OpenAI-compatible endpoint, or a local Ollama server (through its
//! OpenAI-compatible `/v1` layer) to the frontend over a Tauri
//! `Channel<String>` — reusing the same streaming mechanism as terminal output,
//! except AI deltas are already UTF-8 strings so they are sent without base64.
//!
//! Privacy: the API key is resolved at request time from the same keychain /
//! encrypted vault used for SSH secrets (see `commands::ai_stream`); nothing is
//! sent anywhere unless the user explicitly acts.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::ipc::Channel;
use tokio::sync::Notify;

use crate::error::{AppError, AppResult};

/// A single chat turn. `role` is `"system"` | `"user"` | `"assistant"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Provider wire format. Ollama is served through the OpenAI-compatible path.
#[derive(Clone, Copy, PartialEq)]
enum Wire {
    Anthropic,
    OpenAi,
}

/// Resolved configuration for a single streaming request.
pub struct ProviderConfig {
    /// `"claude"` | `"openai"` | `"gemini"` | `"ollama"`.
    pub provider: String,
    pub model: String,
    /// Empty string = use the provider's default endpoint.
    pub base_url: String,
    /// `None` for Ollama (it needs no real key).
    pub api_key: Option<String>,
    pub max_tokens: u32,
}

/// Per-request cancellation handle.
struct Inflight {
    notify: Arc<Notify>,
    cancelled: Arc<AtomicBool>,
}

/// Tauri-managed state: a shared HTTP client and the in-flight request registry.
pub struct AiManager {
    client: reqwest::Client,
    inflight: Mutex<HashMap<String, Inflight>>,
}

impl Default for AiManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AiManager {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(20))
            .user_agent(concat!("AmmaXterm/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            inflight: Mutex::new(HashMap::new()),
        }
    }

    /// Cancel an in-flight stream by its request id (no-op if already finished).
    pub fn cancel(&self, request_id: &str) {
        if let Some(h) = self.inflight.lock().unwrap().get(request_id) {
            h.cancelled.store(true, Ordering::SeqCst);
            h.notify.notify_waiters();
        }
    }

    /// Stream a chat completion, pushing each text delta to `on_chunk`.
    /// Resolves once the stream completes, is cancelled, or errors.
    pub async fn stream(
        &self,
        request_id: String,
        cfg: ProviderConfig,
        messages: Vec<ChatMessage>,
        system: Option<String>,
        on_chunk: Channel<String>,
    ) -> AppResult<()> {
        let notify = Arc::new(Notify::new());
        let cancelled = Arc::new(AtomicBool::new(false));
        self.inflight.lock().unwrap().insert(
            request_id.clone(),
            Inflight {
                notify: notify.clone(),
                cancelled: cancelled.clone(),
            },
        );
        let res = self
            .run(cfg, messages, system, on_chunk, notify, cancelled)
            .await;
        self.inflight.lock().unwrap().remove(&request_id);
        res
    }

    async fn run(
        &self,
        cfg: ProviderConfig,
        messages: Vec<ChatMessage>,
        system: Option<String>,
        on_chunk: Channel<String>,
        notify: Arc<Notify>,
        cancelled: Arc<AtomicBool>,
    ) -> AppResult<()> {
        let wire = if cfg.provider == "claude" {
            Wire::Anthropic
        } else {
            Wire::OpenAi
        };
        let (url, body) = build_request(&cfg, wire, &messages, &system);

        let mut req = self.client.post(&url).json(&body);
        req = match wire {
            Wire::Anthropic => {
                let key = cfg
                    .api_key
                    .as_deref()
                    .ok_or_else(|| AppError::Auth("AI API key not set".into()))?;
                req.header("x-api-key", key)
                    .header("anthropic-version", "2023-06-01")
            }
            Wire::OpenAi => {
                // Ollama ignores the key but its /v1 layer still wants a Bearer.
                let key = cfg.api_key.clone().unwrap_or_else(|| "ollama".to_string());
                req.header("authorization", format!("Bearer {key}"))
            }
        };

        let resp = req
            .send()
            .await
            .map_err(|e| AppError::Other(format!("AI request failed: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Other(format!(
                "AI provider error ({status}): {}",
                truncate(text.trim(), 500)
            )));
        }

        let idle = Duration::from_secs(120);
        let mut stream = resp.bytes_stream();
        let mut buf: Vec<u8> = Vec::new();

        loop {
            if cancelled.load(Ordering::SeqCst) {
                break;
            }
            let next = tokio::select! {
                biased;
                _ = notify.notified() => break,
                r = tokio::time::timeout(idle, stream.next()) => r,
            };
            let chunk = match next {
                Err(_) => return Err(AppError::Other("AI stream idle timeout".into())),
                Ok(None) => break,
                Ok(Some(Ok(bytes))) => bytes,
                Ok(Some(Err(e))) => return Err(AppError::Other(format!("AI stream error: {e}"))),
            };
            buf.extend_from_slice(chunk.as_ref());

            // SSE frames are line-oriented; both Anthropic and OpenAI put the JSON
            // payload on a single `data:` line, so we can process line by line and
            // ignore `event:` lines (the JSON carries its own `type`).
            while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                let raw: Vec<u8> = buf.drain(..=pos).collect();
                let line = String::from_utf8_lossy(&raw);
                let line = line.trim_end_matches(['\r', '\n']);
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                match parse_delta(wire, data.trim()) {
                    Delta::Text(t) => {
                        if !t.is_empty() && on_chunk.send(t).is_err() {
                            return Ok(()); // frontend went away
                        }
                    }
                    Delta::Done => return Ok(()),
                    Delta::Error(msg) => {
                        return Err(AppError::Other(format!("AI provider error: {msg}")))
                    }
                    Delta::Ignore => {}
                }
            }
        }
        Ok(())
    }

    /// Fetch the available model ids for a provider (AI-5, P3).
    pub async fn list_models(&self, cfg: ProviderConfig) -> AppResult<Vec<String>> {
        let provider = cfg.provider.as_str();
        let (url, kind) = if provider == "ollama" {
            // Ollama lists models at the host root (/api/tags), not under /v1.
            let host = if cfg.base_url.is_empty() {
                "http://localhost:11434".to_string()
            } else {
                let b = cfg.base_url.trim_end_matches('/');
                b.strip_suffix("/v1").unwrap_or(b).to_string()
            };
            (format!("{host}/api/tags"), ModelList::Ollama)
        } else if provider == "claude" {
            let base = if cfg.base_url.is_empty() {
                "https://api.anthropic.com"
            } else {
                cfg.base_url.trim_end_matches('/')
            };
            (format!("{base}/v1/models"), ModelList::OpenAiData)
        } else {
            // OpenAI + Gemini (both via the OpenAI-compatible layer).
            (
                format!("{}/models", openai_base(&cfg)),
                ModelList::OpenAiData,
            )
        };

        let mut req = self.client.get(&url);
        if provider == "claude" {
            let key = cfg
                .api_key
                .as_deref()
                .ok_or_else(|| AppError::Auth("AI API key not set".into()))?;
            req = req
                .header("x-api-key", key)
                .header("anthropic-version", "2023-06-01");
        } else if provider != "ollama" {
            let key = cfg
                .api_key
                .as_deref()
                .ok_or_else(|| AppError::Auth("AI API key not set".into()))?;
            req = req.header("authorization", format!("Bearer {key}"));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| AppError::Other(format!("AI request failed: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Other(format!(
                "AI provider error ({status}): {}",
                truncate(text.trim(), 300)
            )));
        }
        let v: Value = resp
            .json()
            .await
            .map_err(|e| AppError::Other(format!("AI response parse error: {e}")))?;
        let mut models = parse_models(kind, &v);
        models.sort();
        models.dedup();
        Ok(models)
    }
}

/// Base URL for an OpenAI-compatible provider (chat + model listing).
/// Gemini is served through Google's OpenAI-compatible layer, so it reuses the
/// whole OpenAI request/SSE path — only the default endpoint differs.
fn openai_base(cfg: &ProviderConfig) -> String {
    if !cfg.base_url.is_empty() {
        return cfg.base_url.trim_end_matches('/').to_string();
    }
    match cfg.provider.as_str() {
        "ollama" => "http://localhost:11434/v1".to_string(),
        "gemini" => "https://generativelanguage.googleapis.com/v1beta/openai".to_string(),
        _ => "https://api.openai.com/v1".to_string(),
    }
}

/// Build the endpoint URL and JSON body for a provider/request.
fn build_request(
    cfg: &ProviderConfig,
    wire: Wire,
    messages: &[ChatMessage],
    system: &Option<String>,
) -> (String, Value) {
    match wire {
        Wire::Anthropic => {
            let base = if cfg.base_url.is_empty() {
                "https://api.anthropic.com"
            } else {
                cfg.base_url.trim_end_matches('/')
            };
            let url = format!("{base}/v1/messages");
            // Anthropic carries the system prompt at the top level; the messages
            // array must not contain a system role.
            let msgs: Vec<Value> = messages
                .iter()
                .filter(|m| m.role != "system")
                .map(|m| json!({ "role": m.role, "content": m.content }))
                .collect();
            let mut body = json!({
                "model": cfg.model,
                "max_tokens": cfg.max_tokens,
                "messages": msgs,
                "stream": true,
            });
            if let Some(sys) = system {
                if !sys.is_empty() {
                    body["system"] = json!(sys);
                }
            }
            (url, body)
        }
        Wire::OpenAi => {
            let base = openai_base(cfg);
            let url = format!("{base}/chat/completions");
            let mut msgs: Vec<Value> = Vec::new();
            if let Some(sys) = system {
                if !sys.is_empty() {
                    msgs.push(json!({ "role": "system", "content": sys }));
                }
            }
            for m in messages {
                msgs.push(json!({ "role": m.role, "content": m.content }));
            }
            let body = json!({
                "model": cfg.model,
                "messages": msgs,
                "max_tokens": cfg.max_tokens,
                "stream": true,
            });
            (url, body)
        }
    }
}

/// Outcome of interpreting one SSE `data:` payload.
enum Delta {
    Text(String),
    Done,
    Error(String),
    Ignore,
}

/// Interpret a single SSE `data:` payload for the given wire format.
/// Unknown/forward-compatible payloads map to `Ignore`.
fn parse_delta(wire: Wire, data: &str) -> Delta {
    match wire {
        Wire::OpenAi => {
            if data == "[DONE]" {
                return Delta::Done;
            }
            let v: Value = match serde_json::from_str(data) {
                Ok(v) => v,
                Err(_) => return Delta::Ignore,
            };
            if let Some(err) = v.get("error") {
                let msg = err
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error");
                return Delta::Error(msg.to_string());
            }
            match v
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("delta"))
                .and_then(|d| d.get("content"))
                .and_then(Value::as_str)
            {
                Some(t) => Delta::Text(t.to_string()),
                None => Delta::Ignore,
            }
        }
        Wire::Anthropic => {
            let v: Value = match serde_json::from_str(data) {
                Ok(v) => v,
                Err(_) => return Delta::Ignore,
            };
            match v.get("type").and_then(Value::as_str) {
                Some("content_block_delta") => match v
                    .get("delta")
                    .and_then(|d| d.get("text"))
                    .and_then(Value::as_str)
                {
                    Some(t) => Delta::Text(t.to_string()),
                    None => Delta::Ignore,
                },
                Some("message_stop") => Delta::Done,
                Some("error") => {
                    let msg = v
                        .get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(Value::as_str)
                        .unwrap_or("unknown error");
                    Delta::Error(msg.to_string())
                }
                _ => Delta::Ignore,
            }
        }
    }
}

/// Shape of a model-listing response.
#[derive(Clone, Copy)]
enum ModelList {
    /// `{ "data": [ { "id": "..." }, ... ] }` (Anthropic + OpenAI).
    OpenAiData,
    /// `{ "models": [ { "name": "..." }, ... ] }` (Ollama /api/tags).
    Ollama,
}

/// Extract model ids from a listing response.
fn parse_models(kind: ModelList, v: &Value) -> Vec<String> {
    let (array, field) = match kind {
        ModelList::OpenAiData => (v.get("data"), "id"),
        ModelList::Ollama => (v.get("models"), "name"),
    };
    array
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|m| m.get(field).and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Best-effort scrub of obvious secrets before text leaves the process (AI-N4).
/// Not a guarantee — the UI must say so.
pub fn redact(text: &str) -> String {
    const KEYS: [&str; 7] = [
        "password", "passwd", "secret", "token", "api_key", "apikey", "api-key",
    ];
    let mut out: Vec<String> = Vec::new();
    let mut in_key_block = false;
    for line in text.lines() {
        if line.contains("BEGIN") && line.contains("PRIVATE KEY") {
            in_key_block = true;
            out.push("[REDACTED PRIVATE KEY]".to_string());
            continue;
        }
        if in_key_block {
            if line.contains("END") && line.contains("PRIVATE KEY") {
                in_key_block = false;
            }
            continue;
        }
        out.push(redact_line(line, &KEYS));
    }
    out.join("\n")
}

/// Known high-signal secret token prefixes (provider keys / tokens).
const TOKEN_PREFIXES: [&str; 11] = [
    "sk-",
    "ghp_",
    "gho_",
    "ghs_",
    "ghu_",
    "github_pat_",
    "xoxb-",
    "xoxp-",
    "AKIA",
    "ASIA",
    "AIza",
];

fn redact_line(line: &str, keys: &[&str]) -> String {
    let lower = line.to_lowercase();
    if keys.iter().any(|k| lower.contains(k)) {
        if let Some(idx) = line.find(['=', ':']) {
            return format!("{} [REDACTED]", &line[..=idx]);
        }
    }
    if let Some(idx) = line.find("Bearer ") {
        return format!("{}[REDACTED]", &line[..idx + "Bearer ".len()]);
    }
    // Scrub bare provider tokens (e.g. `sk-…`, `ghp_…`, `AKIA…`). Only rewrites
    // lines that contain a known prefix; spacing is normalized on those only.
    if TOKEN_PREFIXES.iter().any(|p| line.contains(p)) {
        return line
            .split_whitespace()
            .map(|tok| {
                if tok.len() >= 12 && TOKEN_PREFIXES.iter().any(|p| tok.starts_with(p)) {
                    "[REDACTED]"
                } else {
                    tok
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
    }
    line.to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut t: String = s.chars().take(max).collect();
    t.push('…');
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_done_and_delta() {
        assert!(matches!(parse_delta(Wire::OpenAi, "[DONE]"), Delta::Done));
        let d = parse_delta(
            Wire::OpenAi,
            r#"{"choices":[{"delta":{"content":"hi"},"finish_reason":null}]}"#,
        );
        assert!(matches!(d, Delta::Text(t) if t == "hi"));
        // Role-only first chunk has no content → ignored.
        assert!(matches!(
            parse_delta(
                Wire::OpenAi,
                r#"{"choices":[{"delta":{"role":"assistant"}}]}"#
            ),
            Delta::Ignore
        ));
    }

    #[test]
    fn anthropic_delta_stop_and_unknown() {
        let d = parse_delta(
            Wire::Anthropic,
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"yo"}}"#,
        );
        assert!(matches!(d, Delta::Text(t) if t == "yo"));
        assert!(matches!(
            parse_delta(Wire::Anthropic, r#"{"type":"message_stop"}"#),
            Delta::Done
        ));
        // ping / unknown events are forward-compatibly ignored.
        assert!(matches!(
            parse_delta(Wire::Anthropic, r#"{"type":"ping"}"#),
            Delta::Ignore
        ));
    }

    #[test]
    fn provider_error_surfaces() {
        assert!(matches!(
            parse_delta(Wire::OpenAi, r#"{"error":{"message":"bad key"}}"#),
            Delta::Error(m) if m == "bad key"
        ));
        assert!(matches!(
            parse_delta(
                Wire::Anthropic,
                r#"{"type":"error","error":{"type":"overloaded_error","message":"overloaded"}}"#
            ),
            Delta::Error(m) if m == "overloaded"
        ));
    }

    #[test]
    fn redact_scrubs_secrets() {
        let input =
            "normal line\nexport PASSWORD=hunter2\nAuthorization: Bearer sk-abc123\nplain text";
        let out = redact(input);
        assert!(out.contains("normal line"));
        assert!(out.contains("[REDACTED]"));
        assert!(!out.contains("hunter2"));
        assert!(!out.contains("sk-abc123"));
        assert!(out.contains("plain text"));
    }

    #[test]
    fn redact_drops_private_key_block() {
        let input = "before\n-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\nBBBB\n-----END OPENSSH PRIVATE KEY-----\nafter";
        let out = redact(input);
        assert!(out.contains("before"));
        assert!(out.contains("after"));
        assert!(out.contains("[REDACTED PRIVATE KEY]"));
        assert!(!out.contains("AAAA"));
    }

    #[test]
    fn anthropic_request_lifts_system_out_of_messages() {
        let cfg = ProviderConfig {
            provider: "claude".into(),
            model: "claude-sonnet-4-6".into(),
            base_url: String::new(),
            api_key: Some("k".into()),
            max_tokens: 256,
        };
        let msgs = vec![ChatMessage {
            role: "user".into(),
            content: "hi".into(),
        }];
        let (url, body) = build_request(&cfg, Wire::Anthropic, &msgs, &Some("be terse".into()));
        assert_eq!(url, "https://api.anthropic.com/v1/messages");
        assert_eq!(body["system"], json!("be terse"));
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
        assert_eq!(body["stream"], json!(true));
    }

    #[test]
    fn ollama_defaults_to_localhost_v1() {
        let cfg = ProviderConfig {
            provider: "ollama".into(),
            model: "llama3.1".into(),
            base_url: String::new(),
            api_key: None,
            max_tokens: 256,
        };
        let (url, body) = build_request(&cfg, Wire::OpenAi, &[], &None);
        assert_eq!(url, "http://localhost:11434/v1/chat/completions");
        assert_eq!(body["model"], json!("llama3.1"));
    }

    #[test]
    fn gemini_uses_google_openai_compat_base() {
        let cfg = ProviderConfig {
            provider: "gemini".into(),
            model: "gemini-2.5-flash".into(),
            base_url: String::new(),
            api_key: Some("k".into()),
            max_tokens: 256,
        };
        let msgs = vec![ChatMessage {
            role: "user".into(),
            content: "hi".into(),
        }];
        let (url, body) = build_request(&cfg, Wire::OpenAi, &msgs, &None);
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
        );
        assert_eq!(body["model"], json!("gemini-2.5-flash"));
        // A custom base_url overrides the default.
        let custom = ProviderConfig {
            base_url: "https://proxy.example/v1/".into(),
            ..cfg
        };
        let (url2, _) = build_request(&custom, Wire::OpenAi, &msgs, &None);
        assert_eq!(url2, "https://proxy.example/v1/chat/completions");
    }

    #[test]
    fn parse_models_openai_and_ollama() {
        let openai = serde_json::json!({
            "data": [{ "id": "gpt-4o" }, { "id": "gpt-4o-mini" }]
        });
        assert_eq!(
            parse_models(ModelList::OpenAiData, &openai),
            vec!["gpt-4o".to_string(), "gpt-4o-mini".to_string()]
        );
        let ollama = serde_json::json!({
            "models": [{ "name": "llama3.1:latest" }, { "name": "qwen2.5:7b" }]
        });
        assert_eq!(
            parse_models(ModelList::Ollama, &ollama),
            vec!["llama3.1:latest".to_string(), "qwen2.5:7b".to_string()]
        );
        // Missing/empty arrays degrade gracefully.
        assert!(parse_models(ModelList::OpenAiData, &serde_json::json!({})).is_empty());
    }

    #[test]
    fn redact_scrubs_bare_tokens() {
        let out = redact("found AKIAIOSFODNN7EXAMPLE and ghp_0123456789abcdef in logs");
        assert!(!out.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!out.contains("ghp_0123456789abcdef"));
        assert!(out.contains("[REDACTED]"));
        assert!(out.contains("found"));
        assert!(out.contains("in logs"));
    }
}
