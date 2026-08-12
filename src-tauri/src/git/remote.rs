use super::command;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Clone)]
pub(crate) struct ConfiguredRemote {
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) host: Option<String>,
    pub(crate) identity: String,
}

pub(crate) fn configured(root: &Path) -> Option<ConfiguredRemote> {
    let names = command::optional_text(root, &["remote"])?;
    let name = names
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .find(|name| *name == "origin")
        .or_else(|| {
            names
                .lines()
                .map(str::trim)
                .find(|name| !name.is_empty())
        })?
        .to_owned();
    let url = command::optional_text(root, &["remote", "get-url", &name])?
        .trim()
        .to_owned();
    let push_url = command::optional_text(root, &["remote", "get-url", "--push", &name])
        .map(|value| value.trim().to_owned())
        .unwrap_or_else(|| url.clone());
    Some(ConfiguredRemote {
        name,
        identity: remote_identity(&url, &push_url),
        host: remote_host(&url),
        url,
    })
}

pub(crate) fn default_branch(root: &Path, remote: &str) -> Option<String> {
    let advertised = command::optional_text(root, &["ls-remote", "--symref", remote, "HEAD"])
        .and_then(|value| advertised_default_branch(&value));
    if advertised.is_some() {
        return advertised;
    }
    let reference = format!("refs/remotes/{remote}/HEAD");
    let prefix = format!("{remote}/");
    command::optional_text(root, &["symbolic-ref", "--quiet", "--short", &reference])
        .and_then(|value| value.trim().strip_prefix(&prefix).map(str::to_owned))
}

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
    remote: &str,
    branch: &str,
    destination: &str,
    label: &str,
) -> Result<String, String> {
    fetch_ref(root, remote, &format!("refs/heads/{branch}"), destination, label)
}

pub(super) fn fetch_pull_request_head(
    root: &Path,
    number: u64,
    destination: &str,
) -> Result<String, String> {
    let remote = configured(root)
        .map(|remote| remote.name)
        .unwrap_or_else(|| "origin".to_owned());
    fetch_ref(
        root,
        &remote,
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

pub(super) fn has_remote(root: &Path, remote: &str) -> bool {
    command::optional_text(root, &["remote", "get-url", remote]).is_some()
}

#[cfg(test)]
mod tests {
    use super::{
        advertised_default_branch, base_reference, pull_request_base_reference,
        pull_request_head_reference, remote_host,
    };

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
        assert_eq!(
            advertised_default_branch("ref: refs/heads/trunk\tHEAD\nabc123\tHEAD\n"),
            Some("trunk".to_owned())
        );
        assert_eq!(advertised_default_branch("abc123\tHEAD\n"), None);
        assert_eq!(
            remote_host("https://dev.azure.com/example/project/_git/repo"),
            Some("dev.azure.com".to_owned())
        );
        assert_eq!(
            remote_host("https://user@dev.azure.com/example/project/_git/repo"),
            Some("dev.azure.com".to_owned())
        );
        assert_eq!(
            remote_host("ssh://git@ssh.dev.azure.com/v3/example/project/repo"),
            Some("ssh.dev.azure.com".to_owned())
        );
        assert_eq!(
            remote_host("git@ssh.dev.azure.com:v3/example/project/repo"),
            Some("ssh.dev.azure.com".to_owned())
        );
        assert_eq!(
            remote_host("git@github.com:owner/repo.git"),
            Some("github.com".to_owned())
        );
        assert_eq!(remote_host("/tmp/repositories/repo.git"), None);
    }
}

fn fetch_ref(
    root: &Path,
    remote: &str,
    source: &str,
    destination: &str,
    label: &str,
) -> Result<String, String> {
    let refspec = format!("+{source}:{destination}");
    command::output(
        root,
        &[
            "fetch",
            "--no-tags",
            "--no-write-fetch-head",
            remote,
            &refspec,
        ],
    )
    .map_err(|error| format!("Could not fetch {label}: {error}"))?;
    cached_commit(root, destination)
        .ok_or_else(|| format!("Could not read the fetched {label} commit"))
}

fn remote_host(url: &str) -> Option<String> {
    let value = url.trim();
    if value.is_empty() || value.starts_with('/') || value.starts_with("./") || value.starts_with("../") {
        return None;
    }
    let authority = if let Some((_, rest)) = value.split_once("://") {
        rest.split('/').next().unwrap_or_default()
    } else {
        value
    };
    let without_credentials = authority.rsplit_once('@').map_or(authority, |(_, rest)| rest);
    let host = if without_credentials.contains(':') {
        host_part(without_credentials)
    } else if value.contains("://") {
        Some(without_credentials)
    } else {
        None
    }?;
    (!host.is_empty()).then(|| host.to_owned())
}

fn advertised_default_branch(value: &str) -> Option<String> {
    value.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        if fields.next() != Some("ref:") {
            return None;
        }
        fields
            .next()?
            .strip_prefix("refs/heads/")
            .map(str::to_owned)
    })
}

fn remote_identity(url: &str, push_url: &str) -> String {
    let identity_source = format!("{url}\0{push_url}");
    Sha256::digest(identity_source.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn host_part(value: &str) -> Option<&str> {
    if let Some(value) = value.strip_prefix('[') {
        return value.split(']').next();
    }
    value.split(['/', ':']).next()
}
