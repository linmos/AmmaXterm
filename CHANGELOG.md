# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Bastion / ProxyJump (multi-hop) and the remaining per-site overrides.
- PuTTY session import.

## [0.3.1] - 2026-06-20

### Added
- **Duplicate session** from the session-list right-click menu (Connect / Edit /
  Duplicate / Delete). The clone copies the stored password and key passphrase.

### Fixed
- Port-forwarding activity-bar badge no longer keeps a stale tunnel count after a
  session is closed or dropped — the session's tunnels are now torn down on any
  disconnect (not just an explicit one) and the count refreshes immediately.
- Widened the session dialog so the auto-tunnel "destination host" field is no
  longer clipped.

## [0.3.0] - 2026-06-19

### Added
- **Light theme** for the whole shell (not just the terminal), with a new
  "follow system" theme option that tracks the OS colour scheme live and is now
  the default.
- **Copy-on-select** for the terminal: selecting text copies it to the clipboard
  automatically (MobaXterm/PuTTY style). On by default; toggle in Settings.
- **SFTP multi-select** with Ctrl/Shift click or Shift+Up/Down (arrow keys move a
  cursor; Enter opens a folder, Esc clears) and **batch download**: select several
  remote files and download them all into one folder in a single action.
- App-wide suppression of the WebView's default right-click menu (the terminal
  keeps its own copy/paste handler).

### Changed
- Session rows are indented under their group header for clearer nesting.

## [0.2.0] - 2026-06-19

M2 (v1.0 feature set). "Session" = a saved connection in the UI.

### Added
- **Site management:** groups/folders with drag-to-reorder, live search over
  name/host/user/group/tags, `user@host:port` quick-connect, import from an
  OpenSSH `config`, and export/backup (no secrets written).
- **Terminal:** in-terminal search (Ctrl+Shift+F), theme / font / scrollback
  settings with global defaults, SSH keepalive with manual & auto reconnect,
  and per-session output logging to a file.
- **SFTP:** a transfer queue with progress, speed, pause/resume, cancel and
  auto-retry (resumable transfers); drag-and-drop upload; dual-pane (local ⇆
  remote); `chmod` with owner/permission display; filter & sort; follow the
  terminal's working directory; and large-directory virtualization.
- **Port forwarding:** Local `-L`, dynamic SOCKS5 `-D`, and Remote `-R` with a
  live tunnel-management panel; tunnels can be saved per site and auto-started.
- **Keys & secrets:** an Ed25519 / RSA key generator (copy public key, save the
  pair) and an AES-256-GCM + Argon2id master-key-encrypted local vault.
- Window size, position and maximized state are remembered across launches; a
  private-key path can be picked with a file browser.

### Security
- **PF-7:** local forward listeners bind `127.0.0.1`, never `0.0.0.0`; remote
  (`-R`) LAN exposure is off by default and warned about when enabled.

## [0.1.0] - 2026-06-19

### Added
- Project scaffolding: Tauri v2 + SvelteKit (Svelte 5) + TypeScript.
- Repository governance: LICENSE (MIT), README, CONTRIBUTING, CODE_OF_CONDUCT,
  SECURITY policy, issue/PR templates.
- Product requirements (`docs/PRD-AmmaXterm-v0.5.md`) and development plan
  (`docs/開發計畫-AmmaXterm.md`).
- M0 technical validation: SSH connection + PTY shell over `russh` (ring
  backend), terminal streaming (xterm.js ↔ Tauri channel) with window resize,
  host-key verification (`known_hosts`, trust-on-first-use + change rejection),
  and basic SFTP (list/upload/download) with a minimal file panel. See
  `docs/M0-技術驗證結論.md`.
- M1 (MVP): saved-site management (CRUD + local `sites.json`), OS-keychain
  credential storage and full auth (password / public-key + passphrase /
  keyboard-interactive), three-pane UI (site sidebar, multi-tab terminals,
  SFTP panel), connection lifecycle events, full SFTP file operations
  (mkdir / rename / delete / upload / download), terminal copy-paste,
  interactive host-key prompt, two-step destructive confirmations,
  i18n (繁體中文 / English), and CI + release (tauri-action) workflows.
