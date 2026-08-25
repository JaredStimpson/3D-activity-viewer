use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFile {
    pub format_version: u32,
    pub project_id: String,
    pub revision: u64,
    pub title: String,
    pub activity_source: String,
    pub video: VideoSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VideoSettings {
    pub aspect_ratio: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub duration_seconds: f64,
}

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("project serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("project save failed: {0}")]
    Io(#[from] std::io::Error),
}

pub fn save_atomic(path: &Path, project: &ProjectFile) -> Result<(), ProjectError> {
    let temporary = path.with_extension("json.tmp");
    let backup = path.with_extension("json.bak");
    let bytes = serde_json::to_vec_pretty(project)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::File::create(&temporary)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    let _: ProjectFile = serde_json::from_slice(&fs::read(&temporary)?)?;

    if path.exists() {
        if backup.exists() {
            fs::remove_file(&backup)?;
        }
        fs::rename(path, &backup)?;
    }
    fs::rename(temporary, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_project_json() {
        let project = ProjectFile {
            format_version: 1,
            project_id: "test".into(),
            revision: 1,
            title: "Test Ride".into(),
            activity_source: "activity/activity.gpx".into(),
            video: VideoSettings {
                aspect_ratio: "16:9".into(),
                width: 1920,
                height: 1080,
                fps: 30,
                duration_seconds: 30.0,
            },
        };
        let json = serde_json::to_string(&project).expect("serialize");
        let parsed: ProjectFile = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, project);
    }
}
