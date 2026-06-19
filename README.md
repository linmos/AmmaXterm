# AmmaXterm

> A lightweight, open-source, cross-platform SSH terminal — SSH, SFTP, site
> management, and port forwarding done simply, fast, and securely.
>
> 輕量、開源、跨平台的 SSH 終端工具 —— 把 SSH 終端、SFTP 傳檔、站台管理與
> 連接埠轉發做到簡單、快速、安全。

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Status](https://img.shields.io/badge/status-early%20development-orange)

---

## What is AmmaXterm?

AmmaXterm focuses on the few things people actually use a remote tool for, and
does them well:

- **SSH terminal** — SSH 2.0 with password / public-key / keyboard-interactive
  auth, multi-tab sessions, true color, UTF-8, and mandatory host-key verification.
- **SFTP file transfer** — browse, upload/download (recursive), and manage
  remote files.
- **Site management** — save, group, search, and quick-connect to hosts.
- **Port forwarding** — Local (`-L`), Remote (`-R`), and Dynamic SOCKS (`-D`),
  with secure defaults (binds to `127.0.0.1`).

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

🚧 **Early development.** Currently at **M0 (technical validation)** — proving
the Tauri + russh + xterm.js pipeline. See the roadmap below.

## Build from source

Prerequisites: **Rust** (stable), **Node.js 20+**, and your platform's
[Tauri prerequisites](https://tauri.app/start/prerequisites/).

```bash
npm install
npm run tauri dev     # run in development
npm run tauri build   # produce a release bundle
```

## Documentation

- Product requirements: [docs/PRD-AmmaXterm-v0.5.md](docs/PRD-AmmaXterm-v0.5.md)
- Development plan / roadmap: [docs/開發計畫-AmmaXterm.md](docs/開發計畫-AmmaXterm.md)

### Roadmap (high level)

| Milestone | Goal |
|-----------|------|
| **M0** | Tech validation: single SSH shell + basic SFTP + host-key check |
| **M1 (MVP)** | Site CRUD, secure credential storage, multi-tab terminal, SFTP basics, first release |
| **M2 (v1.0)** | Groups/search/import, reconnect, bastion (ProxyJump), transfer queue, **port forwarding** |
| **M3 (v1.x)** | Split panes, edit-in-place, tags, broadcast input, full i18n |

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) and our
[Code of Conduct](CODE_OF_CONDUCT.md). Report security issues privately per
[SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE) © AmmaXterm contributors. Third-party dependencies retain their
own licenses (Apache-2.0 / BSD / MIT); see their respective notices.
