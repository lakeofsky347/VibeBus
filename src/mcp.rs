use std::path::PathBuf;

use rmcp::{
    ErrorData, ServerHandler, ServiceExt,
    handler::server::wrapper::Parameters,
    model::{Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};

use crate::{Bus, BusError};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RootRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub name: String,
    pub role: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentRecoverRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub name: String,
    pub recovery_key: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentAuthRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub from: String,
    pub token: String,
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
    pub thread_id: Option<String>,
    #[schemars(description = "One of low, normal, high, urgent")]
    pub priority: Option<String>,
    pub requires_ack: Option<bool>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InboxRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: String,
    pub unread_only: Option<bool>,
    pub include_closed: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: String,
    pub message_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskCreateRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: String,
    pub task_id: String,
    pub title: String,
    pub description: Option<String>,
    pub depends_on: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskClaimRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: String,
    pub task_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskCompleteRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: String,
    pub task_id: String,
    pub expected_version: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskUpdateRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: String,
    pub task_id: String,
    pub expected_version: i64,
    pub status: String,
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskShowRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub task_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ThreadBindingRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: String,
    pub task_id: String,
    pub thread_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ThreadListRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub task_id: Option<String>,
    pub active_only: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReservationAddRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: String,
    #[schemars(description = "Project-relative file or directory path")]
    pub path: String,
    pub ttl_seconds: Option<i64>,
    pub exclusive: Option<bool>,
    pub reason: Option<String>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReservationReleaseRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: String,
    pub reservation_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReservationRenewRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: String,
    pub reservation_id: String,
    pub ttl_seconds: Option<i64>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactPublishRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: String,
    pub kind: String,
    #[schemars(description = "Project-relative path to an existing file")]
    pub path: String,
    pub summary: String,
    pub task_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactListRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub task_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EventListRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub after_sequence: Option<i64>,
    pub limit: Option<usize>,
    pub event_types: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionCreateRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: String,
    pub name: String,
    pub event_types: Option<Vec<String>>,
    pub from_sequence: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionPollRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: String,
    pub name: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionAckRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: String,
    pub name: String,
    pub delivery_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandoffSendRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub from: String,
    pub token: String,
    pub to: Vec<String>,
    pub summary: String,
    pub task_id: Option<String>,
    pub decisions: Option<Vec<String>>,
    pub artifacts: Option<Vec<String>>,
    pub blockers: Option<Vec<String>>,
    pub next_actions: Option<Vec<String>>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandoffSnapshotRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: String,
    pub after_sequence: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    #[schemars(description = "Absolute or caller-selected path for a new SQLite backup file")]
    pub output: String,
}

#[derive(Debug, Clone)]
pub struct VibeBusMcp {
    default_root: PathBuf,
    data_home: Option<PathBuf>,
}

impl VibeBusMcp {
    pub fn new(default_root: PathBuf, data_home: Option<PathBuf>) -> Self {
        Self {
            default_root,
            data_home,
        }
    }

    fn open(&self, root: Option<&str>) -> Result<Bus, ErrorData> {
        let root = root
            .map(PathBuf::from)
            .unwrap_or_else(|| self.default_root.clone());
        Bus::open(&root, self.data_home.as_deref()).map_err(bus_error)
    }
}

