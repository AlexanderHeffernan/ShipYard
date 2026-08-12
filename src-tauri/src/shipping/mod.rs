use crate::{agents, git};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

mod cleanup;
pub(crate) use cleanup::ShippingCleanup;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShippingRequest {
    project_id: String,
    #[serde(rename = "workItemId")]
    _work_item_id: String,
    source_path: String,
    source_branch: Option<String>,
    default_branch: String,
    remote_name: String,
    remote_identity: Option<String>,
    github_repository: Option<String>,
    action: ShippingAction,
    pull_request_number: Option<u64>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ShippingAction {
    CreatePullRequest,
    UpdatePullRequest,
    MergePullRequest,
    DirectToMain,
    PushBranch,
    PushDefault,
    IntegrateToDefault,
}

pub(crate) struct PreparedShipping {
    pub(crate) script_path: PathBuf,
    pub(crate) working_directory: PathBuf,
    pub(crate) cleanup: Option<ShippingCleanup>,
}

pub(crate) fn prepare(base: &Path, request: ShippingRequest) -> Result<PreparedShipping, String> {
    let adapter = agents::selected(base)?;
    prepare_with_adapter(base, request, adapter.as_ref())
}

fn prepare_with_adapter(
    base: &Path,
    request: ShippingRequest,
    adapter: &dyn agents::AgentAdapter,
) -> Result<PreparedShipping, String> {
    let source = git::validate_worktree(&request.project_id, &request.source_path)?;
    let primary_checkout = git::primary_worktree_path(&source)?;
    let configured_remote = git::configured_remote(&source)
        .ok_or_else(|| "This project has no configured Git remote. Add a remote before shipping.".to_owned())?;
    if configured_remote.name != request.remote_name {
        return Err(
            "The configured Git remote changed since this project was scanned. Refresh the project and try again."
                .to_owned(),
        );
    }
    if request.remote_identity.as_deref() != Some(configured_remote.identity.as_str()) {
        return Err(
            "The configured Git remote URL changed since this project was scanned. Refresh the project and try again."
                .to_owned(),
        );
    }
    if let Some(detected_default) = git::remote_default_branch(&source, &request.remote_name) {
        if detected_default != request.default_branch {
            return Err(format!(
                "The remote default branch changed from {} to {}. Refresh the project and try again.",
                request.default_branch, detected_default
            ));
        }
    }
    let branch = request.source_branch.as_deref();
    if !matches!(request.action, ShippingAction::MergePullRequest) && branch.is_none() {
        return Err("Create a branch before shipping this work".to_owned());
    }
    if branch == Some(request.default_branch.as_str())
        && !matches!(request.action, ShippingAction::PushDefault)
    {
        return Err(format!(
            "Work on the default branch can only use Push {}",
            request.default_branch
        ));
    }
    if matches!(
        request.action,
        ShippingAction::CreatePullRequest
            | ShippingAction::UpdatePullRequest
            | ShippingAction::MergePullRequest
            | ShippingAction::DirectToMain
    ) && request.github_repository.is_none()
    {
        return Err("This project is not connected to a GitHub repository".to_owned());
    }
    if matches!(request.action, ShippingAction::PushDefault)
        && branch != Some(request.default_branch.as_str())
    {
        return Err(format!(
            "Push {} only from the local {} branch",
            request.default_branch, request.default_branch
        ));
    }
    let operation_id = format!(
        "shipping-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let operation_dir = base.join("operations").join(&operation_id);
    fs::create_dir_all(&operation_dir).map_err(|error| error.to_string())?;
    let metadata_prompt = operation_dir.join("metadata-prompt.txt");
    let conflict_prompt = operation_dir.join("conflict-prompt.txt");
    fs::write(&metadata_prompt, metadata_prompt_text(&request))
        .map_err(|error| error.to_string())?;
    fs::write(&conflict_prompt, conflict_prompt_text(&request))
        .map_err(|error| error.to_string())?;

    let script_path = operation_dir.join("ship.sh");
    let resolution_path = base.join("resolutions").join(&operation_id);
    let shipped_commit_path = operation_dir.join("shipped-commit.txt");
    let managed_checkout = matches!(request.action, ShippingAction::MergePullRequest)
        .then(|| request.pull_request_number.map(|number| {
            git::managed_pull_request_checkout_path(base, &request.project_id, number)
        }))
        .flatten();
    let script = script(
        &request,
        adapter,
        &operation_dir,
        &metadata_prompt,
        &conflict_prompt,
        &resolution_path,
        &shipped_commit_path,
        &primary_checkout,
        managed_checkout.as_deref(),
    )?;
    fs::write(&script_path, script).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o700))
            .map_err(|error| error.to_string())?;
    }
    Ok(PreparedShipping {
        script_path,
        working_directory: source.clone(),
        cleanup: matches!(request.action, ShippingAction::DirectToMain).then(|| ShippingCleanup {
            project_id: request.project_id,
            source,
            remote: request.remote_name,
            branch: branch.unwrap_or_default().to_owned(),
            base: request.default_branch,
            receipt: shipped_commit_path,
        }),
    })
}

pub(crate) fn cleanup_after_success(cleanup: &ShippingCleanup) -> Result<String, String> {
    cleanup::after_success(cleanup)
}

pub(crate) fn cleanup_project_id(cleanup: &ShippingCleanup) -> &str {
    &cleanup.project_id
}

fn metadata_prompt_text(request: &ShippingRequest) -> String {
    format!(
        "Inspect the work in this repository relative to {remote}/{base}. Return ONLY one JSON object with exactly these string fields: commitSubject, commitBody, pullRequestTitle, pullRequestBody. Describe the intent and user-visible effect accurately. Do not edit files, run destructive commands, commit, or push. Do not use Markdown fences. The branch is {branch}.",
        remote = request.remote_name,
        base = request.default_branch,
        branch = request.source_branch.as_deref().unwrap_or_default(),
    )
}

fn conflict_prompt_text(request: &ShippingRequest) -> String {
    format!(
        "Resolve the current Git merge conflicts while bringing branch {branch} up to date with its remote work and {remote}/{base}. Inspect both sides and preserve their intended behavior. Edit only what is needed to resolve the merge. Run focused checks when useful. Do not commit, push, rebase, switch branches, or abort the merge. Finish only when every conflict is resolved and staged or ready to stage.",
        remote = request.remote_name,
        base = request.default_branch,
        branch = request.source_branch.as_deref().unwrap_or_default(),
    )
}

