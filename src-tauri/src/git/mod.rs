mod base_branch;
mod branch;
mod branch_reader;
mod command;
mod diff;
mod diff_stats;
mod project;
mod references;
mod repository;
mod scanner;
mod ship_safety;
mod work_item;
mod work_status;
mod worktree;
mod worktree_reader;

pub use project::Project;
pub(crate) use repository::belongs_to_project;
pub(crate) use repository::project_id;
pub use scanner::scan_project_with_conflicts;
pub(crate) use ship_safety::{is_merged, validate_ship};

#[cfg(test)]
mod tests;
