use crate::git;
use std::{fs, path::PathBuf};

pub(crate) struct ShippingCleanup {
    pub(super) project_id: String,
    pub(super) source: PathBuf,
    pub(super) branch: String,
    pub(super) base: String,
    pub(super) receipt: PathBuf,
}

pub(super) fn after_success(cleanup: &ShippingCleanup) -> Result<String, String> {
    let shipped_sha = fs::read_to_string(&cleanup.receipt)
        .map_err(|error| format!("could not read the shipped commit receipt: {error}"))?;
    let shipped_sha = shipped_sha.trim();
    if !(40..=64).contains(&shipped_sha.len())
        || !shipped_sha.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("the shipped commit receipt is invalid".to_owned());
    }

    let source_text = cleanup.source.to_string_lossy();
    let source = git::validate_worktree(&cleanup.project_id, &source_text)?;
    verify_checkout(cleanup, &source, shipped_sha)?;

    git::command::output(&source, &["fetch", "origin", &cleanup.base])?;
    let remote_base = format!("refs/remotes/origin/{}", cleanup.base);
    if !is_ancestor(&source, shipped_sha, &remote_base)? {
        return Err(format!(
            "origin/{} does not contain the shipped commit; the checkout was preserved",
            cleanup.base
        ));
    }

    verify_checkout(cleanup, &source, shipped_sha)?;
    let primary = git::primary_worktree_path(&source)?
        .canonicalize()
        .map_err(|error| format!("could not resolve the primary checkout: {error}"))?;
    if source == primary {
        return Ok("ShipYard · primary checkout preserved after shipping\n".to_owned());
    }

    git::command::output(
        &primary,
        &["worktree", "remove", "--", source_text.as_ref()],
    )?;
    let branch_ref = format!("refs/heads/{}", cleanup.branch);
    git::command::output(&primary, &["update-ref", "-d", &branch_ref, shipped_sha])?;
    Ok("ShipYard · removed the shipped worktree and local branch\n".to_owned())
}

fn verify_checkout(
    cleanup: &ShippingCleanup,
    source: &std::path::Path,
    shipped_sha: &str,
) -> Result<(), String> {
    let current_branch = git::command::text(source, &["branch", "--show-current"])?;
    if current_branch.trim() != cleanup.branch {
        return Err("the checkout branch moved after shipping; it was preserved".to_owned());
    }
    if !git::command::text(
        source,
        &["status", "--porcelain", "--untracked-files=normal"],
    )?
    .is_empty()
    {
        return Err("the checkout became dirty after shipping; it was preserved".to_owned());
    }
    let branch_ref = format!("refs/heads/{}", cleanup.branch);
    let current_sha = git::command::text(source, &["rev-parse", &branch_ref])?;
    if current_sha.trim() != shipped_sha {
        return Err("the shipped branch moved unexpectedly; the checkout was preserved".to_owned());
    }
    Ok(())
}

fn is_ancestor(root: &std::path::Path, ancestor: &str, descendant: &str) -> Result<bool, String> {
    let output = git::command::output_allow_failure(
        root,
        &["merge-base", "--is-ancestor", ancestor, descendant],
    );
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(git::command::error(&output)),
    }
}

#[cfg(test)]
mod tests {
    use super::{after_success, ShippingCleanup};
    use crate::git;
    use std::{fs, path::Path, path::PathBuf, process::Command, time::SystemTime};

    #[test]
    fn removes_a_clean_linked_worktree_only_after_the_remote_contains_its_commit() {
        let fixture = Fixture::new("success");
        fixture.push_feature_to_main();

        assert!(after_success(&fixture.cleanup())
            .unwrap()
            .contains("removed"));
        assert!(!fixture.linked.exists());
        assert!(!succeeds(
            &fixture.primary,
            &["show-ref", "--verify", "--quiet", "refs/heads/feature/test"]
        ));
        fixture.remove();
    }

    #[test]
    fn preserves_the_worktree_when_the_remote_does_not_contain_the_commit() {
        let fixture = Fixture::new("remote-guard");
        let error = after_success(&fixture.cleanup()).unwrap_err();

        assert!(error.contains("does not contain"));
        assert!(fixture.linked.exists());
        assert!(succeeds(
            &fixture.primary,
            &["show-ref", "--verify", "--quiet", "refs/heads/feature/test"]
        ));
        fixture.remove();
    }

