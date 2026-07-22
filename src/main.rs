use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use serde_json::json;
use vibebus::{
    Bus, BusError, CodexHook, CredentialVault, Result, RetentionPolicy, SecretSource,
    StoredOperatorCredential, discover_project, initialize_project, mcp::run_mcp,
    operator_credential_delivery, recovery_delivery, recovery_key_delivery, registration_delivery,
    resolve_agent_recovery_key, resolve_agent_token, resolve_operator_secret, run_codex_hook,
    system_credential_vault,
};

#[derive(Debug, Parser)]
#[command(
    name = "vibebus",
    version,
    about = "Local coordination bus for independent Codex tasks"
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
            HookCommand::SessionStart => CodexHook::SessionStart,
            HookCommand::PostToolUse => CodexHook::PostToolUse,
            HookCommand::Stop => CodexHook::Stop,
        };
        return match run_codex_hook(hook) {
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
            "artifacts": bus.list_artifacts(None)?
        }),
        Command::Doctor => json!(bus.doctor()?),
        Command::Backup { output } => json!(bus.backup_to(&output)?),
        Command::Maintenance { .. } => unreachable!(),
        Command::Hook { .. } => unreachable!(),
        Command::Mcp => unreachable!(),
    };
    Ok(json!({"ok": true, "result": result}))
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
