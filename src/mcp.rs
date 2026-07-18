use std::{path::PathBuf, sync::Arc};

use rmcp::{
    ErrorData, ServerHandler, ServiceExt,
    handler::server::wrapper::Parameters,
    model::{Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};

use crate::{
    Bus, BusError, CredentialVault, ResolvedSecret, RetentionPolicy, SecretSource,
    recovery_delivery, recovery_key_delivery, registration_delivery, resolve_agent_recovery_key,
    resolve_agent_token, system_credential_vault,
};

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
    #[schemars(
        description = "Store both secrets in the current-user credential vault and redact them from the response"
    )]
    pub store_credentials: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentRecoverRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub name: String,
    #[schemars(
        description = "One-time recovery key; optional when current credentials are stored in the vault"
    )]
    pub recovery_key: Option<String>,
    #[schemars(
        description = "Store the rotated secret pair in the current-user credential vault and redact it from the response"
    )]
    pub store_credentials: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryProvisionRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    #[schemars(
        description = "Bearer token; optional when stored in the current-user credential vault"
    )]
    pub token: Option<String>,
    #[schemars(
        description = "Store the updated secret pair in the current-user credential vault and redact it from the response"
    )]
    pub store_credentials: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CredentialRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CredentialDeleteRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    #[schemars(
        description = "Must be true to acknowledge removal of the local OS credential entry"
    )]
    pub confirm: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentAuthRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub from: String,
    pub token: Option<String>,
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
    pub token: Option<String>,
    pub unread_only: Option<bool>,
    pub include_closed: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: Option<String>,
    pub message_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskCreateRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: Option<String>,
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
    pub token: Option<String>,
    pub task_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskCompleteRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: Option<String>,
    pub task_id: String,
    pub expected_version: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskUpdateRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: Option<String>,
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
    pub token: Option<String>,
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
    pub token: Option<String>,
    #[schemars(description = "Project-relative file or directory path")]
    pub path: String,
    pub ttl_seconds: Option<i64>,
    pub exclusive: Option<bool>,
    pub reason: Option<String>,
    pub task_id: Option<String>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReservationReleaseRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: Option<String>,
    pub reservation_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReservationRenewRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: Option<String>,
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
    pub token: Option<String>,
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
pub struct DecisionConfirmRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: Option<String>,
    pub key: String,
    pub task_id: String,
    pub summary: String,
    pub artifact_ids: Option<Vec<String>>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResponsibilityInspectRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResponsibilityOverrideRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: Option<String>,
    pub task_id: String,
    pub grantee: String,
    pub path_pattern: String,
    pub reason: String,
    pub ttl_seconds: Option<i64>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitRecordRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: Option<String>,
    pub task_id: String,
    pub commit_sha: String,
    pub summary: String,
    pub changed_paths: Vec<String>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TestResultRecordRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: Option<String>,
    pub task_id: String,
    pub result_key: String,
    pub suite: String,
    pub outcome: String,
    pub summary: String,
    pub command: Option<String>,
    pub report_artifact_id: Option<String>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContextSyncRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: Option<String>,
    #[schemars(description = "Opaque continuation cursor returned by a previous context sync")]
    pub cursor: Option<String>,
    #[schemars(description = "Maximum projected facts to return; defaults to 100")]
    pub item_limit: Option<usize>,
    #[schemars(description = "Maximum serialized item bytes to return; defaults to 65536")]
    pub byte_budget: Option<usize>,
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
pub struct RetentionPlanRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: Option<String>,
    pub event_max_age_days: Option<i64>,
    pub keep_recent_events: Option<i64>,
    pub idempotency_max_age_days: Option<i64>,
    pub closed_message_max_age_days: Option<i64>,
    pub terminal_binding_max_age_days: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RetentionApplyRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: Option<String>,
    pub plan_id: String,
    pub event_max_age_days: Option<i64>,
    pub keep_recent_events: Option<i64>,
    pub idempotency_max_age_days: Option<i64>,
    pub closed_message_max_age_days: Option<i64>,
    pub terminal_binding_max_age_days: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionCreateRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: Option<String>,
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
    pub token: Option<String>,
    pub name: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionAckRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: Option<String>,
    pub name: String,
    pub delivery_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandoffSendRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub from: String,
    pub token: Option<String>,
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
    pub token: Option<String>,
    pub after_sequence: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandoffProposalRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    pub agent: String,
    pub token: Option<String>,
    pub task_id: String,
    pub item_limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupRequest {
    #[schemars(description = "Absolute path inside the VibeBus project")]
    pub root: Option<String>,
    #[schemars(description = "Absolute or caller-selected path for a new SQLite backup file")]
    pub output: String,
}

