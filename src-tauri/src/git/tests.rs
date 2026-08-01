use super::{scan_project, validate_worktree, work_status::WorkStatus, worktree_paths};
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
