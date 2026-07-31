use super::{open_request::OpenRequest, store};
use std::{path::Path, process::Command};

pub fn open_checkout(base: &Path, request: OpenRequest) -> Result<(), String> {
    let application = store::stored_application(base, &request.application_id)?;
    let application_path = store::validate_application_path(Path::new(&application.app_path))?;
    let checkout_path = crate::git::validate_worktree(&request.project_id, &request.checkout_path)?;
    let mut command = build_command(&application_path, &checkout_path)?;
    let output = command
        .output()
        .map_err(|error| format!("could not start macOS Open: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let detail = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "{} could not open {}: {}",
            application.label,
            checkout_path.display(),
            detail.trim()
        ))
    }
}

#[cfg(target_os = "macos")]
pub(super) fn build_command(application: &Path, checkout: &Path) -> Result<Command, String> {
    let mut command = Command::new("/usr/bin/open");
    command.arg("-a").arg(application).arg(checkout);
    Ok(command)
}

#[cfg(not(target_os = "macos"))]
pub(super) fn build_command(_application: &Path, _checkout: &Path) -> Result<Command, String> {
    Err("opening checkouts is currently supported only on macOS".to_owned())
}
