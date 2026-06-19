# Security Policy

AmmaXterm handles SSH credentials and private keys, so we take security
seriously. Thank you for helping keep the project and its users safe.

## Supported Versions

During pre-1.0 development, only the latest release receives security fixes.

| Version | Supported |
|---------|-----------|
| latest  | ✅        |
| older   | ❌        |

## Reporting a Vulnerability

**Please do not open a public issue for security vulnerabilities.**

Instead, report privately via GitHub's **"Report a vulnerability"** button
under the repository's **Security** tab (GitHub Security Advisories). If that
is unavailable, contact the maintainers privately.

When reporting, please include:

- A description of the issue and its impact.
- Steps to reproduce (proof of concept if possible).
- Affected version / commit and platform (Windows / macOS / Linux).
- Any suggested remediation.

### What to expect

- We aim to acknowledge reports within **5 business days**.
- We will keep you informed of progress and coordinate a disclosure timeline.
- With your consent, we will credit you in the release notes.

## Security Posture (by design)

- Credentials are stored in the OS keychain (`keyring`); passwords/keys are
  never written to disk in plaintext.
- Host key verification is mandatory; key changes trigger a clear warning.
- Port forwarding binds to `127.0.0.1` by default; remote forwarding is
  off by default and warns about network exposure.
- Secrets in memory are zeroized as soon as they are no longer needed.
- No telemetry is collected.

See [docs/PRD-AmmaXterm-v0.5.md](docs/PRD-AmmaXterm-v0.5.md) §6.1 for details.