    #[test]
    fn preserves_the_worktree_if_it_becomes_dirty_after_shipping() {
        let fixture = Fixture::new("dirty-guard");
        fixture.push_feature_to_main();
        fs::write(fixture.linked.join("late.txt"), "late work\n").unwrap();

        let error = after_success(&fixture.cleanup()).unwrap_err();
        assert!(error.contains("became dirty"));
        assert!(fixture.linked.exists());
        fixture.remove();
    }

    #[test]
    fn preserves_the_worktree_if_the_branch_moves_after_shipping() {
        let fixture = Fixture::new("ref-guard");
        fixture.push_feature_to_main();
        fs::write(fixture.linked.join("later.txt"), "later commit\n").unwrap();
        run(&fixture.linked, &["add", "."]);
        run(&fixture.linked, &["commit", "-m", "Later work"]);

        let error = after_success(&fixture.cleanup()).unwrap_err();
        assert!(error.contains("moved unexpectedly"));
        assert!(fixture.linked.exists());
        fixture.remove();
    }

    #[test]
    fn preserves_the_primary_checkout_without_switching_branches() {
        let fixture = Fixture::new("primary");
        fixture.push_feature_to_main();
        let mut cleanup = fixture.cleanup();
        run(
            &fixture.primary,
            &["worktree", "remove", "--", fixture.linked.to_str().unwrap()],
        );
        cleanup.source = fixture.primary.clone();
        run(&fixture.primary, &["switch", "feature/test"]);

        let message = after_success(&cleanup).unwrap();
        assert!(message.contains("primary checkout preserved"));
        assert_eq!(
            text(&fixture.primary, &["branch", "--show-current"]),
            "feature/test"
        );
        fixture.remove();
    }

    struct Fixture {
        root: PathBuf,
        primary: PathBuf,
        linked: PathBuf,
        project_id: String,
        receipt: PathBuf,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let suffix = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!("shipyard-cleanup-{label}-{suffix}"));
            let remote = root.join("remote.git");
            let primary = root.join("primary");
            let linked = root.join("linked");
            fs::create_dir_all(&root).unwrap();
            run(&root, &["init", "--bare", remote.to_str().unwrap()]);
            run(
                &root,
                &["clone", remote.to_str().unwrap(), primary.to_str().unwrap()],
            );
            run(&primary, &["switch", "-c", "main"]);
            run(&primary, &["config", "user.name", "ShipYard Test"]);
            run(&primary, &["config", "user.email", "shipyard@example.test"]);
            fs::write(primary.join("README.md"), "initial\n").unwrap();
            run(&primary, &["add", "."]);
            run(&primary, &["commit", "-m", "Initial"]);
            run(&primary, &["push", "-u", "origin", "main"]);
            run(
                &primary,
                &[
                    "worktree",
                    "add",
                    "-b",
                    "feature/test",
                    linked.to_str().unwrap(),
                    "main",
                ],
            );
            fs::write(linked.join("feature.txt"), "feature\n").unwrap();
            run(&linked, &["add", "."]);
            run(&linked, &["commit", "-m", "Feature"]);
            let sha = text(&linked, &["rev-parse", "HEAD"]);
            let receipt = root.join("receipt");
            fs::write(&receipt, format!("{sha}\n")).unwrap();
            let project_id = git::resolve(primary.to_str().unwrap())
                .unwrap()
                .1
                .to_string_lossy()
                .into_owned();
            Self {
                root,
                primary,
                linked,
                project_id,
                receipt,
            }
        }

        fn cleanup(&self) -> ShippingCleanup {
            ShippingCleanup {
                project_id: self.project_id.clone(),
                source: self.linked.clone(),
                branch: "feature/test".to_owned(),
                base: "main".to_owned(),
                receipt: self.receipt.clone(),
            }
        }

        fn push_feature_to_main(&self) {
            run(&self.linked, &["push", "origin", "HEAD:main"]);
        }

        fn remove(self) {
            let _ = Command::new("git")
                .arg("-C")
                .arg(&self.primary)
                .args(["worktree", "remove", "--force", "--"])
                .arg(&self.linked)
                .output();
            let _ = fs::remove_dir_all(self.root);
        }
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

    fn succeeds(root: &Path, args: &[&str]) -> bool {
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap()
            .status
            .success()
    }
}
