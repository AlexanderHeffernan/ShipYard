use super::command::{self, CancellationToken};
use std::path::{Path, PathBuf};

pub(crate) fn resolve(selected_path: &str) -> Result<(PathBuf, PathBuf), String> {
    resolve_with_cancellation(selected_path, None)
}

pub(crate) fn resolve_with_cancellation(
    selected_path: &str,
    cancellation: Option<&CancellationToken>,
) -> Result<(PathBuf, PathBuf), String> {
    let selected_path = Path::new(selected_path);
    let root_text = command::output_with_cancellation(
        selected_path,
        &["rev-parse", "--show-toplevel"],
        cancellation,
    )?;
    let root = canonical_path(Path::new(command::bytes_text(&root_text.stdout).trim()))?;
    let common_dir_output =
        command::output_with_cancellation(&root, &["rev-parse", "--git-common-dir"], cancellation)?;
    let common_dir = PathBuf::from(command::bytes_text(&common_dir_output.stdout).trim());
    let common_dir = if common_dir.is_absolute() {
        common_dir
    } else {
        root.join(common_dir)
    };

    Ok((root, canonical_path(&common_dir)?))
}

pub(super) fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub(crate) fn belongs_to_project(project_id: &str, worktree_path: &str) -> Result<bool, String> {
    let (_, common_dir) = resolve(worktree_path)?;
    Ok(path_string(&common_dir) == project_id)
}

pub(crate) fn validate_worktree(project_id: &str, path: &str) -> Result<PathBuf, String> {
    validate_worktree_with_cancellation(project_id, path, None)
}

pub(crate) fn validate_worktree_with_cancellation(
    project_id: &str,
    path: &str,
    cancellation: Option<&CancellationToken>,
) -> Result<PathBuf, String> {
    let requested = canonical_path(Path::new(path))?;
    let (root, common_dir) = resolve_with_cancellation(path, cancellation)
        .map_err(|error| format!("selected checkout is not an available Git worktree: {error}"))?;
    if root != requested || path_string(&common_dir) != project_id {
        return Err("selected path is not a checkout for this project".to_owned());
    }
    Ok(root)
}

fn canonical_path(path: &Path) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|error| format!("could not resolve {}: {error}", path.display()))
}
