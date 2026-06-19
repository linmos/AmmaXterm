# CLAUDE.md

Guidance for Claude Code (and contributors) working in this repository.

## What this is

**AmmaXterm** — a lightweight, open-source, cross-platform SSH terminal (SSH + SFTP +
saved-session management + port forwarding). Aims to be lighter than MobaXterm but good
enough for daily use. MIT-licensed, distributed via GitHub Releases.

**Stack:** Tauri v2 (system WebView2) · Rust backend (`russh`) · SvelteKit + Svelte 5
(runes) + TypeScript · xterm.js.

**Docs:** PRD `docs/PRD-AmmaXterm-v0.5.md` · plan `docs/開發計畫-AmmaXterm.md` ·
M0 spike `docs/M0-技術驗證結論.md`. The plan enumerates every requirement
(SM/TM/FT/PF/AK/ST) and milestone (M0–M3) — consult it before starting a feature.

## Status

M0 + M1 (MVP) are **done and shipped**: `main`, tagged `v0.1.0`, CI green. Current work
is **M2 (v1.0)** — see §"M2 scope" below.

## Commands

```bash
# Frontend (run from repo root)
npm run dev            # Vite dev server (frontend only, no Tauri shell)
npm run check          # svelte-check + svelte-kit sync — TS/Svelte type check
npm run build          # production frontend build

# Full app (Tauri) — this is how you actually run/verify the app
npm run tauri dev      # build Rust + launch desktop app with hot-reload frontend
npm run tauri build    # produce platform installer

# Backend (run from src-tauri/)
cargo fmt              # 4-space indent (rustfmt default)
cargo clippy --all-targets
cargo test             # see GOTCHA below — bare cargo test can't launch the GUI-linked test
```

Always run `cargo fmt` + `cargo clippy` + `npm run check` before committing — CI enforces
all three (plus `cargo-deny`).

## Architecture

**Backend (`src-tauri/src/`)** — one module per concern:
- `ssh/mod.rs` — `russh` client: connect, auth (password / public-key+passphrase /
  keyboard-interactive), PTY shell, host-key verification, `run_session` actor loop.
- `session/mod.rs` — `SessionManager` (`Mutex<HashMap<id, Session>>`); spawns one tokio
  task + mpsc command channel **per session** (actor-per-session); one session's failure
  never affects others.
- `sftp/mod.rs` — `russh-sftp` ops (list/upload/download/mkdir/rename/delete, recursive).
- `store/mod.rs` — `Site` model + `sites.json` persistence (has `schema_version`, CRUD
  with rollback).
- `secrets/mod.rs` — `keyring` per-OS native backend; **only key-store references** live
  in `sites.json`, never plaintext/ciphertext secrets.
- `commands.rs` — all `#[tauri::command]` entry points (the entire IPC surface).
- `error.rs` — `AppError` (classified: DNS/timeout/auth/key-changed/channel), serializes
  to `{kind, message}` for the frontend.
- `lib.rs` — registers commands, manages state, plugins (opener, dialog).

**Frontend (`src/`)**:
- `lib/state.svelte.ts` — `AppState` runes class: sites, tabs, active tab, host-key
  prompt. Single source of truth (`export const app`).
- `lib/terminal/Terminal.svelte` — xterm.js wrapper (fit/webgl/search/unicode11,
  copy-paste).
- `lib/session/TerminalTabs.svelte`, `lib/sites/`, `lib/sftp/`, `lib/HostKeyDialog.svelte`
  — UI components. `routes/+page.svelte` is the three-pane layout.
- `lib/i18n.svelte.ts` — dependency-free i18n (zh-TW / en + toggle).

**Key data flows:**
- Terminal output: backend → **base64 over a Tauri `Channel<String>`** → decoded in
  `state.svelte.ts` → `xterm.write`. (Channel, not events — it's a stream.)
- Connection lifecycle: Tauri **events** `ssh://closed`, `ssh://host-key-prompt`.
- Host-key prompt: backend emits event + awaits a oneshot (120s timeout); frontend shows
  dialog → `host_key_decision` command resolves it. TOFU otherwise; app-private
  `known_hosts` (OpenSSH format) under the app config dir.

## Load-bearing decisions (not obvious from code)

- `russh` 0.61 uses the **`ring`** crypto backend, **not** aws-lc-rs (Windows build).
- `keyring` 3.x with per-OS native features (`windows-native` / `apple-native` /
  `sync-secret-service`) — see the `[target.'cfg(...)']` blocks in `Cargo.toml`.
- Internal type is named **`Site`** (a saved connection); the **UI** calls it a
  **"Session"** (MobaXterm parity). Keep that distinction — don't rename the Rust type.
- i18n strings are externalized from day one; add new strings to **both** en and zh-TW.

## Conventions

- **Indentation differs by file type** (this trips up edits):
  - Rust (`.rs`): **4 spaces** (rustfmt).
  - Svelte / TS (`.svelte`, `.ts`): **tabs**.
- New Tauri command → add `#[tauri::command]` in `commands.rs`, register in `lib.rs`,
  add a typed wrapper call in `state.svelte.ts`, and grant any needed capability in
  `capabilities/default.json`.
- Secrets never touch `sites.json` — always go through `secrets/mod.rs` (keyring).
- Port forwarding (M2): listeners **must** bind `127.0.0.1`, never `0.0.0.0`; Remote (`-R`)
  forwarding defaults **off** with an exposure warning (PRD PF-7, P0 security default).

## Gotchas

- **`cargo test` can't launch the integration test on Windows** since the dialog plugin
  was added: the test exe links the Tauri GUI stack → `STATUS_ENTRYPOINT_NOT_FOUND`
  (0xc0000139) at exe load. Not a logic failure (app runs fine). SSH/SFTP logic was
  Docker-integration-verified before the plugin landed. Planned fix: move integration
  tests to a non-Tauri crate. Unit tests in pure modules still run.
- **Computer-use can't drive the dev `.exe`** — the resolver only matches Start-menu-
  installed apps. Verify the GUI manually via `npm run tauri dev`.
- **Pushing `.github/workflows/` over HTTPS fails** — the gh OAuth token lacks the
  `workflow` scope. The remote is configured for **SSH**; push works there.
- After adding Rust deps, the first `npm run tauri dev` may force a Vite re-optimization
  and exit once; re-run.

## M2 scope (current milestone, P1)

Per `docs/開發計畫-AmmaXterm.md` §M2: session grouping/folders + search/filter, quick
connect (`user@host:port`), per-site overrides, import OpenSSH config / PuTTY sessions,
export/backup; auto-reconnect + keepalive, ProxyJump, terminal output search, theming/
fonts, session logging; SFTP transfer queue (progress/pause/resume/retry), drag-drop,
follow-cd, resumable transfers, chmod, large-dir virtualization, dual-pane; **port
forwarding** Local `-L` / dynamic SOCKS5 `-D` / Remote `-R` with the PF-7 security
default; key generator (Ed25519/RSA) + master-key-encrypted local vault.

A `tunnel/` module under `src-tauri/src/` is the planned home for port-forwarding logic.
