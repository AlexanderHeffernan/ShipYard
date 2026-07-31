use super::{
    active_states, context::ShipContext, load_settings, record_conflict, record_success,
    script_path,
};
use crate::git;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

#[test]
fn provides_a_conservative_default_script() {
    let data = temporary_directory("settings");
    let settings = load_settings(&data, "project").unwrap();
    assert_eq!(settings.scripts.len(), 1);
    assert!(settings.scripts[0].content.contains("SHIPYARD_SOURCE_SHA"));
    assert!(settings.scripts[0]
        .content
        .contains("SHIPYARD_TARGET_WORKTREE_PATH"));
    fs::remove_dir_all(data).unwrap();
}

#[test]
fn default_script_merges_the_validated_source_commit() {
    let (root, source, data, context) = repository_fixture("default-success");
    let settings = load_settings(&data, &context.project_id).unwrap();
    let script = script_path(
        &data,
        &context.project_id,
        settings.default_script_id.as_deref().unwrap(),
    )
    .unwrap();
    let status = Command::new("/bin/zsh")
        .arg(script)
        .env("SHIPYARD_WORKTREE_PATH", &context.source_path)
        .env("SHIPYARD_SOURCE_SHA", &context.source_sha)
        .env("SHIPYARD_DEFAULT_BRANCH", &context.default_branch)
        .env("SHIPYARD_TARGET_WORKTREE_PATH", &context.target_path)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(git::is_merged(root.to_str().unwrap(), &context.source_sha, "main").unwrap());
    cleanup(root, source, data);
}

#[test]
fn conflict_persists_until_git_proves_the_source_was_merged() {
    let (root, source, data, context) = repository_fixture("conflict");
    record_conflict(&data, &context).unwrap();
    assert_eq!(
        active_states(&data, &context.project_id)
            .unwrap()
            .conflicts
            .len(),
        1
    );
    run(&root, &["merge", "--no-ff", "work/ship", "-m", "Ship work"]);
    assert!(active_states(&data, &context.project_id)
        .unwrap()
        .conflicts
        .is_empty());
    cleanup(root, source, data);
}

#[test]
fn successful_ship_state_survives_ordinary_rescans() {
    let (root, source, data, context) = repository_fixture("success");
    record_success(&data, &context).unwrap();
    let states = active_states(&data, &context.project_id).unwrap();
    assert_eq!(
        states.shipped,
        vec![(context.work_item_id.clone(), context.source_sha.clone())]
    );
    assert_eq!(
        active_states(&data, &context.project_id)
            .unwrap()
            .shipped
            .len(),
        1
    );
    cleanup(root, source, data);
}

#[test]
fn ship_safety_blocks_dirty_source_and_target_worktrees() {
    let (root, source, data, context) = repository_fixture("safety");
    fs::write(source.join("dirty.txt"), "dirty\n").unwrap();
    assert!(validated(&context).unwrap_err().contains("source worktree"));
    fs::remove_file(source.join("dirty.txt")).unwrap();
    fs::write(root.join("dirty.txt"), "dirty\n").unwrap();
    assert!(validated(&context)
        .unwrap_err()
        .contains("default branch worktree"));
    cleanup(root, source, data);
}

#[test]
fn ship_safety_accepts_a_clean_detached_source() {
    let (root, source, data, mut context) = repository_fixture("detached");
    run(&source, &["switch", "--detach"]);
    context.source_branch = None;
    assert_eq!(validated(&context).unwrap(), root.canonicalize().unwrap());
    cleanup(root, source, data);
}

fn validated(context: &ShipContext) -> Result<PathBuf, String> {
    git::validate_ship(
        &context.project_id,
        context.source_path.to_str().unwrap(),
        context.source_branch.as_deref(),
        &context.source_sha,
        &context.default_branch,
    )
}

fn repository_fixture(label: &str) -> (PathBuf, PathBuf, PathBuf, ShipContext) {
    let root = temporary_directory(label);
    run(&root, &["init", "-b", "main"]);
    run(&root, &["config", "user.name", "Shipyard Test"]);
    run(&root, &["config", "user.email", "shipyard@example.test"]);
    fs::write(root.join("base.txt"), "base\n").unwrap();
    run(&root, &["add", "."]);
    run(&root, &["commit", "-m", "Base"]);
    let source = root.with_extension("source");
    run(
        &root,
        &[
            "worktree",
            "add",
            "-b",
            "work/ship",
            source.to_str().unwrap(),
            "main",
        ],
    );
    fs::write(source.join("work.txt"), "work\n").unwrap();
    run(&source, &["add", "."]);
    run(&source, &["commit", "-m", "Work"]);
    let project_id = git::project_id(root.to_str().unwrap()).unwrap();
    let source_sha = text(&source, &["rev-parse", "HEAD"]);
    let data = temporary_directory(&format!("{label}-data"));
    let context = ShipContext {
        project_id: project_id.clone(),
        work_item_id: format!("{project_id}::branch::refs/heads/work/ship"),
        source_path: source.clone(),
        source_branch: Some("work/ship".to_owned()),
        source_sha,
        default_branch: "main".to_owned(),
        target_path: root.clone(),
    };
    (root, source, data, context)
}

fn cleanup(root: PathBuf, source: PathBuf, data: PathBuf) {
    run(
        &root,
        &["worktree", "remove", "--force", source.to_str().unwrap()],
    );
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(data).unwrap();
}

fn temporary_directory(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("shipyard-ship-{label}-{suffix}"));
    fs::create_dir_all(&path).unwrap();
    path
}

fn run(root: &Path, args: &[&str]) {
    let output = command(root, args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn text(root: &Path, args: &[&str]) -> String {
    String::from_utf8(command(root, args).stdout)
        .unwrap()
        .trim()
        .to_owned()
}
fn command(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap()
}
