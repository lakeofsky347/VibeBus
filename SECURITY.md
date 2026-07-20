# Security policy

## Supported versions

Security fixes are developed against the latest `0.10.x` source and its current stacked pull-request baseline. Older snapshots and local backup artifacts are retained only as recovery evidence and are not separately supported release lines.

## Report a vulnerability privately

Do not open a public issue, discussion, or pull request for a suspected vulnerability. Use the repository's [private vulnerability reporting form](https://github.com/lakeofsky347/VibeBus/security/advisories/new) so maintainers can review the report before disclosure.

Include the affected version or commit, platform, impact, minimal reproduction steps, and suggested mitigation when available. Do not include live Agent tokens, recovery keys, Operator secrets, Windows Credential Manager or macOS Keychain exports, signing/notarization credentials, cloud credentials, database copies, or unrelated personal data. Use placeholders in reproductions.

VibeBus is a local coordination boundary, not an operating-system sandbox. Reports should distinguish application authorization bypasses from raw filesystem access already available to another process running as the same operating-system user.

## Disclosure

Please allow maintainers time to reproduce and remediate the issue before public disclosure. No fixed response or remediation SLA is promised. Accepted fixes must preserve credential redaction, project scoping, explicit Operator approval for destructive maintenance, and the no-transcript/no-raw-log lifecycle-fact boundary.
