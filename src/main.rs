use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use serde_json::json;
use vibebus::{
    AgentHook, AgentProfiles, Bus, BusError, CredentialVault, Result, RetentionPolicy,
    SecretSource, StoredOperatorCredential, discover_project, initialize_project, mcp::run_mcp,
    operator_credential_delivery, recovery_delivery, recovery_key_delivery, registration_delivery,
    resolve_agent_recovery_key, resolve_agent_token, resolve_operator_secret, run_agent_hook,
    spawn_once, system_credential_vault,
};

#[derive(Debug, Parser)]
#[command(
    name = "vibebus",
    version,
    about = "Local coordination bus for independent coding-agent tasks"
)]
struct Cli {
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,

    #[arg(long, global = true)]
    data_home: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init {
        #[arg(long)]
        name: String,
    },
    Register {
        #[arg(long)]
        name: String,
        #[arg(long)]
        role: String,
        #[arg(long)]
        store_credentials: bool,
    },
    Agents,
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
    Credential {
        #[command(subcommand)]
        command: CredentialCommand,
    },
    Operator {
        #[command(subcommand)]
        command: OperatorCommand,
    },
    Api {
        #[command(subcommand)]
        command: ApiCommand,
    },
    Send(SendArgs),
    Inbox(AgentAuthArgs),
    Read(MessageAuthArgs),
    Ack(MessageAuthArgs),
    Close(MessageAuthArgs),
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    Thread {
        #[command(subcommand)]
        command: ThreadCommand,
    },
    Reserve {
        #[command(subcommand)]
        command: ReservationCommand,
    },
    Artifact {
        #[command(subcommand)]
        command: ArtifactCommand,
    },
    Decision {
        #[command(subcommand)]
        command: DecisionCommand,
    },
    Responsibility {
        #[command(subcommand)]
        command: ResponsibilityCommand,
    },
    Fact {
        #[command(subcommand)]
        command: FactCommand,
    },
    Context {
        #[command(subcommand)]
        command: ContextCommand,
    },
    Event {
        #[command(subcommand)]
        command: EventCommand,
    },
    Subscription {
        #[command(subcommand)]
        command: SubscriptionCommand,
    },
    Retention {
        #[command(subcommand)]
        command: RetentionCommand,
    },
    Handoff {
        #[command(subcommand)]
        command: HandoffCommand,
    },
    Status,
    Doctor,
    Backup {
        #[arg(long)]
        output: PathBuf,
    },
    Maintenance {
        #[command(subcommand)]
        command: MaintenanceCommand,
    },
    #[command(hide = true)]
    Hook {
        #[command(subcommand)]
        command: HookCommand,
    },
    Mcp,
    #[command(subcommand)]
    Skill(SkillCommand),
    #[command(subcommand)]
    Plugin(PluginCommand),
}

#[derive(Debug, Subcommand)]
enum SkillCommand {
    /// List registered skills (id, version, agents_supported).
    List,
    /// Install a registered skill into an agent CLI's local skill directory.
    Install {
        #[arg(long)]
        id: String,
        #[arg(long)]
        agent: String,
    },
    /// Register or update a skill from a SKILL.md file.
    Register {
        #[arg(long)]
        id: String,
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "1.0.0")]
        version: String,
        #[arg(long)]
        file: PathBuf,
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        agents: Vec<String>,
        /// Prerequisite skill ids installed first (comma-separated).
        #[arg(long, value_delimiter = ',', num_args = 0..)]
        deps: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
enum PluginCommand {
    /// List registered plugins.
    List,
}

#[derive(Debug, Subcommand)]
enum HookCommand {
    SessionStart,
    PostToolUse,
    Stop,
}

#[derive(Debug, Args)]
struct SendArgs {
    #[arg(long)]
    from: String,
    #[arg(long)]
    token: Option<String>,
    #[arg(long, value_delimiter = ',', num_args = 1..)]
    to: Vec<String>,
    #[arg(long)]
    subject: String,
    #[arg(long)]
    body: String,
    #[arg(long)]
    thread: Option<String>,
    #[arg(long, default_value = "normal")]
    priority: String,
    #[arg(long)]
    requires_ack: bool,
    #[arg(long)]
    idempotency_key: Option<String>,
}

#[derive(Debug, Args)]
struct AgentAuthArgs {
    #[arg(long)]
    agent: String,
    #[arg(long)]
    token: Option<String>,
    #[arg(long)]
    all: bool,
    #[arg(long)]
    include_closed: bool,
}

#[derive(Debug, Args)]
struct MessageAuthArgs {
    #[arg(long)]
    agent: String,
    #[arg(long)]
    token: Option<String>,
    #[arg(long)]
    message: String,
}

#[derive(Debug, Subcommand)]
enum TaskCommand {
    Create {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        id: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long, value_delimiter = ',')]
        depends_on: Vec<String>,
    },
    Claim {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        id: String,
    },
    Update {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        id: String,
        #[arg(long)]
        expected_version: i64,
        #[arg(long)]
        status: String,
        #[arg(long)]
        blocked_reason: Option<String>,
    },
    Complete {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        id: String,
        #[arg(long)]
        expected_version: i64,
    },
    Show {
        #[arg(long)]
        id: String,
    },
    List,
}

