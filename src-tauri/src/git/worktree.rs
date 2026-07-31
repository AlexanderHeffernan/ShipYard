use std::path::PathBuf;

#[derive(Default)]
pub(super) struct Worktree {
    pub(super) path: PathBuf,
    pub(super) sha: String,
    pub(super) branch: Option<String>,
    pub(super) detached: bool,
    pub(super) bare: bool,
}
