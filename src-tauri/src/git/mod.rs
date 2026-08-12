mod base_branch;
mod branch;
mod branch_reader;
mod checkout;
pub(crate) mod command;
mod deletion;
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

pub use deletion::{delete as delete_work_item, inspect as inspect_work_item_deletion};
pub use deletion::{DeleteWorkItemRequest, DeletionPlan, DeletionResult};
pub use checkout::{pull_request as checkout_pull_request, CheckoutPullRequestRequest, CheckoutPullRequestResult};
pub use project::Project;
pub(crate) use references::ahead_behind;
pub(crate) use repository::validate_worktree;
pub(crate) use repository::{belongs_to_project, resolve};
pub use scanner::scan_project;
pub(crate) use work_item::{PullRequest, WorkItem};
pub(crate) use work_status::WorkStatus;
pub(crate) use worktree_reader::paths as worktree_paths;
pub(crate) use worktree_reader::primary_path as primary_worktree_path;

#[cfg(test)]
mod tests;
