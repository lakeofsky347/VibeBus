# Contributing to VibeBus

Thanks for contributing. By participating, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Start with an issue or discussion

Use the issue templates for reproducible bugs and feature proposals. Do not open a public issue for a security vulnerability; follow [SECURITY.md](SECURITY.md) instead. Small documentation fixes can be proposed directly, but please describe the problem they solve.

## Development expectations

1. Work from a focused branch and keep each pull request to one reviewable concern.
2. Preserve the project's security boundaries: do not add credentials, recovery material, local databases, backups, generated binaries, or release archives.
3. Update tests and documentation when behavior, compatibility, installation, or release policy changes.
4. Use the project language and style already established in the surrounding files.

For Rust, protocol, or plugin changes, run:

```sh
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
```

Run platform packaging or installer checks when your change affects them. Pure documentation changes may use narrower validation, but the pull request must state what was checked and what was not run.

## Pull requests

Use the pull-request template. Explain the user-visible purpose, validation, compatibility impact, and SemVer impact. Keep generated artifacts out of the pull request; GitHub Releases are created only from an accepted stable tag on `main`.

Maintainers review changes for correctness, tests, documentation, security, compatibility, and release readiness. A pull request is not a release and does not authorize a tag, a GitHub Release, or external publishing.

## Versioning and changelog

VibeBus uses SemVer. Add an entry under `Unreleased` in [CHANGELOG.md](CHANGELOG.md) for user-visible fixes, features, breaking changes, security changes, and changes to distribution behavior. Keep historic entries immutable; describe the current behavior rather than rewriting release history.
