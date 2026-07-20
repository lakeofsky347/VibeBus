use std::{
    collections::HashSet,
    env, fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::Command,
};

use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{
    Bus, BusError, Result, discover_project, resolve_agent_token, system_credential_vault,
};

#[derive(Debug, Clone, Copy)]
pub enum CodexHook {
    SessionStart,
    PostToolUse,
    Stop,
}

#[derive(Debug, Deserialize)]
struct HookInput {
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    cwd: PathBuf,
    #[serde(default)]
    hook_event_name: String,
    #[serde(default)]
    tool_name: String,
    #[serde(default)]
    tool_input: Value,
    #[serde(default)]
    tool_response: Value,
    #[serde(default)]
    stop_hook_active: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActiveBinding {
    task_id: String,
    agent: String,
}

pub fn run_codex_hook(hook: CodexHook) -> Result<Option<Value>> {
    let input = read_hook_input()?;
    match hook {
        CodexHook::SessionStart => session_start(&input),
        CodexHook::PostToolUse => Ok(post_tool_use(&input).unwrap_or_else(|error| {
            Some(hook_message(&format!(
                "VibeBus lifecycle fact capture failed: {error}. The completed tool side effects were not changed."
            )))
        })),
        CodexHook::Stop => Ok(stop(&input).unwrap_or_else(|error| {
            Some(hook_message(&format!(
                "VibeBus handoff proposal generation failed: {error}. No handoff was sent."
            )))
        })),
    }
}

fn read_hook_input() -> Result<HookInput> {
    let mut raw = String::new();
    io::stdin().read_to_string(&mut raw)?;
    if raw.trim().is_empty() {
        return Err(BusError::Validation("hook input is empty".into()));
    }
    Ok(serde_json::from_str(&raw)?)
}

fn session_start(input: &HookInput) -> Result<Option<Value>> {
    if input.hook_event_name != "SessionStart" {
        return Ok(None);
    }
    let Some((root, project)) = find_project(&input.cwd)? else {
        return Ok(None);
    };
    let root = root.to_string_lossy();
    let context = format!(
        "VibeBus project '{}' is active at '{}'. For every VibeBus MCP call, pass root='{}'. Register this independent task once with storeCredentials=true, verify credential status, then use vault-backed handoff snapshots or inbox checks at turn boundaries without copying secrets into the task. If vault storage fails, retain the returned token and recovery key only in private credential context. Atomically claim tasks before work, bind a claimed task to the real Codex task ID when available, inspect the responsibility policy, reserve precise task-scoped project-relative paths before editing, obtain an authenticated expiring override for cross-domain paths, close processed messages, use idempotency keys for retried writes, inspect retention status before history replay, and prefer replay-safe subscription peek/ack over legacy consume-on-poll. PostToolUse may record bounded Git/test facts for the active binding; Stop only prepares a reviewable proposal and never sends a handoff automatically. VibeBus is a durable fact bus; it does not interrupt a model that is already generating.",
        project.name, root, root
    );
    Ok(Some(json!({
        "hookSpecificOutput": {
            "hookEventName": "SessionStart",
            "additionalContext": context
        }
    })))
}

fn post_tool_use(input: &HookInput) -> Result<Option<Value>> {
    if input.hook_event_name != "PostToolUse" || input.tool_name != "Bash" {
        return Ok(None);
    }
    let Some((root, _)) = find_project(&input.cwd)? else {
        return Ok(None);
    };
    let Some(binding) = resolve_binding(&root, &input.session_id)? else {
        return Ok(None);
    };
    let command = hook_command(&input.tool_input);
    if command.is_empty() {
        return Ok(None);
    }
    let is_commit = is_git_commit_command(&command);
    let suite = test_suite(&command);
    if !is_commit && suite.is_none() {
        return Ok(None);
    }
    let Some(exit_code) = hook_exit_code(&input.tool_response) else {
        return Ok(Some(hook_message(
            "VibeBus skipped lifecycle fact capture because Bash did not expose a reliable exit code.",
        )));
    };

    let dry_run = env::var_os("VIBEBUS_HOOK_DRY_RUN").as_deref() == Some("1".as_ref());
    let mut messages = Vec::new();
    let mut bus = if dry_run {
        None
    } else {
        Some(Bus::open(&root, None)?)
    };

    if is_commit && exit_code == 0 {
        let (commit_sha, summary, changed_paths) = if dry_run {
            let git: Value = serde_json::from_str(&required_env("VIBEBUS_HOOK_TEST_GIT")?)?;
            let sha = required_string(&git, "commitSha")?;
            let summary = required_string(&git, "summary")?;
            let paths = git
                .get("changedPaths")
                .and_then(Value::as_array)
                .ok_or_else(|| BusError::Validation("dry-run changedPaths are unavailable".into()))?
                .iter()
                .map(|value| {
                    value.as_str().map(str::to_owned).ok_or_else(|| {
                        BusError::Validation("dry-run changed path must be a string".into())
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            (sha, summary, paths)
        } else {
            git_commit_metadata(&root)?
        };
        if changed_paths.len() > 200 {
            return Err(BusError::Validation(
                "the commit exceeds the 200-path fact limit".into(),
            ));
        }
        if let Some(bus) = bus.as_mut() {
            let token = hook_token(bus, &binding.agent)?;
            bus.record_git_commit_idempotent(
                &binding.agent,
                &token,
                &binding.task_id,
                &commit_sha,
                &limit_text(&summary, 512),
                &changed_paths,
                Some(&format!("hook-git-{commit_sha}")),
            )?;
        }
        messages.push(format!(
            "VibeBus recorded Git commit {commit_sha} for task {} using changed paths only.",
            binding.task_id
        ));
    }

    if let Some(suite) = suite {
        let outcome = if exit_code == 0 { "passed" } else { "failed" };
        let bounded_command = limit_text(&command, 512);
        let (head, working_state) = if dry_run {
            ("dry-run-head".to_owned(), "dry-run-state".to_owned())
        } else {
            git_working_state(&root)
        };
        let result_hash = sha256(&format!(
            "{}|{bounded_command}|{head}|{working_state}",
            binding.task_id
        ));
        let result_key = format!("hook-test-{}", &result_hash[..48]);
        if let Some(bus) = bus.as_mut() {
            let token = hook_token(bus, &binding.agent)?;
            bus.record_test_result_idempotent(
                &binding.agent,
                &token,
                &binding.task_id,
                &result_key,
                suite,
                outcome,
                &format!("Observed test command exited with code {exit_code}."),
                Some(&bounded_command),
                None,
                Some(&format!("hook-test-{result_hash}")),
            )?;
        }
        messages.push(format!(
            "VibeBus recorded a bounded {suite} result ({outcome}) for task {}; command output was not stored.",
            binding.task_id
        ));
    }

    if messages.is_empty() {
        Ok(None)
    } else {
        Ok(Some(hook_message(&messages.join(" "))))
    }
}

fn stop(input: &HookInput) -> Result<Option<Value>> {
    if input.hook_event_name != "Stop" || input.stop_hook_active {
        return Ok(None);
    }
    let Some((root, _)) = find_project(&input.cwd)? else {
        return Ok(None);
    };
    let Some(binding) = resolve_binding(&root, &input.session_id)? else {
        return Ok(None);
    };
    let proposal = if env::var_os("VIBEBUS_HOOK_DRY_RUN").as_deref() == Some("1".as_ref()) {
        serde_json::from_str::<Value>(&required_env("VIBEBUS_HOOK_TEST_PROPOSAL")?)?
    } else {
        let bus = Bus::open(&root, None)?;
        let token = hook_token(&bus, &binding.agent)?;
        serde_json::to_value(bus.handoff_proposal(&binding.agent, &token, &binding.task_id, 10)?)?
    };
    let plugin_data = env::var_os("PLUGIN_DATA")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| BusError::Validation("PLUGIN_DATA is not available".into()))?;
    let proposal_root = plugin_data.join("handoff-proposals");
    fs::create_dir_all(&proposal_root)?;
    let safe_session = input
        .session_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    let timestamp = Utc::now().format("%Y%m%dT%H%M%S%3fZ");
    let proposal_path = proposal_root.join(format!("{safe_session}-{timestamp}.json"));
    fs::write(
        &proposal_path,
        format!("{}\n", serde_json::to_string_pretty(&proposal)?),
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&proposal_path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(Some(hook_message(&format!(
        "VibeBus prepared a bounded handoff proposal for task {} at '{}'. Review it before explicitly sending; no handoff was sent automatically.",
        binding.task_id,
        proposal_path.display()
    ))))
}

fn find_project(cwd: &Path) -> Result<Option<(PathBuf, crate::ProjectMetadata)>> {
    if cwd.as_os_str().is_empty() {
        return Err(BusError::Validation("hook cwd is unavailable".into()));
    }
    match discover_project(cwd) {
        Ok(project) => Ok(Some(project)),
        Err(BusError::ProjectNotFound(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

fn resolve_binding(root: &Path, session_id: &str) -> Result<Option<ActiveBinding>> {
    if env::var_os("VIBEBUS_HOOK_DRY_RUN").as_deref() == Some("1".as_ref()) {
        let binding = required_env("VIBEBUS_HOOK_TEST_BINDING")?;
        return Ok(Some(serde_json::from_str(&binding)?));
    }
    let bus = Bus::open(root, None)?;
    let mut matches = bus
        .list_task_thread_bindings(None, false)?
        .into_iter()
        .filter(|binding| {
            binding.unbound_at.is_none()
                && (binding.thread_id == session_id
                    || binding.thread_id == format!("codex:{session_id}"))
        });
    let first = matches.next();
    if matches.next().is_some() {
        return Err(BusError::Validation(
            "multiple active VibeBus task bindings match this Codex task".into(),
        ));
    }
    Ok(first.map(|binding| ActiveBinding {
        task_id: binding.task_id,
        agent: binding.agent,
    }))
}

fn hook_token(bus: &Bus, agent: &str) -> Result<String> {
    let vault = system_credential_vault();
    Ok(resolve_agent_token(vault.as_ref(), &bus.project().project_id, agent, None, None)?.value)
}

fn hook_command(tool_input: &Value) -> String {
    match tool_input.get("command") {
        Some(Value::String(command)) => command.trim().to_owned(),
        Some(Value::Array(commands)) => commands
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_owned(),
        _ => String::new(),
    }
}

fn hook_exit_code(response: &Value) -> Option<i32> {
    let candidates = [
        response.get("exit_code"),
        response.get("exitCode"),
        response
            .get("metadata")
            .and_then(|value| value.get("exit_code")),
        response
            .get("metadata")
            .and_then(|value| value.get("exitCode")),
    ];
    for candidate in candidates.into_iter().flatten() {
        if let Some(value) = candidate
            .as_i64()
            .and_then(|value| i32::try_from(value).ok())
        {
            return Some(value);
        }
        if let Some(value) = candidate.as_str().and_then(|value| value.parse().ok()) {
            return Some(value);
        }
    }
    let raw = response.as_str()?;
    raw.lines().find_map(|line| {
        let lower = line.trim().to_ascii_lowercase();
        if !(lower.starts_with("exit code") || lower.starts_with("process exited with code")) {
            return None;
        }
        lower
            .split(|character: char| !(character.is_ascii_digit() || character == '-'))
            .rfind(|part| !part.is_empty() && *part != "-")
            .and_then(|part| part.parse().ok())
    })
}

fn is_git_commit_command(command: &str) -> bool {
    command_segments(command).any(|words| {
        words.iter().enumerate().any(|(index, word)| {
            if !is_program(word, "git") {
                return false;
            }
            if index > 0
                && !words[..index].iter().all(|prefix| {
                    prefix.contains('=')
                        || matches!(prefix.to_ascii_lowercase().as_str(), "env" | "command")
                })
            {
                return false;
            }
            let mut cursor = index + 1;
            while cursor < words.len() {
                let candidate = words[cursor];
                if candidate.eq_ignore_ascii_case("commit") {
                    return true;
                }
                if matches!(
                    candidate,
                    "-c" | "-C" | "--git-dir" | "--work-tree" | "--namespace"
                ) {
                    cursor += 2;
                    continue;
                }
                if candidate.starts_with('-') {
                    cursor += 1;
                    continue;
                }
                return false;
            }
            false
        })
    })
}

fn test_suite(command: &str) -> Option<&'static str> {
    for words in command_segments(command) {
        for (index, word) in words.iter().enumerate() {
            let tail = &words[index + 1..];
            if is_program(word, "cargo")
                && tail
                    .iter()
                    .find(|candidate| !candidate.starts_with('-') && !candidate.starts_with('+'))
                    .is_some_and(|candidate| candidate.eq_ignore_ascii_case("test"))
            {
                return Some("cargo test");
            }
            if ["npm", "pnpm", "yarn"]
                .iter()
                .any(|program| is_program(word, program))
                && tail
                    .iter()
                    .take(2)
                    .any(|candidate| candidate.eq_ignore_ascii_case("test"))
            {
                return Some("JavaScript tests");
            }
            if is_program(word, "pytest")
                || (is_program(word, "python")
                    && tail
                        .windows(2)
                        .any(|pair| pair[0] == "-m" && pair[1].eq_ignore_ascii_case("pytest")))
            {
                return Some("pytest");
            }
            if is_program(word, "go")
                && tail
                    .first()
                    .is_some_and(|candidate| candidate.eq_ignore_ascii_case("test"))
            {
                return Some("go test");
            }
            if is_program(word, "dotnet")
                && tail
                    .first()
                    .is_some_and(|candidate| candidate.eq_ignore_ascii_case("test"))
            {
                return Some("dotnet test");
            }
            let lower = word.to_ascii_lowercase();
            let file = lower.rsplit('/').next().unwrap_or(&lower);
            if file.starts_with("test-") && (file.ends_with(".ps1") || file.ends_with(".sh")) {
                return Some("lifecycle acceptance");
            }
        }
    }
    None
}

fn command_segments(command: &str) -> impl Iterator<Item = Vec<&str>> {
    command
        .split([';', '&', '|'])
        .map(|segment| segment.split_whitespace().collect::<Vec<_>>())
        .filter(|words| !words.is_empty())
}

fn is_program(candidate: &str, program: &str) -> bool {
    candidate
        .trim_matches(['\'', '"'])
        .rsplit(['/', '\\'])
        .next()
        .is_some_and(|name| {
            name.eq_ignore_ascii_case(program)
                || name.eq_ignore_ascii_case(&format!("{program}.exe"))
        })
}

fn git_commit_metadata(root: &Path) -> Result<(String, String, Vec<String>)> {
    let commit_sha = git_output(root, &["rev-parse", "HEAD"])?;
    let summary = git_output(root, &["show", "-s", "--format=%s", "HEAD"])?;
    let raw_paths = git_output(
        root,
        &[
            "show",
            "--pretty=format:",
            "--name-only",
            "--no-renames",
            "HEAD",
        ],
    )?;
    let mut seen = HashSet::new();
    let changed_paths = raw_paths
        .lines()
        .map(str::trim)
        .filter(|path| !path.is_empty() && seen.insert((*path).to_owned()))
        .map(str::to_owned)
        .collect();
    Ok((commit_sha, summary, changed_paths))
}

fn git_working_state(root: &Path) -> (String, String) {
    let head = match git_output(root, &["rev-parse", "HEAD"]) {
        Ok(value) if !value.is_empty() => value,
        _ => "no-head".into(),
    };
    let raw = git_output(root, &["status", "--porcelain=v1", "-uno"])
        .unwrap_or_else(|_| "status-unavailable".into());
    (head, sha256(&raw))
}

fn git_output(root: &Path, arguments: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()?;
    if !output.status.success() {
        return Err(BusError::Validation(format!(
            "git {} failed with exit code {}",
            arguments.first().copied().unwrap_or("command"),
            output.status.code().unwrap_or(-1)
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| BusError::Validation(format!("{name} is unavailable")))
}

fn required_string(value: &Value, field: &str) -> Result<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| BusError::Validation(format!("dry-run {field} is unavailable")))
}

fn sha256(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn limit_text(text: &str, maximum: usize) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(maximum)
        .collect()
}

fn hook_message(message: &str) -> Value {
    json!({"systemMessage": limit_text(message, 1200)})
}

#[cfg(test)]
mod tests {
    use super::{hook_exit_code, is_git_commit_command, test_suite};
    use serde_json::json;

    #[test]
    fn recognizes_bounded_lifecycle_commands_and_exit_codes() {
        assert!(is_git_commit_command(
            "git -c user.name=test commit -m fixture"
        ));
        assert!(!is_git_commit_command("echo git commit"));
        assert_eq!(
            test_suite("cargo --locked test --all-targets"),
            Some("cargo test")
        );
        assert_eq!(
            test_suite("./scripts/test-lifecycle-hooks.sh"),
            Some("lifecycle acceptance")
        );
        assert_eq!(
            hook_exit_code(&json!({"metadata": {"exitCode": 7}})),
            Some(7)
        );
        assert_eq!(
            hook_exit_code(&json!("process exited with code: -2")),
            Some(-2)
        );
    }
}
