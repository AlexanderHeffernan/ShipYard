use super::{base_branch::BaseBranch, branch::Branch, command, work_status::WorkStatus};
use std::path::Path;

pub(super) fn find_base(root: &Path, branches: &[Branch]) -> Option<BaseBranch> {
    if let Some(base) = remote_base(root, branches) {
        return Some(base);
    }
    for name in ["main", "master", "trunk", "develop"] {
        if has_branch(branches, name) {
            return Some(BaseBranch {
                reference: format!("refs/heads/{name}"),
                name: name.to_owned(),
            });
        }
    }
    current_base(root, branches)
}

fn remote_base(root: &Path, branches: &[Branch]) -> Option<BaseBranch> {
    let remote_head = command::optional_text(
        root,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    )?;
    let name = remote_head.trim().strip_prefix("origin/")?;
    has_branch(branches, name).then(|| BaseBranch {
        reference: format!("refs/heads/{name}"),
        name: name.to_owned(),
    })
}

fn current_base(root: &Path, branches: &[Branch]) -> Option<BaseBranch> {
    let current = command::optional_text(root, &["symbolic-ref", "--quiet", "--short", "HEAD"])?;
    let name = current.trim().to_owned();
    has_branch(branches, &name).then(|| BaseBranch {
        reference: format!("refs/heads/{name}"),
        name,
    })
}

fn has_branch(branches: &[Branch], name: &str) -> bool {
    branches.iter().any(|branch| branch.name == name)
}

pub(super) fn comparison(
    root: &Path,
    branch: &Branch,
    base: Option<&BaseBranch>,
) -> Option<String> {
    if base.is_some_and(|base| base.reference == branch.reference) {
        let upstream = command::optional_text(
            root,
            &["for-each-ref", "--format=%(upstream)", &branch.reference],
        )?;
        let upstream = upstream.trim();
        (!upstream.is_empty()).then(|| upstream.to_owned())
    } else {
        base.map(|base| base.reference.clone())
    }
}

pub(super) fn classify(
    root: &Path,
    dirty: bool,
    item_ref: &str,
    comparison_ref: Option<&str>,
) -> Result<WorkStatus, String> {
    if dirty {
        return Ok(WorkStatus::Working);
    }
    let Some(comparison_ref) = comparison_ref else {
        return Ok(WorkStatus::Ready);
    };
    if is_ancestor(root, item_ref, comparison_ref)? {
        Ok(WorkStatus::Shipped)
    } else {
        Ok(WorkStatus::Ready)
    }
}

pub(super) fn ahead_behind(root: &Path, base: &str, item: &str) -> Result<(u32, u32), String> {
    let range = format!("{base}...{item}");
    let counts = command::text(root, &["rev-list", "--left-right", "--count", &range])?;
    let mut counts = counts.split_whitespace();
    let behind = parse_count(counts.next());
    let ahead = parse_count(counts.next());
    Ok((ahead, behind))
}

fn parse_count(value: Option<&str>) -> u32 {
    value.and_then(|value| value.parse().ok()).unwrap_or(0)
}

fn is_ancestor(root: &Path, ancestor: &str, descendant: &str) -> Result<bool, String> {
    let output =
        command::output_allow_failure(root, &["merge-base", "--is-ancestor", ancestor, descendant]);
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(command::error(&output)),
    }
}

pub(super) fn same_commit(root: &Path, left: &str, right: &str) -> Result<bool, String> {
    let left = command::text(root, &["rev-parse", left])?;
    let right = command::text(root, &["rev-parse", right])?;
    Ok(left.trim() == right.trim())
}

pub(super) fn commit_details(worktree: &Path) -> Result<(String, u64), String> {
    let details = command::text(worktree, &["show", "-s", "--format=%s%x00%ct", "HEAD"])?;
    let mut fields = details.split('\0');
    let subject = fields.next().unwrap_or_default().to_owned();
    let timestamp = fields
        .next()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or_default();
    Ok((subject, timestamp))
}
