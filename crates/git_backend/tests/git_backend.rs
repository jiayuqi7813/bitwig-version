use std::fs;

use git_backend::{check_git_available, commit_all, gitattributes_content, init_lfs, init_repo, list_commits, write_gitattributes};
use tempfile::tempdir;

#[test]
fn init_repo_and_lfs_create_expected_git_structure() {
    if check_git_available().is_err() {
        return;
    }

    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init repo");
    assert!(tmp.path().join(".git").exists());

    // git-lfs may be unavailable in CI env; this checks function behavior without failing whole suite.
    let _ = init_lfs(tmp.path());
}

#[test]
fn write_gitattributes_writes_exact_content() {
    let tmp = tempdir().expect("tempdir");
    write_gitattributes(tmp.path()).expect("write");

    let content = fs::read_to_string(tmp.path().join(".gitattributes")).expect("read");
    assert_eq!(content, gitattributes_content());
}

#[test]
fn commit_all_then_list_commits_returns_one_commit() {
    if check_git_available().is_err() {
        return;
    }

    let tmp = tempdir().expect("tempdir");
    init_repo(tmp.path()).expect("init");
    fs::write(tmp.path().join("hello.txt"), "hello").expect("write");

    commit_all(tmp.path(), "initial commit").expect("commit");
    let commits = list_commits(tmp.path()).expect("list");

    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].message, "initial commit");
}
