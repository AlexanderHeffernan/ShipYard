use crate::git::{self, Project, PullRequest, WorkStatus};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashSet, path::Path, process::Command};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitHubStatus {
    installed: bool,
    authenticated: bool,
    account: Option<String>,
    version: Option<String>,
    scopes: Vec<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PullRequestRequest {
    repository: String,
    number: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PullRequestCommentRequest {
    repository: String,
    number: u64,
    body: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PullRequestChecks {
    overall_state: String,
    total: usize,
    passed: usize,
    failed: usize,
    pending: usize,
    neutral: usize,
    last_updated_at: Option<String>,
    checks: Vec<PullRequestCheck>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PullRequestCheck {
    id: String,
    name: String,
    workflow_name: Option<String>,
    status: String,
    conclusion: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
    url: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConversationAuthor {
    login: String,
    avatar_url: Option<String>,
    profile_url: Option<String>,
    name: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConversationEntry {
    id: String,
    kind: String,
    author: Option<ConversationAuthor>,
    body: String,
    timestamp: String,
    updated_at: Option<String>,
    state: Option<String>,
    url: Option<String>,
    path: Option<String>,
    line: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PullRequestConversation {
    viewer_login: Option<String>,
    entries: Vec<ConversationEntry>,
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
    #[serde(default)]
    status_check_rollup: Option<Vec<GhCheck>>,
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
    conclusion: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    context: Option<String>,
    #[serde(default)]
    workflow_name: Option<String>,
    #[serde(default)]
    started_at: Option<String>,
    #[serde(default)]
    completed_at: Option<String>,
    #[serde(default)]
    details_url: Option<String>,
    #[serde(default)]
    target_url: Option<String>,
}

#[derive(Deserialize)]
struct GhUser {
    login: String,
    #[serde(default)]
    avatar_url: Option<String>,
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Deserialize)]
struct GhAccount {
    login: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhCheckResponse {
    #[serde(default)]
    status_check_rollup: Option<Vec<GhCheck>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct GhIssueComment {
    id: u64,
    #[serde(default)]
    body: String,
    created_at: String,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    html_url: Option<String>,
    user: Option<GhUser>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct GhReview {
    id: u64,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    submitted_at: Option<String>,
    #[serde(default)]
    html_url: Option<String>,
    user: Option<GhUser>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct GhReviewComment {
    id: u64,
    #[serde(default)]
    body: String,
    created_at: String,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    line: Option<u64>,
    user: Option<GhUser>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct GhTimelineEvent {
    id: u64,
    event: String,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    actor: Option<GhUser>,
    #[serde(default)]
    label: Option<GhLabel>,
}

#[derive(Deserialize)]
struct GhLabel {
    name: String,
}

pub(crate) fn status() -> GitHubStatus {
    let Some(path) = executable("gh") else {
        return GitHubStatus {
            installed: false,
            authenticated: false,
            account: None,
            version: None,
            scopes: Vec::new(),
            error: Some("GitHub CLI was not found. Install it, then refresh settings.".to_owned()),
        };
    };
    let version = command(&path, None, &["--version"])
        .ok()
        .and_then(|value| value.lines().next().map(str::to_owned));
    let scopes = auth_scopes(&path);
    match command(&path, None, &["api", "user"])
        .and_then(|value| serde_json::from_str::<GhUser>(&value).map_err(|error| error.to_string()))
    {
        Ok(user) => GitHubStatus {
            installed: true,
            authenticated: true,
            account: Some(user.login),
            version,
            scopes,
            error: None,
        },
        Err(error) => GitHubStatus {
            installed: true,
            authenticated: false,
            account: None,
            version,
            scopes,
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
                    let (local_commits, remote_commits) =
                        synchronization(root, &item.head_sha, &pull_request.head_ref_oid);
                    item.pull_request = Some(hydrate_pull_request(
                        pull_request,
                        local_commits,
                        remote_commits,
                    ));
                } else if relevant(pull_request, &user.login) {
                    project.work_items.push(remote_pull_request_item(
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

pub(crate) fn checks(request: PullRequestRequest) -> Result<PullRequestChecks, String> {
    let path = executable("gh")
        .ok_or_else(|| "GitHub CLI is not installed. Install it and sign in to load checks.".to_owned())?;
    validate_request(&request)?;
    let number = request.number.to_string();
    let response = command(
        &path,
        None,
        &[
            "pr",
            "view",
            &number,
            "--repo",
            &request.repository,
            "--json",
            "statusCheckRollup",
        ],
    )
    .map_err(|error| github_api_error(error, "load checks"))
    .and_then(|value| {
        serde_json::from_str::<GhCheckResponse>(&value)
            .map_err(|error| github_api_error(error.to_string(), "parse checks"))
    })?;

    Ok(summarize_checks(
        response.status_check_rollup.unwrap_or_default(),
    ))
}

pub(crate) fn conversation(
    request: PullRequestRequest,
) -> Result<PullRequestConversation, String> {
    let path = executable("gh").ok_or_else(|| {
        "GitHub CLI is not installed. Install it and sign in to load pull request discussion."
            .to_owned()
    })?;
    validate_request(&request)?;
    let number = request.number.to_string();
    let issue_comments = paginate::<GhIssueComment>(
        &path,
        &format!(
            "repos/{}/issues/{}/comments?per_page=100",
            request.repository, number
        ),
        "load pull request comments",
    )?;
    let reviews = paginate::<GhReview>(
        &path,
        &format!(
            "repos/{}/pulls/{}/reviews?per_page=100",
            request.repository, number
        ),
        "load pull request reviews",
    )?;
    let review_comments = paginate::<GhReviewComment>(
        &path,
        &format!(
            "repos/{}/pulls/{}/comments?per_page=100",
            request.repository, number
        ),
        "load inline review comments",
    )?;

    let mut entries = Vec::with_capacity(
        issue_comments.len() + reviews.len() + review_comments.len(),
    );
    entries.extend(issue_comments.into_iter().map(issue_comment_entry));
    entries.extend(reviews.into_iter().map(review_entry));
    entries.extend(review_comments.into_iter().map(review_comment_entry));

    // Timeline events are supplementary: older GitHub installations and tokens can reject this
    // endpoint even when the actual discussion endpoints above are available.
    if let Ok(events) = paginate::<GhTimelineEvent>(
        &path,
        &format!(
            "repos/{}/issues/{}/timeline?per_page=100",
            request.repository, number
        ),
        "load pull request timeline",
    ) {
        let existing_ids: HashSet<String> = entries.iter().map(|entry| entry.id.clone()).collect();
        entries.extend(events.into_iter().filter_map(|event| {
            let entry = timeline_entry(event)?;
            (!existing_ids.contains(&entry.id)).then_some(entry)
        }));
    }

    entries.sort_by(|left, right| {
        left.timestamp
            .cmp(&right.timestamp)
            .then_with(|| left.id.cmp(&right.id))
    });

    let viewer_login = command(&path, None, &["api", "user"])
        .ok()
        .and_then(|value| serde_json::from_str::<GhUser>(&value).ok())
        .map(|user| user.login);

    Ok(PullRequestConversation {
        viewer_login,
        entries,
    })
}

pub(crate) fn post_comment(
    request: PullRequestCommentRequest,
) -> Result<ConversationEntry, String> {
    let path = executable("gh").ok_or_else(|| {
        "GitHub CLI is not installed. Install it and sign in before posting a comment.".to_owned()
    })?;
    validate_request(&PullRequestRequest {
        repository: request.repository.clone(),
        number: request.number,
    })?;
    let body = request.body.trim();
    if body.is_empty() {
        return Err("Comment cannot be empty".to_owned());
    }
    if body.chars().count() > 65_536 {
        return Err("Comment is too long for GitHub (maximum 65,536 characters)".to_owned());
    }

    let endpoint = format!(
        "repos/{}/issues/{}/comments",
        request.repository, request.number
    );
    let body_arg = format!("body={body}");
    let response = command(
        &path,
        None,
        &["api", &endpoint, "--method", "POST", "-f", &body_arg],
    )
    .map_err(|error| github_api_error(error, "post comment"))?;
    let comment = serde_json::from_str::<GhIssueComment>(&response)
        .map_err(|error| github_api_error(error.to_string(), "parse posted comment"))?;
    Ok(issue_comment_entry(comment))
}

fn validate_request(request: &PullRequestRequest) -> Result<(), String> {
    if request.number == 0 {
        return Err("A pull request number is required".to_owned());
    }
    let mut segments = request.repository.split('/');
    let valid_segment = |segment: Option<&str>| {
        segment.is_some_and(|value| {
            !value.is_empty()
                && value.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
                })
        })
    };
    if !valid_segment(segments.next()) || !valid_segment(segments.next()) || segments.next().is_some() {
        return Err("This project is not connected to a valid GitHub repository".to_owned());
    }
    Ok(())
}

fn summarize_checks(values: Vec<GhCheck>) -> PullRequestChecks {
    let checks: Vec<PullRequestCheck> = values.into_iter().filter_map(normalize_check).collect();
    let passed = checks.iter().filter(|check| check_passed(check)).count();
    let failed = checks.iter().filter(|check| check_failed(check)).count();
    let pending = checks.iter().filter(|check| check_pending(check)).count();
    let neutral = checks.len().saturating_sub(passed + failed + pending);
    let overall_state = if failed > 0 {
        "failure"
    } else if pending > 0 {
        "pending"
    } else if checks.is_empty() {
        "unknown"
    } else if passed == checks.len() {
        "success"
    } else {
        "neutral"
    };
    let last_updated_at = checks
        .iter()
        .flat_map(|check| [check.completed_at.as_ref(), check.started_at.as_ref()])
        .flatten()
        .max()
        .cloned();

    PullRequestChecks {
        overall_state: overall_state.to_owned(),
        total: checks.len(),
        passed,
        failed,
        pending,
        neutral,
        last_updated_at,
        checks,
    }
}

fn normalize_check(value: GhCheck) -> Option<PullRequestCheck> {
    let name = value.name.or(value.context)?;
    let status = value
        .status
        .or(value.state)
        .unwrap_or_else(|| "UNKNOWN".to_owned());
    let id = format!(
        "{}:{}:{}",
        name,
        value.details_url.as_deref().unwrap_or_default(),
        value.started_at.as_deref().unwrap_or_default()
    );
    Some(PullRequestCheck {
        id,
        name,
        workflow_name: value.workflow_name.filter(|workflow| !workflow.is_empty()),
        status,
        conclusion: value.conclusion.filter(|conclusion| !conclusion.is_empty()),
        started_at: value.started_at,
        completed_at: value.completed_at,
        url: value.details_url.or(value.target_url),
    })
}

fn check_passed(check: &PullRequestCheck) -> bool {
    matches!(
        check.conclusion.as_deref().unwrap_or(check.status.as_str()),
        "SUCCESS" | "NEUTRAL" | "SKIPPED"
    )
}

fn check_failed(check: &PullRequestCheck) -> bool {
    matches!(
        check.conclusion.as_deref().unwrap_or(check.status.as_str()),
        "FAILURE" | "ERROR" | "CANCELLED" | "TIMED_OUT" | "ACTION_REQUIRED"
    ) || matches!(check.status.as_str(), "FAILURE" | "ERROR")
}

fn check_pending(check: &PullRequestCheck) -> bool {
    check.conclusion.is_none()
        && matches!(
            check.status.as_str(),
            "IN_PROGRESS" | "QUEUED" | "PENDING" | "EXPECTED" | "REQUESTED"
        )
}

fn issue_comment_entry(comment: GhIssueComment) -> ConversationEntry {
    ConversationEntry {
        id: format!("comment:{}", comment.id),
        kind: "comment".to_owned(),
        author: user_author(comment.user),
        body: comment.body,
        timestamp: comment.created_at,
        updated_at: comment.updated_at,
        state: None,
        url: comment.html_url,
        path: None,
        line: None,
    }
}

fn review_entry(review: GhReview) -> ConversationEntry {
    let state = review.state.filter(|state| !state.is_empty());
    let body = review
        .body
        .filter(|body| !body.trim().is_empty())
        .or_else(|| state.as_deref().map(review_message))
        .unwrap_or_else(|| "Submitted a review.".to_owned());
    ConversationEntry {
        id: format!("review:{}", review.id),
        kind: "review".to_owned(),
        author: user_author(review.user),
        body,
        timestamp: review.submitted_at.unwrap_or_default(),
        updated_at: None,
        state,
        url: review.html_url,
        path: None,
        line: None,
    }
}

fn review_comment_entry(comment: GhReviewComment) -> ConversationEntry {
    ConversationEntry {
        id: format!("review-comment:{}", comment.id),
        kind: "reviewComment".to_owned(),
        author: user_author(comment.user),
        body: comment.body,
        timestamp: comment.created_at,
        updated_at: comment.updated_at,
        state: None,
        url: comment.html_url,
        path: comment.path,
        line: comment.line,
    }
}

fn timeline_entry(event: GhTimelineEvent) -> Option<ConversationEntry> {
    let body = timeline_message(&event.event, event.label.as_ref())?;
    Some(ConversationEntry {
        id: format!("timeline:{}", event.id),
        kind: "system".to_owned(),
        author: user_author(event.actor),
        body,
        timestamp: event.created_at.unwrap_or_default(),
        updated_at: None,
        state: None,
        url: None,
        path: None,
        line: None,
    })
}

fn user_author(user: Option<GhUser>) -> Option<ConversationAuthor> {
    user.map(|user| ConversationAuthor {
        login: user.login,
        avatar_url: user.avatar_url,
        profile_url: user.html_url,
        name: user.name,
    })
}

fn review_message(state: &str) -> String {
    match state {
        "APPROVED" => "Approved this pull request.".to_owned(),
        "CHANGES_REQUESTED" => "Requested changes on this pull request.".to_owned(),
        "DISMISSED" => "Dismissed this review.".to_owned(),
        _ => "Submitted a review.".to_owned(),
    }
}

fn timeline_message(event: &str, label: Option<&GhLabel>) -> Option<String> {
    let message = match event {
        "closed" => "Closed this pull request.",
        "merged" => "Merged this pull request.",
        "reopened" => "Reopened this pull request.",
        "ready_for_review" => "Marked this pull request ready for review.",
        "converted_to_draft" => "Converted this pull request to a draft.",
        "review_requested" => "Requested a review.",
        "review_request_removed" => "Removed a review request.",
        "assigned" => "Updated pull request assignees.",
        "unassigned" => "Updated pull request assignees.",
        "labeled" => return label.map(|label| format!("Added the **{}** label.", label.name)),
        "unlabeled" => return label.map(|label| format!("Removed the **{}** label.", label.name)),
        _ => return None,
    };
    Some(message.to_owned())
}

fn paginate<T: DeserializeOwned>(path: &str, endpoint: &str, operation: &str) -> Result<Vec<T>, String> {
    let response = command(path, None, &["api", "--paginate", "--slurp", endpoint])
        .map_err(|error| github_api_error(error, operation))?;
    let value = serde_json::from_str::<Value>(&response)
        .map_err(|error| github_api_error(error.to_string(), operation))?;
    let values = paginated_values(value);
    values
        .into_iter()
        .map(|value| serde_json::from_value(value).map_err(|error| github_api_error(error.to_string(), operation)))
        .collect()
}

fn paginated_values(value: Value) -> Vec<Value> {
    match value {
        Value::Array(pages) => pages
            .into_iter()
            .flat_map(|page| match page {
                Value::Array(items) => items,
                item => vec![item],
            })
            .collect(),
        item => vec![item],
    }
}

fn github_api_error(error: String, operation: &str) -> String {
    let lower = error.to_ascii_lowercase();
    if lower.contains("rate limit") || lower.contains("secondary rate limit") {
        return format!("GitHub API rate limit reached while trying to {operation}. Try again later.");
    }
    if lower.contains("authentication")
        || lower.contains("bad credentials")
        || lower.contains("not logged in")
        || lower.contains("http 401")
    {
        return format!("GitHub authentication is required to {operation}. Sign in with `gh auth login`.");
    }
    if lower.contains("resource not accessible")
        || lower.contains("insufficient")
        || lower.contains("forbidden")
        || lower.contains("http 403")
    {
        let scope = if operation == "post comment" {
            "Issues: write (or the classic `repo`/`public_repo` scope)"
        } else {
            "Pull requests: read and Actions: read (or the classic `repo` scope)"
        };
        return format!(
            "GitHub denied this request while trying to {operation}. The signed-in token may be missing {scope}. Refresh it with `gh auth refresh`."
        );
    }
    if lower.contains("not found") || lower.contains("http 404") {
        return format!(
            "GitHub could not find this pull request while trying to {operation}. Check repository access and the current pull request number."
        );
    }
    format!("GitHub could not {operation}: {}", error.trim())
}

fn auth_scopes(path: &str) -> Vec<String> {
    let output = Command::new(path)
        .args(["auth", "status", "--hostname", "github.com"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let Some(scopes) = text
        .lines()
        .find_map(|line| line.split_once("Token scopes:").map(|(_, value)| value))
    else {
        return Vec::new();
    };
    scopes
        .split([',', ' ', '\'', '"'])
        .map(str::trim)
        .filter(|scope| !scope.is_empty() && *scope != "Token" && *scope != "scopes:")
        .map(str::to_owned)
        .collect()
}

fn relevant(pull_request: &GhPullRequest, login: &str) -> bool {
    pull_request.author.as_ref().is_some_and(|author| author.login == login)
        || pull_request.assignees.iter().any(|assignee| assignee.login == login)
        || pull_request
            .review_requests
            .iter()
            .any(|reviewer| reviewer.login == login)
}

fn remote_pull_request_item(project_id: &str, pull_request: &GhPullRequest) -> git::WorkItem {
    git::WorkItem {
        id: pull_request_id(project_id, pull_request.number),
        project_id: project_id.to_owned(),
        branch: None,
        worktree_path: None,
        head_sha: pull_request.head_ref_oid.clone(),
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
    let check_rollup = value.status_check_rollup.as_deref().unwrap_or_default();
    let checks_pending = check_rollup.iter().any(|check| {
        matches!(check.status.as_deref(), Some("IN_PROGRESS" | "QUEUED"))
            || matches!(check.state.as_deref(), Some("PENDING"))
            || (check.conclusion.is_none() && check.state.is_none() && check.status.is_none())
    });
    let checks_failed = check_rollup.iter().any(|check| {
        matches!(
            check.conclusion.as_deref(),
            Some("FAILURE" | "CANCELLED" | "TIMED_OUT" | "ACTION_REQUIRED")
        ) || matches!(check.state.as_deref(), Some("FAILURE" | "ERROR"))
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
        base_branch: value.base_ref_name.clone(),
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
    use super::{
        github_api_error, paginated_values, parse_repository, summarize_checks, GhCheck,
    };
    use serde_json::json;

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
    fn summarizes_check_runs_for_readiness() {
        let result = summarize_checks(vec![
            GhCheck {
                conclusion: Some("SUCCESS".to_owned()),
                status: Some("COMPLETED".to_owned()),
                state: None,
                name: Some("unit tests".to_owned()),
                context: None,
                workflow_name: Some("CI".to_owned()),
                started_at: Some("2026-08-12T10:00:00Z".to_owned()),
                completed_at: Some("2026-08-12T10:01:30Z".to_owned()),
                details_url: Some("https://github.com/owner/repo/actions/runs/1".to_owned()),
                target_url: None,
            },
            GhCheck {
                conclusion: None,
                status: Some("IN_PROGRESS".to_owned()),
                state: None,
                name: Some("lint".to_owned()),
                context: None,
                workflow_name: Some("CI".to_owned()),
                started_at: Some("2026-08-12T10:02:00Z".to_owned()),
                completed_at: None,
                details_url: None,
                target_url: None,
            },
            GhCheck {
                conclusion: Some("FAILURE".to_owned()),
                status: Some("COMPLETED".to_owned()),
                state: None,
                name: None,
                context: Some("required/status".to_owned()),
                workflow_name: None,
                started_at: None,
                completed_at: None,
                details_url: None,
                target_url: Some("https://github.com/owner/repo/status".to_owned()),
            },
        ]);

        assert_eq!(result.overall_state, "failure");
        assert_eq!(result.total, 3);
        assert_eq!(result.passed, 1);
        assert_eq!(result.pending, 1);
        assert_eq!(result.failed, 1);
        assert_eq!(result.checks[0].workflow_name.as_deref(), Some("CI"));
    }

    #[test]
    fn flattens_paginated_json_without_losing_single_page_responses() {
        assert_eq!(paginated_values(json!([[{"id": 1}], [{"id": 2}]])), vec![json!({"id": 1}), json!({"id": 2})]);
        assert_eq!(paginated_values(json!([{"id": 1}])), vec![json!({"id": 1})]);
    }

    #[test]
    fn explains_rate_limits_and_missing_permissions() {
        assert!(github_api_error("API rate limit exceeded".to_owned(), "load checks").contains("rate limit"));
        assert!(github_api_error("HTTP 403: Resource not accessible by integration".to_owned(), "post comment").contains("Issues: write"));
    }
}
