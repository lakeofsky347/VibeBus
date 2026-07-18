use tempfile::TempDir;
use vibebus::{
    Bus, BusError, CredentialVault, MemoryCredentialVault, SecretSource, StoredAgentCredentials,
    credential_target, initialize_project, operator_credential_delivery,
    operator_credential_target, recovery_delivery, registration_delivery,
    resolve_agent_recovery_key, resolve_agent_token, resolve_operator_secret,
};

#[test]
fn memory_vault_is_project_scoped_and_respects_resolution_precedence() {
    let vault = MemoryCredentialVault::default();
    let stored = StoredAgentCredentials::new("vault-token".into(), "vault-recovery".into(), 3);
    vault.store("prj_one", "worker", &stored).unwrap();

    assert_eq!(
        credential_target("prj_one", "worker").unwrap(),
        "VibeBus:prj_one:worker"
    );
    assert!(credential_target("prj_one", "../worker").is_err());
    assert!(vault.load("prj_two", "worker").unwrap().is_none());

    let explicit = resolve_agent_token(
        &vault,
        "prj_one",
        "worker",
        Some("argument-token"),
        Some("environment-token"),
    )
    .unwrap();
    assert_eq!(explicit.value, "argument-token");
    assert_eq!(explicit.source, SecretSource::Argument);

    let environment =
        resolve_agent_token(&vault, "prj_one", "worker", None, Some("environment-token")).unwrap();
    assert_eq!(environment.value, "environment-token");
    assert_eq!(environment.source, SecretSource::Environment);

    let from_vault = resolve_agent_token(&vault, "prj_one", "worker", None, None).unwrap();
    assert_eq!(from_vault.value, "vault-token");
    assert_eq!(from_vault.source, SecretSource::Vault);
    assert_eq!(
        resolve_agent_recovery_key(&vault, "prj_one", "worker", None)
            .unwrap()
            .value,
        "vault-recovery"
    );

    let status = vault.status("prj_one", "worker").unwrap();
    assert!(status.stored);
    assert_eq!(status.token_generation, Some(3));
    assert!(vault.delete("prj_one", "worker").unwrap());
    assert!(!vault.delete("prj_one", "worker").unwrap());
}

#[test]
fn operator_vault_entry_is_separate_redacted_and_generation_tracked() {
    let project = TempDir::new().unwrap();
    let data_home = TempDir::new().unwrap();
    initialize_project(project.path(), "Operator vault", Some(data_home.path())).unwrap();
    let mut bus = Bus::open(project.path(), Some(data_home.path())).unwrap();
    let project_id = bus.project().project_id.clone();
    let vault = MemoryCredentialVault::default();

    let first = bus.initialize_operator().unwrap();
    let delivered = operator_credential_delivery(&vault, &project_id, &first);
    assert_eq!(delivered["secretRedacted"], true);
    assert!(delivered.get("operatorSecret").is_none());
    assert_eq!(
        operator_credential_target(&project_id).unwrap(),
        format!("VibeBusOperator:{project_id}")
    );
    assert_ne!(
        operator_credential_target(&project_id).unwrap(),
        credential_target(&project_id, "operator").unwrap()
    );
    assert_eq!(
        resolve_operator_secret(&vault, &project_id).unwrap(),
        first.operator_secret
    );
    assert_eq!(
        vault.operator_status(&project_id).unwrap().generation,
        Some(1)
    );

    let second = bus.rotate_operator(&first.operator_secret).unwrap();
    let delivered = operator_credential_delivery(&vault, &project_id, &second);
    assert_eq!(delivered["secretRedacted"], true);
    assert_eq!(
        vault.operator_status(&project_id).unwrap().generation,
        Some(2)
    );
    assert_eq!(
        resolve_operator_secret(&vault, &project_id).unwrap(),
        second.operator_secret
    );
    assert!(vault.delete_operator(&project_id).unwrap());
    assert!(!vault.operator_status(&project_id).unwrap().stored);
    assert!(bus.operator_status().unwrap().configured);
    assert_eq!(bus.operator_status().unwrap().generation, Some(2));
    assert!(resolve_operator_secret(&vault, &project_id).is_err());
}

