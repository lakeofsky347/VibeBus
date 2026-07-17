# VibeBus Codex plugin

This plugin bundles the VibeBus coordination skill, a native Windows stdio MCP server, and a read-only session-start discovery hook. Version 0.4 adds message closing and durable task-to-Codex-thread bindings on top of recoverable sessions, replay-safe subscriptions, renewable reservations, and structured handoffs.

The hook requires explicit trust in Codex. It only walks upward from the session working directory, reads `.vibebus/project.json` when present, and adds concise coordination instructions to the session context.

The packaged executable belongs at `bin/vibebus.exe`. Build and package it from the repository root with `powershell -File scripts/package-plugin.ps1`.
