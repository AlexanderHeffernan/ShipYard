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
    github_repository: String,
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
    let branch = request
        .source_branch
        .as_deref()
        .ok_or_else(|| "Create a branch before shipping this work".to_owned())?;
    if branch == request.default_branch {
        return Err("Work on the default branch cannot be shipped as a pull request".to_owned());
    }
    if request.github_repository.split('/').count() != 2 {
        return Err("This project is not connected to a GitHub repository".to_owned());
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
    let script = script(
        &request,
        adapter,
        &operation_dir,
        &metadata_prompt,
        &conflict_prompt,
        &resolution_path,
        &shipped_commit_path,
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
            branch: branch.to_owned(),
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
        "Inspect the work in this repository relative to origin/{base}. Return ONLY one JSON object with exactly these string fields: commitSubject, commitBody, pullRequestTitle, pullRequestBody. Describe the intent and user-visible effect accurately. Do not edit files, run destructive commands, commit, or push. Do not use Markdown fences. The branch is {branch}.",
        base = request.default_branch,
        branch = request.source_branch.as_deref().unwrap_or_default(),
    )
}

fn conflict_prompt_text(request: &ShippingRequest) -> String {
    format!(
        "Resolve the current Git merge conflicts while bringing branch {branch} up to date with its remote work and origin/{base}. Inspect both sides and preserve their intended behavior. Edit only what is needed to resolve the merge. Run focused checks when useful. Do not commit, push, rebase, switch branches, or abort the merge. Finish only when every conflict is resolved and staged or ready to stage.",
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
) -> Result<String, String> {
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
repository={repository}
resolution={resolution}

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
  if [[ "$(git -C "$source" branch --show-current)" == "$branch" ]] &&
     [[ "$(git -C "$source" rev-parse HEAD)" == "$source_sha" ]] &&
     [[ -z "$(git -C "$source" status --porcelain --untracked-files=normal)" ]]; then
    git -C "$source" merge --ff-only "$RESOLVED_SHA"
  fi
  git -C "$source" worktree remove "$resolution"
  echo "Shipyard · remote work integrated"
}}
"#,
        source = shell(source),
        branch = shell_text(branch),
        base = shell_text(&request.default_branch),
        repository = shell_text(&request.github_repository),
        resolution = shell(resolution_path),
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
git -C "$source" fetch origin {fetch_targets}
source_sha="$(git -C "$source" rev-parse HEAD)"
{synchronize_pull_request}
if ! git -C "$source" merge-tree --write-tree "$source_sha" "origin/$base" >/dev/null; then
  integrate_target "$source_sha" "origin/$base" "branch conflicts with origin/$base"
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
                    r#"remote_sha="$(git -C "$source" rev-parse "origin/$branch")"
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
git -C "$source" fetch origin "$base" "$branch"
if [[ "$(git -C "$source" branch --show-current)" == "$branch" ]] &&
   [[ -n "$(git -C "$source" status --porcelain --untracked-files=normal)" ]]; then
  echo "Shipyard · local changes are not in the pull request; update it before merging" >&2
  exit 1
fi
local_sha="$(git -C "$source" rev-parse "refs/heads/$branch")"
source_sha="$(git -C "$source" rev-parse "origin/$branch")"
if [[ "$local_sha" != "$source_sha" ]]; then
  echo "Shipyard · local commits are not synchronized with the pull request; update it before merging" >&2
  exit 1
fi
if ! git -C "$source" merge-tree --write-tree "$source_sha" "origin/$base" >/dev/null; then
  integrate_target "$source_sha" "origin/$base" "branch conflicts with origin/$base"
  git -C "$source" push origin "$RESOLVED_SHA:refs/heads/$branch"
fi
echo "Shipyard · merging pull request #{number}"
gh pr merge {number} --repo "$repository" --squash --delete-branch
git -C "$source" fetch --prune origin
echo "Shipyard · pull request merged"
"#,
                number = number,
            )
        }
    };

    let finish = match request.action {
        ShippingAction::CreatePullRequest => format!(
            r#"
git -C "$source" push -u origin "HEAD:$branch"
echo "Shipyard · creating pull request"
gh pr create --repo "$repository" --base "$base" --head "$branch" --title "$(cat {title})" --body-file {body}
echo "Shipyard · pull request created"
"#,
            title = shell(&pr_title),
            body = shell(&pr_body),
        ),
        ShippingAction::UpdatePullRequest => r#"
git -C "$source" push origin "HEAD:$branch"
echo "Shipyard · pull request updated"
"#
        .to_owned(),
        ShippingAction::DirectToMain => r#"
source_sha="$(git -C "$source" rev-parse HEAD)"
if ! git -C "$source" merge-base --is-ancestor "origin/$base" "$source_sha"; then
  integrate_target "$source_sha" "origin/$base" "branch needs the latest origin/$base"
  source_sha="$RESOLVED_SHA"
fi
[[ "$(git -C "$source" branch --show-current)" == "$branch" ]]
[[ "$(git -C "$source" rev-parse HEAD)" == "$source_sha" ]]
[[ -z "$(git -C "$source" status --porcelain --untracked-files=normal)" ]]
echo "ShipYard · pushing the resolved commit directly to $base"
git -C "$source" push origin "${source_sha}:refs/heads/$base"
git -C "$source" fetch origin "$base"
git -C "$source" merge-base --is-ancestor "$source_sha" "origin/$base"
printf '%s\n' "$source_sha" > {shipped_commit}
echo "ShipYard · shipped directly to $base"
"#
        .replace("{shipped_commit}", &shell(shipped_commit_path)),
        ShippingAction::MergePullRequest => String::new(),
    };

    Ok(format!(
        "#!/bin/zsh\nset -euo pipefail\n\n{common}\n{action}\n{finish}"
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
    use super::{prepare_with_adapter, ShippingAction, ShippingRequest};
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
                github_repository: "owner/repo".into(),
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
                github_repository: "owner/repo".into(),
                action: ShippingAction::MergePullRequest,
                pull_request_number: Some(1),
            },
            &adapter,
        )
        .unwrap();
        let blocked_output = Command::new("/bin/zsh")
            .arg(blocked_merge.script_path)
            .output()
            .unwrap();
        assert!(!blocked_output.status.success());
        assert!(String::from_utf8_lossy(&blocked_output.stderr)
            .contains("local changes are not in the pull request"));

        let prepared = prepare_with_adapter(
            &data,
            ShippingRequest {
                project_id,
                _work_item_id: "test-item".into(),
                source_path: checkout.to_string_lossy().into_owned(),
                source_branch: Some("feature/update".into()),
                default_branch: "main".into(),
                github_repository: "owner/repo".into(),
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
}
