pub mod error;
pub mod mcp;
pub mod models;
pub mod project;
pub mod store;
pub mod vault;

pub use error::{BusError, Result};
pub use models::*;
pub use project::{database_path, discover_project, initialize_project};
pub use store::Bus;
pub use vault::{
    CredentialVault, MemoryCredentialVault, ResolvedSecret, SecretSource, StoredAgentCredentials,
    VaultCredentialStatus, credential_target, recovery_delivery, recovery_key_delivery,
    registration_delivery, resolve_agent_recovery_key, resolve_agent_token,
    system_credential_vault,
};
