use super::{
    base_branch::BaseBranch, branch::Branch, branch_reader, diff, diff_stats::DiffStats,
    project::Project, references, repository, work_item::WorkItem, work_status::WorkStatus,
    worktree::Worktree, worktree_reader,
};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

pub fn scan_project(selected_path: &str) -> Result<Project, String> {
    let (root, common_dir) = repository::resolve(selected_path)?;
    let project_id = repository::path_string(&common_dir);
    let worktrees = worktree_reader::read(&root)?;
    let branches = branch_reader::read(&root)?;
    let base = references::find_base(&root, &branches);
    let worktree_by_branch = index_worktrees(&worktrees);
    let processed = branches
        .iter()
        .map(|branch| branch.reference.clone())
        .collect();
    let mut items = branch_items(
        &root,
        &project_id,
        branches,
        &worktree_by_branch,
        base.as_ref(),
    )?;
    items.extend(unborn_items(&project_id, &worktrees, &processed)?);
    items.extend(detached_items(
        &root,
        &project_id,
        &worktrees,
        base.as_ref(),
    )?);
    items.sort_by_key(|item| std::cmp::Reverse(item.updated_at));

    let project_root = project_root(&root, &worktrees);
    Ok(Project {
        id: project_id,
        name: project_name(project_root),
        path: repository::path_string(project_root),
        default_branch: base.map(|base| base.name),
        work_items: items,
        github_repository: None,
        github_error: None,
    })
}

fn index_worktrees(worktrees: &[Worktree]) -> HashMap<String, &Worktree> {
    worktrees
        .iter()
        .filter_map(|worktree| {
            worktree
                .branch
                .as_ref()
                .map(|branch| (branch.clone(), worktree))
        })
        .collect()
}

fn branch_items(
    root: &Path,
    project_id: &str,
    branches: Vec<Branch>,
    worktrees: &HashMap<String, &Worktree>,
    base: Option<&BaseBranch>,
) -> Result<Vec<WorkItem>, String> {
    branches
        .into_iter()
        .filter_map(|branch| {
            let item = branch_item(root, project_id, branch, worktrees, base);
            match item {
                Ok(None) => None,
                Ok(Some(item)) => Some(Ok(item)),
                Err(error) => Some(Err(error)),
            }
        })
        .collect()
}

fn branch_item(
    root: &Path,
    project_id: &str,
    branch: Branch,
    worktrees: &HashMap<String, &Worktree>,
    base: Option<&BaseBranch>,
) -> Result<Option<WorkItem>, String> {
    let worktree = worktrees.get(&branch.reference).copied();
    let stats = worktree
        .map(|worktree| diff::read_stats(&worktree.path))
        .transpose()?
        .unwrap_or_default();
    let comparison = references::comparison(root, &branch, base);
    let is_base = base.is_some_and(|base| base.reference == branch.reference);
    let status = branch_status(root, &branch, &stats, comparison.as_deref(), is_base)?;
    if is_base && !stats.dirty && status == WorkStatus::Shipped {
        return Ok(None);
    }
    let (ahead, behind) = comparison
        .as_deref()
        .map(|reference| references::ahead_behind(root, reference, &branch.reference))
        .transpose()?
        .unwrap_or_default();
    Ok(Some(branch_work_item(
        project_id, branch, worktree, stats, status, ahead, behind,
    )))
}

fn branch_status(
    root: &Path,
    branch: &Branch,
    stats: &DiffStats,
    comparison: Option<&str>,
    is_base: bool,
) -> Result<WorkStatus, String> {
    if is_base && comparison.is_none() && !stats.dirty {
        Ok(WorkStatus::Shipped)
    } else if !is_base
        && !stats.dirty
        && comparison
            .map(|base| references::same_commit(root, &branch.reference, base))
            .transpose()?
            .unwrap_or(false)
    {
        Ok(WorkStatus::Working)
    } else {
        references::classify(root, stats.dirty, &branch.reference, comparison)
    }
}

