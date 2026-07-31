use crate::run::{self, RunSettings, ScriptInput};
use std::path::{Path, PathBuf};

const SCOPE: &str = "ship";
const DEFAULT_LABEL: &str = "Merge into default branch";
const DEFAULT_SCRIPT: &str = r#"#!/bin/zsh
set -euo pipefail

[[ "$(git -C "$SHIPYARD_WORKTREE_PATH" rev-parse HEAD)" == "$SHIPYARD_SOURCE_SHA" ]]
[[ -z "$(git -C "$SHIPYARD_WORKTREE_PATH" status --porcelain --untracked-files=normal)" ]]
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
    if !run::load_scoped_settings(base, project_id, SCOPE)?
        .scripts
        .is_empty()
    {
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
