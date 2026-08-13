use std::path::PathBuf;

#[derive(Clone, Default)]
pub(super) struct Worktree {
    pub(super) path: PathBuf,
    pub(super) sha: String,
    pub(super) branch: Option<String>,
    pub(super) pull_request_number: Option<u64>,
    pub(super) detached: bool,
    pub(super) bare: bool,
}
