# Release engineering

VibeBus 0.9 has one repeatable Windows release path shared by local validation and GitHub Actions. Pull requests build unsigned acceptance packages. Production tag releases are fail-closed: they must sign both the executable and MSI before GitHub Release publication.

## Outputs

`scripts/build-release.ps1` produces these files under ignored `dist/`:

| File | Purpose |
| --- | --- |
| `VibeBus-X.Y.Z-windows-x64.msi` | Per-user Windows Installer package |
| `VibeBus-X.Y.Z-windows-x64.zip` | Portable marketplace root with plugin, README, and license |
| `VibeBus-Codex-plugin-X.Y.Z.zip` | Standalone `vibebus` plugin directory |
| `SHA256SUMS.txt` | SHA-256 checksums generated after all signing |
| `release-manifest.json` | Version, platform, signed state, sizes, and hashes |

The MSI installs to `%LOCALAPPDATA%\Programs\VibeBus`, adds the bundled plugin binary directory to the current user's `PATH`, and includes the repository-shaped local marketplace at the install root. It does not run Codex or mutate Codex configuration through a custom action. Register the installed marketplace explicitly:

```powershell
codex plugin marketplace add "$env:LOCALAPPDATA\Programs\VibeBus"
codex plugin add vibebus@vibebus-local
```

## Local build and acceptance

The repository pins Rust 1.97.1 in `rust-toolchain.toml` and WiX 4.0.6 in `.config/dotnet-tools.json`.

```powershell
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
./scripts/build-release.ps1
$msi = Get-ChildItem ./dist/VibeBus-*-windows-x64.msi | Select-Object -First 1
./scripts/test-installer.ps1 -MsiPath $msi.FullName
```

The installer acceptance runs the stock MSI ICE checks except ICE91. ICE91 only warns that the intentionally per-user payload would not behave like a per-machine payload. The acceptance then creates an administrative image in a unique temporary directory, verifies the marketplace, manifest, MCP configuration, binary, hook, script, and Skill, executes the extracted binary, and removes the temporary image.

Local builds are unsigned unless `-Sign` is supplied with both signing environment variables. This permits PR validation without sharing a private key. A local unsigned package is not a production release.

## Authenticode signing

`scripts/sign-windows.ps1` locates the newest x64 SignTool from the installed Windows SDK, decodes a temporary PFX, signs with SHA-256, requests an RFC 3161 SHA-256 timestamp, verifies with the default Authenticode policy, and deletes the temporary PFX in `finally`.

The production workflow requires two repository or `release` environment secrets:

- `WINDOWS_SIGNING_CERTIFICATE_BASE64`: Base64 form of a code-signing PFX;
- `WINDOWS_SIGNING_CERTIFICATE_PASSWORD`: its password.

The optional repository variable `WINDOWS_TIMESTAMP_URL` overrides the default `http://timestamp.digicert.com`. Use the timestamp service supplied by the certificate authority when it differs.

To set the PFX without printing its Base64 value:

```powershell
$pfxBase64 = [Convert]::ToBase64String([IO.File]::ReadAllBytes("C:\private\vibebus-signing.pfx"))
$pfxBase64 | gh secret set WINDOWS_SIGNING_CERTIFICATE_BASE64 --repo lakeofsky347/VibeBus
gh secret set WINDOWS_SIGNING_CERTIFICATE_PASSWORD --repo lakeofsky347/VibeBus
Remove-Variable pfxBase64
```

Do not commit a PFX, Base64 certificate, password, or decoded temporary file. GitHub repository secrets have a size limit; review the GitHub guidance before using an unusually large certificate bundle.

## GitHub Actions behavior

`.github/workflows/ci.yml` runs on pull requests, `main` pushes, and manual dispatch. It has read-only `contents` permission and performs formatting, tests, Clippy-as-error, release packaging, MSI validation, administrative extraction, and 14-day workflow artifact upload.

`.github/workflows/release.yml` runs for `v*.*.*` tag pushes or manual dispatch against an existing tag. It:

1. validates the exact `vX.Y.Z` tag against Cargo and plugin versions;
2. repeats all Rust gates;
3. refuses to continue unless both signing secrets exist;
4. signs and verifies `vibebus.exe` before staging;
5. builds the MSI from the signed payload, then signs and verifies the MSI;
6. creates the archives, checksums, and signed release manifest;
7. validates the MSI and uploads workflow artifacts;
8. publishes the five assets with `gh release create --verify-tag --generate-notes` using only `contents: write` on the job-scoped `GITHUB_TOKEN`.

Creating and pushing a release tag is an explicit maintainer action. The workflow never creates a missing tag, and no release is published from an ordinary branch or pull request.

## Maintainer release checklist

1. Confirm the release commit is accepted and all stacked changes intended for the release are present.
2. Confirm `Cargo.toml`, `Cargo.lock`, and `plugins/vibebus/.codex-plugin/plugin.json` use the same version.
3. Confirm the `release` environment and signing secrets are configured by the repository owner.
4. Run the local unsigned acceptance path.
5. Create and push an annotated `vX.Y.Z` tag.
6. Wait for the Release workflow, then download the assets and verify `SHA256SUMS.txt`.
7. Verify Authenticode on both executable and MSI with `signtool verify /pa /tw /v` or `Get-AuthenticodeSignature`.
8. Install on a disposable Windows user profile, register the installed marketplace, start a new Codex task, then uninstall and verify PATH/configuration cleanup.

## Deliberate boundaries

- WiX 4.0.6 is pinned under the Microsoft Reciprocal License. WiX 7 was evaluated but requires an explicit Open Source Maintenance Fee EULA acceptance; automated acceptance on behalf of a maintainer is intentionally excluded.
- The current pipeline accepts PFX-based SignTool signing because it works with GitHub-hosted Windows runners and any compatible certificate authority. Azure Trusted Signing or hardware-backed signing can replace only the signing step later.
- Packages are repeatably produced from the same scripts and locked dependencies, but are not claimed to be bit-for-bit reproducible because MSI product codes, archive metadata, and signing timestamps can differ.
- GitHub artifact attestations are not enabled. The repository is private, and private-repository attestations require an eligible GitHub Enterprise Cloud plan. Add them when the repository plan and verification policy support them.
- No real production certificate, tag, or GitHub Release was used during the 0.8 implementation acceptance. The unsigned local package path and fail-closed missing-secret boundary are the currently verified states.