fn script(
    request: &ShippingRequest,
    adapter: &dyn agents::AgentAdapter,
    operation_dir: &Path,
    metadata_prompt: &Path,
    conflict_prompt: &Path,
    resolution_path: &Path,
    shipped_commit_path: &Path,
    primary_checkout: &Path,
    managed_checkout: Option<&Path>,
) -> Result<String, String> {
    if matches!(
        request.action,
        ShippingAction::PushBranch
            | ShippingAction::PushDefault
            | ShippingAction::IntegrateToDefault
    ) {
        return generic_script(
            request,
            adapter,
            operation_dir,
            metadata_prompt,
            conflict_prompt,
            resolution_path,
        );
    }
    let branch = request.source_branch.as_deref().unwrap_or_default();
    let metadata_command = agent_command(
        adapter.executable(),
        &adapter.metadata_args(),
        metadata_prompt,
    );
    let conflict_command = agent_command(
        adapter.executable(),
        &adapter.conflict_args(),
        conflict_prompt,
    );
    let metadata = operation_dir.join("metadata-output.txt");
    let parsed = operation_dir.join("metadata.json");
    let subject = operation_dir.join("commit-subject.txt");
    let body = operation_dir.join("commit-body.txt");
    let pr_title = operation_dir.join("pr-title.txt");
    let pr_body = operation_dir.join("pr-body.txt");
    let source = Path::new(&request.source_path);
    let common = format!(
        r#"source={source}
branch={branch}
base={base}
remote={remote}
repository={repository}
resolution={resolution}
primary_checkout={primary_checkout}
managed_checkout={managed_checkout}

echo "Shipyard · checking local work"
{checkout_guard}

integrate_target() {{
  local source_sha="$1"
  local target="$2"
  local reason="$3"
  mkdir -p "$(dirname "$resolution")"
  git -C "$source" worktree add --detach "$resolution" "$source_sha"
  echo "Shipyard · $reason"
  if ! git -C "$resolution" merge --no-edit "$target"; then
    [[ -n "$(git -C "$resolution" diff --name-only --diff-filter=U)" ]]
    echo "Shipyard · resolving automatically with {agent_label}"
    (cd "$resolution" && {conflict_command})
  fi
  if [[ -n "$(git -C "$resolution" diff --name-only --diff-filter=U)" ]]; then
    echo "Shipyard · the agent left unresolved files" >&2
    echo "Resolution checkout preserved at $resolution" >&2
    exit 1
  fi
  git -C "$resolution" add -A
  if [[ -f "$(git -C "$resolution" rev-parse --git-path MERGE_HEAD)" ]]; then
    GIT_EDITOR=true git -C "$resolution" commit --no-edit
  fi
  RESOLVED_SHA="$(git -C "$resolution" rev-parse HEAD)"
  if ! git -C "$resolution" merge-base --is-ancestor "$source_sha" "$RESOLVED_SHA" ||
     ! git -C "$resolution" merge-base --is-ancestor "$target" "$RESOLVED_SHA"; then
    echo "Shipyard · the resolution did not preserve both sides of the merge" >&2
    echo "Resolution checkout preserved at $resolution" >&2
    exit 1
  fi
  if [[ "$(git -C "$source" branch --show-current)" == "$branch" ]] &&
     [[ "$(git -C "$source" rev-parse HEAD)" == "$source_sha" ]] &&
     [[ -z "$(git -C "$source" status --porcelain --untracked-files=normal)" ]]; then
    git -C "$source" merge --ff-only "$RESOLVED_SHA"
  else
    checked_out_branch="$(git -C "$source" worktree list --porcelain | awk -v expected="refs/heads/$branch" '$1 == "branch" && $2 == expected {{ print; exit }}')"
    [[ -z "$checked_out_branch" ]] || {{
      echo "ShipYard · $branch is checked out in another worktree; no local branch was changed" >&2
      echo "Resolution checkout preserved at $resolution" >&2
      exit 1
    }}
    git -C "$source" update-ref "refs/heads/$branch" "$RESOLVED_SHA" "$source_sha"
  fi
  git -C "$source" worktree remove "$resolution"
  echo "Shipyard · remote work integrated"
}}
"#,
        source = shell(source),
        branch = shell_text(branch),
        base = shell_text(&request.default_branch),
        remote = shell_text(&request.remote_name),
        repository = shell_text(request.github_repository.as_deref().unwrap_or_default()),
        resolution = shell(resolution_path),
        primary_checkout = shell(primary_checkout),
        managed_checkout = managed_checkout.map(shell).unwrap_or_else(|| "''".to_owned()),
        agent_label = adapter.label(),
        conflict_command = conflict_command,
        checkout_guard = if matches!(request.action, ShippingAction::MergePullRequest) {
            ":"
        } else {
            "[[ \"$(git -C \"$source\" branch --show-current)\" == \"$branch\" ]]"
        },
    );

    let action = match request.action {
        ShippingAction::CreatePullRequest
        | ShippingAction::UpdatePullRequest
        | ShippingAction::DirectToMain => format!(
            r#"
{metadata_guard}
echo "Shipyard · asking {agent_label} to describe the change"
({metadata_command}) | tee {metadata}
python3 - {metadata} {parsed} {subject} {body} {pr_title} {pr_body} <<'PY'
import json, pathlib, sys
raw = pathlib.Path(sys.argv[1]).read_text()
decoder = json.JSONDecoder()
value = None
for index, character in enumerate(raw):
    if character != '{{':
        continue
    try:
        candidate, _ = decoder.raw_decode(raw[index:])
        if all(isinstance(candidate.get(key), str) and candidate[key].strip() for key in ("commitSubject", "pullRequestTitle", "pullRequestBody")):
            value = candidate
            break
    except (json.JSONDecodeError, AttributeError):
        pass
if value is None:
    raise SystemExit("The coding agent did not return valid shipping metadata")
pathlib.Path(sys.argv[2]).write_text(json.dumps(value, indent=2))
for path, key in zip(sys.argv[3:], ("commitSubject", "commitBody", "pullRequestTitle", "pullRequestBody")):
    pathlib.Path(path).write_text(value.get(key, "").strip() + "\n")
PY

git -C "$source" add -A
if ! git -C "$source" diff --cached --quiet; then
  {{ cat {subject}; echo; cat {body}; }} | git -C "$source" commit -F -
  echo "Shipyard · committed local work"
fi
{metadata_guard_end}
[[ -z "$(git -C "$source" status --porcelain --untracked-files=normal)" ]]
git -C "$source" fetch "$remote" {fetch_targets}
source_sha="$(git -C "$source" rev-parse HEAD)"
{synchronize_pull_request}
if ! git -C "$source" merge-tree --write-tree "$source_sha" "refs/remotes/$remote/$base" >/dev/null; then
  integrate_target "$source_sha" "refs/remotes/$remote/$base" "branch conflicts with $remote/$base"
  source_sha="$RESOLVED_SHA"
fi
"#,
            metadata_guard = if matches!(request.action, ShippingAction::UpdatePullRequest) {
                "if [[ -n \"$(git -C \"$source\" status --porcelain --untracked-files=normal)\" ]]; then"
            } else {
                ""
            },
            metadata_guard_end = if matches!(request.action, ShippingAction::UpdatePullRequest) {
                "fi"
            } else {
                ""
            },
            agent_label = adapter.label(),
            metadata_command = metadata_command,
            metadata = shell(&metadata),
            parsed = shell(&parsed),
            subject = shell(&subject),
            body = shell(&body),
            pr_title = shell(&pr_title),
            pr_body = shell(&pr_body),
            fetch_targets = if matches!(request.action, ShippingAction::UpdatePullRequest) {
                "\"$base\" \"$branch\""
            } else {
                "\"$base\""
            },
            synchronize_pull_request =
                if matches!(request.action, ShippingAction::UpdatePullRequest) {
                    r#"remote_sha="$(git -C "$source" rev-parse "$remote/$branch")"
if ! git -C "$source" merge-base --is-ancestor "$remote_sha" "$source_sha"; then
  integrate_target "$source_sha" "$remote_sha" "local checkout and pull request need reconciliation"
  source_sha="$RESOLVED_SHA"
fi"#
                } else {
                    ""
                },
        ),
        ShippingAction::MergePullRequest => {
            let number = request
                .pull_request_number
                .ok_or_else(|| "Pull request number is required".to_owned())?;
            format!(
                r#"
{local_guard}
if [[ -n "${{source_sha:-}}" ]] && ! git -C "$source" merge-tree --write-tree "$source_sha" "refs/remotes/$remote/$base" >/dev/null; then
  integrate_target "$source_sha" "refs/remotes/$remote/$base" "branch conflicts with $remote/$base"
  git -C "$source" push "$remote" "${{RESOLVED_SHA}}:refs/heads/${{branch}}"
fi
echo "Shipyard · merging pull request #{number}"
gh pr merge {number} --repo "$repository" --squash --delete-branch
git -C "$source" fetch --prune "$remote"
if [[ -n "$managed_checkout" ]] && [[ -d "$managed_checkout" ]]; then
  cd "$primary_checkout"
  git worktree remove --force -- "$managed_checkout"
  echo "Shipyard · removed the merged pull request checkout"
fi
echo "Shipyard · pull request merged"
"#,
                number = number,
                local_guard = request.source_branch.as_deref().map(|branch| format!(
                    r#"branch={branch}
git -C "$source" fetch "$remote" "$base" "$branch"
if [[ "$(git -C "$source" branch --show-current)" == "$branch" ]] &&
   [[ -n "$(git -C "$source" status --porcelain --untracked-files=normal)" ]]; then
  echo "Shipyard · local changes are not in the pull request; update it before merging" >&2
  exit 1
fi
local_sha="$(git -C "$source" rev-parse "refs/heads/$branch")"
source_sha="$(git -C "$source" rev-parse "$remote/$branch")"
if [[ "$local_sha" != "$source_sha" ]]; then
  echo "Shipyard · local commits are not synchronized with the pull request; update it before merging" >&2
  exit 1
fi
"#,
                    branch = shell_text(branch),
                )).unwrap_or_else(|| "git -C \"$source\" fetch \"$remote\" \"$base\"\n".to_owned()),
            )
        }
        ShippingAction::PushBranch
        | ShippingAction::PushDefault
        | ShippingAction::IntegrateToDefault => unreachable!(),
    };

    let finish = match request.action {
        ShippingAction::CreatePullRequest => format!(
            r#"
git -C "$source" push -u "$remote" "HEAD:$branch"
echo "Shipyard · creating pull request"
gh pr create --repo "$repository" --base "$base" --head "$branch" --title "$(cat {title})" --body-file {body}
echo "Shipyard · pull request created"
"#,
            title = shell(&pr_title),
            body = shell(&pr_body),
        ),
        ShippingAction::UpdatePullRequest => r#"
git -C "$source" push "$remote" "HEAD:$branch"
echo "Shipyard · pull request updated"
"#
        .to_owned(),
        ShippingAction::DirectToMain => r#"
source_sha="$(git -C "$source" rev-parse HEAD)"
if ! git -C "$source" merge-base --is-ancestor "refs/remotes/$remote/$base" "$source_sha"; then
  integrate_target "$source_sha" "refs/remotes/$remote/$base" "branch needs the latest $remote/$base"
  source_sha="$RESOLVED_SHA"
fi
[[ "$(git -C "$source" branch --show-current)" == "$branch" ]]
[[ "$(git -C "$source" rev-parse HEAD)" == "$source_sha" ]]
[[ -z "$(git -C "$source" status --porcelain --untracked-files=normal)" ]]
echo "ShipYard · pushing the resolved commit directly to $base"
git -C "$source" push "$remote" "${source_sha}:refs/heads/$base"
git -C "$source" fetch "$remote" "$base"
git -C "$source" merge-base --is-ancestor "$source_sha" "refs/remotes/$remote/$base"
printf '%s\n' "$source_sha" > {shipped_commit}
echo "ShipYard · shipped directly to $base"
"#
        .replace("{shipped_commit}", &shell(shipped_commit_path)),
        ShippingAction::MergePullRequest => String::new(),
        ShippingAction::PushBranch
        | ShippingAction::PushDefault
        | ShippingAction::IntegrateToDefault => unreachable!(),
    };

    Ok(format!(
        "#!/bin/zsh\nset -euo pipefail\n\n{common}\n{action}\n{finish}"
    ))
}