fn branch_work_item(
    project_id: &str,
    branch: Branch,
    worktree: Option<&Worktree>,
    stats: DiffStats,
    status: WorkStatus,
    ahead: u32,
    behind: u32,
) -> WorkItem {
    WorkItem {
        id: format!("{project_id}::branch::{}", branch.reference),
        project_id: project_id.to_owned(),
        branch: Some(branch.name),
        worktree_path: worktree.map(|item| repository::path_string(&item.path)),
        head_sha: branch.sha,
        last_commit_subject: branch.subject,
        status,
        pull_request: None,
        completed: false,
        additions: stats.additions,
        deletions: stats.deletions,
        changed_files: stats.changed_files,
        ahead,
        behind,
        updated_at: branch.updated_at,
    }
}

fn unborn_items(
    project_id: &str,
    worktrees: &[Worktree],
    processed: &HashSet<String>,
) -> Result<Vec<WorkItem>, String> {
    let mut items = Vec::new();
    for worktree in worktrees
        .iter()
        .filter(|worktree| is_unborn(worktree, processed))
    {
        if let Some(item) = unborn_item(project_id, worktree)? {
            items.push(item);
        }
    }
    Ok(items)
}

fn is_unborn(worktree: &Worktree, processed: &HashSet<String>) -> bool {
    !worktree.bare
        && !worktree.detached
        && worktree
            .branch
            .as_ref()
            .is_some_and(|branch| !processed.contains(branch))
}

fn unborn_item(project_id: &str, worktree: &Worktree) -> Result<Option<WorkItem>, String> {
    let stats = diff::read_stats(&worktree.path)?;
    if !stats.dirty {
        return Ok(None);
    }
    let branch_ref = worktree.branch.as_deref().unwrap_or_default();
    Ok(Some(WorkItem {
        id: format!("{project_id}::branch::{branch_ref}"),
        project_id: project_id.to_owned(),
        branch: Some(short_branch_name(branch_ref)),
        worktree_path: Some(repository::path_string(&worktree.path)),
        head_sha: worktree.sha.clone(),
        last_commit_subject: String::new(),
        status: WorkStatus::Working,
        pull_request: None,
        completed: false,
        additions: stats.additions,
        deletions: stats.deletions,
        changed_files: stats.changed_files,
        ahead: 0,
        behind: 0,
        updated_at: 0,
    }))
}

fn short_branch_name(reference: &str) -> String {
    reference
        .strip_prefix("refs/heads/")
        .unwrap_or(reference)
        .to_owned()
}

fn detached_items(
    root: &Path,
    project_id: &str,
    worktrees: &[Worktree],
    base: Option<&BaseBranch>,
) -> Result<Vec<WorkItem>, String> {
    worktrees
        .iter()
        .filter(|worktree| worktree.detached && !worktree.bare)
        .map(|worktree| detached_item(root, project_id, worktree, base))
        .collect()
}

fn detached_item(
    root: &Path,
    project_id: &str,
    worktree: &Worktree,
    base: Option<&BaseBranch>,
) -> Result<WorkItem, String> {
    let stats = diff::read_stats(&worktree.path)?;
    let comparison = base.map(|base| base.reference.as_str());
    let status = references::classify(root, stats.dirty, &worktree.sha, comparison)?;
    let (ahead, behind) = comparison
        .map(|reference| references::ahead_behind(root, reference, &worktree.sha))
        .transpose()?
        .unwrap_or_default();
    let (subject, updated_at) = references::commit_details(&worktree.path)?;
    Ok(WorkItem {
        id: format!(
            "{project_id}::worktree::{}",
            repository::path_string(&worktree.path)
        ),
        project_id: project_id.to_owned(),
        branch: None,
        worktree_path: Some(repository::path_string(&worktree.path)),
        head_sha: worktree.sha.clone(),
        last_commit_subject: subject,
        status,
        pull_request: None,
        completed: false,
        additions: stats.additions,
        deletions: stats.deletions,
        changed_files: stats.changed_files,
        ahead,
        behind,
        updated_at,
    })
}

fn project_root<'a>(root: &'a Path, worktrees: &'a [Worktree]) -> &'a Path {
    worktrees
        .iter()
        .find(|worktree| !worktree.bare)
        .map(|worktree| worktree.path.as_path())
        .unwrap_or(root)
}

fn project_name(root: &Path) -> String {
    root.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Repository")
        .to_owned()
}
