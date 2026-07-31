use super::command;
use std::path::{Path, PathBuf};

pub(super) fn resolve(selected_path: &str) -> Result<(PathBuf, PathBuf), String> {
    let selected_path = Path::new(selected_path);
    let root = canonical_path(Path::new(
        command::text(selected_path, &["rev-parse", "--show-toplevel"])?.trim(),
    ))?;
    let common_dir =
        PathBuf::from(command::text(&root, &["rev-parse", "--git-common-dir"])?.trim());
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

fn canonical_path(path: &Path) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|error| format!("could not resolve {}: {error}", path.display()))
}