fn generic_script(
    request: &ShippingRequest,
    adapter: &dyn agents::AgentAdapter,
    operation_dir: &Path,
    metadata_prompt: &Path,
    conflict_prompt: &Path,
    resolution_path: &Path,
) -> Result<String, String> {
    let branch = request.source_branch.as_deref().unwrap_or_default();
    let source = Path::new(&request.source_path);
    let metadata = operation_dir.join("metadata-output.txt");
    let parsed = operation_dir.join("metadata.json");
    let subject = operation_dir.join("commit-subject.txt");
    let body = operation_dir.join("commit-body.txt");
    let metadata_command = agent_command(
        adapter.executable(),
        &adapter.metadata_args(),
        metadata_prompt,
    );
    let conflict_command = agent_command(
        adapter.executable(),
        &adapter.conflict_args(),
        conflict_prompt,
    );
    let common = format!(
        r#"source={source}
branch={branch}
base={base}
remote={remote}
resolution={resolution}

export GIT_TERMINAL_PROMPT=0

echo "ShipYard · checking the selected checkout"
git -C "$source" check-ref-format --branch "$branch"
git -C "$source" check-ref-format --branch "$base"
current_branch="$(git -C "$source" branch --show-current)"
checkout_status="$(git -C "$source" status --porcelain --untracked-files=normal)"

if [[ "$current_branch" == "$branch" ]]; then
  source_ref="HEAD"
else
  [[ -z "$checkout_status" ]] || {{
    echo "ShipYard · the selected checkout is on $current_branch and has local changes; no files were changed" >&2
    exit 1
  }}
  source_ref="refs/heads/$branch"
fi

remote_branch_exists() {{
  git -C "$source" ls-remote --exit-code --heads "$remote" "refs/heads/$1" >/dev/null 2>&1
  local remote_status=$?
  if [[ "$remote_status" == 0 ]]; then
    return 0
  fi
  if [[ "$remote_status" == 2 ]]; then
    return 1
  fi
  echo "ShipYard · could not inspect $remote/$1; no remote changes were made" >&2
  return "$remote_status"
}}

fetch_remote_branch() {{
  local name="$1"
  git -C "$source" fetch --no-tags --no-write-fetch-head "$remote" "+refs/heads/${{name}}:refs/remotes/${{remote}}/${{name}}"
}}

integrate_target() {{
  local source_sha="$1"
  local target="$2"
  local reason="$3"
  mkdir -p "$(dirname "$resolution")"
  git -C "$source" worktree add --detach "$resolution" "$source_sha"
  echo "ShipYard · $reason"
  if ! git -C "$resolution" merge --no-edit "$target"; then
    [[ -n "$(git -C "$resolution" diff --name-only --diff-filter=U)" ]]
    echo "ShipYard · resolving automatically with {agent_label}"
    (cd "$resolution" && {conflict_command})
  fi
  if [[ -n "$(git -C "$resolution" diff --name-only --diff-filter=U)" ]]; then
    echo "ShipYard · the agent left unresolved files" >&2
    echo "Resolution checkout preserved at $resolution" >&2
    exit 1
  fi
  git -C "$resolution" add -A
  if [[ -f "$(git -C "$resolution" rev-parse --git-path MERGE_HEAD)" ]]; then
    GIT_EDITOR=true git -C "$resolution" commit --no-edit
  fi
  RESOLVED_SHA="$(git -C "$resolution" rev-parse HEAD)"
  if ! git -C "$resolution" merge-base --is-ancestor "$source_sha" "$RESOLVED_SHA" ||
     ! git -C "$resolution" merge-base --is-ancestor "$target" "$RESOLVED_SHA"; then
    echo "ShipYard · the resolution did not preserve both sides of the merge" >&2
    echo "Resolution checkout preserved at $resolution" >&2
    exit 1
  fi
  if [[ "$(git -C "$source" branch --show-current)" == "$branch" ]] &&
     [[ "$(git -C "$source" rev-parse HEAD)" == "$source_sha" ]] &&
     [[ -z "$(git -C "$source" status --porcelain --untracked-files=normal)" ]]; then
    git -C "$source" merge --ff-only "$RESOLVED_SHA"
  else
    checked_out_branch="$(git -C "$source" worktree list --porcelain | awk -v expected="refs/heads/$branch" '$1 == "branch" && $2 == expected {{ print; exit }}')"
    [[ -z "$checked_out_branch" ]] || {{
      echo "ShipYard · $branch is checked out in another worktree; no local branch was changed" >&2
      echo "Resolution checkout preserved at $resolution" >&2
      exit 1
    }}
    git -C "$source" update-ref "refs/heads/$branch" "$RESOLVED_SHA" "$source_sha"
  fi
  git -C "$source" worktree remove "$resolution"
  echo "ShipYard · remote work integrated"
}}
"#,
        source = shell(source),
        branch = shell_text(branch),
        base = shell_text(&request.default_branch),
        remote = shell_text(&request.remote_name),
        resolution = shell(resolution_path),
        agent_label = adapter.label(),
        conflict_command = conflict_command,
    );

    let metadata = format!(
        r#"
if [[ -n "$checkout_status" ]]; then
  [[ "$current_branch" == "$branch" ]] || {{
    echo "ShipYard · local changes are not on $branch; no files were changed" >&2
    exit 1
  }}
  echo "ShipYard · asking {agent_label} to describe the local change"
  ({metadata_command}) | tee {metadata}
  python3 - {metadata} {parsed} {subject} {body} <<'PY'
import json, pathlib, sys
raw = pathlib.Path(sys.argv[1]).read_text()
decoder = json.JSONDecoder()
value = None
for index, character in enumerate(raw):
    if character != '{{':
        continue
    try:
        candidate, _ = decoder.raw_decode(raw[index:])
        if all(isinstance(candidate.get(key), str) and candidate[key].strip() for key in ("commitSubject",)):
            value = candidate
            break
    except (json.JSONDecodeError, AttributeError):
        pass
if value is None:
    raise SystemExit("The coding agent did not return valid commit metadata")
pathlib.Path(sys.argv[2]).write_text(json.dumps(value, indent=2))
pathlib.Path(sys.argv[3]).write_text(value["commitSubject"].strip() + "\n")
pathlib.Path(sys.argv[4]).write_text(value.get("commitBody", "").strip() + "\n")
PY
  git -C "$source" add -A
  if ! git -C "$source" diff --cached --quiet; then
    {{ cat {subject}; echo; cat {body}; }} | git -C "$source" commit -F -
    echo "ShipYard · committed local work"
  fi
  checkout_status="$(git -C "$source" status --porcelain --untracked-files=normal)"
fi
[[ -z "$checkout_status" ]] || {{
  echo "ShipYard · local changes remain after preparing the work; no remote changes were made" >&2
  exit 1
}}
"#,
        agent_label = adapter.label(),
        metadata_command = metadata_command,
        metadata = shell(&metadata),
        parsed = shell(&parsed),
        subject = shell(&subject),
        body = shell(&body),
    );

    let operation = match request.action {
        ShippingAction::PushBranch => r#"
source_sha="$(git -C "$source" rev-parse "$source_ref")"
if remote_branch_exists "$branch"; then
  fetch_remote_branch "$branch"
  remote_sha="$(git -C "$source" rev-parse "refs/remotes/$remote/$branch")"
  if ! git -C "$source" merge-base --is-ancestor "$remote_sha" "$source_sha"; then
    integrate_target "$source_sha" "$remote_sha" "branch is behind remote work"
    source_sha="$RESOLVED_SHA"
  fi
else
  remote_status=$?
  [[ "$remote_status" == 1 ]] || exit "$remote_status"
fi
[[ "$(git -C "$source" branch --show-current)" != "$branch" || "$(git -C "$source" rev-parse HEAD)" == "$source_sha" ]]
if [[ "$(git -C "$source" branch --show-current)" == "$branch" ]]; then
  git -C "$source" push -u "$remote" "$branch"
else
  git -C "$source" push "$remote" "${source_sha}:refs/heads/${branch}"
  git -C "$source" branch --set-upstream-to="$remote/$branch" "$branch"
fi
fetch_remote_branch "$branch"
[[ "$(git -C "$source" rev-parse "refs/remotes/$remote/$branch")" == "$source_sha" ]]
echo "ShipYard · pushed $branch to $remote and set its upstream"
"#,
        ShippingAction::PushDefault => r#"
[[ "$current_branch" == "$branch" ]] || {
  echo "ShipYard · Push $base requires the selected checkout to be on $base; no files were changed" >&2
  exit 1
}
if remote_branch_exists "$base"; then
  fetch_remote_branch "$base"
  remote_sha="$(git -C "$source" rev-parse "refs/remotes/$remote/$base")"
  git -C "$source" merge-base --is-ancestor "$remote_sha" "$source_sha" || {
    echo "ShipYard · $remote/$base is ahead; pull or reconcile it before pushing" >&2
    exit 1
  }
else
  remote_status=$?
  [[ "$remote_status" == 1 ]] || exit "$remote_status"
fi
git -C "$source" push -u "$remote" "${source_sha}:refs/heads/${base}"
fetch_remote_branch "$base"
[[ "$(git -C "$source" rev-parse "refs/remotes/$remote/$base")" == "$source_sha" ]]
echo "ShipYard · pushed $base to $remote"
"#,
        ShippingAction::IntegrateToDefault => r#"
source_sha="$(git -C "$source" rev-parse "$source_ref")"
if remote_branch_exists "$base"; then
  fetch_remote_branch "$base"
  target="refs/remotes/$remote/$base"
else
  remote_status=$?
  [[ "$remote_status" == 1 ]] || exit "$remote_status"
  target="refs/heads/$base"
fi
if ! git -C "$source" merge-base --is-ancestor "$target" "$source_sha"; then
  integrate_target "$source_sha" "$target" "integrating $target into $branch before pushing"
  source_sha="$RESOLVED_SHA"
fi
git -C "$source" push "$remote" "${source_sha}:refs/heads/${base}"
fetch_remote_branch "$base"
git -C "$source" merge-base --is-ancestor "$source_sha" "refs/remotes/$remote/$base"
echo "ShipYard · integrated $branch and pushed $base to $remote"
"#,
        _ => unreachable!(),
    };
    let source_sha = "source_sha=\"$(git -C \"$source\" rev-parse \"$source_ref\")\"";

    Ok(format!(
        "#!/bin/zsh\nset -euo pipefail\n\n{common}\n{metadata}\n{source_sha}\n{operation}",
        source_sha = source_sha,
    ))
}

