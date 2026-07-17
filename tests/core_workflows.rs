use std::fs;
use std::sync::{Arc, Barrier};
use std::thread;

use rusqlite::Connection;
use tempfile::TempDir;
use vibebus::{Bus, BusError, initialize_project};

struct Harness {
    project: TempDir,
    data: TempDir,
}

impl Harness {
    fn new() -> Self {
        let project = tempfile::tempdir().expect("project temp directory");
        let data = tempfile::tempdir().expect("data temp directory");
        initialize_project(project.path(), "test-project", Some(data.path()))
            .expect("initialize project");
        Self { project, data }
    }

    fn bus(&self) -> Bus {
        Bus::open(self.project.path(), Some(self.data.path())).expect("open bus")
    }
}

#[test]
fn directed_messages_are_isolated_and_acknowledged() {
    let harness = Harness::new();
    let mut bus = harness.bus();
    let frontend = bus
        .register_agent("frontend", "frontend")
        .expect("register frontend");
    let backend = bus
        .register_agent("backend", "backend")
        .expect("register backend");
    let deploy = bus
        .register_agent("deploy", "deploy")
        .expect("register deploy");

    let message = bus
        .send_message(
            "backend",
            &backend.token,
            &["frontend".to_owned()],
            "Users API ready",
            "Artifact: docs/api/users.md",
            Some("TASK-102"),
            "high",
            true,
        )
        .expect("send directed message");

    let frontend_inbox = bus
        .inbox("frontend", &frontend.token, true)
        .expect("frontend inbox");
    assert_eq!(frontend_inbox.len(), 1);
    assert_eq!(frontend_inbox[0].message_id, message.message_id);
    assert_eq!(frontend_inbox[0].sender, "backend");
    assert!(frontend_inbox[0].requires_ack);

    let deploy_inbox = bus
        .inbox("deploy", &deploy.token, false)
        .expect("deploy inbox");
    assert!(
        deploy_inbox.is_empty(),
        "unrelated agent must not see message"
    );

    let unauthorized = bus.inbox("frontend", &deploy.token, false);
    assert!(matches!(unauthorized, Err(BusError::Unauthorized(_))));

    let receipt = bus
        .acknowledge_message("frontend", &frontend.token, &message.message_id)
        .expect("ack message");
    assert!(receipt.ack_at.is_some());
    assert!(
        bus.inbox("frontend", &frontend.token, true)
            .expect("unread inbox after ack")
            .is_empty()
    );
    let all = bus
        .inbox("frontend", &frontend.token, false)
        .expect("full inbox after ack");
    assert!(all[0].read_at.is_some());
    assert!(all[0].ack_at.is_some());
}

#[test]
fn messages_require_ack_before_close_and_closed_items_are_hidden_by_default() {
    let harness = Harness::new();
    let mut bus = harness.bus();
    let sender = bus.register_agent("closer-sender", "coordination").unwrap();
    let recipient = bus
        .register_agent("closer-recipient", "implementation")
        .unwrap();
    let unrelated = bus.register_agent("closer-other", "review").unwrap();

    let required = bus
        .send_message(
            "closer-sender",
            &sender.token,
            std::slice::from_ref(&recipient.name),
            "Approval needed",
            "Acknowledge before closing",
            Some("close/001"),
            "high",
            true,
        )
        .unwrap();
    assert!(matches!(
        bus.close_message("closer-recipient", &recipient.token, &required.message_id),
        Err(BusError::Conflict(_))
    ));
    assert!(matches!(
        bus.close_message("closer-other", &unrelated.token, &required.message_id),
        Err(BusError::Unauthorized(_))
    ));

    let acknowledged = bus
        .acknowledge_message("closer-recipient", &recipient.token, &required.message_id)
        .unwrap();
    assert!(acknowledged.ack_at.is_some());
    let closed = bus
        .close_message("closer-recipient", &recipient.token, &required.message_id)
        .unwrap();
    assert!(closed.closed_at.is_some());
    let closed_retry = bus
        .close_message("closer-recipient", &recipient.token, &required.message_id)
        .unwrap();
    assert_eq!(closed.closed_at, closed_retry.closed_at);
    assert!(matches!(
        bus.mark_read("closer-recipient", &recipient.token, &required.message_id),
        Err(BusError::Conflict(_))
    ));
    assert!(matches!(
        bus.acknowledge_message("closer-recipient", &recipient.token, &required.message_id),
        Err(BusError::Conflict(_))
    ));
    assert!(
        bus.inbox("closer-recipient", &recipient.token, false)
            .unwrap()
            .is_empty()
    );
    let including_closed = bus
        .inbox_with_options("closer-recipient", &recipient.token, false, true)
        .unwrap();
    assert_eq!(including_closed.len(), 1);
    assert_eq!(including_closed[0].closed_at, closed.closed_at);

    let informational = bus
        .send_message(
            "closer-sender",
            &sender.token,
            std::slice::from_ref(&recipient.name),
            "For information",
            "No acknowledgement required",
            None,
            "normal",
            false,
        )
        .unwrap();
    let informational_closed = bus
        .close_message(
            "closer-recipient",
            &recipient.token,
            &informational.message_id,
        )
        .unwrap();
    assert!(informational_closed.closed_at.is_some());
    assert_eq!(
        informational_closed.read_at,
        informational_closed.closed_at.unwrap()
    );

    let closed_events = bus
        .list_events(0, 100, &["message_closed".to_owned()])
        .unwrap();
    assert_eq!(
        closed_events.len(),
        2,
        "close retries must not duplicate events"
    );
}

