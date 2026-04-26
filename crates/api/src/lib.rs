use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use bitwig_core::{
    create_workspace, generate_snapshot_id, get_incoming_exports_dir, get_snapshots_dir, load_project_config,
    save_project_config, Change, ProjectConfig, ProjectDiff, SnapshotManifest, SnapshotSummary,
};
use chrono::Utc;
use dawproject::build_semantic_index;
use diff_engine::diff_indexes;
use git_backend::{
    commit_all, current_branch, fetch, get_file_at_commit, list_commits, pull_ff_only, push, set_remote, PullResult,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use walkdir::WalkDir;

#[derive(Clone, Default)]
pub struct ApiState {
    inner: Arc<RwLock<AppContext>>,
}

#[derive(Default)]
struct AppContext {
    config: Option<ProjectConfig>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

impl ErrorResponse {
    fn client(msg: impl Into<String>, code: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: msg.into(),
                code: code.into(),
            }),
        )
    }

    fn server(msg: impl Into<String>, code: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: msg.into(),
                code: code.into(),
            }),
        )
    }
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub bound: bool,
    pub project_name: Option<String>,
    pub branch: Option<String>,
    pub remote: Option<String>,
    pub latest_incoming: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BindRequest {
    pub project_path: String,
    pub project_name: String,
}

#[derive(Debug, Serialize)]
pub struct BindResponse {
    pub success: bool,
    pub bwversions_path: String,
}

#[derive(Debug, Deserialize)]
pub struct SetRemoteRequest {
    pub remote_url: String,
}

#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct ImportResponse {
    pub success: bool,
    pub file_name: String,
    pub copied_to: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveSnapshotRequest {
    pub message: String,
    pub author: String,
}

#[derive(Debug, Serialize)]
pub struct SaveSnapshotResponse {
    pub success: bool,
    pub snapshot_id: String,
    pub commit_hash: String,
    pub changes: Vec<Change>,
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub snapshots: Vec<SnapshotSummary>,
}

#[derive(Debug, Serialize)]
pub struct PushResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PullResponse {
    pub result: String,
    pub commits_pulled: Option<usize>,
    pub local_ahead: Option<usize>,
    pub remote_ahead: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct CompareQuery {
    pub from: Option<String>,
    pub to: String,
}

#[derive(Debug, Deserialize)]
pub struct RestoreRequest {
    pub snapshot_id: String,
    pub output_path: String,
}

#[derive(Debug, Serialize)]
pub struct RestoreResponse {
    pub success: bool,
    pub files_copied: Vec<String>,
}

pub fn app(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/projects/bind", post(bind_project))
        .route("/remote/set", post(remote_set))
        .route("/snapshots/import", post(import_latest_snapshot))
        .route("/snapshots/save", post(save_snapshot))
        .route("/history", get(history))
        .route("/git/push", post(git_push))
        .route("/git/pull", post(git_pull))
        .route("/compare", get(compare))
        .route("/restore", post(restore))
        .with_state(state)
}

pub async fn run_server(state: ApiState) -> anyhow::Result<()> {
    let addr: SocketAddr = "127.0.0.1:47321".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app(state)).await?;
    Ok(())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: "0.1.0".to_string(),
    })
}

async fn status(State(state): State<ApiState>) -> Json<StatusResponse> {
    let ctx = state.inner.read().await;
    if let Some(config) = &ctx.config {
        let incoming = latest_dawproject_file(&get_incoming_exports_dir(&config.bwversions_path))
            .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string());
        let branch = current_branch(&config.project_path).ok();
        Json(StatusResponse {
            bound: true,
            project_name: Some(config.project_name.clone()),
            branch,
            remote: config.remote_url.clone(),
            latest_incoming: incoming,
        })
    } else {
        Json(StatusResponse {
            bound: false,
            project_name: None,
            branch: None,
            remote: None,
            latest_incoming: None,
        })
    }
}

