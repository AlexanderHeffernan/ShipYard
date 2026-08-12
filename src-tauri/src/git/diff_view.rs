use super::{command, remote, repository};
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
    #[serde(default)]
    pub(super) pull_request_base_branch: Option<String>,
    #[serde(default)]
    pub(super) pull_request_head_sha: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemDiff {
    pub(super) patch: String,
    pub(super) comparison_label: String,
}

struct DiffReferences {
    base: String,
    head: Option<String>,
    label: String,
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
    validate_target(&project_root, checkout.as_deref(), &request)?;

    let references = comparison_references(&project_root, checkout.as_deref(), &request)?;
    let mut args = vec![
        "diff",
        "--no-ext-diff",
        "--find-renames",
        "--find-copies",
        "--binary",
        "--full-index",
        "--unified=5",
        &references.base,
    ];
    if checkout.is_none() {
        args.push(references.head.as_deref().unwrap_or(&request.head_sha));
    }
    args.push("--");

    let diff_root = checkout.as_deref().unwrap_or(&project_root);
    let mut patch = command::text(diff_root, &args)?;
    if checkout.is_some() {
        append_untracked_files(diff_root, &mut patch)?;
    }

    Ok(WorkItemDiff {
        patch,
        comparison_label: references.label,
    })
}

fn comparison_references(
    project_root: &Path,
    checkout: Option<&Path>,
    request: &WorkItemDiffRequest,
) -> Result<DiffReferences, String> {
    if let Some(number) = request.pull_request_number {
        return pull_request_references(project_root, checkout, request, number);
    }

    let Some(default_branch) = request.default_branch.as_deref() else {
        return Ok(DiffReferences {
            base: request.head_sha.clone(),
            head: None,
            label: "working tree".to_owned(),
        });
    };
    if request.branch.as_deref() == Some(default_branch) {
        return Ok(DiffReferences {
            base: request.head_sha.clone(),
            head: None,
            label: "working tree".to_owned(),
        });
    }

    local_branch_references(project_root, default_branch, &request.head_sha)
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

fn pull_request_references(
    project_root: &Path,
    checkout: Option<&Path>,
    request: &WorkItemDiffRequest,
    number: u64,
) -> Result<DiffReferences, String> {
    let base_branch = request
        .pull_request_base_branch
        .as_deref()
        .or(request.default_branch.as_deref())
        .ok_or_else(|| format!("Pull request #{number} does not specify a base branch"))?;
    let base_reference = remote::pull_request_base_reference(number);
    let base_cached = remote::cached_commit(project_root, &base_reference);
    let remote_name = remote::configured(project_root)
        .map(|remote| remote.name)
        .unwrap_or_else(|| "origin".to_owned());
    let base_source = match remote::fetch_branch(
        project_root,
        &remote_name,
        base_branch,
        &base_reference,
        &format!("the base branch {base_branch} for pull request #{number}"),
    ) {
        Ok(_) => "remote",
        Err(_error) if base_cached.is_some() => "cached",
        Err(error) => return Err(error),
    };

    let head_reference = remote::pull_request_head_reference(number);
    let expected_head = request
        .pull_request_head_sha
        .as_deref()
        .unwrap_or(&request.head_sha);
    let use_remote_head = checkout.is_none() && request.branch.is_none();
    let head_cached = remote::cached_commit(project_root, &head_reference);
    let head = match remote::fetch_pull_request_head(project_root, number, &head_reference) {
        Ok(fetched) if fetched == expected_head => Some(head_reference),
        Ok(_) if use_remote_head => {
            return Err(
                "The pull request changed on GitHub. Refresh the project and try again.".to_owned(),
            )
        }
        Ok(_) => None,
        Err(_error) if use_remote_head && head_cached.as_deref() == Some(expected_head) => {
            Some(head_reference)
        }
        Err(error) if use_remote_head => return Err(error),
        Err(_) => None,
    };
    let merge_head = head.as_deref().unwrap_or(&request.head_sha);
    let base = merge_base(project_root, &base_reference, merge_head)?;

    Ok(DiffReferences {
        base,
        head,
        label: match base_source {
            "remote" => base_branch.to_owned(),
            _ => format!("{base_branch} (cached)"),
        },
    })
}

fn local_branch_references(
    project_root: &Path,
    default_branch: &str,
    head_sha: &str,
) -> Result<DiffReferences, String> {
    let remote_reference = remote::base_reference(default_branch);
    let cached = remote::cached_commit(project_root, &remote_reference);
    let local_reference = format!("refs/heads/{default_branch}");
    let remote_name = remote::configured(project_root)
        .map(|remote| remote.name)
        .unwrap_or_else(|| "origin".to_owned());
    let local = remote::cached_commit(project_root, &local_reference);
    let source = match remote::fetch_branch(
        project_root,
        &remote_name,
        default_branch,
        &remote_reference,
        &format!("the remote base branch {default_branch}"),
    ) {
        Ok(_) => "remote",
        Err(_error) if cached.is_some() => "cached",
        Err(_error) if local.is_some() => "local",
        Err(error) => return Err(error),
    };
    let base = match source {
        "remote" | "cached" => remote_reference,
        _ => local_reference,
    };
    Ok(DiffReferences {
        base: merge_base(project_root, &base, head_sha)?,
        head: None,
        label: match source {
            "remote" => default_branch.to_owned(),
            "cached" => format!("{default_branch} (cached)"),
            _ if remote::has_remote(project_root, &remote_name) => format!("{default_branch} (local)"),
            _ => default_branch.to_owned(),
        },
    })
}

fn merge_base(root: &Path, base: &str, head: &str) -> Result<String, String> {
    command::text(root, &["merge-base", base, head]).map(|value| value.trim().to_owned())
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