#[derive(Clone)]
pub struct VibeBusMcp {
    default_root: PathBuf,
    data_home: Option<PathBuf>,
    vault: Arc<dyn CredentialVault>,
}

impl VibeBusMcp {
    pub fn new(default_root: PathBuf, data_home: Option<PathBuf>) -> Self {
        Self {
            default_root,
            data_home,
            vault: system_credential_vault(),
        }
    }

    pub fn with_vault(
        default_root: PathBuf,
        data_home: Option<PathBuf>,
        vault: Arc<dyn CredentialVault>,
    ) -> Self {
        Self {
            default_root,
            data_home,
            vault,
        }
    }

    fn open(&self, root: Option<&str>) -> Result<Bus, ErrorData> {
        let root = root
            .map(PathBuf::from)
            .unwrap_or_else(|| self.default_root.clone());
        Bus::open(&root, self.data_home.as_deref()).map_err(bus_error)
    }

    fn token(
        &self,
        bus: &Bus,
        agent: &str,
        provided: Option<&str>,
    ) -> Result<ResolvedSecret, ErrorData> {
        resolve_agent_token(
            self.vault.as_ref(),
            &bus.project().project_id,
            agent,
            provided,
            None,
        )
        .map_err(bus_error)
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
            "retention": bus.retention_state().map_err(bus_error)?,
            "operator": bus.operator_status().map_err(bus_error)?,
            "reservations": bus.list_active_reservations().map_err(bus_error)?,
            "artifacts": bus.list_artifacts(None).map_err(bus_error)?
        }))
    }

    #[tool(
        description = "Register a VibeBus agent; optionally store its one-time secrets in the current-user credential vault"
    )]
    fn vibebus_register(
        &self,
        Parameters(request): Parameters<RegisterRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        let registration = bus
            .register_agent(&request.name, &request.role)
            .map_err(bus_error)?;
        json_text(&registration_delivery(
            self.vault.as_ref(),
            &bus.project().project_id,
            &registration,
            request.store_credentials.unwrap_or(false),
        ))
    }

    #[tool(
        description = "Recover an agent by rotating both its bearer token and one-time recovery key"
    )]
    fn vibebus_agent_recover(
        &self,
        Parameters(request): Parameters<AgentRecoverRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        let resolved = resolve_agent_recovery_key(
            self.vault.as_ref(),
            &bus.project().project_id,
            &request.name,
            request.recovery_key.as_deref(),
        )
        .map_err(bus_error)?;
        let store_credentials =
            request.store_credentials.unwrap_or(false) || resolved.source == SecretSource::Vault;
        let recovery = bus
            .recover_agent(&request.name, &resolved.value)
            .map_err(bus_error)?;
        json_text(&recovery_delivery(
            self.vault.as_ref(),
            &bus.project().project_id,
            &recovery,
            store_credentials,
        ))
    }

    #[tool(
        description = "Rotate or provision an agent recovery key using its current bearer token"
    )]
    fn vibebus_recovery_provision(
        &self,
        Parameters(request): Parameters<RecoveryProvisionRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        let resolved = self.token(&bus, &request.agent, request.token.as_deref())?;
        let store_credentials =
            request.store_credentials.unwrap_or(false) || resolved.source == SecretSource::Vault;
        let recovery = bus
            .provision_recovery_key(&request.agent, &resolved.value)
            .map_err(bus_error)?;
        json_text(&recovery_key_delivery(
            self.vault.as_ref(),
            &bus.project().project_id,
            &resolved.value,
            &recovery,
            store_credentials,
        ))
    }

    #[tool(
        description = "Inspect whether an agent credential pair is stored in the current-user credential vault"
    )]
    fn vibebus_credential_status(
        &self,
        Parameters(request): Parameters<CredentialRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        json_text(
            &self
                .vault
                .status(&bus.project().project_id, &request.agent)
                .map_err(bus_error)?,
        )
    }

    #[tool(description = "Delete an agent credential pair from the current-user credential vault")]
    fn vibebus_credential_delete(
        &self,
        Parameters(request): Parameters<CredentialDeleteRequest>,
    ) -> Result<String, ErrorData> {
        if !request.confirm {
            return Err(bus_error(BusError::Validation(
                "credential deletion requires confirm=true".into(),
            )));
        }
        let bus = self.open(request.root.as_deref())?;
        let deleted = self
            .vault
            .delete(&bus.project().project_id, &request.agent)
            .map_err(bus_error)?;
        json_text(&serde_json::json!({
            "deleted": deleted,
            "credentials": self.vault.status(&bus.project().project_id, &request.agent).map_err(bus_error)?
        }))
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
                &self
                    .token(&bus, &request.from, request.token.as_deref())?
                    .value,
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
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
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
            &bus.mark_read(
                &request.agent,
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
                &request.message_id,
            )
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
            &bus.acknowledge_message(
                &request.agent,
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
                &request.message_id,
            )
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
            &bus.close_message(
                &request.agent,
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
                &request.message_id,
            )
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
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
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
            &bus.claim_task(
                &request.agent,
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
                &request.task_id,
            )
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
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
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
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
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
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
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
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
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
            &bus.reserve_path_for_task_idempotent(
                &request.agent,
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
                &request.path,
                request.ttl_seconds.unwrap_or(3600),
                request.exclusive.unwrap_or(true),
                request.reason.as_deref(),
                request.task_id.as_deref(),
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
            &bus.release_reservation(
                &request.agent,
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
                &request.reservation_id,
            )
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
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
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
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
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

    #[tool(description = "Confirm one immutable, task-scoped decision using a stable semantic key")]
    fn vibebus_decision_confirm(
        &self,
        Parameters(request): Parameters<DecisionConfirmRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.confirm_decision_idempotent(
                &request.agent,
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
                &request.key,
                &request.task_id,
                &request.summary,
                request.artifact_ids.as_deref().unwrap_or(&[]),
                request.idempotency_key.as_deref(),
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(
        description = "Inspect an Agent's configured responsibility paths and active task overrides"
    )]
    fn vibebus_responsibility_inspect(
        &self,
        Parameters(request): Parameters<ResponsibilityInspectRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.inspect_responsibility_policy(
                &request.agent,
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(description = "Grant an authenticated, expiring, task-scoped responsibility override")]
    fn vibebus_responsibility_override(
        &self,
        Parameters(request): Parameters<ResponsibilityOverrideRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.grant_responsibility_override_idempotent(
                &request.agent,
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
                &request.task_id,
                &request.grantee,
                &request.path_pattern,
                &request.reason,
                request.ttl_seconds.unwrap_or(3600),
                request.idempotency_key.as_deref(),
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(description = "Record one bounded immutable Git commit fact for an owned task")]
    fn vibebus_git_commit_record(
        &self,
        Parameters(request): Parameters<GitCommitRecordRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.record_git_commit_idempotent(
                &request.agent,
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
                &request.task_id,
                &request.commit_sha,
                &request.summary,
                &request.changed_paths,
                request.idempotency_key.as_deref(),
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(description = "Record one bounded immutable test-result fact for an owned task")]
    fn vibebus_test_result_record(
        &self,
        Parameters(request): Parameters<TestResultRecordRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.record_test_result_idempotent(
                &request.agent,
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
                &request.task_id,
                &request.result_key,
                &request.suite,
                &request.outcome,
                &request.summary,
                request.command.as_deref(),
                request.report_artifact_id.as_deref(),
                request.idempotency_key.as_deref(),
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(
        description = "Return a deterministic, budgeted Agent context projection with stable continuation"
    )]
    fn vibebus_context_sync(
        &self,
        Parameters(request): Parameters<ContextSyncRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.context_sync(
                &request.agent,
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
                request.cursor.as_deref(),
                request.item_limit.unwrap_or(100),
                request.byte_budget.unwrap_or(65_536),
            )
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

    #[tool(
        description = "Preview bounded retention candidates without deleting data; returns a confirmation plan ID"
    )]
    fn vibebus_retention_plan(
        &self,
        Parameters(request): Parameters<RetentionPlanRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        let policy = retention_policy(
            request.event_max_age_days,
            request.keep_recent_events,
            request.idempotency_max_age_days,
            request.closed_message_max_age_days,
            request.terminal_binding_max_age_days,
        );
        json_text(
            &bus.plan_retention(
                &request.agent,
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
                &policy,
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(
        description = "Apply an unchanged retention plan after a separate interactive CLI operator approval; stale confirmations conflict and successful retries are replay-safe"
    )]
    fn vibebus_retention_apply(
        &self,
        Parameters(request): Parameters<RetentionApplyRequest>,
    ) -> Result<String, ErrorData> {
        let mut bus = self.open(request.root.as_deref())?;
        let policy = retention_policy(
            request.event_max_age_days,
            request.keep_recent_events,
            request.idempotency_max_age_days,
            request.closed_message_max_age_days,
            request.terminal_binding_max_age_days,
        );
        json_text(
            &bus.apply_retention(
                &request.agent,
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
                &policy,
                &request.plan_id,
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(description = "Inspect the retained event-history floor and most recent cleanup run")]
    fn vibebus_retention_status(
        &self,
        Parameters(request): Parameters<RootRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        json_text(&bus.retention_state().map_err(bus_error)?)
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
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
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
            &bus.list_subscriptions(
                &request.agent,
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
            )
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
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
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
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
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
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
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
                &self
                    .token(&bus, &request.from, request.token.as_deref())?
                    .value,
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
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
                request.after_sequence.unwrap_or(0),
            )
            .map_err(bus_error)?,
        )
    }

    #[tool(
        description = "Build a bounded read-only handoff proposal from task facts without sending a message"
    )]
    fn vibebus_handoff_propose(
        &self,
        Parameters(request): Parameters<HandoffProposalRequest>,
    ) -> Result<String, ErrorData> {
        let bus = self.open(request.root.as_deref())?;
        json_text(
            &bus.handoff_proposal(
                &request.agent,
                &self
                    .token(&bus, &request.agent, request.token.as_deref())?
                    .value,
                &request.task_id,
                request.item_limit.unwrap_or(10),
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

fn retention_policy(
    event_max_age_days: Option<i64>,
    keep_recent_events: Option<i64>,
    idempotency_max_age_days: Option<i64>,
    closed_message_max_age_days: Option<i64>,
    terminal_binding_max_age_days: Option<i64>,
) -> RetentionPolicy {
    let defaults = RetentionPolicy::default();
    RetentionPolicy {
        event_max_age_days: event_max_age_days.unwrap_or(defaults.event_max_age_days),
        keep_recent_events: keep_recent_events.unwrap_or(defaults.keep_recent_events),
        idempotency_max_age_days: idempotency_max_age_days
            .unwrap_or(defaults.idempotency_max_age_days),
        closed_message_max_age_days: closed_message_max_age_days
            .unwrap_or(defaults.closed_message_max_age_days),
        terminal_binding_max_age_days: terminal_binding_max_age_days
            .unwrap_or(defaults.terminal_binding_max_age_days),
    }
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
        | BusError::OperatorUnauthorized
        | BusError::OperatorApprovalRequired(_)
        | BusError::AgentNotFound(_)
        | BusError::ProjectNotFound(_) => ErrorData::invalid_params(message, None),
        BusError::Io(_)
        | BusError::Database(_)
        | BusError::Json(_)
        | BusError::CredentialVault(_) => ErrorData::internal_error(message, None),
    }
}
