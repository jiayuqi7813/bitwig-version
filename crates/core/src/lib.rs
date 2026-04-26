use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("workspace does not exist at {0}")]
    WorkspaceNotFound(PathBuf),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectConfig {
    pub project_name: String,
    pub project_path: PathBuf,
    pub bwversions_path: PathBuf,
    pub remote_url: Option<String>,
    pub current_branch: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SnapshotManifest {
    pub snapshot_id: String,
    pub message: String,
    pub author: String,
    pub timestamp: String,
    pub parent_snapshot_id: Option<String>,
    pub dawproject_hash: String,
    pub semantic_index_hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SnapshotSummary {
    pub snapshot_id: String,
    pub message: String,
    pub author: String,
    pub timestamp: String,
    pub change_count: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SemanticIndex {
    pub snapshot_id: String,
    pub project_name: String,
    pub tempo: Option<f64>,
    pub time_signature: Option<String>,
    pub tracks: Vec<TrackIndex>,
    pub assets: Vec<AssetIndex>,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrackIndex {
    pub id: String,
    pub name: String,
    pub track_type: String,
    pub clips: Vec<ClipIndex>,
    pub devices: Vec<DeviceIndex>,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClipIndex {
    pub id: String,
    pub name: Option<String>,
    pub start: f64,
    pub length: f64,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceIndex {
    pub id: String,
    pub name: String,
    pub device_type: String,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AssetIndex {
    pub path: String,
    pub hash: String,
    pub asset_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectDiff {
    pub from_snapshot: Option<String>,
    pub to_snapshot: String,
    pub changes: Vec<Change>,
    pub has_conflicts: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Change {
    pub change_type: ChangeType,
    pub entity_id: String,
    pub entity_name: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ChangeType {
    TrackAdded,
    TrackRemoved,
    TrackModified,
    ClipAdded,
    ClipRemoved,
    ClipModified,
    DeviceAdded,
    DeviceRemoved,
    DeviceStateChanged,
    AssetAdded,
    AssetRemoved,
    AssetChanged,
    TempoChanged,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Conflict {
    pub entity_id: String,
    pub description: String,
    pub local_value: String,
    pub remote_value: String,
}

pub fn create_workspace(project_path: &Path, project_name: &str) -> Result<ProjectConfig> {
    let bwversions_path = project_path.join(".bwversions");
    fs::create_dir_all(get_snapshots_dir(&bwversions_path))?;
    fs::create_dir_all(get_incoming_exports_dir(&bwversions_path))?;
    fs::create_dir_all(bwversions_path.join("cache"))?;
    fs::create_dir_all(bwversions_path.join("tmp"))?;

    let now = Utc::now().to_rfc3339();
    let config = ProjectConfig {
        project_name: project_name.to_string(),
        project_path: project_path.to_path_buf(),
        bwversions_path,
        remote_url: None,
        current_branch: "main".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };

    save_project_config(&config)?;
    Ok(config)
}

pub fn load_project_config(bwversions_path: &Path) -> Result<ProjectConfig> {
    let config_path = bwversions_path.join("project.json");
    if !config_path.exists() {
        return Err(CoreError::WorkspaceNotFound(bwversions_path.to_path_buf()));
    }

    let content = fs::read_to_string(config_path)?;
    Ok(serde_json::from_str(&content)?)
}

pub fn save_project_config(config: &ProjectConfig) -> Result<()> {
    fs::create_dir_all(&config.bwversions_path)?;
    let config_path = config.bwversions_path.join("project.json");
    let mut value = serde_json::to_value(config)?;
    value["updated_at"] = serde_json::Value::String(Utc::now().to_rfc3339());
    let content = serde_json::to_string_pretty(&value)?;
    fs::write(config_path, content)?;
    Ok(())
}

pub fn get_incoming_exports_dir(bwversions_path: &Path) -> PathBuf {
    bwversions_path.join("exports").join("incoming")
}

pub fn get_snapshots_dir(bwversions_path: &Path) -> PathBuf {
    bwversions_path.join("snapshots")
}

pub fn generate_snapshot_id() -> String {
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let rand_suffix = format!("{:06x}", rand::thread_rng().gen_range(0..=0xFF_FFFF_u32));
    format!("{timestamp}-{rand_suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_id_matches_required_format() {
        let id = generate_snapshot_id();
        assert_eq!(id.len(), 22);

        let bytes = id.as_bytes();
        assert!(bytes[..8].iter().all(u8::is_ascii_digit));
        assert_eq!(bytes[8], b'-');
        assert!(bytes[9..15].iter().all(u8::is_ascii_digit));
        assert_eq!(bytes[15], b'-');
        assert!(bytes[16..].iter().all(u8::is_ascii_hexdigit));
    }
}
