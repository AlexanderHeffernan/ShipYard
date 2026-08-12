use super::{command, remote, repository};
use serde::{Deserialize, Serialize};
use std::path::Path;

const MAX_COMMITS: usize = 200;
const MISSING_HEAD: &str = "0000000000000000000000000000000000000000";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemCommitsRequest {
    pub(super) project_path: String,
    pub(super) project_id: String,
    pub(super) branch: Option<String>,
    pub(super) worktree_path: Option<String>,
    pub(super) head_sha: String,
    pub(super) default_branch: Option<String>,
    #[serde(default)]
    pub(super) pull_request_number: Option<u64>,
    #[serde(default)]
    pub(super) pull_request_base_branch: Option<String>,
    #[serde(default)]
    pub(super) pull_request_head_sha: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemCommitHistory {
    pub(super) commits: Vec<WorkItemCommit>,
    pub(super) total: usize,
    pub(super) has_more: bool,
    pub(super) source: String,
    pub(super) comparison_label: Option<String>,
    pub(super) head_sha: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemCommit {
    pub(super) sha: String,
    pub(super) short_sha: String,
    pub(super) subject: String,
    pub(super) body: String,
    pub(super) author_name: String,
    pub(super) author_email: String,
    pub(super) authored_at: u64,
    pub(super) committed_at: u64,
    pub(super) verification: String,
    pub(super) verification_signer: Option<String>,
    pub(super) parent_count: usize,
    pub(super) additions: u64,
    pub(super) deletions: u64,
    pub(super) changed_files: usize,
}

struct CommitReferences {
    head: String,
    range: Option<String>,
    comparison_label: Option<String>,
    source: String,
}

pub fn read(request: WorkItemCommitsRequest) -> Result<WorkItemCommitHistory, String> {
    let (project_root, common_dir) = repository::resolve(&request.project_path)?;
    if repository::path_string(&common_dir) != request.project_id {
        return Err("work item does not belong to the selected project".to_owned());
    }

    let checkout = request
        .worktree_path
        .as_deref()
        .map(|path| repository::validate_worktree(&request.project_id, path))
        .transpose()?;
    let source = history_source(&request, checkout.is_some());
    validate_target(&project_root, checkout.as_deref(), &request)?;

    if missing_head(&request.head_sha) {
        return Ok(empty_history(source));
    }

    let references = commit_references(&project_root, &request, source)?;
    let target = references
        .range
        .as_deref()
        .unwrap_or(&references.head);
    let total = commit_count(&project_root, target)?;
    if total == 0 {
        return Ok(WorkItemCommitHistory {
            commits: Vec::new(),
            total,
            has_more: false,
            source: references.source,
            comparison_label: references.comparison_label,
            head_sha: Some(references.head),
        });
    }

    let output = commit_log(&project_root, target)?;
    let mut commits = parse_commits(&output);
    commits.truncate(MAX_COMMITS);
    Ok(WorkItemCommitHistory {
        commits,
        total,
        has_more: total > MAX_COMMITS,
        source: references.source,
        comparison_label: references.comparison_label,
        head_sha: Some(references.head),
    })
}

fn history_source(request: &WorkItemCommitsRequest, has_checkout: bool) -> &'static str {
    if request.pull_request_number.is_some() && request.branch.is_none() && !has_checkout {
        "pullRequest"
    } else if request.branch.is_some() {
        "local"
    } else {
        "detached"
    }
}

fn empty_history(source: &str) -> WorkItemCommitHistory {
    WorkItemCommitHistory {
        commits: Vec::new(),
        total: 0,
        has_more: false,
        source: source.to_owned(),
        comparison_label: None,
        head_sha: None,
    }
}

fn missing_head(head: &str) -> bool {
    head.is_empty() || head == MISSING_HEAD || head.chars().all(|character| character == '0')
}

fn validate_target(
    project_root: &Path,
    checkout: Option<&Path>,
    request: &WorkItemCommitsRequest,
) -> Result<(), String> {
    if missing_head(&request.head_sha) {
        return Ok(());
    }

    if let Some(branch) = request.branch.as_deref() {
        let reference = format!("refs/heads/{branch}");
        let branch_head = command::text(project_root, &["rev-parse", "--verify", &reference])?;
        if branch_head.trim() != request.head_sha {
            return Err(
                "This work item changed since the last scan. Refresh and try again.".to_owned(),
            );
        }
    }

    if let Some(checkout) = checkout {
        let actual_head = command::text(checkout, &["rev-parse", "HEAD"])?;
        if actual_head.trim() != request.head_sha {
            return Err(
                "This work item changed since the last scan. Refresh and try again.".to_owned(),
            );
        }
        if let Some(branch) = request.branch.as_deref() {
            let current = command::text(checkout, &["symbolic-ref", "--quiet", "--short", "HEAD"])?;
            if current.trim() != branch {
                return Err("The selected worktree now contains a different branch.".to_owned());
            }
        }
    }

    Ok(())
}

fn commit_references(
    project_root: &Path,
    request: &WorkItemCommitsRequest,
    source: &str,
) -> Result<CommitReferences, String> {
    let head = if source == "pullRequest" {
        pull_request_head(project_root, request)?
    } else {
        request.head_sha.clone()
    };
    let comparison_branch = request
        .pull_request_base_branch
        .as_deref()
        .or(request.default_branch.as_deref());
    let comparison = comparison_branch.and_then(|branch| base_reference(project_root, branch));
    let (range, comparison_label) = comparison
        .and_then(|(base, label)| {
            command::text(project_root, &["merge-base", &base, &head])
                .ok()
                .map(|merge_base| (format!("{}..{head}", merge_base.trim()), label))
        })
        .map_or((None, None), |(range, label)| (Some(range), Some(label)));

    Ok(CommitReferences {
        head,
        range,
        comparison_label,
        source: source.to_owned(),
    })
}

fn pull_request_head(
    project_root: &Path,
    request: &WorkItemCommitsRequest,
) -> Result<String, String> {
    let number = request
        .pull_request_number
        .ok_or_else(|| "pull request history is missing its number".to_owned())?;
    let expected = request
        .pull_request_head_sha
        .as_deref()
        .unwrap_or(&request.head_sha);
    let reference = remote::pull_request_head_reference(number);
    let cached = remote::cached_commit(project_root, &reference);
    match remote::fetch_pull_request_head(project_root, number, &reference) {
        Ok(fetched) if fetched == expected => Ok(fetched),
        Ok(_) => Err("The pull request changed on GitHub. Refresh the project and try again.".to_owned()),
        Err(_error) if cached.as_deref() == Some(expected) => Ok(expected.to_owned()),
        Err(_error)
            if command::optional_text(
                project_root,
                &["rev-parse", "--verify", &format!("{expected}^{{commit}}")],
            )
            .is_some() => Ok(expected.to_owned()),
        Err(error) => Err(error),
    }
}

fn base_reference(project_root: &Path, branch: &str) -> Option<(String, String)> {
    let remote_reference = remote::base_reference(branch);
    let cached_remote = remote::cached_commit(project_root, &remote_reference);
    if remote::fetch_branch(
        project_root,
        branch,
        &remote_reference,
        &format!("the remote base branch {branch}"),
    )
    .is_ok()
        || cached_remote.is_some()
    {
        return Some((remote_reference, branch.to_owned()));
    }

    let local_reference = format!("refs/heads/{branch}");
    remote::cached_commit(project_root, &local_reference)
        .map(|_| (local_reference, branch.to_owned()))
}

fn commit_count(root: &Path, target: &str) -> Result<usize, String> {
    command::text(root, &["rev-list", "--count", target])
        .map(|value| value.trim().parse().unwrap_or_default())
}

fn commit_log(root: &Path, target: &str) -> Result<String, String> {
    const FORMAT: &str = "%x1e%H%x00%an%x00%ae%x00%at%x00%ct%x00%G?%x00%GS%x00%P%x00%s%x00%b%x00%x1f";
    command::text(
        root,
        &[
            "log",
            &format!("--max-count={MAX_COMMITS}"),
            "--date-order",
            &format!("--format={FORMAT}"),
            "--shortstat",
            target,
            "--",
        ],
    )
}

fn parse_commits(output: &str) -> Vec<WorkItemCommit> {
    output
        .split('\x1e')
        .filter_map(parse_commit)
        .collect()
}

fn parse_commit(record: &str) -> Option<WorkItemCommit> {
    let (metadata, stats) = record.split_once('\x1f')?;
    let fields = metadata.split('\0').collect::<Vec<_>>();
    if fields.len() < 10 || fields[0].trim().is_empty() {
        return None;
    }

    let sha = fields[0].trim().to_owned();
    let stats = parse_shortstat(stats);
    let verification = verification_status(fields[5]);
    Some(WorkItemCommit {
        short_sha: sha.chars().take(7).collect(),
        sha,
        subject: fields[8].trim().to_owned(),
        body: fields[9].trim().to_owned(),
        author_name: fields[1].trim().to_owned(),
        author_email: fields[2].trim().to_owned(),
        authored_at: fields[3].trim().parse().unwrap_or_default(),
        committed_at: fields[4].trim().parse().unwrap_or_default(),
        verification,
        verification_signer: non_empty(fields[6]),
        parent_count: fields[7].split_whitespace().count(),
        additions: stats.additions,
        deletions: stats.deletions,
        changed_files: stats.changed_files,
    })
}

fn non_empty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.trim().to_owned())
}

