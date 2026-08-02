//! Agent Spawn Bridge — derive external agent CLI subprocesses as tracked collaborators.
//!
//! Mirrors the agent-link-mcp model: spawn an agent CLI (codex, claude, gemini,
//! aider, ...), send it a task wrapped in a small marker protocol, and read its
//! output until it emits `[RESULT]` / `[QUESTION]` / `[ERROR]`, exits, or times
//! out. Sessions are recorded in an in-memory registry so an MCP server can
//! report status across calls.
//!
//! The bridge itself holds no business state: durable facts (tasks, messages,
//! handoffs) remain in the VibeBus SQLite bus. The host agent decides how to
//! persist a spawn outcome via the existing `vibebus_*` tools.

use std::{
    collections::HashMap,
    path::Path,
    process::Stdio,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    time::timeout,
};
use uuid::Uuid;

use crate::BusError;

const DEFAULT_TIMEOUT_SECS: u64 = 1800;
const RESULT_MARKER: &str = "[RESULT]";
const QUESTION_MARKER: &str = "[QUESTION]";
const ERROR_MARKER: &str = "[ERROR]";
const MAX_MESSAGE_CHARS: usize = 4000;

/// A declared external agent CLI that VibeBus can spawn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    /// Executable name or absolute path.
    pub command: String,
    /// Fixed arguments placed before the prompt argument.
    #[serde(default)]
    pub args: Vec<String>,
    /// Roles this profile is suited for (planner, reviewer, executor, ...).
    #[serde(default)]
    pub roles: Vec<String>,
    /// Extra environment variables for the child process.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// When true, send the prompt on stdin instead of as a trailing argument.
    #[serde(default)]
    pub prompt_via_stdin: bool,
}

/// A summary of a profile, for listing available agents.
#[derive(Debug, Clone, Serialize)]
pub struct ProfileSummary {
    pub name: String,
    pub command: String,
    pub roles: Vec<String>,
}

/// Collection of profiles loaded from `.vibebus/agents.json` with built-in defaults.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentProfiles {
    /// Map of profile name → profile.
    pub profiles: HashMap<String, AgentProfile>,
}

impl AgentProfiles {
    /// Load profiles from `.vibebus/agents.json` beside the project marker,
    /// merged over the built-in defaults for common agent CLIs.
    pub fn load(project_root: &Path) -> Self {
        let mut profiles = Self::defaults();
        let path = project_root.join(".vibebus").join("agents.json");
        if let Ok(text) = std::fs::read_to_string(&path)
            && let Ok(loaded) = serde_json::from_str::<AgentProfiles>(&text)
        {
            for (name, profile) in loaded.profiles {
                profiles.profiles.insert(name, profile);
            }
        }
        profiles
    }

    /// Built-in profiles for well-known agent CLIs. Presence on PATH is
    /// validated at spawn time, not at load time.
    pub fn defaults() -> Self {
        let mut profiles = HashMap::new();
        profiles.insert(
            "codex".into(),
            AgentProfile {
                command: "codex".into(),
                args: vec!["exec".into()],
                roles: vec!["executor".into(), "reviewer".into()],
                env: HashMap::new(),
                prompt_via_stdin: false,
            },
        );
        profiles.insert(
            "claude".into(),
            AgentProfile {
                command: "claude".into(),
                args: vec!["-p".into()],
                roles: vec!["planner".into(), "reviewer".into()],
                env: HashMap::new(),
                prompt_via_stdin: false,
            },
        );
        profiles.insert(
            "gemini".into(),
            AgentProfile {
                command: "gemini".into(),
                args: vec![],
                roles: vec!["researcher".into()],
                env: HashMap::new(),
                prompt_via_stdin: true,
            },
        );
        Self { profiles }
    }

    pub fn resolve(&self, name: &str) -> Option<&AgentProfile> {
        self.profiles.get(name)
    }

    pub fn list(&self) -> Vec<ProfileSummary> {
        self.profiles
            .iter()
            .map(|(name, p)| ProfileSummary {
                name: name.clone(),
                command: p.command.clone(),
                roles: p.roles.clone(),
            })
            .collect()
    }
}

/// Wrap a user task with the spawn marker protocol so the child emits parseable
/// result/question/error markers on its own stdout.
pub fn build_protocol_prompt(task: &str, role: Option<&str>) -> String {
    let role_line = role
        .map(|r| format!("You are acting in the role: {r}.\n\n"))
        .unwrap_or_default();
    format!(
        "{role_line}\
{task}\n\n\
--- VibeBus spawn protocol ---\n\
When you have a final answer, output a line that starts exactly with `[RESULT]` followed by the answer.\n\
If you need clarification from the host before proceeding, output a line that starts exactly with `[QUESTION]` followed by the question, then stop and wait.\n\
If you cannot complete the task, output a line that starts exactly with `[ERROR]` followed by the reason.\n\
Keep each marker line self-contained on a single line. Use only one marker per response.\n"
    )
}

/// A parsed marker line.
#[derive(Debug, Clone, PartialEq)]
pub enum Marker {
    Result(String),
    Question(String),
    Error(String),
    None,
}

