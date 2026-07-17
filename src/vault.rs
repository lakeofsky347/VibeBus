use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::{AgentRecovery, AgentRegistration, BusError, RecoveryKeyView, Result};

const CREDENTIAL_FORMAT_VERSION: u32 = 1;
#[cfg(windows)]
const MAX_CREDENTIAL_BLOB_BYTES: usize =
    windows_sys::Win32::Security::Credentials::CRED_MAX_CREDENTIAL_BLOB_SIZE as usize;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoredAgentCredentials {
    pub format_version: u32,
    pub token: String,
    pub recovery_key: String,
    pub token_generation: i64,
}

impl StoredAgentCredentials {
    pub fn new(token: String, recovery_key: String, token_generation: i64) -> Self {
        Self {
            format_version: CREDENTIAL_FORMAT_VERSION,
            token,
            recovery_key,
            token_generation,
        }
    }

    fn validate(&self) -> Result<()> {
        if self.format_version != CREDENTIAL_FORMAT_VERSION {
            return Err(BusError::CredentialVault(format!(
                "unsupported stored credential format version {}",
                self.format_version
            )));
        }
        if self.token.is_empty() || self.recovery_key.is_empty() {
            return Err(BusError::CredentialVault(
                "stored token and recovery key must not be empty".into(),
            ));
        }
        if self.token_generation < 1 {
            return Err(BusError::CredentialVault(
                "stored token generation must be positive".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VaultCredentialStatus {
    pub backend: String,
    pub target: String,
    pub supported: bool,
    pub stored: bool,
    pub has_recovery_key: bool,
    pub token_generation: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretSource {
    Argument,
    Environment,
    Vault,
}

pub struct ResolvedSecret {
    pub value: String,
    pub source: SecretSource,
}

pub trait CredentialVault: Send + Sync {
    fn backend(&self) -> &'static str;

    fn supported(&self) -> bool;

    fn store(
        &self,
        project_id: &str,
        agent: &str,
        credentials: &StoredAgentCredentials,
    ) -> Result<()>;

    fn load(&self, project_id: &str, agent: &str) -> Result<Option<StoredAgentCredentials>>;

    fn delete(&self, project_id: &str, agent: &str) -> Result<bool>;

    fn status(&self, project_id: &str, agent: &str) -> Result<VaultCredentialStatus> {
        let target = credential_target(project_id, agent)?;
        if !self.supported() {
            return Ok(VaultCredentialStatus {
                backend: self.backend().into(),
                target,
                supported: false,
                stored: false,
                has_recovery_key: false,
                token_generation: None,
            });
        }

        let credentials = self.load(project_id, agent)?;
        Ok(VaultCredentialStatus {
            backend: self.backend().into(),
            target,
            supported: true,
            stored: credentials.is_some(),
            has_recovery_key: credentials.is_some(),
            token_generation: credentials.map(|value| value.token_generation),
        })
    }
}

pub fn credential_target(project_id: &str, agent: &str) -> Result<String> {
    let valid_project = !project_id.is_empty()
        && project_id.len() <= 128
        && project_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'));
    let valid_agent = !agent.is_empty()
        && agent.len() <= 64
        && agent
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'));
    if !valid_project || !valid_agent {
        return Err(BusError::Validation(
            "credential project and agent identifiers must use bounded ASCII letters, digits, '-' or '_'"
                .into(),
        ));
    }
    Ok(format!("VibeBus:{project_id}:{agent}"))
}

pub fn resolve_agent_token(
    vault: &dyn CredentialVault,
    project_id: &str,
    agent: &str,
    argument: Option<&str>,
    environment: Option<&str>,
) -> Result<ResolvedSecret> {
    if let Some(value) = argument.filter(|value| !value.is_empty()) {
        return Ok(ResolvedSecret {
            value: value.into(),
            source: SecretSource::Argument,
        });
    }
    if let Some(value) = environment.filter(|value| !value.is_empty()) {
        return Ok(ResolvedSecret {
            value: value.into(),
            source: SecretSource::Environment,
        });
    }
    if let Some(credentials) = vault.load(project_id, agent)? {
        return Ok(ResolvedSecret {
            value: credentials.token,
            source: SecretSource::Vault,
        });
    }
    Err(BusError::Validation(
        "agent token is required via an explicit token, VIBEBUS_AGENT_TOKEN, or the current-user credential vault".into(),
    ))
}

pub fn resolve_agent_recovery_key(
    vault: &dyn CredentialVault,
    project_id: &str,
    agent: &str,
    argument: Option<&str>,
) -> Result<ResolvedSecret> {
    if let Some(value) = argument.filter(|value| !value.is_empty()) {
        return Ok(ResolvedSecret {
            value: value.into(),
            source: SecretSource::Argument,
        });
    }
    if let Some(credentials) = vault.load(project_id, agent)? {
        return Ok(ResolvedSecret {
            value: credentials.recovery_key,
            source: SecretSource::Vault,
        });
    }
    Err(BusError::Validation(
        "recovery key is required explicitly or through the current-user credential vault".into(),
    ))
}

pub fn registration_delivery(
    vault: &dyn CredentialVault,
    project_id: &str,
    registration: &AgentRegistration,
    store_requested: bool,
) -> serde_json::Value {
    let plain = serde_json::json!(registration);
    if !store_requested {
        return plain;
    }
    let stored = StoredAgentCredentials::new(
        registration.token.clone(),
        registration.recovery_key.clone(),
        registration.token_generation,
    );
    match vault.store(project_id, &registration.name, &stored) {
        Ok(()) => serde_json::json!({
            "agentId": registration.agent_id,
            "name": registration.name,
            "role": registration.role,
            "tokenGeneration": registration.token_generation,
            "createdAt": registration.created_at,
            "credentials": stored_status(vault, project_id, &registration.name, &stored),
            "secretsRedacted": true
        }),
        Err(error) => storage_failure(plain, vault, project_id, &registration.name, error),
    }
}

pub fn recovery_delivery(
    vault: &dyn CredentialVault,
    project_id: &str,
    recovery: &AgentRecovery,
    store_requested: bool,
) -> serde_json::Value {
    let plain = serde_json::json!(recovery);
    if !store_requested {
        return plain;
    }
    let stored = StoredAgentCredentials::new(
        recovery.token.clone(),
        recovery.recovery_key.clone(),
        recovery.token_generation,
    );
    match vault.store(project_id, &recovery.name, &stored) {
        Ok(()) => serde_json::json!({
            "agentId": recovery.agent_id,
            "name": recovery.name,
            "role": recovery.role,
            "tokenGeneration": recovery.token_generation,
            "recoveredAt": recovery.recovered_at,
            "credentials": stored_status(vault, project_id, &recovery.name, &stored),
            "secretsRedacted": true
        }),
        Err(error) => storage_failure(plain, vault, project_id, &recovery.name, error),
    }
}

pub fn recovery_key_delivery(
    vault: &dyn CredentialVault,
    project_id: &str,
    token: &str,
    recovery: &RecoveryKeyView,
    store_requested: bool,
) -> serde_json::Value {
    let plain = serde_json::json!(recovery);
    if !store_requested {
        return plain;
    }
    let stored = StoredAgentCredentials::new(
        token.into(),
        recovery.recovery_key.clone(),
        recovery.token_generation,
    );
    match vault.store(project_id, &recovery.name, &stored) {
        Ok(()) => serde_json::json!({
            "agentId": recovery.agent_id,
            "name": recovery.name,
            "tokenGeneration": recovery.token_generation,
            "issuedAt": recovery.issued_at,
            "credentials": stored_status(vault, project_id, &recovery.name, &stored),
            "secretsRedacted": true
        }),
        Err(error) => storage_failure(plain, vault, project_id, &recovery.name, error),
    }
}

fn stored_status(
    vault: &dyn CredentialVault,
    project_id: &str,
    agent: &str,
    credentials: &StoredAgentCredentials,
) -> VaultCredentialStatus {
    VaultCredentialStatus {
        backend: vault.backend().into(),
        target: credential_target(project_id, agent)
            .unwrap_or_else(|_| "VibeBus:invalid-target".into()),
        supported: vault.supported(),
        stored: true,
        has_recovery_key: true,
        token_generation: Some(credentials.token_generation),
    }
}

fn storage_failure(
    mut plain: serde_json::Value,
    vault: &dyn CredentialVault,
    project_id: &str,
    agent: &str,
    error: BusError,
) -> serde_json::Value {
    if let Some(object) = plain.as_object_mut() {
        object.insert(
            "credentials".into(),
            serde_json::json!({
                "backend": vault.backend(),
                "target": credential_target(project_id, agent)
                    .unwrap_or_else(|_| "VibeBus:invalid-target".into()),
                "supported": vault.supported(),
                "stored": false,
                "hasRecoveryKey": false,
                "tokenGeneration": null
            }),
        );
        object.insert("secretsRedacted".into(), serde_json::Value::Bool(false));
        object.insert(
            "credentialStorageError".into(),
            serde_json::Value::String(error.to_string()),
        );
    }
    plain
}

#[derive(Default)]
pub struct MemoryCredentialVault {
    entries: Mutex<HashMap<String, StoredAgentCredentials>>,
}

impl CredentialVault for MemoryCredentialVault {
    fn backend(&self) -> &'static str {
        "memory-test-vault"
    }

    fn supported(&self) -> bool {
        true
    }

    fn store(
        &self,
        project_id: &str,
        agent: &str,
        credentials: &StoredAgentCredentials,
    ) -> Result<()> {
        credentials.validate()?;
        let target = credential_target(project_id, agent)?;
        self.entries
            .lock()
            .map_err(|_| BusError::CredentialVault("memory vault lock poisoned".into()))?
            .insert(target, credentials.clone());
        Ok(())
    }

    fn load(&self, project_id: &str, agent: &str) -> Result<Option<StoredAgentCredentials>> {
        let target = credential_target(project_id, agent)?;
        let value = self
            .entries
            .lock()
            .map_err(|_| BusError::CredentialVault("memory vault lock poisoned".into()))?
            .get(&target)
            .cloned();
        if let Some(credentials) = &value {
            credentials.validate()?;
        }
        Ok(value)
    }

    fn delete(&self, project_id: &str, agent: &str) -> Result<bool> {
        let target = credential_target(project_id, agent)?;
        Ok(self
            .entries
            .lock()
            .map_err(|_| BusError::CredentialVault("memory vault lock poisoned".into()))?
            .remove(&target)
            .is_some())
    }
}

pub fn system_credential_vault() -> Arc<dyn CredentialVault> {
    Arc::new(PlatformCredentialVault)
}

#[derive(Debug, Clone, Copy)]
struct PlatformCredentialVault;

#[cfg(windows)]
impl CredentialVault for PlatformCredentialVault {
    fn backend(&self) -> &'static str {
        "windows-credential-manager"
    }

    fn supported(&self) -> bool {
        true
    }

    fn store(
        &self,
        project_id: &str,
        agent: &str,
        credentials: &StoredAgentCredentials,
    ) -> Result<()> {
        use std::os::windows::ffi::OsStrExt;

        use windows_sys::Win32::Foundation::GetLastError;
        use windows_sys::Win32::Security::Credentials::{
            CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredWriteW,
        };

        credentials.validate()?;
        let target = credential_target(project_id, agent)?;
        let mut target_wide = std::ffi::OsStr::new(&target)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let mut blob = serde_json::to_vec(credentials)?;
        if blob.len() > MAX_CREDENTIAL_BLOB_BYTES {
            blob.fill(0);
            return Err(BusError::CredentialVault(format!(
                "serialized credential is larger than {MAX_CREDENTIAL_BLOB_BYTES} bytes"
            )));
        }

        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: target_wide.as_mut_ptr(),
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            ..Default::default()
        };
        let written = unsafe { CredWriteW(&credential, 0) };
        let error = if written == 0 {
            Some(unsafe { GetLastError() })
        } else {
            None
        };
        blob.fill(0);
        if let Some(code) = error {
            return Err(win32_error("CredWriteW", code));
        }
        Ok(())
    }

    fn load(&self, project_id: &str, agent: &str) -> Result<Option<StoredAgentCredentials>> {
        use std::{os::windows::ffi::OsStrExt, ptr::null_mut, slice};

        use windows_sys::Win32::Foundation::{ERROR_NOT_FOUND, GetLastError};
        use windows_sys::Win32::Security::Credentials::{
            CRED_TYPE_GENERIC, CREDENTIALW, CredFree, CredReadW,
        };

        let target = credential_target(project_id, agent)?;
        let target_wide = std::ffi::OsStr::new(&target)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let mut credential: *mut CREDENTIALW = null_mut();
        let read =
            unsafe { CredReadW(target_wide.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };
        if read == 0 {
            let code = unsafe { GetLastError() };
            if code == ERROR_NOT_FOUND {
                return Ok(None);
            }
            return Err(win32_error("CredReadW", code));
        }

        struct CredentialBuffer(*mut CREDENTIALW);
        impl Drop for CredentialBuffer {
            fn drop(&mut self) {
                unsafe { CredFree(self.0.cast()) };
            }
        }
        let buffer = CredentialBuffer(credential);
        let raw = unsafe { &*buffer.0 };
        let blob_len = raw.CredentialBlobSize as usize;
        if blob_len > MAX_CREDENTIAL_BLOB_BYTES || (blob_len > 0 && raw.CredentialBlob.is_null()) {
            return Err(BusError::CredentialVault(
                "Windows returned an invalid credential blob".into(),
            ));
        }
        let mut blob = unsafe { slice::from_raw_parts(raw.CredentialBlob, blob_len).to_vec() };
        if blob_len > 0 {
            unsafe { std::ptr::write_bytes(raw.CredentialBlob, 0, blob_len) };
        }
        let parsed = serde_json::from_slice::<StoredAgentCredentials>(&blob);
        blob.fill(0);
        let credentials = parsed.map_err(|error| {
            BusError::CredentialVault(format!("stored credential is not valid JSON: {error}"))
        })?;
        credentials.validate()?;
        Ok(Some(credentials))
    }

    fn delete(&self, project_id: &str, agent: &str) -> Result<bool> {
        use std::os::windows::ffi::OsStrExt;

        use windows_sys::Win32::Foundation::{ERROR_NOT_FOUND, GetLastError};
        use windows_sys::Win32::Security::Credentials::{CRED_TYPE_GENERIC, CredDeleteW};

        let target = credential_target(project_id, agent)?;
        let target_wide = std::ffi::OsStr::new(&target)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let deleted = unsafe { CredDeleteW(target_wide.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if deleted != 0 {
            return Ok(true);
        }
        let code = unsafe { GetLastError() };
        if code == ERROR_NOT_FOUND {
            return Ok(false);
        }
        Err(win32_error("CredDeleteW", code))
    }
}

#[cfg(windows)]
fn win32_error(operation: &str, code: u32) -> BusError {
    BusError::CredentialVault(format!(
        "{operation} failed with Windows error {code}: {}",
        std::io::Error::from_raw_os_error(code as i32)
    ))
}

#[cfg(not(windows))]
impl CredentialVault for PlatformCredentialVault {
    fn backend(&self) -> &'static str {
        "unsupported-platform"
    }

    fn supported(&self) -> bool {
        false
    }

    fn store(
        &self,
        _project_id: &str,
        _agent: &str,
        _credentials: &StoredAgentCredentials,
    ) -> Result<()> {
        Err(BusError::CredentialVault(
            "the system credential vault is currently supported only on Windows".into(),
        ))
    }

    fn load(&self, _project_id: &str, _agent: &str) -> Result<Option<StoredAgentCredentials>> {
        Ok(None)
    }

    fn delete(&self, _project_id: &str, _agent: &str) -> Result<bool> {
        Ok(false)
    }
}
