use super::{command, remote, repository, worktree_reader};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::{Path, PathBuf}};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutPullRequestRequest {
    project_id: String,
    project_path: String,
    pull_request_number: u64,
    head_sha: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutPullRequestResult {
    pub worktree_path: String,
}

pub fn pull_request(
    app_data: &Path,
    request: CheckoutPullRequestRequest,
) -> Result<CheckoutPullRequestResult, String> {
    let root = repository::validate_worktree(&request.project_id, &request.project_path)?;
    let (_, common_dir) = repository::resolve(&request.project_path)?;
    if repository::path_string(&common_dir) != request.project_id {
        return Err("project identity no longer matches; rescan before checking out the pull request".to_owned());
    }

    let remote_name = remote::configured(&root)
        .map(|remote| remote.name)
        .unwrap_or_else(|| "origin".to_owned());
    command::output(
        &root,
        &[
            "fetch",
            "--no-tags",
            &remote_name,
            &format!("pull/{}/head", request.pull_request_number),
        ],
    )?;
    let fetched_sha = command::text(&root, &["rev-parse", "FETCH_HEAD"])?;
    let fetched_sha = fetched_sha.trim();
    if fetched_sha != request.head_sha {
        return Err("the pull request changed on GitHub; refresh it and try again".to_owned());
    }

    if let Some(existing) = worktree_reader::read(&root)?
        .into_iter()
        .find(|worktree| !worktree.bare && worktree.sha == fetched_sha)
    {
        link_node_modules(&root, &existing.path)?;
        return Ok(CheckoutPullRequestResult {
            worktree_path: repository::path_string(&existing.path),
        });
    }

    let path = managed_pull_request_checkout_path(app_data, &request.project_id, request.pull_request_number);
    if path.exists() {
        let path_text = repository::path_string(&path);
        let _ = command::output(&root, &["worktree", "remove", "--force", "--", &path_text]);
        if path.exists() {
            fs::remove_dir_all(&path)
                .map_err(|error| format!("could not clear stale PR checkout: {error}"))?;
        }
    }
    let parent = path.parent().ok_or_else(|| "invalid checkout path".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let path_text = repository::path_string(&path);
    command::output(&root, &["worktree", "add", "--detach", "--", &path_text, fetched_sha])?;
    link_node_modules(&root, &path)?;
    Ok(CheckoutPullRequestResult { worktree_path: path_text })
}

fn link_node_modules(project: &Path, checkout: &Path) -> Result<(), String> {
    let source = project.join("node_modules");
    let destination = checkout.join("node_modules");
    if !source.is_dir() || destination.exists() {
        return Ok(());
    }
    fs::create_dir_all(checkout)
        .map_err(|error| format!("could not prepare PR checkout dependencies: {error}"))?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(&source, &destination)
        .map_err(|error| format!("could not link project dependencies into PR checkout: {error}"))?;
    Ok(())
}

pub(crate) fn managed_pull_request_checkout_path(
    app_data: &Path,
    project_id: &str,
    number: u64,
) -> PathBuf {
    let digest = Sha256::digest(project_id.as_bytes());
    let project = digest.iter().take(8).map(|byte| format!("{byte:02x}")).collect::<String>();
    app_data.join("pull-request-checkouts").join(project).join(format!("pr-{number}"))
}

#[cfg(test)]
mod tests {
    use super::{pull_request, CheckoutPullRequestRequest};
    use crate::git;
    use std::{fs, path::Path, process::Command, time::{SystemTime, UNIX_EPOCH}};

    #[test]
    fn checks_out_a_pull_request_once_and_reuses_it() {
        let root = temporary("checkout");
        let remote = root.join("remote.git");
        let checkout = root.join("checkout");
        let app_data = root.join("data");
        run(&root, &["init", "--bare", remote.to_str().unwrap()]);
        run(&root, &["clone", remote.to_str().unwrap(), checkout.to_str().unwrap()]);
        run(&checkout, &["switch", "-c", "main"]);
        run(&checkout, &["config", "user.name", "Shipyard Test"]);
        run(&checkout, &["config", "user.email", "shipyard@example.test"]);
        run(&checkout, &["remote", "rename", "origin", "github"]);
        fs::write(checkout.join("README.md"), "main\n").unwrap();
        run(&checkout, &["add", "."]);
        run(&checkout, &["commit", "-m", "Main"]);
        run(&checkout, &["push", "-u", "github", "main"]);
        run(&checkout, &["switch", "-c", "feature/pr"]);
        fs::write(checkout.join("feature.txt"), "review me\n").unwrap();
        run(&checkout, &["add", "."]);
        run(&checkout, &["commit", "-m", "Feature"]);
        let head = text(&checkout, &["rev-parse", "HEAD"]);
        run(&checkout, &["push", "github", "feature/pr"]);
        run(&remote, &["update-ref", "refs/pull/7/head", &head]);
        run(&checkout, &["switch", "main"]);
        let project_id = git::resolve(checkout.to_str().unwrap())
            .unwrap()
            .1
            .to_string_lossy()
            .into_owned();
        let request = || CheckoutPullRequestRequest {
            project_id: project_id.clone(),
            project_path: checkout.to_string_lossy().into_owned(),
            pull_request_number: 7,
            head_sha: head.clone(),
        };

        let first = pull_request(&app_data, request()).unwrap();
        assert!(Path::new(&first.worktree_path).is_dir());
        assert_eq!(text(Path::new(&first.worktree_path), &["rev-parse", "HEAD"]), head);
        let second = pull_request(&app_data, request()).unwrap();
        assert_eq!(first.worktree_path, second.worktree_path);

        run(&checkout, &["worktree", "remove", "--force", &first.worktree_path]);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn links_existing_node_modules_into_a_new_checkout() {
        let root = temporary("node-modules");
        let project = root.join("project");
        let checkout = root.join("checkout");
        fs::create_dir_all(project.join("node_modules/vite")).unwrap();
        super::link_node_modules(&project, &checkout).unwrap();
        assert_eq!(fs::read_link(checkout.join("node_modules")).unwrap(), project.join("node_modules"));
        fs::remove_dir_all(root).unwrap();
    }

    fn temporary(label: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("shipyard-pr-{label}-{suffix}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn run(root: &Path, args: &[&str]) {
        let output = Command::new("git").arg("-C").arg(root).args(args).output().unwrap();
        assert!(output.status.success(), "git {}: {}", args.join(" "), String::from_utf8_lossy(&output.stderr));
    }

    fn text(root: &Path, args: &[&str]) -> String {
        let output = Command::new("git").arg("-C").arg(root).args(args).output().unwrap();
        assert!(output.status.success());
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }
}