/// Parse a single stdout line for a spawn protocol marker.
pub fn parse_marker(line: &str) -> Marker {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix(RESULT_MARKER) {
        Marker::Result(rest.trim().to_string())
    } else if let Some(rest) = trimmed.strip_prefix(QUESTION_MARKER) {
        Marker::Question(rest.trim().to_string())
    } else if let Some(rest) = trimmed.strip_prefix(ERROR_MARKER) {
        Marker::Error(rest.trim().to_string())
    } else {
        Marker::None
    }
}

/// The terminal state of a spawn session.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SpawnState {
    Running,
    Question,
    Result,
    Error,
    Timeout,
    Closed,
}

/// The outcome of a spawn call, returned to the host agent.
#[derive(Debug, Clone, Serialize)]
pub struct SpawnOutcome {
    pub session_id: String,
    pub agent: String,
    pub role: Option<String>,
    pub state: SpawnState,
    pub message: String,
    pub elapsed_ms: u128,
}

/// A recorded spawn session kept in the registry for status queries.
#[derive(Debug, Clone, Serialize)]
pub struct SpawnRecord {
    pub session_id: String,
    pub agent: String,
    pub role: Option<String>,
    pub state: SpawnState,
    pub message: String,
    pub started_at_ms: u128,
    pub elapsed_ms: u128,
}

/// In-memory registry of spawn sessions, shared across MCP tool calls.
#[derive(Debug, Default)]
pub struct SpawnRegistry {
    records: Mutex<Vec<SpawnRecord>>,
}

impl SpawnRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn record(&self, outcome: &SpawnOutcome) {
        let record = SpawnRecord {
            session_id: outcome.session_id.clone(),
            agent: outcome.agent.clone(),
            role: outcome.role.clone(),
            state: outcome.state.clone(),
            message: outcome.message.clone(),
            started_at_ms: 0,
            elapsed_ms: outcome.elapsed_ms,
        };
        if let Ok(mut records) = self.records.lock() {
            records.push(record);
            // Keep the registry bounded.
            let overflow = records.len().saturating_sub(200);
            if overflow > 0 {
                records.drain(0..overflow);
            }
        }
    }

    pub fn status(&self) -> Vec<SpawnRecord> {
        self.records.lock().map(|r| r.clone()).unwrap_or_default()
    }
}

enum ReadOutcome {
    Marker(SpawnState, String),
    Eof(String),
}

