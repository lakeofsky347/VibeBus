//! API injection engine — resolve project-scoped API providers into process
//! environment variables for TUI agent CLIs.
//!
//! This module implements the phase-1 "injection base" of the API injection
//! plan (docs/api-injection-plan.md §3). It holds the provider model
//! (`ApiProvider`, `ApiFormat`), the project-scoped `.vibebus/api.json`
//! configuration, Base-URL normalization, standard env-var derivation for each
//! target protocol, system-env conflict detection, write-ahead backups with
//! restore, and a local (offline) health check. API keys never appear here:
//! they are resolved from the OS credential vault at injection time via
//! `ApiCredentialVault` and returned only inside `ResolvedApiEnv`.
//!
//! Design invariants:
//! - Default is zero injection: an absent or empty config resolves to `None`.
//! - Provider env is derived from `api_format` + `base_url` + `model`; a
//!   per-provider `env_template` may add or override protocol-standard vars.
//! - Backups are write-ahead: `use` snapshots the config before mutation.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{BusError, Result};
use crate::vault::{ApiCredentialVault, SecretSource};

/// Schema version written to `.vibebus/api.json`; used by `ApiConfig` round
/// trips and kept for forward-compatible migrations (phase 3 registers the
/// same shape in the bus).
const API_CONFIG_VERSION: u32 = 1;
const MAX_BACKUPS: usize = 10;
const CONFIG_FILE: &str = "api.json";

/// The API wire protocol an injected provider speaks. Determines which
/// environment variables are derived for the target agent CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiFormat {
    /// Anthropic Messages protocol (`ANTHROPIC_BASE_URL` / `ANTHROPIC_AUTH_TOKEN`).
    Anthropic,
    /// OpenAI Chat Completions protocol (`OPENAI_BASE_URL` / `OPENAI_API_KEY`).
    OpenAIChat,
    /// OpenAI Responses protocol (same env vars as chat; wire format differs).
    OpenAIResponses,
    /// Google Gemini protocol (`GEMINI_BASE_URL` / `GEMINI_API_KEY`).
    Gemini,
}

impl ApiFormat {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "anthropic" => Ok(Self::Anthropic),
            "openai-chat" => Ok(Self::OpenAIChat),
            "openai-responses" => Ok(Self::OpenAIResponses),
            "gemini" => Ok(Self::Gemini),
            other => Err(BusError::Validation(format!(
                "unknown api format '{other}'; expected anthropic | openai-chat | openai-responses | gemini"
            ))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAIChat => "openai-chat",
            Self::OpenAIResponses => "openai-responses",
            Self::Gemini => "gemini",
        }
    }

    /// The standard env var holding the Base URL for this protocol.
    fn base_url_var(&self) -> &'static str {
        match self {
            Self::Anthropic => "ANTHROPIC_BASE_URL",
            Self::OpenAIChat | Self::OpenAIResponses => "OPENAI_BASE_URL",
            Self::Gemini => "GEMINI_BASE_URL",
        }
    }

    /// The standard env var holding the API key for this protocol.
    fn api_key_var(&self) -> &'static str {
        match self {
            Self::Anthropic => "ANTHROPIC_AUTH_TOKEN",
            Self::OpenAIChat | Self::OpenAIResponses => "OPENAI_API_KEY",
            Self::Gemini => "GEMINI_API_KEY",
        }
    }

    /// Optional standard env var for the default model, when one is configured.
    fn model_var(&self) -> Option<&'static str> {
        match self {
            Self::Anthropic => Some("ANTHROPIC_MODEL"),
            Self::OpenAIChat | Self::OpenAIResponses | Self::Gemini => None,
        }
    }
}

/// A declared API provider (non-secret metadata). The API key itself is never
/// stored here — it lives in the OS credential vault keyed by provider name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApiProvider {
    pub name: String,
    /// Normalized Base URL without a trailing slash and without a protocol path
    /// (e.g. `https://voltapi.com` — never `/v1/messages`).
    pub base_url: String,
    pub api_format: ApiFormat,
    /// Optional default model injected via the protocol's model env var.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Optional extra environment variables merged over the protocol defaults.
    #[serde(default)]
    pub env_template: HashMap<String, String>,
}