#[test]
fn task_dependencies_versions_and_unlocking_are_enforced() {
    let harness = Harness::new();
    let mut bus = harness.bus();
    let backend = bus
        .register_agent("backend", "backend")
        .expect("register backend");
    let frontend = bus
        .register_agent("frontend", "frontend")
        .expect("register frontend");

    let dependency = bus
        .create_task(
            "backend",
            &backend.token,
            "TASK-102",
            "Implement users API",
            None,
            &[],
        )
        .expect("create dependency task");
    assert_eq!(dependency.status, "ready");
    assert_eq!(dependency.version, 1);

    let child = bus
        .create_task(
            "frontend",
            &frontend.token,
            "TASK-201",
            "Implement users page",
            None,
            &["TASK-102".to_owned()],
        )
        .expect("create dependent task");
    assert_eq!(child.status, "pending");
    assert!(
        bus.claim_task("frontend", &frontend.token, "TASK-201")
            .is_err()
    );

    let claimed = bus
        .claim_task("backend", &backend.token, "TASK-102")
        .expect("claim dependency");
    assert_eq!(claimed.status, "claimed");
    assert_eq!(claimed.version, 2);
    let working = bus
        .update_task(
            "backend",
            &backend.token,
            "TASK-102",
            claimed.version,
            "working",
            None,
        )
        .expect("start dependency");
    let completed = bus
        .update_task(
            "backend",
            &backend.token,
            "TASK-102",
            working.version,
            "completed",
            None,
        )
        .expect("complete dependency");
    assert_eq!(completed.status, "completed");

    let ready_child = bus.get_task("TASK-201").expect("unlocked child");
    assert_eq!(ready_child.status, "ready");
    assert_eq!(ready_child.version, 2);
    let claimed_child = bus
        .claim_task("frontend", &frontend.token, "TASK-201")
        .expect("claim child");
    assert_eq!(claimed_child.owner.as_deref(), Some("frontend"));

    let stale = bus.update_task(
        "frontend",
        &frontend.token,
        "TASK-201",
        ready_child.version,
        "working",
        None,
    );
    assert!(matches!(stale, Err(BusError::Conflict(_))));
}

#[test]
fn task_thread_bindings_are_owner_scoped_idempotent_and_terminal_safe() {
    let harness = Harness::new();
    let mut bus = harness.bus();
    let owner = bus
        .register_agent("thread-owner", "implementation")
        .unwrap();
    let other = bus.register_agent("thread-other", "review").unwrap();
    bus.create_task(
        "thread-owner",
        &owner.token,
        "THREAD-001",
        "Exercise thread binding",
        None,
        &[],
    )
    .unwrap();
    let claimed = bus
        .claim_task("thread-owner", &owner.token, "THREAD-001")
        .unwrap();

    assert!(matches!(
        bus.bind_task_thread("thread-other", &other.token, "THREAD-001", "codex:other"),
        Err(BusError::Unauthorized(_))
    ));
    assert!(matches!(
        bus.bind_task_thread("thread-owner", &owner.token, "THREAD-001", "bad thread id"),
        Err(BusError::Validation(_))
    ));
    let first = bus
        .bind_task_thread("thread-owner", &owner.token, "THREAD-001", "codex:thread-1")
        .unwrap();
    let first_retry = bus
        .bind_task_thread("thread-owner", &owner.token, "THREAD-001", "codex:thread-1")
        .unwrap();
    assert_eq!(first.binding_id, first_retry.binding_id);
    assert!(matches!(
        bus.bind_task_thread("thread-owner", &owner.token, "THREAD-001", "codex:thread-2"),
        Err(BusError::Conflict(_))
    ));
    let snapshot = bus
        .handoff_snapshot("thread-owner", &owner.token, 0)
        .unwrap();
    assert_eq!(snapshot.task_thread_bindings.len(), 1);

    let unbound = bus
        .unbind_task_thread("thread-owner", &owner.token, "THREAD-001", "codex:thread-1")
        .unwrap();
    let unbound_retry = bus
        .unbind_task_thread("thread-owner", &owner.token, "THREAD-001", "codex:thread-1")
        .unwrap();
    assert_eq!(unbound.binding_id, unbound_retry.binding_id);
    assert_eq!(unbound.unbound_at, unbound_retry.unbound_at);
    let second = bus
        .bind_task_thread("thread-owner", &owner.token, "THREAD-001", "codex:thread-2")
        .unwrap();
    assert_ne!(first.binding_id, second.binding_id);

    let working = bus
        .update_task(
            "thread-owner",
            &owner.token,
            "THREAD-001",
            claimed.version,
            "working",
            None,
        )
        .unwrap();
    bus.update_task(
        "thread-owner",
        &owner.token,
        "THREAD-001",
        working.version,
        "completed",
        None,
    )
    .unwrap();
    assert!(
        bus.list_task_thread_bindings(None, true)
            .unwrap()
            .is_empty()
    );
    let history = bus
        .list_task_thread_bindings(Some("THREAD-001"), false)
        .unwrap();
    assert_eq!(history.len(), 2);
    assert!(history.iter().all(|binding| binding.unbound_at.is_some()));
    assert!(matches!(
        bus.bind_task_thread("thread-owner", &owner.token, "THREAD-001", "codex:thread-3"),
        Err(BusError::Conflict(_))
    ));
}

