use super::{
    run_event::RunEvent, run_finished::RunFinished, run_manager::RunManager,
    run_request::RunRequest, run_session::RunSession, run_started::RunStarted,
    spawned_terminal::SpawnedTerminal, store, terminal_size::TerminalSize,
};
use crate::{git, shipping};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::{
    io::Read,
    path::Path,
    sync::{atomic::Ordering, Arc},
    thread,
};
use tauri::{Emitter, Manager};

const INITIAL_TERMINAL_SIZE: PtySize = PtySize {
    rows: 30,
    cols: 120,
    pixel_width: 0,
    pixel_height: 0,
};

impl RunManager {
    pub fn start_shipping(
        &self,
        app: tauri::AppHandle,
        request: shipping::ShippingRequest,
    ) -> Result<RunStarted, String> {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| error.to_string())?;
        let prepared = shipping::prepare(&data_dir, request)?;
        let run_id = format!("run-{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let terminal = spawn_terminal(
            &prepared.script_path,
            prepared.working_directory.to_string_lossy().as_ref(),
        )?;
        self.sessions
            .lock()
            .map_err(lock_error)?
            .insert(run_id.clone(), terminal.session.clone());
        let output = stream_output(app.clone(), run_id.clone(), terminal.reader);
        wait_for_exit(
            app,
            run_id.clone(),
            terminal.child,
            output,
            self.sessions.clone(),
        );
        Ok(RunStarted { run_id })
    }

    pub fn start(&self, app: tauri::AppHandle, request: RunRequest) -> Result<RunStarted, String> {
        validate_working_directory(&request)?;
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| error.to_string())?;
        let script_path = store::script_path(&data_dir, &request.project_id, &request.script_id)?;
        let run_id = format!("run-{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let terminal = spawn_terminal(&script_path, &request.working_directory)?;
        self.sessions
            .lock()
            .map_err(lock_error)?
            .insert(run_id.clone(), terminal.session.clone());
        let output = stream_output(app.clone(), run_id.clone(), terminal.reader);
        wait_for_exit(
            app,
            run_id.clone(),
            terminal.child,
            output,
            self.sessions.clone(),
        );
        Ok(RunStarted { run_id })
    }

    pub fn cancel(&self, run_id: &str) -> Result<(), String> {
        let session = self.active_session(run_id)?;
        session.terminate()?;
        force_termination(run_id.to_owned(), session, self.sessions.clone());
        Ok(())
    }

    pub fn write(&self, run_id: &str, input: &str) -> Result<(), String> {
        self.active_session(run_id)?.write(input)
    }

    pub fn resize(&self, run_id: &str, size: TerminalSize) -> Result<(), String> {
        self.active_session(run_id)?.resize(size.into_pty_size()?)
    }

    pub fn terminate_all(&self) {
        if let Ok(sessions) = self.sessions.lock() {
            for session in sessions.values() {
                session.force_terminate();
            }
        }
    }

    fn active_session(&self, run_id: &str) -> Result<Arc<RunSession>, String> {
        self.sessions
            .lock()
            .map_err(lock_error)?
            .get(run_id)
            .cloned()
            .ok_or_else(|| "run is not active".to_owned())
    }
}

pub(super) fn spawn_terminal(
    script: &Path,
    working_directory: &str,
) -> Result<SpawnedTerminal, String> {
    spawn_command(run_command(script, working_directory))
}

fn spawn_command(command: CommandBuilder) -> Result<SpawnedTerminal, String> {
    let pair = native_pty_system()
        .openpty(INITIAL_TERMINAL_SIZE)
        .map_err(|error| error.to_string())?;
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| error.to_string())?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| error.to_string())?;
    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| format!("could not start run script: {error}"))?;
    let killer = child.clone_killer();
    #[cfg(unix)]
    let process_group = pair
        .master
        .process_group_leader()
        .or_else(|| child.process_id().map(|pid| pid as i32));
    #[cfg(not(unix))]
    let process_group = None;
    drop(pair.slave);
    Ok(SpawnedTerminal {
        child,
        reader,
        session: Arc::new(RunSession::new(pair.master, writer, killer, process_group)),
    })
}

fn run_command(script: &Path, working_directory: &str) -> CommandBuilder {
    let mut command = CommandBuilder::new("/bin/zsh");
    command.arg(script);
    command.cwd(working_directory);
    command.env("SHIPYARD_WORKTREE_PATH", working_directory);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    command
}

fn validate_working_directory(request: &RunRequest) -> Result<(), String> {
    if !Path::new(&request.working_directory).is_dir() {
        return Err("run checkout no longer exists".to_owned());
    }
    if !git::belongs_to_project(&request.project_id, &request.working_directory)? {
        return Err("run checkout does not belong to this project".to_owned());
    }
    Ok(())
}

fn stream_output(
    app: tauri::AppHandle,
    run_id: String,
    mut reader: Box<dyn Read + Send>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0; 4096];
        while let Ok(count) = reader.read(&mut buffer) {
            if count == 0 {
                break;
            }
            let _ = app.emit(
                "run-output",
                RunEvent {
                    run_id: run_id.clone(),
                    data: buffer[..count].to_vec(),
                },
            );
        }
    })
}

fn wait_for_exit(
    app: tauri::AppHandle,
    run_id: String,
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
    output: thread::JoinHandle<()>,
    sessions: Arc<std::sync::Mutex<std::collections::HashMap<String, Arc<RunSession>>>>,
) {
    thread::spawn(move || {
        let status = child.wait();
        let _ = output.join();
        if let Ok(mut sessions) = sessions.lock() {
            sessions.remove(&run_id);
        }
        let event = match status {
            Ok(status) => RunFinished {
                run_id,
                exit_code: Some(status.exit_code() as i32),
                success: status.success(),
            },
            Err(_) => RunFinished {
                run_id,
                exit_code: None,
                success: false,
            },
        };
        let _ = app.emit("run-finished", event);
    });
}

fn force_termination(
    run_id: String,
    session: Arc<RunSession>,
    sessions: Arc<std::sync::Mutex<std::collections::HashMap<String, Arc<RunSession>>>>,
) {
    thread::spawn(move || {
        thread::sleep(std::time::Duration::from_secs(2));
        let still_running = sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(&run_id).cloned())
            .is_some_and(|active| Arc::ptr_eq(&active, &session));
        if still_running {
            session.force_terminate();
        }
    });
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> String {
    error.to_string()
}