impl ApiProvider {
    /// Validate and normalize a Base URL: must be http/https and must not end
    /// with a slash (a trailing slash causes `//` double-slash path joins in
    /// the target CLI, the classic cc-switch pitfall).
    pub fn normalize_base_url(raw: &str) -> Result<String> {
        let trimmed = raw.trim().trim_end_matches('/');
        let lower = trimmed.to_ascii_lowercase();
        if !(lower.starts_with("http://") || lower.starts_with("https://")) {
            return Err(BusError::Validation(format!(
                "base URL '{raw}' must start with http:// or https://"
            )));
        }
        if trimmed.is_empty() {
            return Err(BusError::Validation("base URL cannot be empty".into()));
        }
        Ok(trimmed.to_string())
    }

    /// Derive the protocol-standard env vars for this provider (Base URL +
    /// optional model). The API key is injected separately by the resolver so
    /// the key never appears in the on-disk config.
    pub fn standard_env(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        vars.insert(self.api_format.base_url_var().to_string(), self.base_url.clone());
        if let Some(model) = &self.model
            && let Some(var) = self.api_format.model_var()
        {
            vars.insert(var.to_string(), model.clone());
        }
        for (key, value) in &self.env_template {
            vars.insert(key.clone(), value.clone());
        }
        vars
    }
}

/// The project-scoped API configuration persisted at `.vibebus/api.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApiConfig {
    pub version: u32,
    #[serde(default)]
    pub providers: HashMap<String, ApiProvider>,
    #[serde(default)]
    pub active: Option<String>,
}

impl ApiConfig {
    /// Load the config from `<root>/.vibebus/api.json`. A missing file or an
    /// empty provider map is a valid zero-injection state (never an error).
    pub fn load(root: &Path) -> Result<Self> {
        let path = config_path(root);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(error.into()),
        };
        let config: ApiConfig = serde_json::from_str(&text).map_err(|error| {
            BusError::Validation(format!(
                "invalid API config at {}: {error}",
                path.display()
            ))
        })?;
        if config.version > API_CONFIG_VERSION {
            return Err(BusError::Validation(format!(
                "API config version {} is newer than this build supports ({API_CONFIG_VERSION}); upgrade VibeBus",
                config.version
            )));
        }
        Ok(config)
    }

    /// Persist the config, creating the `.vibebus` directory if needed. The
    /// schema version is stamped on every write.
    pub fn save(&self, root: &Path) -> Result<()> {
        let mut stamped = self.clone();
        stamped.version = API_CONFIG_VERSION;
        let dir = config_dir(root);
        fs::create_dir_all(&dir)?;
        let path = config_path(root);
        let payload = serde_json::to_string_pretty(&stamped)?;
        fs::write(&path, format!("{payload}\n"))?;
        Ok(())
    }

    /// Whether injection is currently active for a named provider.
    pub fn active_provider(&self) -> Option<&ApiProvider> {
        self.active
            .as_deref()
            .and_then(|name| self.providers.get(name))
    }
}

/// A resolved, injectable environment snapshot. The API key is included only in
/// this transient value (destined for a child-process env), never persisted.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedApiEnv {
    pub provider: String,
    pub vars: HashMap<String, String>,
    pub source: SecretSource,
}

/// Resolve the active provider (or an explicitly named one) into the concrete
/// environment variable set to inject into an agent child process.
///
/// `vault` must implement `ApiCredentialVault`; on platforms without a system
/// vault the resolved key lookup fails with a clear `CredentialVault` error so
/// injection is never half-configured. When no provider is configured or the
/// requested provider is unknown, returns `Ok(None)` (zero injection).
pub fn resolve_api_env(
    config: &ApiConfig,
    vault: &dyn ApiCredentialVault,
    project_id: &str,
    provider: Option<&str>,
) -> Result<Option<ResolvedApiEnv>> {
    let provider = provider.or(config.active.as_deref());
    let Some(provider) = provider else {
        return Ok(None);
    };
    let provider = config.providers.get(provider).ok_or_else(|| {
        BusError::Validation(format!(
            "unknown api provider '{provider}'; configured providers: {}",
            if config.providers.is_empty() {
                "(none)".to_string()
            } else {
                config.providers.keys().cloned().collect::<Vec<_>>().join(", ")
            }
        ))
    })?;

    let mut vars = provider.standard_env();
    let api_key = vault
        .load_api(project_id, &provider.name)?
        .ok_or_else(|| {
            BusError::CredentialVault(format!(
                "api key for provider '{}' is not stored in the OS credential vault; run `vibebus api provider add --key` or `vibebus api key set` first",
                provider.name
            ))
        })?;
    vars.insert(provider.api_format.api_key_var().to_string(), api_key);

    Ok(Some(ResolvedApiEnv {
        provider: provider.name.clone(),
        vars,
        source: SecretSource::Vault,
    }))
}

