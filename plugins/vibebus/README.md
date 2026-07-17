# VibeBus Codex plugin

This plugin bundles the VibeBus coordination skill, a native Windows stdio MCP server, and a read-only session-start discovery hook. Version 0.7 adds repeatable Windows CI, per-user MSI and portable release packaging, checksums, and fail-closed Authenticode signing for production releases on top of Windows credential-vault storage, confirmed bounded retention, recoverable sessions, message/thread lifecycles, replay-safe subscriptions, renewable reservations, and structured handoffs.

The hook requires explicit trust in Codex. It only walks upward from the session working directory, reads `.vibebus/project.json` when present, and adds concise coordination instructions to the session context.

The packaged executable belongs at `bin/vibebus.exe`. Build and package it from the repository root with `powershell -File scripts/package-plugin.ps1`.