#[test]
fn concurrent_thread_binding_allows_exactly_one_winner() {
    let harness = Harness::new();
    let mut bus = harness.bus();
    let owner = bus
        .register_agent("thread-racer", "implementation")
        .unwrap();
    bus.create_task(
        "thread-racer",
        &owner.token,
        "THREAD-RACE",
        "Bind once",
        None,
        &[],
    )
    .unwrap();
    bus.claim_task("thread-racer", &owner.token, "THREAD-RACE")
        .unwrap();

    let barrier = Arc::new(Barrier::new(3));
    let handles = ["codex:race-a", "codex:race-b"]
        .into_iter()
        .map(|thread_id| {
            let root = harness.project.path().to_path_buf();
            let data = harness.data.path().to_path_buf();
            let token = owner.token.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                let mut bus = Bus::open(&root, Some(&data)).unwrap();
                barrier.wait();
                bus.bind_task_thread("thread-racer", &token, "THREAD-RACE", thread_id)
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    assert_eq!(bus.list_task_thread_bindings(None, true).unwrap().len(), 1);
}

#[test]
fn concurrent_claim_allows_exactly_one_winner() {
    let harness = Harness::new();
    let mut bus = harness.bus();
    let first = bus
        .register_agent("worker-a", "worker")
        .expect("register worker-a");
    let second = bus
        .register_agent("worker-b", "worker")
        .expect("register worker-b");
    bus.create_task(
        "worker-a",
        &first.token,
        "TASK-RACE",
        "Claim exactly once",
        None,
        &[],
    )
    .expect("create race task");
    drop(bus);

    let root = harness.project.path().to_path_buf();
    let data = harness.data.path().to_path_buf();
    let barrier = Arc::new(Barrier::new(3));
    let handles = [("worker-a", first.token), ("worker-b", second.token)]
        .into_iter()
        .map(|(name, token)| {
            let root = root.clone();
            let data = data.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                let mut bus = Bus::open(&root, Some(&data)).expect("open concurrent bus");
                barrier.wait();
                bus.claim_task(name, &token, "TASK-RACE")
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();

    let results = handles
        .into_iter()
        .map(|handle| handle.join().expect("claim thread"))
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
}

#[test]
fn overlapping_exclusive_reservations_conflict_until_release() {
    let harness = Harness::new();
    let mut bus = harness.bus();
    let frontend = bus
        .register_agent("frontend", "frontend")
        .expect("register frontend");
    let backend = bus
        .register_agent("backend", "backend")
        .expect("register backend");

    let reservation = bus
        .reserve_path(
            "frontend",
            &frontend.token,
            "src/components",
            3600,
            true,
            Some("TASK-201"),
        )
        .expect("reserve component tree");
    let conflict = bus.reserve_path(
        "backend",
        &backend.token,
        "src/components/users",
        3600,
        true,
        Some("TASK-202"),
    );
    assert!(matches!(conflict, Err(BusError::Conflict(_))));

    bus.release_reservation("frontend", &frontend.token, &reservation.reservation_id)
        .expect("release reservation");
    bus.reserve_path(
        "backend",
        &backend.token,
        "src/components/users",
        3600,
        true,
        Some("TASK-202"),
    )
    .expect("reserve after release");
}

#[test]
fn reservations_reject_absolute_paths_and_tasks_require_valid_owner_transitions() {
    let harness = Harness::new();
    let mut bus = harness.bus();
    let first = bus.register_agent("owner", "implementation").unwrap();
    let second = bus.register_agent("observer", "review").unwrap();

    assert!(
        bus.reserve_path(
            "owner",
            &first.token,
            "C:\\Windows\\System32",
            60,
            true,
            None
        )
        .is_err()
    );
    assert!(
        bus.reserve_path("owner", &first.token, "/etc/passwd", 60, true, None)
            .is_err()
    );

    let task = bus
        .create_task(
            "owner",
            &first.token,
            "TASK-STATE",
            "State machine",
            None,
            &[],
        )
        .unwrap();
    assert!(
        bus.update_task(
            "owner",
            &first.token,
            "TASK-STATE",
            task.version,
            "completed",
            None,
        )
        .is_err()
    );
    let claimed = bus.claim_task("owner", &first.token, "TASK-STATE").unwrap();
    assert!(
        bus.update_task(
            "observer",
            &second.token,
            "TASK-STATE",
            claimed.version,
            "working",
            None,
        )
        .is_err()
    );
    let completed = bus
        .update_task(
            "owner",
            &first.token,
            "TASK-STATE",
            claimed.version,
            "completed",
            None,
        )
        .unwrap();
    assert_eq!(completed.status, "completed");
    assert!(
        bus.update_task(
            "owner",
            &first.token,
            "TASK-STATE",
            completed.version,
            "working",
            None,
        )
        .is_err()
    );
}

#[test]
fn artifacts_are_project_scoped_hashed_and_filterable() {
    let harness = Harness::new();
    fs::write(
        harness.project.path().join("result.txt"),
        "durable result\n",
    )
    .unwrap();
    let mut bus = harness.bus();
    let agent = bus.register_agent("publisher", "documentation").unwrap();
    let task = bus
        .create_task(
            "publisher",
            &agent.token,
            "TASK-ART",
            "Publish result",
            None,
            &[],
        )
        .unwrap();

    let artifact = bus
        .publish_artifact(
            "publisher",
            &agent.token,
            "report",
            "result.txt",
            "Verified durable result",
            Some(&task.task_id),
            Some(&serde_json::json!({"format": "text"})),
        )
        .unwrap();
    assert_eq!(artifact.path, "result.txt");
    assert_eq!(artifact.sha256.len(), 64);
    assert_eq!(artifact.metadata["format"], "text");

    let listed = bus.list_artifacts(Some("TASK-ART")).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].artifact_id, artifact.artifact_id);
    assert!(bus.list_artifacts(Some("TASK-OTHER")).unwrap().is_empty());
}

#[test]
fn doctor_reports_healthy_database_and_backup_is_consistent() {
    let harness = Harness::new();
    let mut bus = harness.bus();
    bus.register_agent("backup-agent", "operations").unwrap();

    let doctor = bus.doctor().unwrap();
    assert!(doctor.ok);
    assert_eq!(doctor.integrity, "ok");
    assert_eq!(doctor.journal_mode.to_ascii_lowercase(), "wal");
    assert!(doctor.foreign_keys_enabled);
    assert_eq!(doctor.schema_version, 7);

    let destination = harness.data.path().join("backups").join("snapshot.db");
    let backup = bus.backup_to(&destination).unwrap();
    assert!(destination.is_file());
    assert!(backup.bytes > 0);
    assert_eq!(backup.sha256.len(), 64);
    let copied = Connection::open(&destination).unwrap();
    let integrity: String = copied
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
    assert!(bus.backup_to(&destination).is_err());
}

#[test]
fn recovery_rotates_both_secrets_and_reservations_can_be_renewed_by_owner() {
    let harness = Harness::new();
    let mut bus = harness.bus();
    let owner = bus.register_agent("recoverable", "implementation").unwrap();
    let other = bus.register_agent("other", "review").unwrap();
    assert!(owner.token.starts_with("vbt_"));
    assert!(owner.recovery_key.starts_with("vbr_"));
    assert_eq!(owner.token_generation, 1);

    let reservation = bus
        .reserve_path(
            "recoverable",
            &owner.token,
            "src/recovery.rs",
            60,
            true,
            None,
        )
        .unwrap();
    assert!(
        bus.renew_reservation("other", &other.token, &reservation.reservation_id, 120,)
            .is_err()
    );
    let renewed = bus
        .renew_reservation(
            "recoverable",
            &owner.token,
            &reservation.reservation_id,
            120,
        )
        .unwrap();
    assert!(renewed.expires_at > reservation.expires_at);

    let recovered = bus
        .recover_agent("recoverable", &owner.recovery_key)
        .unwrap();
    assert_eq!(recovered.token_generation, 2);
    assert!(
        bus.inbox("recoverable", &owner.token, false).is_err(),
        "old bearer token must stop working"
    );
    assert!(
        bus.recover_agent("recoverable", &owner.recovery_key)
            .is_err(),
        "old recovery key must be single-use"
    );
    bus.inbox("recoverable", &recovered.token, false).unwrap();

    let provisioned = bus
        .provision_recovery_key("recoverable", &recovered.token)
        .unwrap();
    assert_eq!(provisioned.token_generation, 2);
    assert!(
        bus.recover_agent("recoverable", &recovered.recovery_key)
            .is_err(),
        "provisioning must revoke the previous recovery key"
    );
    let recovered_again = bus
        .recover_agent("recoverable", &provisioned.recovery_key)
        .unwrap();
    assert_eq!(recovered_again.token_generation, 3);
}

#[test]
fn legacy_agent_schema_is_migrated_without_recreating_the_database() {
    let project = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let initialized = initialize_project(project.path(), "legacy", Some(data.path())).unwrap();
    let database_path = std::path::PathBuf::from(initialized.database_path);
    fs::create_dir_all(database_path.parent().unwrap()).unwrap();
    let legacy = Connection::open(&database_path).unwrap();
    legacy
        .execute_batch(
            "CREATE TABLE agents (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                name TEXT NOT NULL,
                role TEXT NOT NULL,
                token_hash TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                last_seen_at INTEGER NOT NULL,
                UNIQUE(project_id, name)
             );
             CREATE TABLE message_receipts (
                message_id TEXT NOT NULL,
                recipient_agent_id TEXT NOT NULL,
                delivered_at INTEGER NOT NULL,
                read_at INTEGER,
                ack_at INTEGER,
                PRIMARY KEY(message_id, recipient_agent_id)
             );",
        )
        .unwrap();
    drop(legacy);

    let bus = Bus::open(project.path(), Some(data.path())).unwrap();
    let connection = Connection::open(bus.database_path()).unwrap();
    let columns: Vec<String> = connection
        .prepare("PRAGMA table_info(agents)")
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert!(columns.contains(&"recovery_hash".to_owned()));
    assert!(columns.contains(&"token_generation".to_owned()));
    let receipt_columns: Vec<String> = connection
        .prepare("PRAGMA table_info(message_receipts)")
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert!(receipt_columns.contains(&"closed_at".to_owned()));
    let subscription_columns: Vec<String> = connection
        .prepare("PRAGMA table_info(subscriptions)")
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert!(subscription_columns.contains(&"pending_delivery_id".to_owned()));
    assert!(subscription_columns.contains(&"pending_from_sequence".to_owned()));
    assert!(subscription_columns.contains(&"pending_through_sequence".to_owned()));
    assert!(subscription_columns.contains(&"pending_created_at".to_owned()));
    assert!(subscription_columns.contains(&"last_acked_delivery_id".to_owned()));
    assert!(subscription_columns.contains(&"last_acked_through_sequence".to_owned()));
    assert!(subscription_columns.contains(&"last_acked_at".to_owned()));
    let thread_binding_columns: Vec<String> = connection
        .prepare("PRAGMA table_info(task_thread_bindings)")
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert!(thread_binding_columns.contains(&"thread_id".to_owned()));
    assert!(thread_binding_columns.contains(&"unbound_at".to_owned()));
}

#[test]
fn idempotency_keys_deduplicate_retries_and_reject_payload_reuse() {
    let harness = Harness::new();
    fs::write(
        harness.project.path().join("idempotent.txt"),
        "same content\n",
    )
    .unwrap();
    let mut bus = harness.bus();
    let sender = bus.register_agent("idem-sender", "coordination").unwrap();
    let recipient = bus
        .register_agent("idem-recipient", "implementation")
        .unwrap();

    let barrier = Arc::new(Barrier::new(3));
    let handles = (0..2)
        .map(|_| {
            let root = harness.project.path().to_path_buf();
            let data = harness.data.path().to_path_buf();
            let token = sender.token.clone();
            let recipient = recipient.name.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                let mut bus = Bus::open(&root, Some(&data)).unwrap();
                barrier.wait();
                bus.send_message_idempotent(
                    "idem-sender",
                    &token,
                    &[recipient],
                    "Retry-safe",
                    "Only one message should exist",
                    None,
                    "normal",
                    false,
                    Some("send:001"),
                )
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results[0].message_id, results[1].message_id);
    assert_eq!(
        bus.inbox("idem-recipient", &recipient.token, false)
            .unwrap()
            .len(),
        1
    );
    assert!(
        bus.send_message_idempotent(
            "idem-sender",
            &sender.token,
            std::slice::from_ref(&recipient.name),
            "Retry-safe",
            "Different payload",
            None,
            "normal",
            false,
            Some("send:001"),
        )
        .is_err()
    );

    let reserved = bus
        .reserve_path_idempotent(
            "idem-sender",
            &sender.token,
            "src/idempotent",
            60,
            true,
            None,
            Some("reserve:001"),
        )
        .unwrap();
    let reserved_retry = bus
        .reserve_path_idempotent(
            "idem-sender",
            &sender.token,
            "src/idempotent",
            60,
            true,
            None,
            Some("reserve:001"),
        )
        .unwrap();
    assert_eq!(reserved.reservation_id, reserved_retry.reservation_id);

    let renewed = bus
        .renew_reservation_idempotent(
            "idem-sender",
            &sender.token,
            &reserved.reservation_id,
            120,
            Some("renew:001"),
        )
        .unwrap();
    let renewed_retry = bus
        .renew_reservation_idempotent(
            "idem-sender",
            &sender.token,
            &reserved.reservation_id,
            120,
            Some("renew:001"),
        )
        .unwrap();
    assert_eq!(renewed.expires_at, renewed_retry.expires_at);

    let artifact = bus
        .publish_artifact_idempotent(
            "idem-sender",
            &sender.token,
            "test",
            "idempotent.txt",
            "Retry-safe artifact",
            None,
            None,
            Some("artifact:001"),
        )
        .unwrap();
    let artifact_retry = bus
        .publish_artifact_idempotent(
            "idem-sender",
            &sender.token,
            "test",
            "idempotent.txt",
            "Retry-safe artifact",
            None,
            None,
            Some("artifact:001"),
        )
        .unwrap();
    assert_eq!(artifact.artifact_id, artifact_retry.artifact_id);
    assert_eq!(bus.list_artifacts(None).unwrap().len(), 1);
}

#[test]
fn project_identity_is_shared_from_nested_working_directories_and_uses_wal() {
    let harness = Harness::new();
    let nested = harness.project.path().join("nested").join("worktree-view");
    fs::create_dir_all(&nested).expect("create nested path");
    let bus =
        Bus::open(&nested, Some(harness.data.path())).expect("discover project from nested path");
    assert_eq!(
        bus.project_root(),
        harness
            .project
            .path()
            .canonicalize()
            .expect("canonical project")
    );
    assert!(bus.database_path().starts_with(harness.data.path()));

    let connection = Connection::open(bus.database_path()).expect("open database directly");
    let journal_mode: String = connection
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .expect("read journal mode");
    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
}

#[test]
fn event_subscriptions_filter_and_advance_a_durable_cursor() {
    let harness = Harness::new();
    let mut bus = harness.bus();
    let sender = bus.register_agent("event-sender", "coordination").unwrap();
    let receiver = bus
        .register_agent("event-receiver", "implementation")
        .unwrap();

    let subscription = bus
        .create_subscription(
            "event-receiver",
            &receiver.token,
            "messages",
            &["message_sent".to_owned()],
            Some(0),
        )
        .unwrap();
    assert_eq!(subscription.cursor_sequence, 0);

    let message = bus
        .send_message(
            "event-sender",
            &sender.token,
            &["event-receiver".to_owned()],
            "Event-backed message",
            "Consume exactly once per durable cursor",
            None,
            "normal",
            false,
        )
        .unwrap();
    let polled = bus
        .poll_subscription("event-receiver", &receiver.token, "messages", 10)
        .unwrap();
    assert_eq!(polled.events.len(), 1);
    assert_eq!(polled.events[0].event_type, "message_sent");
    assert_eq!(polled.events[0].entity_id, message.message_id);
    assert!(polled.scanned_through_sequence > 0);

    let empty = bus
        .poll_subscription("event-receiver", &receiver.token, "messages", 10)
        .unwrap();
    assert!(empty.events.is_empty());
    assert!(empty.scanned_through_sequence >= polled.scanned_through_sequence);

    let listed = bus
        .list_events(0, 10, &["message_sent".to_owned()])
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].entity_id, message.message_id);
    assert!(bus.list_events(-1, 10, &[]).is_err());

    let subscriptions = bus
        .list_subscriptions("event-receiver", &receiver.token)
        .unwrap();
    assert_eq!(subscriptions.len(), 1);
    assert_eq!(subscriptions[0].name, "messages");
}

