use super::{command, repository};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemDiffRequest {
    pub(super) project_path: String,
    pub(super) project_id: String,
    pub(super) branch: Option<String>,
    pub(super) worktree_path: Option<String>,
    pub(super) head_sha: String,
    pub(super) default_branch: Option<String>,
    #[serde(default)]
    pub(super) pull_request_number: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemDiff {
    pub(super) patch: String,
    pub(super) comparison_label: String,
}

pub fn read(request: WorkItemDiffRequest) -> Result<WorkItemDiff, String> {
    let (project_root, common_dir) = repository::resolve(&request.project_path)?;
    if repository::path_string(&common_dir) != request.project_id {
        return Err("work item does not belong to the selected project".to_owned());
    }

    let checkout = request
        .worktree_path
        .as_deref()
        .map(|path| repository::validate_worktree(&request.project_id, path))
        .transpose()?;
    ensure_head_available(&project_root, checkout.as_deref(), &request)?;
    validate_target(&project_root, checkout.as_deref(), &request)?;

    let (base, comparison_label) = comparison_base(&project_root, &request)?;
    let mut args = vec![
        "diff",
        "--no-ext-diff",
        "--find-renames",
        "--find-copies",
        "--binary",
        "--full-index",
        "--unified=5",
        &base,
    ];
    if checkout.is_none() {
        args.push(&request.head_sha);
    }
    args.push("--");

    let diff_root = checkout.as_deref().unwrap_or(&project_root);
    let mut patch = command::text(diff_root, &args)?;
    if checkout.is_some() {
        append_untracked_files(diff_root, &mut patch)?;
    }

    Ok(WorkItemDiff {
        patch,
        comparison_label,
    })
}

fn ensure_head_available(
    project_root: &Path,
    checkout: Option<&Path>,
    request: &WorkItemDiffRequest,
) -> Result<(), String> {
    if checkout.is_some()
        || request.branch.is_some()
        || commit_exists(project_root, &request.head_sha)
    {
        return Ok(());
    }

    let Some(number) = request.pull_request_number else {
        return Err(
            "The pull request commit is not available locally. Refresh the project and try again."
                .to_owned(),
        );
    };
    let reference = format!("refs/shipyard/pull-requests/{number}/head");
    let refspec = format!("+refs/pull/{number}/head:{reference}");
    command::output(project_root, &["fetch", "--no-tags", "origin", &refspec])
        .map_err(|error| format!("Could not fetch pull request #{number} for review: {error}"))?;

    let fetched_sha = command::text(
        project_root,
        &["rev-parse", "--verify", &format!("{reference}^{{commit}}")],
    )?;
    if fetched_sha.trim() != request.head_sha {
        return Err(
            "The pull request changed on GitHub. Refresh the project and try again.".to_owned(),
        );
    }
    Ok(())
}

fn commit_exists(root: &Path, sha: &str) -> bool {
    command::optional_text(root, &["cat-file", "-e", &format!("{sha}^{{commit}}")]).is_some()
}

fn validate_target(
    project_root: &Path,
    checkout: Option<&Path>,
    request: &WorkItemDiffRequest,
) -> Result<(), String> {
    if let Some(checkout) = checkout {
        let actual_head = command::text(checkout, &["rev-parse", "HEAD"])?;
        if actual_head.trim() != request.head_sha {
            return Err(
                "This work item changed since the last scan. Refresh and try again.".to_owned(),
            );
        }
    }

    if let Some(branch) = request.branch.as_deref() {
        let reference = format!("refs/heads/{branch}");
        let branch_head = command::text(project_root, &["rev-parse", "--verify", &reference])?;
        if branch_head.trim() != request.head_sha {
            return Err(
                "This work item branch changed since the last scan. Refresh and try again."
                    .to_owned(),
            );
        }
        if let Some(checkout) = checkout {
            let current = command::text(checkout, &["symbolic-ref", "--quiet", "--short", "HEAD"])?;
            if current.trim() != branch {
                return Err("The selected worktree now contains a different branch.".to_owned());
            }
        }
    }
    Ok(())
}

fn comparison_base(
    project_root: &Path,
    request: &WorkItemDiffRequest,
) -> Result<(String, String), String> {
    let Some(default_branch) = request.default_branch.as_deref() else {
        return Ok((request.head_sha.clone(), "working tree".to_owned()));
    };
    if request.branch.as_deref() == Some(default_branch) {
        return Ok((request.head_sha.clone(), "working tree".to_owned()));
    }

    let reference = format!("refs/heads/{default_branch}");
    let base = command::text(project_root, &["merge-base", &reference, &request.head_sha])?;
    Ok((base.trim().to_owned(), default_branch.to_owned()))
}

fn append_untracked_files(root: &Path, patch: &mut String) -> Result<(), String> {
    let output = command::output(
        root,
        &["ls-files", "--others", "--exclude-standard", "-z", "--"],
    )?;
    for bytes in output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
    {
        let path = std::str::from_utf8(bytes)
            .map_err(|_| "An untracked file path is not valid UTF-8".to_owned())?;
        append_untracked_file(root, PathBuf::from(path), patch)?;
    }
    Ok(())
}

fn append_untracked_file(root: &Path, path: PathBuf, patch: &mut String) -> Result<(), String> {
    let path_text = path
        .to_str()
        .ok_or_else(|| "An untracked file path is not valid UTF-8".to_owned())?;
    let output = command::output_allow_failure(
        root,
        &[
            "diff",
            "--no-index",
            "--binary",
            "--",
            "/dev/null",
            path_text,
        ],
    );
    if !output.stdout.is_empty() {
        if !patch.ends_with('\n') && !patch.is_empty() {
            patch.push('\n');
        }
        patch.push_str(command::bytes_text(&output.stdout));
    } else if root
        .join(&path)
        .metadata()
        .is_ok_and(|metadata| metadata.len() == 0)
    {
        patch.push_str(&format!(
            "diff --git a/{path_text} b/{path_text}\nnew file mode 100644\nindex 0000000..e69de29\n"
        ));
    } else if !output.status.success() {
        return Err(format!(
            "Could not read untracked file {path_text}: {}",
            command::error(&output)
        ));
    }
    Ok(())
}