/// A system environment variable that would conflict with an injected one.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvConflict {
    pub var: String,
    pub current: Option<String>,
}

/// Detect variables already present in the current process environment that the
/// injected set would set. A match means the agent child would inherit the
/// *parent* value instead of the injected one (or the injection would silently
/// override a user's explicit env) — worth surfacing before `use`.
pub fn detect_env_conflicts(vars: &HashMap<String, String>) -> Vec<EnvConflict> {
    vars.iter()
        .filter_map(|(key, value)| match std::env::var(key) {
            Ok(current) if current != *value => Some(EnvConflict {
                var: key.clone(),
                current: Some(current),
            }),
            Ok(_) => None,
            Err(_) => None,
        })
        .collect()
}

/// The directory that holds the API config and its backups:
/// `<root>/.vibebus/api/`.
fn config_dir(root: &Path) -> PathBuf {
    root.join(".vibebus").join("api")
}

fn config_path(root: &Path) -> PathBuf {
    config_dir(root).join(CONFIG_FILE)
}

fn backups_dir(root: &Path) -> PathBuf {
    config_dir(root).join("backups")
}

/// Write-ahead backup of the current config, returning the backup path. Called
/// by `api use` before mutating so `restore` can recover the previous state.
pub fn backup_config(root: &Path) -> Result<PathBuf> {
    let config = ApiConfig::load(root)?;
    let dir = backups_dir(root);
    fs::create_dir_all(&dir)?;
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
    let path = dir.join(format!("api-{timestamp}.json"));
    let payload = serde_json::to_string_pretty(&config)?;
    fs::write(&path, format!("{payload}\n"))?;
    prune_backups(root)?;
    Ok(path)
}

/// Remove backups beyond the newest `MAX_BACKUPS`, oldest first.
fn prune_backups(root: &Path) -> Result<()> {
    let dir = backups_dir(root);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let mut backups: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "json")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("api-"))
        })
        .collect();
    backups.sort();
    while backups.len() > MAX_BACKUPS {
        let oldest = backups.remove(0);
        fs::remove_file(&oldest)?;
    }
    Ok(())
}

/// List available backups (newest first), for CLI display. An absent backup
/// directory is an empty list, not an error.
pub fn list_backups(root: &Path) -> Result<Vec<String>> {
    let dir = backups_dir(root);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut backups: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("api-") && name.ends_with(".json"))
        .collect();
    backups.sort();
    backups.reverse();
    Ok(backups)
}

/// Restore the `n`-th newest backup (1 = newest) as the active config. Returns
/// the restored provider set so the caller can confirm what changed.
pub fn restore_config(root: &Path, n: usize) -> Result<ApiConfig> {
    let backups = list_backups(root)?;
    let Some(selected) = backups.get(n.saturating_sub(1)) else {
        return Err(BusError::Validation(format!(
            "no backup at index {n}; available backups: {}",
            if backups.is_empty() {
                "(none)".to_string()
            } else {
                backups.join(", ")
            }
        )));
    };
    let text = fs::read_to_string(backups_dir(root).join(selected))?;
    let restored: ApiConfig = serde_json::from_str(&text).map_err(|error| {
        BusError::Validation(format!("backup '{selected}' is not valid JSON: {error}"))
    })?;
    restored.save(root)?;
    Ok(restored)
}

/// The environment export file names written next to `api.json` (POSIX shell
/// and Windows PowerShell). A TUI launcher can `source`/dot-import the file so
/// the active provider's env reaches a manually started agent CLI.
const ENV_EXPORT_SH: &str = "env.sh";
const ENV_EXPORT_PS1: &str = "env.ps1";

/// The shell command used to fetch the API key at *source time* instead of
/// writing it into the export file. This keeps the key out of `.vibebus/`
/// entirely: the file holds `$(vibebus api key get --name X)` which resolves
/// against the OS vault when the launcher runs. Powershell uses `&`.
const KEY_GET_SH: &str = "vibebus api key get --name";
const KEY_GET_PS1: &str = "vibebus api key get --name";

