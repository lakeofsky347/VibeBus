# Release engineering

## Distribution contract

GitHub Releases are VibeBus's only official distribution channel. Stable releases use SemVer tags in the exact form `vX.Y.Z`. The workflow checks that the tag resolves to the checked-out commit and that the commit is reachable from `main`; it refuses release-candidate and branch names. A pull-request artifact, local package, container image, or manually copied file is never a release substitute.

Release candidates are intentionally disabled until the project adopts a separate, testable prerelease contract. Do not create `vX.Y.Z-rc.N` expecting the stable workflow to publish it.

## Stable release assets

Each stable GitHub Release must contain one coherent evidence set:

| Asset | Purpose |
| --- | --- |
| `VibeBus-X.Y.Z-source.zip` | Source archive from the tagged commit. |
| `VibeBus-X.Y.Z-windows-x64.msi` | Signed per-user Windows installer. |
| `VibeBus-X.Y.Z-windows-x64.zip` | Signed portable marketplace payload. |
| `VibeBus-Codex-plugin-X.Y.Z.zip` | Signed standalone Codex plugin payload. |
| `SHA256SUMS.txt` | SHA-256 values for distributable and supply-chain files. |
| `release-manifest.json` | Version, source revision, source ref, platform, signing state, artifacts, and evidence names. |
| `VibeBus-X.Y.Z-windows-x64.cdx.json` | Normalized CycloneDX SBOM. |
| `supply-chain-evidence.json` | Source revision and completed `cargo deny` gates. |

macOS and Linux are source and CI-supported today. They are not GitHub Release assets until maintainers define and verify platform-specific public distribution, signing, and downloaded-artifact acceptance.

## Maintainer procedure

1. Merge reviewed changes to `main`; ensure required CI, supply-chain checks, and review gates pass.
2. Confirm `Cargo.toml` and plugin manifest versions match the intended SemVer version. Update `CHANGELOG.md` before tagging.
3. Create and push an annotated `vX.Y.Z` tag that points to a commit reachable from `main`.
4. Configure the protected `release` environment and Windows signing secrets outside the repository. The workflow fails closed if either signing value is absent.
5. Let `.github/workflows/release.yml` build from the existing tag. It does not create missing tags or publish from an untagged commit.
6. Inspect the GitHub Release evidence set and workflow logs. Do not edit a release asset in place; publish a corrective patch release instead.

## Verifying a download

Download every asset listed above from the same GitHub Release. Verify the checksums before installing:

```powershell
Get-Content .\SHA256SUMS.txt | ForEach-Object {
  $parts = $_ -split '\s{2,}', 2
  if ($parts.Count -ne 2) { throw "Malformed checksum entry: $_" }
  $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $parts[1]).Hash.ToLowerInvariant()
  if ($actual -ne $parts[0]) { throw "Checksum mismatch: $($parts[1])" }
}
Get-AuthenticodeSignature .\VibeBus-X.Y.Z-windows-x64.msi
```

Then install on a disposable Windows user profile, verify `vibebus --version`, register the marketplace explicitly, and uninstall. The installer is per-user and does not modify Codex configuration through custom actions.

For a subsequent upgrade, retain the prior downloaded MSI and run:

```powershell
./scripts/test-installer.ps1 `
  -MsiPath .\VibeBus-X.Y.Z-windows-x64.msi `
  -PreviousMsiPath .\VibeBus-X.Y.Z-previous-windows-x64.msi `
  -ExerciseLifecycle
```

Windows MSI deliberately blocks an in-place downgrade. To roll back, preserve project data independently, uninstall the newer MSI, install the earlier verified MSI, then repeat smoke tests. A failed upgrade should use Windows Installer rollback behavior first; do not force-delete the installation directory or marketplace files while the installer transaction is active.

## Local and CI acceptance

Local and pull-request builds are unsigned acceptance artifacts. They exercise packaging, plugin validation, MSI extraction, and installer lifecycle checks but never claim a GitHub Release. The release workflow preserves the pinned three-platform CI, source-built plugin, `cargo deny`, normalized SBOM, checksums, and Windows signing gate.
