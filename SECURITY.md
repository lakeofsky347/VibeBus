# Security policy

## Supported versions

Security fixes are developed for the latest stable GitHub Release and the `main` branch. Older releases may receive guidance, but are not guaranteed to receive backports. Check [GitHub Releases](https://github.com/lakeofsky347/VibeBus/releases) for the current stable version, its GitHub-generated source archives, and its source-verification evidence set.

Stable source-only releases are protected by pinned GitHub Actions, Rust quality gates, `cargo deny` advisory/license/source gates, normalized CycloneDX SBOM generation, immutable tag-to-`main` verification, and SHA-256 checksums for the uploaded evidence files. These controls reduce risk; they do not replace independent review before deployment.

## Reporting a vulnerability

Do not open a public issue, discussion, or pull request for a suspected vulnerability. Use the repository's [private vulnerability reporting form](https://github.com/lakeofsky347/VibeBus/security/advisories/new).

Include the affected version or commit, supported platform, impact, minimal safe reproduction, and suggested mitigation where possible. Do not attach Agent tokens, recovery keys, Operator secrets, Keychain or Credential Manager exports, signing material, cloud credentials, local databases, or unrelated personal data.

Maintainers will acknowledge and triage reports privately. Coordinated disclosure timing depends on severity, reproduction, affected users, and availability of a safe fix. Do not assume an issue is fixed until a maintainer identifies a released version or published advisory.

## Security-sensitive contributions

Security changes must preserve project scoping, credential redaction, explicit Operator approval for destructive maintenance, and the bounded no-transcript/no-raw-log lifecycle-fact boundary. If a proposed change weakens one of these controls, explain the threat model, alternatives, migration, tests, and rollback plan in the private report or pull request as appropriate.
