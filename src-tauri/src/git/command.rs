use std::{
    io::Read,
    path::Path,
    process::{Child, Command, Output, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(90);
const OUTPUT_DRAIN_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Clone, Default)]
pub(crate) struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub(crate) fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

pub(crate) fn text(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = output(root, args)?;
    Ok(bytes_text(&output.stdout).to_owned())
}

pub(super) fn optional_text(root: &Path, args: &[&str]) -> Option<String> {
    optional_text_with_cancellation(root, args, None)
}

pub(super) fn optional_text_with_cancellation(
    root: &Path,
    args: &[&str],
    cancellation: Option<&CancellationToken>,
) -> Option<String> {
    let output = output_allow_failure_with_cancellation(root, args, cancellation);
    output
        .status
        .success()
        .then(|| bytes_text(&output.stdout).to_owned())
}

pub(crate) fn output(root: &Path, args: &[&str]) -> Result<Output, String> {
    output_with_cancellation(root, args, None)
}

pub(crate) fn output_with_cancellation(
    root: &Path,
    args: &[&str],
    cancellation: Option<&CancellationToken>,
) -> Result<Output, String> {
    let output = output_allow_failure_with_cancellation(root, args, cancellation);
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
    output_allow_failure_with_cancellation(root, args, None)
}

pub(crate) fn output_allow_failure_with_cancellation(
    root: &Path,
    args: &[&str],
    cancellation: Option<&CancellationToken>,
) -> Output {
    execute_git(root, args, cancellation)
}

pub(crate) fn error(output: &Output) -> String {
    let message = bytes_text(&output.stderr).trim();
    if message.is_empty() {
        let stdout = bytes_text(&output.stdout).trim();
        if stdout.is_empty() {
            output
                .status
                .code()
                .map(|code| format!("Git exited with status {code}"))
                .unwrap_or_else(|| "Git terminated unexpectedly".to_owned())
        } else {
            stdout.to_owned()
        }
    } else {
        message.to_owned()
    }
}

pub(super) fn bytes_text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).unwrap_or_default()
}

fn execute_git(root: &Path, args: &[&str], cancellation: Option<&CancellationToken>) -> Output {
    execute_program("git", root, args, cancellation, COMMAND_TIMEOUT, true)
}

fn execute_program(
    program: &str,
    root: &Path,
    args: &[&str],
    cancellation: Option<&CancellationToken>,
    timeout: Duration,
    git_environment: bool,
) -> Output {
    let mut command = Command::new(program);
    command.current_dir(root).args(args);
    if git_environment {
        command
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GCM_INTERACTIVE", "Never")
            .env("LC_ALL", "C");
        // Do not override an explicitly configured SSH command, but prevent a
        // system SSH prompt from blocking a desktop app when the user has no
        // usable key or agent.
        if std::env::var_os("GIT_SSH_COMMAND").is_none() {
            command.env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes");
        }
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Git may launch ssh or a credential helper. Keep the whole operation
        // in a process group so cancellation and timeouts cannot leave a child
        // holding stdout/stderr open after git itself exits.
        command.process_group(0);
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return failed_output(format!("could not execute Git: {error}")),
    };
    let process_id = child.id();
    let stdout_reader = spawn_reader(child.stdout.take());
    let stderr_reader = spawn_reader(child.stderr.take());
    let started = Instant::now();
    let mut termination_message = None;

    let status = loop {
        if cancellation.is_some_and(|token| token.is_cancelled()) {
            termination_message = Some("Git command cancelled".to_owned());
            terminate(&mut child, process_id);
            break child.wait().ok();
        }
        if started.elapsed() >= timeout {
            termination_message = Some(format!(
                "Git command timed out after {} seconds",
                timeout.as_secs()
            ));
            terminate(&mut child, process_id);
            break child.wait().ok();
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(error) => {
                termination_message = Some(format!("could not wait for Git: {error}"));
                terminate(&mut child, process_id);
                break child.wait().ok();
            }
        }
    };

    let mut stdout = receive_output(&stdout_reader, OUTPUT_DRAIN_TIMEOUT);
    let mut stderr = receive_output(&stderr_reader, OUTPUT_DRAIN_TIMEOUT);
    if stdout.is_none() || stderr.is_none() {
        // A helper that inherited one of the pipes can otherwise keep the
        // reader alive forever even though git has exited.
        terminate_process_group(process_id);
        if stdout.is_none() {
            stdout = receive_output(&stdout_reader, Duration::from_millis(250));
        }
        if stderr.is_none() {
            stderr = receive_output(&stderr_reader, Duration::from_millis(250));
        }
    }

    let mut stderr = stderr.unwrap_or_default();
    if let Some(message) = termination_message {
        if !stderr.is_empty() && !stderr.ends_with(b"\n") {
            stderr.push(b'\n');
        }
        stderr.extend_from_slice(message.as_bytes());
        stderr.push(b'\n');
    }

    Output {
        status: status.unwrap_or_else(command_failure_status),
        stdout: stdout.unwrap_or_default(),
        stderr,
    }
}

