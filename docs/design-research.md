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
