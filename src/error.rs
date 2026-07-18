use thiserror::Error;

#[derive(Debug, Error)]
pub enum BusError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("credential vault error: {0}")]
    CredentialVault(String),

    #[error("VibeBus project not found from {0}")]
    ProjectNotFound(String),

    #[error("agent not found: {0}")]
    AgentNotFound(String),

    #[error("authentication failed for agent: {0}")]
    Unauthorized(String),

    #[error("operator authentication failed")]
    OperatorUnauthorized,

    #[error("operator approval required: {0}")]
    OperatorApprovalRequired(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("validation failed: {0}")]
    Validation(String),
}

pub type Result<T> = std::result::Result<T, BusError>;
