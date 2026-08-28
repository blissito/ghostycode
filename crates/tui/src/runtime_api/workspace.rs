use std::path::{Path as FsPath, PathBuf};

use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::dependencies::{ExternalTool as _, Git};

use super::{ApiError, RuntimeApiState};

#[derive(Debug, Serialize)]
pub(super) struct WorkspaceStatusResponse {
    pub(super) workspace: PathBuf,
    pub(super) git_repo: bool,
    pub(super) branch: Option<String>,
    pub(super) head: Option<String>,
    pub(super) dirty: bool,
    pub(super) staged: usize,
    pub(super) unstaged: usize,
    pub(super) untracked: usize,
    pub(super) ahead: Option<u32>,
    pub(super) behind: Option<u32>,
}

#[derive(Debug, Default)]
pub(super) struct WorkspaceGitMetadata {
    pub(super) branch: Option<String>,
    pub(super) head: Option<String>,
    pub(super) dirty: bool,
}

pub(super) async fn workspace_status(
    State(state): State<RuntimeApiState>,
) -> Result<Json<WorkspaceStatusResponse>, ApiError> {
    Ok(Json(collect_workspace_status(&state.workspace)))
}

pub(super) fn collect_workspace_status(workspace: &FsPath) -> WorkspaceStatusResponse {
    let mut status = WorkspaceStatusResponse {
        workspace: workspace.to_path_buf(),
        git_repo: false,
        branch: None,
        head: None,
        dirty: false,
        staged: 0,
        unstaged: 0,
        untracked: 0,
        ahead: None,
        behind: None,
    };

    let Some(repo_check) = run_git(workspace, &["rev-parse", "--is-inside-work-tree"]) else {
        return status;
    };
    if repo_check.trim() != "true" {
        return status;
    }

    status.git_repo = true;
    let metadata = collect_workspace_git_metadata(workspace);
    status.branch = metadata.branch;
    status.head = metadata.head;
    status.dirty = metadata.dirty;

    if let Some(porcelain) = run_git(workspace, &["status", "--porcelain=v1"]) {
        for line in porcelain.lines() {
            if line.starts_with("??") {
                status.untracked += 1;
                continue;
            }
            let chars: Vec<char> = line.chars().collect();
            if chars.len() >= 2 {
                if chars[0] != ' ' {
                    status.staged += 1;
                }
                if chars[1] != ' ' {
                    status.unstaged += 1;
                }
            }
        }
    }

    if let Some(counts) = run_git(
        workspace,
        &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"],
    ) {
        let mut parts = counts.split_whitespace();
        if let (Some(behind), Some(ahead)) = (parts.next(), parts.next()) {
            status.behind = behind.parse::<u32>().ok();
            status.ahead = ahead.parse::<u32>().ok();
        }
    }

    status
}

pub(super) fn collect_workspace_git_metadata(workspace: &FsPath) -> WorkspaceGitMetadata {
    let Some(repo_check) = run_git(workspace, &["rev-parse", "--is-inside-work-tree"]) else {
        return WorkspaceGitMetadata::default();
    };
    if repo_check.trim() != "true" {
        return WorkspaceGitMetadata::default();
    }

    WorkspaceGitMetadata {
        branch: current_git_branch(workspace),
        head: current_git_head(workspace),
        dirty: run_git(workspace, &["status", "--porcelain=v1"])
            .is_some_and(|porcelain| !porcelain.trim().is_empty()),
    }
}

fn run_git(workspace: &FsPath, args: &[&str]) -> Option<String> {
    let output = Git::output(args, workspace).ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn current_git_branch(workspace: &FsPath) -> Option<String> {
    let repo_check = run_git(workspace, &["rev-parse", "--is-inside-work-tree"])?;
    if repo_check.trim() != "true" {
        return None;
    }
    let branch = run_git(workspace, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let branch = branch.trim();
    if branch.is_empty() {
        return None;
    }
    if branch != "HEAD" {
        return Some(branch.to_string());
    }
    let short_hash = run_git(workspace, &["rev-parse", "--short", "HEAD"])?;
    let short_hash = short_hash.trim();
    (!short_hash.is_empty()).then(|| format!("detached@{short_hash}"))
}

fn current_git_head(workspace: &FsPath) -> Option<String> {
    let head = run_git(workspace, &["rev-parse", "--short", "HEAD"])?;
    let head = head.trim();
    (!head.is_empty()).then(|| head.to_string())
}
