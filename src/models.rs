use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMetadata {
    pub project_id: String,
    pub name: String,
    pub created_at: String,
    pub schema_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInit {
    pub project: ProjectMetadata,
    pub marker_path: String,
    pub database_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRegistration {
    pub agent_id: String,
    pub name: String,
    pub role: String,
    pub token: String,
    pub recovery_key: String,
    pub token_generation: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRecovery {
    pub agent_id: String,
    pub name: String,
    pub role: String,
    pub token: String,
    pub recovery_key: String,
    pub token_generation: i64,
    pub recovered_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryKeyView {
    pub agent_id: String,
    pub name: String,
    pub recovery_key: String,
    pub token_generation: i64,
    pub issued_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorCredentialView {
    pub operator_secret: String,
    pub generation: i64,
    pub issued_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorStatusView {
    pub configured: bool,
    pub generation: Option<i64>,
    pub configured_at: Option<i64>,
    pub rotated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentView {
    pub agent_id: String,
    pub name: String,
    pub role: String,
    pub status: String,
    pub last_seen_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageView {
    pub message_id: String,
    pub sender: String,
    pub recipient: String,
    pub thread_id: Option<String>,
    pub priority: String,
    pub subject: String,
    pub body: String,
    pub requires_ack: bool,
    pub created_at: i64,
    pub read_at: Option<i64>,
    pub ack_at: Option<i64>,
    pub closed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageReceipt {
    pub message_id: String,
    pub recipient: String,
    pub read_at: i64,
    pub ack_at: Option<i64>,
    pub closed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskView {
    pub task_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub owner: Option<String>,
    pub version: i64,
    pub blocked_reason: Option<String>,
    pub depends_on: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskThreadBindingView {
    pub binding_id: String,
    pub task_id: String,
    pub thread_id: String,
    pub agent: String,
    pub bound_at: i64,
    pub unbound_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReservationView {
    pub reservation_id: String,
    pub owner: String,
    pub task_id: Option<String>,
    pub path_pattern: String,
    pub exclusive: bool,
    pub reason: Option<String>,
    pub created_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseResult {
    pub reservation_id: String,
    pub released_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactView {
    pub artifact_id: String,
    pub publisher: String,
    pub task_id: Option<String>,
    pub kind: String,
    pub path: String,
    pub sha256: String,
    pub summary: String,
    pub metadata: serde_json::Value,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DecisionView {
    pub decision_id: String,
    pub key: String,
    pub task_id: String,
    pub author: String,
    pub summary: String,
    pub artifact_ids: Vec<String>,
    pub confirmed_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResponsibilityOverrideView {
    pub override_id: String,
    pub task_id: String,
    pub grantee: String,
    pub granted_by: String,
    pub path_pattern: String,
    pub reason: String,
    pub created_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponsibilityPolicyView {
    pub agent: String,
    pub role: String,
    pub configured: bool,
    pub source_path: String,
    pub policy_sha256: Option<String>,
    pub allowed_paths: Vec<String>,
    pub active_overrides: Vec<ResponsibilityOverrideView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitFactView {
    pub fact_id: String,
    pub task_id: String,
    pub author: String,
    pub commit_sha: String,
    pub summary: String,
    pub changed_paths: Vec<String>,
    pub recorded_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TestResultFactView {
    pub fact_id: String,
    pub task_id: String,
    pub author: String,
    pub result_key: String,
    pub suite: String,
    pub outcome: String,
    pub summary: String,
    pub command: Option<String>,
    pub report_artifact_id: Option<String>,
    pub recorded_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffProposalView {
    pub task: TaskView,
    pub summary: String,
    pub git_commits: Vec<GitCommitFactView>,
    pub test_results: Vec<TestResultFactView>,
    pub decisions: Vec<DecisionView>,
    pub artifacts: Vec<ArtifactView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextScopeView {
    pub owned_task_ids: Vec<String>,
    pub direct_dependency_task_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextItemView {
    pub cursor: String,
    pub kind: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSyncView {
    pub agent: String,
    pub scope: ContextScopeView,
    pub items: Vec<ContextItemView>,
    pub item_limit: usize,
    pub byte_budget: usize,
    pub item_count: usize,
    pub bytes_used: usize,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorReport {
    pub ok: bool,
    pub integrity: String,
    pub journal_mode: String,
    pub foreign_keys_enabled: bool,
    pub schema_version: i64,
    pub project_root: String,
    pub database_path: String,
    pub agents: i64,
    pub messages: i64,
    pub tasks: i64,
    pub active_reservations: i64,
    pub artifacts: i64,
    pub decisions: i64,
    pub responsibility_overrides: i64,
    pub git_commit_facts: i64,
    pub test_result_facts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupView {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventView {
    pub sequence: i64,
    pub event_id: String,
    pub actor: Option<String>,
    pub event_type: String,
    pub entity_type: String,
    pub entity_id: String,
    pub payload: serde_json::Value,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPolicy {
    pub event_max_age_days: i64,
    pub keep_recent_events: i64,
    pub idempotency_max_age_days: i64,
    pub closed_message_max_age_days: i64,
    pub terminal_binding_max_age_days: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RetentionCounts {
    pub events: i64,
    pub idempotency_records: i64,
    pub message_receipts: i64,
    pub messages: i64,
    pub task_thread_bindings: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionProtectionView {
    pub latest_event_sequence: i64,
    pub events_pruned_through_sequence: i64,
    pub safe_event_sequence: i64,
    pub event_prune_through_sequence: i64,
    pub subscription_count: i64,
    pub pending_delivery_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPlan {
    pub plan_id: String,
    pub generated_at: i64,
    pub policy: RetentionPolicy,
    pub protection: RetentionProtectionView,
    pub candidates: RetentionCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionReport {
    pub plan_id: String,
    #[serde(default)]
    pub operator_approval_id: Option<String>,
    pub applied_at: i64,
    pub deleted: RetentionCounts,
    pub events_pruned_through_sequence: i64,
    pub replayed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionApprovalView {
    pub approval_id: String,
    pub plan_id: String,
    pub operator_generation: i64,
    pub approved_at: i64,
    pub expires_at: i64,
    pub consumed_at: Option<i64>,
    pub consumed_by: Option<String>,
    pub plan: RetentionPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionStateView {
    pub events_pruned_through_sequence: i64,
    pub earliest_available_event_sequence: i64,
    pub last_applied_at: Option<i64>,
    pub last_plan_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionView {
    pub subscription_id: String,
    pub agent: String,
    pub name: String,
    pub event_types: Vec<String>,
    pub cursor_sequence: i64,
    pub pending_delivery: Option<SubscriptionDeliveryView>,
    pub last_acked_delivery_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDeliveryView {
    pub delivery_id: String,
    pub from_sequence: i64,
    pub through_sequence: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionPoll {
    pub subscription: SubscriptionView,
    pub events: Vec<EventView>,
    pub scanned_through_sequence: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionPeek {
    pub subscription: SubscriptionView,
    pub delivery: Option<SubscriptionDeliveryView>,
    pub events: Vec<EventView>,
    pub scanned_through_sequence: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionAck {
    pub subscription_id: String,
    pub agent: String,
    pub name: String,
    pub delivery_id: String,
    pub cursor_sequence: i64,
    pub acknowledged_at: i64,
    pub replayed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffSnapshot {
    pub agent: String,
    pub unread_messages: Vec<MessageView>,
    pub owned_tasks: Vec<TaskView>,
    pub active_reservations: Vec<ReservationView>,
    pub task_thread_bindings: Vec<TaskThreadBindingView>,
    pub recent_artifacts: Vec<ArtifactView>,
    pub recent_events: Vec<EventView>,
    pub latest_event_sequence: i64,
    pub retention_state: RetentionStateView,
}
