# Backup, export, and restore drill

VibeBus online backups are consistent SQLite recovery points. A usable export consists of three pieces kept together:

1. the project marker `.vibebus/project.json`;
2. the database produced by `vibebus backup`;
3. the returned SHA-256 digest and byte count in an external recovery inventory.

The database contains Agent and Operator credential digests, not the secrets stored in Windows Credential Manager. A move to another Windows user or host therefore also requires a separately retained Agent recovery key or another explicit identity-recovery decision. Never add bearer tokens, recovery keys, Operator secrets, vault exports, signing credentials, or cloud credentials to the recovery bundle.

## Automated disposable drill

Run the repository-owned drill after building the release binary:

```powershell
./scripts/test-backup-restore.ps1 -BinaryPath ./target/release/vibebus.exe
```

The script uses only a random directory below the system temporary directory. It creates one Agent and one task, takes an online backup, creates a second task after that recovery point, and imports the marker plus backup into a separate root and data home. Acceptance requires:

- the imported file hash to equal the backup SHA-256;
- `doctor.ok=true`, schema 11, WAL, and foreign keys enabled;
- the restored Agent credential digest to accept the original in-process token;
- exactly the pre-backup task to exist in the restored project;
- the post-backup task to remain absent;
- the temporary source, export, and restored data to be deleted;
- no credential or recovery secret to be printed.

The drill never reads, replaces, compacts, or mutates the live project database.

## Production recovery procedure

VibeBus deliberately does not automate destructive cutover. For a real recovery:

1. Stop every CLI, MCP, Hook, and Codex task using the target project. Record the maintenance window outside VibeBus because the authoritative database is about to be offline.
2. Create one final online backup of the current database when possible and retain its hash. Do not overwrite an older recovery point.
3. Copy the target project's `.vibebus/project.json` to an isolated recovery root. Confirm that its `projectId` is the intended project.
4. Copy the selected backup, without editing it, to `<isolated-data-home>/projects/<projectId>/vibebus.db`.
5. Run `doctor`, `status`, and authenticated read checks against the isolated root and isolated data home. Compare task, Agent, artifact, binding, reservation, event, and retention expectations with the recovery inventory.
6. If the operating-system vault entry is unavailable, use a separately secured Agent recovery key through the documented recovery flow. Do not treat the database digest as a recoverable secret.
7. Only after isolated verification, make a separate, maintainer-controlled offline cutover decision. Keep the displaced database as a rollback artifact and restart one client first before reopening the project to all tasks.

Do not copy only `vibebus.db` into an unrelated project marker. Do not replace the database while any VibeBus process is open. Do not restore by replaying raw SQL or editing project IDs.

## Physical compaction boundary

Logical retention remains independent from SQLite file compaction, and the restore drill never runs `VACUUM`. Physical compaction is now available only as `vibebus maintenance compact --root <project> --backup <new-path>` from a real terminal. It requires the exact `compact:<project-id>` confirmation, the current vault-backed Operator secret, complete project downtime, zero active tasks/bindings/reservations, a fail-fast exclusive SQLite boundary, at least twice the database size free on its volume, and a new backup path. VibeBus creates and verifies that backup before `VACUUM`, then restores/checkpoints WAL and returns bounded before/after hashes, sizes, page counts, reclaimed bytes, and integrity evidence. It is absent from MCP. Always test the command first on a disposable restored copy; the repository tests invoke compaction only under temporary project/data homes, and this implementation round did not run it against the live VibeBus project.