async fn bind_project(
    State(state): State<ApiState>,
    Json(req): Json<BindRequest>,
) -> impl IntoResponse {
    let project_path = PathBuf::from(&req.project_path);
    let result = create_workspace(&project_path, &req.project_name);
    match result {
        Ok(config) => {
            let mut ctx = state.inner.write().await;
            ctx.config = Some(config.clone());
            (
                StatusCode::OK,
                Json(BindResponse {
                    success: true,
                    bwversions_path: config.bwversions_path.to_string_lossy().to_string(),
                }),
            )
                .into_response()
        }
        Err(e) => ErrorResponse::server(e.to_string(), "BIND_FAILED").into_response(),
    }
}

async fn remote_set(
    State(state): State<ApiState>,
    Json(req): Json<SetRemoteRequest>,
) -> impl IntoResponse {
    let mut ctx = state.inner.write().await;
    let Some(mut config) = ctx.config.clone() else {
        return ErrorResponse::client("Project not bound", "NOT_BOUND").into_response();
    };

    if let Err(e) = set_remote(&config.project_path, &req.remote_url) {
        return ErrorResponse::server(e.to_string(), "REMOTE_SET_FAILED").into_response();
    }

    config.remote_url = Some(req.remote_url);
    if let Err(e) = save_project_config(&config) {
        return ErrorResponse::server(e.to_string(), "CONFIG_SAVE_FAILED").into_response();
    }

    ctx.config = Some(config);
    (StatusCode::OK, Json(SuccessResponse { success: true })).into_response()
}

async fn import_latest_snapshot(State(state): State<ApiState>) -> impl IntoResponse {
    let ctx = state.inner.read().await;
    let Some(config) = &ctx.config else {
        return ErrorResponse::client("Project not bound", "NOT_BOUND").into_response();
    };

    let incoming_dir = get_incoming_exports_dir(&config.bwversions_path);
    let Some(latest) = latest_dawproject_file(&incoming_dir) else {
        return ErrorResponse::client("No .dawproject file found", "NO_INCOMING").into_response();
    };

    let file_name = latest
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project.dawproject".to_string());

    (StatusCode::OK, Json(ImportResponse {
        success: true,
        file_name,
        copied_to: latest.to_string_lossy().to_string(),
    }))
        .into_response()
}

async fn save_snapshot(
    State(state): State<ApiState>,
    Json(req): Json<SaveSnapshotRequest>,
) -> impl IntoResponse {
    let ctx = state.inner.read().await;
    let Some(config) = &ctx.config else {
        return ErrorResponse::client("Project not bound", "NOT_BOUND").into_response();
    };

    let incoming_dir = get_incoming_exports_dir(&config.bwversions_path);
    let Some(latest_dawproject) = latest_dawproject_file(&incoming_dir) else {
        return ErrorResponse::client("No incoming .dawproject found", "NO_INCOMING").into_response();
    };

    let snapshot_id = generate_snapshot_id();
    let snapshot_dir = get_snapshots_dir(&config.bwversions_path).join(&snapshot_id);
    if let Err(e) = fs::create_dir_all(&snapshot_dir) {
        return ErrorResponse::server(e.to_string(), "SNAPSHOT_DIR_FAILED").into_response();
    }

    let project_file = snapshot_dir.join("project.dawproject");
    if let Err(e) = fs::copy(&latest_dawproject, &project_file) {
        return ErrorResponse::server(e.to_string(), "COPY_DAWPROJECT_FAILED").into_response();
    }

    let semantic_index = match build_semantic_index(&project_file, &snapshot_id) {
        Ok(v) => v,
        Err(e) => return ErrorResponse::server(e.to_string(), "SEMANTIC_INDEX_FAILED").into_response(),
    };

    let semantic_index_path = snapshot_dir.join("semantic-index.json");
    if let Err(e) = write_json(&semantic_index_path, &semantic_index) {
        return ErrorResponse::server(e.to_string(), "SEMANTIC_INDEX_WRITE_FAILED").into_response();
    }

    let parent_index = latest_snapshot_dir(&config.bwversions_path).and_then(|dir| {
        if dir == snapshot_dir {
            return None;
        }
        let idx = dir.join("semantic-index.json");
        fs::read_to_string(idx)
            .ok()
            .and_then(|s| serde_json::from_str::<bitwig_core::SemanticIndex>(&s).ok())
    });

    let diff = diff_indexes(parent_index.as_ref(), &semantic_index);
    let diff_path = snapshot_dir.join("diff-from-parent.json");
    if let Err(e) = write_json(&diff_path, &diff) {
        return ErrorResponse::server(e.to_string(), "DIFF_WRITE_FAILED").into_response();
    }

    let project_bytes = fs::read(&project_file).unwrap_or_default();
    let semantic_bytes = fs::read(&semantic_index_path).unwrap_or_default();

    let manifest = SnapshotManifest {
        snapshot_id: snapshot_id.clone(),
        message: req.message.clone(),
        author: req.author.clone(),
        timestamp: Utc::now().to_rfc3339(),
        parent_snapshot_id: parent_index.as_ref().map(|i| i.snapshot_id.clone()),
        dawproject_hash: blake3::hash(&project_bytes).to_hex().to_string(),
        semantic_index_hash: blake3::hash(&semantic_bytes).to_hex().to_string(),
    };
    if let Err(e) = write_json(&snapshot_dir.join("manifest.json"), &manifest) {
        return ErrorResponse::server(e.to_string(), "MANIFEST_WRITE_FAILED").into_response();
    }

    let commit_hash = match commit_all(&config.project_path, &format!("Save version: {}", req.message)) {
        Ok(hash) => hash,
        Err(e) => return ErrorResponse::server(e.to_string(), "GIT_COMMIT_FAILED").into_response(),
    };

    (StatusCode::OK, Json(SaveSnapshotResponse {
        success: true,
        snapshot_id,
        commit_hash,
        changes: diff.changes,
    }))
        .into_response()
}

