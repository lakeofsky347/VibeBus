# Release engineering

## G1 production boundary

The frozen `v0.10.0` candidate is a **Windows x64 signed GitHub Release only**. macOS packages remain local-development acceptance artifacts and the Linux image remains a separately controlled container delivery path; neither is authorized for this production release. The release workflow refuses to publish from a dirty checkout, a missing tag, or a source tree that tracks `plugins/vibebus/bin/vibebus.exe`.

VibeBus 0.10 has repeatable Windows and macOS local/CI packaging paths. Windows production tag releases remain fail-closed: they must sign both the executable and MSI before publication. macOS currently produces an ad-hoc-signed local acceptance package; Developer ID signing and notarization are explicit production gates and are not claimed by that artifact.

The plugin manifest uses the officially supported `interface.composerIcon` and `interface.logo` fields. Those paths remain relative to the plugin root and point to the transparent PNGs under `plugins/vibebus/assets/`. The release staging step copies the complete plugin tree, while the WiX payload lists all four light/dark icon and logo files explicitly so MSI and ZIP delivery stay aligned.

## Outputs

`scripts/build-release.ps1` produces these files under ignored `dist/`:

| File | Purpose |
| --- | --- |
| `VibeBus-X.Y.Z-windows-x64.msi` | Per-user Windows Installer package |
| `VibeBus-X.Y.Z-windows-x64.zip` | Portable marketplace root with plugin, README, and license |
| `VibeBus-Codex-plugin-X.Y.Z.zip` | Standalone `vibebus` plugin directory |
| `SHA256SUMS.txt` | SHA-256 checksums generated after all signing |
| `release-manifest.json` | Version, platform, signed state, sizes, and hashes |

`scripts/package-plugin-macos.sh` additionally produces host-architecture macOS artifacts:

| File | Purpose |
| --- | --- |
| `VibeBus-X.Y.Z-macos-arm64.tar.gz` | Portable marketplace root for Apple Silicon |
| `VibeBus-Codex-plugin-X.Y.Z-macos-arm64.zip` | Standalone macOS plugin directory |
| `SHA256SUMS-macos-arm64.txt` | SHA-256 checksums for both archives |
| `release-manifest-macos-arm64.json` | Version, platform, ad-hoc signature state, sizes, and hashes |

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
cargo install cargo-deny --version 0.20.2 --locked
cargo install cargo-cyclonedx --version 0.5.9 --locked
cargo deny check advisories bans licenses sources
cargo cyclonedx --format json
./scripts/test-lifecycle-hooks.ps1
./scripts/build-release.ps1
$msi = Get-ChildItem ./dist/VibeBus-*-windows-x64.msi | Select-Object -First 1
./scripts/test-installer.ps1 -MsiPath $msi.FullName
```

The lifecycle acceptance runs deterministic dry-run fixtures for Git commit capture, test-result capture, unknown-outcome refusal, and review-only Stop proposals. The installer acceptance runs the stock MSI ICE checks except ICE91. ICE91 only warns that the intentionally per-user payload would not behave like a per-machine payload. The acceptance then creates an administrative image in a unique temporary directory, verifies the marketplace, manifest, MCP configuration, binary, Hooks, scripts, and Skill, executes the extracted binary, and removes the temporary image.

The plugin validator also checks that the manifest icon and logo references resolve to non-empty PNG files. Visual review remains a human-facing acceptance step: inspect each generated asset on both light and dark surfaces before publishing a package.

Local builds are unsigned unless `-Sign` is supplied with both signing environment variables. This permits PR validation without sharing a private key. A local unsigned package is not a production release.

## macOS local build and acceptance

The macOS path requires the Xcode Command Line Tools, Python 3 for repository-only JSON validation, and Rust 1.97.1. The packaged plugin runtime itself is a single native binary and does not require Python, PowerShell, Node, or jq.

```sh
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
./scripts/test-lifecycle-hooks.sh
./scripts/test-macos-keychain.sh
./scripts/package-plugin-macos.sh
./scripts/validate-plugin-macos.sh \
  ./dist/staging/VibeBus-0.10.0-macos-arm64/plugins/vibebus