fn agent_command(executable: &Path, args: &[String], prompt: &Path) -> String {
    let mut command = shell(executable);
    for arg in args {
        command.push(' ');
        command.push_str(&shell_text(arg));
    }
    command.push_str(" < ");
    command.push_str(&shell(prompt));
    command
}

fn shell(path: &Path) -> String {
    shell_text(&path.to_string_lossy())
}

fn shell_text(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::{prepare_with_adapter, PreparedShipping, ShippingAction, ShippingRequest};
    use crate::agents::AgentAdapter;
    use std::{
        fs,
        path::{Path, PathBuf},
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TestAdapter {
        executable: PathBuf,
    }

    impl AgentAdapter for TestAdapter {
        fn label(&self) -> &str {
            "Test agent"
        }

        fn executable(&self) -> &Path {
            &self.executable
        }

        fn metadata_args(&self) -> Vec<String> {
            Vec::new()
        }

        fn conflict_args(&self) -> Vec<String> {
            Vec::new()
        }
    }

    #[test]
    fn direct_shipping_commits_and_resolves_a_changed_base_with_an_adapter() {
        let root = temporary("pipeline");
        let remote = root.join("remote.git");
        let checkout = root.join("checkout");
        let data = root.join("data");
        run(&root, &["init", "--bare", remote.to_str().unwrap()]);
        run(
            &root,
            &[
                "clone",
                remote.to_str().unwrap(),
                checkout.to_str().unwrap(),
            ],
        );
        run(&checkout, &["switch", "-c", "main"]);
        run(&checkout, &["config", "user.name", "ShipYard Test"]);
        run(
            &checkout,
            &["config", "user.email", "shipyard@example.test"],
        );
        fs::write(checkout.join("conflict.txt"), "base\n").unwrap();
        run(&checkout, &["add", "."]);
        run(&checkout, &["commit", "-m", "Initial"]);
        run(&checkout, &["push", "-u", "origin", "main"]);
        run(&checkout, &["switch", "-c", "feature/test"]);
        fs::write(checkout.join("conflict.txt"), "feature\n").unwrap();
        run(&checkout, &["add", "."]);
        run(&checkout, &["commit", "-m", "Feature"]);
        run(&checkout, &["switch", "main"]);
        fs::write(checkout.join("conflict.txt"), "main\n").unwrap();
        run(&checkout, &["add", "."]);
        run(&checkout, &["commit", "-m", "Advance main"]);
        run(&checkout, &["push", "origin", "main"]);
        run(&checkout, &["switch", "feature/test"]);
        fs::write(checkout.join("work.txt"), "uncommitted\n").unwrap();

        let adapter = test_adapter(&root);
        let project_id = crate::git::resolve(checkout.to_str().unwrap())
            .unwrap()
            .1
            .to_string_lossy()
            .into_owned();
        let prepared = prepare_with_adapter(
            &data,
            ShippingRequest {
                project_id,
                _work_item_id: "test-item".into(),
                source_path: checkout.to_string_lossy().into_owned(),
                source_branch: Some("feature/test".into()),
                default_branch: "main".into(),
                remote_name: "origin".into(),
                remote_identity: Some(configured_remote_identity(&checkout)),
                github_repository: Some("owner/repo".into()),
                action: ShippingAction::DirectToMain,
                pull_request_number: None,
            },
            &adapter,
        )
        .unwrap();
        let previous_main = text(&checkout, &["rev-parse", "origin/main"]);
        let output = Command::new("/bin/zsh")
            .arg(&prepared.script_path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        run(&checkout, &["fetch", "origin", "main"]);
        assert_eq!(
            text(&checkout, &["rev-parse", "origin/main"]),
            text(&checkout, &["rev-parse", "HEAD"])
        );
        assert_ne!(previous_main, text(&checkout, &["rev-parse", "HEAD"]));
        assert_eq!(
            fs::read_to_string(&prepared.cleanup.as_ref().unwrap().receipt)
                .unwrap()
                .trim(),
            text(&checkout, &["rev-parse", "HEAD"])
        );
        assert_eq!(
            fs::read_to_string(checkout.join("conflict.txt")).unwrap(),
            "resolved\n"
        );
        assert_eq!(text(&checkout, &["status", "--porcelain"]), "");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn updating_a_pull_request_commits_and_pushes_local_work() {
        let root = temporary("update-pr");
        let remote = root.join("remote.git");
        let checkout = root.join("checkout");
        let data = root.join("data");
        run(&root, &["init", "--bare", remote.to_str().unwrap()]);
        run(
            &root,
            &[
                "clone",
                remote.to_str().unwrap(),
                checkout.to_str().unwrap(),
            ],
        );
        run(&checkout, &["switch", "-c", "main"]);
        run(&checkout, &["config", "user.name", "ShipYard Test"]);
        run(
            &checkout,
            &["config", "user.email", "shipyard@example.test"],
        );
        fs::write(checkout.join("README.md"), "initial\n").unwrap();
        run(&checkout, &["add", "."]);
        run(&checkout, &["commit", "-m", "Initial"]);
        run(&checkout, &["push", "-u", "origin", "main"]);
        run(&checkout, &["switch", "-c", "feature/update"]);
        fs::write(checkout.join("feature.txt"), "first version\n").unwrap();
        run(&checkout, &["add", "."]);
        run(&checkout, &["commit", "-m", "Start feature"]);
        run(&checkout, &["push", "-u", "origin", "feature/update"]);
        let previous_pull_request_head = text(&checkout, &["rev-parse", "origin/feature/update"]);
        fs::write(checkout.join("feature.txt"), "updated locally\n").unwrap();

        let project_id = crate::git::resolve(checkout.to_str().unwrap())
            .unwrap()
            .1
            .to_string_lossy()
            .into_owned();
        let adapter = test_adapter(&root);
        let blocked_merge = prepare_with_adapter(
            &data,
            ShippingRequest {
                project_id: project_id.clone(),
                _work_item_id: "test-item".into(),
                source_path: checkout.to_string_lossy().into_owned(),
                source_branch: Some("feature/update".into()),
                default_branch: "main".into(),
                remote_name: "origin".into(),
                remote_identity: Some(configured_remote_identity(&checkout)),
                github_repository: Some("owner/repo".into()),
                action: ShippingAction::MergePullRequest,
                pull_request_number: Some(1),
            },
            &adapter,
        )
        .unwrap();
        let blocked_output = Command::new("/bin/zsh")
            .arg(&blocked_merge.script_path)
            .output()
            .unwrap();
        assert!(!blocked_output.status.success());
        assert!(String::from_utf8_lossy(&blocked_output.stderr)
            .contains("local changes are not in the pull request"));
        assert!(fs::read_to_string(&blocked_merge.script_path)
            .unwrap()
            .contains("removed the merged pull request checkout"));

        let prepared = prepare_with_adapter(
            &data,
            ShippingRequest {
                project_id,
                _work_item_id: "test-item".into(),
                source_path: checkout.to_string_lossy().into_owned(),
                source_branch: Some("feature/update".into()),
                default_branch: "main".into(),
                remote_name: "origin".into(),
                remote_identity: Some(configured_remote_identity(&checkout)),
                github_repository: Some("owner/repo".into()),
                action: ShippingAction::UpdatePullRequest,
                pull_request_number: Some(1),
            },
            &adapter,
        )
        .unwrap();
        let output = Command::new("/bin/zsh")
            .arg(prepared.script_path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        run(&checkout, &["fetch", "origin", "feature/update"]);
        let updated_pull_request_head = text(&checkout, &["rev-parse", "origin/feature/update"]);
        assert_ne!(previous_pull_request_head, updated_pull_request_head);
        assert_eq!(
            updated_pull_request_head,
            text(&checkout, &["rev-parse", "HEAD"])
        );
        assert_eq!(text(&checkout, &["status", "--porcelain"]), "");
        assert_eq!(
            text(&checkout, &["show", "-s", "--format=%s", "HEAD"]),
            "Ship work"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pushes_dirty_worktree_branch_to_a_non_github_remote_and_sets_upstream() {
        let fixture = GenericFixture::new("push-branch", "trunk");
        let linked = fixture.root.join("feature-worktree");
        run(
            &fixture.checkout,
            &[
                "worktree",
                "add",
                "-b",
                "feature/azure",
                linked.to_str().unwrap(),
                "trunk",
            ],
        );
        fs::write(linked.join("feature.txt"), "local work\n").unwrap();

        let prepared = fixture.prepare(
            &linked,
            "feature/azure",
            ShippingAction::PushBranch,
        );
        let output = run_script(&prepared.script_path);

        assert!(output.status.success(), "{}", output_text(&output));
        assert_eq!(
            text(&fixture.remote, &["rev-parse", "refs/heads/feature/azure"]),
            text(&linked, &["rev-parse", "HEAD"])
        );
        assert_eq!(
            text(&linked, &["config", "--get", "branch.feature/azure.remote"]),
            "azure"
        );
        assert_eq!(
            text(&linked, &["config", "--get", "branch.feature/azure.merge"]),
            "refs/heads/feature/azure"
        );
        assert_eq!(text(&linked, &["status", "--porcelain"]), "");
        fixture.remove_worktree(&linked);
        fixture.remove();
    }

    #[test]
    fn reconciles_a_remote_ahead_feature_branch_before_pushing_it() {
        let fixture = GenericFixture::new("push-branch-non-fast-forward", "trunk");
        let linked = fixture.add_feature_worktree("feature/non-fast-forward");
        fs::write(linked.join("local.txt"), "local\n").unwrap();
        run(&linked, &["add", "."]);
        run(&linked, &["commit", "-m", "Local feature"]);
        run(&linked, &["push", "-u", "azure", "feature/non-fast-forward"]);

        let other = fixture.root.join("other-checkout");
        run(
            &fixture.root,
            &[
                "clone",
                fixture.remote.to_str().unwrap(),
                other.to_str().unwrap(),
            ],
        );
        run(&other, &["remote", "rename", "origin", "azure"]);
        run(&other, &["switch", "feature/non-fast-forward"]);
        run(&other, &["config", "user.name", "ShipYard Test"]);
        run(
            &other,
            &["config", "user.email", "shipyard@example.test"],
        );
        fs::write(other.join("remote.txt"), "remote\n").unwrap();
        run(&other, &["add", "."]);
        run(&other, &["commit", "-m", "Remote feature"]);
        run(&other, &["push", "azure", "feature/non-fast-forward"]);

        let prepared = fixture.prepare(
            &linked,
            "feature/non-fast-forward",
            ShippingAction::PushBranch,
        );
        let output = run_script(&prepared.script_path);

        assert!(output.status.success(), "{}", output_text(&output));
        let remote_sha = text(
            &fixture.remote,
            &["rev-parse", "refs/heads/feature/non-fast-forward"],
        );
        assert_eq!(text(&linked, &["rev-parse", "HEAD"]), remote_sha);
        assert_eq!(text(&fixture.remote, &["show", &format!("{remote_sha}:local.txt")]), "local");
        assert_eq!(text(&fixture.remote, &["show", &format!("{remote_sha}:remote.txt")]), "remote");
        fixture.remove_worktree(&linked);
        fixture.remove();
    }

    #[test]
    fn pushes_dirty_non_main_default_branch_without_using_main_as_a_special_case() {
        let fixture = GenericFixture::new("push-default", "trunk");
        fs::write(fixture.checkout.join("release-note.txt"), "ready\n").unwrap();

        let prepared = fixture.prepare(
            &fixture.checkout,
            "trunk",
            ShippingAction::PushDefault,
        );
        let output = run_script(&prepared.script_path);

        assert!(output.status.success(), "{}", output_text(&output));
        assert_eq!(
            text(&fixture.remote, &["rev-parse", "refs/heads/trunk"]),
            text(&fixture.checkout, &["rev-parse", "HEAD"])
        );
        assert!(text(&fixture.checkout, &["show", "-s", "--format=%s", "HEAD"])
            .contains("Ship work"));
        fixture.remove();
    }

    #[test]
    fn refuses_to_force_push_when_the_default_remote_branch_is_ahead() {
        let fixture = GenericFixture::new("push-default-ahead", "trunk");
        let other = fixture.root.join("other-default-checkout");
        run(
            &fixture.root,
            &[
                "clone",
                fixture.remote.to_str().unwrap(),
                other.to_str().unwrap(),
            ],
        );
        run(&other, &["remote", "rename", "origin", "azure"]);
        run(&other, &["config", "user.name", "ShipYard Test"]);
        run(
            &other,
            &["config", "user.email", "shipyard@example.test"],
        );
        fs::write(other.join("remote-only.txt"), "remote\n").unwrap();
        run(&other, &["add", "."]);
        run(&other, &["commit", "-m", "Advance trunk remotely"]);
        run(&other, &["push", "azure", "trunk"]);

        let local_sha = text(&fixture.checkout, &["rev-parse", "HEAD"]);
        let prepared = fixture.prepare(
            &fixture.checkout,
            "trunk",
            ShippingAction::PushDefault,
        );
        let output = run_script(&prepared.script_path);

        assert!(!output.status.success());
        assert!(output_text(&output).contains("is ahead; pull or reconcile it before pushing"));
        assert_eq!(text(&fixture.checkout, &["rev-parse", "HEAD"]), local_sha);
        assert_ne!(text(&fixture.remote, &["rev-parse", "refs/heads/trunk"]), local_sha);
        fixture.remove();
    }

    #[test]
    fn integrates_a_feature_worktree_into_a_non_main_default_branch() {
        let fixture = GenericFixture::new("integrate-default", "trunk");
        let linked = fixture.add_feature_worktree("feature/integrate");
        fs::write(linked.join("feature.txt"), "feature\n").unwrap();
        run(&linked, &["add", "."]);
        run(&linked, &["commit", "-m", "Feature"]);

        let prepared = fixture.prepare(
            &linked,
            "feature/integrate",
            ShippingAction::IntegrateToDefault,
        );
        let output = run_script(&prepared.script_path);

        assert!(output.status.success(), "{}", output_text(&output));
        let default_sha = text(&fixture.remote, &["rev-parse", "refs/heads/trunk"]);
        let feature_file = format!("{default_sha}:feature.txt");
        assert_eq!(
            text(&fixture.checkout, &["show", &feature_file]),
            "feature"
        );
        assert_eq!(
            text(&fixture.remote, &["rev-parse", "refs/heads/trunk"]),
            text(&linked, &["rev-parse", "HEAD"])
        );
        fixture.remove_worktree(&linked);
        fixture.remove();
    }

    #[test]
    fn resolves_conflicts_in_an_isolated_worktree_before_integrating_to_default() {
        let fixture = GenericFixture::new("integrate-conflict", "trunk");
        let linked = fixture.add_feature_worktree("feature/conflict");
        fs::write(linked.join("conflict.txt"), "feature\n").unwrap();
        run(&linked, &["add", "conflict.txt"]);
        run(&linked, &["commit", "-m", "Feature conflict"]);
        fs::write(fixture.checkout.join("conflict.txt"), "default\n").unwrap();
        run(&fixture.checkout, &["add", "conflict.txt"]);
        run(&fixture.checkout, &["commit", "-m", "Advance trunk"]);
        run(&fixture.checkout, &["push", "azure", "trunk"]);

        let prepared = fixture.prepare(
            &linked,
            "feature/conflict",
            ShippingAction::IntegrateToDefault,
        );
        let output = run_script(&prepared.script_path);

        assert!(output.status.success(), "{}", output_text(&output));
        let default_sha = text(&fixture.remote, &["rev-parse", "refs/heads/trunk"]);
        let conflict_file = format!("{default_sha}:conflict.txt");
        assert_eq!(
            text(&fixture.remote, &["show", &conflict_file]),
            "resolved"
        );
        assert_eq!(text(&linked, &["status", "--porcelain"]), "");
        assert!(!prepared
            .script_path
            .parent()
            .unwrap()
            .join("../resolutions")
            .exists());
        fixture.remove_worktree(&linked);
        fixture.remove();
    }

    #[test]
    fn refuses_to_touch_a_remote_when_the_selected_checkout_is_dirty_on_another_branch() {
        let fixture = GenericFixture::new("dirty-guard", "trunk");
        run(&fixture.checkout, &["switch", "-c", "feature/guard"]);
        run(&fixture.checkout, &["switch", "trunk"]);
        fs::write(fixture.checkout.join("unrelated.txt"), "keep me\n").unwrap();
        let prepared = fixture.prepare(
            &fixture.checkout,
            "feature/guard",
            ShippingAction::PushBranch,
        );
        let output = run_script(&prepared.script_path);

        assert!(!output.status.success());
        assert!(output_text(&output).contains("selected checkout is on trunk and has local changes"));
        assert!(!ref_exists(&fixture.remote, "refs/heads/feature/guard"));
        assert!(fixture.checkout.join("unrelated.txt").exists());
        fixture.remove();
    }

    #[test]
    fn reports_a_missing_configured_remote_before_creating_a_shipping_script() {
        let fixture = GenericFixture::new("missing-remote", "trunk");
        run(&fixture.checkout, &["remote", "remove", "azure"]);
        let project_id = fixture.project_id.clone();
        let error = match prepare_with_adapter(
            &fixture.data,
            ShippingRequest {
                project_id,
                _work_item_id: "test-item".into(),
                source_path: fixture.checkout.to_string_lossy().into_owned(),
                source_branch: Some("trunk".into()),
                default_branch: "trunk".into(),
                remote_name: "azure".into(),
                remote_identity: Some("stale-remote-identity".into()),
                github_repository: None,
                action: ShippingAction::PushDefault,
                pull_request_number: None,
            },
            &fixture.adapter,
        ) {
            Ok(_) => panic!("shipping preparation unexpectedly succeeded"),
            Err(error) => error,
        };

        assert!(error.contains("no configured Git remote"));
        fixture.remove();
    }

    #[test]
    fn refuses_to_ship_when_a_remote_url_changes_after_scanning() {
        let fixture = GenericFixture::new("changed-remote-url", "trunk");
        let stale_identity = configured_remote_identity(&fixture.checkout);
        run(
            &fixture.checkout,
            &["remote", "set-url", "azure", "/tmp/another-repository.git"],
        );
        let error = match prepare_with_adapter(
            &fixture.data,
            ShippingRequest {
                project_id: fixture.project_id.clone(),
                _work_item_id: "test-item".into(),
                source_path: fixture.checkout.to_string_lossy().into_owned(),
                source_branch: Some("trunk".into()),
                default_branch: "trunk".into(),
                remote_name: "azure".into(),
                remote_identity: Some(stale_identity),
                github_repository: None,
                action: ShippingAction::PushDefault,
                pull_request_number: None,
            },
            &fixture.adapter,
        ) {
            Ok(_) => panic!("shipping preparation unexpectedly succeeded"),
            Err(error) => error,
        };

        assert!(error.contains("remote URL changed"));
        fixture.remove();
    }

    #[test]
    fn refuses_to_ship_when_the_remote_default_branch_changes_after_scanning() {
        let fixture = GenericFixture::new("changed-default-branch", "trunk");
        run(&fixture.checkout, &["branch", "release"]);
        run(&fixture.checkout, &["push", "azure", "release"]);
        run(
            &fixture.remote,
            &["symbolic-ref", "HEAD", "refs/heads/release"],
        );

        let error = match prepare_with_adapter(
            &fixture.data,
            ShippingRequest {
                project_id: fixture.project_id.clone(),
                _work_item_id: "test-item".into(),
                source_path: fixture.checkout.to_string_lossy().into_owned(),
                source_branch: Some("trunk".into()),
                default_branch: "trunk".into(),
                remote_name: "azure".into(),
                remote_identity: Some(configured_remote_identity(&fixture.checkout)),
                github_repository: None,
                action: ShippingAction::PushDefault,
                pull_request_number: None,
            },
            &fixture.adapter,
        ) {
            Ok(_) => panic!("shipping preparation unexpectedly succeeded"),
            Err(error) => error,
        };

        assert!(error.contains("default branch changed from trunk to release"));
        fixture.remove();
    }

    struct GenericFixture {
        root: PathBuf,
        remote: PathBuf,
        checkout: PathBuf,
        data: PathBuf,
        project_id: String,
        adapter: TestAdapter,
    }

    impl GenericFixture {
        fn new(label: &str, default_branch: &str) -> Self {
            let root = temporary(label);
            let remote = root.join("remote.git");
            let checkout = root.join("checkout");
            let data = root.join("data");
            run(&root, &["init", "--bare", remote.to_str().unwrap()]);
            run(
                &root,
                &[
                    "clone",
                    remote.to_str().unwrap(),
                    checkout.to_str().unwrap(),
                ],
            );
            run(&checkout, &["switch", "-c", default_branch]);
            run(&checkout, &["config", "user.name", "ShipYard Test"]);
            run(
                &checkout,
                &["config", "user.email", "shipyard@example.test"],
            );
            run(
                &checkout,
                &["remote", "rename", "origin", "azure"],
            );
            fs::write(checkout.join("README.md"), "initial\n").unwrap();
            run(&checkout, &["add", "."]);
            run(&checkout, &["commit", "-m", "Initial"]);
            run(&checkout, &["push", "-u", "azure", default_branch]);
            run(
                &remote,
                &["symbolic-ref", "HEAD", &format!("refs/heads/{default_branch}")],
            );
            let project_id = crate::git::resolve(checkout.to_str().unwrap())
                .unwrap()
                .1
                .to_string_lossy()
                .into_owned();
            let adapter = test_adapter(&root);
            Self {
                root,
                remote,
                checkout,
                data,
                project_id,
                adapter,
            }
        }

        fn add_feature_worktree(&self, branch: &str) -> PathBuf {
            let linked = self.root.join(branch.replace('/', "-"));
            run(
                &self.checkout,
                &[
                    "worktree",
                    "add",
                    "-b",
                    branch,
                    linked.to_str().unwrap(),
                    "HEAD",
                ],
            );
            linked
        }

        fn prepare(
            &self,
            source: &Path,
            branch: &str,
            action: ShippingAction,
        ) -> PreparedShipping {
            prepare_with_adapter(
                &self.data,
                ShippingRequest {
                    project_id: self.project_id.clone(),
                    _work_item_id: "test-item".into(),
                    source_path: source.to_string_lossy().into_owned(),
                    source_branch: Some(branch.into()),
                    default_branch: "trunk".into(),
                    remote_name: "azure".into(),
                    remote_identity: Some(configured_remote_identity(source)),
                    github_repository: None,
                    action,
                    pull_request_number: None,
                },
                &self.adapter,
            )
            .unwrap()
        }

        fn remove_worktree(&self, path: &Path) {
            let _ = Command::new("git")
                .arg("-C")
                .arg(&self.checkout)
                .args(["worktree", "remove", "--force", "--"])
                .arg(path)
                .output();
        }

        fn remove(self) {
            let _ = fs::remove_dir_all(self.root);
        }
    }

    fn test_adapter(root: &Path) -> TestAdapter {
        let agent = root.join("agent.sh");
        fs::write(&agent, r###"#!/bin/zsh
prompt="$(cat)"
if [[ "$prompt" == *"Return ONLY"* ]]; then
  printf '%s\n' '{"commitSubject":"Ship work","commitBody":"Prepared by the test agent.","pullRequestTitle":"Ship work","pullRequestBody":"Ship work"}'
else
  echo 'resolved' > conflict.txt
  git add conflict.txt
fi
"###).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&agent, fs::Permissions::from_mode(0o700)).unwrap();
        }
        TestAdapter { executable: agent }
    }

    fn configured_remote_identity(root: &Path) -> String {
        crate::git::configured_remote(root).unwrap().identity
    }

    fn temporary(label: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("shipyard-shipping-{label}-{suffix}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn run(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn text(root: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    fn run_script(path: &Path) -> std::process::Output {
        Command::new("/bin/zsh").arg(path).output().unwrap()
    }

    fn output_text(output: &std::process::Output) -> String {
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }

    fn ref_exists(root: &Path, reference: &str) -> bool {
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["show-ref", "--verify", "--quiet", reference])
            .status()
            .unwrap()
            .success()
    }
}
