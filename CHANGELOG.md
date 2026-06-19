# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
