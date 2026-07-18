pub mod error;
pub mod mcp;
pub mod models;
pub mod policy;
pub mod project;
pub mod store;
pub mod vault;

pub use error::{BusError, Result};
pub use models::*;
pub use policy::{ResponsibilityPolicy, normalize_policy_pattern, normalize_project_path};
pub use project::{database_path, discover_project, initialize_project};
pub use store::Bus;
pub use vault::{
    CredentialVault, MemoryCredentialVault, ResolvedSecret, SecretSource, StoredAgentCredentials,
    StoredOperatorCredential, VaultCredentialStatus, VaultOperatorStatus, credential_target,
    operator_credential_delivery, operator_credential_target, recovery_delivery,
    recovery_key_delivery, registration_delivery, resolve_agent_recovery_key, resolve_agent_token,
    resolve_operator_secret, system_credential_vault,
};
