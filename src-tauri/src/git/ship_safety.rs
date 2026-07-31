use super::{command, repository, worktree_reader};
use std::path::{Path, PathBuf};

pub(crate) fn validate_ship(
    project_id: &str,
    source_path: &str,
    source_branch: Option<&str>,
    source_sha: &str,
    default_branch: &str,
) -> Result<PathBuf, String> {
    if !repository::belongs_to_project(project_id, source_path)? {
        return Err("Ship source no longer belongs to this project".to_owned());
    }
    let source = Path::new(source_path)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    validate_source(&source, source_branch, source_sha)?;
    let target = default_worktree(&source, default_branch)?;
    if target == source {
        return Err("the default branch cannot be shipped into itself".to_owned());
    }
    validate_target(&target, default_branch)?;
    Ok(target)
}

pub(crate) fn is_merged(
    target_path: &str,
    source_sha: &str,
    default_branch: &str,
) -> Result<bool, String> {
    let target = Path::new(target_path);
    let reference = format!("refs/heads/{default_branch}");
    let output = command::output_allow_failure(
        target,
        &["merge-base", "--is-ancestor", source_sha, &reference],
    );
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(command::error(&output)),
    }
}

fn validate_source(source: &Path, branch: Option<&str>, sha: &str) -> Result<(), String> {
    ensure_clean(source, "source")?;
    let head = command::text(source, &["rev-parse", "HEAD"])?;
    if head.trim() != sha {
        return Err("Ship source changed; rescan before retrying".to_owned());
    }
    let actual = command::optional_text(source, &["symbolic-ref", "--quiet", "--short", "HEAD"]);
    if actual.as_deref().map(str::trim) != branch {
        return Err("Ship source branch or detached state changed; rescan first".to_owned());
    }
    Ok(())
}

fn default_worktree(source: &Path, default_branch: &str) -> Result<PathBuf, String> {
    let reference = format!("refs/heads/{default_branch}");
    worktree_reader::read(source)?
        .into_iter()
        .find(|worktree| worktree.branch.as_deref() == Some(reference.as_str()))
        .map(|worktree| worktree.path)
        .ok_or_else(|| {
            format!("default branch “{default_branch}” must be checked out in a worktree")
        })
}

fn validate_target(target: &Path, default_branch: &str) -> Result<(), String> {
    ensure_clean(target, "default branch")?;
    let actual = command::text(target, &["symbolic-ref", "--quiet", "--short", "HEAD"])?;
    if actual.trim() != default_branch {
        return Err("default-branch worktree changed; rescan first".to_owned());
    }
    Ok(())
}

fn ensure_clean(path: &Path, label: &str) -> Result<(), String> {
    if !command::text(path, &["status", "--porcelain", "--untracked-files=normal"])?.is_empty() {
        return Err(format!(
            "Ship blocked: {label} worktree has uncommitted changes"
        ));
    }
    Ok(())
}
