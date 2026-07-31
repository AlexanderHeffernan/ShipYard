use super::work_status::WorkStatus;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WorkItem {
    pub(super) id: String,
    pub(super) project_id: String,
    pub(super) branch: Option<String>,
    pub(super) worktree_path: Option<String>,
    pub(super) head_sha: String,
    pub(super) last_commit_subject: String,
    pub(super) status: WorkStatus,
    pub(super) additions: u64,
    pub(super) deletions: u64,
    pub(super) changed_files: usize,
    pub(super) ahead: u32,
    pub(super) behind: u32,
    pub(super) updated_at: u64,
}