/// Write (or remove) the env export files for the active provider. When the
/// config has no active provider both files are removed so a stale launcher
/// cannot inject a provider that is no longer selected. `key_stored` controls
/// whether the key command substitution is emitted (the caller resolves the
/// key presence so a missing key fails `use`, not the file writer).
pub fn write_env_export(root: &Path, config: &ApiConfig, key_stored: bool) -> Result<()> {
    let dir = config_dir(root);
    fs::create_dir_all(&dir)?;
    let Some(provider) = config.active_provider() else {
        return remove_env_export(root);
    };

    let mut vars = provider.standard_env();
    // The key variable is exported as a command substitution that resolves the
    // vault at source time; never embed the literal key.
    let key_var = provider.api_format.api_key_var();
    if key_stored {
        vars.insert(
            key_var.to_string(),
            format!("$( {KEY_GET_SH} {})", provider.name),
        );
    }

    fs::write(dir.join(ENV_EXPORT_SH), shell_export_body(&vars))?;
    fs::write(
        dir.join(ENV_EXPORT_PS1),
        powershell_export_body(&vars, key_stored, &provider.name),
    )?;
    Ok(())
}

/// Render `export KEY=value` lines for POSIX shells. Values are single-quoted
/// so spaces/`$`/backticks in URLs or templates survive; the key placeholder is
/// already a `$( ... )` command substitution that must NOT be quoted, so the
/// caller passes the raw placeholder string and we detect it here.
fn shell_export_body(vars: &HashMap<String, String>) -> String {
    let mut keys: Vec<&String> = vars.keys().collect();
    keys.sort();
    let mut out = String::from(
        "# Generated by `vibebus api use`. Source this file before starting an\n\
         # agent CLI to inject the active API provider env. The API key is\n\
         # resolved from the OS credential vault at source time; it is never\n\
         # stored in this file.\n",
    );
    for key in keys {
        let value = &vars[key];
        let rendered = if value.starts_with("$( ") {
            value.clone()
        } else {
            format!("'{}'", value.replace('\'', "'\\''"))
        };
        out.push_str(&format!("export {key}={rendered}\n"));
    }
    out
}

/// Render `$env:KEY = value` lines for Windows PowerShell. The key placeholder
/// becomes an invocation of the same CLI so no secret is written to disk.
fn powershell_export_body(
    vars: &HashMap<String, String>,
    key_stored: bool,
    provider: &str,
) -> String {
    let mut keys: Vec<&String> = vars.keys().collect();
    keys.sort();
    let mut out = String::from(
        "# Generated by `vibebus api use`. Dot-import this file before starting\n\
         # an agent CLI to inject the active API provider env. The API key is\n\
         # resolved from the OS credential vault at import time; it is never\n\
         # stored in this file.\n",
    );
    for key in keys {
        let value = &vars[key];
        if key_stored && value.starts_with("$( ") {
            out.push_str(&format!("$env:{key} = (& {KEY_GET_PS1} {provider})[0]\n"));
        } else {
            let escaped = value.replace('\'', "''");
            out.push_str(&format!("$env:{key} = '{escaped}'\n"));
        }
    }
    out
}

/// The name of the active provider, if any (used by the `api run` launcher and
/// by tests to assert the export targets the right provider).
pub fn active_provider_name(config: &ApiConfig) -> Option<&str> {
    config.active.as_deref()
}

