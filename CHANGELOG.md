# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Bastion / ProxyJump (multi-hop) and the remaining per-site overrides.
- PuTTY session import.

## [0.4.7] - 2026-06-29

### Fixed
- **Full-screen apps (vi/vim/tmux/less) still froze the packaged terminal.**
  0.4.5's WebGL-renderer removal fixed the freeze under `tauri dev` but not in the
  shipped build. The real cause was the production minifier: esbuild dropped the
  `var` declaration of an xterm `const enum` holder, emitting `void 0 || (i = {})`
  with `i` undeclared. ES modules run in strict mode, so that threw
  `ReferenceError: i is not defined` the first time xterm answered a DECRQM query
  — which vi/vim/tmux/less send on startup — crashing xterm's write loop and
  freezing the terminal. Switched the Vite minifier from esbuild to terser.

### Changed
- The GitHub Release body and the updater's `latest.json` notes are now
  auto-populated from this changelog (they were previously blank).

## [0.4.6] - 2026-06-28

### Added
- **SFTP folder download.** Whole directories can now be downloaded (recursively),
  mirroring folder upload — via the context menu, the selection bar, or the
  single-entry download. Previously only individual files could be fetched.
- **Drag-to-select in the SFTP listing.** Press and drag over the file list to
  box-select rows like a file manager. The drag may start on a row or the empty
  background; ordinary click / ctrl / shift / double-click are unchanged, and
  dragging near an edge auto-scrolls.

### Changed
- **SFTP panel follows the terminal's connection.** When a session drops, the
  file panel now goes offline (overlay + disabled controls) instead of showing a
  stale listing whose operations silently fail; a reconnect restores it.
- Renamed the selection bar's **"Clear"** action to **"Cancel"**.

## [0.4.5] - 2026-06-28

### Added
- **SFTP auto-refresh** — the remote listing now reloads automatically when an
  upload finishes, so a just-uploaded file appears without a manual refresh.
  Uploads run asynchronously through the transfer queue, which previously left the
  pane showing stale contents until reloaded by hand.

### Fixed
- **vi/vim/tmux froze the terminal.** Full-screen apps that switch to the
  alternate screen buffer left the screen frozen (nano, which doesn't use the
  alternate buffer, was unaffected). The cause was the xterm WebGL renderer
  failing to repaint after the alt-buffer switch inside Windows WebView2. Dropped
  the WebGL addon and fell back to xterm's default DOM renderer.

## [0.4.4] - 2026-06-26

### Fixed
- **Terminal size** is now synced to the window. Full-screen apps (vi, nano,
  tmux, less) rendered into an 80×24 corner instead of filling the terminal: the
  PTY stayed at the hardcoded default because the initial fit happened before the
  resize listener was attached and while the session was still connecting, so the
  real dimensions never reached the backend. The terminal now reports its size as
  soon as it fits and again once the session is established.

## [0.4.3] - 2026-06-26

### Added
- **SFTP batch delete** — "Delete selected" in the selection bar removes every
  selected file and folder. Both it and the right-click delete now confirm via a
  popup dialog; right-clicking an item inside the selection acts on the whole
  selection.

### Changed
- **SFTP selection** now follows the desktop file-manager convention: single
  click selects (folders included), double-click opens a folder or downloads a
  file. Building a multi-selection no longer enters a folder by accident.

### Fixed
- **Symlinked directories** in the SFTP pane are now navigable — a symlink that
  points to a directory is detected by following the link, so it can be opened
  and is marked with a 🔗 indicator.
- A folder double-click could land on the wrong row because showing the selection
  bar reflowed the list between the two clicks; the bar now floats over the list
  without shifting rows.

## [0.4.1] - 2026-06-22

### Fixed
- **Reconnect** on a dropped/closed session now works. The "Reconnect" badge was
  visually on top but unclickable — xterm's canvas layers carry a positive
  `z-index` and were swallowing the click; the badge now sits above them. And once
  clicked, reconnecting reused the tab's output channel whose message index was
  stuck at the previous session's high-water mark, so the fresh session's output
  was buffered forever and the screen looked frozen — reconnect now hands the tab a
  new channel and clears the stale screen.

## [0.3.2] - 2026-06-20

### Added
- **About dialog** (info button in the activity bar) showing the app version,
  Tauri version, license, and a link to the GitHub repository.

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
