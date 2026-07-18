use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::{BusError, Result};

const POLICY_RELATIVE_PATH: &str = ".vibebus/responsibility.json";
const POLICY_VERSION: u32 = 1;
const MAX_POLICY_BYTES: u64 = 64 * 1024;
const MAX_POLICY_ROLES: usize = 128;
const MAX_PATTERNS_PER_ROLE: usize = 256;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PolicyDocument {
    version: u32,
    #[serde(default)]
    default_allowed_paths: Vec<String>,
    #[serde(default)]
    roles: BTreeMap<String, RoleDocument>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RoleDocument {
    #[serde(default)]
    allowed_paths: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ResponsibilityPolicy {
    configured: bool,
    source_path: String,
    sha256: Option<String>,
    default_allowed_paths: Vec<String>,
    roles: BTreeMap<String, Vec<String>>,
}

impl ResponsibilityPolicy {
    pub fn load(project_root: &Path) -> Result<Self> {
        let path = project_root.join(POLICY_RELATIVE_PATH);
        if !path.exists() {
            return Ok(Self {
                configured: false,
                source_path: POLICY_RELATIVE_PATH.to_owned(),
                sha256: None,
                default_allowed_paths: vec!["**".to_owned()],
                roles: BTreeMap::new(),
            });
        }
        if !path.is_file() {
            return Err(BusError::Validation(format!(
                "responsibility policy '{}' must be a file",
                path.display()
            )));
        }
        let metadata = fs::metadata(&path)?;
        if metadata.len() > MAX_POLICY_BYTES {
            return Err(BusError::Validation(format!(
                "responsibility policy exceeds {MAX_POLICY_BYTES} bytes"
            )));
        }
        let bytes = fs::read(&path)?;
        let document: PolicyDocument = serde_json::from_slice(&bytes).map_err(|error| {
            BusError::Validation(format!("responsibility policy is invalid JSON: {error}"))
        })?;
        if document.version != POLICY_VERSION {
            return Err(BusError::Validation(format!(
                "responsibility policy version {} is unsupported; expected {POLICY_VERSION}",
                document.version
            )));
        }
        if document.roles.len() > MAX_POLICY_ROLES {
            return Err(BusError::Validation(format!(
                "responsibility policy can define at most {MAX_POLICY_ROLES} roles"
            )));
        }

        let default_allowed_paths =
            normalize_patterns("defaultAllowedPaths", document.default_allowed_paths)?;
        let mut roles = BTreeMap::new();
        for (role, role_document) in document.roles {
            let role = role.trim();
            if role.is_empty() || role.len() > 128 {
                return Err(BusError::Validation(
                    "responsibility role names must be 1-128 UTF-8 bytes".into(),
                ));
            }
            roles.insert(
                role.to_owned(),
                normalize_patterns(
                    &format!("roles.{role}.allowedPaths"),
                    role_document.allowed_paths,
                )?,
            );
        }
        let sha256 = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Ok(Self {
            configured: true,
            source_path: POLICY_RELATIVE_PATH.to_owned(),
            sha256: Some(sha256),
            default_allowed_paths,
            roles,
        })
    }

    pub fn configured(&self) -> bool {
        self.configured
    }

    pub fn source_path(&self) -> &str {
        &self.source_path
    }

    pub fn sha256(&self) -> Option<&str> {
        self.sha256.as_deref()
    }

    pub fn allowed_paths(&self, role: &str) -> Vec<String> {
        self.roles
            .get(role)
            .unwrap_or(&self.default_allowed_paths)
            .clone()
    }

    pub fn allows(&self, role: &str, project_path: &str) -> bool {
        self.roles
            .get(role)
            .unwrap_or(&self.default_allowed_paths)
            .iter()
            .any(|pattern| policy_pattern_matches(pattern, project_path))
    }
}

pub fn normalize_project_path(path: &str) -> Result<String> {
    let trimmed = path.trim();
    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute()
        || trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || trimmed.as_bytes().get(1).is_some_and(|byte| *byte == b':')
    {
        return Err(BusError::Validation("path must be project-relative".into()));
    }
    let normalized = trimmed.replace('\\', "/").trim_matches('/').to_owned();
    if normalized.is_empty()
        || normalized.starts_with("..")
        || normalized
            .split('/')
            .any(|segment| segment == ".." || segment.is_empty())
        || normalized.contains('*')
    {
        return Err(BusError::Validation(
            "path must be project-relative, cannot contain '..', empty segments, or wildcards"
                .into(),
        ));
    }
    Ok(platform_normalize(normalized))
}

pub fn normalize_policy_pattern(pattern: &str) -> Result<String> {
    let trimmed = pattern.trim().replace('\\', "/");
    if trimmed == "**" {
        return Ok(trimmed);
    }
    if let Some(base) = trimmed.strip_suffix("/**") {
        if base.contains('*') {
            return Err(BusError::Validation(format!(
                "responsibility path pattern '{pattern}' may only use a terminal '/**'"
            )));
        }
        return Ok(format!("{}/**", normalize_project_path(base)?));
    }
    if trimmed.contains('*') {
        return Err(BusError::Validation(format!(
            "responsibility path pattern '{pattern}' may only use '**' or a terminal '/**'"
        )));
    }
    normalize_project_path(&trimmed)
}

pub fn policy_pattern_matches(pattern: &str, project_path: &str) -> bool {
    if pattern == "**" {
        return true;
    }
    if let Some(base) = pattern.strip_suffix("/**") {
        return project_path == base
            || project_path
                .strip_prefix(base)
                .is_some_and(|suffix| suffix.starts_with('/'));
    }
    pattern == project_path
}

fn normalize_patterns(label: &str, patterns: Vec<String>) -> Result<Vec<String>> {
    if patterns.len() > MAX_PATTERNS_PER_ROLE {
        return Err(BusError::Validation(format!(
            "{label} can contain at most {MAX_PATTERNS_PER_ROLE} patterns"
        )));
    }
    let mut normalized = patterns
        .iter()
        .map(|pattern| normalize_policy_pattern(pattern))
        .collect::<Result<Vec<_>>>()?;
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn platform_normalize(path: String) -> String {
    if cfg!(windows) {
        path.to_ascii_lowercase()
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responsibility_patterns_are_bounded_and_match_directories() {
        assert_eq!(normalize_policy_pattern(r"src\**").unwrap(), "src/**");
        assert!(policy_pattern_matches("src/**", "src"));
        assert!(policy_pattern_matches("src/**", "src/store.rs"));
        assert!(!policy_pattern_matches("src/**", "scripts/test.ps1"));
        assert!(normalize_policy_pattern("src/*.rs").is_err());
        assert!(normalize_project_path("../outside").is_err());
    }
}
