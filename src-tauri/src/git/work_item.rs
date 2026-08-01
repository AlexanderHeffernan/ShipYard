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
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkItem {
    pub(super) id: String,
    pub(super) project_id: String,
    pub(crate) branch: Option<String>,
    pub(super) worktree_path: Option<String>,
    pub(crate) head_sha: String,
    pub(super) last_commit_subject: String,
    pub(super) status: WorkStatus,
    pub(crate) pull_request: Option<PullRequest>,
    pub(crate) completed: bool,
    pub(super) additions: u64,
    pub(super) deletions: u64,
    pub(super) changed_files: usize,
    pub(super) ahead: u32,
    pub(super) behind: u32,
    pub(super) updated_at: u64,
}