#[test]
fn subscription_peek_replays_until_idempotent_ack_and_blocks_legacy_poll() {
    let harness = Harness::new();
    let mut bus = harness.bus();
    let sender = bus.register_agent("peek-sender", "coordination").unwrap();
    let receiver = bus
        .register_agent("peek-receiver", "implementation")
        .unwrap();
    bus.create_subscription(
        "peek-receiver",
        &receiver.token,
        "replay-safe",
        &["message_sent".to_owned()],
        Some(0),
    )
    .unwrap();

    let first = bus
        .send_message(
            "peek-sender",
            &sender.token,
            &["peek-receiver".to_owned()],
            "First",
            "First replay-safe message",
            None,
            "normal",
            false,
        )
        .unwrap();
    bus.send_message(
        "peek-sender",
        &sender.token,
        &["peek-receiver".to_owned()],
        "Second",
        "Second replay-safe message",
        None,
        "normal",
        false,
    )
    .unwrap();

    let peeked = bus
        .peek_subscription("peek-receiver", &receiver.token, "replay-safe", 1)
        .unwrap();
    let delivery = peeked.delivery.clone().expect("pending delivery");
    assert_eq!(peeked.events.len(), 1);
    assert_eq!(peeked.events[0].entity_id, first.message_id);
    assert_eq!(
        peeked
            .subscription
            .pending_delivery
            .as_ref()
            .unwrap()
            .delivery_id,
        delivery.delivery_id
    );
    assert_eq!(peeked.subscription.cursor_sequence, 0);

    bus.send_message(
        "peek-sender",
        &sender.token,
        &["peek-receiver".to_owned()],
        "Third",
        "Created while the first delivery is pending",
        None,
        "normal",
        false,
    )
    .unwrap();
    let replayed = bus
        .peek_subscription("peek-receiver", &receiver.token, "replay-safe", 500)
        .unwrap();
    assert_eq!(
        replayed.delivery.as_ref().unwrap().delivery_id,
        delivery.delivery_id
    );
    assert_eq!(replayed.events.len(), 1);
    assert_eq!(replayed.events[0].entity_id, first.message_id);
    assert!(
        bus.poll_subscription("peek-receiver", &receiver.token, "replay-safe", 10)
            .is_err(),
        "legacy consume-on-poll must not cross a pending replay-safe delivery"
    );
    assert!(
        bus.acknowledge_subscription("peek-receiver", &receiver.token, "replay-safe", "sdl_wrong",)
            .is_err()
    );

    let acknowledged = bus
        .acknowledge_subscription(
            "peek-receiver",
            &receiver.token,
            "replay-safe",
            &delivery.delivery_id,
        )
        .unwrap();
    assert!(!acknowledged.replayed);
    assert_eq!(acknowledged.cursor_sequence, delivery.through_sequence);
    let acknowledged_retry = bus
        .acknowledge_subscription(
            "peek-receiver",
            &receiver.token,
            "replay-safe",
            &delivery.delivery_id,
        )
        .unwrap();
    assert!(acknowledged_retry.replayed);
    assert_eq!(
        acknowledged_retry.acknowledged_at,
        acknowledged.acknowledged_at
    );

    let next = bus
        .peek_subscription("peek-receiver", &receiver.token, "replay-safe", 10)
        .unwrap();
    let next_delivery = next.delivery.clone().unwrap();
    assert_eq!(next.events.len(), 2);
    assert!(
        next.events
            .iter()
            .all(|event| event.event_type == "message_sent")
    );
    bus.acknowledge_subscription(
        "peek-receiver",
        &receiver.token,
        "replay-safe",
        &next_delivery.delivery_id,
    )
    .unwrap();
    let caught_up = bus
        .peek_subscription("peek-receiver", &receiver.token, "replay-safe", 10)
        .unwrap();
    assert!(caught_up.delivery.is_none());
    assert!(caught_up.events.is_empty());

    bus.create_subscription(
        "peek-receiver",
        &receiver.token,
        "filtered-empty",
        &["artifact_published".to_owned()],
        Some(0),
    )
    .unwrap();
    let empty = bus
        .peek_subscription("peek-receiver", &receiver.token, "filtered-empty", 10)
        .unwrap();
    let empty_delivery = empty.delivery.clone().unwrap();
    assert!(empty.events.is_empty());
    assert!(empty_delivery.through_sequence > empty_delivery.from_sequence);
    let empty_replay = bus
        .peek_subscription("peek-receiver", &receiver.token, "filtered-empty", 10)
        .unwrap();
    assert_eq!(
        empty_replay.delivery.unwrap().delivery_id,
        empty_delivery.delivery_id
    );
    bus.acknowledge_subscription(
        "peek-receiver",
        &receiver.token,
        "filtered-empty",
        &empty_delivery.delivery_id,
    )
    .unwrap();
}

