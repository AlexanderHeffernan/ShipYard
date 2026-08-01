use crate::git::{self, Project, PullRequest};
use serde::{Deserialize, Serialize};
use std::{path::Path, process::Command};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitHubStatus {
    installed: bool,
    authenticated: bool,
    account: Option<String>,
    version: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhPullRequest {
    number: u64,
    title: String,
    url: String,
    is_draft: bool,
    merge_state_status: String,
    head_ref_name: String,
    head_ref_oid: String,
    state: String,
    review_decision: String,
    status_check_rollup: Vec<GhCheck>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhCheck {
    #[serde(default)]
    conclusion: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    state: String,
}

#[derive(Deserialize)]
struct GhUser {
    login: String,
}

pub(crate) fn status() -> GitHubStatus {
    let Some(path) = executable("gh") else {
        return GitHubStatus {
            installed: false,
            authenticated: false,
            account: None,
            version: None,
            error: Some("GitHub CLI was not found. Install it, then refresh settings.".to_owned()),
        };
    };
    let version = command(&path, None, &["--version"])
        .ok()
        .and_then(|value| value.lines().next().map(str::to_owned));
    match command(&path, None, &["api", "user"])
        .and_then(|value| serde_json::from_str::<GhUser>(&value).map_err(|error| error.to_string()))
    {
        Ok(user) => GitHubStatus {
            installed: true,
            authenticated: true,
            account: Some(user.login),
            version,
            error: None,
        },
        Err(error) => GitHubStatus {
            installed: true,
            authenticated: false,
            account: None,
            version,
            error: Some(format!("Sign in with `gh auth login`: {error}")),
        },
    }
}

pub(crate) fn enrich_project(root: &Path, project: &mut Project) {
    let Some(repository) = repository_name(root) else {
        return;
    };
    project.github_repository = Some(repository.clone());
    let Some(path) = executable("gh") else {
        project.github_error = Some("GitHub CLI is not installed".to_owned());
        return;
    };
    let result = command(
        &path,
        Some(root),
        &[
            "pr",
            "list",
            "--repo",
            &repository,
            "--state",
            "all",
            "--limit",
            "100",
            "--json",
            "number,title,url,isDraft,mergeStateStatus,headRefName,headRefOid,state,reviewDecision,statusCheckRollup",
        ],
    )
    .and_then(|value| serde_json::from_str::<Vec<GhPullRequest>>(&value).map_err(|e| e.to_string()));
    match result {
        Ok(pull_requests) => {
            for item in &mut project.work_items {
                let Some(branch) = item.branch.as_deref() else {
                    continue;
                };
                let matching = pull_requests
                    .iter()
                    .filter(|pull_request| pull_request.head_ref_name == branch)
                    .collect::<Vec<_>>();
                if let Some(pull_request) = matching
                    .iter()
                    .find(|pull_request| pull_request.state == "OPEN")
                {
                    let (local_commits, remote_commits) =
                        synchronization(root, &item.head_sha, &pull_request.head_ref_oid);
                    item.pull_request = Some(hydrate_pull_request(
                        pull_request,
                        local_commits,
                        remote_commits,
                    ));
                    continue;
                }
                item.completed = matching.iter().any(|pull_request| {
                    pull_request.head_ref_oid == item.head_sha
                        && matches!(pull_request.state.as_str(), "CLOSED" | "MERGED")
                });
            }
        }
        Err(error) => project.github_error = Some(error),
    }
}

pub(crate) fn repository_name(root: &Path) -> Option<String> {
    let remote = git_text(root, &["remote", "get-url", "origin"])?;
    parse_repository(remote.trim())
}

fn synchronization(root: &Path, local_sha: &str, remote_sha: &str) -> (u32, u32) {
    if local_sha == remote_sha {
        return (0, 0);
    }
    git::ahead_behind(root, remote_sha, local_sha).unwrap_or((0, 1))
}

fn hydrate_pull_request(
    value: &GhPullRequest,
    local_commits: u32,
    remote_commits: u32,
) -> PullRequest {
    let checks_pending = value.status_check_rollup.iter().any(|check| {
        check.status == "IN_PROGRESS"
            || check.status == "QUEUED"
            || check.state == "PENDING"
            || (check.conclusion.is_empty() && check.state.is_empty())
    });
    let checks_failed = value.status_check_rollup.iter().any(|check| {
        matches!(
            check.conclusion.as_str(),
            "FAILURE" | "CANCELLED" | "TIMED_OUT" | "ACTION_REQUIRED"
        ) || matches!(check.state.as_str(), "FAILURE" | "ERROR")
    });
    let merge_state = if value.is_draft {
        "draft"
    } else if value.merge_state_status == "DIRTY" {
        "conflicting"
    } else if checks_failed {
        "checksFailed"
    } else if checks_pending {
        "checksPending"
    } else if value.review_decision == "REVIEW_REQUIRED"
        || value.review_decision == "CHANGES_REQUESTED"
        || value.merge_state_status == "BLOCKED"
    {
        "reviewRequired"
    } else {
        "ready"
    };
    PullRequest {
        number: value.number,
        title: value.title.clone(),
        url: value.url.clone(),
        draft: value.is_draft,
        mergeable: match value.merge_state_status.as_str() {
            "CLEAN" | "HAS_HOOKS" | "UNSTABLE" => Some(true),
            "DIRTY" => Some(false),
            _ => None,
        },
        merge_state: merge_state.to_owned(),
        head_branch: value.head_ref_name.clone(),
        head_sha: value.head_ref_oid.clone(),
        local_commits,
        remote_commits,
    }
}

fn parse_repository(remote: &str) -> Option<String> {
    let path = remote
        .strip_prefix("git@github.com:")
        .or_else(|| remote.strip_prefix("ssh://git@github.com/"))
        .or_else(|| remote.strip_prefix("https://github.com/"))?;
    let path = path.strip_suffix(".git").unwrap_or(path).trim_matches('/');
    (path.split('/').count() == 2).then(|| path.to_owned())
}

fn executable(name: &str) -> Option<String> {
    let output = Command::new("/bin/zsh")
        .args(["-lc", &format!("command -v {name}")])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|path| !path.is_empty())
}

fn command(path: &str, root: Option<&Path>, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new(path);
    command.args(args);
    if let Some(root) = root {
        command.current_dir(root);
    }
    let output = command.output().map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

fn git_text(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::parse_repository;

    #[test]
    fn parses_supported_github_remotes() {
        assert_eq!(
            parse_repository("git@github.com:owner/repo.git").as_deref(),
            Some("owner/repo")
        );
        assert_eq!(
            parse_repository("https://github.com/owner/repo.git").as_deref(),
            Some("owner/repo")
        );
        assert_eq!(parse_repository("https://gitlab.com/owner/repo.git"), None);
    }
}
