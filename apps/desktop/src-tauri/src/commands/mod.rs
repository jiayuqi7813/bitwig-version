use std::path::PathBuf;

use api::{load_bound_project, run_fetch, ApiState};
use bitwig_core::{
    create_workspace, generate_snapshot_id, get_incoming_exports_dir, get_snapshots_dir, load_project_config,
    save_project_config, Change, ProjectConfig, ProjectDiff, SnapshotSummary,
};
use dawproject::build_semantic_index;
use diff_engine::diff_indexes;
use git_backend::{
    check_git_available, check_git_lfs_available, commit_all, current_branch, list_commits, pull_ff_only, push,
    set_remote as git_set_remote,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct DepsStatus {
    pub git: Option<String>,
    pub git_lfs: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AppStatus {
    pub project_name: Option<String>,
    pub branch: Option<String>,
    pub remote: Option<String>,
    pub latest_incoming: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub file_name: String,
    pub copied_to: String,
}

#[derive(Debug, Serialize)]
pub struct SaveResult {
    pub snapshot_id: String,
    pub commit_hash: String,
    pub changes: Vec<Change>,
}

#[derive(Debug, Serialize)]
pub struct RestoreResult {
    pub files_copied: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PushResult {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PullResult {
    pub result: String,
    pub commits_pulled: Option<usize>,
    pub local_ahead: Option<usize>,
    pub remote_ahead: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct AppContext {
    bwversions_path: String,
}

fn app_context_file() -> PathBuf {
    PathBuf::from(".bwversions-app.json")
}

fn save_context(path: &PathBuf) -> Result<(), String> {
    let v = AppContext {
        bwversions_path: path.to_string_lossy().to_string(),
    };
    let content = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?;
    std::fs::write(app_context_file(), content).map_err(|e| e.to_string())
}

fn load_context() -> Result<ProjectConfig, String> {
    let content = std::fs::read_to_string(app_context_file()).map_err(|e| e.to_string())?;
    let ctx: AppContext = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    load_project_config(PathBuf::from(ctx.bwversions_path).as_path()).map_err(|e| e.to_string())
}

#[cfg_attr(feature = "with-tauri", tauri::command)]
pub async fn check_dependencies() -> Result<DepsStatus, String> {
    Ok(DepsStatus {
        git: check_git_available().ok(),
        git_lfs: check_git_lfs_available().ok(),
    })
}

#[cfg_attr(feature = "with-tauri", tauri::command)]
pub async fn bind_project(project_path: String, project_name: String) -> Result<ProjectConfig, String> {
    let config = create_workspace(PathBuf::from(project_path).as_path(), &project_name).map_err(|e| e.to_string())?;
    save_context(&config.bwversions_path)?;
    Ok(config)
}

#[cfg_attr(feature = "with-tauri", tauri::command)]
pub async fn set_remote(remote_url: String) -> Result<(), String> {
    let mut config = load_context()?;
    git_set_remote(&config.project_path, &remote_url).map_err(|e| e.to_string())?;
    config.remote_url = Some(remote_url);
    save_project_config(&config).map_err(|e| e.to_string())
}

#[cfg_attr(feature = "with-tauri", tauri::command)]
pub async fn get_status() -> Result<AppStatus, String> {
    let config = load_context()?;
    let latest = std::fs::read_dir(get_incoming_exports_dir(&config.bwversions_path))
        .ok()
        .into_iter()
        .flat_map(|it| it.filter_map(|e| e.ok()))
        .map(|e| e.path())
        .filter(|p| p.extension().map(|v| v == "dawproject").unwrap_or(false))
        .max()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));

    Ok(AppStatus {
        project_name: Some(config.project_name),
        branch: current_branch(&config.project_path).ok(),
        remote: config.remote_url,
        latest_incoming: latest,
    })
}

#[cfg_attr(feature = "with-tauri", tauri::command)]
pub async fn import_latest_dawproject() -> Result<ImportResult, String> {
    let config = load_context()?;
    let latest = std::fs::read_dir(get_incoming_exports_dir(&config.bwversions_path))
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|v| v == "dawproject").unwrap_or(false))
        .max()
        .ok_or_else(|| "No incoming .dawproject found".to_string())?;

    Ok(ImportResult {
        file_name: latest
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown.dawproject".to_string()),
        copied_to: latest.to_string_lossy().to_string(),
    })
}

#[cfg_attr(feature = "with-tauri", tauri::command)]
pub async fn save_version(message: String, author: String) -> Result<SaveResult, String> {
    let config = load_context()?;
    let snapshot_id = generate_snapshot_id();
    let snapshot_dir = get_snapshots_dir(&config.bwversions_path).join(&snapshot_id);
    std::fs::create_dir_all(&snapshot_dir).map_err(|e| e.to_string())?;

    let incoming = std::fs::read_dir(get_incoming_exports_dir(&config.bwversions_path))
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|v| v == "dawproject").unwrap_or(false))
        .max()
        .ok_or_else(|| "No incoming .dawproject found".to_string())?;

    let project_out = snapshot_dir.join("project.dawproject");
    std::fs::copy(&incoming, &project_out).map_err(|e| e.to_string())?;

    let next = build_semantic_index(&project_out, &snapshot_id).map_err(|e| e.to_string())?;
    std::fs::write(
        snapshot_dir.join("semantic-index.json"),
        serde_json::to_string_pretty(&next).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let previous = std::fs::read_dir(get_snapshots_dir(&config.bwversions_path))
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.file_name().map(|n| n != snapshot_id.as_str()).unwrap_or(false))
        .max()
        .and_then(|p| std::fs::read_to_string(p.join("semantic-index.json")).ok())
        .and_then(|s| serde_json::from_str::<bitwig_core::SemanticIndex>(&s).ok());

    let diff = diff_indexes(previous.as_ref(), &next);
    std::fs::write(
        snapshot_dir.join("diff-from-parent.json"),
        serde_json::to_string_pretty(&diff).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let manifest = bitwig_core::SnapshotManifest {
        snapshot_id: snapshot_id.clone(),
        message: message.clone(),
        author,
        timestamp: chrono::Utc::now().to_rfc3339(),
        parent_snapshot_id: previous.map(|p| p.snapshot_id),
        dawproject_hash: blake3::hash(&std::fs::read(&project_out).map_err(|e| e.to_string())?)
            .to_hex()
            .to_string(),
        semantic_index_hash: blake3::hash(
            serde_json::to_string(&next)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
        .to_hex()
        .to_string(),
    };
    std::fs::write(
        snapshot_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let commit_hash = commit_all(&config.project_path, &format!("Save version: {message}"))
        .map_err(|e| e.to_string())?;

    Ok(SaveResult {
        snapshot_id,
        commit_hash,
        changes: diff.changes,
    })
}

#[cfg_attr(feature = "with-tauri", tauri::command)]
pub async fn get_history() -> Result<Vec<SnapshotSummary>, String> {
    let config = load_context()?;
    let commits = list_commits(&config.project_path).map_err(|e| e.to_string())?;
    Ok(commits
        .into_iter()
        .map(|c| SnapshotSummary {
            snapshot_id: c.hash,
            message: c.message,
            author: c.author,
            timestamp: c.timestamp,
            change_count: 0,
        })
        .collect())
}

#[cfg_attr(feature = "with-tauri", tauri::command)]
pub async fn compare_snapshots(from: String, to: String) -> Result<ProjectDiff, String> {
    let config = load_context()?;
    let read_idx = |id: &str| -> Result<bitwig_core::SemanticIndex, String> {
        let path = get_snapshots_dir(&config.bwversions_path)
            .join(id)
            .join("semantic-index.json");
        let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&raw).map_err(|e| e.to_string())
    };
    let base = read_idx(&from)?;
    let next = read_idx(&to)?;
    Ok(diff_indexes(Some(&base), &next))
}

#[cfg_attr(feature = "with-tauri", tauri::command)]
pub async fn restore_snapshot(snapshot_id: String, output_path: String) -> Result<RestoreResult, String> {
    let config = load_context()?;
    let src = get_snapshots_dir(&config.bwversions_path).join(&snapshot_id);
    let out = PathBuf::from(output_path);
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;

    let mut files = Vec::new();
    for file in ["project.dawproject", "manifest.json", "semantic-index.json"] {
        let from = src.join(file);
        if from.exists() {
            let to = out.join(file);
            std::fs::copy(&from, &to).map_err(|e| e.to_string())?;
            files.push(to.to_string_lossy().to_string());
        }
    }
    Ok(RestoreResult { files_copied: files })
}

#[cfg_attr(feature = "with-tauri", tauri::command)]
pub async fn git_push() -> Result<PushResult, String> {
    let config = load_context()?;
    let branch = current_branch(&config.project_path).unwrap_or_else(|_| "main".to_string());
    match push(&config.project_path, &branch) {
        Ok(_) => Ok(PushResult {
            success: true,
            error: None,
        }),
        Err(e) => Ok(PushResult {
            success: false,
            error: Some(e.to_string()),
        }),
    }
}

#[cfg_attr(feature = "with-tauri", tauri::command)]
pub async fn git_pull() -> Result<PullResult, String> {
    let config = load_context()?;
    let branch = current_branch(&config.project_path).unwrap_or_else(|_| "main".to_string());
    let res = pull_ff_only(&config.project_path, &branch).map_err(|e| e.to_string())?;
    let mapped = match res {
        git_backend::PullResult::UpToDate => PullResult {
            result: "up_to_date".to_string(),
            commits_pulled: None,
            local_ahead: None,
            remote_ahead: None,
        },
        git_backend::PullResult::Updated { commits_pulled } => PullResult {
            result: "updated".to_string(),
            commits_pulled: Some(commits_pulled),
            local_ahead: None,
            remote_ahead: None,
        },
        git_backend::PullResult::Diverged {
            local_ahead,
            remote_ahead,
        } => PullResult {
            result: "diverged".to_string(),
            commits_pulled: None,
            local_ahead: Some(local_ahead),
            remote_ahead: Some(remote_ahead),
        },
    };
    Ok(mapped)
}

#[cfg_attr(feature = "with-tauri", tauri::command)]
pub async fn open_folder(path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(path);
    if !path_buf.exists() {
        return Err("Path does not exist".to_string());
    }
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open")
        .arg(path_buf)
        .spawn()
        .map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg(path_buf)
        .spawn()
        .map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer")
        .arg(path_buf)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn start_background_api(api_state: ApiState) {
    let _ = run_fetch(&api_state).await;
    let _ = load_bound_project(&api_state, &PathBuf::from(".bwversions")).await;
}
