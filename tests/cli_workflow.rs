use std::fs;
use std::process::Command;

use serde_json::{Value, json};
use tempfile::TempDir;
use vibebus::{Bus, RetentionPolicy, initialize_project};

#[test]
fn cli_accepts_metadata_file_on_windows_safe_path() {
    let project = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    initialize_project(project.path(), "CLI test", Some(data.path())).unwrap();
    fs::write(project.path().join("artifact.txt"), "CLI artifact\n").unwrap();
    fs::write(
        project.path().join("metadata.json"),
        serde_json::to_vec_pretty(&json!({"platform": "windows", "version": 1})).unwrap(),
    )
    .unwrap();

    let registration = run_cli(
        project.path(),
        data.path(),
        &["register", "--name", "cli-agent", "--role", "test"],
    );
    let token = registration["result"]["token"].as_str().unwrap();
    let published = run_cli(
        project.path(),
        data.path(),
        &[
            "artifact",
            "publish",
            "--agent",
            "cli-agent",
            "--token",
            token,
            "--kind",
            "test",
            "--path",
            "artifact.txt",
            "--summary",
            "CLI metadata file",
            "--metadata-file",
            project.path().join("metadata.json").to_str().unwrap(),
        ],
    );
    assert_eq!(published["result"]["metadata"]["platform"], "windows");
    assert_eq!(published["result"]["metadata"]["version"], 1);
}

#[test]
fn cli_context_sync_matches_the_core_projection() {
    let project = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    initialize_project(project.path(), "CLI context", Some(data.path())).unwrap();
    fs::write(
        project.path().join("context.txt"),
        "large report stays external\n",
    )
    .unwrap();

    let registration = run_cli(
        project.path(),
        data.path(),
        &["register", "--name", "cli-context", "--role", "test"],
    );
    let token = registration["result"]["token"].as_str().unwrap();
    run_cli(
        project.path(),
        data.path(),
        &[
            "task",
            "create",
            "--agent",
            "cli-context",
            "--token",
            token,
            "--id",
            "CLI-CONTEXT-001",
            "--title",
            "CLI context task",
        ],
    );
    run_cli(
        project.path(),
        data.path(),
        &[
            "task",
            "claim",
            "--agent",
            "cli-context",
            "--token",
            token,
            "--id",
            "CLI-CONTEXT-001",
        ],
    );
    let artifact = run_cli(
        project.path(),
        data.path(),
        &[
            "artifact",
            "publish",
            "--agent",
            "cli-context",
            "--token",
            token,
            "--kind",
            "report",
            "--path",
            "context.txt",
            "--summary",
            "CLI context artifact",
            "--task",
            "CLI-CONTEXT-001",
        ],
    );
    let artifact_id = artifact["result"]["artifactId"].as_str().unwrap();
    let confirmed = run_cli(
        project.path(),
        data.path(),
        &[
            "decision",
            "confirm",
            "--agent",
            "cli-context",
            "--token",
            token,
            "--key",
            "cli.context",
            "--task",
            "CLI-CONTEXT-001",
            "--summary",
            "CLI and MCP share one projection.",
            "--artifacts",
            artifact_id,
            "--idempotency-key",
            "cli-context-decision",
        ],
    );
    let replayed = run_cli(
        project.path(),
        data.path(),
        &[
            "decision",
            "confirm",
            "--agent",
            "cli-context",
            "--token",
            token,
            "--key",
            "cli.context",
            "--task",
            "CLI-CONTEXT-001",
            "--summary",
            "CLI and MCP share one projection.",
            "--artifacts",
            artifact_id,
            "--idempotency-key",
            "cli-context-decision",
        ],
    );
    assert_eq!(
        confirmed["result"]["decisionId"],
        replayed["result"]["decisionId"]
    );

    let projected = run_cli(
        project.path(),
        data.path(),
        &[
            "context",
            "sync",
            "--agent",
            "cli-context",
            "--token",
            token,
            "--item-limit",
            "500",
            "--byte-budget",
            "1048576",
        ],
    );
    let bus = Bus::open(project.path(), Some(data.path())).unwrap();
    let direct = bus
        .context_sync("cli-context", token, None, 500, 1_048_576)
        .unwrap();
    assert_eq!(projected["result"], serde_json::to_value(direct).unwrap());
    assert!(
        projected["result"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |item| item["kind"] == "confirmedDecision" && item["value"]["key"] == "cli.context"
            )
    );
}

