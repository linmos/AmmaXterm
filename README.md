# AmmaXterm

> A lightweight, open-source, cross-platform SSH terminal — SSH, SFTP, site
> management, and port forwarding done simply, fast, and securely.
>
> 輕量、開源、跨平台的 SSH 終端工具 —— 把 SSH 終端、SFTP 傳檔、站台管理與
> 連接埠轉發做到簡單、快速、安全。

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Status](https://img.shields.io/badge/status-active%20development-brightgreen)

---

## What is AmmaXterm?

AmmaXterm focuses on the few things people actually use a remote tool for, and
does them well:

- **SSH terminal** — SSH 2.0 with password / public-key / keyboard-interactive
  auth, multi-tab sessions, true color, UTF-8, mandatory host-key verification,
  in-terminal search, themes & fonts, keepalive + auto-reconnect, and session
  logging.
- **SFTP file transfer** — browse with filter/sort, a transfer queue (progress,
  speed, pause/resume, auto-retry), drag-and-drop upload, dual-pane (local ⇆
  remote), `chmod`, follow-terminal-cd, and large-directory virtualization.
- **Site management** — save, group (drag to reorder), search, `user@host:port`
  quick-connect, import OpenSSH `config`, and export/backup.
- **Port forwarding** — Local (`-L`), Dynamic SOCKS5 (`-D`), and Remote (`-R`),
  with secure defaults (local listeners bind `127.0.0.1`; `-R` exposure off by
  default) and a live tunnel-management panel.
- **Keys & secrets** — credentials in the OS keychain; an Ed25519 / RSA key
  generator; and an AES-256-GCM + Argon2id encrypted vault for systems without a
  keychain.

Built to be **fast to start, intuitive, secure by default, and auditable** —
because the source is open.

## Why

MobaXterm is powerful but heavy, Windows-only, and gates sessions behind a paid
tier. AmmaXterm aims to be the lightweight, cross-platform, MIT-licensed
alternative for the core workflows.

## Tech stack

- **Desktop:** [Tauri v2](https://tauri.app) (system webview — small footprint)
- **Backend:** Rust ([`russh`](https://crates.io/crates/russh) for SSH/SFTP,
  [`keyring`](https://crates.io/crates/keyring) for credentials)
- **Frontend:** [SvelteKit](https://svelte.dev) (Svelte 5) + TypeScript
- **Terminal:** [xterm.js](https://xtermjs.org) + WebGL addon

## Status

🚀 **Active development.** M0 + **M1 (MVP)** are shipped (tagged `v0.1.0`), and
**M2 (v1.0)** is feature-complete on `main` with CI green — site management,
terminal enhancements, the full SFTP suite, port forwarding (`-L`/`-D`/`-R`),
and key generation + the encrypted vault. Remaining for the v1.0 tag: bastion
**ProxyJump**, PuTTY-session import, and broader real-world testing.

## Build from source

Prerequisites: **Rust** (stable), **Node.js 20+**, **pnpm** (the package
manager — easiest via `corepack enable`), and your platform's
[Tauri prerequisites](https://tauri.app/start/prerequisites/).

```bash
corepack enable        # makes pnpm available (bundled with Node)
pnpm install
pnpm tauri dev         # run in development
pnpm tauri build       # produce a release bundle
```

## Documentation

- Product requirements: [docs/PRD-AmmaXterm-v0.5.md](docs/PRD-AmmaXterm-v0.5.md)
- Development plan / roadmap: [docs/開發計畫-AmmaXterm.md](docs/開發計畫-AmmaXterm.md)

### Roadmap (high level)

| Milestone | Goal | Status |
|-----------|------|--------|
| **M0** | Tech validation: single SSH shell + basic SFTP + host-key check | ✅ Done |
| **M1 (MVP)** | Site CRUD, secure credential storage, multi-tab terminal, SFTP basics, first release | ✅ Shipped (`v0.1.0`) |
| **M2 (v1.0)** | Groups/search/import, reconnect, transfer queue, **port forwarding**, key gen + vault | ✅ Feature-complete (ProxyJump / PuTTY import pending) |
| **M3 (v1.x)** | Split panes, edit-in-place, tags, broadcast input, full i18n | ⬜ Planned |

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) and our
[Code of Conduct](CODE_OF_CONDUCT.md). Report security issues privately per
[SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE) © AmmaXterm contributors. Third-party dependencies retain their
own licenses (Apache-2.0 / BSD / MIT); see their respective notices.
