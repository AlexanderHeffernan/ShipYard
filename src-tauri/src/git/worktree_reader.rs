use super::{command, worktree::Worktree};
use std::path::{Path, PathBuf};

pub(super) fn read(root: &Path) -> Result<Vec<Worktree>, String> {
    let output = command::output(root, &["worktree", "list", "--porcelain", "-z"])?;
    let mut worktrees = Vec::new();
    let mut current = None;

    for field in output.stdout.split(|byte| *byte == 0) {
        if field.is_empty() {
            push_current(&mut worktrees, &mut current);
            continue;
        }
        parse_field(command::bytes_text(field), &mut worktrees, &mut current);
    }
    push_current(&mut worktrees, &mut current);

    Ok(worktrees)
}

fn parse_field(field: &str, worktrees: &mut Vec<Worktree>, current: &mut Option<Worktree>) {
    if let Some(path) = field.strip_prefix("worktree ") {
        push_current(worktrees, current);
        *current = Some(Worktree {
            path: PathBuf::from(path),
            ..Worktree::default()
        });
        return;
    }

    let Some(worktree) = current.as_mut() else {
        return;
    };
    if let Some(sha) = field.strip_prefix("HEAD ") {
        worktree.sha = sha.to_owned();
    } else if let Some(branch) = field.strip_prefix("branch ") {
        worktree.branch = Some(branch.to_owned());
    } else if field == "detached" {
        worktree.detached = true;
    } else if field == "bare" {
        worktree.bare = true;
    }
}

fn push_current(worktrees: &mut Vec<Worktree>, current: &mut Option<Worktree>) {
    if let Some(worktree) = current.take() {
        worktrees.push(worktree);
    }
}