async fn history(State(state): State<ApiState>) -> impl IntoResponse {
    let ctx = state.inner.read().await;
    let Some(config) = &ctx.config else {
        return ErrorResponse::client("Project not bound", "NOT_BOUND").into_response();
    };

    let commits = match list_commits(&config.project_path) {
        Ok(c) => c,
        Err(e) => return ErrorResponse::server(e.to_string(), "HISTORY_FAILED").into_response(),
    };

    let snapshots = commits
        .into_iter()
        .map(|c| SnapshotSummary {
            snapshot_id: c.hash,
            message: c.message,
            author: c.author,
            timestamp: c.timestamp,
            change_count: 0,
        })
        .collect();

    (StatusCode::OK, Json(HistoryResponse { snapshots })).into_response()
}

async fn git_push(State(state): State<ApiState>) -> impl IntoResponse {
    let ctx = state.inner.read().await;
    let Some(config) = &ctx.config else {
        return ErrorResponse::client("Project not bound", "NOT_BOUND").into_response();
    };

    let branch = current_branch(&config.project_path).unwrap_or_else(|_| "main".to_string());
    match push(&config.project_path, &branch) {
        Ok(_) => (StatusCode::OK, Json(PushResponse { success: true, error: None })).into_response(),
        Err(e) => (StatusCode::OK, Json(PushResponse {
            success: false,
            error: Some(e.to_string()),
        }))
            .into_response(),
    }
}

async fn git_pull(State(state): State<ApiState>) -> impl IntoResponse {
    let ctx = state.inner.read().await;
    let Some(config) = &ctx.config else {
        return ErrorResponse::client("Project not bound", "NOT_BOUND").into_response();
    };

    let branch = current_branch(&config.project_path).unwrap_or_else(|_| "main".to_string());
    let result = pull_ff_only(&config.project_path, &branch);
    match result {
        Ok(PullResult::UpToDate) => (StatusCode::OK, Json(PullResponse {
            result: "up_to_date".to_string(),
            commits_pulled: None,
            local_ahead: None,
            remote_ahead: None,
        }))
            .into_response(),
        Ok(PullResult::Updated { commits_pulled }) => (StatusCode::OK, Json(PullResponse {
            result: "updated".to_string(),
            commits_pulled: Some(commits_pulled),
            local_ahead: None,
            remote_ahead: None,
        }))
            .into_response(),
        Ok(PullResult::Diverged {
            local_ahead,
            remote_ahead,
        }) => (StatusCode::OK, Json(PullResponse {
            result: "diverged".to_string(),
            commits_pulled: None,
            local_ahead: Some(local_ahead),
            remote_ahead: Some(remote_ahead),
        }))
            .into_response(),
        Err(e) => ErrorResponse::server(e.to_string(), "GIT_PULL_FAILED").into_response(),
    }
}

