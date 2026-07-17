use std::fs;
use std::process::Command;

use serde_json::{Value, json};
use tempfile::TempDir;
use vibebus::initialize_project;

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