(cd dist && shasum -a 256 -c SHA256SUMS-macos-arm64.txt)
```

The build copies the optimized Mach-O to `bin/vibebus`, applies an ad-hoc signature by default, substitutes the macOS MCP configuration only inside the staged marketplace, removes the Windows executable from the macOS payload, and validates the binary/version/manifest/Hook/Skill contract before archiving. It supports native `arm64` and `x86_64` hosts, naming the latter `x64`; only `arm64` has current live acceptance. Supplying `VIBEBUS_CODESIGN_IDENTITY` signs with that identity, hardened runtime, and a secure timestamp and records `signature: "identity"`; this does not by itself claim Developer ID provenance or notarization.

The CI workflow runs the Rust gates, seven native Hook fixtures, disposable Keychain lifecycle, package validation, and checksum verification on `macos-latest`, then uploads the local acceptance artifacts for 14 days.

## macOS production signing boundary

Ad-hoc signing proves that the local Mach-O is internally consistent after packaging, but it does not establish publisher identity or satisfy downloaded-artifact Gatekeeper policy. A public macOS release must add all of these maintainer-owned gates before publication:

1. sign the final binary with a Developer ID Application identity and hardened runtime;
2. submit the final archive or containing app/package to Apple notarization and wait for acceptance;
3. staple the notarization ticket where the distribution shape supports stapling;
4. verify `codesign`, `spctl`, checksums, architecture, and execution from a freshly downloaded quarantined artifact on a disposable user profile;
5. keep the certificate/private key and App Store Connect/notary credentials outside the repository and logs.

The repository does not store or simulate these credentials. `release-manifest-macos-*.json` reports `signature: "adhoc"` by default and `signature: "identity"` only when the caller explicitly supplies `VIBEBUS_CODESIGN_IDENTITY`; notarization remains a separate fail-closed gate.

## Authenticode signing

`scripts/sign-windows.ps1` locates the newest x64 SignTool from the installed Windows SDK, decodes a temporary PFX, signs with SHA-256, requests an RFC 3161 SHA-256 timestamp, verifies with the default Authenticode policy, and deletes the temporary PFX in `finally`.

The production workflow requires two repository or `release` environment secrets:

- `WINDOWS_SIGNING_CERTIFICATE_BASE64`: Base64 form of a code-signing PFX;
- `WINDOWS_SIGNING_CERTIFICATE_PASSWORD`: its password.

The optional repository variable `WINDOWS_TIMESTAMP_URL` overrides the default `https://timestamp.digicert.com`. Only HTTPS RFC 3161 timestamp endpoints are accepted; use the certificate authority's HTTPS endpoint when it differs.

## Supply-chain gates and candidate evidence

Both CI and the tag workflow install pinned `cargo-deny 0.20.2` and `cargo-cyclonedx 0.5.9`. `deny.toml` fails the build on RustSec advisories, yanked crates, unapproved licenses, unknown registries, and unknown Git dependencies. The exception lists are empty for this candidate. Before an SBOM is uploaded or released, `scripts/normalize-cyclonedx.py` replaces cargo's local `path+file` references with stable package URLs, removes volatile serial/timestamp metadata, and recursively rejects the GitHub workspace, runner temp directory, current user home, or any absolute file path. It uses only the Python standard library; run `python3 scripts/normalize-cyclonedx.py --self-test` locally before generating a sanitized candidate.

The tag workflow publishes a CycloneDX `*.cdx.json` SBOM plus `supply-chain-evidence.json` with the source revision and the four completed `cargo-deny` gates. Their SHA-256 entries are appended to `SHA256SUMS.txt`; treat these files, the signed MSI/portable/plugin archives, checksums, and `release-manifest.json` as one candidate evidence set. Artifact upload does not substitute for the later downloaded-artifact verification.

To set the PFX without printing its Base64 value:

```powershell
$pfxBase64 = [Convert]::ToBase64String([IO.File]::ReadAllBytes("C:\private\vibebus-signing.pfx"))
$pfxBase64 | gh secret set WINDOWS_SIGNING_CERTIFICATE_BASE64 --repo lakeofsky347/VibeBus
gh secret set WINDOWS_SIGNING_CERTIFICATE_PASSWORD --repo lakeofsky347/VibeBus
Remove-Variable pfxBase64
```

Do not commit a PFX, Base64 certificate, password, or decoded temporary file. GitHub repository secrets have a size limit; review the GitHub guidance before using an unusually large certificate bundle.

## GitHub Actions behavior

`.github/workflows/ci.yml` runs on pull requests, `main` pushes, and manual dispatch. It has read-only `contents` permission and pins every third-party Action to an immutable SHA. A Linux supply-chain job produces the CycloneDX candidate evidence and runs the `cargo-deny` gates. Windows performs formatting, tests, Clippy-as-error, PowerShell Hook fixtures, release packaging, MSI validation, administrative extraction, disposable install/uninstall/PATH/marketplace cleanup, and artifact upload. macOS independently performs local-development Rust/package acceptance; it is not a G1 production target. Linux retains the non-root container path.

`.github/workflows/release.yml` runs for `v*.*.*` tag pushes or manual dispatch against an existing tag. It:

1. validates the exact `vX.Y.Z` tag against Cargo and plugin versions;
2. repeats all Rust gates;
3. refuses to continue unless both signing secrets exist;
4. signs and verifies `vibebus.exe` before staging;
5. builds the MSI from the signed payload, then signs and verifies the MSI;
6. creates the archives, checksums, signed release manifest, CycloneDX SBOM, and supply-chain evidence;
7. validates the MSI, including a disposable install/uninstall/PATH/marketplace-cleanup exercise, and uploads workflow artifacts;
8. publishes the candidate assets with `gh release create --verify-tag --generate-notes` using only `contents: write` on the job-scoped `GITHUB_TOKEN`.

Creating and pushing a release tag is an explicit maintainer action. The workflow never creates a missing tag, and no release is published from an ordinary branch or pull request.

## Maintainer release checklist

1. Confirm the release commit is accepted and all stacked changes intended for the release are present.
2. Confirm `Cargo.toml`, `Cargo.lock`, and `plugins/vibebus/.codex-plugin/plugin.json` use the same version.
3. Confirm the `release` environment and signing secrets are configured by the repository owner.
4. Run the local unsigned acceptance path.
5. Create and push an annotated `vX.Y.Z` tag.
6. Wait for the Release workflow, then download the assets and verify `SHA256SUMS.txt`.
7. Verify Authenticode on both executable and MSI with `signtool verify /pa /tw /v` or `Get-AuthenticodeSignature`.
8. Install on a disposable Windows user profile, register the installed marketplace, start a new Codex task, then uninstall and verify user `PATH` and marketplace cleanup. If a prior MSI is available, run `test-installer.ps1 -PreviousMsiPath <prior.msi> -ExerciseLifecycle` to exercise the major upgrade before the final uninstall.

## Deliberate boundaries

- WiX 4.0.6 is pinned under the Microsoft Reciprocal License. WiX 7 was evaluated but requires an explicit Open Source Maintenance Fee EULA acceptance; automated acceptance on behalf of a maintainer is intentionally excluded.
- The current pipeline accepts PFX-based SignTool signing because it works with GitHub-hosted Windows runners and any compatible certificate authority. Azure Trusted Signing or hardware-backed signing can replace only the signing step later.
- Packages are repeatably produced from the same scripts and locked dependencies, but are not claimed to be bit-for-bit reproducible because MSI product codes, archive metadata, and signing timestamps can differ.
- GitHub artifact attestations are not enabled. The repository is private, and private-repository attestations require an eligible GitHub Enterprise Cloud plan. Add them when the repository plan and verification policy support them.
- No real production certificate, tag, or GitHub Release was used during the 0.8 implementation acceptance. The unsigned local package path and fail-closed missing-secret boundary are the currently verified states.
