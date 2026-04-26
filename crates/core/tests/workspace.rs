use std::fs;

use bitwig_core::{create_workspace, generate_snapshot_id, load_project_config, ProjectConfig};
use tempfile::tempdir;

#[test]
fn create_workspace_creates_expected_structure() {
    let tmp = tempdir().expect("tempdir");
    let project = tmp.path().join("MySong");
    fs::create_dir_all(&project).expect("mkdir project");

    let config = create_workspace(&project, "MySong").expect("create workspace");
    let bw = config.bwversions_path;

    assert!(bw.exists());
    assert!(bw.join("snapshots").is_dir());
    assert!(bw.join("exports").join("incoming").is_dir());
    assert!(bw.join("cache").is_dir());
    assert!(bw.join("tmp").is_dir());
    assert!(bw.join("project.json").is_file());

    let loaded = load_project_config(&bw).expect("load project");
    assert_eq!(loaded.project_name, "MySong");
    assert_eq!(loaded.project_path, project);
}

#[test]
fn project_config_round_trip_serde() {
    let config = ProjectConfig {
        project_name: "RoundTrip".to_string(),
        project_path: "/tmp/project".into(),
        bwversions_path: "/tmp/project/.bwversions".into(),
        remote_url: Some("git@example.com:foo/bar.git".to_string()),
        current_branch: "main".to_string(),
        created_at: "2026-04-25T00:00:00Z".to_string(),
        updated_at: "2026-04-25T00:00:00Z".to_string(),
    };

    let json = serde_json::to_string(&config).expect("serialize");
    let parsed: ProjectConfig = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(parsed.project_name, config.project_name);
    assert_eq!(parsed.project_path, config.project_path);
    assert_eq!(parsed.bwversions_path, config.bwversions_path);
    assert_eq!(parsed.remote_url, config.remote_url);
    assert_eq!(parsed.current_branch, config.current_branch);
    assert_eq!(parsed.created_at, config.created_at);
    assert_eq!(parsed.updated_at, config.updated_at);
}

#[test]
fn snapshot_id_format_check() {
    let id = generate_snapshot_id();
    let parts: Vec<_> = id.split('-').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0].len(), 8);
    assert_eq!(parts[1].len(), 6);
    assert_eq!(parts[2].len(), 6);
    assert!(parts[2].chars().all(|c| c.is_ascii_hexdigit()));
}
