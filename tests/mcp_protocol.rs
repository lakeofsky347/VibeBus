use std::path::PathBuf;

use rmcp::ServiceExt;
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use vibebus::{initialize_project, mcp::VibeBusMcp};

#[tokio::test(flavor = "current_thread")]
async fn mcp_negotiates_lists_tools_and_calls_status() {
    let project = TempDir::new().unwrap();
    let data_home = TempDir::new().unwrap();
    initialize_project(project.path(), "MCP test", Some(data_home.path())).unwrap();

    let (client_stream, server_stream) = tokio::io::duplex(128 * 1024);
    let project_root = project.path().to_path_buf();
    let server_data_home = data_home.path().to_path_buf();
    let server_task = tokio::spawn(async move {
        let service = VibeBusMcp::new(project_root, Some(server_data_home))
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