fn verification_status(value: &str) -> String {
    match value.trim() {
        "G" => "verified",
        "B" | "R" | "E" => "unverified",
        "N" => "unsigned",
        _ => "unknown",
    }
    .to_owned()
}

#[derive(Default)]
struct ShortStat {
    changed_files: usize,
    additions: u64,
    deletions: u64,
}

fn parse_shortstat(value: &str) -> ShortStat {
    let mut stat = ShortStat::default();
    for part in value.split(',') {
        let part = part.trim();
        let count = part
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or_default();
        if part.contains("file changed") || part.contains("files changed") {
            stat.changed_files = count as usize;
        } else if part.contains("insertion") {
            stat.additions = count;
        } else if part.contains("deletion") {
            stat.deletions = count;
        }
    }
    stat
}

#[cfg(test)]
mod tests {
    use super::{parse_shortstat, verification_status};

    #[test]
    fn parses_shortstat_variants() {
        let stat = parse_shortstat(" 2 files changed, 4 insertions(+), 1 deletion(-)");
        assert_eq!(stat.changed_files, 2);
        assert_eq!(stat.additions, 4);
        assert_eq!(stat.deletions, 1);
    }

    #[test]
    fn maps_git_signature_states_to_ui_statuses() {
        assert_eq!(verification_status("G"), "verified");
        assert_eq!(verification_status("N"), "unsigned");
        assert_eq!(verification_status("B"), "unverified");
    }
}