#[test]
fn stored_registration_and_recovery_are_redacted_and_keep_vault_current() {
    let project = TempDir::new().unwrap();
    let data_home = TempDir::new().unwrap();
    initialize_project(
        project.path(),
        "Credential delivery",
        Some(data_home.path()),
    )
    .unwrap();
    let mut bus = Bus::open(project.path(), Some(data_home.path())).unwrap();
    let project_id = bus.project().project_id.clone();
    let vault = MemoryCredentialVault::default();

    let registration = bus
        .register_agent("vault-worker", "implementation")
        .unwrap();
    let first_token = registration.token.clone();
    let delivered = registration_delivery(&vault, &project_id, &registration, true);
    assert_eq!(delivered["secretsRedacted"], true);
    assert!(delivered.get("token").is_none());
    assert!(delivered.get("recoveryKey").is_none());

    let resolved = resolve_agent_token(&vault, &project_id, "vault-worker", None, None).unwrap();
    assert_eq!(resolved.source, SecretSource::Vault);
    bus.inbox_with_options("vault-worker", &resolved.value, true, false)
        .unwrap();

    let recovery_key =
        resolve_agent_recovery_key(&vault, &project_id, "vault-worker", None).unwrap();
    let recovery = bus
        .recover_agent("vault-worker", &recovery_key.value)
        .unwrap();
    let recovered = recovery_delivery(&vault, &project_id, &recovery, true);
    assert_eq!(recovered["secretsRedacted"], true);
    assert!(recovered.get("token").is_none());

    let current = vault.load(&project_id, "vault-worker").unwrap().unwrap();
    assert_eq!(current.token_generation, 2);
    assert_eq!(current.token, recovery.token);
    assert!(
        bus.inbox_with_options("vault-worker", &first_token, true, false)
            .is_err()
    );
    bus.inbox_with_options("vault-worker", &current.token, true, false)
        .unwrap();
}

#[test]
fn storage_failure_returns_the_only_usable_secret_pair_with_an_error_marker() {
    let project = TempDir::new().unwrap();
    let data_home = TempDir::new().unwrap();
    initialize_project(
        project.path(),
        "Credential fallback",
        Some(data_home.path()),
    )
    .unwrap();
    let mut bus = Bus::open(project.path(), Some(data_home.path())).unwrap();
    let registration = bus.register_agent("fallback-worker", "test").unwrap();

    let delivered = registration_delivery(
        &FailingVault,
        &bus.project().project_id,
        &registration,
        true,
    );
    assert_eq!(delivered["secretsRedacted"], false);
    assert_eq!(delivered["credentials"]["stored"], false);
    assert!(
        delivered["credentialStorageError"]
            .as_str()
            .unwrap()
            .contains("simulated write failure")
    );
    assert_eq!(delivered["token"], registration.token);
    assert_eq!(delivered["recoveryKey"], registration.recovery_key);

    let operator = bus.initialize_operator().unwrap();
    let operator_delivery =
        operator_credential_delivery(&FailingVault, &bus.project().project_id, &operator);
    assert_eq!(operator_delivery["secretRedacted"], false);
    assert_eq!(operator_delivery["credential"]["stored"], false);
    assert_eq!(
        operator_delivery["operatorSecret"],
        operator.operator_secret
    );
    assert!(
        operator_delivery["credentialStorageError"]
            .as_str()
            .unwrap()
            .contains("not supported")
    );
    assert_eq!(
        bus.verify_operator_secret(operator_delivery["operatorSecret"].as_str().unwrap())
            .unwrap(),
        1
    );
}

struct FailingVault;

impl CredentialVault for FailingVault {
    fn backend(&self) -> &'static str {
        "failing-test-vault"
    }

    fn supported(&self) -> bool {
        true
    }

    fn store(
        &self,
        _project_id: &str,
        _agent: &str,
        _credentials: &StoredAgentCredentials,
    ) -> vibebus::Result<()> {
        Err(BusError::CredentialVault("simulated write failure".into()))
    }

    fn load(
        &self,
        _project_id: &str,
        _agent: &str,
    ) -> vibebus::Result<Option<StoredAgentCredentials>> {
        Ok(None)
    }

    fn delete(&self, _project_id: &str, _agent: &str) -> vibebus::Result<bool> {
        Ok(false)
    }
}
