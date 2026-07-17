use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use rusqlite::{
    Connection, OptionalExtension, Transaction, TransactionBehavior, params, params_from_iter,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{BusError, Result};
use crate::models::{
    AgentRecovery, AgentRegistration, AgentView, ArtifactView, BackupView, DoctorReport, EventView,
    HandoffSnapshot, MessageReceipt, MessageView, ProjectMetadata, RecoveryKeyView, ReleaseResult,
    ReservationView, SubscriptionPoll, SubscriptionView, TaskView,
};
use crate::project::{database_path, discover_project};

const SCHEMA_VERSION: i64 = 5;
const TASK_STATUSES: &[&str] = &[
    "pending",
    "ready",
    "claimed",
    "working",
    "review",
    "blocked",
    "completed",
    "abandoned",
];

#[derive(Debug, Clone)]
struct AuthenticatedAgent {
    id: String,
    name: String,
}

struct TaskRecord {
    task_id: String,
    title: String,
    description: Option<String>,
    status: String,
    owner: Option<String>,
    version: i64,
    blocked_reason: Option<String>,
    created_at: i64,
    updated_at: i64,
}

pub struct Bus {
    conn: Connection,
    project_root: PathBuf,
    project: ProjectMetadata,
    database_path: PathBuf,
}

impl Bus {
    pub fn open(start: &Path, data_home: Option<&Path>) -> Result<Self> {
        let (project_root, project) = discover_project(start)?;
        let database_path = database_path(&project.project_id, data_home)?;
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&database_path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA foreign_keys=ON;
             PRAGMA busy_timeout=5000;",
        )?;

        let mut bus = Self {
            conn,
            project_root,
            project,
            database_path,
        };
        bus.migrate()?;
        Ok(bus)
    }

    pub fn project(&self) -> &ProjectMetadata {
        &self.project
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    pub fn register_agent(&mut self, name: &str, role: &str) -> Result<AgentRegistration> {
        validate_identifier("agent name", name)?;
        if role.trim().is_empty() {
            return Err(BusError::Validation("agent role cannot be empty".into()));
        }

        let now = now_ms();
        let agent_id = format!("agt_{}", Uuid::new_v4().simple());
        let token = generate_secret("vbt");
        let recovery_key = generate_secret("vbr");
        let token_hash = hash_secret(&token);
        let recovery_hash = hash_secret(&recovery_key);
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM agents WHERE project_id = ?1 AND name = ?2)",
            params![self.project.project_id, name],
            |row| row.get(0),
        )?;
        if exists {
            return Err(BusError::Conflict(format!(
                "agent name '{name}' is already registered"
            )));
        }

        tx.execute(
            "INSERT INTO agents
             (id, project_id, name, role, token_hash, recovery_hash, token_generation,
              status, created_at, last_seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, 'working', ?7, ?7)",
            params![
                agent_id,
                self.project.project_id,
                name,
                role.trim(),
                token_hash,
                recovery_hash,
                now
            ],
        )?;
        append_event(
            &tx,
            &self.project.project_id,
            Some(&agent_id),
            "agent_registered",
            "agent",
            &agent_id,
            json!({"name": name, "role": role.trim()}),
        )?;
        tx.commit()?;

        Ok(AgentRegistration {
            agent_id,
            name: name.to_owned(),
            role: role.trim().to_owned(),
            token,
            recovery_key,
            token_generation: 1,
            created_at: now,
        })
    }

    pub fn recover_agent(&mut self, name: &str, recovery_key: &str) -> Result<AgentRecovery> {
        validate_identifier("agent name", name)?;
        if recovery_key.is_empty() {
            return Err(BusError::Validation("recovery key cannot be empty".into()));
        }
        let now = now_ms();
        let token = generate_secret("vbt");
        let next_recovery_key = generate_secret("vbr");
        let token_hash = hash_secret(&token);
        let next_recovery_hash = hash_secret(&next_recovery_key);
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: Option<(String, String, Option<String>, i64)> = tx
            .query_row(
                "SELECT id, role, recovery_hash, token_generation FROM agents
                 WHERE project_id = ?1 AND name = ?2",
                params![self.project.project_id, name],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        let (agent_id, role, recovery_hash, token_generation) =
            current.ok_or_else(|| BusError::AgentNotFound(name.to_owned()))?;
        let recovery_hash = recovery_hash.ok_or_else(|| {
            BusError::Validation(format!(
                "agent '{name}' has no recovery key; provision one with the current token"
            ))
        })?;
        if !secret_matches(recovery_key, &recovery_hash) {
            return Err(BusError::Unauthorized(format!(
                "invalid recovery key for agent '{name}'"
            )));
        }
        let next_generation = token_generation + 1;
        let changed = tx.execute(
            "UPDATE agents SET token_hash = ?1, recovery_hash = ?2, token_generation = ?3,
                               status = 'working', last_seen_at = ?4
             WHERE project_id = ?5 AND id = ?6 AND token_generation = ?7",
            params![
                token_hash,
                next_recovery_hash,
                next_generation,
                now,
                self.project.project_id,
                agent_id,
                token_generation
            ],
        )?;
        if changed != 1 {
            return Err(BusError::Conflict(format!(
                "agent '{name}' credentials changed concurrently"
            )));
        }
        append_event(
            &tx,
            &self.project.project_id,
            Some(&agent_id),
            "agent_recovered",
            "agent",
            &agent_id,
            json!({"tokenGeneration": next_generation}),
        )?;
        tx.commit()?;
        Ok(AgentRecovery {
            agent_id,
            name: name.to_owned(),
            role,
            token,
            recovery_key: next_recovery_key,
            token_generation: next_generation,
            recovered_at: now,
        })
    }

    pub fn provision_recovery_key(&mut self, agent: &str, token: &str) -> Result<RecoveryKeyView> {
        let actor = self.authenticate(agent, token)?;
        let now = now_ms();
        let recovery_key = generate_secret("vbr");
        let recovery_hash = hash_secret(&recovery_key);
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let token_generation: i64 = tx.query_row(
            "SELECT token_generation FROM agents WHERE project_id = ?1 AND id = ?2",
            params![self.project.project_id, actor.id],
            |row| row.get(0),
        )?;
        tx.execute(
            "UPDATE agents SET recovery_hash = ?1, last_seen_at = ?2
             WHERE project_id = ?3 AND id = ?4",
            params![recovery_hash, now, self.project.project_id, actor.id],
        )?;
        append_event(
            &tx,
            &self.project.project_id,
            Some(&actor.id),
            "agent_recovery_key_rotated",
            "agent",
            &actor.id,
            json!({"tokenGeneration": token_generation}),
        )?;
        tx.commit()?;
        Ok(RecoveryKeyView {
            agent_id: actor.id,
            name: actor.name,
            recovery_key,
            token_generation,
            issued_at: now,
        })
    }

    pub fn list_agents(&self) -> Result<Vec<AgentView>> {
        let mut statement = self.conn.prepare(
            "SELECT id, name, role, status, last_seen_at
             FROM agents WHERE project_id = ?1 ORDER BY name",
        )?;
        let rows = statement.query_map(params![self.project.project_id], |row| {
            Ok(AgentView {
                agent_id: row.get(0)?,
                name: row.get(1)?,
                role: row.get(2)?,
                status: row.get(3)?,
                last_seen_at: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_message(
        &mut self,
        from: &str,
        token: &str,
        recipients: &[String],
        subject: &str,
        body: &str,
        thread_id: Option<&str>,
        priority: &str,
        requires_ack: bool,
    ) -> Result<MessageView> {
        self.send_message_idempotent(
            from,
            token,
            recipients,
            subject,
            body,
            thread_id,
            priority,
            requires_ack,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_message_idempotent(
        &mut self,
        from: &str,
        token: &str,
        recipients: &[String],
        subject: &str,
        body: &str,
        thread_id: Option<&str>,
        priority: &str,
        requires_ack: bool,
        idempotency_key: Option<&str>,
    ) -> Result<MessageView> {
        if recipients.is_empty() {
            return Err(BusError::Validation(
                "at least one recipient is required".into(),
            ));
        }
        if subject.trim().is_empty() || body.trim().is_empty() {
            return Err(BusError::Validation(
                "message subject and body cannot be empty".into(),
            ));
        }
        if !matches!(priority, "low" | "normal" | "high" | "urgent") {
            return Err(BusError::Validation(format!(
                "unsupported priority '{priority}'"
            )));
        }
        let request_hash = idempotency_key
            .map(|key| {
                validate_idempotency_key(key)?;
                hash_json(&json!({
                    "recipients": recipients,
                    "subject": subject.trim(),
                    "body": body.trim(),
                    "threadId": thread_id,
                    "priority": priority,
                    "requiresAck": requires_ack
                }))
            })
            .transpose()?;

        let sender = self.authenticate(from, token)?;
        let message_id = format!("msg_{}", Uuid::new_v4().simple());
        let now = now_ms();
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let (Some(key), Some(request_hash)) = (idempotency_key, request_hash.as_deref())
            && let Some(cached) = load_idempotent::<MessageView>(
                &tx,
                &self.project.project_id,
                &sender.id,
                "message.send",
                key,
                request_hash,
            )?
        {
            tx.commit()?;
            return Ok(cached);
        }
        let mut unique = HashSet::new();
        let mut resolved = Vec::new();
        for recipient in recipients {
            if !unique.insert(recipient.clone()) {
                continue;
            }
            let id: Option<String> = tx
                .query_row(
                    "SELECT id FROM agents WHERE project_id = ?1 AND name = ?2",
                    params![self.project.project_id, recipient],
                    |row| row.get(0),
                )
                .optional()?;
            let id = id.ok_or_else(|| BusError::AgentNotFound(recipient.clone()))?;
            resolved.push((recipient.clone(), id));
        }

        tx.execute(
            "INSERT INTO messages
             (id, project_id, sender_agent_id, thread_id, priority, subject, body,
              requires_ack, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                message_id,
                self.project.project_id,
                sender.id,
                thread_id,
                priority,
                subject.trim(),
                body.trim(),
                requires_ack as i64,
                now
            ],
        )?;
        for (_, recipient_id) in &resolved {
            tx.execute(
                "INSERT INTO message_receipts (message_id, recipient_agent_id, delivered_at)
                 VALUES (?1, ?2, ?3)",
                params![message_id, recipient_id, now],
            )?;
        }
        let recipient_names: Vec<&str> = resolved.iter().map(|(name, _)| name.as_str()).collect();
        let response = MessageView {
            message_id: message_id.clone(),
            sender: sender.name.clone(),
            recipient: resolved
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>()
                .join(","),
            thread_id: thread_id.map(ToOwned::to_owned),
            priority: priority.to_owned(),
            subject: subject.trim().to_owned(),
            body: body.trim().to_owned(),
            requires_ack,
            created_at: now,
            read_at: None,
            ack_at: None,
        };
        if let (Some(key), Some(request_hash)) = (idempotency_key, request_hash.as_deref()) {
            store_idempotent(
                &tx,
                &self.project.project_id,
                &sender.id,
                "message.send",
                key,
                request_hash,
                &response,
            )?;
        }
        append_event(
            &tx,
            &self.project.project_id,
            Some(&sender.id),
            "message_sent",
            "message",
            &message_id,
            json!({
                "recipients": recipient_names,
                "threadId": thread_id,
                "priority": priority,
                "requiresAck": requires_ack
            }),
        )?;
        tx.commit()?;
        Ok(response)
    }

    pub fn inbox(&self, agent: &str, token: &str, unread_only: bool) -> Result<Vec<MessageView>> {
        let recipient = self.authenticate(agent, token)?;
        let sql = if unread_only {
            "SELECT m.id, sender.name, recipient.name, m.thread_id, m.priority,
                    m.subject, m.body, m.requires_ack, m.created_at, r.read_at, r.ack_at
             FROM message_receipts r
             JOIN messages m ON m.id = r.message_id
             JOIN agents sender ON sender.id = m.sender_agent_id
             JOIN agents recipient ON recipient.id = r.recipient_agent_id
             WHERE r.recipient_agent_id = ?1 AND r.read_at IS NULL
             ORDER BY m.created_at DESC"
        } else {
            "SELECT m.id, sender.name, recipient.name, m.thread_id, m.priority,
                    m.subject, m.body, m.requires_ack, m.created_at, r.read_at, r.ack_at
             FROM message_receipts r
             JOIN messages m ON m.id = r.message_id
             JOIN agents sender ON sender.id = m.sender_agent_id
             JOIN agents recipient ON recipient.id = r.recipient_agent_id
             WHERE r.recipient_agent_id = ?1
             ORDER BY m.created_at DESC"
        };
        let mut statement = self.conn.prepare(sql)?;
        let rows = statement.query_map(params![recipient.id], map_message)?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn mark_read(
        &mut self,
        agent: &str,
        token: &str,
        message_id: &str,
    ) -> Result<MessageReceipt> {
        self.update_receipt(agent, token, message_id, false)
    }

    pub fn acknowledge_message(
        &mut self,
        agent: &str,
        token: &str,
        message_id: &str,
    ) -> Result<MessageReceipt> {
        self.update_receipt(agent, token, message_id, true)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_task(
        &mut self,
        creator: &str,
        token: &str,
        task_id: &str,
        title: &str,
        description: Option<&str>,
        depends_on: &[String],
    ) -> Result<TaskView> {
        validate_task_id(task_id)?;
        if title.trim().is_empty() {
            return Err(BusError::Validation("task title cannot be empty".into()));
        }
        let actor = self.authenticate(creator, token)?;
        let now = now_ms();
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        for dependency in depends_on {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM tasks WHERE project_id = ?1 AND id = ?2)",
                params![self.project.project_id, dependency],
                |row| row.get(0),
            )?;
            if !exists {
                return Err(BusError::Validation(format!(
                    "dependency task '{dependency}' does not exist"
                )));
            }
        }

        let status = if dependencies_complete(&tx, &self.project.project_id, depends_on)? {
            "ready"
        } else {
            "pending"
        };
        tx.execute(
            "INSERT INTO tasks
             (id, project_id, title, description, status, version, created_by_agent_id,
              created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7, ?7)",
            params![
                task_id,
                self.project.project_id,
                title.trim(),
                description,
                status,
                actor.id,
                now
            ],
        )
        .map_err(|error| match error {
            rusqlite::Error::SqliteFailure(_, _) => {
                BusError::Conflict(format!("task '{task_id}' already exists"))
            }
            other => BusError::Database(other),
        })?;
        for dependency in depends_on {
            tx.execute(
                "INSERT INTO task_dependencies (project_id, task_id, depends_on_task_id)
                 VALUES (?1, ?2, ?3)",
                params![self.project.project_id, task_id, dependency],
            )?;
        }
        append_event(
            &tx,
            &self.project.project_id,
            Some(&actor.id),
            "task_created",
            "task",
            task_id,
            json!({"status": status, "dependsOn": depends_on}),
        )?;
        tx.commit()?;
        self.get_task(task_id)
    }

    pub fn claim_task(&mut self, agent: &str, token: &str, task_id: &str) -> Result<TaskView> {
        let actor = self.authenticate(agent, token)?;
        let now = now_ms();
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: Option<(String, i64, Option<String>)> = tx
            .query_row(
                "SELECT status, version, owner_agent_id FROM tasks
                 WHERE project_id = ?1 AND id = ?2",
                params![self.project.project_id, task_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let (status, version, owner) = current
            .ok_or_else(|| BusError::Validation(format!("task '{task_id}' does not exist")))?;
        if owner.is_some() || !matches!(status.as_str(), "pending" | "ready") {
            return Err(BusError::Conflict(format!(
                "task '{task_id}' is not claimable (status={status})"
            )));
        }
        let incomplete: i64 = tx.query_row(
            "SELECT COUNT(*) FROM task_dependencies d
             JOIN tasks dependency
               ON dependency.project_id = d.project_id AND dependency.id = d.depends_on_task_id
             WHERE d.project_id = ?1 AND d.task_id = ?2 AND dependency.status != 'completed'",
            params![self.project.project_id, task_id],
            |row| row.get(0),
        )?;
        if incomplete > 0 {
            return Err(BusError::Conflict(format!(
                "task '{task_id}' has {incomplete} incomplete dependencies"
            )));
        }
        let changed = tx.execute(
            "UPDATE tasks SET owner_agent_id = ?1, status = 'claimed', version = version + 1,
                              updated_at = ?2
             WHERE project_id = ?3 AND id = ?4 AND owner_agent_id IS NULL
               AND version = ?5 AND status IN ('pending', 'ready')",
            params![actor.id, now, self.project.project_id, task_id, version],
        )?;
        if changed != 1 {
            return Err(BusError::Conflict(format!(
                "task '{task_id}' was claimed concurrently"
            )));
        }
        append_event(
            &tx,
            &self.project.project_id,
            Some(&actor.id),
            "task_claimed",
            "task",
            task_id,
            json!({"agent": actor.name, "previousVersion": version}),
        )?;
        tx.commit()?;
        self.get_task(task_id)
    }

    pub fn update_task(
        &mut self,
        agent: &str,
        token: &str,
        task_id: &str,
        expected_version: i64,
        status: &str,
        blocked_reason: Option<&str>,
    ) -> Result<TaskView> {
        if !TASK_STATUSES.contains(&status) {
            return Err(BusError::Validation(format!(
                "unsupported task status '{status}'"
            )));
        }
        if status == "blocked" && blocked_reason.is_none_or(|reason| reason.trim().is_empty()) {
            return Err(BusError::Validation(
                "blocked tasks require a non-empty reason".into(),
            ));
        }
        let actor = self.authenticate(agent, token)?;
        let now = now_ms();
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: Option<(String, Option<String>, i64)> = tx
            .query_row(
                "SELECT status, owner_agent_id, version FROM tasks WHERE project_id = ?1 AND id = ?2",
                params![self.project.project_id, task_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let (current_status, owner, actual_version) = current
            .ok_or_else(|| BusError::Validation(format!("task '{task_id}' does not exist")))?;
        if owner.as_deref() != Some(actor.id.as_str()) {
            return Err(BusError::Unauthorized(format!(
                "agent '{}' must claim task '{task_id}' before updating it",
                actor.name,
            )));
        }
        if actual_version != expected_version {
            return Err(BusError::Conflict(format!(
                "task '{task_id}' version mismatch: expected {expected_version}, actual {actual_version}"
            )));
        }
        validate_task_transition(&current_status, status)?;
        let changed = tx.execute(
            "UPDATE tasks SET status = ?1, blocked_reason = ?2, version = version + 1,
                              updated_at = ?3
             WHERE project_id = ?4 AND id = ?5 AND version = ?6",
            params![
                status,
                blocked_reason.map(str::trim),
                now,
                self.project.project_id,
                task_id,
                expected_version
            ],
        )?;
        if changed != 1 {
            return Err(BusError::Conflict(format!(
                "task '{task_id}' changed concurrently"
            )));
        }
        if status == "completed" {
            unlock_ready_tasks(&tx, &self.project.project_id, now)?;
        }
        append_event(
            &tx,
            &self.project.project_id,
            Some(&actor.id),
            if status == "completed" {
                "task_completed"
            } else {
                "task_updated"
            },
            "task",
            task_id,
            json!({
                "status": status,
                "blockedReason": blocked_reason,
                "previousVersion": expected_version
            }),
        )?;
        tx.commit()?;
        self.get_task(task_id)
    }

    pub fn get_task(&self, task_id: &str) -> Result<TaskView> {
        let base: Option<TaskRecord> = self
            .conn
            .query_row(
                "SELECT t.id, t.title, t.description, t.status, owner.name, t.version,
                            t.blocked_reason, t.created_at, t.updated_at
                     FROM tasks t LEFT JOIN agents owner ON owner.id = t.owner_agent_id
                     WHERE t.project_id = ?1 AND t.id = ?2",
                params![self.project.project_id, task_id],
                |row| {
                    Ok(TaskRecord {
                        task_id: row.get(0)?,
                        title: row.get(1)?,
                        description: row.get(2)?,
                        status: row.get(3)?,
                        owner: row.get(4)?,
                        version: row.get(5)?,
                        blocked_reason: row.get(6)?,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                    })
                },
            )
            .optional()?;
        let record =
            base.ok_or_else(|| BusError::Validation(format!("task '{task_id}' does not exist")))?;
        Ok(TaskView {
            depends_on: self.task_dependencies(&record.task_id)?,
            task_id: record.task_id,
            title: record.title,
            description: record.description,
            status: record.status,
            owner: record.owner,
            version: record.version,
            blocked_reason: record.blocked_reason,
            created_at: record.created_at,
            updated_at: record.updated_at,
        })
    }

    pub fn list_tasks(&self) -> Result<Vec<TaskView>> {
        let task_ids = {
            let mut statement = self
                .conn
                .prepare("SELECT id FROM tasks WHERE project_id = ?1 ORDER BY created_at, id")?;
            let rows = statement.query_map(params![self.project.project_id], |row| row.get(0))?;
            rows.collect::<std::result::Result<Vec<String>, _>>()?
        };
        task_ids
            .iter()
            .map(|task_id| self.get_task(task_id))
            .collect()
    }

    pub fn reserve_path(
        &mut self,
        agent: &str,
        token: &str,
        path_pattern: &str,
        ttl_seconds: i64,
        exclusive: bool,
        reason: Option<&str>,
    ) -> Result<ReservationView> {
        self.reserve_path_idempotent(
            agent,
            token,
            path_pattern,
            ttl_seconds,
            exclusive,
            reason,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reserve_path_idempotent(
        &mut self,
        agent: &str,
        token: &str,
        path_pattern: &str,
        ttl_seconds: i64,
        exclusive: bool,
        reason: Option<&str>,
        idempotency_key: Option<&str>,
    ) -> Result<ReservationView> {
        validate_reservation_ttl(ttl_seconds)?;
        let normalized = normalize_path_pattern(path_pattern)?;
        let request_hash = idempotency_key
            .map(|key| {
                validate_idempotency_key(key)?;
                hash_json(&json!({
                    "pathPattern": normalized,
                    "ttlSeconds": ttl_seconds,
                    "exclusive": exclusive,
                    "reason": reason
                }))
            })
            .transpose()?;
        let actor = self.authenticate(agent, token)?;
        let now = now_ms();
        let expires_at = now + ttl_seconds * 1_000;
        let reservation_id = format!("rsv_{}", Uuid::new_v4().simple());
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let (Some(key), Some(request_hash)) = (idempotency_key, request_hash.as_deref())
            && let Some(cached) = load_idempotent::<ReservationView>(
                &tx,
                &self.project.project_id,
                &actor.id,
                "reservation.add",
                key,
                request_hash,
            )?
        {
            tx.commit()?;
            return Ok(cached);
        }

        let active = {
            let mut statement = tx.prepare(
                "SELECT r.id, owner.name, r.path_pattern, r.exclusive
                 FROM reservations r JOIN agents owner ON owner.id = r.owner_agent_id
                 WHERE r.project_id = ?1 AND r.released_at IS NULL AND r.expires_at > ?2",
            )?;
            let rows = statement.query_map(params![self.project.project_id, now], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, bool>(3)?,
                ))
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        };
        for (id, owner, existing, existing_exclusive) in active {
            if owner != actor.name
                && paths_overlap(&normalized, &existing)
                && (exclusive || existing_exclusive)
            {
                return Err(BusError::Conflict(format!(
                    "path '{normalized}' overlaps reservation {id} owned by {owner} on '{existing}'"
                )));
            }
        }

        tx.execute(
            "INSERT INTO reservations
             (id, project_id, owner_agent_id, path_pattern, exclusive, reason, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                reservation_id,
                self.project.project_id,
                actor.id,
                normalized,
                exclusive as i64,
                reason,
                now,
                expires_at
            ],
        )?;
        let response = ReservationView {
            reservation_id: reservation_id.clone(),
            owner: actor.name.clone(),
            path_pattern: normalized.clone(),
            exclusive,
            reason: reason.map(ToOwned::to_owned),
            created_at: now,
            expires_at,
        };
        if let (Some(key), Some(request_hash)) = (idempotency_key, request_hash.as_deref()) {
            store_idempotent(
                &tx,
                &self.project.project_id,
                &actor.id,
                "reservation.add",
                key,
                request_hash,
                &response,
            )?;
        }
        append_event(
            &tx,
            &self.project.project_id,
            Some(&actor.id),
            "paths_reserved",
            "reservation",
            &reservation_id,
            json!({
                "pathPattern": normalized,
                "exclusive": exclusive,
                "expiresAt": expires_at,
                "reason": reason
            }),
        )?;
        tx.commit()?;
        Ok(response)
    }

    pub fn release_reservation(
        &mut self,
        agent: &str,
        token: &str,
        reservation_id: &str,
    ) -> Result<ReleaseResult> {
        let actor = self.authenticate(agent, token)?;
        let now = now_ms();
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = tx.execute(
            "UPDATE reservations SET released_at = ?1
             WHERE id = ?2 AND project_id = ?3 AND owner_agent_id = ?4 AND released_at IS NULL",
            params![now, reservation_id, self.project.project_id, actor.id],
        )?;
        if changed != 1 {
            return Err(BusError::Conflict(format!(
                "active reservation '{reservation_id}' is not owned by agent '{}'",
                actor.name
            )));
        }
        append_event(
            &tx,
            &self.project.project_id,
            Some(&actor.id),
            "paths_released",
            "reservation",
            reservation_id,
            json!({"releasedAt": now}),
        )?;
        tx.commit()?;
        Ok(ReleaseResult {
            reservation_id: reservation_id.to_owned(),
            released_at: now,
        })
    }

    pub fn renew_reservation(
        &mut self,
        agent: &str,
        token: &str,
        reservation_id: &str,
        ttl_seconds: i64,
    ) -> Result<ReservationView> {
        self.renew_reservation_idempotent(agent, token, reservation_id, ttl_seconds, None)
    }

    pub fn renew_reservation_idempotent(
        &mut self,
        agent: &str,
        token: &str,
        reservation_id: &str,
        ttl_seconds: i64,
        idempotency_key: Option<&str>,
    ) -> Result<ReservationView> {
        validate_reservation_ttl(ttl_seconds)?;
        let request_hash = idempotency_key
            .map(|key| {
                validate_idempotency_key(key)?;
                hash_json(&json!({
                    "reservationId": reservation_id,
                    "ttlSeconds": ttl_seconds
                }))
            })
            .transpose()?;
        let actor = self.authenticate(agent, token)?;
        let now = now_ms();
        let expires_at = now + ttl_seconds * 1_000;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let (Some(key), Some(request_hash)) = (idempotency_key, request_hash.as_deref())
            && let Some(cached) = load_idempotent::<ReservationView>(
                &tx,
                &self.project.project_id,
                &actor.id,
                "reservation.renew",
                key,
                request_hash,
            )?
        {
            tx.commit()?;
            return Ok(cached);
        }
        let reservation: Option<(String, bool, Option<String>, i64)> = tx
            .query_row(
                "SELECT path_pattern, exclusive, reason, created_at FROM reservations
                 WHERE id = ?1 AND project_id = ?2 AND owner_agent_id = ?3
                   AND released_at IS NULL AND expires_at > ?4",
                params![reservation_id, self.project.project_id, actor.id, now],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        let (path_pattern, exclusive, reason, created_at) = reservation.ok_or_else(|| {
            BusError::Conflict(format!(
                "active reservation '{reservation_id}' is expired or not owned by agent '{}'",
                actor.name
            ))
        })?;
        tx.execute(
            "UPDATE reservations SET expires_at = ?1 WHERE id = ?2 AND project_id = ?3",
            params![expires_at, reservation_id, self.project.project_id],
        )?;
        let response = ReservationView {
            reservation_id: reservation_id.to_owned(),
            owner: actor.name.clone(),
            path_pattern,
            exclusive,
            reason,
            created_at,
            expires_at,
        };
        if let (Some(key), Some(request_hash)) = (idempotency_key, request_hash.as_deref()) {
            store_idempotent(
                &tx,
                &self.project.project_id,
                &actor.id,
                "reservation.renew",
                key,
                request_hash,
                &response,
            )?;
        }
        append_event(
            &tx,
            &self.project.project_id,
            Some(&actor.id),
            "paths_renewed",
            "reservation",
            reservation_id,
            json!({"expiresAt": expires_at, "ttlSeconds": ttl_seconds}),
        )?;
        tx.commit()?;
        Ok(response)
    }

    pub fn list_active_reservations(&self) -> Result<Vec<ReservationView>> {
        let now = now_ms();
        let mut statement = self.conn.prepare(
            "SELECT r.id, owner.name, r.path_pattern, r.exclusive, r.reason,
                    r.created_at, r.expires_at
             FROM reservations r JOIN agents owner ON owner.id = r.owner_agent_id
             WHERE r.project_id = ?1 AND r.released_at IS NULL AND r.expires_at > ?2
             ORDER BY r.path_pattern",
        )?;
        let rows = statement.query_map(params![self.project.project_id, now], |row| {
            Ok(ReservationView {
                reservation_id: row.get(0)?,
                owner: row.get(1)?,
                path_pattern: row.get(2)?,
                exclusive: row.get(3)?,
                reason: row.get(4)?,
                created_at: row.get(5)?,
                expires_at: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn publish_artifact(
        &mut self,
        agent: &str,
        token: &str,
        kind: &str,
        path: &str,
        summary: &str,
        task_id: Option<&str>,
        metadata: Option<&serde_json::Value>,
    ) -> Result<ArtifactView> {
        self.publish_artifact_idempotent(agent, token, kind, path, summary, task_id, metadata, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn publish_artifact_idempotent(
        &mut self,
        agent: &str,
        token: &str,
        kind: &str,
        path: &str,
        summary: &str,
        task_id: Option<&str>,
        metadata: Option<&serde_json::Value>,
        idempotency_key: Option<&str>,
    ) -> Result<ArtifactView> {
        if kind.trim().is_empty() || summary.trim().is_empty() {
            return Err(BusError::Validation(
                "artifact kind and summary cannot be empty".into(),
            ));
        }
        let actor = self.authenticate(agent, token)?;
        let normalized = normalize_path_pattern(path)?;
        let project_root = self.project_root.canonicalize()?;
        let artifact_path = self.project_root.join(&normalized).canonicalize()?;
        if !artifact_path.starts_with(&project_root) || !artifact_path.is_file() {
            return Err(BusError::Validation(format!(
                "artifact path '{normalized}' must resolve to a file inside the project"
            )));
        }
        let sha256 = sha256_file(&artifact_path)?;
        let metadata = metadata.cloned().unwrap_or_else(|| json!({}));
        let request_hash = idempotency_key
            .map(|key| {
                validate_idempotency_key(key)?;
                hash_json(&json!({
                    "kind": kind.trim(),
                    "path": normalized,
                    "sha256": sha256,
                    "summary": summary.trim(),
                    "taskId": task_id,
                    "metadata": metadata
                }))
            })
            .transpose()?;
        let artifact_id = format!("art_{}", Uuid::new_v4().simple());
        let now = now_ms();
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let (Some(key), Some(request_hash)) = (idempotency_key, request_hash.as_deref())
            && let Some(cached) = load_idempotent::<ArtifactView>(
                &tx,
                &self.project.project_id,
                &actor.id,
                "artifact.publish",
                key,
                request_hash,
            )?
        {
            tx.commit()?;
            return Ok(cached);
        }
        if let Some(task_id) = task_id {
            let task_exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM tasks WHERE project_id = ?1 AND id = ?2)",
                params![self.project.project_id, task_id],
                |row| row.get(0),
            )?;
            if !task_exists {
                return Err(BusError::Validation(format!(
                    "artifact task '{task_id}' does not exist"
                )));
            }
        }
        tx.execute(
            "INSERT INTO artifacts
             (id, project_id, publisher_agent_id, task_id, kind, path, sha256, summary,
              metadata_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                artifact_id,
                self.project.project_id,
                actor.id,
                task_id,
                kind.trim(),
                normalized,
                sha256,
                summary.trim(),
                serde_json::to_string(&metadata)?,
                now
            ],
        )?;
        let response = ArtifactView {
            artifact_id: artifact_id.clone(),
            publisher: actor.name.clone(),
            task_id: task_id.map(ToOwned::to_owned),
            kind: kind.trim().to_owned(),
            path: normalized.clone(),
            sha256: sha256.clone(),
            summary: summary.trim().to_owned(),
            metadata: metadata.clone(),
            created_at: now,
        };
        if let (Some(key), Some(request_hash)) = (idempotency_key, request_hash.as_deref()) {
            store_idempotent(
                &tx,
                &self.project.project_id,
                &actor.id,
                "artifact.publish",
                key,
                request_hash,
                &response,
            )?;
        }
        append_event(
            &tx,
            &self.project.project_id,
            Some(&actor.id),
            "artifact_published",
            "artifact",
            &artifact_id,
            json!({
                "taskId": task_id,
                "kind": kind.trim(),
                "path": normalized,
                "sha256": sha256
            }),
        )?;
        tx.commit()?;
        Ok(response)
    }

    pub fn list_artifacts(&self, task_id: Option<&str>) -> Result<Vec<ArtifactView>> {
        let sql = if task_id.is_some() {
            "SELECT a.id, publisher.name, a.task_id, a.kind, a.path, a.sha256,
                    a.summary, a.metadata_json, a.created_at
             FROM artifacts a JOIN agents publisher ON publisher.id = a.publisher_agent_id
             WHERE a.project_id = ?1 AND a.task_id = ?2 ORDER BY a.created_at DESC"
        } else {
            "SELECT a.id, publisher.name, a.task_id, a.kind, a.path, a.sha256,
                    a.summary, a.metadata_json, a.created_at
             FROM artifacts a JOIN agents publisher ON publisher.id = a.publisher_agent_id
             WHERE a.project_id = ?1 ORDER BY a.created_at DESC"
        };
        let mut statement = self.conn.prepare(sql)?;
        let map = |row: &rusqlite::Row<'_>| -> rusqlite::Result<ArtifactView> {
            let metadata_json: String = row.get(7)?;
            let metadata = serde_json::from_str(&metadata_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    7,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(ArtifactView {
                artifact_id: row.get(0)?,
                publisher: row.get(1)?,
                task_id: row.get(2)?,
                kind: row.get(3)?,
                path: row.get(4)?,
                sha256: row.get(5)?,
                summary: row.get(6)?,
                metadata,
                created_at: row.get(8)?,
            })
        };
        let rows = if let Some(task_id) = task_id {
            statement.query_map(params![self.project.project_id, task_id], map)?
        } else {
            statement.query_map(params![self.project.project_id], map)?
        };
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn list_events(
        &self,
        after_sequence: i64,
        limit: usize,
        event_types: &[String],
    ) -> Result<Vec<EventView>> {
        validate_event_query(after_sequence, limit, event_types)?;
        query_events(
            &self.conn,
            &self.project.project_id,
            after_sequence,
            limit,
            event_types,
        )
    }

    pub fn create_subscription(
        &mut self,
        agent: &str,
        token: &str,
        name: &str,
        event_types: &[String],
        from_sequence: Option<i64>,
    ) -> Result<SubscriptionView> {
        validate_identifier("subscription name", name)?;
        validate_event_query(0, 1, event_types)?;
        let actor = self.authenticate(agent, token)?;
        let now = now_ms();
        let subscription_id = format!("sub_{}", Uuid::new_v4().simple());
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let latest_sequence = latest_event_sequence(&tx, &self.project.project_id)?;
        let cursor_sequence = from_sequence.unwrap_or(latest_sequence);
        if !(0..=latest_sequence).contains(&cursor_sequence) {
            return Err(BusError::Validation(format!(
                "subscription cursor must be between 0 and current sequence {latest_sequence}"
            )));
        }
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM subscriptions
             WHERE project_id = ?1 AND agent_id = ?2 AND name = ?3)",
            params![self.project.project_id, actor.id, name],
            |row| row.get(0),
        )?;
        if exists {
            return Err(BusError::Conflict(format!(
                "subscription '{name}' already exists for agent '{}'",
                actor.name
            )));
        }
        tx.execute(
            "INSERT INTO subscriptions
             (id, project_id, agent_id, name, event_types_json, cursor_sequence,
              created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![
                subscription_id,
                self.project.project_id,
                actor.id,
                name,
                serde_json::to_string(event_types)?,
                cursor_sequence,
                now
            ],
        )?;
        append_event(
            &tx,
            &self.project.project_id,
            Some(&actor.id),
            "subscription_created",
            "subscription",
            &subscription_id,
            json!({
                "name": name,
                "eventTypes": event_types,
                "cursorSequence": cursor_sequence
            }),
        )?;
        tx.commit()?;
        Ok(SubscriptionView {
            subscription_id,
            agent: actor.name,
            name: name.to_owned(),
            event_types: event_types.to_vec(),
            cursor_sequence,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn list_subscriptions(&self, agent: &str, token: &str) -> Result<Vec<SubscriptionView>> {
        let actor = self.authenticate(agent, token)?;
        let mut statement = self.conn.prepare(
            "SELECT id, name, event_types_json, cursor_sequence, created_at, updated_at
             FROM subscriptions WHERE project_id = ?1 AND agent_id = ?2 ORDER BY name",
        )?;
        let rows = statement.query_map(params![self.project.project_id, actor.id], |row| {
            let event_types_json: String = row.get(2)?;
            let event_types = parse_json_column(&event_types_json, 2)?;
            Ok(SubscriptionView {
                subscription_id: row.get(0)?,
                agent: actor.name.clone(),
                name: row.get(1)?,
                event_types,
                cursor_sequence: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn poll_subscription(
        &mut self,
        agent: &str,
        token: &str,
        name: &str,
        limit: usize,
    ) -> Result<SubscriptionPoll> {
        validate_identifier("subscription name", name)?;
        if !(1..=500).contains(&limit) {
            return Err(BusError::Validation(
                "subscription poll limit must be between 1 and 500".into(),
            ));
        }
        let actor = self.authenticate(agent, token)?;
        let now = now_ms();
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: Option<(String, String, i64, i64, i64)> = tx
            .query_row(
                "SELECT id, event_types_json, cursor_sequence, created_at, updated_at
                 FROM subscriptions WHERE project_id = ?1 AND agent_id = ?2 AND name = ?3",
                params![self.project.project_id, actor.id, name],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional()?;
        let (subscription_id, event_types_json, cursor_sequence, created_at, _updated_at) = current
            .ok_or_else(|| {
                BusError::Validation(format!(
                    "subscription '{name}' does not exist for agent '{}'",
                    actor.name
                ))
            })?;
        let event_types: Vec<String> = serde_json::from_str(&event_types_json)?;
        let events = query_events(
            &tx,
            &self.project.project_id,
            cursor_sequence,
            limit,
            &event_types,
        )?;
        let latest_sequence = latest_event_sequence(&tx, &self.project.project_id)?;
        let scanned_through_sequence = events
            .last()
            .map(|event| event.sequence)
            .unwrap_or(latest_sequence);
        let changed = tx.execute(
            "UPDATE subscriptions SET cursor_sequence = ?1, updated_at = ?2
             WHERE project_id = ?3 AND id = ?4 AND cursor_sequence = ?5",
            params![
                scanned_through_sequence,
                now,
                self.project.project_id,
                subscription_id,
                cursor_sequence
            ],
        )?;
        if changed != 1 {
            return Err(BusError::Conflict(format!(
                "subscription '{name}' was polled concurrently"
            )));
        }
        tx.commit()?;
        Ok(SubscriptionPoll {
            subscription: SubscriptionView {
                subscription_id,
                agent: actor.name,
                name: name.to_owned(),
                event_types,
                cursor_sequence: scanned_through_sequence,
                created_at,
                updated_at: now,
            },
            events,
            scanned_through_sequence,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_handoff(
        &mut self,
        from: &str,
        token: &str,
        recipients: &[String],
        summary: &str,
        task_id: Option<&str>,
        decisions: &[String],
        artifacts: &[String],
        blockers: &[String],
        next_actions: &[String],
        idempotency_key: Option<&str>,
    ) -> Result<MessageView> {
        if summary.trim().is_empty() {
            return Err(BusError::Validation(
                "handoff summary cannot be empty".into(),
            ));
        }
        self.authenticate(from, token)?;
        if let Some(task_id) = task_id {
            self.get_task(task_id)?;
        }
        for artifact_id in artifacts {
            let exists: bool = self.conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM artifacts WHERE project_id = ?1 AND id = ?2)",
                params![self.project.project_id, artifact_id],
                |row| row.get(0),
            )?;
            if !exists {
                return Err(BusError::Validation(format!(
                    "handoff artifact '{artifact_id}' does not exist"
                )));
            }
        }
        let body = serde_json::to_string_pretty(&json!({
            "kind": "handoff",
            "summary": summary.trim(),
            "taskId": task_id,
            "decisions": decisions,
            "artifacts": artifacts,
            "blockers": blockers,
            "nextActions": next_actions
        }))?;
        let subject_summary: String = summary.trim().chars().take(80).collect();
        self.send_message_idempotent(
            from,
            token,
            recipients,
            &format!("Handoff: {subject_summary}"),
            &body,
            task_id,
            "high",
            true,
            idempotency_key,
        )
    }

    pub fn handoff_snapshot(
        &self,
        agent: &str,
        token: &str,
        after_sequence: i64,
    ) -> Result<HandoffSnapshot> {
        self.authenticate(agent, token)?;
        let unread_messages = self.inbox(agent, token, true)?;
        let owned_tasks = self
            .list_tasks()?
            .into_iter()
            .filter(|task| {
                task.owner.as_deref() == Some(agent)
                    && !matches!(task.status.as_str(), "completed" | "abandoned")
            })
            .collect();
        let active_reservations = self
            .list_active_reservations()?
            .into_iter()
            .filter(|reservation| reservation.owner == agent)
            .collect();
        let recent_artifacts = self
            .list_artifacts(None)?
            .into_iter()
            .filter(|artifact| artifact.publisher == agent)
            .take(20)
            .collect();
        let recent_events = self.list_events(after_sequence, 50, &[])?;
        let latest_event_sequence = latest_event_sequence(&self.conn, &self.project.project_id)?;
        Ok(HandoffSnapshot {
            agent: agent.to_owned(),
            unread_messages,
            owned_tasks,
            active_reservations,
            recent_artifacts,
            recent_events,
            latest_event_sequence,
        })
    }

    pub fn doctor(&self) -> Result<DoctorReport> {
        let integrity: String = self
            .conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        let journal_mode: String = self
            .conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
        let foreign_keys: i64 = self
            .conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
        let schema_version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;
        let counts: (i64, i64, i64, i64, i64) = self.conn.query_row(
            "SELECT
               (SELECT COUNT(*) FROM agents WHERE project_id = ?1),
               (SELECT COUNT(*) FROM messages WHERE project_id = ?1),
               (SELECT COUNT(*) FROM tasks WHERE project_id = ?1),
               (SELECT COUNT(*) FROM reservations
                 WHERE project_id = ?1 AND released_at IS NULL AND expires_at > ?2),
               (SELECT COUNT(*) FROM artifacts WHERE project_id = ?1)",
            params![self.project.project_id, now_ms()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )?;
        Ok(DoctorReport {
            ok: integrity == "ok"
                && journal_mode.eq_ignore_ascii_case("wal")
                && foreign_keys == 1
                && schema_version == SCHEMA_VERSION,
            integrity,
            journal_mode,
            foreign_keys_enabled: foreign_keys == 1,
            schema_version,
            project_root: self.project_root.to_string_lossy().into_owned(),
            database_path: self.database_path.to_string_lossy().into_owned(),
            agents: counts.0,
            messages: counts.1,
            tasks: counts.2,
            active_reservations: counts.3,
            artifacts: counts.4,
        })
    }

    pub fn backup_to(&self, destination: &Path) -> Result<BackupView> {
        if destination.exists() {
            return Err(BusError::Conflict(format!(
                "backup destination '{}' already exists",
                destination.display()
            )));
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary =
            destination.with_extension(format!("vibebus-tmp-{}", Uuid::new_v4().simple()));
        let backup_result = (|| -> Result<()> {
            let mut destination_connection = Connection::open(&temporary)?;
            let backup = rusqlite::backup::Backup::new(&self.conn, &mut destination_connection)?;
            backup.run_to_completion(64, Duration::from_millis(10), None)?;
            drop(backup);
            destination_connection.close().map_err(|(_, error)| error)?;
            fs::rename(&temporary, destination)?;
            Ok(())
        })();
        if backup_result.is_err() && temporary.exists() {
            let _ = fs::remove_file(&temporary);
        }
        backup_result?;
        let metadata = fs::metadata(destination)?;
        Ok(BackupView {
            path: destination.to_string_lossy().into_owned(),
            bytes: metadata.len(),
            sha256: sha256_file(destination)?,
            created_at: now_ms(),
        })
    }

    fn update_receipt(
        &mut self,
        agent: &str,
        token: &str,
        message_id: &str,
        acknowledge: bool,
    ) -> Result<MessageReceipt> {
        let actor = self.authenticate(agent, token)?;
        let now = now_ms();
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = if acknowledge {
            tx.execute(
                "UPDATE message_receipts SET read_at = COALESCE(read_at, ?1), ack_at = COALESCE(ack_at, ?1)
                 WHERE message_id = ?2 AND recipient_agent_id = ?3",
                params![now, message_id, actor.id],
            )?
        } else {
            tx.execute(
                "UPDATE message_receipts SET read_at = COALESCE(read_at, ?1)
                 WHERE message_id = ?2 AND recipient_agent_id = ?3",
                params![now, message_id, actor.id],
            )?
        };
        if changed != 1 {
            return Err(BusError::Unauthorized(format!(
                "message '{message_id}' is not addressed to agent '{}'",
                actor.name
            )));
        }
        append_event(
            &tx,
            &self.project.project_id,
            Some(&actor.id),
            if acknowledge {
                "message_acked"
            } else {
                "message_read"
            },
            "message",
            message_id,
            json!({"agent": actor.name, "at": now}),
        )?;
        let (read_at, ack_at): (i64, Option<i64>) = tx.query_row(
            "SELECT read_at, ack_at FROM message_receipts
             WHERE message_id = ?1 AND recipient_agent_id = ?2",
            params![message_id, actor.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        tx.commit()?;
        Ok(MessageReceipt {
            message_id: message_id.to_owned(),
            recipient: actor.name,
            read_at,
            ack_at,
        })
    }

    fn authenticate(&self, name: &str, token: &str) -> Result<AuthenticatedAgent> {
        let row: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT id, token_hash FROM agents WHERE project_id = ?1 AND name = ?2",
                params![self.project.project_id, name],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let (id, token_hash) = row.ok_or_else(|| BusError::AgentNotFound(name.to_owned()))?;
        if token.is_empty() || !secret_matches(token, &token_hash) {
            return Err(BusError::Unauthorized(name.to_owned()));
        }
        self.conn.execute(
            "UPDATE agents SET last_seen_at = ?1 WHERE id = ?2",
            params![now_ms(), id],
        )?;
        Ok(AuthenticatedAgent {
            id,
            name: name.to_owned(),
        })
    }

    fn task_dependencies(&self, task_id: &str) -> Result<Vec<String>> {
        let mut statement = self.conn.prepare(
            "SELECT depends_on_task_id FROM task_dependencies
             WHERE project_id = ?1 AND task_id = ?2 ORDER BY depends_on_task_id",
        )?;
        let rows =
            statement.query_map(params![self.project.project_id, task_id], |row| row.get(0))?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    fn migrate(&mut self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                root_path TEXT NOT NULL,
                created_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS agents (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                name TEXT NOT NULL,
                role TEXT NOT NULL,
                token_hash TEXT NOT NULL,
                recovery_hash TEXT,
                token_generation INTEGER NOT NULL DEFAULT 1,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                last_seen_at INTEGER NOT NULL,
                UNIQUE(project_id, name),
                FOREIGN KEY(project_id) REFERENCES projects(id)
             );
             CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                sender_agent_id TEXT NOT NULL,
                thread_id TEXT,
                priority TEXT NOT NULL,
                subject TEXT NOT NULL,
                body TEXT NOT NULL,
                requires_ack INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY(project_id) REFERENCES projects(id),
                FOREIGN KEY(sender_agent_id) REFERENCES agents(id)
             );
             CREATE TABLE IF NOT EXISTS message_receipts (
                message_id TEXT NOT NULL,
                recipient_agent_id TEXT NOT NULL,
                delivered_at INTEGER NOT NULL,
                read_at INTEGER,
                ack_at INTEGER,
                closed_at INTEGER,
                PRIMARY KEY(message_id, recipient_agent_id),
                FOREIGN KEY(message_id) REFERENCES messages(id),
                FOREIGN KEY(recipient_agent_id) REFERENCES agents(id)
             );
             CREATE TABLE IF NOT EXISTS tasks (
                id TEXT NOT NULL,
                project_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL,
                owner_agent_id TEXT,
                version INTEGER NOT NULL,
                blocked_reason TEXT,
                created_by_agent_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY(project_id, id),
                FOREIGN KEY(project_id) REFERENCES projects(id),
                FOREIGN KEY(owner_agent_id) REFERENCES agents(id),
                FOREIGN KEY(created_by_agent_id) REFERENCES agents(id)
             );
             CREATE TABLE IF NOT EXISTS task_dependencies (
                project_id TEXT NOT NULL,
                task_id TEXT NOT NULL,
                depends_on_task_id TEXT NOT NULL,
                PRIMARY KEY(project_id, task_id, depends_on_task_id),
                FOREIGN KEY(project_id, task_id) REFERENCES tasks(project_id, id),
                FOREIGN KEY(project_id, depends_on_task_id) REFERENCES tasks(project_id, id)
             );
             CREATE TABLE IF NOT EXISTS reservations (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                owner_agent_id TEXT NOT NULL,
                path_pattern TEXT NOT NULL,
                exclusive INTEGER NOT NULL,
                reason TEXT,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL,
                released_at INTEGER,
                FOREIGN KEY(project_id) REFERENCES projects(id),
                FOREIGN KEY(owner_agent_id) REFERENCES agents(id)
             );
             CREATE TABLE IF NOT EXISTS artifacts (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                publisher_agent_id TEXT NOT NULL,
                task_id TEXT,
                kind TEXT NOT NULL,
                path TEXT NOT NULL,
                sha256 TEXT NOT NULL,
                summary TEXT NOT NULL,
                metadata_json TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY(project_id) REFERENCES projects(id),
                FOREIGN KEY(publisher_agent_id) REFERENCES agents(id),
                FOREIGN KEY(project_id, task_id) REFERENCES tasks(project_id, id)
             );
             CREATE TABLE IF NOT EXISTS idempotency_records (
                project_id TEXT NOT NULL,
                actor_agent_id TEXT NOT NULL,
                operation TEXT NOT NULL,
                idempotency_key TEXT NOT NULL,
                request_hash TEXT NOT NULL,
                response_json TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY(project_id, actor_agent_id, operation, idempotency_key),
                FOREIGN KEY(project_id) REFERENCES projects(id),
                FOREIGN KEY(actor_agent_id) REFERENCES agents(id)
             );
             CREATE TABLE IF NOT EXISTS subscriptions (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                name TEXT NOT NULL,
                event_types_json TEXT NOT NULL,
                cursor_sequence INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                UNIQUE(project_id, agent_id, name),
                FOREIGN KEY(project_id) REFERENCES projects(id),
                FOREIGN KEY(agent_id) REFERENCES agents(id)
             );
             CREATE TABLE IF NOT EXISTS events (
                sequence INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT NOT NULL UNIQUE,
                project_id TEXT NOT NULL,
                actor_agent_id TEXT,
                event_type TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                idempotency_key TEXT,
                FOREIGN KEY(project_id) REFERENCES projects(id),
                FOREIGN KEY(actor_agent_id) REFERENCES agents(id)
             );
             CREATE UNIQUE INDEX IF NOT EXISTS idx_events_idempotency
                ON events(project_id, idempotency_key) WHERE idempotency_key IS NOT NULL;
             CREATE INDEX IF NOT EXISTS idx_receipts_agent_unread
                ON message_receipts(recipient_agent_id, read_at);
             CREATE INDEX IF NOT EXISTS idx_tasks_status
                ON tasks(project_id, status);
             CREATE INDEX IF NOT EXISTS idx_reservations_active
                ON reservations(project_id, released_at, expires_at);
             CREATE INDEX IF NOT EXISTS idx_artifacts_task
                ON artifacts(project_id, task_id, created_at);
             CREATE INDEX IF NOT EXISTS idx_idempotency_created
                ON idempotency_records(project_id, created_at);
             CREATE INDEX IF NOT EXISTS idx_subscriptions_agent
                ON subscriptions(project_id, agent_id, name);",
        )?;
        ensure_column(&self.conn, "agents", "recovery_hash", "TEXT")?;
        ensure_column(
            &self.conn,
            "agents",
            "token_generation",
            "INTEGER NOT NULL DEFAULT 1",
        )?;
        self.conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![SCHEMA_VERSION, now_ms()],
        )?;
        self.conn.execute(
            "INSERT INTO projects (id, name, root_path, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET name = excluded.name, root_path = excluded.root_path",
            params![
                self.project.project_id,
                self.project.name,
                self.project_root.to_string_lossy(),
                now_ms()
            ],
        )?;
        Ok(())
    }
}

fn map_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<MessageView> {
    Ok(MessageView {
        message_id: row.get(0)?,
        sender: row.get(1)?,
        recipient: row.get(2)?,
        thread_id: row.get(3)?,
        priority: row.get(4)?,
        subject: row.get(5)?,
        body: row.get(6)?,
        requires_ack: row.get(7)?,
        created_at: row.get(8)?,
        read_at: row.get(9)?,
        ack_at: row.get(10)?,
    })
}

fn append_event(
    tx: &Transaction<'_>,
    project_id: &str,
    actor_agent_id: Option<&str>,
    event_type: &str,
    entity_type: &str,
    entity_id: &str,
    payload: serde_json::Value,
) -> Result<()> {
    tx.execute(
        "INSERT INTO events
         (id, project_id, actor_agent_id, event_type, entity_type, entity_id,
          payload_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            format!("evt_{}", Uuid::new_v4().simple()),
            project_id,
            actor_agent_id,
            event_type,
            entity_type,
            entity_id,
            serde_json::to_string(&payload)?,
            now_ms()
        ],
    )?;
    Ok(())
}

fn validate_event_query(after_sequence: i64, limit: usize, event_types: &[String]) -> Result<()> {
    if after_sequence < 0 {
        return Err(BusError::Validation(
            "event cursor cannot be negative".into(),
        ));
    }
    if !(1..=500).contains(&limit) {
        return Err(BusError::Validation(
            "event query limit must be between 1 and 500".into(),
        ));
    }
    if event_types.len() > 32 {
        return Err(BusError::Validation(
            "event query accepts at most 32 event types".into(),
        ));
    }
    for event_type in event_types {
        let valid = !event_type.is_empty()
            && event_type.len() <= 64
            && event_type.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
            });
        if !valid {
            return Err(BusError::Validation(
                "event type must be 1-64 ASCII letters, digits, '-', '_' or '.'".into(),
            ));
        }
    }
    Ok(())
}

fn latest_event_sequence(connection: &Connection, project_id: &str) -> Result<i64> {
    Ok(connection.query_row(
        "SELECT COALESCE(MAX(sequence), 0) FROM events WHERE project_id = ?1",
        params![project_id],
        |row| row.get(0),
    )?)
}

fn query_events(
    connection: &Connection,
    project_id: &str,
    after_sequence: i64,
    limit: usize,
    event_types: &[String],
) -> Result<Vec<EventView>> {
    validate_event_query(after_sequence, limit, event_types)?;
    let mut sql = String::from(
        "SELECT e.sequence, e.id, actor.name, e.event_type, e.entity_type,
                e.entity_id, e.payload_json, e.created_at
         FROM events e
         LEFT JOIN agents actor ON actor.id = e.actor_agent_id
         WHERE e.project_id = ? AND e.sequence > ?",
    );
    if !event_types.is_empty() {
        sql.push_str(" AND e.event_type IN (");
        sql.push_str(&vec!["?"; event_types.len()].join(", "));
        sql.push(')');
    }
    sql.push_str(" ORDER BY e.sequence LIMIT ?");

    let mut values = Vec::with_capacity(event_types.len() + 3);
    values.push(rusqlite::types::Value::Text(project_id.to_owned()));
    values.push(rusqlite::types::Value::Integer(after_sequence));
    values.extend(
        event_types
            .iter()
            .cloned()
            .map(rusqlite::types::Value::Text),
    );
    values.push(rusqlite::types::Value::Integer(limit as i64));

    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(values.iter()), |row| {
        let payload_json: String = row.get(6)?;
        Ok(EventView {
            sequence: row.get(0)?,
            event_id: row.get(1)?,
            actor: row.get(2)?,
            event_type: row.get(3)?,
            entity_type: row.get(4)?,
            entity_id: row.get(5)?,
            payload: parse_json_column(&payload_json, 6)?,
            created_at: row.get(7)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
}

fn parse_json_column<T: serde::de::DeserializeOwned>(
    value: &str,
    column: usize,
) -> rusqlite::Result<T> {
    serde_json::from_str(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn load_idempotent<T: serde::de::DeserializeOwned>(
    tx: &Transaction<'_>,
    project_id: &str,
    actor_agent_id: &str,
    operation: &str,
    idempotency_key: &str,
    request_hash: &str,
) -> Result<Option<T>> {
    let cached: Option<(String, String)> = tx
        .query_row(
            "SELECT request_hash, response_json FROM idempotency_records
             WHERE project_id = ?1 AND actor_agent_id = ?2 AND operation = ?3
               AND idempotency_key = ?4",
            params![project_id, actor_agent_id, operation, idempotency_key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((cached_hash, response_json)) = cached else {
        return Ok(None);
    };
    if cached_hash != request_hash {
        return Err(BusError::Conflict(format!(
            "idempotency key '{idempotency_key}' was already used with a different {operation} request"
        )));
    }
    Ok(Some(serde_json::from_str(&response_json)?))
}

#[allow(clippy::too_many_arguments)]
fn store_idempotent<T: serde::Serialize>(
    tx: &Transaction<'_>,
    project_id: &str,
    actor_agent_id: &str,
    operation: &str,
    idempotency_key: &str,
    request_hash: &str,
    response: &T,
) -> Result<()> {
    tx.execute(
        "INSERT INTO idempotency_records
         (project_id, actor_agent_id, operation, idempotency_key, request_hash,
          response_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            project_id,
            actor_agent_id,
            operation,
            idempotency_key,
            request_hash,
            serde_json::to_string(response)?,
            now_ms()
        ],
    )?;
    Ok(())
}

fn hash_json(value: &serde_json::Value) -> Result<String> {
    let bytes = serde_json::to_vec(value)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn dependencies_complete(
    tx: &Transaction<'_>,
    project_id: &str,
    dependencies: &[String],
) -> Result<bool> {
    for dependency in dependencies {
        let status: String = tx.query_row(
            "SELECT status FROM tasks WHERE project_id = ?1 AND id = ?2",
            params![project_id, dependency],
            |row| row.get(0),
        )?;
        if status != "completed" {
            return Ok(false);
        }
    }
    Ok(true)
}

fn unlock_ready_tasks(tx: &Transaction<'_>, project_id: &str, now: i64) -> Result<()> {
    tx.execute(
        "UPDATE tasks SET status = 'ready', version = version + 1, updated_at = ?1
         WHERE project_id = ?2 AND status = 'pending'
           AND NOT EXISTS (
             SELECT 1 FROM task_dependencies d
             JOIN tasks dependency
               ON dependency.project_id = d.project_id AND dependency.id = d.depends_on_task_id
             WHERE d.project_id = tasks.project_id AND d.task_id = tasks.id
               AND dependency.status != 'completed'
           )",
        params![now, project_id],
    )?;
    Ok(())
}

fn validate_identifier(label: &str, value: &str) -> Result<()> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'));
    if valid {
        Ok(())
    } else {
        Err(BusError::Validation(format!(
            "{label} must be 1-64 ASCII letters, digits, '-' or '_'"
        )))
    }
}

fn validate_task_id(task_id: &str) -> Result<()> {
    let valid = !task_id.is_empty()
        && task_id.len() <= 96
        && task_id.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        });
    if valid {
        Ok(())
    } else {
        Err(BusError::Validation(
            "task id must be 1-96 ASCII letters, digits, '-', '_' or '.'".into(),
        ))
    }
}

fn validate_idempotency_key(key: &str) -> Result<()> {
    let valid = !key.is_empty()
        && key.len() <= 128
        && key.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':')
        });
    if valid {
        Ok(())
    } else {
        Err(BusError::Validation(
            "idempotency key must be 1-128 ASCII letters, digits, '-', '_', '.' or ':'".into(),
        ))
    }
}

fn hash_secret(secret: &str) -> String {
    Sha256::digest(secret.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn generate_secret(prefix: &str) -> String {
    format!(
        "{prefix}_{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    )
}

fn secret_matches(secret: &str, expected_hash: &str) -> bool {
    let actual_hash = hash_secret(secret);
    if actual_hash.len() != expected_hash.len() {
        return false;
    }
    actual_hash
        .as_bytes()
        .iter()
        .zip(expected_hash.as_bytes())
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

fn validate_reservation_ttl(ttl_seconds: i64) -> Result<()> {
    if ttl_seconds <= 0 || ttl_seconds > 86_400 {
        return Err(BusError::Validation(
            "reservation TTL must be between 1 and 86400 seconds".into(),
        ));
    }
    Ok(())
}

fn ensure_column(
    connection: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    let exists = columns
        .collect::<std::result::Result<Vec<_>, _>>()?
        .iter()
        .any(|existing| existing == column);
    drop(statement);
    if !exists {
        connection.execute_batch(&format!(
            "ALTER TABLE {table} ADD COLUMN {column} {definition}"
        ))?;
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

fn normalize_path_pattern(path: &str) -> Result<String> {
    let trimmed = path.trim();
    let candidate = Path::new(trimmed);
    if candidate.is_absolute()
        || trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || trimmed.as_bytes().get(1).is_some_and(|byte| *byte == b':')
    {
        return Err(BusError::Validation(
            "reservation path must be project-relative".into(),
        ));
    }
    let normalized = trimmed.replace('\\', "/").trim_matches('/').to_owned();
    if normalized.is_empty()
        || normalized.starts_with("..")
        || normalized.split('/').any(|segment| segment == "..")
    {
        return Err(BusError::Validation(
            "reservation path must be project-relative and cannot contain '..'".into(),
        ));
    }
    Ok(if cfg!(windows) {
        normalized.to_ascii_lowercase()
    } else {
        normalized
    })
}

fn validate_task_transition(current: &str, next: &str) -> Result<()> {
    let allowed = match current {
        "claimed" => matches!(next, "working" | "blocked" | "completed" | "abandoned"),
        "working" => matches!(next, "review" | "blocked" | "completed" | "abandoned"),
        "review" => matches!(next, "working" | "blocked" | "completed" | "abandoned"),
        "blocked" => matches!(next, "working" | "abandoned"),
        _ => false,
    };
    if !allowed {
        return Err(BusError::Conflict(format!(
            "unsupported task transition from '{current}' to '{next}'"
        )));
    }
    Ok(())
}

fn paths_overlap(left: &str, right: &str) -> bool {
    left == right
        || left
            .strip_prefix(right)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || right
            .strip_prefix(left)
            .is_some_and(|suffix| suffix.starts_with('/'))
}
