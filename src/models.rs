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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageReceipt {
    pub message_id: String,
    pub recipient: String,
    pub read_at: i64,
    pub ack_at: Option<i64>,
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
pub struct ReservationView {
    pub reservation_id: String,
    pub owner: String,
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
    pub recent_artifacts: Vec<ArtifactView>,
    pub recent_events: Vec<EventView>,
    pub latest_event_sequence: i64,
}