#[tool_router]
impl VibeBusMcp {
    #[tool(
        description = "Inspect VibeBus project identity, agents, tasks, and active reservations"
    )]
    fn vibebus_status(
        &self,
        Parameters(request): Parameters<RootRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        json_text(&serde_json::json!({
            "project": bus.project(),
            "projectRoot": bus.project_root().to_string_lossy(),
            "databasePath": bus.database_path().to_string_lossy(),
            "agents": bus.list_agents().map_err(bus_error)?,
            "tasks": bus.list_tasks().map_err(bus_error)?,
            "threadBindings": bus.list_task_thread_bindings(None, true).map_err(bus_error)?,
            "reservations": bus.list_active_reservations().map_err(bus_error)?,
            "artifacts": bus.list_artifacts(None).map_err(bus_error)?
        }))
    }

    #[tool(description = "Register a VibeBus agent and return its one-time bearer token")]
    fn vibebus_register(
        &self,
        Parameters(request): Parameters<RegisterRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.register_agent(&request.name, &request.role)
                .map_err(bus_error)?,
        )
    }

    #[tool(
        description = "Recover an agent by rotating both its bearer token and one-time recovery key"
    )]
    fn vibebus_agent_recover(
        &self,
        Parameters(request): Parameters<AgentRecoverRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.recover_agent(&request.name, &request.recovery_key)
                .map_err(bus_error)?,
        )
    }

    #[tool(
        description = "Rotate or provision an agent recovery key using its current bearer token"
    )]
    fn vibebus_recovery_provision(
        &self,
        Parameters(request): Parameters<AgentAuthRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.provision_recovery_key(&request.agent, &request.token)
                .map_err(bus_error)?,
        )
    }

    #[tool(description = "List agents registered in a VibeBus project")]
    fn vibebus_agents(
        &self,
        Parameters(request): Parameters<RootRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        json_text(&bus.list_agents().map_err(bus_error)?)
    }

    #[tool(description = "Send a directed structured message to one or more VibeBus agents")]
    fn vibebus_send(
        &self,
        Parameters(request): Parameters<SendRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.send_message_idempotent(
                &request.from,
                &request.token,
                &request.to,
                &request.subject,
                &request.body,
                request.thread_id.as_deref(),
                request.priority.as_deref().unwrap_or("normal"),
                request.requires_ack.unwrap_or(false),
                request.idempotency_key.as_deref(),
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(
        description = "Read an authenticated agent inbox; messages for other agents are never returned"
    )]
    fn vibebus_inbox(
        &self,
        Parameters(request): Parameters<InboxRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.inbox_with_options(
                &request.agent,
                &request.token,
                request.unread_only.unwrap_or(true),
                request.include_closed.unwrap_or(false),
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(description = "Mark one inbox message as read")]
    fn vibebus_read(
        &self,
        Parameters(request): Parameters<ReceiptRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.mark_read(&request.agent, &request.token, &request.message_id)
                .map_err(bus_error)?,
        )
    }

    #[tool(description = "Acknowledge one inbox message and record its read time")]
    fn vibebus_ack(
        &self,
        Parameters(request): Parameters<ReceiptRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.acknowledge_message(&request.agent, &request.token, &request.message_id)
                .map_err(bus_error)?,
        )
    }

    #[tool(
        description = "Close one inbox message; acknowledgement-required messages must be acknowledged first"
    )]
    fn vibebus_close(
        &self,
        Parameters(request): Parameters<ReceiptRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.close_message(&request.agent, &request.token, &request.message_id)
                .map_err(bus_error)?,
        )
    }

    #[tool(description = "Create a versioned task with optional dependency task IDs")]
    fn vibebus_task_create(
        &self,
        Parameters(request): Parameters<TaskCreateRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.create_task(
                &request.agent,
                &request.token,
                &request.task_id,
                &request.title,
                request.description.as_deref(),
                request.depends_on.as_deref().unwrap_or(&[]),
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(
        description = "Atomically claim a ready VibeBus task; concurrent claimers cannot both win"
    )]
    fn vibebus_task_claim(
        &self,
        Parameters(request): Parameters<TaskClaimRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.claim_task(&request.agent, &request.token, &request.task_id)
                .map_err(bus_error)?,
        )
    }

    #[tool(description = "Update a task using optimistic version checking")]
    fn vibebus_task_update(
        &self,
        Parameters(request): Parameters<TaskUpdateRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.update_task(
                &request.agent,
                &request.token,
                &request.task_id,
                request.expected_version,
                &request.status,
                request.blocked_reason.as_deref(),
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(description = "Complete a task using optimistic version checking")]
    fn vibebus_task_complete(
        &self,
        Parameters(request): Parameters<TaskCompleteRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.update_task(
                &request.agent,
                &request.token,
                &request.task_id,
                request.expected_version,
                "completed",
                None,
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(description = "Show one VibeBus task and its dependencies")]
    fn vibebus_task_show(
        &self,
        Parameters(request): Parameters<TaskShowRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        json_text(&bus.get_task(&request.task_id).map_err(bus_error)?)
    }

    #[tool(description = "List all tasks in a VibeBus project")]
    fn vibebus_task_list(
        &self,
        Parameters(request): Parameters<RootRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        json_text(&bus.list_tasks().map_err(bus_error)?)
    }

    #[tool(description = "Bind an owned active VibeBus task to one Codex thread identifier")]
    fn vibebus_thread_bind(
        &self,
        Parameters(request): Parameters<ThreadBindingRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.bind_task_thread(
                &request.agent,
                &request.token,
                &request.task_id,
                &request.thread_id,
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(description = "Unbind an owned VibeBus task from its active Codex thread identifier")]
    fn vibebus_thread_unbind(
        &self,
        Parameters(request): Parameters<ThreadBindingRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.unbind_task_thread(
                &request.agent,
                &request.token,
                &request.task_id,
                &request.thread_id,
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(description = "List task-to-Codex-thread bindings, optionally including history")]
    fn vibebus_thread_list(
        &self,
        Parameters(request): Parameters<ThreadListRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.list_task_thread_bindings(
                request.task_id.as_deref(),
                request.active_only.unwrap_or(true),
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(description = "Acquire a TTL-backed project-relative file or directory reservation")]
    fn vibebus_reserve(
        &self,
        Parameters(request): Parameters<ReservationAddRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.reserve_path_idempotent(
                &request.agent,
                &request.token,
                &request.path,
                request.ttl_seconds.unwrap_or(3600),
                request.exclusive.unwrap_or(true),
                request.reason.as_deref(),
                request.idempotency_key.as_deref(),
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(description = "Release an active reservation owned by the authenticated agent")]
    fn vibebus_release(
        &self,
        Parameters(request): Parameters<ReservationReleaseRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.release_reservation(&request.agent, &request.token, &request.reservation_id)
                .map_err(bus_error)?,
        )
    }

    #[tool(description = "Extend an active reservation owned by the authenticated agent")]
    fn vibebus_reservation_renew(
        &self,
        Parameters(request): Parameters<ReservationRenewRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.renew_reservation_idempotent(
                &request.agent,
                &request.token,
                &request.reservation_id,
                request.ttl_seconds.unwrap_or(3600),
                request.idempotency_key.as_deref(),
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(description = "List active unexpired file reservations")]
    fn vibebus_reservations(
        &self,
        Parameters(request): Parameters<RootRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        json_text(&bus.list_active_reservations().map_err(bus_error)?)
    }

    #[tool(description = "Publish an existing project file as a hashed VibeBus artifact")]
    fn vibebus_artifact_publish(
        &self,
        Parameters(request): Parameters<ArtifactPublishRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.publish_artifact_idempotent(
                &request.agent,
                &request.token,
                &request.kind,
                &request.path,
                &request.summary,
                request.task_id.as_deref(),
                request.metadata.as_ref(),
                request.idempotency_key.as_deref(),
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(description = "List published VibeBus artifacts, optionally filtered by task ID")]
    fn vibebus_artifact_list(
        &self,
        Parameters(request): Parameters<ArtifactListRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.list_artifacts(request.task_id.as_deref())
                .map_err(bus_error)?,
        )
    }

    #[tool(description = "List ordered project events after a durable sequence cursor")]
    fn vibebus_events(
        &self,
        Parameters(request): Parameters<EventListRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.list_events(
                request.after_sequence.unwrap_or(0),
                request.limit.unwrap_or(100),
                request.event_types.as_deref().unwrap_or(&[]),
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(description = "Create a named authenticated event subscription with a durable cursor")]
    fn vibebus_subscription_create(
        &self,
        Parameters(request): Parameters<SubscriptionCreateRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.create_subscription(
                &request.agent,
                &request.token,
                &request.name,
                request.event_types.as_deref().unwrap_or(&[]),
                request.from_sequence,
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(description = "List named event subscriptions owned by an authenticated agent")]
    fn vibebus_subscription_list(
        &self,
        Parameters(request): Parameters<AgentAuthRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.list_subscriptions(&request.agent, &request.token)
                .map_err(bus_error)?,
        )
    }

    #[tool(
        description = "Poll a named event subscription and atomically advance its durable cursor"
    )]
    fn vibebus_subscription_poll(
        &self,
        Parameters(request): Parameters<SubscriptionPollRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.poll_subscription(
                &request.agent,
                &request.token,
                &request.name,
                request.limit.unwrap_or(100),
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(
        description = "Peek a replay-safe subscription delivery without advancing its committed cursor"
    )]
    fn vibebus_subscription_peek(
        &self,
        Parameters(request): Parameters<SubscriptionPollRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.peek_subscription(
                &request.agent,
                &request.token,
                &request.name,
                request.limit.unwrap_or(100),
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(
        description = "Acknowledge a replay-safe subscription delivery and advance its committed cursor"
    )]
    fn vibebus_subscription_ack(
        &self,
        Parameters(request): Parameters<SubscriptionAckRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.acknowledge_subscription(
                &request.agent,
                &request.token,
                &request.name,
                &request.delivery_id,
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(
        description = "Send a high-priority structured handoff that requires recipient acknowledgement"
    )]
    fn vibebus_handoff_send(
        &self,
        Parameters(request): Parameters<HandoffSendRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.send_handoff(
                &request.from,
                &request.token,
                &request.to,
                &request.summary,
                request.task_id.as_deref(),
                request.decisions.as_deref().unwrap_or(&[]),
                request.artifacts.as_deref().unwrap_or(&[]),
                request.blockers.as_deref().unwrap_or(&[]),
                request.next_actions.as_deref().unwrap_or(&[]),
                request.idempotency_key.as_deref(),
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(
        description = "Build an authenticated resume snapshot of unread messages, owned work, reservations, artifacts, and recent events"
    )]
    fn vibebus_handoff_snapshot(
        &self,
        Parameters(request): Parameters<HandoffSnapshotRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.handoff_snapshot(
                &request.agent,
                &request.token,
                request.after_sequence.unwrap_or(0),
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(description = "Run SQLite integrity, WAL, foreign-key, schema, and entity-count checks")]
    fn vibebus_doctor(
        &self,
        Parameters(request): Parameters<RootRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        json_text(&bus.doctor().map_err(bus_error)?)
    }

    #[tool(description = "Create a consistent SQLite backup at a new destination path")]
    fn vibebus_backup(
        &self,
        Parameters(request): Parameters<BackupRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.backup_to(std::path::Path::new(&request.output))
                .map_err(bus_error)?,
        )
    }
}

#[tool_handler]
impl ServerHandler for VibeBusMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("vibebus", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Coordinate independent Codex tasks through a project-scoped SQLite bus. Pass the absolute project root to every tool when using the bundled plugin.",
            )
    }
}

pub async fn run_mcp(
    default_root: PathBuf,
    data_home: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let service = VibeBusMcp::new(default_root, data_home)
        .serve(rmcp::transport::stdio())
        .await?;
    service.waiting().await?;
    Ok(())
}

fn json_text<T: Serialize>(value: &T) -> Result<String, ErrorData> {
    serde_json::to_string_pretty(value)
        .map_err(|error| ErrorData::internal_error(error.to_string(), None))
}

fn bus_error(error: BusError) -> ErrorData {
    let message = error.to_string();
    match error {
        BusError::Validation(_)
        | BusError::Conflict(_)
        | BusError::Unauthorized(_)
        | BusError::AgentNotFound(_)
        | BusError::ProjectNotFound(_) => ErrorData::invalid_params(message, None),
        BusError::Io(_) | BusError::Database(_) | BusError::Json(_) => {
            ErrorData::internal_error(message, None)
        }
    }
}
