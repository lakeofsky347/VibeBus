use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use serde_json::json;
use vibebus::{Bus, BusError, Result, initialize_project, mcp::run_mcp};

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
    },
    Agents,
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
    Send(SendArgs),
    Inbox(AgentAuthArgs),
    Read(MessageAuthArgs),
    Ack(MessageAuthArgs),
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    Reserve {
        #[command(subcommand)]
        command: ReservationCommand,
    },
    Artifact {
        #[command(subcommand)]
        command: ArtifactCommand,
    },
    Event {
        #[command(subcommand)]
        command: EventCommand,
    },
    Subscription {
        #[command(subcommand)]
        command: SubscriptionCommand,
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
    Mcp,
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
        recovery_key: String,
    },
    ProvisionRecovery {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        token: Option<String>,
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
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let cli = Cli::parse();
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

    let mut bus = Bus::open(&cli.root, cli.data_home.as_deref())?;
    let result = match cli.command {
        Command::Init { .. } => unreachable!(),
        Command::Register { name, role } => json!(bus.register_agent(&name, &role)?),
        Command::Agents => json!(bus.list_agents()?),
        Command::Agent { command } => match command {
            AgentCommand::Recover { name, recovery_key } => {
                json!(bus.recover_agent(&name, &recovery_key)?)
            }
            AgentCommand::ProvisionRecovery { agent, token } => {
                let token = resolve_token(token)?;
                json!(bus.provision_recovery_key(&agent, &token)?)
            }
        },
        Command::Send(args) => {
            let token = resolve_token(args.token)?;
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
            let token = resolve_token(args.token)?;
            json!(bus.inbox(&args.agent, &token, !args.all)?)
        }
        Command::Read(args) => {
            let token = resolve_token(args.token)?;
            json!(bus.mark_read(&args.agent, &token, &args.message)?)
        }
        Command::Ack(args) => {
            let token = resolve_token(args.token)?;
            json!(bus.acknowledge_message(&args.agent, &token, &args.message)?)
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
                let token = resolve_token(token)?;
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
                let token = resolve_token(token)?;
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
                let token = resolve_token(token)?;
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
                let token = resolve_token(token)?;
                json!(bus.update_task(&agent, &token, &id, expected_version, "completed", None,)?)
            }
            TaskCommand::Show { id } => json!(bus.get_task(&id)?),
            TaskCommand::List => json!(bus.list_tasks()?),
        },
        Command::Reserve { command } => match command {
            ReservationCommand::Add {
                agent,
                token,
                path,
                ttl,
                exclusive,
                reason,
                idempotency_key,
            } => {
                let token = resolve_token(token)?;
                json!(bus.reserve_path_idempotent(
                    &agent,
                    &token,
                    &path,
                    ttl,
                    exclusive,
                    reason.as_deref(),
                    idempotency_key.as_deref(),
                )?)
            }
            ReservationCommand::Release { agent, token, id } => {
                let token = resolve_token(token)?;
                json!(bus.release_reservation(&agent, &token, &id)?)
            }
            ReservationCommand::Renew {
                agent,
                token,
                id,
                ttl,
                idempotency_key,
            } => {
                let token = resolve_token(token)?;
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
                let token = resolve_token(token)?;
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
                let token = resolve_token(token)?;
                json!(bus.create_subscription(
                    &agent,
                    &token,
                    &name,
                    &event_types,
                    from_sequence,
                )?)
            }
            SubscriptionCommand::List { agent, token } => {
                let token = resolve_token(token)?;
                json!(bus.list_subscriptions(&agent, &token)?)
            }
            SubscriptionCommand::Poll {
                agent,
                token,
                name,
                limit,
            } => {
                let token = resolve_token(token)?;
                json!(bus.poll_subscription(&agent, &token, &name, limit)?)
            }
            SubscriptionCommand::Peek {
                agent,
                token,
                name,
                limit,
            } => {
                let token = resolve_token(token)?;
                json!(bus.peek_subscription(&agent, &token, &name, limit)?)
            }
            SubscriptionCommand::Ack {
                agent,
                token,
                name,
                delivery,
            } => {
                let token = resolve_token(token)?;
                json!(bus.acknowledge_subscription(&agent, &token, &name, &delivery)?)
            }
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
                let token = resolve_token(token)?;
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
                let token = resolve_token(token)?;
                json!(bus.handoff_snapshot(&agent, &token, after_sequence)?)
            }
        },
        Command::Status => json!({
            "project": bus.project(),
            "projectRoot": bus.project_root().to_string_lossy(),
            "databasePath": bus.database_path().to_string_lossy(),
            "agents": bus.list_agents()?,
            "tasks": bus.list_tasks()?,
            "reservations": bus.list_active_reservations()?,
            "artifacts": bus.list_artifacts(None)?
        }),
        Command::Doctor => json!(bus.doctor()?),
        Command::Backup { output } => json!(bus.backup_to(&output)?),
        Command::Mcp => unreachable!(),
    };
    Ok(json!({"ok": true, "result": result}))
}

fn resolve_token(argument: Option<String>) -> Result<String> {
    argument
        .or_else(|| std::env::var("VIBEBUS_AGENT_TOKEN").ok())
        .filter(|token| !token.is_empty())
        .ok_or_else(|| {
            BusError::Validation(
                "agent token is required via --token or VIBEBUS_AGENT_TOKEN".into(),
            )
        })
}

fn print_json(value: &impl Serialize) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("serialize CLI response")
    );
}

fn error_kind(error: &BusError) -> &'static str {
    match error {
        BusError::Io(_) => "io",
        BusError::Database(_) => "database",
        BusError::Json(_) => "json",
        BusError::ProjectNotFound(_) => "project_not_found",
        BusError::AgentNotFound(_) => "agent_not_found",
        BusError::Unauthorized(_) => "unauthorized",
        BusError::Conflict(_) => "conflict",
        BusError::Validation(_) => "validation",
    }
}
