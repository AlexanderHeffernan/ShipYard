use super::work_status::WorkStatus;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PullRequest {
    pub(crate) number: u64,
    pub(crate) title: String,
    pub(crate) url: String,
    pub(crate) draft: bool,
    pub(crate) mergeable: Option<bool>,
    pub(crate) merge_state: String,
    pub(crate) head_branch: String,
    pub(crate) base_branch: String,
    pub(crate) head_sha: String,
    pub(crate) local_commits: u32,
    pub(crate) remote_commits: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkItem {
    pub(crate) id: String,
    pub(crate) project_id: String,
    pub(crate) branch: Option<String>,
    pub(crate) worktree_path: Option<String>,
    pub(crate) head_sha: String,
    pub(crate) last_commit_subject: String,
    pub(crate) status: WorkStatus,
    pub(crate) pull_request: Option<PullRequest>,
    pub(crate) completed: bool,
    pub(crate) additions: u64,
    pub(crate) deletions: u64,
    pub(crate) changed_files: usize,
    pub(crate) ahead: u32,
    pub(crate) behind: u32,
    pub(crate) updated_at: u64,
}