fn spawn_reader<R>(reader: Option<R>) -> Option<Receiver<Vec<u8>>>
where
    R: Read + Send + 'static,
{
    let reader = reader?;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = reader;
        let mut output = Vec::new();
        let _ = reader.read_to_end(&mut output);
        let _ = sender.send(output);
    });
    Some(receiver)
}

fn receive_output(receiver: &Option<Receiver<Vec<u8>>>, timeout: Duration) -> Option<Vec<u8>> {
    receiver.as_ref()?.recv_timeout(timeout).ok()
}

fn failed_output(message: String) -> Output {
    Output {
        status: command_failure_status(),
        stdout: Vec::new(),
        stderr: message.into_bytes(),
    }
}

fn terminate(child: &mut Child, process_id: u32) {
    terminate_process_group(process_id);
    let _ = child.kill();
}

fn terminate_process_group(process_id: u32) {
    #[cfg(unix)]
    {
        let _ = unsafe { libc::kill(-(process_id as libc::pid_t), libc::SIGKILL) };
    }
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

#[cfg(test)]
mod tests {
    use super::{error, execute_program, CancellationToken};
    use std::{
        path::Path,
        thread,
        time::{Duration, Instant},
    };

    #[test]
    fn drains_large_stdout_and_stderr_without_blocking_the_process() {
        let output = execute_program(
            "/bin/sh",
            Path::new("."),
            &[
                "-c",
                "i=0; while [ $i -lt 20000 ]; do printf stdout-%s\\n $i; printf stderr-%s\\n $i >&2; i=$((i + 1)); done",
            ],
            None,
            Duration::from_secs(5),
            false,
        );

        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("stdout-19999"));
        assert!(String::from_utf8_lossy(&output.stderr).contains("stderr-19999"));
    }

    #[test]
    fn cancellation_terminates_a_running_process_group() {
        let token = CancellationToken::default();
        let cancel = token.clone();
        let started = Instant::now();
        let thread = thread::spawn(move || {
            thread::sleep(Duration::from_millis(75));
            cancel.cancel();
        });
        let output = execute_program(
            "/bin/sh",
            Path::new("."),
            &["-c", "sleep 30"],
            Some(&token),
            Duration::from_secs(5),
            false,
        );
        thread.join().unwrap();

        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(error(&output).contains("cancelled"));
    }

    #[test]
    fn timeout_terminates_a_running_process_group() {
        let started = Instant::now();
        let output = execute_program(
            "/bin/sh",
            Path::new("."),
            &["-c", "sleep 30"],
            None,
            Duration::from_millis(75),
            false,
        );

        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(error(&output).contains("timed out"));
    }

    #[test]
    fn reports_the_exit_status_when_git_writes_no_diagnostics() {
        let output = execute_program(
            "/bin/sh",
            Path::new("."),
            &["-c", "exit 7"],
            None,
            Duration::from_secs(1),
            false,
        );

        assert_eq!(error(&output), "Git exited with status 7");
    }
}
