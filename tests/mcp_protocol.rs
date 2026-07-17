use std::{path::PathBuf, sync::Arc};

use rmcp::ServiceExt;
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use vibebus::{MemoryCredentialVault, initialize_project, mcp::VibeBusMcp};

#[tokio::test(flavor = "current_thread")]
async fn mcp_negotiates_lists_tools_and_calls_status() {
    let project = TempDir::new().unwrap();
    let data_home = TempDir::new().unwrap();
    initialize_project(project.path(), "MCP test", Some(data_home.path())).unwrap();

    let (client_stream, server_stream) = tokio::io::duplex(128 * 1024);
    let project_root = project.path().to_path_buf();
    let server_data_home = data_home.path().to_path_buf();
    let vault = Arc::new(MemoryCredentialVault::default());
    let server_task = tokio::spawn(async move {
        let service = VibeBusMcp::with_vault(project_root, Some(server_data_home), vault)
            .serve(server_stream)
            .await
            .unwrap();
        service.waiting().await.unwrap();
    });

    let (reader, mut writer) = tokio::io::split(client_stream);
    let mut lines = BufReader::new(reader).lines();

    send(
        &mut writer,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "vibebus-test", "version": "0.1.0"}
            }
        }),
    )
    .await;
    let initialized = response(&mut lines, 1).await;
    assert_eq!(initialized["result"]["serverInfo"]["name"], "vibebus");

    send(
        &mut writer,
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }),
    )
    .await;
    send(
        &mut writer,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
    )
    .await;
    let listed = response(&mut lines, 2).await;
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert!(names.contains(&"vibebus_status"));
    assert!(names.contains(&"vibebus_task_claim"));
    assert!(names.contains(&"vibebus_reserve"));
    assert!(names.contains(&"vibebus_agent_recover"));
    assert!(names.contains(&"vibebus_reservation_renew"));
    assert!(names.contains(&"vibebus_events"));
    assert!(names.contains(&"vibebus_subscription_poll"));
    assert!(names.contains(&"vibebus_subscription_peek"));
    assert!(names.contains(&"vibebus_subscription_ack"));
    assert!(names.contains(&"vibebus_handoff_send"));
    assert!(names.contains(&"vibebus_handoff_snapshot"));
    assert!(names.contains(&"vibebus_close"));
    assert!(names.contains(&"vibebus_thread_bind"));
    assert!(names.contains(&"vibebus_thread_unbind"));
    assert!(names.contains(&"vibebus_thread_list"));
    assert!(names.contains(&"vibebus_retention_plan"));
    assert!(names.contains(&"vibebus_retention_apply"));
    assert!(names.contains(&"vibebus_retention_status"));
    assert!(names.contains(&"vibebus_credential_status"));
    assert!(names.contains(&"vibebus_credential_delete"));
    assert!(names.iter().all(|name| !name.contains("operator")));

    send(
        &mut writer,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "vibebus_status",
                "arguments": {"root": path_text(project.path())}
            }
        }),
    )
    .await;
    let called = response(&mut lines, 3).await;
    assert_eq!(called["result"]["isError"], false);
    assert!(
        called["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("MCP test")
    );
    let status = tool_text(&called);
    assert_eq!(status["operator"]["configured"], false);

    send(
        &mut writer,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "vibebus_register",
                "arguments": {
                    "root": path_text(project.path()),
                    "name": "mcp-vault-agent",
                    "role": "test",
                    "storeCredentials": true
                }
            }
        }),
    )
    .await;
    let registered = response(&mut lines, 4).await;
    let registered = tool_text(&registered);
    assert_eq!(registered["secretsRedacted"], true);
    assert!(registered.get("token").is_none());
    assert_eq!(registered["credentials"]["stored"], true);

    send(
        &mut writer,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "vibebus_inbox",
                "arguments": {
                    "root": path_text(project.path()),
                    "agent": "mcp-vault-agent"
                }
            }
        }),
    )
    .await;
    let inbox = response(&mut lines, 5).await;
    assert_eq!(inbox["result"]["isError"], false);
    assert_eq!(tool_text(&inbox), json!([]));

    send(
        &mut writer,
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "vibebus_agent_recover",
                "arguments": {
                    "root": path_text(project.path()),
                    "name": "mcp-vault-agent"
                }
            }
        }),
    )
    .await;
    let recovered = tool_text(&response(&mut lines, 6).await);
    assert_eq!(recovered["secretsRedacted"], true);
    assert_eq!(recovered["tokenGeneration"], 2);
    assert!(recovered.get("recoveryKey").is_none());

    send(
        &mut writer,
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "vibebus_retention_plan",
                "arguments": {
                    "root": path_text(project.path()),
                    "agent": "mcp-vault-agent"
                }
            }
        }),
    )
    .await;
    let plan = tool_text(&response(&mut lines, 7).await);
    let plan_id = plan["planId"].as_str().unwrap();

    send(
        &mut writer,
        json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": {
                "name": "vibebus_retention_apply",
                "arguments": {
                    "root": path_text(project.path()),
                    "agent": "mcp-vault-agent",
                    "planId": plan_id
                }
            }
        }),
    )
    .await;
    let unapproved = response(&mut lines, 8).await;
    assert!(unapproved["error"].is_object());
    assert!(
        unapproved["error"]["message"]
            .as_str()
            .unwrap()
            .contains("operator approval required")
    );

    send(
        &mut writer,
        json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {
                "name": "vibebus_credential_delete",
                "arguments": {
                    "root": path_text(project.path()),
                    "agent": "mcp-vault-agent",
                    "confirm": true
                }
            }
        }),
    )
    .await;
    let deleted = tool_text(&response(&mut lines, 9).await);
    assert_eq!(deleted["deleted"], true);
    assert_eq!(deleted["credentials"]["stored"], false);

    send(
        &mut writer,
        json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {
                "name": "vibebus_inbox",
                "arguments": {
                    "root": path_text(project.path()),
                    "agent": "mcp-vault-agent"
                }
            }
        }),
    )
    .await;
    let missing = response(&mut lines, 10).await;
    assert!(missing["error"].is_object());
    assert!(
        missing["error"]["message"]
            .as_str()
            .unwrap()
            .contains("credential vault")
    );

    drop(writer);
    drop(lines);
    server_task.await.unwrap();
}

async fn send(writer: &mut tokio::io::WriteHalf<tokio::io::DuplexStream>, value: Value) {
    writer
        .write_all(value.to_string().as_bytes())
        .await
        .unwrap();
    writer.write_all(b"\n").await.unwrap();
    writer.flush().await.unwrap();
}

async fn response<R>(lines: &mut tokio::io::Lines<BufReader<R>>, id: i64) -> Value
where
    R: tokio::io::AsyncRead + Unpin,
{
    loop {
        let line = lines.next_line().await.unwrap().unwrap();
        let value: Value = serde_json::from_str(&line).unwrap();
        if value["id"] == id {
            return value;
        }
    }
}

fn path_text(path: &std::path::Path) -> String {
    PathBuf::from(path).to_string_lossy().into_owned()
}

fn tool_text(response: &Value) -> Value {
    serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}
