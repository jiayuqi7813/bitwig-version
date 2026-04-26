use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, GitError>;

const GITATTRIBUTES_CONTENT: &str = "*.dawproject filter=lfs diff=lfs merge=lfs -text\n*.wav filter=lfs diff=lfs merge=lfs -text\n*.aif filter=lfs diff=lfs merge=lfs -text\n*.aiff filter=lfs diff=lfs merge=lfs -text\n*.flac filter=lfs diff=lfs merge=lfs -text\n*.mp3 filter=lfs diff=lfs merge=lfs -text\n*.ogg filter=lfs diff=lfs merge=lfs -text\n*.zip filter=lfs diff=lfs merge=lfs -text\n*.bwproject filter=lfs diff=lfs merge=lfs -text\n";

#[derive(Debug, Error)]
pub enum GitError {
    #[error("git is not installed")]
    NotInstalled,
    #[error("git-lfs is not installed")]
    LfsNotInstalled,
    #[error("git command failed: {0}")]
    CommandFailed(String),
    #[error("local and remote have diverged")]
    Diverged,
    #[error("remote is not set")]
    RemoteNotSet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub branch: String,
    pub staged: usize,
    pub unstaged: usize,
    pub untracked: usize,
    pub clean: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub author: String,
    pub timestamp: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PullResult {
    UpToDate,
    Updated { commits_pulled: usize },
    Diverged { local_ahead: usize, remote_ahead: usize },
}

fn run_git(repo_path: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .map_err(|_| GitError::NotInstalled)?;

    if !output.status.success() {
        return Err(GitError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn check_git_available() -> Result<String> {
    let output = Command::new("git")
        .arg("--version")
        .output()
        .map_err(|_| GitError::NotInstalled)?;
    if !output.status.success() {
        return Err(GitError::NotInstalled);
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn check_git_lfs_available() -> Result<String> {
    let output = Command::new("git")
        .args(["lfs", "version"])
        .output()
        .map_err(|_| GitError::LfsNotInstalled)?;
    if !output.status.success() {
        return Err(GitError::LfsNotInstalled);
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn init_repo(repo_path: &Path) -> Result<()> {
    run_git(repo_path, &["init"])?;
    Ok(())
}

pub fn init_lfs(repo_path: &Path) -> Result<()> {
    run_git(repo_path, &["lfs", "install", "--local"]).map(|_| ()).map_err(|err| match err {
        GitError::NotInstalled => GitError::LfsNotInstalled,
        GitError::CommandFailed(_) => GitError::LfsNotInstalled,
        _ => err,
    })
}

pub fn write_gitattributes(repo_path: &Path) -> Result<()> {
    std::fs::write(repo_path.join(".gitattributes"), GITATTRIBUTES_CONTENT)
        .map_err(|e| GitError::CommandFailed(e.to_string()))
}

pub fn set_remote(repo_path: &Path, remote_url: &str) -> Result<()> {
    let has_remote = run_git(repo_path, &["remote"]).map(|s| s.lines().any(|r| r == "origin"))?;
    if has_remote {
        run_git(repo_path, &["remote", "set-url", "origin", remote_url])?;
    } else {
        run_git(repo_path, &["remote", "add", "origin", remote_url])?;
    }
    Ok(())
}

pub fn get_status(repo_path: &Path) -> Result<GitStatus> {
    let branch = current_branch(repo_path)?;
    let porcelain = run_git(repo_path, &["status", "--porcelain"])?;
    let mut staged = 0;
    let mut unstaged = 0;
    let mut untracked = 0;
    for line in porcelain.lines() {
        if line.starts_with("??") {
            untracked += 1;
            continue;
        }
        let bytes = line.as_bytes();
        if !bytes.is_empty() && bytes[0] != b' ' {
            staged += 1;
        }
        if bytes.len() > 1 && bytes[1] != b' ' {
            unstaged += 1;
        }
    }
    Ok(GitStatus {
        branch,
        staged,
        unstaged,
        untracked,
        clean: staged == 0 && unstaged == 0 && untracked == 0,
    })
}

pub fn current_branch(repo_path: &Path) -> Result<String> {
    run_git(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"])
}

pub fn create_branch(repo_path: &Path, branch_name: &str) -> Result<()> {
    run_git(repo_path, &["branch", branch_name])?;
    Ok(())
}

pub fn checkout_branch(repo_path: &Path, branch_name: &str) -> Result<()> {
    run_git(repo_path, &["checkout", branch_name])?;
    Ok(())
}

pub fn commit_all(repo_path: &Path, message: &str) -> Result<String> {
    run_git(repo_path, &["add", "."])?;
    run_git(repo_path, &["-c", "user.name=Bitwig Versions", "-c", "user.email=bitwig@versions.local", "commit", "-m", message])?;
    run_git(repo_path, &["rev-parse", "HEAD"])
}

pub fn push(repo_path: &Path, branch: &str) -> Result<()> {
    ensure_remote_set(repo_path)?;
    run_git(repo_path, &["push", "origin", branch])?;
    Ok(())
}

pub fn fetch(repo_path: &Path) -> Result<()> {
    ensure_remote_set(repo_path)?;
    run_git(repo_path, &["fetch", "origin"])?;
    Ok(())
}

pub fn pull_ff_only(repo_path: &Path, branch: &str) -> Result<PullResult> {
    ensure_remote_set(repo_path)?;
    fetch(repo_path)?;

    let local_ref = run_git(repo_path, &["rev-parse", "HEAD"])?;
    let remote_ref = run_git(repo_path, &["rev-parse", &format!("origin/{branch}")])?;
    let base = run_git(repo_path, &["merge-base", "HEAD", &format!("origin/{branch}")])?;

    if local_ref == remote_ref {
        return Ok(PullResult::UpToDate);
    }

    if local_ref == base {
        run_git(repo_path, &["pull", "--ff-only", "origin", branch])?;
        let pulled = run_git(repo_path, &["rev-list", "--count", &format!("{local_ref}..{remote_ref}")])
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        return Ok(PullResult::Updated { commits_pulled: pulled });
    }

    if remote_ref == base {
        return Ok(PullResult::UpToDate);
    }

    let local_ahead = run_git(repo_path, &["rev-list", "--count", &format!("origin/{branch}..HEAD")])
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    let remote_ahead = run_git(repo_path, &["rev-list", "--count", &format!("HEAD..origin/{branch}")])
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);

    Ok(PullResult::Diverged {
        local_ahead,
        remote_ahead,
    })
}

pub fn list_commits(repo_path: &Path) -> Result<Vec<CommitInfo>> {
    let output = run_git(repo_path, &["log", "--pretty=format:%H\t%an\t%aI\t%s"])?;
    Ok(output
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(4, '\t');
            Some(CommitInfo {
                hash: parts.next()?.to_string(),
                author: parts.next()?.to_string(),
                timestamp: parts.next()?.to_string(),
                message: parts.next()?.to_string(),
            })
        })
        .collect())
}

pub fn get_file_at_commit(repo_path: &Path, commit: &str, file_path: &str) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .args(["show", &format!("{commit}:{file_path}")])
        .current_dir(repo_path)
        .output()
        .map_err(|_| GitError::NotInstalled)?;
    if !output.status.success() {
        return Err(GitError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(output.stdout)
}

fn ensure_remote_set(repo_path: &Path) -> Result<()> {
    let remotes = run_git(repo_path, &["remote"])?;
    if remotes.lines().any(|r| r == "origin") {
        Ok(())
    } else {
        Err(GitError::RemoteNotSet)
    }
}

pub fn gitattributes_content() -> &'static str {
    GITATTRIBUTES_CONTENT
}
