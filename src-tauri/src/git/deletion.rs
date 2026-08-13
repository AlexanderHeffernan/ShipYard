use super::{branch_reader, command, references, repository, worktree::Worktree, worktree_reader};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteWorkItemRequest {
    pub(super) project_path: String,
    pub(super) project_id: String,
    pub(super) work_item_id: String,
    pub(super) branch: Option<String>,
    pub(super) worktree_path: Option<String>,
    pub(super) head_sha: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionPlan {
    pub(super) project_id: String,
    pub(super) work_item_id: String,
    pub(super) branch: Option<String>,
    pub(super) worktree_path: Option<String>,
    pub(super) default_branch: Option<String>,
    pub(super) removes_worktree: bool,
    pub(super) deletes_branch: bool,
    pub(super) switches_primary_checkout: bool,
    pub(super) has_uncommitted_changes: bool,
    pub(super) unpushed_commits: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionResult {
    pub(super) project_id: String,
    pub(super) work_item_id: String,
    pub(super) worktree_removed: bool,
    pub(super) branch_deleted: bool,
    pub(super) switched_primary_to_default: bool,
}

struct ValidatedDeletion {
    root: PathBuf,
    primary_path: PathBuf,
    branch_ref: Option<String>,
    worktree: Option<Worktree>,
    plan: DeletionPlan,
}

pub fn inspect(request: DeleteWorkItemRequest) -> Result<DeletionPlan, String> {
    Ok(validate(&request)?.plan)
}

pub fn delete(
    request: DeleteWorkItemRequest,
    confirmed_plan: DeletionPlan,
) -> Result<DeletionResult, String> {
    let validated = validate(&request)?;
    if validated.plan != confirmed_plan {
        return Err(
            "This work item changed after the confirmation opened. Review its current state and try again."
                .to_owned(),
        );
    }

    let mut result = DeletionResult {
        project_id: validated.plan.project_id.clone(),
        work_item_id: validated.plan.work_item_id.clone(),
        worktree_removed: false,
        branch_deleted: false,
        switched_primary_to_default: false,
    };

    if let Some(worktree) = validated.worktree.as_ref() {
        if worktree.path == validated.primary_path {
            let default_branch = validated.plan.default_branch.as_deref().ok_or_else(|| {
                "ShipYard could not determine the default branch, so it did not change the primary checkout."
                    .to_owned()
            })?;
            command::output(&validated.root, &["switch", "--no-guess", default_branch]).map_err(
                |error| {
                    format!("Could not switch the primary checkout to {default_branch}: {error}")
                },
            )?;
            result.switched_primary_to_default = true;
        } else {
            let path = repository::path_string(&worktree.path);
            command::output(&validated.root, &["worktree", "remove", "--force", &path])
                .map_err(|error| format!("Could not remove worktree at {path}: {error}"))?;
            result.worktree_removed = true;
        }
    }

    if let (Some(branch), Some(branch_ref)) = (
        validated.plan.branch.as_deref(),
        validated.branch_ref.as_deref(),
    ) {
        let current_sha = command::optional_text(&validated.root, &["rev-parse", branch_ref])
            .ok_or_else(|| {
                format!(
                    "The worktree was removed, but branch {branch} no longer exists. Rescan the project."
                )
            })?;
        if current_sha.trim() != request.head_sha {
            return Err(format!(
                "The worktree was removed, but branch {branch} changed before it could be deleted. The branch was left intact."
            ));
        }
        command::output(&validated.root, &["branch", "-D", "--", branch])
            .map_err(|error| format!("Could not delete local branch {branch}: {error}"))?;
        result.branch_deleted = true;
    }

    Ok(result)
}

fn validate(request: &DeleteWorkItemRequest) -> Result<ValidatedDeletion, String> {
    let root = repository::validate_worktree(&request.project_id, &request.project_path)
        .map_err(|error| format!("Project identity no longer matches: {error}"))?;
    let (_, common_dir) = repository::resolve(&request.project_path)?;
    if repository::path_string(&common_dir) != request.project_id {
        return Err(
            "Project identity no longer matches. Rescan the project and try again.".to_owned(),
        );
    }

    let worktrees = worktree_reader::read(&root)?;
    let primary_path = worktrees
        .iter()
        .find(|worktree| !worktree.bare)
        .map(|worktree| worktree.path.clone())
        .ok_or_else(|| "This repository has no primary checkout to protect.".to_owned())?;
    if root != primary_path {
        return Err("The saved project path is no longer the primary checkout. Rescan the project and try again."
            .to_owned());
    }

    let branches = branch_reader::read(&root)?;
    let base = references::find_base(&root, &branches);
    let default_branch = base.as_ref().map(|branch| branch.name.clone());
    let (branch_ref, branch_sha) = validate_branch(request, &root, &branches)?;
    let worktree = validate_worktree_identity(request, &worktrees, branch_ref.as_deref())?;
    let actual_sha = branch_sha
        .as_deref()
        .or_else(|| worktree.as_ref().map(|item| item.sha.as_str()))
        .ok_or_else(|| "This work item no longer exists. Rescan the project.".to_owned())?;
    if actual_sha != request.head_sha {
        return Err(
            "This work item changed since the last scan. Rescan the project and try again."
                .to_owned(),
        );
    }

    let expected_id = if let Some(reference) = branch_ref.as_deref() {
        format!("{}::branch::{reference}", request.project_id)
    } else if let Some(worktree) = worktree.as_ref() {
        format!(
            "{}::worktree::{}",
            request.project_id,
            repository::path_string(&worktree.path)
        )
    } else {
        return Err("This work item no longer exists. Rescan the project.".to_owned());
    };
    if request.work_item_id != expected_id
        && !is_pull_request_id(&request.work_item_id, &request.project_id)
    {
        return Err(
            "Work item identity does not match the current branch or worktree. Rescan the project."
                .to_owned(),
        );
    }

    if request.branch.as_deref() == default_branch.as_deref() {
        return Err("The repository's default branch cannot be deleted.".to_owned());
    }

    let is_primary = worktree
        .as_ref()
        .is_some_and(|worktree| worktree.path == primary_path);
    if is_primary && request.branch.is_none() {
        return Err("The primary repository checkout cannot be removed.".to_owned());
    }
    let has_uncommitted_changes = worktree
        .as_ref()
        .filter(|worktree| worktree.path.exists())
        .map(|worktree| is_dirty(&worktree.path))
        .transpose()?
        .unwrap_or(false);
    if is_primary && has_uncommitted_changes {
        return Err(
            "The primary checkout has uncommitted changes. Commit, stash, or discard them before deleting this branch."
                .to_owned(),
        );
    }
    if is_primary && default_branch.is_none() {
        return Err("ShipYard could not determine the default branch, so it will not change the primary checkout."
            .to_owned());
    }

    let target = branch_ref.as_deref().unwrap_or(actual_sha);
    let unpushed_commits = unreachable_commit_count(&root, target, branch_ref.as_deref())?;
    let plan = DeletionPlan {
        project_id: request.project_id.clone(),
        work_item_id: request.work_item_id.clone(),
        branch: request.branch.clone(),
        worktree_path: worktree
            .as_ref()
            .map(|worktree| repository::path_string(&worktree.path)),
        default_branch,
        removes_worktree: worktree.is_some() && !is_primary,
        deletes_branch: request.branch.is_some(),
        switches_primary_checkout: is_primary,
        has_uncommitted_changes,
        unpushed_commits,
    };

    Ok(ValidatedDeletion {
        root,
        primary_path,
        branch_ref,
        worktree,
        plan,
    })
}

fn is_pull_request_id(id: &str, project_id: &str) -> bool {
    let Some(number) = id.strip_prefix(&format!("{project_id}::pull-request::")) else {
        return false;
    };
    !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
}

fn validate_branch(
    request: &DeleteWorkItemRequest,
    root: &Path,
    branches: &[super::branch::Branch],
) -> Result<(Option<String>, Option<String>), String> {
    let Some(branch) = request.branch.as_deref() else {
        return Ok((None, None));
    };
    command::output(root, &["check-ref-format", "--branch", branch])
        .map_err(|_| "The branch name is not valid and will not be deleted.".to_owned())?;
    let reference = format!("refs/heads/{branch}");
    let branch = branches
        .iter()
        .find(|branch| branch.reference == reference)
        .ok_or_else(|| "This local branch no longer exists. Rescan the project.".to_owned())?;
    Ok((Some(reference), Some(branch.sha.clone())))
}

fn validate_worktree_identity(
    request: &DeleteWorkItemRequest,
    worktrees: &[Worktree],
    branch_ref: Option<&str>,
) -> Result<Option<Worktree>, String> {
    let matching_branch = branch_ref.and_then(|reference| {
        worktrees
            .iter()
            .find(|worktree| worktree.branch.as_deref() == Some(reference))
    });
    let worktree = match request.worktree_path.as_deref() {
        Some(requested_path) => {
            let found = worktrees
                .iter()
                .find(|worktree| repository::path_string(&worktree.path) == requested_path)
                .ok_or_else(|| {
                    "The expected worktree is no longer registered with this project. Rescan before deleting."
                        .to_owned()
                })?;
            if found.bare
                || branch_ref.is_some_and(|reference| found.branch.as_deref() != Some(reference))
            {
                return Err(
                    "The path now belongs to a different worktree. Nothing was deleted.".to_owned(),
                );
            }
            if branch_ref.is_none() && !found.detached {
                return Err("The worktree is no longer detached. Nothing was deleted.".to_owned());
            }
            Some(found)
        }
        None => {
            if matching_branch.is_some() {
                return Err(
                    "This branch is now checked out in a worktree. Rescan before deleting it."
                        .to_owned(),
                );
            }
            None
        }
    };

    Ok(worktree.cloned())
}

fn is_dirty(path: &Path) -> Result<bool, String> {
    Ok(!command::output(
        path,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )?
    .stdout
    .is_empty())
}

fn unreachable_commit_count(
    root: &Path,
    target: &str,
    excluded_ref: Option<&str>,
) -> Result<u32, String> {
    let refs = command::text(root, &["for-each-ref", "--format=%(refname)"])?;
    let mut args = vec!["rev-list", "--count", target, "--not"];
    args.extend(
        refs.lines()
            .filter(|reference| Some(*reference) != excluded_ref),
    );
    command::text(root, &args)?
        .trim()
        .parse()
        .map_err(|error| format!("Could not count commits unique to this work item: {error}"))
}
