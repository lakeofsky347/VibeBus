use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use directories::ProjectDirs;
use uuid::Uuid;

use crate::error::{BusError, Result};
use crate::models::{ProjectInit, ProjectMetadata};

const MARKER_DIRECTORY: &str = ".vibebus";
const MARKER_FILE: &str = "project.json";

pub fn initialize_project(
    root: &Path,
    name: &str,
    data_home: Option<&Path>,
) -> Result<ProjectInit> {
    if name.trim().is_empty() {
        return Err(BusError::Validation("project name cannot be empty".into()));
    }

    fs::create_dir_all(root)?;
    let marker_dir = root.join(MARKER_DIRECTORY);
    fs::create_dir_all(&marker_dir)?;
    let marker_path = marker_dir.join(MARKER_FILE);

    let project = if marker_path.exists() {
        read_project_marker(&marker_path)?
    } else {
        let project = ProjectMetadata {
            project_id: format!("prj_{}", Uuid::new_v4().simple()),
            name: name.trim().to_owned(),
            created_at: Utc::now().to_rfc3339(),
            schema_version: 1,
        };
        let payload = serde_json::to_string_pretty(&project)?;
        fs::write(&marker_path, format!("{payload}\n"))?;
        project
    };

    let database_path = database_path(&project.project_id, data_home)?;
    if let Some(parent) = database_path.parent() {
        fs::create_dir_all(parent)?;
    }

    Ok(ProjectInit {
        project,
        marker_path: marker_path.to_string_lossy().into_owned(),
        database_path: database_path.to_string_lossy().into_owned(),
    })
}

pub fn discover_project(start: &Path) -> Result<(PathBuf, ProjectMetadata)> {
    let canonical = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
    let initial = if canonical.is_file() {
        canonical.parent().unwrap_or(&canonical).to_path_buf()
    } else {
        canonical
    };

    for candidate in initial.ancestors() {
        let marker = candidate.join(MARKER_DIRECTORY).join(MARKER_FILE);
        if marker.is_file() {
            return Ok((candidate.to_path_buf(), read_project_marker(&marker)?));
        }
    }

    Err(BusError::ProjectNotFound(
        start.to_string_lossy().into_owned(),
    ))
}

pub fn database_path(project_id: &str, data_home: Option<&Path>) -> Result<PathBuf> {
    let base = match data_home {
        Some(path) => path.to_path_buf(),
        None => {
            if let Some(override_path) = std::env::var_os("VIBEBUS_DATA_HOME") {
                PathBuf::from(override_path)
            } else if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
                PathBuf::from(local_app_data).join("VibeBus")
            } else {
                ProjectDirs::from("dev", "VibeBus", "VibeBus")
                    .ok_or_else(|| {
                        BusError::Validation(
                            "cannot resolve local application data directory".into(),
                        )
                    })?
                    .data_local_dir()
                    .to_path_buf()
            }
        }
    };

    Ok(base.join("projects").join(project_id).join("vibebus.db"))
}

fn read_project_marker(marker_path: &Path) -> Result<ProjectMetadata> {
    let content = fs::read_to_string(marker_path)?;
    let project: ProjectMetadata = serde_json::from_str(&content)?;
    if !project.project_id.starts_with("prj_") || project.project_id.len() < 12 {
        return Err(BusError::Validation(format!(
            "invalid project id in {}",
            marker_path.display()
        )));
    }
    Ok(project)
}
