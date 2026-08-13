use crate::git::{self, Project, PullRequest, WorkStatus};
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
    base_ref_name: String,
    review_decision: String,
    status_check_rollup: Vec<GhCheck>,
    #[serde(default)]
    assignees: Vec<GhAccount>,
    #[serde(default)]
    review_requests: Vec<GhAccount>,
    author: Option<GhAccount>,
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

#[derive(Deserialize)]
struct GhAccount {
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
    let user = command(&path, None, &["api", "user"])
        .and_then(|value| serde_json::from_str::<GhUser>(&value).map_err(|error| error.to_string()));
    let Ok(user) = user else {
        project.github_error = Some("Sign in with `gh auth login` to load pull requests".to_owned());
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
            "open",
            "--limit",
            "100",
            "--json",
            "number,title,url,isDraft,mergeStateStatus,headRefName,headRefOid,baseRefName,reviewDecision,statusCheckRollup,assignees,reviewRequests,author",
        ],
    )
    .and_then(|value| serde_json::from_str::<Vec<GhPullRequest>>(&value).map_err(|e| e.to_string()));
    match result {
        Ok(pull_requests) => {
            for pull_request in &pull_requests {
                let local_item = project.work_items.iter_mut().find(|item| {
                    item.branch.as_deref() == Some(&pull_request.head_ref_name)
                        || (item.branch.is_none() && item.head_sha == pull_request.head_ref_oid)
                });
                if let Some(item) = local_item {
                    item.id = pull_request_id(&project.id, pull_request.number);
                    if item.agent_thread_url.is_none() {
                        item.agent_thread_url = git::agent_thread_url(root, &pull_request.head_ref_oid);
                    }
                    let (local_commits, remote_commits) =
                        synchronization(root, &item.head_sha, &pull_request.head_ref_oid);
                    item.pull_request = Some(hydrate_pull_request(
                        pull_request,
                        local_commits,
                        remote_commits,
                    ));
                } else if relevant(pull_request, &user.login) {
                    project.work_items.push(remote_pull_request_item(
                        root,
                        &project.id,
                        pull_request,
                    ));
                }
            }
            project.work_items.sort_by_key(|item| std::cmp::Reverse(item.updated_at));
        }
        Err(error) => project.github_error = Some(error),
    }
}

fn relevant(pull_request: &GhPullRequest, login: &str) -> bool {
    pull_request.author.as_ref().is_some_and(|author| author.login == login)
        || pull_request.assignees.iter().any(|assignee| assignee.login == login)
        || pull_request
            .review_requests
            .iter()
            .any(|reviewer| reviewer.login == login)
}

fn remote_pull_request_item(
    root: &Path,
    project_id: &str,
    pull_request: &GhPullRequest,
) -> git::WorkItem {
    git::WorkItem {
        id: pull_request_id(project_id, pull_request.number),
        project_id: project_id.to_owned(),
        branch: None,
        worktree_path: None,
        head_sha: pull_request.head_ref_oid.clone(),
        agent_thread_url: git::agent_thread_url(root, &pull_request.head_ref_oid),
        last_commit_subject: pull_request.title.clone(),
        status: WorkStatus::Ready,
        pull_request: Some(hydrate_pull_request(pull_request, 0, 0)),
        completed: false,
        additions: 0,
        deletions: 0,
        changed_files: 0,
        ahead: 0,
        behind: 0,
        updated_at: 0,
    }
}

fn pull_request_id(project_id: &str, number: u64) -> String {
    format!("{project_id}::pull-request::{number}")
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
    let attention_state = normalized_attention_state(value, checks_pending, checks_failed);
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
        attention_state,
        head_branch: value.head_ref_name.clone(),
        base_branch: value.base_ref_name.clone(),
        head_sha: value.head_ref_oid.clone(),
        local_commits,
        remote_commits,
    }
}

fn normalized_attention_state(
    value: &GhPullRequest,
    checks_pending: bool,
    checks_failed: bool,
) -> String {
    let review = match value.review_decision.as_str() {
        "APPROVED" => "approved",
        "CHANGES_REQUESTED" => "changesRequested",
        "REVIEW_REQUIRED" => "reviewRequired",
        "" => "none",
        _ => "other",
    };
    let checks = if checks_failed {
        "failed"
    } else if checks_pending {
        "pending"
    } else if value.status_check_rollup.is_empty() {
        "none"
    } else {
        "passed"
    };
    format!("review={review}|checks={checks}")
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
    use super::{normalized_attention_state, parse_repository, GhCheck, GhPullRequest};

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

    #[test]
    fn normalizes_review_and_check_attention_without_user_content() {
        let value = GhPullRequest {
            number: 7,
            title: "private title".to_owned(),
            url: "https://github.com/owner/repo/pull/7".to_owned(),
            is_draft: false,
            merge_state_status: "CLEAN".to_owned(),
            head_ref_name: "private-branch".to_owned(),
            head_ref_oid: "sha".to_owned(),
            base_ref_name: "main".to_owned(),
            review_decision: "CHANGES_REQUESTED".to_owned(),
            status_check_rollup: vec![GhCheck {
                conclusion: "FAILURE".to_owned(),
                status: "COMPLETED".to_owned(),
                state: "".to_owned(),
            }],
            assignees: Vec::new(),
            review_requests: Vec::new(),
            author: None,
        };

        assert_eq!(
            normalized_attention_state(&value, false, true),
            "review=changesRequested|checks=failed"
        );
    }
}
