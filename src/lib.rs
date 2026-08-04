pub mod error;
pub mod hooks;
pub mod inject;
pub mod mcp;
pub mod models;
pub mod policy;
pub mod project;
pub mod spawn;
pub mod store;
pub mod vault;

pub use error::{BusError, Result};
pub use hooks::{AgentHook, run_agent_hook};
pub use inject::{
    ApiConfig, ApiFormat, ApiProvider, EnvConflict, ResolvedApiEnv, active_provider_name,
    backup_config, check_provider_network, detect_env_conflicts, health_check, list_backups,
    remove_env_export, resolve_api_env, restore_config, revert_claude_config, write_claude_config,
    write_env_export,
};
pub use models::*;
pub use policy::{ResponsibilityPolicy, normalize_policy_pattern, normalize_project_path};
pub use project::{database_path, discover_project, initialize_project};
pub use spawn::{
    AgentProfile, AgentProfiles, ProfileSummary, SessionRegistry, SessionStatus, SpawnOutcome,
    SpawnState, spawn_once,
};
pub use store::Bus;
pub use vault::{
    ApiCredentialVault, CredentialVault, MemoryCredentialVault, ResolvedSecret, SecretSource,
    StoredAgentCredentials, StoredApiCredential, StoredOperatorCredential, VaultCredentialStatus,
    VaultOperatorStatus, api_credential_target, credential_target, operator_credential_delivery,
    operator_credential_target, recovery_delivery, recovery_key_delivery, registration_delivery,
    resolve_agent_recovery_key, resolve_agent_token, resolve_operator_secret,
    system_api_credential_vault, system_credential_vault,
};
