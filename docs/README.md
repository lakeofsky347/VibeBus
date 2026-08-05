# VibeBus Documentation

Welcome to the VibeBus documentation hub. VibeBus is a local, structured fact bus for independent coding-agent tasks. This directory hosts the full documentation set in two languages.

**Choose your language / 选择语言:**

| Language | Entry point |
| --- | --- |
| English | [docs/en/README.md](en/README.md) — full documentation set |
| 简体中文 | [docs/zh-CN/README.md](zh-CN/README.md) — 完整文档集 |

## How the language routing works

- **English is the canonical source.** All authoritative content lives under `docs/en/` and mirrors the repository's source of truth. When a change lands in the codebase, update the English document first.
- **Simplified Chinese is a translation.** Chinese documents live under `docs/zh-CN/` and are translated from the English source with identical structure, code blocks, tables, and links. They are not the source of truth.
- **Routing by path.** There is no runtime configuration: the directory path itself selects the language (`docs/en/*.md` vs `docs/zh-CN/*.md`). GitHub renders whichever path you open. Cross-language links follow the convention `../en/<file>.md` and `../zh-CN/<file>.md`.
- **Root-level files** (`README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`) remain in English at the repository root because GitHub, tooling, and release workflows expect them there; their Chinese translations live in `docs/zh-CN/`.

## Document index

| Document | English | 简体中文 |
| --- | --- | --- |
| README (project overview) | [en/README.md](en/README.md) | [zh-CN/README.md](zh-CN/README.md) |
| Architecture | [en/architecture.md](en/architecture.md) | [zh-CN/architecture.md](zh-CN/architecture.md) |
| CLI and MCP protocol | [en/protocol.md](en/protocol.md) | [zh-CN/protocol.md](zh-CN/protocol.md) |
| Multi-tool adapters | [en/multi-tool-adapters.md](en/multi-tool-adapters.md) | [zh-CN/multi-tool-adapters.md](zh-CN/multi-tool-adapters.md) |
| Cross-agent collaboration (design) | [en/cross-agent-collaboration.md](en/cross-agent-collaboration.md) | [zh-CN/cross-agent-collaboration.md](zh-CN/cross-agent-collaboration.md) |
| Design research and decisions | [en/design-research.md](en/design-research.md) | [zh-CN/design-research.md](zh-CN/design-research.md) |
| Backup, export, and restore | [en/backup-restore.md](en/backup-restore.md) | [zh-CN/backup-restore.md](zh-CN/backup-restore.md) |
| Release engineering | [en/release.md](en/release.md) | [zh-CN/release.md](zh-CN/release.md) |
| macOS development | [en/macos.md](en/macos.md) | [zh-CN/macos.md](zh-CN/macos.md) |
| Container development | [en/container.md](en/container.md) | [zh-CN/container.md](zh-CN/container.md) |
| Maintainer handoff | [en/HANDOFF.md](en/HANDOFF.md) | [zh-CN/HANDOFF.md](zh-CN/HANDOFF.md) |
| Changelog | [CHANGELOG.md](../CHANGELOG.md) | [zh-CN/CHANGELOG.md](zh-CN/CHANGELOG.md) |
| Contributing | [CONTRIBUTING.md](../CONTRIBUTING.md) | [zh-CN/CONTRIBUTING.md](zh-CN/CONTRIBUTING.md) |
| Security policy | [SECURITY.md](../SECURITY.md) | [zh-CN/SECURITY.md](zh-CN/SECURITY.md) |
| Code of conduct | [CODE_OF_CONDUCT.md](../CODE_OF_CONDUCT.md) | [zh-CN/CODE_OF_CONDUCT.md](zh-CN/CODE_OF_CONDUCT.md) |

## Translation status

See [i18n/translation-matrix.md](i18n/translation-matrix.md) for the complete translation coverage matrix, missing translations, and stale-content tracking.

## Maintenance

See [i18n/maintenance.md](i18n/maintenance.md) for the bilingual maintenance workflow: how to add a new document, translate it, and keep both languages in sync.

## Plugin and runtime files

- `plugins/*/README.md` — per-adapter user-facing READMEs. `plugins/vibebus/README.md` is the canonical Codex adapter README and is bilingual-friendly.
- `plugins/*/skills/vibebus-coordination/SKILL.md` — **runtime files loaded by MCP and agent tooling. They must remain in English and are deliberately excluded from translation** (see the maintenance guide).
- `plugins/opencode/AGENTS.md` — runtime instruction file, English only.
