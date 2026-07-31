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
mod work_item;
mod work_status;
mod worktree;
mod worktree_reader;

pub use project::Project;
pub(crate) use repository::belongs_to_project;
pub use scanner::scan_project;

#[cfg(test)]
mod tests;
