use super::{command, command::CancellationToken, worktree::Worktree};
use std::path::{Path, PathBuf};

pub(super) fn read(root: &Path) -> Result<Vec<Worktree>, String> {
    read_with_cancellation(root, None)
}

pub(super) fn read_with_cancellation(
    root: &Path,
    cancellation: Option<&CancellationToken>,
) -> Result<Vec<Worktree>, String> {
    let output = command::output_with_cancellation(
        root,
        &["worktree", "list", "--porcelain", "-z"],
        cancellation,
    )?;
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
    read_pull_request_metadata(&mut worktrees, cancellation);

    Ok(worktrees)
}

pub(crate) fn paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    Ok(read(root)?
        .into_iter()
        .filter(|worktree| {
            !worktree.bare
                && (!is_shipyard_managed(&worktree.path) || worktree.pull_request_number.is_some())
        })
        .map(|worktree| worktree.path)
        .collect())
}

pub(crate) fn is_shipyard_managed(path: &Path) -> bool {
    let components: Vec<_> = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect();
    components
        .windows(3)
        .any(|window| window[0] == "pull-request-checkouts" && window[2].starts_with("pr-"))
        || components
            .windows(2)
            .any(|window| window[0] == "resolutions" && window[1].starts_with("shipping-"))
}

pub(crate) fn primary_path(root: &Path) -> Result<PathBuf, String> {
    read(root)?
        .into_iter()
        .find(|worktree| !worktree.bare)
        .map(|worktree| worktree.path)
        .ok_or_else(|| "project has no primary checkout".to_owned())
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

fn read_pull_request_metadata(
    worktrees: &mut [Worktree],
    cancellation: Option<&CancellationToken>,
) {
    for worktree in worktrees.iter_mut().filter(|worktree| !worktree.bare) {
        worktree.pull_request_number = command::optional_text_with_cancellation(
            &worktree.path,
            [
                "config",
                "--worktree",
                "--get",
                "shipyard.pull-request-number",
            ]
            .as_slice(),
            cancellation,
        )
        .or_else(|| {
            command::optional_text_with_cancellation(
                &worktree.path,
                ["config", "--get", "shipyard.pull-request-number"].as_slice(),
                cancellation,
            )
        })
        .and_then(|value| value.trim().parse().ok());
    }
}