#[derive(Debug, Subcommand)]
enum ThreadCommand {
    Bind {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        task: String,
        #[arg(long)]
        thread: String,
    },
    Unbind {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        task: String,
        #[arg(long)]
        thread: String,
    },
    List {
        #[arg(long)]
        task: Option<String>,
        #[arg(long)]
        all: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ReservationCommand {
    Add {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        path: String,
        #[arg(long, default_value_t = 3600)]
        ttl: i64,
        #[arg(long, default_value_t = true)]
        exclusive: bool,
        #[arg(long)]
        reason: Option<String>,
        #[arg(long)]
        task: Option<String>,
        #[arg(long)]
        idempotency_key: Option<String>,
    },
    Release {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        id: String,
    },
    Renew {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        id: String,
        #[arg(long, default_value_t = 3600)]
        ttl: i64,
        #[arg(long)]
        idempotency_key: Option<String>,
    },
    List,
}

#[derive(Debug, Subcommand)]
enum AgentCommand {
    Recover {
        #[arg(long)]
        name: String,
        #[arg(long)]
        recovery_key: Option<String>,
        #[arg(long)]
        store_credentials: bool,
    },
    ProvisionRecovery {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        store_credentials: bool,
    },
    /// Spawn an external agent CLI as a collaborator, link it to a VibeBus task,
    /// and print the outcome.
    Spawn {
        /// Agent CLI profile name (codex, claude, gemini, demo, or a custom one).
        #[arg(long)]
        profile: String,
        /// Authenticated VibeBus agent that owns the delegation task.
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        task: String,
        #[arg(long)]
        role: Option<String>,
        #[arg(long)]
        task_id: Option<String>,
        #[arg(long)]
        timeout_secs: Option<u64>,
        /// API provider (from `.vibebus/api.json`) whose env is injected into
        /// the spawned child. Defaults to the profile's `api` binding, then to
        /// the active provider.
        #[arg(long)]
        api: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum CredentialCommand {
    Status {
        #[arg(long)]
        agent: String,
    },
    Delete {
        #[arg(long)]
        agent: String,
    },
}

#[derive(Debug, Subcommand)]
enum OperatorCommand {
    Status,
    Init,
    Rotate,
    RestoreCredential,
    DeleteCredential,
    ApproveRetention {
        #[arg(long)]
        plan: String,
        #[arg(long, default_value_t = 600)]
        ttl: i64,
        #[command(flatten)]
        policy: RetentionPolicyArgs,
    },
}

#[derive(Debug, Subcommand)]
enum ApiCommand {
    /// Manage API providers (config lives in `.vibebus/api.json`, keys in the
    /// OS credential vault).
    Provider {
        #[command(subcommand)]
        command: ApiProviderCommand,
    },
    /// Activate a provider so its env is injected into spawned agents.
    Use {
        /// Provider name to activate.
        #[arg(long)]
        name: String,
        /// Override system-environment conflicts (injection wins).
        #[arg(long)]
        force: bool,
        /// Also write the injected env into Claude Code's project-level
        /// config (`.claude/settings.json`), backing up the original first.
        #[arg(long)]
        write_config: bool,
    },
    /// Deactivate the active provider (keys stay in the vault).
    Unuse,
    /// Restore a previous config from a write-ahead backup.
    Restore {
        /// Backup index, 1 = newest.
        #[arg(long, default_value_t = 1)]
        n: usize,
    },
    /// Sync providers from `.vibebus/api.json` into the bus registry (phase 3).
    SyncProviders,
    /// Local health check for a provider (config + key presence; no network).
    Health {
        #[arg(long)]
        name: String,
        /// Also probe TCP reachability of the Base URL host (default off).
        #[arg(long)]
        check_network: bool,
    },
    /// Launch an agent CLI with the active provider's env injected. The key is
    /// resolved from the OS vault at launch; nothing is written to disk.
    /// Example: `vibebus api run -- codex exec "task"`.
    Run {
        /// Agent CLI command and its arguments, everything after `--`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        command: Vec<String>,
    },
    /// Resolve a provider's API key from the vault to stdout. Used by the
    /// generated env export files (`source .vibebus/api/env.sh`); the key is
    /// never persisted by VibeBus itself.
    Key {
        #[command(subcommand)]
        command: ApiKeyCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ApiKeyCommand {
    /// Print the stored API key for a provider (stdout only).
    Get {
        #[arg(long)]
        name: String,
    },
}

#[derive(Debug, Subcommand)]
enum ApiProviderCommand {
    /// Add a provider; the API key goes to the OS credential vault only.
    Add {
        /// Provider name (ASCII letters/digits/`-`/`_`).
        #[arg(long)]
        name: String,
        /// Base URL without a trailing slash (e.g. https://voltapi.com).
        #[arg(long)]
        base_url: String,
        /// API format: anthropic | openai-chat | openai-responses | gemini.
        #[arg(long)]
        format: String,
        /// Optional default model injected via the protocol's model env var.
        #[arg(long)]
        model: Option<String>,
        /// API key. When omitted, prompts interactively (safe default).
        #[arg(long)]
        key: Option<String>,
    },
    /// List configured providers (names/formats/base URLs; never keys).
    List,
}

#[derive(Debug, Subcommand)]
enum ArtifactCommand {
    Publish {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        path: String,
        #[arg(long)]
        summary: String,
        #[arg(long)]
        task: Option<String>,
        #[arg(long)]
        metadata: Option<String>,
        #[arg(long, conflicts_with = "metadata")]
        metadata_file: Option<PathBuf>,
        #[arg(long)]
        idempotency_key: Option<String>,
    },
    List {
        #[arg(long)]
        task: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum DecisionCommand {
    Confirm {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        key: String,
        #[arg(long)]
        task: String,
        #[arg(long)]
        summary: String,
        #[arg(long, value_delimiter = ',')]
        artifacts: Vec<String>,
        #[arg(long)]
        idempotency_key: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum ResponsibilityCommand {
    Inspect {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
    },
    Override {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        task: String,
        #[arg(long)]
        grantee: String,
        #[arg(long)]
        path: String,
        #[arg(long)]
        reason: String,
        #[arg(long, default_value_t = 3600)]
        ttl: i64,
        #[arg(long)]
        idempotency_key: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum FactCommand {
    GitCommit {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        task: String,
        #[arg(long)]
        commit_sha: String,
        #[arg(long)]
        summary: String,
        #[arg(long = "changed-path", value_delimiter = ',')]
        changed_paths: Vec<String>,
        #[arg(long)]
        idempotency_key: Option<String>,
    },
    TestResult {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        task: String,
        #[arg(long)]
        result_key: String,
        #[arg(long)]
        suite: String,
        #[arg(long)]
        outcome: String,
        #[arg(long)]
        summary: String,
        #[arg(long = "command")]
        command_text: Option<String>,
        #[arg(long)]
        report_artifact: Option<String>,
        #[arg(long)]
        idempotency_key: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum ContextCommand {
    Sync {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long, default_value_t = 100)]
        item_limit: usize,
        #[arg(long, default_value_t = 65_536)]
        byte_budget: usize,
    },
}

#[derive(Debug, Subcommand)]
enum EventCommand {
    List {
        #[arg(long, default_value_t = 0)]
        after: i64,
        #[arg(long, default_value_t = 100)]
        limit: usize,
        #[arg(long, value_delimiter = ',')]
        event_types: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
enum SubscriptionCommand {
    Create {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        name: String,
        #[arg(long, value_delimiter = ',')]
        event_types: Vec<String>,
        #[arg(long)]
        from_sequence: Option<i64>,
    },
    List {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
    },
    Poll {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        name: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    Peek {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        name: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    Ack {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        name: String,
        #[arg(long)]
        delivery: String,
    },
}

#[derive(Debug, Subcommand)]
enum HandoffCommand {
    Send {
        #[arg(long)]
        from: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        to: Vec<String>,
        #[arg(long)]
        summary: String,
        #[arg(long)]
        task: Option<String>,
        #[arg(long, value_delimiter = ',')]
        decisions: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        artifacts: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        blockers: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        next_actions: Vec<String>,
        #[arg(long)]
        idempotency_key: Option<String>,
    },
    Snapshot {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long, default_value_t = 0)]
        after_sequence: i64,
    },
    Propose {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        task: String,
        #[arg(long, default_value_t = 10)]
        item_limit: usize,
    },
}

#[derive(Debug, Args)]
struct RetentionPolicyArgs {
    #[arg(long, default_value_t = 90)]
    event_max_age_days: i64,
    #[arg(long, default_value_t = 1_000)]
    keep_recent_events: i64,
    #[arg(long, default_value_t = 30)]
    idempotency_max_age_days: i64,
    #[arg(long, default_value_t = 30)]
    closed_message_max_age_days: i64,
    #[arg(long, default_value_t = 90)]
    terminal_binding_max_age_days: i64,
}

impl From<RetentionPolicyArgs> for RetentionPolicy {
    fn from(args: RetentionPolicyArgs) -> Self {
        Self {
            event_max_age_days: args.event_max_age_days,
            keep_recent_events: args.keep_recent_events,
            idempotency_max_age_days: args.idempotency_max_age_days,
            closed_message_max_age_days: args.closed_message_max_age_days,
            terminal_binding_max_age_days: args.terminal_binding_max_age_days,
        }
    }
}

#[derive(Debug, Subcommand)]
enum RetentionCommand {
    Plan {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[command(flatten)]
        policy: RetentionPolicyArgs,
    },
    Apply {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        plan: String,
        #[command(flatten)]
        policy: RetentionPolicyArgs,
    },
    Status,
}

#[derive(Debug, Subcommand)]
enum MaintenanceCommand {
    Compact {
        #[arg(long)]
        backup: PathBuf,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Command::Hook { command } = &cli.command {
        let hook = match command {
            HookCommand::SessionStart => AgentHook::SessionStart,
            HookCommand::PostToolUse => AgentHook::PostToolUse,
            HookCommand::Stop => AgentHook::Stop,
        };
        return match run_agent_hook(hook) {
            Ok(Some(value)) => {
                print_json(&value);
                ExitCode::SUCCESS
            }
            Ok(None) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("VibeBus hook failed: {error}");
                ExitCode::FAILURE
            }
        };
    }
    if matches!(&cli.command, Command::Mcp) {
        return match run_mcp(cli.root, cli.data_home).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("VibeBus MCP server failed: {error}");
                ExitCode::FAILURE
            }
        };
    }
    if let Command::Agent {
        command:
            AgentCommand::Spawn {
                profile,
                agent,
                token,
                task,
                role,
                task_id,
                timeout_secs,
                api,
            },
    } = &cli.command
    {
        return match handle_agent_spawn(
            &cli.root,
            profile,
            agent,
            token.as_deref(),
            task,
            role.as_deref(),
            task_id.as_deref(),
            *timeout_secs,
            api.as_deref(),
        )
        .await
        {
            Ok(value) => {
                print_json(&value);
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "ok": false,
                        "error": error.to_string(),
                        "kind": error_kind(&error)
                    }))
                    .expect("serialize CLI error")
                );
                ExitCode::FAILURE
            }
        };
    }

    match run(cli) {
        Ok(value) => {
            print_json(&value);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": false,
                    "error": error.to_string(),
                    "kind": error_kind(&error)
                }))
                .expect("serialize CLI error")
            );
            ExitCode::FAILURE
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_agent_spawn(
    root: &std::path::Path,
    profile: &str,
    agent: &str,
    token: Option<&str>,
    task: &str,
    role: Option<&str>,
    task_id: Option<&str>,
    timeout_secs: Option<u64>,
    api: Option<&str>,
) -> Result<serde_json::Value> {
    let profiles = AgentProfiles::load(root);
    let profile = profiles.resolve(profile).cloned().ok_or_else(|| {
        BusError::Validation(format!(
            "unknown agent profile '{}'; available: {}",
            profile,
            profiles
                .list()
                .iter()
                .map(|p| p.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })?;
    // Resolve the injection env before spawning so a missing key fails the
    // call instead of silently running without it. Precedence: explicit --api
    // > profile.api binding > active provider (resolve_api_env falls back to
    // the config's active provider when `api` is None).
    let mut bus = Bus::open(root, None)?;
    let project_id = bus.project().project_id.clone();
    let api_vault = vibebus::system_api_credential_vault();
    let api_config = vibebus::ApiConfig::load(root)?;
    let api_name = api.or(profile.api.as_deref());
    let api_env =
        match vibebus::resolve_api_env(&api_config, api_vault.as_ref(), &project_id, api_name)? {
            Some(resolved) => Some(resolved.vars),
            None => None,
        };
    let outcome = spawn_once(&profile, task, role, timeout_secs, api_env.as_ref()).await?;
    // Link the spawn to a durable VibeBus task + message (same logic as the
    // MCP tool). One-shot CLI cannot reply, so a Question state still leaves
    // the task claimed for the host to handle.
    let vault = system_credential_vault();
    let resolved = resolve_agent_token(vault.as_ref(), &project_id, agent, token, None)?;
    let link = vibebus::spawn::link_spawn_outcome(
        &mut bus,
        agent,
        &resolved.value,
        &outcome,
        task_id,
        task,
        role,
        &profile.command,
    )?;
    Ok(json!({
        "ok": true,
        "result": outcome,
        "link": link,
        "api": api_env.map(|env| serde_json::json!({
            "provider": profile.api.or(api.map(str::to_owned)).unwrap_or_else(|| api_config.active.clone().unwrap_or_default()),
            "injectedVars": env.keys().collect::<Vec<_>>(),
        })),
    }))
}

fn run(cli: Cli) -> Result<serde_json::Value> {
    if let Command::Init { name } = &cli.command {
        let initialized = initialize_project(&cli.root, name, cli.data_home.as_deref())?;
        let bus = Bus::open(&cli.root, cli.data_home.as_deref())?;
        return Ok(json!({
            "ok": true,
            "project": initialized.project,
            "markerPath": initialized.marker_path,
            "databasePath": bus.database_path().to_string_lossy(),
            "journalMode": "WAL"
        }));
    }

    if let Command::Maintenance {
        command: MaintenanceCommand::Compact { backup },
    } = &cli.command
    {
        let (_, project) = discover_project(&cli.root)?;
        require_interactive_confirmation(
            "Compact the project database offline. Type compact:<project-id> to continue",
            &format!("compact:{}", project.project_id),
        )?;
        let vault = system_credential_vault();
        let operator_secret = resolve_operator_secret(vault.as_ref(), &project.project_id)?;
        return Ok(json!({
            "ok": true,
            "result": Bus::compact_offline(
                &cli.root,
                cli.data_home.as_deref(),
                &operator_secret,
                backup,
            )?
        }));
    }

    let mut bus = Bus::open(&cli.root, cli.data_home.as_deref())?;
    let project_id = bus.project().project_id.clone();
    let vault = system_credential_vault();
    let result = match cli.command {
        Command::Init { .. } => unreachable!(),
        Command::Register {
            name,
            role,
            store_credentials,
        } => {
            let registration = bus.register_agent(&name, &role)?;
            registration_delivery(
                vault.as_ref(),
                &project_id,
                &registration,
                store_credentials,
            )
        }
        Command::Agents => json!(bus.list_agents()?),
        Command::Agent { command } => match command {
            AgentCommand::Recover {
                name,
                recovery_key,
                store_credentials,
            } => {
                let resolved = resolve_agent_recovery_key(
                    vault.as_ref(),
                    &project_id,
                    &name,
                    recovery_key.as_deref(),
                )?;
                let store_credentials = store_credentials || resolved.source == SecretSource::Vault;
                let recovery = bus.recover_agent(&name, &resolved.value)?;
                recovery_delivery(vault.as_ref(), &project_id, &recovery, store_credentials)
            }
            AgentCommand::ProvisionRecovery {
                agent,
                token,
                store_credentials,
            } => {
                let resolved = resolve_cli_secret(vault.as_ref(), &project_id, &agent, token)?;
                let store_credentials = store_credentials || resolved.source == SecretSource::Vault;
                let recovery = bus.provision_recovery_key(&agent, &resolved.value)?;
                recovery_key_delivery(
                    vault.as_ref(),
                    &project_id,
                    &resolved.value,
                    &recovery,
                    store_credentials,
                )
            }
            AgentCommand::Spawn { .. } => unreachable!("agent spawn is handled in main"),
        },
        Command::Credential { command } => match command {
            CredentialCommand::Status { agent } => {
                json!(vault.status(&project_id, &agent)?)
            }
            CredentialCommand::Delete { agent } => {
                let deleted = vault.delete(&project_id, &agent)?;
                json!({
                    "deleted": deleted,
                    "credentials": vault.status(&project_id, &agent)?
                })
            }
        },
        Command::Operator { command } => match command {
            OperatorCommand::Status => {
                let operator = bus.operator_status()?;
                let credential = vault.operator_status(&project_id)?;
                let ready = operator.configured
                    && credential.stored
                    && operator.generation == credential.generation;
                json!({
                    "ready": ready,
                    "operator": operator,
                    "credential": credential
                })
            }
            OperatorCommand::Init => {
                require_interactive_confirmation(
                    "Initialize the project operator credential. Type the full project ID to continue",
                    &project_id,
                )?;
                let credential = bus.initialize_operator()?;
                operator_credential_delivery(vault.as_ref(), &project_id, &credential)
            }
            OperatorCommand::Rotate => {
                require_interactive_confirmation(
                    "Rotate the project operator credential. Type rotate:<project-id> to continue",
                    &format!("rotate:{project_id}"),
                )?;
                let current = resolve_operator_secret(vault.as_ref(), &project_id)?;
                let credential = bus.rotate_operator(&current)?;
                operator_credential_delivery(vault.as_ref(), &project_id, &credential)
            }
            OperatorCommand::RestoreCredential => {
                require_interactive_confirmation(
                    "Restore the current operator secret to the OS vault. Type restore:<project-id> to continue",
                    &format!("restore:{project_id}"),
                )?;
                let secret = rpassword::prompt_password("Current operator secret: ")?;
                let generation = bus.verify_operator_secret(&secret)?;
                let credential = StoredOperatorCredential::new(secret, generation);
                vault.store_operator(&project_id, &credential)?;
                let operator = bus.operator_status()?;
                let credential = vault.operator_status(&project_id)?;
                json!({
                    "restored": true,
                    "ready": operator.generation == credential.generation,
                    "operator": operator,
                    "credential": credential
                })
            }
            OperatorCommand::DeleteCredential => {
                require_interactive_confirmation(
                    "Delete the project operator credential from the OS vault. Type delete:<project-id> to continue",
                    &format!("delete:{project_id}"),
                )?;
                let deleted = vault.delete_operator(&project_id)?;
                let operator = bus.operator_status()?;
                let credential = vault.operator_status(&project_id)?;
                let ready = operator.configured
                    && credential.stored
                    && operator.generation == credential.generation;
                json!({
                    "deleted": deleted,
                    "ready": ready,
                    "operator": operator,
                    "credential": credential
                })
            }
            OperatorCommand::ApproveRetention { plan, ttl, policy } => {
                let policy: RetentionPolicy = policy.into();
                let current = bus.preview_retention_for_operator(&policy)?;
                if current.plan_id != plan {
                    return Err(BusError::Conflict(format!(
                        "retention plan is stale: confirmed '{plan}', current '{}'",
                        current.plan_id
                    )));
                }
                eprintln!(
                    "Retention candidates: {}",
                    serde_json::to_string_pretty(&current)?
                );
                require_interactive_confirmation(
                    "Approve this retention plan. Type the full plan ID to continue",
                    &plan,
                )?;
                let secret = resolve_operator_secret(vault.as_ref(), &project_id)?;
                json!(bus.approve_retention(&secret, &policy, &plan, ttl)?)
            }
        },
        Command::Api { command } => {
            let api_vault = vibebus::system_api_credential_vault();
            handle_api_command(
                &mut bus,
                &cli.root,
                &project_id,
                api_vault.as_ref(),
                command,
            )?
        }
        Command::Send(args) => {
            let token = resolve_token(vault.as_ref(), &project_id, &args.from, args.token)?;
            json!(bus.send_message_idempotent(
                &args.from,
                &token,
                &args.to,
                &args.subject,
                &args.body,
                args.thread.as_deref(),
                &args.priority,
                args.requires_ack,
                args.idempotency_key.as_deref(),
            )?)
        }
        Command::Inbox(args) => {
            let token = resolve_token(vault.as_ref(), &project_id, &args.agent, args.token)?;
            json!(bus.inbox_with_options(&args.agent, &token, !args.all, args.include_closed,)?)
        }
        Command::Read(args) => {
            let token = resolve_token(vault.as_ref(), &project_id, &args.agent, args.token)?;
            json!(bus.mark_read(&args.agent, &token, &args.message)?)
        }
        Command::Ack(args) => {
            let token = resolve_token(vault.as_ref(), &project_id, &args.agent, args.token)?;
            json!(bus.acknowledge_message(&args.agent, &token, &args.message)?)
        }
        Command::Close(args) => {
            let token = resolve_token(vault.as_ref(), &project_id, &args.agent, args.token)?;
            json!(bus.close_message(&args.agent, &token, &args.message)?)
        }
        Command::Task { command } => match command {
            TaskCommand::Create {
                agent,
                token,
                id,
                title,
                description,
                depends_on,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.create_task(
                    &agent,
                    &token,
                    &id,
                    &title,
                    description.as_deref(),
                    &depends_on,
                )?)
            }
            TaskCommand::Claim { agent, token, id } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.claim_task(&agent, &token, &id)?)
            }
            TaskCommand::Update {
                agent,
                token,
                id,
                expected_version,
                status,
                blocked_reason,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.update_task(
                    &agent,
                    &token,
                    &id,
                    expected_version,
                    &status,
                    blocked_reason.as_deref(),
                )?)
            }
            TaskCommand::Complete {
                agent,
                token,
                id,
                expected_version,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.update_task(&agent, &token, &id, expected_version, "completed", None,)?)
            }
            TaskCommand::Show { id } => json!(bus.get_task(&id)?),
            TaskCommand::List => json!(bus.list_tasks()?),
        },
        Command::Thread { command } => match command {
            ThreadCommand::Bind {
                agent,
                token,
                task,
                thread,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.bind_task_thread(&agent, &token, &task, &thread)?)
            }
            ThreadCommand::Unbind {
                agent,
                token,
                task,
                thread,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.unbind_task_thread(&agent, &token, &task, &thread)?)
            }
            ThreadCommand::List { task, all } => {
                json!(bus.list_task_thread_bindings(task.as_deref(), !all)?)
            }
        },
        Command::Reserve { command } => match command {
            ReservationCommand::Add {
                agent,
                token,
                path,
                ttl,
                exclusive,
                reason,
                task,
                idempotency_key,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.reserve_path_for_task_idempotent(
                    &agent,
                    &token,
                    &path,
                    ttl,
                    exclusive,
                    reason.as_deref(),
                    task.as_deref(),
                    idempotency_key.as_deref(),
                )?)
            }
            ReservationCommand::Release { agent, token, id } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.release_reservation(&agent, &token, &id)?)
            }
            ReservationCommand::Renew {
                agent,
                token,
                id,
                ttl,
                idempotency_key,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.renew_reservation_idempotent(
                    &agent,
                    &token,
                    &id,
                    ttl,
                    idempotency_key.as_deref(),
                )?)
            }
            ReservationCommand::List => json!(bus.list_active_reservations()?),
        },
        Command::Artifact { command } => match command {
            ArtifactCommand::Publish {
                agent,
                token,
                kind,
                path,
                summary,
                task,
                metadata,
                metadata_file,
                idempotency_key,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                let metadata_text = if let Some(path) = metadata_file {
                    Some(std::fs::read_to_string(path)?)
                } else {
                    metadata
                };
                let metadata = metadata_text
                    .as_deref()
                    .map(serde_json::from_str)
                    .transpose()?;
                json!(bus.publish_artifact_idempotent(
                    &agent,
                    &token,
                    &kind,
                    &path,
                    &summary,
                    task.as_deref(),
                    metadata.as_ref(),
                    idempotency_key.as_deref(),
                )?)
            }
            ArtifactCommand::List { task } => json!(bus.list_artifacts(task.as_deref())?),
        },
        Command::Decision { command } => match command {
            DecisionCommand::Confirm {
                agent,
                token,
                key,
                task,
                summary,
                artifacts,
                idempotency_key,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.confirm_decision_idempotent(
                    &agent,
                    &token,
                    &key,
                    &task,
                    &summary,
                    &artifacts,
                    idempotency_key.as_deref(),
                )?)
            }
        },
        Command::Responsibility { command } => match command {
            ResponsibilityCommand::Inspect { agent, token } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.inspect_responsibility_policy(&agent, &token)?)
            }
            ResponsibilityCommand::Override {
                agent,
                token,
                task,
                grantee,
                path,
                reason,
                ttl,
                idempotency_key,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.grant_responsibility_override_idempotent(
                    &agent,
                    &token,
                    &task,
                    &grantee,
                    &path,
                    &reason,
                    ttl,
                    idempotency_key.as_deref(),
                )?)
            }
        },
        Command::Fact { command } => match command {
            FactCommand::GitCommit {
                agent,
                token,
                task,
                commit_sha,
                summary,
                changed_paths,
                idempotency_key,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.record_git_commit_idempotent(
                    &agent,
                    &token,
                    &task,
                    &commit_sha,
                    &summary,
                    &changed_paths,
                    idempotency_key.as_deref(),
                )?)
            }
            FactCommand::TestResult {
                agent,
                token,
                task,
                result_key,
                suite,
                outcome,
                summary,
                command_text,
                report_artifact,
                idempotency_key,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.record_test_result_idempotent(
                    &agent,
                    &token,
                    &task,
                    &result_key,
                    &suite,
                    &outcome,
                    &summary,
                    command_text.as_deref(),
                    report_artifact.as_deref(),
                    idempotency_key.as_deref(),
                )?)
            }
        },
        Command::Context { command } => match command {
            ContextCommand::Sync {
                agent,
                token,
                cursor,
                item_limit,
                byte_budget,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.context_sync(
                    &agent,
                    &token,
                    cursor.as_deref(),
                    item_limit,
                    byte_budget,
                )?)
            }
        },
        Command::Event { command } => match command {
            EventCommand::List {
                after,
                limit,
                event_types,
            } => json!(bus.list_events(after, limit, &event_types)?),
        },
        Command::Subscription { command } => match command {
            SubscriptionCommand::Create {
                agent,
                token,
                name,
                event_types,
                from_sequence,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.create_subscription(
                    &agent,
                    &token,
                    &name,
                    &event_types,
                    from_sequence,
                )?)
            }
            SubscriptionCommand::List { agent, token } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.list_subscriptions(&agent, &token)?)
            }
            SubscriptionCommand::Poll {
                agent,
                token,
                name,
                limit,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.poll_subscription(&agent, &token, &name, limit)?)
            }
            SubscriptionCommand::Peek {
                agent,
                token,
                name,
                limit,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.peek_subscription(&agent, &token, &name, limit)?)
            }
            SubscriptionCommand::Ack {
                agent,
                token,
                name,
                delivery,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.acknowledge_subscription(&agent, &token, &name, &delivery)?)
            }
        },
        Command::Retention { command } => match command {
            RetentionCommand::Plan {
                agent,
                token,
                policy,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.plan_retention(&agent, &token, &policy.into())?)
            }
            RetentionCommand::Apply {
                agent,
                token,
                plan,
                policy,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.apply_retention(&agent, &token, &policy.into(), &plan)?)
            }
            RetentionCommand::Status => json!(bus.retention_state()?),
        },
        Command::Handoff { command } => match command {
            HandoffCommand::Send {
                from,
                token,
                to,
                summary,
                task,
                decisions,
                artifacts,
                blockers,
                next_actions,
                idempotency_key,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &from, token)?;
                json!(bus.send_handoff(
                    &from,
                    &token,
                    &to,
                    &summary,
                    task.as_deref(),
                    &decisions,
                    &artifacts,
                    &blockers,
                    &next_actions,
                    idempotency_key.as_deref(),
                )?)
            }
            HandoffCommand::Snapshot {
                agent,
                token,
                after_sequence,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.handoff_snapshot(&agent, &token, after_sequence)?)
            }
            HandoffCommand::Propose {
                agent,
                token,
                task,
                item_limit,
            } => {
                let token = resolve_token(vault.as_ref(), &project_id, &agent, token)?;
                json!(bus.handoff_proposal(&agent, &token, &task, item_limit)?)
            }
        },
        Command::Skill(command) => match command {
            SkillCommand::List => json!(bus.list_skills()?),
            SkillCommand::Install { id, agent } => {
                let path = bus.install_skill(&id, &agent, &cli.root)?;
                json!({
                    "skill": id,
                    "agent": agent,
                    "installedAt": path.to_string_lossy(),
                })
            }
            SkillCommand::Register {
                id,
                name,
                version,
                file,
                agents,
                deps,
            } => {
                let content = std::fs::read_to_string(&file)?;
                let record = vibebus::SkillRecord {
                    id: id.clone(),
                    project_id: project_id.clone(),
                    version,
                    name,
                    description: None,
                    agents_supported: agents,
                    deps,
                    content,
                    created_at: 0,
                };
                bus.register_skill(&record)?;
                json!({"registered": id, "file": file.to_string_lossy()})
            }
        },
        Command::Plugin(command) => match command {
            PluginCommand::List => json!(bus.list_plugins()?),
        },
        Command::Status => json!({
            "project": bus.project(),
            "projectRoot": bus.project_root().to_string_lossy(),
            "databasePath": bus.database_path().to_string_lossy(),
            "agents": bus.list_agents()?,
            "tasks": bus.list_tasks()?,
            "threadBindings": bus.list_task_thread_bindings(None, true)?,
            "retention": bus.retention_state()?,
            "operator": bus.operator_status()?,
            "reservations": bus.list_active_reservations()?,
            "artifacts": bus.list_artifacts(None)?,
            "providers": bus.list_providers()?,
            "skills": bus.list_skills()?,
            "plugins": bus.list_plugins()?,
        }),
        Command::Doctor => json!(bus.doctor()?),
        Command::Backup { output } => json!(bus.backup_to(&output)?),
        Command::Maintenance { .. } => unreachable!(),
        Command::Hook { .. } => unreachable!(),
        Command::Mcp => unreachable!(),
    };
    Ok(json!({"ok": true, "result": result}))
}