/// Remove the env export files (used by `api unuse` and `restore` when the
/// restored config has no active provider).
pub fn remove_env_export(root: &Path) -> Result<()> {
    let dir = config_dir(root);
    for name in [ENV_EXPORT_SH, ENV_EXPORT_PS1] {
        let path = dir.join(name);
        if path.exists() {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Channel 4: opt-in config-file injection (--write-config), Claude Code
// ---------------------------------------------------------------------------

/// Write (merge) an API provider's env into the project-level Claude Code
/// config file (`<root>/.claude/settings.json`), backing up the original first
/// under `.vibebus/api/config-backups/`. Only called when `--write-config` is
/// explicitly passed to `api use` — this is a conscious opt-in that mirrors
/// cc-switch's file-based injection for the rare case where env alone is not
/// enough (e.g. Claude Code running in a context that ignores process env).
///
/// Returns the backup path so the caller can list/revert it.
pub fn write_claude_config(
    root: &Path,
    vars: &HashMap<String, String>,
    provider: &str,
) -> Result<PathBuf> {
    let config_path = root.join(".claude").join("settings.json");
    let backup = backup_file(root, &config_path, "claude-settings")?;
    let mut settings: serde_json::Value = if config_path.exists() {
        let text = fs::read_to_string(&config_path)?;
        serde_json::from_str(&text).map_err(|error| {
            BusError::Validation(format!(
                "existing Claude settings at {} is not valid JSON: {error}",
                config_path.display()
            ))
        })?
    } else {
        serde_json::json!({})
    };
    // Deep-merge the injected vars into the `env` key, preserving everything
    // else the user may have configured (hooks, MCP servers, etc.).
    let env = settings
        .as_object_mut()
        .ok_or_else(|| BusError::Validation("Claude settings is not a JSON object".into()))?
        .entry("env")
        .or_insert_with(|| serde_json::json!({}));
    let env_obj = env
        .as_object_mut()
        .ok_or_else(|| BusError::Validation("Claude settings `env` is not a JSON object".into()))?;
    for (key, value) in vars {
        env_obj.insert(key.clone(), serde_json::Value::String(value.clone()));
    }
    // Also record a non-functional key so the user knows which provider wrote
    // this and how to revert (`vibebus api unuse` restores the backup).
    env_obj.insert(
        "_vibebus_provider".into(),
        serde_json::Value::String(provider.to_string()),
    );

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = serde_json::to_string_pretty(&settings)?;
    fs::write(&config_path, format!("{payload}\n"))?;
    Ok(backup)
}

/// Restore the latest Claude Code config backup (used by `api unuse` when a
/// provider was applied with `--write-config`). If there is no backup the
/// function is a no-op; if the backup directory is absent it is also a no-op.
pub fn revert_claude_config(root: &Path) -> Result<()> {
    let target = root.join(".claude").join("settings.json");
    revert_backup(root, &target, "claude-settings")
}

/// Copy a config file into the VibeBus backup directory before mutating it.
fn backup_file(root: &Path, source: &Path, prefix: &str) -> Result<PathBuf> {
    let dir = config_dir(root).join("config-backups");
    fs::create_dir_all(&dir)?;
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
    let backup = dir.join(format!("{prefix}-{timestamp}.json"));
    if source.exists() {
        fs::copy(source, &backup)?;
    }
    Ok(backup)
}

/// Restore the latest backup for a given config file (prefix). No-op when
/// no backup exists.
fn revert_backup(root: &Path, target: &Path, prefix: &str) -> Result<()> {
    let dir = config_dir(root).join("config-backups");
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let mut backups: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(prefix))
        })
        .collect();
    backups.sort();
    let Some(latest) = backups.last() else {
        // No backup → no-op (config file may not exist or was never written).
        return Ok(());
    };
    fs::copy(latest, target)?;
    Ok(())
}



/// Local (offline) health check for a provider: config present, Base URL
/// normalized, API key resolvable from the vault. Does not make network
/// requests (the phase-2 connectivity check is a separate concern).
pub fn health_check(
    config: &ApiConfig,
    vault: &dyn ApiCredentialVault,
    project_id: &str,
    provider: &str,
) -> Result<serde_json::Value> {
    let provider_config = config.providers.get(provider).ok_or_else(|| {
        BusError::Validation(format!(
            "unknown api provider '{provider}'; configured providers: {}",
            if config.providers.is_empty() {
                "(none)".to_string()
            } else {
                config.providers.keys().cloned().collect::<Vec<_>>().join(", ")
            }
        ))
    })?;
    let base_url_ok = ApiProvider::normalize_base_url(&provider_config.base_url).is_ok();
    let key_stored = vault.load_api(project_id, provider)?.is_some();
    let active = config.active.as_deref() == Some(provider);
    let expected_vars = provider_config.standard_env().keys().cloned().collect::<Vec<_>>();
    Ok(serde_json::json!({
        "provider": provider,
        "ok": base_url_ok && key_stored,
        "baseUrl": provider_config.base_url,
        "baseUrlOk": base_url_ok,
        "format": provider_config.api_format.as_str(),
        "model": provider_config.model,
        "keyStored": key_stored,
        "active": active,
        "vaultBackend": vault.api_backend(),
        "injects": expected_vars,
    }))
}

