use super::command;
use std::path::Path;

pub(super) fn base_reference(branch: &str) -> String {
    format!("refs/shipyard/bases/{branch}")
}

pub(super) fn pull_request_base_reference(number: u64) -> String {
    format!("refs/shipyard/pull-requests/{number}/base")
}

pub(super) fn pull_request_head_reference(number: u64) -> String {
    format!("refs/shipyard/pull-requests/{number}/head")
}

pub(super) fn fetch_branch(
    root: &Path,
    branch: &str,
    destination: &str,
    label: &str,
) -> Result<String, String> {
    fetch_ref(root, &format!("refs/heads/{branch}"), destination, label)
}

pub(super) fn fetch_pull_request_head(
    root: &Path,
    number: u64,
    destination: &str,
) -> Result<String, String> {
    fetch_ref(
        root,
        &format!("refs/pull/{number}/head"),
        destination,
        &format!("pull request #{number}"),
    )
}

pub(super) fn cached_commit(root: &Path, reference: &str) -> Option<String> {
    command::optional_text(
        root,
        &["rev-parse", "--verify", &format!("{reference}^{{commit}}")],
    )
    .map(|value| value.trim().to_owned())
}

pub(super) fn has_origin(root: &Path) -> bool {
    command::optional_text(root, &["remote", "get-url", "origin"]).is_some()
}

fn fetch_ref(root: &Path, source: &str, destination: &str, label: &str) -> Result<String, String> {
    let refspec = format!("+{source}:{destination}");
    command::output(
        root,
        &[
            "fetch",
            "--no-tags",
            "--no-write-fetch-head",
            "origin",
            &refspec,
        ],
    )
    .map_err(|error| format!("Could not fetch {label}: {error}"))?;
    cached_commit(root, destination)
        .ok_or_else(|| format!("Could not read the fetched {label} commit"))
}

#[cfg(test)]
mod tests {
    use super::{base_reference, pull_request_base_reference, pull_request_head_reference};

    #[test]
    fn keeps_shipyard_references_separate_from_user_branches() {
        assert_eq!(base_reference("main"), "refs/shipyard/bases/main");
        assert_eq!(
            pull_request_base_reference(7),
            "refs/shipyard/pull-requests/7/base"
        );
        assert_eq!(
            pull_request_head_reference(7),
            "refs/shipyard/pull-requests/7/head"
        );
    }
}
