# VibeBus Codex plugin

This plugin bundles the VibeBus coordination skill, a native Windows stdio MCP server, and a read-only session-start discovery hook. Version 0.8 adds a separate local operator credential and short-lived, single-use interactive approval for destructive retention. Agent-authenticated planning and application remain available through MCP, but operator initialization, rotation, credential restoration, and approval are deliberately CLI-only and require a real terminal.

The hook requires explicit trust in Codex. It only walks upward from the session working directory, reads `.vibebus/project.json` when present, and adds concise coordination instructions to the session context.

The packaged executable belongs at `bin/vibebus.exe`. Build and package it from the repository root with `powershell -File scripts/package-plugin.ps1`.