/// Spawn an external agent CLI, send it a task, and read stdout until a protocol
/// marker, process exit, or timeout. The child is killed on timeout.
pub async fn spawn_once(
    profile: &AgentProfile,
    task: &str,
    role: Option<&str>,
    timeout_secs: Option<u64>,
) -> Result<SpawnOutcome, BusError> {
    let session_id = format!("spawn-{}", &Uuid::new_v4().to_string()[..8]);
    let started = Instant::now();
    let limit = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
    let prompt = build_protocol_prompt(task, role);

    let mut child = spawn_child(profile, &prompt).await?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| BusError::Validation("agent child produced no stdout pipe".into()))?;
    let mut reader = BufReader::new(stdout).lines();

    let read = timeout(limit, async {
        let mut acc = String::new();
        while let Ok(Some(line)) = reader.next_line().await {
            match parse_marker(&line) {
                Marker::Result(text) => return ReadOutcome::Marker(SpawnState::Result, text),
                Marker::Question(text) => return ReadOutcome::Marker(SpawnState::Question, text),
                Marker::Error(text) => return ReadOutcome::Marker(SpawnState::Error, text),
                Marker::None => {
                    if !line.is_empty() {
                        acc.push_str(&line);
                        acc.push('\n');
                    }
                }
            }
        }
        ReadOutcome::Eof(acc)
    })
    .await;

    let (state, message) = match read {
        Ok(ReadOutcome::Marker(state, text)) => {
            let msg = if text.is_empty() {
                "(empty marker body)".to_string()
            } else {
                text
            };
            (state, msg)
        }
        Ok(ReadOutcome::Eof(acc)) => {
            let trimmed = acc.trim();
            if trimmed.is_empty() {
                (SpawnState::Error, "agent exited with no output".to_string())
            } else {
                (SpawnState::Result, limit_text(trimmed, MAX_MESSAGE_CHARS))
            }
        }
        Err(_) => {
            let _ = child.kill().await;
            (
                SpawnState::Timeout,
                format!("agent timed out after {}s", limit.as_secs()),
            )
        }
    };

    // Wait for the child to exit so we don't leak a process.
    let _ = child.wait().await;

    Ok(SpawnOutcome {
        session_id,
        agent: profile.command.clone(),
        role: role.map(str::to_owned),
        state,
        message,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

async fn spawn_child(profile: &AgentProfile, prompt: &str) -> Result<Child, BusError> {
    let mut cmd = Command::new(&profile.command);
    cmd.args(&profile.args);
    for (key, value) in &profile.env {
        cmd.env(key, value);
    }
    if profile.prompt_via_stdin {
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    } else {
        cmd.arg(prompt)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    }
    // Suppress interactive prompts: agent CLIs must not hang on a TTY.
    cmd.env("CI", "1");

    let mut child = cmd.spawn().map_err(|error| {
        BusError::Validation(format!(
            "failed to spawn agent '{}': {error}. Is the CLI installed and on PATH?",
            profile.command
        ))
    })?;

    if profile.prompt_via_stdin
        && let Some(mut stdin) = child.stdin.take()
    {
        // Best-effort write; a closed stdin is fine for CLIs that exit early.
        let _ = stdin.write_all(prompt.as_bytes()).await;
        let _ = stdin.shutdown().await;
    }

    Ok(child)
}

fn limit_text(text: &str, maximum: usize) -> String {
    if text.chars().count() <= maximum {
        return text.to_string();
    }
    let mut out: String = text.chars().take(maximum).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn defaults_cover_common_agent_clis() {
        let profiles = AgentProfiles::defaults();
        assert!(profiles.resolve("codex").is_some());
        assert!(profiles.resolve("claude").is_some());
        assert!(profiles.resolve("gemini").is_some());
        assert!(profiles.resolve("unknown").is_none());
        let codex = profiles.resolve("codex").unwrap();
        assert!(codex.args.contains(&"exec".to_string()));
        assert!(codex.roles.contains(&"executor".to_string()));
    }

    #[test]
    fn load_merges_over_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let vibebus_dir = dir.path().join(".vibebus");
        std::fs::create_dir_all(&vibebus_dir).unwrap();
        let mut file = std::fs::File::create(vibebus_dir.join("agents.json")).unwrap();
        writeln!(
            file,
            "{{\"profiles\":{{\"custom\":{{\"command\":\"my-agent\",\"args\":[\"-x\"],\"roles\":[\"reviewer\"]}}}}}}"
        )
        .unwrap();
        let profiles = AgentProfiles::load(dir.path());
        // default still present
        assert!(profiles.resolve("codex").is_some());
        // custom merged in
        let custom = profiles.resolve("custom").unwrap();
        assert_eq!(custom.command, "my-agent");
        assert!(custom.args.contains(&"-x".to_string()));
    }

    #[test]
    fn protocol_prompt_includes_role_and_markers() {
        let prompt = build_protocol_prompt("refactor the auth module", Some("reviewer"));
        assert!(prompt.contains("role: reviewer"));
        assert!(prompt.contains("refactor the auth module"));
        assert!(prompt.contains("[RESULT]"));
        assert!(prompt.contains("[QUESTION]"));
        assert!(prompt.contains("[ERROR]"));
    }

    #[test]
    fn parse_marker_recognizes_all_markers() {
        assert_eq!(
            parse_marker("[RESULT] all good"),
            Marker::Result("all good".into())
        );
        assert_eq!(
            parse_marker("  [QUESTION] which cache?  "),
            Marker::Question("which cache?".into())
        );
        assert_eq!(parse_marker("[ERROR] nope"), Marker::Error("nope".into()));
        assert_eq!(parse_marker("just output"), Marker::None);
        assert_eq!(parse_marker("[result] lower case"), Marker::None);
    }

    #[test]
    fn registry_records_and_lists_outcomes() {
        let registry = SpawnRegistry::new();
        let outcome = SpawnOutcome {
            session_id: "spawn-deadbeef".into(),
            agent: "codex".into(),
            role: Some("reviewer".into()),
            state: SpawnState::Result,
            message: "looks good".into(),
            elapsed_ms: 1234,
        };
        registry.record(&outcome);
        let status = registry.status();
        assert_eq!(status.len(), 1);
        assert_eq!(status[0].session_id, "spawn-deadbeef");
        assert_eq!(status[0].state, SpawnState::Result);
    }

    #[tokio::test]
    async fn spawn_once_captures_result_marker_from_a_script() {
        // Use the shell to simulate an agent CLI that prints a [RESULT] marker.
        let profile = AgentProfile {
            command: "sh".into(),
            args: vec![
                "-c".into(),
                "echo 'working...'; echo '[RESULT] 42 is the answer'".into(),
            ],
            roles: vec![],
            env: HashMap::new(),
            prompt_via_stdin: false,
        };
        let outcome = spawn_once(&profile, "any task", None, Some(5))
            .await
            .unwrap();
        assert_eq!(outcome.state, SpawnState::Result);
        assert_eq!(outcome.message, "42 is the answer");
        assert_eq!(outcome.agent, "sh");
    }

    #[tokio::test]
    async fn spawn_once_times_out_and_kills() {
        let profile = AgentProfile {
            command: "sh".into(),
            args: vec!["-c".into(), "sleep 30".into()],
            roles: vec![],
            env: HashMap::new(),
            prompt_via_stdin: false,
        };
        let outcome = spawn_once(&profile, "slow task", None, Some(1))
            .await
            .unwrap();
        assert_eq!(outcome.state, SpawnState::Timeout);
        assert!(outcome.message.contains("timed out"));
    }
}