#[test]
fn cli_exposes_subscription_and_structured_handoff_workflows() {
    let project = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    initialize_project(project.path(), "CLI coordination", Some(data.path())).unwrap();

    let sender = run_cli(
        project.path(),
        data.path(),
        &["register", "--name", "cli-sender", "--role", "lead"],
    );
    let receiver = run_cli(
        project.path(),
        data.path(),
        &[
            "register",
            "--name",
            "cli-receiver",
            "--role",
            "implementation",
        ],
    );
    let sender_token = sender["result"]["token"].as_str().unwrap();
    let receiver_token = receiver["result"]["token"].as_str().unwrap();

    let subscription = run_cli(
        project.path(),
        data.path(),
        &[
            "subscription",
            "create",
            "--agent",
            "cli-receiver",
            "--token",
            receiver_token,
            "--name",
            "handoffs",
            "--event-types",
            "message_sent",
            "--from-sequence",
            "0",
        ],
    );
    assert_eq!(subscription["result"]["name"], "handoffs");

    let handoff = run_cli(
        project.path(),
        data.path(),
        &[
            "handoff",
            "send",
            "--from",
            "cli-sender",
            "--token",
            sender_token,
            "--to",
            "cli-receiver",
            "--summary",
            "Resume from the CLI",
            "--decisions",
            "Use durable cursors",
            "--next-actions",
            "Poll and acknowledge",
            "--idempotency-key",
            "cli-handoff:001",
        ],
    );
    assert_eq!(handoff["result"]["priority"], "high");
    assert_eq!(handoff["result"]["requiresAck"], true);

    let peeked = run_cli(
        project.path(),
        data.path(),
        &[
            "subscription",
            "peek",
            "--agent",
            "cli-receiver",
            "--token",
            receiver_token,
            "--name",
            "handoffs",
        ],
    );
    assert_eq!(peeked["result"]["events"].as_array().unwrap().len(), 1);
    let delivery_id = peeked["result"]["delivery"]["deliveryId"].as_str().unwrap();
    let replayed = run_cli(
        project.path(),
        data.path(),
        &[
            "subscription",
            "peek",
            "--agent",
            "cli-receiver",
            "--token",
            receiver_token,
            "--name",
            "handoffs",
        ],
    );
    assert_eq!(replayed["result"]["delivery"]["deliveryId"], delivery_id);
    let acknowledged = run_cli(
        project.path(),
        data.path(),
        &[
            "subscription",
            "ack",
            "--agent",
            "cli-receiver",
            "--token",
            receiver_token,
            "--name",
            "handoffs",
            "--delivery",
            delivery_id,
        ],
    );
    assert_eq!(acknowledged["result"]["replayed"], false);
    let acknowledged_retry = run_cli(
        project.path(),
        data.path(),
        &[
            "subscription",
            "ack",
            "--agent",
            "cli-receiver",
            "--token",
            receiver_token,
            "--name",
            "handoffs",
            "--delivery",
            delivery_id,
        ],
    );
    assert_eq!(acknowledged_retry["result"]["replayed"], true);

    let snapshot = run_cli(
        project.path(),
        data.path(),
        &[
            "handoff",
            "snapshot",
            "--agent",
            "cli-receiver",
            "--token",
            receiver_token,
        ],
    );
    assert_eq!(
        snapshot["result"]["unreadMessages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn cli_exposes_message_close_and_task_thread_bindings() {
    let project = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    initialize_project(project.path(), "CLI lifecycle", Some(data.path())).unwrap();

    let sender = run_cli(
        project.path(),
        data.path(),
        &["register", "--name", "lifecycle-sender", "--role", "lead"],
    );
    let owner = run_cli(
        project.path(),
        data.path(),
        &[
            "register",
            "--name",
            "lifecycle-owner",
            "--role",
            "implementation",
        ],
    );
    let sender_token = sender["result"]["token"].as_str().unwrap();
    let owner_token = owner["result"]["token"].as_str().unwrap();

    run_cli(
        project.path(),
        data.path(),
        &[
            "task",
            "create",
            "--agent",
            "lifecycle-owner",
            "--token",
            owner_token,
            "--id",
            "CLI-THREAD",
            "--title",
            "Bind this task",
        ],
    );
    run_cli(
        project.path(),
        data.path(),
        &[
            "task",
            "claim",
            "--agent",
            "lifecycle-owner",
            "--token",
            owner_token,
            "--id",
            "CLI-THREAD",
        ],
    );
    let bound = run_cli(
        project.path(),
        data.path(),
        &[
            "thread",
            "bind",
            "--agent",
            "lifecycle-owner",
            "--token",
            owner_token,
            "--task",
            "CLI-THREAD",
            "--thread",
            "codex:cli-thread",
        ],
    );
    assert_eq!(bound["result"]["threadId"], "codex:cli-thread");
    let status = run_cli(project.path(), data.path(), &["status"]);
    assert_eq!(
        status["result"]["threadBindings"].as_array().unwrap().len(),
        1
    );

    let sent = run_cli(
        project.path(),
        data.path(),
        &[
            "send",
            "--from",
            "lifecycle-sender",
            "--token",
            sender_token,
            "--to",
            "lifecycle-owner",
            "--subject",
            "Close me",
            "--body",
            "ACK then close",
            "--requires-ack",
        ],
    );
    let message_id = sent["result"]["messageId"].as_str().unwrap();
    run_cli(
        project.path(),
        data.path(),
        &[
            "ack",
            "--agent",
            "lifecycle-owner",
            "--token",
            owner_token,
            "--message",
            message_id,
        ],
    );
    let closed = run_cli(
        project.path(),
        data.path(),
        &[
            "close",
            "--agent",
            "lifecycle-owner",
            "--token",
            owner_token,
            "--message",
            message_id,
        ],
    );
    assert!(closed["result"]["closedAt"].as_i64().is_some());
    let default_inbox = run_cli(
        project.path(),
        data.path(),
        &[
            "inbox",
            "--agent",
            "lifecycle-owner",
            "--token",
            owner_token,
            "--all",
        ],
    );
    assert!(default_inbox["result"].as_array().unwrap().is_empty());
    let closed_inbox = run_cli(
        project.path(),
        data.path(),
        &[
            "inbox",
            "--agent",
            "lifecycle-owner",
            "--token",
            owner_token,
            "--all",
            "--include-closed",
        ],
    );
    assert_eq!(closed_inbox["result"].as_array().unwrap().len(), 1);

    let unbound = run_cli(
        project.path(),
        data.path(),
        &[
            "thread",
            "unbind",
            "--agent",
            "lifecycle-owner",
            "--token",
            owner_token,
            "--task",
            "CLI-THREAD",
            "--thread",
            "codex:cli-thread",
        ],
    );
    assert!(unbound["result"]["unboundAt"].as_i64().is_some());
}

#[test]
fn cli_requires_a_confirmed_retention_plan_and_replays_apply() {
    let project = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    initialize_project(project.path(), "CLI retention", Some(data.path())).unwrap();
    let owner = run_cli(
        project.path(),
        data.path(),
        &[
            "register",
            "--name",
            "retention-cli",
            "--role",
            "operations",
        ],
    );
    let token = owner["result"]["token"].as_str().unwrap();
    let plan = run_cli(
        project.path(),
        data.path(),
        &[
            "retention",
            "plan",
            "--agent",
            "retention-cli",
            "--token",
            token,
            "--event-max-age-days",
            "1",
            "--keep-recent-events",
            "1",
            "--idempotency-max-age-days",
            "1",
            "--closed-message-max-age-days",
            "1",
            "--terminal-binding-max-age-days",
            "1",
        ],
    );
    let plan_id = plan["result"]["planId"].as_str().unwrap();
    assert!(plan_id.starts_with("rtp_"));
    let policy = RetentionPolicy {
        event_max_age_days: 1,
        keep_recent_events: 1,
        idempotency_max_age_days: 1,
        closed_message_max_age_days: 1,
        terminal_binding_max_age_days: 1,
    };
    let mut bus = Bus::open(project.path(), Some(data.path())).unwrap();
    let operator = bus.initialize_operator().unwrap();
    bus.approve_retention(&operator.operator_secret, &policy, plan_id, 600)
        .unwrap();
    drop(bus);
    let applied = run_cli(
        project.path(),
        data.path(),
        &[
            "retention",
            "apply",
            "--agent",
            "retention-cli",
            "--token",
            token,
            "--plan",
            plan_id,
            "--event-max-age-days",
            "1",
            "--keep-recent-events",
            "1",
            "--idempotency-max-age-days",
            "1",
            "--closed-message-max-age-days",
            "1",
            "--terminal-binding-max-age-days",
            "1",
        ],
    );
    assert_eq!(applied["result"]["replayed"], false);
    let replayed = run_cli(
        project.path(),
        data.path(),
        &[
            "retention",
            "apply",
            "--agent",
            "retention-cli",
            "--token",
            token,
            "--plan",
            plan_id,
            "--event-max-age-days",
            "1",
            "--keep-recent-events",
            "1",
            "--idempotency-max-age-days",
            "1",
            "--closed-message-max-age-days",
            "1",
            "--terminal-binding-max-age-days",
            "1",
        ],
    );
    assert_eq!(replayed["result"]["replayed"], true);
    assert_eq!(
        replayed["result"]["appliedAt"],
        applied["result"]["appliedAt"]
    );
    let status = run_cli(project.path(), data.path(), &["retention", "status"]);
    assert_eq!(status["result"]["lastPlanId"], plan_id);
}

#[test]
fn cli_exposes_responsibility_facts_and_bounded_handoff_proposals() {
    let project = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    initialize_project(project.path(), "CLI responsibility", Some(data.path())).unwrap();
    fs::create_dir_all(project.path().join(".vibebus")).unwrap();
    fs::write(
        project.path().join(".vibebus/responsibility.json"),
        serde_json::to_vec_pretty(&json!({
            "version": 1,
            "defaultAllowedPaths": [],
            "roles": {"implementation": {"allowedPaths": ["src/**"]}}
        }))
        .unwrap(),
    )
    .unwrap();

    let registration = run_cli(
        project.path(),
        data.path(),
        &[
            "register",
            "--name",
            "cli-policy",
            "--role",
            "implementation",
        ],
    );
    let token = registration["result"]["token"].as_str().unwrap();
    run_cli(
        project.path(),
        data.path(),
        &[
            "task",
            "create",
            "--agent",
            "cli-policy",
            "--token",
            token,
            "--id",
            "CLI-POLICY-001",
            "--title",
            "Exercise policy facts",
        ],
    );
    run_cli(
        project.path(),
        data.path(),
        &[
            "task",
            "claim",
            "--agent",
            "cli-policy",
            "--token",
            token,
            "--id",
            "CLI-POLICY-001",
        ],
    );

    let inspected = run_cli(
        project.path(),
        data.path(),
        &[
            "responsibility",
            "inspect",
            "--agent",
            "cli-policy",
            "--token",
            token,
        ],
    );
    assert_eq!(inspected["result"]["configured"], true);
    assert_eq!(inspected["result"]["allowedPaths"][0], "src/**");

    let granted = run_cli(
        project.path(),
        data.path(),
        &[
            "responsibility",
            "override",
            "--agent",
            "cli-policy",
            "--token",
            token,
            "--task",
            "CLI-POLICY-001",
            "--grantee",
            "cli-policy",
            "--path",
            "docs/**",
            "--reason",
            "Document this task",
            "--ttl",
            "600",
            "--idempotency-key",
            "cli-policy-override",
        ],
    );
    assert_eq!(granted["result"]["pathPattern"], "docs/**");

    let commit = run_cli(
        project.path(),
        data.path(),
        &[
            "fact",
            "git-commit",
            "--agent",
            "cli-policy",
            "--token",
            token,
            "--task",
            "CLI-POLICY-001",
            "--commit-sha",
            "0123456789abcdef0123456789abcdef01234567",
            "--summary",
            "Add responsibility documentation",
            "--changed-path",
            "docs/policy.md",
            "--idempotency-key",
            "cli-policy-commit",
        ],
    );
    assert_eq!(commit["result"]["changedPaths"][0], "docs/policy.md");

    let result = run_cli(
        project.path(),
        data.path(),
        &[
            "fact",
            "test-result",
            "--agent",
            "cli-policy",
            "--token",
            token,
            "--task",
            "CLI-POLICY-001",
            "--result-key",
            "cargo-test-head",
            "--suite",
            "cargo test",
            "--outcome",
            "passed",
            "--summary",
            "All tests passed",
            "--command",
            "cargo test --all-targets --locked",
            "--idempotency-key",
            "cli-policy-test",
        ],
    );
    assert_eq!(result["result"]["outcome"], "passed");

    let proposal = run_cli(
        project.path(),
        data.path(),
        &[
            "handoff",
            "propose",
            "--agent",
            "cli-policy",
            "--token",
            token,
            "--task",
            "CLI-POLICY-001",
            "--item-limit",
            "5",
        ],
    );
    assert_eq!(proposal["result"]["task"]["taskId"], "CLI-POLICY-001");
    assert_eq!(
        proposal["result"]["gitCommits"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        proposal["result"]["testResults"].as_array().unwrap().len(),
        1
    );
}

#[test]
fn operator_mutations_reject_redirected_noninteractive_cli_calls() {
    let project = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    initialize_project(project.path(), "CLI operator", Some(data.path())).unwrap();
    for command in ["init", "delete-credential"] {
        let output = Command::new(env!("CARGO_BIN_EXE_vibebus"))
            .arg("--root")
            .arg(project.path())
            .arg("--data-home")
            .arg(data.path())
            .args(["operator", command])
            .output()
            .unwrap();
        assert!(!output.status.success());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["kind"], "validation");
        assert!(
            error["error"]
                .as_str()
                .unwrap()
                .contains("interactive terminal")
        );
    }
    let bus = Bus::open(project.path(), Some(data.path())).unwrap();
    assert!(!bus.operator_status().unwrap().configured);
}

fn run_cli(project: &std::path::Path, data: &std::path::Path, args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_vibebus"))
        .arg("--root")
        .arg(project)
        .arg("--data-home")
        .arg(data)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
