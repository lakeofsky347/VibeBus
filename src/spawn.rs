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
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::Mutex as AsyncMutex,
    time::timeout,
};
use uuid::Uuid;

use crate::{Bus, BusError};

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
        // A no-dependency demo profile so the spawn bridge is runnable without
        // any external agent CLI installed. Emits a [RESULT] marker immediately.
        profiles.insert(
            "demo".into(),
            AgentProfile {
                command: "sh".into(),
                args: vec![
                    "-c".into(),
                    "echo 'demo agent: acknowledged task'; echo '[RESULT] demo agent completed the task'".into(),
                ],
                roles: vec!["executor".into()],
                env: HashMap::new(),
                prompt_via_stdin: false,
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
enum ReadOutcome {
    Marker(SpawnState, String),
    Eof(String),
    Timeout,
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
    let mut lines = BufReader::new(stdout).lines();
    let read = read_until_marker(&mut lines, limit).await;

    let (state, message) = match read {
        ReadOutcome::Marker(state, text) => {
            let msg = if text.is_empty() {
                "(empty marker body)".to_string()
            } else {
                text
            };
            (state, msg)
        }
        ReadOutcome::Eof(acc) => {
            let trimmed = acc.trim();
            if trimmed.is_empty() {
                (SpawnState::Error, "agent exited with no output".to_string())
            } else {
                (SpawnState::Result, limit_text(trimmed, MAX_MESSAGE_CHARS))
            }
        }
        ReadOutcome::Timeout => {
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

/// Spawn a child with stdin kept open for bidirectional reply (P2 session
/// path). The prompt is sent as a trailing arg, or via the first stdin write
/// when `prompt_via_stdin` is set; stdin is never closed so the host can reply.
async fn spawn_session_child(
    profile: &AgentProfile,
    prompt: &str,
) -> Result<(Child, ChildStdin, ChildStdout), BusError> {
    let mut cmd = Command::new(&profile.command);
    cmd.args(&profile.args);
    for (key, value) in &profile.env {
        cmd.env(key, value);
    }
    if !profile.prompt_via_stdin {
        cmd.arg(prompt);
    }
    cmd.env("CI", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|error| {
        BusError::Validation(format!(
            "failed to spawn agent '{}': {error}. Is the CLI installed and on PATH?",
            profile.command
        ))
    })?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| BusError::Validation("agent child produced no stdin pipe".into()))?;
    if profile.prompt_via_stdin {
        let _ = stdin.write_all(prompt.as_bytes()).await;
        let _ = stdin.flush().await;
    }
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| BusError::Validation("agent child produced no stdout pipe".into()))?;
    Ok((child, stdin, stdout))
}

fn limit_text(text: &str, maximum: usize) -> String {
    if text.chars().count() <= maximum {
        return text.to_string();
    }
    let mut out: String = text.chars().take(maximum).collect();
    out.push('…');
    out
}

/// Read child stdout lines until a protocol marker, EOF, or timeout. Shared by
/// the one-shot `spawn_once` path and the persistent `SpawnSession` path.
async fn read_until_marker(
    lines: &mut tokio::io::Lines<BufReader<ChildStdout>>,
    limit: Duration,
) -> ReadOutcome {
    match timeout(limit, async {
        let mut acc = String::new();
        while let Ok(Some(line)) = lines.next_line().await {
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
    .await
    {
        Ok(outcome) => outcome,
        Err(_) => ReadOutcome::Timeout,
    }
}

/// A live spawn session kept in the registry so `reply` / `kill` can operate on
/// it across MCP tool calls. The child stays alive while `state == Question`.
pub struct SpawnSession {
    pub id: String,
    pub agent: String,
    pub role: Option<String>,
    pub task: String,
    pub state: SpawnState,
    pub started_at: Instant,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    lines: Option<tokio::io::Lines<BufReader<ChildStdout>>>,
    last_message: String,
}

impl SpawnSession {
    /// Drive the child until it emits a marker, exits, or times out. On a
    /// terminal state the child is reaped; on `Question` it stays alive for a
    /// later `reply`.
    async fn advance(&mut self, limit: Duration) -> SpawnOutcome {
        let Some(lines) = self.lines.as_mut() else {
            return self
                .finalize(SpawnState::Closed, "session has no stdout")
                .await;
        };
        let read = read_until_marker(lines, limit).await;
        let (state, message) = match read {
            ReadOutcome::Marker(state, text) => {
                let msg = if text.is_empty() {
                    "(empty marker body)".to_string()
                } else {
                    text
                };
                (state, msg)
            }
            ReadOutcome::Eof(acc) => {
                let trimmed = acc.trim();
                if trimmed.is_empty() {
                    (SpawnState::Error, "agent exited with no output".to_string())
                } else {
                    (SpawnState::Result, limit_text(trimmed, MAX_MESSAGE_CHARS))
                }
            }
            ReadOutcome::Timeout => (
                SpawnState::Timeout,
                format!("agent timed out after {}s", limit.as_secs()),
            ),
        };
        self.finalize(state, &message).await
    }

    /// Update state, reap the child on any terminal state, and emit an outcome.
    async fn finalize(&mut self, state: SpawnState, message: &str) -> SpawnOutcome {
        if !matches!(state, SpawnState::Question | SpawnState::Running) {
            if let Some(mut child) = self.child.take() {
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
            self.stdin = None;
            self.lines = None;
        }
        self.state = state.clone();
        self.last_message = message.to_string();
        SpawnOutcome {
            session_id: self.id.clone(),
            agent: self.agent.clone(),
            role: self.role.clone(),
            state,
            message: message.to_string(),
            elapsed_ms: self.started_at.elapsed().as_millis(),
        }
    }
}

/// In-memory registry of persistent spawn sessions, shared across MCP calls.
#[derive(Default)]
pub struct SessionRegistry {
    sessions: Mutex<HashMap<String, Arc<AsyncMutex<SpawnSession>>>>,
}

impl SessionRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Spawn a child, run it to its first marker, and store the session. On
    /// `Question` the child stays alive for a later `reply`.
    pub async fn spawn(
        &self,
        profile: &AgentProfile,
        task: &str,
        role: Option<&str>,
        timeout_secs: Option<u64>,
    ) -> Result<SpawnOutcome, BusError> {
        let session_id = format!("spawn-{}", &Uuid::new_v4().to_string()[..8]);
        let limit = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let prompt = build_protocol_prompt(task, role);
        let (child, stdin, stdout) = spawn_session_child(profile, &prompt).await?;
        let lines = BufReader::new(stdout).lines();
        let mut session = SpawnSession {
            id: session_id.clone(),
            agent: profile.command.clone(),
            role: role.map(str::to_owned),
            task: task.to_string(),
            state: SpawnState::Running,
            started_at: Instant::now(),
            child: Some(child),
            stdin: Some(stdin),
            lines: Some(lines),
            last_message: String::new(),
        };
        let outcome = session.advance(limit).await;
        self.sessions
            .lock()
            .map(|mut map| {
                map.insert(session_id, Arc::new(AsyncMutex::new(session)));
            })
            .ok();
        Ok(outcome)
    }

    /// Continue a `Question`-state session by replying to the child and reading
    /// its next marker.
    pub async fn reply(
        &self,
        session_id: &str,
        message: &str,
        timeout_secs: Option<u64>,
    ) -> Result<SpawnOutcome, BusError> {
        let session = self
            .sessions
            .lock()
            .map_err(|_| BusError::Validation("spawn registry poisoned".into()))?
            .get(session_id)
            .cloned()
            .ok_or_else(|| BusError::Validation(format!("unknown spawn session '{session_id}'")))?;
        let mut session = session.lock().await;
        if !matches!(session.state, SpawnState::Question) {
            return Err(BusError::Validation(format!(
                "spawn session '{}' is not awaiting a reply (state: {:?})",
                session_id, session.state
            )));
        }
        if let Some(stdin) = session.stdin.as_mut() {
            stdin
                .write_all(format!("{message}\n").as_bytes())
                .await
                .map_err(|e| BusError::Validation(format!("failed to reply to agent: {e}")))?;
            let _ = stdin.flush().await;
        }
        let limit = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        Ok(session.advance(limit).await)
    }

    /// Kill a live session and mark it closed.
    pub async fn kill(&self, session_id: &str) -> Result<SpawnOutcome, BusError> {
        let session = self
            .sessions
            .lock()
            .map_err(|_| BusError::Validation("spawn registry poisoned".into()))?
            .get(session_id)
            .cloned()
            .ok_or_else(|| BusError::Validation(format!("unknown spawn session '{session_id}'")))?;
        let mut session = session.lock().await;
        Ok(session.finalize(SpawnState::Closed, "killed by host").await)
    }

    /// Snapshot of all sessions for status queries.
    pub fn status(&self) -> Vec<SessionStatus> {
        self.sessions
            .lock()
            .map(|map| {
                map.values()
                    .map(|session| {
                        // Best-effort non-blocking snapshot; state may lag briefly.
                        let session = session.clone();
                        let session = match session.try_lock() {
                            Ok(s) => s,
                            Err(_) => {
                                return SessionStatus {
                                    id: String::new(),
                                    agent: String::new(),
                                    role: None,
                                    state: SpawnState::Running,
                                    message: "(locked)".into(),
                                };
                            }
                        };
                        SessionStatus {
                            id: session.id.clone(),
                            agent: session.agent.clone(),
                            role: session.role.clone(),
                            state: session.state.clone(),
                            message: session.last_message.clone(),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// A non-owning snapshot of a session for status reporting.
#[derive(Debug, Clone, Serialize)]
pub struct SessionStatus {
    pub id: String,
    pub agent: String,
    pub role: Option<String>,
    pub state: SpawnState,
    pub message: String,
}

/// The durable fact-layer linkage created for a spawn outcome: a task bound to
/// the spawn session, optionally completed with a recorded result message.
#[derive(Debug, Clone, Serialize)]
pub struct SpawnLink {
    pub task_id: String,
    pub thread_id: String,
    pub task_version: i64,
    pub completed: bool,
}

/// Create + claim + bind a VibeBus task for a spawn outcome, and on a terminal
/// outcome record a self-message with the result and complete the task. Shared
/// by the MCP tool and the CLI one-shot path so both leave durable facts.
///
/// `task_description` is stored as the task description; `profile_command` and
/// `role` form the task title. Bind/message/complete are best-effort (errors
/// are ignored) so a spawn outcome is never lost because a fact write failed;
/// create + claim are fatal because without them the linkage is meaningless.
#[allow(clippy::too_many_arguments)]
pub fn link_spawn_outcome(
    bus: &mut Bus,
    agent: &str,
    token: &str,
    outcome: &SpawnOutcome,
    task_id: Option<&str>,
    task_description: &str,
    role: Option<&str>,
    profile_command: &str,
) -> Result<SpawnLink, BusError> {
    let task_id = task_id
        .map(str::to_owned)
        .unwrap_or_else(|| outcome.session_id.clone());
    let title = format!("spawn: {} {}", profile_command, role.unwrap_or("default"));
    bus.create_task(agent, token, &task_id, &title, Some(task_description), &[])?;
    let claimed = bus.claim_task(agent, token, &task_id)?;
    let thread_id = format!("spawn:{}", outcome.session_id);
    let _ = bus.bind_task_thread(agent, token, &task_id, &thread_id);
    let mut task_version = claimed.version;
    let terminal = !matches!(outcome.state, SpawnState::Question | SpawnState::Running);
    if terminal {
        let subject = format!("spawn result: {} ({:?})", profile_command, outcome.state);
        let _ = bus.send_message_idempotent(
            agent,
            token,
            &[agent.to_string()],
            &subject,
            &outcome.message,
            Some(&thread_id),
            "normal",
            false,
            Some(&format!("spawn-result-{}", outcome.session_id)),
        );
        if let Ok(completed) =
            bus.update_task(agent, token, &task_id, task_version, "completed", None)
        {
            task_version = completed.version;
        }
    }
    Ok(SpawnLink {
        task_id,
        thread_id,
        task_version,
        completed: terminal,
    })
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

    #[tokio::test]
    async fn session_registry_spawn_status_kill_roundtrip() {
        let registry = SessionRegistry::new();
        let profile = AgentProfile {
            command: "sh".into(),
            args: vec!["-c".into(), "echo '[RESULT] ok'".into()],
            roles: vec![],
            env: HashMap::new(),
            prompt_via_stdin: false,
        };
        let outcome = registry
            .spawn(&profile, "do a thing", None, Some(5))
            .await
            .unwrap();
        assert_eq!(outcome.state, SpawnState::Result);
        assert_eq!(outcome.message, "ok");
        // The session is recorded for status.
        let status = registry.status();
        assert_eq!(status.len(), 1);
        assert_eq!(status[0].state, SpawnState::Result);
        // Killing an already-terminal session is a no-op that reports Closed.
        let killed = registry.kill(&outcome.session_id).await.unwrap();
        assert_eq!(killed.state, SpawnState::Closed);
    }

    #[tokio::test]
    async fn session_registry_reply_to_a_question() {
        // Simulate an agent that asks a question, reads a reply from stdin, then
        // emits a result. stdin stays open so the host can reply.
        let script = "echo '[QUESTION] which cache?'; read -r ans; echo \"[RESULT] using $ans\"";
        let profile = AgentProfile {
            command: "sh".into(),
            args: vec!["-c".into(), script.into()],
            roles: vec![],
            env: HashMap::new(),
            prompt_via_stdin: false,
        };
        let registry = SessionRegistry::new();
        let q = registry
            .spawn(&profile, "add caching", None, Some(5))
            .await
            .unwrap();
        assert_eq!(q.state, SpawnState::Question);
        assert_eq!(q.message, "which cache?");
        let r = registry
            .reply(&q.session_id, "redis", Some(5))
            .await
            .unwrap();
        assert_eq!(r.state, SpawnState::Result);
        assert_eq!(r.message, "using redis");
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
