# VibeBus Codex plugin

This plugin bundles the VibeBus coordination skill, a native Windows stdio MCP server, and three Windows lifecycle Hooks. Version 0.10 adds responsibility-domain policy, task-scoped expiring overrides, bounded Git/test facts, and a review-only terminal handoff proposal. Agent-authenticated operations remain available through MCP, but operator initialization, rotation, credential restoration, explicit vault deletion, and retention approval are deliberately CLI-only and require a real terminal.

Hooks require explicit trust in Codex and must be reviewed again when their definitions change. SessionStart only walks upward and reads `.vibebus/project.json`. PostToolUse observes Bash exit metadata, Git commit identity/path lists, and bounded test-command facts; it does not read transcripts, diffs, or test logs. Stop writes a bounded proposal under `PLUGIN_DATA` for review and never sends a handoff automatically. Hook failure is surfaced as degradation and cannot undo completed tool side effects.

The packaged executable belongs at `bin/vibebus.exe`. Build and package it from the repository root with `powershell -File scripts/package-plugin.ps1`.