#[test]
fn concurrent_subscription_peek_and_ack_converge_on_one_delivery() {
    let harness = Harness::new();
    let mut bus = harness.bus();
    let sender = bus
        .register_agent("concurrent-peek-sender", "coordination")
        .unwrap();
    let receiver = bus
        .register_agent("concurrent-peek-receiver", "implementation")
        .unwrap();
    bus.create_subscription(
        "concurrent-peek-receiver",
        &receiver.token,
        "concurrent",
        &["message_sent".to_owned()],
        Some(0),
    )
    .unwrap();
    bus.send_message(
        "concurrent-peek-sender",
        &sender.token,
        &["concurrent-peek-receiver".to_owned()],
        "Concurrent delivery",
        "Both peekers must receive one delivery identity",
        None,
        "normal",
        false,
    )
    .unwrap();
    drop(bus);

    let root = harness.project.path().to_path_buf();
    let data = harness.data.path().to_path_buf();
    let barrier = Arc::new(Barrier::new(3));
    let peek_handles = (0..2)
        .map(|_| {
            let root = root.clone();
            let data = data.clone();
            let token = receiver.token.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                let mut bus = Bus::open(&root, Some(&data)).unwrap();
                barrier.wait();
                bus.peek_subscription("concurrent-peek-receiver", &token, "concurrent", 10)
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let peeks = peek_handles
        .into_iter()
        .map(|handle| handle.join().unwrap().unwrap())
        .collect::<Vec<_>>();
    let delivery_id = peeks[0].delivery.as_ref().unwrap().delivery_id.clone();
    assert_eq!(peeks[1].delivery.as_ref().unwrap().delivery_id, delivery_id);
    assert_eq!(peeks[0].events.len(), 1);
    assert_eq!(peeks[1].events.len(), 1);

    let barrier = Arc::new(Barrier::new(3));
    let ack_handles = (0..2)
        .map(|_| {
            let root = root.clone();
            let data = data.clone();
            let token = receiver.token.clone();
            let delivery_id = delivery_id.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                let mut bus = Bus::open(&root, Some(&data)).unwrap();
                barrier.wait();
                bus.acknowledge_subscription(
                    "concurrent-peek-receiver",
                    &token,
                    "concurrent",
                    &delivery_id,
                )
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let acknowledgements = ack_handles
        .into_iter()
        .map(|handle| handle.join().unwrap().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        acknowledgements[0].cursor_sequence,
        acknowledgements[1].cursor_sequence
    );
    assert_eq!(
        acknowledgements.iter().filter(|ack| ack.replayed).count(),
        1
    );
}

#[test]
fn structured_handoff_is_retry_safe_acknowledged_and_visible_in_snapshot() {
    let harness = Harness::new();
    fs::write(
        harness.project.path().join("handoff.md"),
        "# Verified handoff\n",
    )
    .unwrap();
    let mut bus = harness.bus();
    let sender = bus
        .register_agent("handoff-sender", "coordination")
        .unwrap();
    let receiver = bus
        .register_agent("handoff-receiver", "implementation")
        .unwrap();
    let task = bus
        .create_task(
            "handoff-sender",
            &sender.token,
            "TASK-HANDOFF",
            "Resume durable work",
            None,
            &[],
        )
        .unwrap();
    let artifact = bus
        .publish_artifact(
            "handoff-sender",
            &sender.token,
            "handoff",
            "handoff.md",
            "Verified handoff notes",
            Some(&task.task_id),
            None,
        )
        .unwrap();

    let sent = bus
        .send_handoff(
            "handoff-sender",
            &sender.token,
            &["handoff-receiver".to_owned()],
            "Continue the durable coordination slice",
            Some(&task.task_id),
            &["Use sequence cursors".to_owned()],
            std::slice::from_ref(&artifact.artifact_id),
            &[],
            &["Claim TASK-HANDOFF".to_owned()],
            Some("handoff:001"),
        )
        .unwrap();
    let retried = bus
        .send_handoff(
            "handoff-sender",
            &sender.token,
            &["handoff-receiver".to_owned()],
            "Continue the durable coordination slice",
            Some(&task.task_id),
            &["Use sequence cursors".to_owned()],
            std::slice::from_ref(&artifact.artifact_id),
            &[],
            &["Claim TASK-HANDOFF".to_owned()],
            Some("handoff:001"),
        )
        .unwrap();
    assert_eq!(sent.message_id, retried.message_id);
    assert_eq!(sent.priority, "high");
    assert!(sent.requires_ack);
    let body: serde_json::Value = serde_json::from_str(&sent.body).unwrap();
    assert_eq!(body["kind"], "handoff");
    assert_eq!(body["taskId"], "TASK-HANDOFF");
    assert_eq!(body["artifacts"][0], artifact.artifact_id);

    let snapshot = bus
        .handoff_snapshot("handoff-receiver", &receiver.token, 0)
        .unwrap();
    assert_eq!(snapshot.unread_messages.len(), 1);
    assert_eq!(snapshot.unread_messages[0].message_id, sent.message_id);
    assert!(snapshot.latest_event_sequence > 0);
    assert!(!snapshot.recent_events.is_empty());

    bus.acknowledge_message("handoff-receiver", &receiver.token, &sent.message_id)
        .unwrap();
    let after_ack = bus
        .handoff_snapshot("handoff-receiver", &receiver.token, 0)
        .unwrap();
    assert!(after_ack.unread_messages.is_empty());
}