/// Probe TCP reachability of a Base URL's host:port. Uses a std `TcpStream`
/// with a short connect timeout so no HTTP client dependency is introduced and
/// a dead host fails fast instead of hanging. This is a connectivity check, not
/// an application-level probe — an auth/route error at the HTTP layer still
/// means the endpoint is reachable.
pub fn check_provider_network(base_url: &str) -> Result<std::time::Duration> {
    let normalized = ApiProvider::normalize_base_url(base_url)?;
    let rest = normalized
        .strip_prefix("https://")
        .or_else(|| normalized.strip_prefix("http://"))
        .ok_or_else(|| BusError::Validation(format!("base URL '{base_url}' has no scheme")))?;
    let (host, port) = match rest.rsplit_once(':') {
        Some((host, port)) if port.chars().all(|c| c.is_ascii_digit()) => {
            (host.to_string(), port.parse::<u16>().unwrap_or(80))
        }
        _ => (rest.to_string(), if normalized.starts_with("https://") { 443 } else { 80 }),
    };
    let address = format!("{host}:{port}");
    let started = std::time::Instant::now();
    let stream = std::net::TcpStream::connect_timeout(
        &std::net::ToSocketAddrs::to_socket_addrs(address.as_str())
            .map_err(|error| {
                BusError::Validation(format!("cannot resolve '{address}': {error}"))
            })?
            .next()
            .ok_or_else(|| BusError::Validation(format!("no address for '{address}'")))?,
        std::time::Duration::from_secs(5),
    )
    .map_err(|error| {
        BusError::Io(std::io::Error::new(
            error.kind(),
            format!("cannot connect to {address}: {error}"),
        ))
    })?;
    let elapsed = started.elapsed();
    drop(stream);
    Ok(elapsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::MemoryCredentialVault;

    fn sample_provider() -> ApiProvider {
        ApiProvider {
            name: "voltapi".into(),
            base_url: "https://voltapi.com".into(),
            api_format: ApiFormat::Anthropic,
            model: Some("claude-sonnet-4-6".into()),
            env_template: HashMap::new(),
        }
    }

    #[test]
    fn normalize_base_url_strips_trailing_slash() {
        let normalized = ApiProvider::normalize_base_url("https://voltapi.com/").unwrap();
        assert_eq!(normalized, "https://voltapi.com");
    }

    #[test]
    fn normalize_base_url_rejects_non_http() {
        let error = ApiProvider::normalize_base_url("voltapi.com").unwrap_err();
        assert!(error.to_string().contains("must start with http:// or https://"));
    }

    #[test]
    fn api_format_parse_roundtrip() {
        for value in ["anthropic", "openai-chat", "openai-responses", "gemini"] {
            let format = ApiFormat::parse(value).unwrap();
            assert_eq!(format.as_str(), value);
        }
        assert!(ApiFormat::parse("nope").is_err());
    }

    #[test]
    fn standard_env_derives_protocol_vars_and_model() {
        let provider = sample_provider();
        let env = provider.standard_env();
        assert_eq!(env.get("ANTHROPIC_BASE_URL").unwrap(), "https://voltapi.com");
        assert_eq!(env.get("ANTHROPIC_MODEL").unwrap(), "claude-sonnet-4-6");
        // The key is NOT part of the standard env (resolved separately).
        assert!(!env.contains_key("ANTHROPIC_AUTH_TOKEN"));
    }

    #[test]
    fn config_roundtrip_via_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = ApiConfig {
            version: API_CONFIG_VERSION,
            ..ApiConfig::default()
        };
        config.providers.insert("voltapi".into(), sample_provider());
        config.active = Some("voltapi".into());
        config.save(dir.path()).unwrap();

        let loaded = ApiConfig::load(dir.path()).unwrap();
        assert_eq!(loaded.active.as_deref(), Some("voltapi"));
        assert_eq!(loaded.providers.len(), 1);
        assert_eq!(
            loaded.providers["voltapi"].base_url,
            "https://voltapi.com"
        );
        // The on-disk file must never contain a key.
        let text = std::fs::read_to_string(dir.path().join(".vibebus/api/api.json")).unwrap();
        assert!(!text.contains("AUTH_TOKEN"));
        assert!(!text.contains("sk-"));
    }

    #[test]
    fn load_missing_config_is_zero_injection_default() {
        let dir = tempfile::tempdir().unwrap();
        let config = ApiConfig::load(dir.path()).unwrap();
        assert!(config.providers.is_empty());
        assert!(config.active.is_none());
        assert!(config.active_provider().is_none());
    }

    #[test]
    fn resolve_api_env_requires_stored_key() {
        let mut config = ApiConfig::default();
        config.providers.insert("voltapi".into(), sample_provider());
        config.active = Some("voltapi".into());
        let vault = MemoryCredentialVault::default();

        let error = resolve_api_env(&config, &vault, "prj_test", None)
            .unwrap_err();
        assert!(error.to_string().contains("not stored in the OS credential vault"));
    }

    #[test]
    fn resolve_api_env_injects_key_from_vault() {
        let mut config = ApiConfig::default();
        config.providers.insert("voltapi".into(), sample_provider());
        config.active = Some("voltapi".into());
        let vault = MemoryCredentialVault::default();
        vault.store_api("prj_test", "voltapi", "sk-secret").unwrap();

        let resolved = resolve_api_env(&config, &vault, "prj_test", None)
            .unwrap()
            .expect("active provider resolves");
        assert_eq!(resolved.provider, "voltapi");
        assert_eq!(resolved.source, SecretSource::Vault);
        assert_eq!(
            resolved.vars.get("ANTHROPIC_AUTH_TOKEN").unwrap(),
            "sk-secret"
        );
        assert_eq!(
            resolved.vars.get("ANTHROPIC_BASE_URL").unwrap(),
            "https://voltapi.com"
        );
    }

    #[test]
    fn resolve_api_env_unknown_provider_is_validation_error() {
        let config = ApiConfig::default();
        let vault = MemoryCredentialVault::default();
        let error = resolve_api_env(&config, &vault, "prj_test", Some("nope"))
            .unwrap_err();
        assert!(error.to_string().contains("unknown api provider 'nope'"));
    }

    #[test]
    fn resolve_api_env_no_active_provider_is_none() {
        let config = ApiConfig::default();
        let vault = MemoryCredentialVault::default();
        assert!(resolve_api_env(&config, &vault, "prj_test", None)
            .unwrap()
            .is_none());
    }

    #[test]
    fn detect_env_conflicts_flags_different_values_only() {
        let mut vars = HashMap::new();
        vars.insert("ANTHROPIC_BASE_URL".to_string(), "https://voltapi.com".to_string());
        vars.insert("ANTHROPIC_MODEL".to_string(), "claude-sonnet-4-6".to_string());
        // Only run if these are not already set in the test environment.
        let conflicts = detect_env_conflicts(&vars);
        for conflict in &conflicts {
            assert!(conflict.current.is_some());
            assert_ne!(
                conflict.current.clone().unwrap(),
                vars.get(&conflict.var).unwrap().clone()
            );
        }
    }

    #[test]
    fn backup_and_restore_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = ApiConfig::default();
        config.providers.insert("voltapi".into(), sample_provider());
        config.active = Some("voltapi".into());
        config.save(dir.path()).unwrap();

        // First use: back up the "current" state.
        let first_backup = backup_config(dir.path()).unwrap();
        assert!(first_backup.exists());

        // Mutate: activate a different provider.
        let mut config2 = ApiConfig::load(dir.path()).unwrap();
        config2.providers.insert(
            "alt".into(),
            ApiProvider {
                name: "alt".into(),
                base_url: "https://alt.example.com".into(),
                api_format: ApiFormat::OpenAIChat,
                model: None,
                env_template: HashMap::new(),
            },
        );
        config2.active = Some("alt".into());
        config2.save(dir.path()).unwrap();

        // Restore newest backup (index 1) → back to the original state.
        let restored = restore_config(dir.path(), 1).unwrap();
        assert_eq!(restored.active.as_deref(), Some("voltapi"));
        assert_eq!(restored.providers.len(), 1);
    }

    #[test]
    fn restore_without_backups_is_validation_error() {
        let dir = tempfile::tempdir().unwrap();
        let error = restore_config(dir.path(), 1).unwrap_err();
        assert!(error.to_string().contains("no backup at index 1"));
    }

    #[test]
    fn health_check_reports_local_state() {
        let mut config = ApiConfig::default();
        config.providers.insert("voltapi".into(), sample_provider());
        config.active = Some("voltapi".into());
        let vault = MemoryCredentialVault::default();
        vault.store_api("prj_test", "voltapi", "sk-secret").unwrap();

        let health = health_check(&config, &vault, "prj_test", "voltapi").unwrap();
        assert_eq!(health["ok"], serde_json::Value::Bool(true));
        assert_eq!(health["active"], serde_json::Value::Bool(true));
        assert_eq!(health["keyStored"], serde_json::Value::Bool(true));

        // Unknown provider fails fast.
        let error = health_check(&config, &vault, "prj_test", "nope").unwrap_err();
        assert!(error.to_string().contains("unknown api provider 'nope'"));
    }

    #[test]
    fn write_env_export_generates_sh_and_ps1_without_key() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = ApiConfig::default();
        config.providers.insert("voltapi".into(), sample_provider());
        config.active = Some("voltapi".into());
        config.save(dir.path()).unwrap();

        write_env_export(dir.path(), &config, true).unwrap();
        let sh = std::fs::read_to_string(dir.path().join(".vibebus/api/env.sh")).unwrap();
        let ps1 = std::fs::read_to_string(dir.path().join(".vibebus/api/env.ps1")).unwrap();

        // Non-secret vars are exported with concrete values.
        assert!(sh.contains("export ANTHROPIC_BASE_URL='https://voltapi.com'"));
        assert!(sh.contains("export ANTHROPIC_MODEL='claude-sonnet-4-6'"));
        assert!(ps1.contains("$env:ANTHROPIC_BASE_URL = 'https://voltapi.com'"));

        // The key is a command substitution resolving the vault at source
        // time — never a literal secret in the export files.
        assert!(sh.contains("export ANTHROPIC_AUTH_TOKEN=$( vibebus api key get --name voltapi)"));
        assert!(ps1.contains("$env:ANTHROPIC_AUTH_TOKEN = (& vibebus api key get --name voltapi)[0]"));
        assert!(!sh.contains("sk-"));
        assert!(!ps1.contains("sk-"));
    }

    #[test]
    fn write_env_export_removes_files_when_no_active_provider() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = ApiConfig::default();
        config.providers.insert("voltapi".into(), sample_provider());
        config.active = Some("voltapi".into());
        config.save(dir.path()).unwrap();
        write_env_export(dir.path(), &config, true).unwrap();
        assert!(dir.path().join(".vibebus/api/env.sh").exists());

        // A config with no active provider (even though a provider exists)
        // must remove the export files so launchers cannot inject stale env.
        let mut inactive = ApiConfig::load(dir.path()).unwrap();
        inactive.active = None;
        write_env_export(dir.path(), &inactive, false).unwrap();
        assert!(!dir.path().join(".vibebus/api/env.sh").exists());
        assert!(!dir.path().join(".vibebus/api/env.ps1").exists());
    }

    #[test]
    fn remove_env_export_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        // Removing with no files present must not error.
        remove_env_export(dir.path()).unwrap();
        remove_env_export(dir.path()).unwrap();
    }

    #[test]
    fn check_provider_network_rejects_invalid_url() {
        let error = check_provider_network("not-a-url").unwrap_err();
        assert!(error.to_string().contains("must start with http:// or https://"));
    }

    #[test]
    fn check_provider_network_connects_to_listener() {
        // Bind a real TCP listener and verify the probe connects to it — this
        // exercises resolution + connect + timing without external network.
        use std::io::Read;
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1];
            let _ = stream.read(&mut buf);
        });
        let elapsed = check_provider_network(&format!("http://127.0.0.1:{port}")).unwrap();
        assert!(elapsed.as_millis() > 0 || elapsed.as_micros() > 0);
        handle.join().unwrap();
    }

    #[test]
    fn write_claude_config_merges_env_and_backs_up_original() {
        let dir = tempfile::tempdir().unwrap();
        // Seed an existing settings.json with a non-env key.
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        let original = serde_json::json!({
            "hooks": {"SessionStart": []},
            "env": {"EXISTING": "keep-me"}
        });
        std::fs::write(
            claude_dir.join("settings.json"),
            serde_json::to_string_pretty(&original).unwrap(),
        )
        .unwrap();

        let mut vars = HashMap::new();
        vars.insert("ANTHROPIC_BASE_URL".into(), "https://voltapi.com".into());
        vars.insert("ANTHROPIC_AUTH_TOKEN".into(), "sk-secret".into());
        let backup = write_claude_config(dir.path(), &vars, "voltapi").unwrap();
        assert!(backup.exists());

        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(claude_dir.join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(written["env"]["ANTHROPIC_BASE_URL"], "https://voltapi.com");
        assert_eq!(written["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-secret");
        assert_eq!(written["env"]["EXISTING"], "keep-me");
        assert_eq!(written["env"]["_vibebus_provider"], "voltapi");
        assert!(written["hooks"].is_object()); // preserved

        // Revert restores the original file.
        revert_claude_config(dir.path()).unwrap();
        let restored: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(claude_dir.join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(restored["env"]["EXISTING"], "keep-me");
        assert!(restored["env"]["ANTHROPIC_BASE_URL"].is_null());
    }

    #[test]
    fn write_claude_config_creates_settings_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let mut vars = HashMap::new();
        vars.insert("ANTHROPIC_BASE_URL".into(), "https://voltapi.com".into());
        write_claude_config(dir.path(), &vars, "voltapi").unwrap();
        let settings = dir.path().join(".claude").join("settings.json");
        assert!(settings.exists());
        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        assert_eq!(content["env"]["ANTHROPIC_BASE_URL"], "https://voltapi.com");
    }

    #[test]
    fn revert_claude_config_is_noop_without_backup() {
        let dir = tempfile::tempdir().unwrap();
        // Must not panic or error when there's nothing to revert.
        assert!(revert_claude_config(dir.path()).is_ok());
    }
}
