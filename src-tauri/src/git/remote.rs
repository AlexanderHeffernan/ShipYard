use super::command::{self, CancellationToken};
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
    fetch_pull_request_head_with_cancellation(root, number, destination, None)
}

pub(super) fn fetch_pull_request_head_with_cancellation(
    root: &Path,
    number: u64,
    destination: &str,
    cancellation: Option<&CancellationToken>,
) -> Result<String, String> {
    fetch_ref_with_cancellation(
        root,
        &format!("refs/pull/{number}/head"),
        destination,
        &format!("pull request #{number}"),
        cancellation,
    )
}

pub(super) fn cached_commit(root: &Path, reference: &str) -> Option<String> {
    cached_commit_with_cancellation(root, reference, None)
}

pub(super) fn cached_commit_with_cancellation(
    root: &Path,
    reference: &str,
    cancellation: Option<&CancellationToken>,
) -> Option<String> {
    command::optional_text_with_cancellation(
        root,
        &["rev-parse", "--verify", &format!("{reference}^{{commit}}")],
        cancellation,
    )
    .map(|value| value.trim().to_owned())
}

pub(super) fn has_origin(root: &Path) -> bool {
    command::optional_text(root, &["remote", "get-url", "origin"]).is_some()
}

fn fetch_ref(root: &Path, source: &str, destination: &str, label: &str) -> Result<String, String> {
    fetch_ref_with_cancellation(root, source, destination, label, None)
}

fn fetch_ref_with_cancellation(
    root: &Path,
    source: &str,
    destination: &str,
    label: &str,
    cancellation: Option<&CancellationToken>,
) -> Result<String, String> {
    let refspec = format!("+{source}:{destination}");
    command::output_with_cancellation(
        root,
        &[
            "fetch",
            "--no-tags",
            "--no-write-fetch-head",
            "origin",
            &refspec,
        ],
        cancellation,
    )
    .map_err(|error| format!("Could not fetch {label}: {error}"))?;
    cached_commit_with_cancellation(root, destination, cancellation)
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
