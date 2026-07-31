use super::{scan_project, work_status::WorkStatus, worktree_paths};
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
    assert!(scan_project(root.to_str().unwrap())
        .unwrap()
        .work_items
        .is_empty());

    fs::write(root.join("README.md"), "initial\ndirty\n").unwrap();
    let project = scan_project(root.to_str().unwrap()).unwrap();
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
    let project = scan_project(root.to_str().unwrap()).unwrap();
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

fn assert_branch_status(root: &Path, branch: &str, expected: WorkStatus) {
    let project = scan_project(root.to_str().unwrap()).unwrap();
    let item = project
        .work_items
        .iter()
        .find(|item| item.branch.as_deref() == Some(branch))
        .unwrap();
    assert_eq!(item.status, expected);
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
