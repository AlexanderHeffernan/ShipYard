use crate::run::{self, RunSettings, ScriptInput};
use std::path::{Path, PathBuf};

const SCOPE: &str = "ship";
const DEFAULT_LABEL: &str = "Merge into default branch";
pub(super) const LEGACY_DEFAULT_SCRIPT: &str = r#"#!/bin/zsh
set -euo pipefail

[[ "$(git -C "$SHIPYARD_WORKTREE_PATH" rev-parse HEAD)" == "$SHIPYARD_SOURCE_SHA" ]]
[[ -z "$(git -C "$SHIPYARD_WORKTREE_PATH" status --porcelain --untracked-files=normal)" ]]
[[ "$(git -C "$SHIPYARD_TARGET_WORKTREE_PATH" branch --show-current)" == "$SHIPYARD_DEFAULT_BRANCH" ]]
[[ -z "$(git -C "$SHIPYARD_TARGET_WORKTREE_PATH" status --porcelain --untracked-files=normal)" ]]
git -C "$SHIPYARD_TARGET_WORKTREE_PATH" merge --no-edit "$SHIPYARD_SOURCE_SHA"
"#;
const DEFAULT_SCRIPT: &str = r#"#!/bin/zsh
set -euo pipefail

[[ "$(git -C "$SHIPYARD_WORKTREE_PATH" rev-parse HEAD)" == "$SHIPYARD_SOURCE_SHA" ]]
[[ -z "$(git -C "$SHIPYARD_WORKTREE_PATH" status --porcelain --untracked-files=normal)" ]]
[[ "$(git -C "$SHIPYARD_TARGET_WORKTREE_PATH" branch --show-current)" == "$SHIPYARD_DEFAULT_BRANCH" ]]
[[ -z "$(git -C "$SHIPYARD_TARGET_WORKTREE_PATH" status --porcelain --untracked-files=normal)" ]]

target_sha="$(git -C "$SHIPYARD_TARGET_WORKTREE_PATH" rev-parse HEAD)"
if ! git -C "$SHIPYARD_TARGET_WORKTREE_PATH" merge-tree --write-tree "$target_sha" "$SHIPYARD_SOURCE_SHA" >/dev/null; then
  echo "Shipyard stopped before merging because the commits do not merge cleanly."
  exit 1
fi

[[ "$(git -C "$SHIPYARD_WORKTREE_PATH" rev-parse HEAD)" == "$SHIPYARD_SOURCE_SHA" ]]
[[ -z "$(git -C "$SHIPYARD_WORKTREE_PATH" status --porcelain --untracked-files=normal)" ]]
[[ "$(git -C "$SHIPYARD_TARGET_WORKTREE_PATH" rev-parse HEAD)" == "$target_sha" ]]
[[ "$(git -C "$SHIPYARD_TARGET_WORKTREE_PATH" branch --show-current)" == "$SHIPYARD_DEFAULT_BRANCH" ]]
[[ -z "$(git -C "$SHIPYARD_TARGET_WORKTREE_PATH" status --porcelain --untracked-files=normal)" ]]
git -C "$SHIPYARD_TARGET_WORKTREE_PATH" merge --no-edit "$SHIPYARD_SOURCE_SHA"
"#;

pub fn load_settings(base: &Path, project_id: &str) -> Result<RunSettings, String> {
    ensure_default(base, project_id)?;
    run::load_scoped_settings(base, project_id, SCOPE)
}

pub fn save_script(
    base: &Path,
    project_id: &str,
    input: ScriptInput,
) -> Result<RunSettings, String> {
    run::save_scoped_script(base, project_id, SCOPE, input)
}

pub fn delete_script(
    base: &Path,
    project_id: &str,
    script_id: &str,
) -> Result<RunSettings, String> {
    run::delete_scoped_script(base, project_id, SCOPE, script_id)
}

pub fn script_path(base: &Path, project_id: &str, script_id: &str) -> Result<PathBuf, String> {
    ensure_default(base, project_id)?;
    run::scoped_script_path(base, project_id, SCOPE, script_id)
}

fn ensure_default(base: &Path, project_id: &str) -> Result<(), String> {
    let settings = run::load_scoped_settings(base, project_id, SCOPE)?;
    if let Some(script) = settings
        .scripts
        .iter()
        .find(|script| script.content == LEGACY_DEFAULT_SCRIPT)
    {
        run::save_scoped_script(
            base,
            project_id,
            SCOPE,
            ScriptInput {
                id: Some(script.id.clone()),
                label: script.label.clone(),
                content: DEFAULT_SCRIPT.to_owned(),
                make_default: settings.default_script_id.as_deref() == Some(&script.id),
            },
        )?;
        return Ok(());
    }
    if !settings.scripts.is_empty() {
        return Ok(());
    }
    run::save_scoped_script(
        base,
        project_id,
        SCOPE,
        ScriptInput {
            id: None,
            label: DEFAULT_LABEL.to_owned(),
            content: DEFAULT_SCRIPT.to_owned(),
            make_default: true,
        },
    )?;
    Ok(())
}
