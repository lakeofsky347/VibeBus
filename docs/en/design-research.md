# Design research and decision record

Decision date: 2026-07-17.

## Compared approaches

| Project or standard | Useful idea | Why it is not the VibeBus core |
| --- | --- | --- |
| [mco-org/squad](https://github.com/mco-org/squad) | Lightweight multi-agent coordination and a permissive reference implementation | VibeBus must preserve independent Codex top-level tasks and native Windows operation instead of adopting another orchestration shell |
| [MCP Agent Mail](https://github.com/Dicklesworthstone/mcp_agent_mail) | Directed inboxes, acknowledgements, and file-reservation behavior | Its implementation and licensing constraints are not used as a code base; VibeBus implements the behavior independently in Rust |
| [Beads](https://github.com/steveyegge/beads) | Durable issue graph and dependency-aware work | Its broader issue-tracker and Dolt-oriented storage model add operational weight; VibeBus needs one small multi-process SQLite source of truth |
| [A2A protocol](https://github.com/a2aproject/A2A) | Interoperable task and artifact semantics | A2A targets networked agent interoperability; VibeBus is deliberately local-first and project-scoped |
| Native Codex tasks and worktrees | Correct isolation, user-visible task ownership, native UI | These remain the execution boundary. VibeBus augments them rather than replacing them |

## Chosen synthesis

The selected design is a thin native Rust binary with one SQLite WAL database per project, exposed through CLI and MCP and distributed as a Codex plugin.

This combination keeps the strongest properties from the references:

- inbox and ACK semantics from mailbox-style coordination;
- atomic ownership and dependencies from task-graph systems;
- artifact-shaped outcomes compatible with broader agent protocols;
- native Codex task and worktree isolation;
- a small auditable deployment surface with no local daemon requirement.

## Rejected alternatives

- A shared Markdown or JSON file: weak concurrent writes, no atomic claim, noisy diffs, and accidental repository coupling.
- One Git branch as the message bus: merge latency and conflicts obscure operational state.
- A local HTTP service: extra lifecycle, port, firewall, authentication, and crash-recovery concerns without an MVP benefit.
- Direct manipulation of Codex internal task state: unstable and outside the product boundary.
- Full transcript synchronization: expensive, privacy-heavy, and contrary to independent-task isolation.

## Compatibility direction

Future network or A2A bridges should translate from the event and artifact model. They must remain adapters; the local SQLite facts stay authoritative for a VibeBus project.

## Release engineering decision

The 0.7 release slice compared four Windows distribution shapes:

| Approach | Strength | Decision |
| --- | --- | --- |
| Portable ZIP only | No installer dependency and easy inspection | Kept as a fallback, but insufficient for PATH and uninstall lifecycle |
| MSIX | Strong platform identity and signing model | Deferred because local/private distribution requires certificate identity and deployment policy that would complicate the MVP |
| Inno Setup or NSIS EXE | Flexible UI and scripting | Rejected for now because custom scripting increases the installer attack and maintenance surface |
| WiX MSI | Native Windows Installer upgrade/uninstall semantics and stock ICE validation | Selected, with an explicitly per-user, no-custom-action package |

WiX 7.0.0 was tested first because it is current, but its command-line tool requires explicit OSMF EULA acceptance. VibeBus does not automate legal acceptance on behalf of a maintainer, so the repository pins WiX 4.0.6 under its published MS-RL license. The v4 schema and command-line flow cover the required MSI without extensions.

For signing, the chosen baseline is SignTool with a CA-issued PFX stored only as GitHub Secrets. It is provider-neutral and available on GitHub-hosted Windows runners. The production workflow signs the executable before MSI construction, signs the MSI afterward, and verifies both. Cloud or hardware-backed signing remains a replaceable adapter when certificate operations justify it.

## Destructive-maintenance authorization decision

Version 0.8 compared three retention authorization shapes:

| Approach | Strength | Decision |
| --- | --- | --- |
| Agent token plus typed plan ID | Simple and already implemented | Rejected as the only gate because the same autonomous caller can both plan and delete |
| Pass a long-lived operator secret through MCP or command arguments | Easy to automate | Rejected because it puts the stronger capability in model-visible inputs and removes meaningful separation |
| Interactive CLI approval stored as a short-lived database capability | Separates planning from local operator consent without exposing the secret to MCP | Selected |

The selected flow keeps SQLite authoritative while adding a procedural human-presence boundary. Operator initialization, rotation, vault restoration, and approval require a real terminal and exact typed confirmation. The secret is stored in a distinct Windows Credential Manager target; the database stores only its digest and generation. An approval binds the exact plan ID, policy, expiry, and operator generation. Apply consumes it atomically with cleanup and report storage.

This is deliberately not claimed as protection from a malicious process already executing as the same Windows user. Such a process sits inside the current local trust boundary. The capability prevents routine MCP/Agent workflows and redirected automation from accidentally authorizing destructive maintenance, while retaining a clear future adapter point for hardware-backed or multi-party approval.
