# Contributing to AmmaXterm

Thanks for your interest in contributing! AmmaXterm is an MIT-licensed,
cross-platform SSH terminal built with **Tauri v2 + Rust + SvelteKit**.

## Prerequisites

- **Rust** (stable, via [rustup](https://rustup.rs))
- **Node.js** 20+ and npm
- **Tauri OS prerequisites** — see <https://tauri.app/start/prerequisites/>
  - Windows: WebView2 Runtime + MSVC Build Tools
  - macOS: Xcode Command Line Tools
  - Linux: `webkit2gtk`, `librsvg`, `libssl`, build essentials

## Build from source

```bash
npm install          # install frontend dependencies
npm run tauri dev    # run the app in development mode
npm run tauri build  # produce a release bundle
```

Frontend-only dev server: `npm run dev`.

## Development workflow

1. Fork and create a feature branch from `main`.
2. Keep changes focused; one logical change per PR.
3. Ensure checks pass locally before opening a PR:
   - Rust: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
   - Frontend: `npm run check`
4. Update `CHANGELOG.md` under `[Unreleased]` when behavior changes.
5. Open a PR using the template; link related issues.

## Code style

- Rust: `rustfmt` defaults + `clippy` clean.
- Commits: [Conventional Commits](https://www.conventionalcommits.org) are
  encouraged (e.g. `feat(ssh): add keyboard-interactive auth`).
- Versioning: [Semantic Versioning](https://semver.org).

## Scope

Please review the product scope before proposing large features:
[docs/PRD-AmmaXterm-v0.5.md](docs/PRD-AmmaXterm-v0.5.md) and the roadmap in
[docs/開發計畫-AmmaXterm.md](docs/開發計畫-AmmaXterm.md). Out-of-scope items
(other protocols, built-in editor, cloud sync, etc.) are intentionally excluded.

## Security

Do **not** file security issues publicly — see [SECURITY.md](SECURITY.md).

## License

By contributing, you agree that your contributions will be licensed under the
MIT License. Keep new dependencies under permissive licenses (MIT / Apache-2.0
/ BSD); avoid GPL/AGPL to preserve license compatibility.
