use super::{
    delete_work_item, inspect_work_item_deletion, read_work_item_diff, scan_project,
    validate_worktree, work_item::WorkItem, work_status::WorkStatus, worktree_paths,
    DeleteWorkItemRequest, Project, WorkItemDiffRequest,
};
use std::{fs, path::Path, path::PathBuf, process::Command, time::SystemTime};

#[test]
fn includes_uncommitted_work_without_an_existing_branch_ref() {
    let root = temporary_repository("unborn");
    run(&root, &["init", "-b", "main"]);
    fs::write(root.join("first-file.txt"), "work in progress\n").unwrap();

    let project = scan_project(root.to_str().unwrap()).unwrap();
    assert_eq!(project.work_items.len(), 1);
    assert_eq!(project.work_items[0].branch.as_deref(), Some("main"));
    assert_eq!(project.work_items[0].status, WorkStatus::Working);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn classifies_work_on_the_default_branch() {
    let root = committed_repository("default-branch");
    assert!(scan(root.to_str().unwrap()).unwrap().work_items.is_empty());

    fs::write(root.join("README.md"), "initial\ndirty\n").unwrap();
    let project = scan(root.to_str().unwrap()).unwrap();
    assert_eq!(project.work_items.len(), 1);
    assert_eq!(project.work_items[0].status, WorkStatus::Working);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn classifies_ready_and_shipped_branches() {
    let root = committed_repository("branch-status");
    run(&root, &["switch", "-c", "feature/test"]);
    fs::write(root.join("feature.txt"), "feature\n").unwrap();
    run(&root, &["add", "feature.txt"]);
    run(&root, &["commit", "-m", "Add feature"]);
    assert_branch_status(&root, "feature/test", WorkStatus::Ready);

    run(&root, &["switch", "main"]);
    run(
        &root,
        &["merge", "--no-ff", "feature/test", "-m", "Merge feature"],
    );
    assert_branch_status(&root, "feature/test", WorkStatus::Shipped);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn associates_a_dirty_linked_worktree_with_its_branch() {
    let root = committed_repository("worktree");
    let linked = root.with_extension("linked-worktree");
    run(
        &root,
        &[
            "worktree",
            "add",
            "-b",
            "feature/worktree",
            linked.to_str().unwrap(),
            "main",
        ],
    );
    fs::write(linked.join("worktree.txt"), "in progress\n").unwrap();
    let canonical = linked.canonicalize().unwrap();
    assert!(worktree_paths(&root).unwrap().contains(&canonical));
    let project = scan(root.to_str().unwrap()).unwrap();
    let item = project
        .work_items
        .iter()
        .find(|item| item.branch.as_deref() == Some("feature/worktree"))
        .unwrap();
    assert_eq!(item.worktree_path.as_deref(), canonical.to_str());
    assert_eq!(item.status, WorkStatus::Working);

    run(
        &root,
        &["worktree", "remove", "--force", linked.to_str().unwrap()],
    );
    assert!(!worktree_paths(&root).unwrap().contains(&canonical));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn fresh_clean_linked_worktree_is_working_until_explicitly_shipped() {
    let root = committed_repository("fresh-worktree");
    let linked = root.with_extension("fresh-linked");
    run(
        &root,
        &[
            "worktree",
            "add",
            "-b",
            "work/fresh",
            linked.to_str().unwrap(),
            "main",
        ],
    );
    assert_branch_status(&root, "work/fresh", WorkStatus::Working);

    run(&root, &["worktree", "remove", linked.to_str().unwrap()]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn classifies_a_primary_feature_branch_as_shipped_when_remote_main_contains_it() {
    let root = committed_repository("remote-shipped");
    let remote = root.with_extension("remote.git");
    run(&root, &["init", "--bare", remote.to_str().unwrap()]);
    run(
        &root,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    run(&root, &["push", "-u", "origin", "main"]);
    run(&root, &["switch", "-c", "feature/direct"]);
    fs::write(root.join("feature.txt"), "feature\n").unwrap();
    run(&root, &["add", "feature.txt"]);
    run(&root, &["commit", "-m", "Feature"]);
    run(&root, &["push", "origin", "HEAD:main"]);
    run(&root, &["fetch", "origin", "main"]);

    assert_branch_status(&root, "feature/direct", WorkStatus::Shipped);
    assert_eq!(text(&root, &["branch", "--show-current"]), "feature/direct");

    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn validates_only_the_exact_checkout_for_its_project() {
    let root = committed_repository("validate-worktree");
    fs::create_dir(root.join("nested")).unwrap();
    let project = scan(root.to_str().unwrap()).unwrap();
    let validated = validate_worktree(&project.id, root.to_str().unwrap()).unwrap();
    assert_eq!(validated, root.canonicalize().unwrap());
    assert!(validate_worktree(&project.id, root.join("nested").to_str().unwrap()).is_err());
    assert!(validate_worktree("another-project", root.to_str().unwrap()).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn deletes_a_dirty_linked_worktree_and_its_local_branch() {
    let root = committed_repository("delete-linked");
    let linked = root.with_extension("delete-linked-worktree");
    run(
        &root,
        &[
            "worktree",
            "add",
            "-b",
            "feature/delete-linked",
            linked.to_str().unwrap(),
            "main",
        ],
    );
    fs::write(linked.join("uncommitted.txt"), "will be deleted\n").unwrap();

    let (request, plan) = deletion_for_branch(&root, "feature/delete-linked");
    assert!(plan.removes_worktree);
    assert!(plan.deletes_branch);
    assert!(plan.has_uncommitted_changes);
    let result = delete_work_item(request, plan).unwrap();

    assert!(result.worktree_removed);
    assert!(result.branch_deleted);
    assert!(!linked.exists());
    assert!(!ref_exists(&root, "refs/heads/feature/delete-linked"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn deletes_a_detached_linked_worktree_without_deleting_a_branch() {
    let root = committed_repository("delete-detached");
    let linked = root.with_extension("delete-detached-worktree");
    run(
        &root,
        &[
            "worktree",
            "add",
            "--detach",
            linked.to_str().unwrap(),
            "main",
        ],
    );
    fs::write(linked.join("detached.txt"), "uncommitted\n").unwrap();
    let project = scan_project(root.to_str().unwrap()).unwrap();
    let item = project
        .work_items
        .iter()
        .find(|item| item.branch.is_none())
        .unwrap();
    let request = deletion_request(&project, item);
    let plan = inspect_work_item_deletion(request.clone()).unwrap();
    assert!(plan.removes_worktree);
    assert!(!plan.deletes_branch);
    assert!(plan.has_uncommitted_changes);

    let result = delete_work_item(request, plan).unwrap();
    assert!(result.worktree_removed);
    assert!(!result.branch_deleted);
    assert!(!linked.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn deletes_an_unchecked_out_local_branch_only() {
    let root = committed_repository("delete-branch-only");
    run(&root, &["branch", "feature/branch-only"]);
    let (request, plan) = deletion_for_branch(&root, "feature/branch-only");
    assert!(!plan.removes_worktree);
    assert!(plan.deletes_branch);

    let result = delete_work_item(request, plan).unwrap();
    assert!(!result.worktree_removed);
    assert!(result.branch_deleted);
    assert!(!ref_exists(&root, "refs/heads/feature/branch-only"));
    assert!(root.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn refuses_to_delete_the_default_branch() {
    let root = committed_repository("refuse-default");
    fs::write(root.join("dirty.txt"), "keep me\n").unwrap();
    let project = scan_project(root.to_str().unwrap()).unwrap();
    let item = project
        .work_items
        .iter()
        .find(|item| item.branch.as_deref() == Some("main"))
        .unwrap();

    let error = inspect_work_item_deletion(deletion_request(&project, item)).unwrap_err();
    assert!(error.contains("default branch"));
    assert!(root.join("dirty.txt").exists());
    assert!(ref_exists(&root, "refs/heads/main"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn refuses_a_dirty_primary_checkout_on_a_feature_branch() {
    let root = committed_repository("refuse-dirty-primary");
    run(&root, &["switch", "-c", "feature/dirty-primary"]);
    fs::write(root.join("dirty.txt"), "keep me\n").unwrap();
    let project = scan_project(root.to_str().unwrap()).unwrap();
    let item = project
        .work_items
        .iter()
        .find(|item| item.branch.as_deref() == Some("feature/dirty-primary"))
        .unwrap();

    let error = inspect_work_item_deletion(deletion_request(&project, item)).unwrap_err();
    assert!(error.contains("primary checkout has uncommitted changes"));
    assert_eq!(current_branch(&root), "feature/dirty-primary");
    assert!(root.join("dirty.txt").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn switches_a_clean_primary_checkout_before_deleting_its_feature_branch() {
    let root = committed_repository("delete-clean-primary");
    run(&root, &["switch", "-c", "feature/clean-primary"]);
    fs::write(root.join("feature.txt"), "feature\n").unwrap();
    run(&root, &["add", "feature.txt"]);
    run(&root, &["commit", "-m", "Feature commit"]);
    let (request, plan) = deletion_for_branch(&root, "feature/clean-primary");
    assert!(plan.switches_primary_checkout);
    assert!(!plan.removes_worktree);

    let result = delete_work_item(request, plan).unwrap();
    assert!(result.switched_primary_to_default);
    assert!(result.branch_deleted);
    assert_eq!(current_branch(&root), "main");
    assert!(root.exists());
    assert!(!ref_exists(&root, "refs/heads/feature/clean-primary"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn refuses_a_stale_or_mismatched_work_item_identity() {
    let root = committed_repository("refuse-stale");
    run(&root, &["branch", "feature/stale"]);
    let project = scan_project(root.to_str().unwrap()).unwrap();
    let item = project
        .work_items
        .iter()
        .find(|item| item.branch.as_deref() == Some("feature/stale"))
        .unwrap();
    let mut request = deletion_request(&project, item);
    request.work_item_id = format!("{}::branch::refs/heads/feature/other", project.id);

    let error = inspect_work_item_deletion(request).unwrap_err();
    assert!(error.contains("identity does not match"));
    assert!(ref_exists(&root, "refs/heads/feature/stale"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reports_uncommitted_and_unique_unpushed_work_for_confirmation() {
    let root = committed_repository("deletion-loss-summary");
    let linked = root.with_extension("deletion-loss-worktree");
    run(
        &root,
        &[
            "worktree",
            "add",
            "-b",
            "feature/loss-summary",
            linked.to_str().unwrap(),
            "main",
        ],
    );
    fs::write(linked.join("committed.txt"), "unique commit\n").unwrap();
    run(&linked, &["add", "committed.txt"]);
    run(&linked, &["commit", "-m", "Unique local commit"]);
    fs::write(linked.join("uncommitted.txt"), "unique file\n").unwrap();

    let (_, plan) = deletion_for_branch(&root, "feature/loss-summary");
    assert!(plan.has_uncommitted_changes);
    assert_eq!(plan.unpushed_commits, 1);

    run(
        &root,
        &["worktree", "remove", "--force", linked.to_str().unwrap()],
    );
    run(&root, &["branch", "-D", "feature/loss-summary"]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reads_committed_dirty_and_untracked_changes_for_a_checked_out_work_item() {
    let root = committed_repository("diff-checked-out");
    run(&root, &["switch", "-c", "feature/review"]);
    fs::write(root.join("committed.txt"), "committed change\n").unwrap();
    run(&root, &["add", "committed.txt"]);
    run(&root, &["commit", "-m", "Add committed change"]);
    fs::write(root.join("README.md"), "initial\ndirty change\n").unwrap();
    fs::write(root.join("untracked.ts"), "export const ready = true;\n").unwrap();

    let project = scan_project(root.to_str().unwrap()).unwrap();
    let item = project
        .work_items
        .iter()
        .find(|item| item.branch.as_deref() == Some("feature/review"))
        .unwrap();
    assert_eq!(item.additions, 2);
    let diff = read_work_item_diff(diff_request(&project, item)).unwrap();

    assert_eq!(diff.comparison_label, "main");
    assert!(diff.patch.contains("b/committed.txt"));
    assert!(diff.patch.contains("dirty change"));
    assert!(diff.patch.contains("b/untracked.ts"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reads_a_clean_branch_that_is_not_checked_out() {
    let root = committed_repository("diff-unchecked");
    run(&root, &["switch", "-c", "feature/unmounted"]);
    fs::write(root.join("feature.txt"), "branch-only change\n").unwrap();
    run(&root, &["add", "feature.txt"]);
    run(&root, &["commit", "-m", "Add branch-only change"]);
    run(&root, &["switch", "main"]);

    let project = scan_project(root.to_str().unwrap()).unwrap();
    let item = project
        .work_items
        .iter()
        .find(|item| item.branch.as_deref() == Some("feature/unmounted"))
        .unwrap();
    assert!(item.worktree_path.is_none());
    let diff = read_work_item_diff(diff_request(&project, item)).unwrap();

    assert!(diff.patch.contains("branch-only change"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reads_a_remote_pull_request_after_fetching_its_head() {
    let root = committed_repository("diff-remote-pr");
    let remote = root.with_extension("remote.git");
    let review = root.with_extension("review");
    run(&root, &["init", "--bare", remote.to_str().unwrap()]);
    run(
        &root,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    run(&root, &["push", "-u", "origin", "main"]);
    run(&root, &["switch", "-c", "feature/remote-review"]);
    fs::write(root.join("remote-feature.txt"), "review remotely\n").unwrap();
    run(&root, &["add", "remote-feature.txt"]);
    run(&root, &["commit", "-m", "Add remote review change"]);
    let head = text(&root, &["rev-parse", "HEAD"]);
    run(&root, &["push", "origin", "HEAD:refs/pull/7/head"]);

    fs::create_dir_all(&review).unwrap();
    run(&review, &["init"]);
    run(
        &review,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    run(
        &review,
        &[
            "fetch",
            "origin",
            "refs/heads/main:refs/remotes/origin/main",
        ],
    );
    run(
        &review,
        &["switch", "--create", "main", "refs/remotes/origin/main"],
    );
    let project = scan_project(review.to_str().unwrap()).unwrap();
    let request = WorkItemDiffRequest {
        project_path: project.path.clone(),
        project_id: project.id.clone(),
        branch: None,
        worktree_path: None,
        head_sha: head.clone(),
        default_branch: Some("main".to_owned()),
        pull_request_number: Some(7),
    };

    let diff = read_work_item_diff(request).unwrap();

    assert!(diff.patch.contains("remote-feature.txt"));
    assert_eq!(
        text(
            &review,
            &["rev-parse", "refs/shipyard/pull-requests/7/head"],
        ),
        head
    );
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(remote).unwrap();
    fs::remove_dir_all(review).unwrap();
}

fn assert_branch_status(root: &Path, branch: &str, expected: WorkStatus) {
    let project = scan(root.to_str().unwrap()).unwrap();
    let item = project
        .work_items
        .iter()
        .find(|item| item.branch.as_deref() == Some(branch))
        .unwrap();
    assert_eq!(item.status, expected);
}

fn scan(path: &str) -> Result<super::Project, String> {
    scan_project(path)
}

fn deletion_for_branch(root: &Path, branch: &str) -> (DeleteWorkItemRequest, super::DeletionPlan) {
    let project = scan_project(root.to_str().unwrap()).unwrap();
    let item = project
        .work_items
        .iter()
        .find(|item| item.branch.as_deref() == Some(branch))
        .unwrap();
    let request = deletion_request(&project, item);
    let plan = inspect_work_item_deletion(request.clone()).unwrap();
    (request, plan)
}

fn deletion_request(project: &Project, item: &WorkItem) -> DeleteWorkItemRequest {
    DeleteWorkItemRequest {
        project_path: project.path.clone(),
        project_id: project.id.clone(),
        work_item_id: item.id.clone(),
        branch: item.branch.clone(),
        worktree_path: item.worktree_path.clone(),
        head_sha: item.head_sha.clone(),
    }
}

fn diff_request(project: &Project, item: &WorkItem) -> WorkItemDiffRequest {
    WorkItemDiffRequest {
        project_path: project.path.clone(),
        project_id: project.id.clone(),
        branch: item.branch.clone(),
        worktree_path: item.worktree_path.clone(),
        head_sha: item.head_sha.clone(),
        default_branch: project.default_branch.clone(),
        pull_request_number: item
            .pull_request
            .as_ref()
            .map(|pull_request| pull_request.number),
    }
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

fn current_branch(root: &Path) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["branch", "--show-current"])
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn committed_repository(label: &str) -> PathBuf {
    let root = temporary_repository(label);
    run(&root, &["init", "-b", "main"]);
    run(&root, &["config", "user.name", "Shipyard Test"]);
    run(&root, &["config", "user.email", "shipyard@example.test"]);
    fs::write(root.join("README.md"), "initial\n").unwrap();
    run(&root, &["add", "README.md"]);
    run(&root, &["commit", "-m", "Initial commit"]);
    root
}

fn temporary_repository(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("shipyard-git-test-{label}-{suffix}"));
    fs::create_dir_all(&root).unwrap();
    root
}

fn run(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn text(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}
