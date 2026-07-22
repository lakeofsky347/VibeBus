# Release engineering

## Distribution contract

GitHub Releases are VibeBus's only official distribution channel. The first `v0.10.0` release is source-only: GitHub automatically provides the tagged **Source code (zip)** and **Source code (tar.gz)** archives, while the workflow uploads only source-verification evidence. Stable releases use SemVer tags in the exact form `vX.Y.Z`. The workflow checks that the tag resolves to the checked-out commit and that the commit is reachable from `main`; it refuses release-candidate and branch names. A pull-request artifact, local package, container image, or manually copied file is never a release substitute.

Release candidates are intentionally disabled until the project adopts a separate, testable prerelease contract. Do not create `vX.Y.Z-rc.N` expecting the stable workflow to publish it.

## Stable release assets

The source-only workflow hand-uploads exactly four evidence files:

| Asset | Purpose |
| --- | --- |
| `SHA256SUMS.txt` | SHA-256 values for the other three uploaded evidence files. |
| `VibeBus-X.Y.Z.cdx.json` | Normalized CycloneDX SBOM for the tagged source. |
| `supply-chain-evidence.json` | Tagged source revision and completed `cargo deny` gates. |
| `source-release-manifest.json` | Version, tag, source revision, GitHub-provided source archive provenance, and evidence names. |

GitHub supplies the source ZIP and tarball automatically from the immutable tag; VibeBus does not build or upload a duplicate source ZIP. There are no Windows MSI, portable ZIP, Codex plugin ZIP, signed executable, or installer assets in this first release. macOS, Windows, and Linux remain source and CI-supported paths. A future binary distribution must use a separately reviewed contract that restores the applicable platform signing, package verification, and downloaded-artifact acceptance.

## Maintainer procedure

1. Merge reviewed changes to `main`; ensure required CI, supply-chain checks, and review gates pass.
2. Confirm `Cargo.toml` and plugin manifest versions match the intended SemVer version. Update `CHANGELOG.md` before tagging.
3. Create and push an annotated `vX.Y.Z` tag that points to a commit reachable from `main`.
4. Let `.github/workflows/release.yml` run from that existing tag. It does not create missing tags, publish from an untagged commit, use a protected release environment, or require Windows signing material for the source-only release.
5. Inspect the GitHub-generated source archives, the four uploaded evidence files, and the workflow logs. Do not edit a release asset or rewrite a tag; publish a corrective patch release instead.

## Verifying a download

Download the GitHub-generated source archive and all four uploaded evidence files from the same GitHub Release. First verify `source-release-manifest.json`: its `tag` and `sourceRevision` must identify the immutable release tag and a commit reachable from `main`. Then verify the three evidence files covered by `SHA256SUMS.txt`:

```powershell
Get-Content .\SHA256SUMS.txt | ForEach-Object {
  $parts = $_ -split '\s{2,}', 2
  if ($parts.Count -ne 2) { throw "Malformed checksum entry: $_" }
  $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $parts[1]).Hash.ToLowerInvariant()
  if ($actual -ne $parts[0]) { throw "Checksum mismatch: $($parts[1])" }
}
```

Confirm that the manifest names the GitHub automatic source ZIP and tarball, the SBOM contains no local build path, and the supply-chain evidence records the same source revision. No first-install, upgrade, binary signature, or MSI rollback assertion applies to this source-only release.

## Local and CI acceptance

Pull-request CI continues to exercise Linux, macOS, and Windows source and package acceptance. The source-only release workflow itself runs the version, Rust quality, `cargo deny`, and normalized SBOM gates on Linux before publishing evidence; it does not use a Windows runner, build an installer, or create binary release assets.
