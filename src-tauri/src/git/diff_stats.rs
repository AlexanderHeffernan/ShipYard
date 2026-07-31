#[derive(Default)]
pub(super) struct DiffStats {
    pub(super) dirty: bool,
    pub(super) additions: u64,
    pub(super) deletions: u64,
    pub(super) changed_files: usize,
}