/// Dispatch `vibebus api` subcommands. `project_id` is the already-resolved
/// project identifier used to key API credentials in the OS vault. Key values
/// never appear in the returned JSON (only presence/status is reported).
fn handle_api_command(
    bus: &mut vibebus::Bus,
    root: &std::path::Path,
    project_id: &str,
    api_vault: &dyn vibebus::ApiCredentialVault,
    command: ApiCommand,
) -> Result<serde_json::Value> {
    use vibebus::{
        ApiConfig, ApiFormat, ApiProvider, backup_config, detect_env_conflicts, health_check,
        list_backups, restore_config,
    };

    match command {
        ApiCommand::Provider {
            command:
                ApiProviderCommand::Add {
                    name,
                    base_url,
                    format,
                    model,
                    key,
                },
        } => {
            // Keys are stored in the OS vault; never accept them from a
            // non-interactive stdin without an explicit --key.
            let api_format = ApiFormat::parse(&format)?;
            let normalized = ApiProvider::normalize_base_url(&base_url)?;
            let mut config = ApiConfig::load(root)?;
            if config.providers.contains_key(&name) {
                return Err(BusError::Conflict(format!(
                    "api provider '{name}' already exists; delete and re-add to replace it"
                )));
            }
            let provider = ApiProvider {
                name: name.clone(),
                base_url: normalized,
                api_format,
                model,
                env_template: std::collections::HashMap::new(),
            };
            // Prompt for the key interactively unless an explicit one is given.
            let key = match key {
                Some(key) => key,
                None => {
                    if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                        rpassword::prompt_password(format!(
                            "API key for provider '{name}' (stored in the OS credential vault): "
                        ))?
                    } else {
                        return Err(BusError::Validation(
                            "no --key provided and stdin is not a terminal; pass the key explicitly".into(),
                        ));
                    }
                }
            };
            if key.trim().is_empty() {
                return Err(BusError::Validation("API key cannot be empty".into()));
            }
            api_vault.store_api(project_id, &name, key.trim())?;
            config.providers.insert(name.clone(), provider);
            config.save(root)?;
            Ok(json!({
                "added": name,
                "baseUrl": config.providers[&name].base_url,
                "format": config.providers[&name].api_format.as_str(),
                "model": config.providers[&name].model,
                "keyStored": true,
                "vaultBackend": api_vault.api_backend(),
            }))
        }
        ApiCommand::Provider {
            command: ApiProviderCommand::List,
        } => {
            let config = ApiConfig::load(root)?;
            let providers = config
                .providers
                .values()
                .map(|provider| {
                    serde_json::json!({
                        "name": provider.name,
                        "baseUrl": provider.base_url,
                        "format": provider.api_format.as_str(),
                        "model": provider.model,
                        "active": config.active.as_deref() == Some(&provider.name),
                    })
                })
                .collect::<Vec<_>>();
            Ok(json!({
                "active": config.active,
                "providers": providers,
                "keyStored": config.providers.keys().map(|name| {
                    (name.clone(), api_vault.load_api(project_id, name).unwrap_or(None).is_some())
                }).collect::<std::collections::HashMap<_, _>>(),
            }))
        }
        ApiCommand::Use {
            name,
            force,
            write_config,
        } => {
            let mut config = ApiConfig::load(root)?;
            if !config.providers.contains_key(&name) {
                return Err(BusError::Validation(format!(
                    "unknown api provider '{name}'; configured providers: {}",
                    if config.providers.is_empty() {
                        "(none)".to_string()
                    } else {
                        config
                            .providers
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                )));
            }
            // Resolve the full injection env to (a) require the key and (b)
            // detect system-env conflicts against the real variable set.
            let resolved =
                vibebus::resolve_api_env(&config, api_vault, project_id, Some(&name))?
                    .ok_or_else(|| BusError::Validation("provider did not resolve".into()))?;
            let conflicts = detect_env_conflicts(&resolved.vars);
            if !conflicts.is_empty() && !force {
                let detail = conflicts
                    .iter()
                    .map(|conflict| {
                        format!(
                            "{} (currently set to '{}')",
                            conflict.var,
                            conflict.current.clone().unwrap_or_default()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(BusError::Conflict(format!(
                    "system environment conflicts with injected vars: {detail}. Pass --force to override"
                )));
            }
            // Write-ahead backup before mutating the active provider.
            let backup = backup_config(root)?;
            config.active = Some(name.clone());
            config.save(root)?;
            // Regenerate the env export files for TUI launchers (key is a
            // vault-resolving command substitution, never a literal).
            vibebus::write_env_export(root, &config, true)?;
            // Audit: record activation with variable *names* only — never keys.
            let injected_names = resolved.vars.keys().cloned().collect::<Vec<_>>();
            bus.record_audit_event(
                "api.provider_activated",
                "api_provider",
                &name,
                json!({
                    "provider": name,
                    "injectedVars": injected_names,
                    "backup": backup.file_name().map(|n| n.to_string_lossy()),
                }),
            )?;
            // Channel 4 (opt-in): write into project-level Claude Code config.
            let mut config_backup: Option<PathBuf> = None;
            if write_config {
                config_backup = Some(vibebus::write_claude_config(root, &resolved.vars, &name)?);
            }
            Ok(json!({
                "active": name,
                "injectedVars": resolved.vars.keys().cloned().collect::<Vec<_>>(),
                "backup": backup.file_name().map(|n| n.to_string_lossy()),
                "envExport": ["env.sh", "env.ps1"],
                "configFile": config_backup.as_ref().map(|p| p.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()),
                "force": force,
            }))
        }
        ApiCommand::Unuse => {
            let mut config = ApiConfig::load(root)?;
            let previous = config.active.clone();
            // Revert the channel-4 config file if it was written.
            vibebus::revert_claude_config(root)?;
            let backup = backup_config(root)?;
            config.active = None;
            config.save(root)?;
            // Remove the export files so a stale launcher cannot inject a
            // provider that is no longer active.
            vibebus::remove_env_export(root)?;
            if let Some(previous) = &previous {
                bus.record_audit_event(
                    "api.provider_deactivated",
                    "api_provider",
                    previous,
                    json!({
                        "provider": previous,
                        "backup": backup.file_name().map(|n| n.to_string_lossy()),
                    }),
                )?;
            }
            Ok(json!({
                "active": Option::<String>::None,
                "previousActive": previous,
                "backup": backup.file_name().map(|n| n.to_string_lossy()),
            }))
        }
        ApiCommand::Restore { n } => {
            let restored = restore_config(root, n)?;
            let names = restored.providers.keys().cloned().collect::<Vec<_>>();
            // Keep the export files in sync with the restored active provider.
            let key_stored = restored
                .active
                .as_deref()
                .map(|name| {
                    api_vault
                        .load_api(project_id, name)
                        .unwrap_or(None)
                        .is_some()
                })
                .unwrap_or(false);
            vibebus::write_env_export(root, &restored, key_stored)?;
            bus.record_audit_event(
                "api.config_restored",
                "api_config",
                "api.json",
                json!({
                    "backupIndex": n,
                    "providers": names,
                    "active": restored.active,
                }),
            )?;
            Ok(json!({
                "restored": names,
                "active": restored.active,
                "remainingBackups": list_backups(root)?,
            }))
        }
        ApiCommand::Health {
            name,
            check_network,
        } => {
            let mut health = health_check(&ApiConfig::load(root)?, api_vault, project_id, &name)?;
            if check_network {
                let config = ApiConfig::load(root)?;
                let provider = config.providers.get(&name).ok_or_else(|| {
                    BusError::Validation(format!("unknown api provider '{name}'"))
                })?;
                let reachable = vibebus::check_provider_network(&provider.base_url);
                health["networkReachable"] = json!(reachable.is_ok());
                if let Err(error) = reachable {
                    health["networkError"] = json!(error.to_string());
                }
            }
            Ok(health)
        }
        ApiCommand::Run { command } => {
            let Some(first) = command.first() else {
                return Err(BusError::Validation(
                    "api run requires an agent CLI command (e.g. `-- codex exec`)".into(),
                ));
            };
            let config = ApiConfig::load(root)?;
            let resolved = vibebus::resolve_api_env(&config, api_vault, project_id, None)?
                .ok_or_else(|| {
                    BusError::Validation(
                        "no active api provider; run `vibebus api use --name <provider>` first"
                            .into(),
                    )
                })?;
            // Detect conflicts in the current shell too; unlike `use` we do not
            // refuse here because the caller explicitly asked to launch, but we
            // surface them in a warning on stderr.
            let conflicts = detect_env_conflicts(&resolved.vars);
            for conflict in &conflicts {
                eprintln!(
                    "warning: system env '{}' would override the injected value; unset it or the provider may not be used",
                    conflict.var
                );
            }
            // Replace this process with the target CLI under the injected env.
            // The key is resolved now (from the vault) and only lives in this
            // process's environment for the child's lifetime.
            let mut cmd = std::process::Command::new(first);
            cmd.args(&command[1..]);
            for (key, value) in &resolved.vars {
                cmd.env(key, value);
            }
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                // `exec` only returns on failure; on success the process image
                // is replaced, so this is the tail expression of the arm.
                Err(cmd.exec().into())
            }
            #[cfg(not(unix))]
            {
                let status = cmd.status()?;
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        ApiCommand::SyncProviders => {
            let config = ApiConfig::load(root)?;
            bus.sync_providers_from_api_json(root, &config)?;
            let records = bus.list_providers()?;
            Ok(json!({
                "synced": records.len(),
                "providers": records.iter().map(|r| json!({
                    "id": r.id,
                    "baseUrl": r.base_url,
                    "apiFormat": r.api_format,
                    "active": r.active,
                })).collect::<Vec<_>>(),
            }))
        }
        ApiCommand::Key {
            command: ApiKeyCommand::Get { name },
        } => {
            let key = api_vault.load_api(project_id, &name)?.ok_or_else(|| {
                BusError::CredentialVault(format!(
                    "api key for provider '{name}' is not stored in the OS credential vault"
                ))
            })?;
            // stdout only; never wrap in the JSON envelope so `source env.sh`
            // command substitution receives exactly the key bytes.
            println!("{key}");
            std::process::exit(0);
        }
    }
}

fn resolve_cli_secret(
    vault: &dyn CredentialVault,
    project_id: &str,
    agent: &str,
    argument: Option<String>,
) -> Result<vibebus::ResolvedSecret> {
    let environment = std::env::var("VIBEBUS_AGENT_TOKEN").ok();
    resolve_agent_token(
        vault,
        project_id,
        agent,
        argument.as_deref(),
        environment.as_deref(),
    )
}

fn resolve_token(
    vault: &dyn CredentialVault,
    project_id: &str,
    agent: &str,
    argument: Option<String>,
) -> Result<String> {
    Ok(resolve_cli_secret(vault, project_id, agent, argument)?.value)
}

fn print_json(value: &impl Serialize) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("serialize CLI response")
    );
}

fn require_interactive_confirmation(prompt: &str, expected: &str) -> Result<()> {
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return Err(BusError::Validation(
            "operator mutations require an interactive terminal and are unavailable through redirected input, MCP, or automation"
                .into(),
        ));
    }
    let mut stderr = io::stderr().lock();
    writeln!(stderr, "{prompt}:")?;
    write!(stderr, "> ")?;
    stderr.flush()?;
    drop(stderr);
    let mut confirmation = String::new();
    io::stdin().read_line(&mut confirmation)?;
    if confirmation.trim() != expected {
        return Err(BusError::Validation(
            "operator confirmation did not match; no mutation was performed".into(),
        ));
    }
    Ok(())
}

fn error_kind(error: &BusError) -> &'static str {
    match error {
        BusError::Io(_) => "io",
        BusError::Database(_) => "database",
        BusError::Json(_) => "json",
        BusError::CredentialVault(_) => "credential_vault",
        BusError::ProjectNotFound(_) => "project_not_found",
        BusError::AgentNotFound(_) => "agent_not_found",
        BusError::Unauthorized(_) => "unauthorized",
        BusError::OperatorUnauthorized => "operator_unauthorized",
        BusError::OperatorApprovalRequired(_) => "operator_approval_required",
        BusError::Conflict(_) => "conflict",
        BusError::Validation(_) => "validation",
    }
}