async fn compare(State(state): State<ApiState>, Query(q): Query<CompareQuery>) -> impl IntoResponse {
    let ctx = state.inner.read().await;
    let Some(config) = &ctx.config else {
        return ErrorResponse::client("Project not bound", "NOT_BOUND").into_response();
    };

    let to_dir = get_snapshots_dir(&config.bwversions_path).join(&q.to);
    let to_idx_path = to_dir.join("semantic-index.json");
    let to_idx = match fs::read_to_string(&to_idx_path)
        .ok()
        .and_then(|s| serde_json::from_str::<bitwig_core::SemanticIndex>(&s).ok())
    {
        Some(v) => v,
        None => return ErrorResponse::client("to snapshot not found", "SNAPSHOT_NOT_FOUND").into_response(),
    };

    let from_idx = q.from.as_ref().and_then(|from| {
        fs::read_to_string(get_snapshots_dir(&config.bwversions_path).join(from).join("semantic-index.json"))
            .ok()
            .and_then(|s| serde_json::from_str::<bitwig_core::SemanticIndex>(&s).ok())
    });

    let diff: ProjectDiff = diff_indexes(from_idx.as_ref(), &to_idx);
    (StatusCode::OK, Json(diff)).into_response()
}

async fn restore(
    State(state): State<ApiState>,
    Json(req): Json<RestoreRequest>,
) -> impl IntoResponse {
    let ctx = state.inner.read().await;
    let Some(config) = &ctx.config else {
        return ErrorResponse::client("Project not bound", "NOT_BOUND").into_response();
    };

    let out_dir = PathBuf::from(&req.output_path);
    if let Err(e) = fs::create_dir_all(&out_dir) {
        return ErrorResponse::server(e.to_string(), "OUTPUT_CREATE_FAILED").into_response();
    }

    let mut files = Vec::new();
    for file in ["project.dawproject", "manifest.json", "semantic-index.json"] {
        let path_in_repo = format!(".bwversions/snapshots/{}/{}", req.snapshot_id, file);
        match get_file_at_commit(&config.project_path, &req.snapshot_id, &path_in_repo) {
            Ok(content) => {
                let out_path = out_dir.join(file);
                if let Err(e) = fs::write(&out_path, content) {
                    return ErrorResponse::server(e.to_string(), "RESTORE_WRITE_FAILED").into_response();
                }
                files.push(out_path.to_string_lossy().to_string());
            }
            Err(_) => {
                let fallback = get_snapshots_dir(&config.bwversions_path)
                    .join(&req.snapshot_id)
                    .join(file);
                if fallback.exists() {
                    let out_path = out_dir.join(file);
                    if let Err(e) = fs::copy(&fallback, &out_path) {
                        return ErrorResponse::server(e.to_string(), "RESTORE_COPY_FAILED").into_response();
                    }
                    files.push(out_path.to_string_lossy().to_string());
                }
            }
        }
    }

    (StatusCode::OK, Json(RestoreResponse {
        success: true,
        files_copied: files,
    }))
        .into_response()
}

fn latest_dawproject_file(dir: &Path) -> Option<PathBuf> {
    let mut candidates: Vec<_> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map(|e| e == "dawproject").unwrap_or(false))
        .collect();
    candidates.sort_by_key(|e| {
        e.metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    candidates.last().map(|e| e.path().to_path_buf())
}

fn latest_snapshot_dir(bwversions_path: &Path) -> Option<PathBuf> {
    let snapshots = get_snapshots_dir(bwversions_path);
    let mut dirs: Vec<_> = fs::read_dir(snapshots)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs.pop()
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> anyhow::Result<()> {
    let content = serde_json::to_string_pretty(value)?;
    fs::write(path, content)?;
    Ok(())
}

pub async fn load_bound_project(state: &ApiState, bwversions_path: &Path) -> anyhow::Result<()> {
    let config = load_project_config(bwversions_path)?;
    let mut ctx = state.inner.write().await;
    ctx.config = Some(config);
    Ok(())
}

pub async fn run_fetch(state: &ApiState) -> anyhow::Result<()> {
    let ctx = state.inner.read().await;
    if let Some(config) = &ctx.config {
        fetch(&config.project_path)?;
    }
    Ok(())
}
