use portable_pty::{ChildKiller, MasterPty, PtySize};
use std::{
    io::Write,
    sync::{Mutex, PoisonError},
};

pub(super) struct RunSession {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    process_group: Option<i32>,
}

impl RunSession {
    pub(super) fn new(
        master: Box<dyn MasterPty + Send>,
        writer: Box<dyn Write + Send>,
        killer: Box<dyn ChildKiller + Send + Sync>,
        process_group: Option<i32>,
    ) -> Self {
        Self {
            master: Mutex::new(master),
            writer: Mutex::new(writer),
            killer: Mutex::new(killer),
            process_group,
        }
    }

    pub(super) fn write(&self, input: &str) -> Result<(), String> {
        let mut writer = self.writer.lock().map_err(lock_error)?;
        writer.write_all(input.as_bytes()).map_err(io_error)?;
        writer.flush().map_err(io_error)
    }

    pub(super) fn resize(&self, size: PtySize) -> Result<(), String> {
        self.master
            .lock()
            .map_err(lock_error)?
            .resize(size)
            .map_err(|error| error.to_string())
    }

    pub(super) fn terminate(&self) -> Result<(), String> {
        #[cfg(unix)]
        if let Some(process_group) = self.process_group {
            return signal_process_group(process_group, libc::SIGTERM);
        }
        self.killer
            .lock()
            .map_err(lock_error)?
            .kill()
            .map_err(io_error)
    }

    pub(super) fn force_terminate(&self) {
        #[cfg(unix)]
        if let Some(process_group) = self.process_group {
            let _ = signal_process_group(process_group, libc::SIGKILL);
            return;
        }
        let _ = self.killer.lock().map(|mut killer| killer.kill());
    }
}

#[cfg(unix)]
fn signal_process_group(process_group: i32, signal: i32) -> Result<(), String> {
    let result = unsafe { libc::kill(-process_group, signal) };
    (result == 0)
        .then_some(())
        .ok_or_else(|| std::io::Error::last_os_error().to_string())
}

fn lock_error<T>(error: PoisonError<T>) -> String {
    error.to_string()
}

fn io_error(error: std::io::Error) -> String {
    error.to_string()
}
