use super::{
    command::{self, CancellationToken},
    remote, repository,
    worktree::Worktree,
    worktree_reader,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct CheckoutManager {
    operations: Mutex<HashMap<String, CheckoutOperation>>,
    pending_cancellations: Mutex<HashMap<String, Instant>>,
}

struct CheckoutOperation {
    project_key: String,
    cancellation: CancellationToken,
}

impl CheckoutManager {
    pub(crate) fn register(
        &self,
        operation_id: &str,
        project_id: &str,
        pull_request_number: u64,
    ) -> Result<CancellationToken, String> {
        if operation_id.trim().is_empty() {
            return Err("checkout operation is missing an identifier".to_owned());
        }
        let project_key = format!("{project_id}::pull-request::{pull_request_number}");
        let mut operations = self.operations.lock().map_err(lock_error)?;
        if operations.contains_key(operation_id) {
            return Err("checkout operation is already active".to_owned());
        }
        if operations
            .values()
            .any(|operation| operation.project_key == project_key)
        {
            return Err("this pull request is already being checked out".to_owned());
        }
        let cancellation = CancellationToken::default();
        let mut pending = self.pending_cancellations.lock().map_err(lock_error)?;
        pending.retain(|_, started| started.elapsed() < Duration::from_secs(60));
        let pre_cancelled = pending.remove(operation_id).is_some();
        drop(pending);
        if pre_cancelled {
            cancellation.cancel();
        }
        operations.insert(
            operation_id.to_owned(),
            CheckoutOperation {
                project_key,
                cancellation: cancellation.clone(),
            },
        );
        Ok(cancellation)
    }

    pub(crate) fn cancel(&self, operation_id: &str) -> Result<(), String> {
        if operation_id.trim().is_empty() {
            return Err("checkout operation is missing an identifier".to_owned());
        }
        let operations = self.operations.lock().map_err(lock_error)?;
        if let Some(operation) = operations.get(operation_id) {
            operation.cancellation.cancel();
            return Ok(());
        }
        // Keep the operations lock while recording a cancellation requested
        // before the async checkout task registered itself. Register and
        // cancel therefore share one lock order and cannot miss each other.
        self.pending_cancellations
            .lock()
            .map_err(lock_error)?
            .insert(operation_id.to_owned(), Instant::now());
        Ok(())
    }

    pub(crate) fn finish(&self, operation_id: &str) {
        if let Ok(mut operations) = self.operations.lock() {
            operations.remove(operation_id);
            // Use the same operations -> pending lock order as register and
            // cancel so an old cancellation cannot land on a reused ID.
            if let Ok(mut pending) = self.pending_cancellations.lock() {
                pending.remove(operation_id);
                pending.retain(|_, started| started.elapsed() < Duration::from_secs(60));
            }
        }
    }

    pub(crate) fn cancel_all(&self) {
        if let Ok(operations) = self.operations.lock() {
            for operation in operations.values() {
                operation.cancellation.cancel();
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutPullRequestRequest {
    pub(crate) project_id: String,
    pub(crate) project_path: String,
    pub(crate) pull_request_number: u64,
    pub(crate) head_sha: String,
    pub(crate) head_branch: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutPullRequestResult {
    pub worktree_path: String,
}

#[cfg(test)]
fn pull_request(
    app_data: &Path,
    request: CheckoutPullRequestRequest,
) -> Result<CheckoutPullRequestResult, String> {
    pull_request_with_cancellation(app_data, request, None)
}

pub fn pull_request_with_cancellation(
    app_data: &Path,
    request: CheckoutPullRequestRequest,
    cancellation: Option<&CancellationToken>,
) -> Result<CheckoutPullRequestResult, String> {
    let root = repository::validate_worktree_with_cancellation(
        &request.project_id,
        &request.project_path,
        cancellation,
    )?;
    let (_, common_dir) =
        repository::resolve_with_cancellation(&request.project_path, cancellation)?;
    if repository::path_string(&common_dir) != request.project_id {
        return Err(
            "project identity no longer matches; rescan before checking out the pull request"
                .to_owned(),
        );
    }

    ensure_not_cancelled(cancellation)?;
    let head_reference = remote::pull_request_head_reference(request.pull_request_number);
    let fetched_sha = remote::fetch_pull_request_head_with_cancellation(
        &root,
        request.pull_request_number,
        &head_reference,
        cancellation,
    )?;
    ensure_not_cancelled(cancellation)?;
    if fetched_sha != request.head_sha {
        return Err("the pull request changed on GitHub; refresh it and try again".to_owned());
    }

    let worktrees = worktree_reader::read_with_cancellation(&root, cancellation)?;
    if let Some(existing) = worktrees.iter().find(|worktree| {
        !worktree.bare
            && worktree.sha == fetched_sha
            && (worktree.detached
                || worktree.branch.as_deref()
                    == Some(&format!("refs/heads/{}", request.head_branch)))
    }) {
        ensure_not_cancelled(cancellation)?;
        let linked = link_node_modules(&root, &existing.path)?;
        if let Err(error) = associate_with_pull_request(
            &root,
            &existing.path,
            request.pull_request_number,
            cancellation,
        ) {
            return Err(with_cleanup_error(
                error,
                remove_dependency_link(&existing.path, linked),
            ));
        }
        return Ok(CheckoutPullRequestResult {
            worktree_path: repository::path_string(&existing.path),
        });
    }

    let path = managed_pull_request_checkout_path(
        app_data,
        &request.project_id,
        request.pull_request_number,
    );
    prepare_managed_path(&root, &path, &worktrees, &fetched_sha, cancellation)?;
    ensure_not_cancelled(cancellation)?;
    let parent = path
        .parent()
        .ok_or_else(|| "invalid checkout path".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let path_text = repository::path_string(&path);
    if let Err(error) = command::output_with_cancellation(
        &root,
        &[
            "worktree",
            "add",
            "--detach",
            "--",
            &path_text,
            &fetched_sha,
        ],
        cancellation,
    ) {
        return Err(with_cleanup_error(
            error,
            cleanup_partial_checkout(&root, &path),
        ));
    }
    let linked = match link_node_modules(&root, &path) {
        Ok(linked) => linked,
        Err(error) => {
            return Err(with_cleanup_error(
                error,
                cleanup_partial_checkout(&root, &path),
            ))
        }
    };
    if let Err(error) =
        associate_with_pull_request(&root, &path, request.pull_request_number, cancellation)
    {
        let dependency_cleanup = remove_dependency_link(&path, linked);
        return Err(with_cleanup_error(
            error,
            combine_cleanup_results(dependency_cleanup, cleanup_partial_checkout(&root, &path)),
        ));
    }
    // Once the worktree has been associated successfully, the checkout is
    // complete. A cancellation arriving in this final window must not turn a
    // successful materialization into an error while leaving the worktree
    // behind for the next scan.
    Ok(CheckoutPullRequestResult {
        worktree_path: path_text,
    })
}

fn associate_with_pull_request(
    root: &Path,
    checkout: &Path,
    number: u64,
    cancellation: Option<&CancellationToken>,
) -> Result<(), String> {
    command::output_with_cancellation(
        root,
        &["config", "extensions.worktreeConfig", "true"],
        cancellation,
    )?;
    let number = number.to_string();
    command::output_with_cancellation(
        checkout,
        &[
            "config",
            "--worktree",
            "shipyard.pull-request-number",
            &number,
        ],
        cancellation,
    )?;
    Ok(())
}

fn link_node_modules(project: &Path, checkout: &Path) -> Result<bool, String> {
    let source = project.join("node_modules");
    let destination = checkout.join("node_modules");
    if !source.is_dir() || destination.exists() || destination.symlink_metadata().is_ok() {
        return Ok(false);
    }
    fs::create_dir_all(checkout)
        .map_err(|error| format!("could not prepare PR checkout dependencies: {error}"))?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(&source, &destination).map_err(|error| {
        format!("could not link project dependencies into PR checkout: {error}")
    })?;
    #[cfg(unix)]
    return Ok(true);
    #[cfg(not(unix))]
    Ok(false)
}

fn prepare_managed_path(
    root: &Path,
    path: &Path,
    worktrees: &[Worktree],
    fetched_sha: &str,
    cancellation: Option<&CancellationToken>,
) -> Result<(), String> {
    let registered = worktrees
        .iter()
        .find(|worktree| same_path(&worktree.path, path));
    if let Some(worktree) = registered {
        if worktree.path.exists()
            && !can_replace_checkout(&worktree.path, fetched_sha, cancellation)
        {
            ensure_not_cancelled(cancellation)?;
            return Err(
                "an existing pull request checkout is dirty or has diverged; preserve it or remove it before retrying"
                    .to_owned(),
            );
        }
        ensure_not_cancelled(cancellation)?;
        remove_existing_worktree(root, path, cancellation)?;
    } else if path.exists() || path.symlink_metadata().is_ok() {
        remove_unregistered_path(path)?;
    }
    Ok(())
}

fn remove_existing_worktree(
    root: &Path,
    path: &Path,
    cancellation: Option<&CancellationToken>,
) -> Result<(), String> {
    let path_text = repository::path_string(path);
    command::output_with_cancellation(
        root,
        &["worktree", "remove", "--force", "--", &path_text],
        cancellation,
    )
    .map_err(|error| format!("could not remove the previous PR checkout: {error}"))?;
    if path.exists() || path.symlink_metadata().is_ok() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("could not clear the previous PR checkout: {error}"))?;
    }
    Ok(())
}

fn can_replace_checkout(
    checkout: &Path,
    fetched_sha: &str,
    cancellation: Option<&CancellationToken>,
) -> bool {
    let status = command::output_allow_failure_with_cancellation(
        checkout,
        &["status", "--porcelain", "--untracked-files=all"],
        cancellation,
    );
    status.status.success()
        && status.stdout.is_empty()
        && command::output_allow_failure_with_cancellation(
            checkout,
            &["merge-base", "--is-ancestor", "HEAD", fetched_sha],
            cancellation,
        )
        .status
        .success()
}

fn cleanup_partial_checkout(root: &Path, path: &Path) -> Result<(), String> {
    let path_text = repository::path_string(path);
    let remove_error =
        command::output(root, &["worktree", "remove", "--force", "--", &path_text]).err();
    if path.exists() || path.symlink_metadata().is_ok() {
        if remove_error.is_some() {
            remove_unregistered_path(path)
                .map_err(|error| format!("could not clean up the partial PR checkout: {error}"))?;
        } else {
            fs::remove_dir_all(path)
                .map_err(|error| format!("could not clean up the partial PR checkout: {error}"))?;
        }
    }
    if let Some(error) = remove_error {
        let still_registered = worktree_reader::read(root)
            .map_err(|read_error| {
                format!(
                    "could not verify partial PR checkout cleanup after Git failed: {error}; {read_error}"
                )
            })?
            .iter()
            .any(|worktree| same_path(&worktree.path, path));
        if still_registered {
            return Err(format!("could not remove the partial PR checkout: {error}"));
        }
    }
    Ok(())
}

fn remove_unregistered_path(path: &Path) -> Result<(), String> {
    let metadata = path
        .symlink_metadata()
        .map_err(|error| format!("could not inspect stale PR checkout: {error}"))?;
    if metadata.file_type().is_symlink() {
        return fs::remove_file(path)
            .map_err(|error| format!("could not remove stale PR checkout link: {error}"));
    }
    if !metadata.is_dir() {
        return Err(format!(
            "stale PR checkout path {} is not a directory; remove it manually before retrying",
            path.display()
        ));
    }
    let mut entries = fs::read_dir(path)
        .map_err(|error| format!("could not inspect stale PR checkout: {error}"))?;
    if entries.next().is_some() {
        return Err(format!(
            "stale PR checkout path {} contains files; remove it manually before retrying",
            path.display()
        ));
    }
    fs::remove_dir(path).map_err(|error| format!("could not remove stale PR checkout: {error}"))
}

fn combine_cleanup_results(
    first: Result<(), String>,
    second: Result<(), String>,
) -> Result<(), String> {
    match (first, second) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(first), Ok(())) | (Ok(()), Err(first)) => Err(first),
        (Err(first), Err(second)) => Err(format!("{first}; {second}")),
    }
}

fn with_cleanup_error(error: String, cleanup: Result<(), String>) -> String {
    match cleanup {
        Ok(()) => error,
        Err(cleanup_error) => format!("{error}; cleanup also failed: {cleanup_error}"),
    }
}

fn remove_dependency_link(checkout: &Path, linked: bool) -> Result<(), String> {
    if !linked {
        return Ok(());
    }
    let destination = checkout.join("node_modules");
    fs::remove_file(&destination).map_err(|error| {
        format!(
            "could not remove the partial PR dependency link {}: {error}",
            destination.display()
        )
    })?;
    Ok(())
}

fn same_path(left: &Path, right: &Path) -> bool {
    left == right
        || left
            .canonicalize()
            .ok()
            .zip(right.canonicalize().ok())
            .is_some_and(|(left, right)| left == right)
}

fn ensure_not_cancelled(cancellation: Option<&CancellationToken>) -> Result<(), String> {
    if cancellation.is_some_and(CancellationToken::is_cancelled) {
        Err("checkout cancelled".to_owned())
    } else {
        Ok(())
    }
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> String {
    error.to_string()
}

pub(crate) fn managed_pull_request_checkout_path(
    app_data: &Path,
    project_id: &str,
    number: u64,
) -> PathBuf {
    let digest = Sha256::digest(project_id.as_bytes());
    let project = digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    app_data
        .join("pull-request-checkouts")
        .join(project)
        .join(format!("pr-{number}"))
}

#[cfg(test)]
mod tests {
    use super::{
        managed_pull_request_checkout_path, pull_request, CheckoutManager,
        CheckoutPullRequestRequest,
    };
    use crate::git;
    use std::{
        fs,
        path::Path,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn serializes_checkout_operations_and_releases_them_after_completion() {
        let manager = CheckoutManager::default();
        let token = manager.register("first", "/repo/.git", 7).unwrap();
        assert!(manager.register("second", "/repo/.git", 7).is_err());

        manager.cancel("first").unwrap();
        assert!(token.is_cancelled());
        manager.finish("first");
        assert!(manager.register("second", "/repo/.git", 7).is_ok());
    }

    #[test]
    fn cancels_all_active_checkout_operations_when_the_app_exits() {
        let manager = CheckoutManager::default();
        let first = manager.register("first", "/repo/.git", 7).unwrap();
        let second = manager.register("second", "/repo/.git", 8).unwrap();

        manager.cancel_all();

        assert!(first.is_cancelled());
        assert!(second.is_cancelled());
        manager.finish("first");
        manager.finish("second");
    }

    #[test]
    fn records_a_cancel_requested_before_the_checkout_task_registers() {
        let manager = CheckoutManager::default();
        manager.cancel("queued").unwrap();
        let token = manager.register("queued", "/repo/.git", 8).unwrap();
        assert!(token.is_cancelled());
        manager.finish("queued");
    }

    #[test]
    fn refuses_to_delete_nonempty_unregistered_checkout_paths() {
        let root = temporary("unsafe-stale-path");
        let path = root.join("pr-12");
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("keep-me.txt"), "local data\n").unwrap();

        let error = super::prepare_managed_path(&root, &path, &[], "deadbeef", None)
            .err()
            .unwrap();

        assert!(error.contains("contains files"));
        assert!(path.join("keep-me.txt").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn checks_out_a_pull_request_once_and_reuses_it() {
        let root = temporary("checkout");
        let remote = root.join("remote.git");
        let checkout = root.join("checkout");
        let app_data = root.join("data");
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
        run(&checkout, &["config", "user.name", "Shipyard Test"]);
        run(
            &checkout,
            &["config", "user.email", "shipyard@example.test"],
        );
        fs::write(checkout.join("README.md"), "main\n").unwrap();
        run(&checkout, &["add", "."]);
        run(&checkout, &["commit", "-m", "Main"]);
        run(&checkout, &["push", "-u", "origin", "main"]);
        run(&checkout, &["switch", "-c", "feature/pr"]);
        fs::write(checkout.join("feature.txt"), "review me\n").unwrap();
        run(&checkout, &["add", "."]);
        run(&checkout, &["commit", "-m", "Feature"]);
        let head = text(&checkout, &["rev-parse", "HEAD"]);
        run(&checkout, &["push", "origin", "feature/pr"]);
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
            head_branch: "feature/pr".into(),
        };

        let mut stale_request = request();
        stale_request.head_sha = "0".repeat(40);
        let stale_error = pull_request(&app_data, stale_request).err().unwrap();
        assert_eq!(
            stale_error,
            "the pull request changed on GitHub; refresh it and try again"
        );
        assert!(!managed_pull_request_checkout_path(&app_data, &project_id, 7).exists());

        let first = pull_request(&app_data, request()).unwrap();
        assert!(Path::new(&first.worktree_path).is_dir());
        assert_eq!(
            text(Path::new(&first.worktree_path), &["rev-parse", "HEAD"]),
            head
        );
        assert_eq!(
            text(
                &checkout,
                &["rev-parse", "refs/shipyard/pull-requests/7/head"],
            ),
            head
        );
        assert_eq!(
            text(&checkout, &["rev-parse", "refs/heads/feature/pr"]),
            head
        );
        let second = pull_request(&app_data, request()).unwrap();
        assert_eq!(first.worktree_path, second.worktree_path);

        let scanned = git::scan_project(checkout.to_str().unwrap()).unwrap();
        let item = scanned
            .work_items
            .iter()
            .find(|item| item.worktree_path.as_deref() == Some(first.worktree_path.as_str()))
            .unwrap();
        assert!(item.managed_checkout);
        assert_eq!(item.pull_request_number, Some(7));
        assert!(git::worktree_paths(&checkout)
            .unwrap()
            .contains(&Path::new(&first.worktree_path).canonicalize().unwrap()));

        fs::write(
            Path::new(&first.worktree_path).join("local.txt"),
            "local work\n",
        )
        .unwrap();
        run(Path::new(&first.worktree_path), &["add", "local.txt"]);
        run(
            Path::new(&first.worktree_path),
            &["commit", "-m", "Local work"],
        );
        let committed = git::scan_project(checkout.to_str().unwrap()).unwrap();
        let item = committed
            .work_items
            .iter()
            .find(|item| item.worktree_path.as_deref() == Some(first.worktree_path.as_str()))
            .unwrap();
        assert_eq!(item.pull_request_number, Some(7));
        let preserve_error = pull_request(&app_data, request()).err().unwrap();
        assert!(preserve_error.contains("dirty or has diverged"));
        assert!(Path::new(&first.worktree_path).is_dir());

        run(
            &checkout,
            &["worktree", "remove", "--force", &first.worktree_path],
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reuses_a_matching_same_repository_branch_worktree_without_creating_a_managed_one() {
        let root = temporary("same-repository-worktree");
        let remote = root.join("remote.git");
        let checkout = root.join("checkout");
        let local_worktree = root.join("local-worktree");
        let app_data = root.join("data");
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
        configure_user(&checkout);
        fs::write(checkout.join("README.md"), "main\n").unwrap();
        run(&checkout, &["add", "."]);
        run(&checkout, &["commit", "-m", "Main"]);
        run(&checkout, &["push", "-u", "origin", "main"]);
        run(&checkout, &["switch", "-c", "feature/same-repo"]);
        fs::write(checkout.join("feature.txt"), "same repository\n").unwrap();
        run(&checkout, &["add", "."]);
        run(&checkout, &["commit", "-m", "Feature"]);
        let head = text(&checkout, &["rev-parse", "HEAD"]);
        run(&checkout, &["push", "origin", "HEAD:refs/pull/9/head"]);
        run(&checkout, &["switch", "main"]);
        run(
            &checkout,
            &[
                "worktree",
                "add",
                local_worktree.to_str().unwrap(),
                "feature/same-repo",
            ],
        );

        let project_id = git::resolve(checkout.to_str().unwrap())
            .unwrap()
            .1
            .to_string_lossy()
            .into_owned();
        let result = pull_request(
            &app_data,
            CheckoutPullRequestRequest {
                project_id: project_id.clone(),
                project_path: checkout.to_string_lossy().into_owned(),
                pull_request_number: 9,
                head_sha: head,
                head_branch: "feature/same-repo".into(),
            },
        )
        .unwrap();

        assert_eq!(
            Path::new(&result.worktree_path).canonicalize().unwrap(),
            local_worktree.canonicalize().unwrap()
        );
        assert!(!managed_pull_request_checkout_path(&app_data, &project_id, 9).exists());
        assert_eq!(
            text(
                &local_worktree,
                &[
                    "config",
                    "--worktree",
                    "--get",
                    "shipyard.pull-request-number"
                ]
            ),
            "9"
        );
        run(
            &checkout,
            &[
                "worktree",
                "remove",
                "--force",
                local_worktree.to_str().unwrap(),
            ],
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn checks_out_a_fork_pull_request_from_the_base_repository_ref() {
        let root = temporary("fork-checkout");
        let base_remote = root.join("base.git");
        let fork_remote = root.join("fork.git");
        let checkout = root.join("checkout");
        let fork_checkout = root.join("fork-checkout");
        let app_data = root.join("data");
        run(&root, &["init", "--bare", base_remote.to_str().unwrap()]);
        run(&root, &["init", "--bare", fork_remote.to_str().unwrap()]);
        run(
            &root,
            &[
                "clone",
                base_remote.to_str().unwrap(),
                checkout.to_str().unwrap(),
            ],
        );
        run(&checkout, &["switch", "-c", "main"]);
        configure_user(&checkout);
        fs::write(checkout.join("README.md"), "main\n").unwrap();
        run(&checkout, &["add", "."]);
        run(&checkout, &["commit", "-m", "Main"]);
        run(&checkout, &["push", "-u", "origin", "main"]);

        run(
            &root,
            &[
                "clone",
                base_remote.to_str().unwrap(),
                fork_checkout.to_str().unwrap(),
            ],
        );
        run(&fork_checkout, &["switch", "-c", "contributor/feature"]);
        configure_user(&fork_checkout);
        run(
            &fork_checkout,
            &["remote", "set-url", "origin", fork_remote.to_str().unwrap()],
        );
        fs::write(fork_checkout.join("fork-feature.txt"), "from a fork\n").unwrap();
        run(&fork_checkout, &["add", "."]);
        run(&fork_checkout, &["commit", "-m", "Fork feature"]);
        let head = text(&fork_checkout, &["rev-parse", "HEAD"]);
        run(
            &fork_checkout,
            &["push", "-u", "origin", "contributor/feature"],
        );
        run(
            &fork_checkout,
            &[
                "push",
                base_remote.to_str().unwrap(),
                "HEAD:refs/pull/8/head",
            ],
        );
        run(&checkout, &["switch", "main"]);

        let project_id = git::resolve(checkout.to_str().unwrap())
            .unwrap()
            .1
            .to_string_lossy()
            .into_owned();
        let result = pull_request(
            &app_data,
            CheckoutPullRequestRequest {
                project_id,
                project_path: checkout.to_string_lossy().into_owned(),
                pull_request_number: 8,
                head_sha: head.clone(),
                head_branch: "contributor/feature".into(),
            },
        )
        .unwrap();

        assert_eq!(
            text(Path::new(&result.worktree_path), &["rev-parse", "HEAD"]),
            head
        );
        assert_eq!(
            fs::read_to_string(Path::new(&result.worktree_path).join("fork-feature.txt")).unwrap(),
            "from a fork\n"
        );
        assert!(!ref_exists(&checkout, "refs/heads/contributor/feature"));
        run(
            &checkout,
            &["worktree", "remove", "--force", &result.worktree_path],
        );
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
        assert_eq!(
            fs::read_link(checkout.join("node_modules")).unwrap(),
            project.join("node_modules")
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn temporary(label: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("shipyard-pr-{label}-{suffix}"));
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

    fn configure_user(root: &Path) {
        run(root, &["config", "user.name", "Shipyard Test"]);
        run(root, &["config", "user.email", "shipyard@example.test"]);
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
