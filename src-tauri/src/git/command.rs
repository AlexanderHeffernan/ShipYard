use std::{
    path::Path,
    process::{Command, Output},
};

pub(crate) fn text(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = output(root, args)?;
    Ok(bytes_text(&output.stdout).to_owned())
}

pub(super) fn optional_text(root: &Path, args: &[&str]) -> Option<String> {
    let output = output_allow_failure(root, args);
    output
        .status
        .success()
        .then(|| bytes_text(&output.stdout).to_owned())
}

pub(crate) fn output(root: &Path, args: &[&str]) -> Result<Output, String> {
    let output = output_allow_failure(root, args);
    if output.status.success() {
        Ok(output)
    } else {
        Err(format!(
            "Git command failed in {}: {}",
            root.display(),
            error(&output)
        ))
    }
}

pub(crate) fn output_allow_failure(root: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("LC_ALL", "C")
        .output()
        .unwrap_or_else(|error| Output {
            status: command_failure_status(),
            stdout: Vec::new(),
            stderr: format!("could not execute Git: {error}").into_bytes(),
        })
}

pub(crate) fn error(output: &Output) -> String {
    let message = bytes_text(&output.stderr).trim();
    if message.is_empty() {
        "unknown Git error".to_owned()
    } else {
        message.to_owned()
    }
}

pub(super) fn bytes_text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).unwrap_or_default()
}

#[cfg(unix)]
fn command_failure_status() -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(1 << 8)
}

#[cfg(windows)]
fn command_failure_status() -> std::process::ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(1)
}
