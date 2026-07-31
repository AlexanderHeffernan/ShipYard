use super::watch_message::WatchMessage;
use crate::git;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};

const DEBOUNCE: Duration = Duration::from_millis(250);
const MAX_DEBOUNCE: Duration = Duration::from_secs(1);

pub(super) struct ProjectWatch {
    sender: Sender<WatchMessage>,
    worker: Option<JoinHandle<()>>,
}

impl ProjectWatch {
    pub(super) fn start(
        app: AppHandle,
        project_id: String,
        root: PathBuf,
        common_dir: PathBuf,
    ) -> Result<Self, String> {
        let (sender, receiver) = mpsc::channel();
        let callback_sender = sender.clone();
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            run_watcher(
                app,
                project_id,
                root,
                common_dir,
                receiver,
                callback_sender,
                ready_sender,
            );
        });
        ready_receiver.recv().map_err(|error| error.to_string())??;
        Ok(Self {
            sender,
            worker: Some(worker),
        })
    }
}

impl Drop for ProjectWatch {
    fn drop(&mut self) {
        let _ = self.sender.send(WatchMessage::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run_watcher(
    app: AppHandle,
    project_id: String,
    root: PathBuf,
    common_dir: PathBuf,
    receiver: Receiver<WatchMessage>,
    callback_sender: Sender<WatchMessage>,
    ready_sender: SyncSender<Result<(), String>>,
) {
    let watcher = notify::recommended_watcher(move |event| {
        let _ = callback_sender.send(WatchMessage::Change(event));
    });
    let mut watcher = match watcher {
        Ok(watcher) => watcher,
        Err(error) => {
            let _ = ready_sender.send(Err(error.to_string()));
            return;
        }
    };
    let mut watched = HashSet::new();
    if let Err(error) = reconcile_paths(&mut watcher, &root, &common_dir, &mut watched) {
        let _ = ready_sender.send(Err(error));
        return;
    }
    let _ = ready_sender.send(Ok(()));
    event_loop(
        app, project_id, root, common_dir, receiver, watcher, watched,
    );
}

fn event_loop(
    app: AppHandle,
    project_id: String,
    root: PathBuf,
    common_dir: PathBuf,
    receiver: Receiver<WatchMessage>,
    mut watcher: RecommendedWatcher,
    mut watched: HashSet<PathBuf>,
) {
    let mut pending_since = None;
    let mut deadline: Option<Instant> = None;
    loop {
        let timeout = deadline
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .unwrap_or(Duration::from_secs(3600));
        match receiver.recv_timeout(timeout) {
            Ok(WatchMessage::Change(event)) if relevant(&event, &common_dir) => {
                let now = Instant::now();
                let first = *pending_since.get_or_insert(now);
                deadline = Some(debounce_deadline(first, now));
            }
            Ok(WatchMessage::Change(_)) => {}
            Ok(WatchMessage::Stop) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) if pending_since.is_some() => {
                let _ = reconcile_paths(&mut watcher, &root, &common_dir, &mut watched);
                let _ = app.emit("project-changed", &project_id);
                pending_since = None;
                deadline = None;
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

fn debounce_deadline(first: Instant, latest: Instant) -> Instant {
    std::cmp::min(latest + DEBOUNCE, first + MAX_DEBOUNCE)
}

fn reconcile_paths(
    watcher: &mut RecommendedWatcher,
    root: &Path,
    common_dir: &Path,
    watched: &mut HashSet<PathBuf>,
) -> Result<(), String> {
    let mut desired: HashSet<_> = git::worktree_paths(root)?.into_iter().collect();
    desired.insert(common_dir.to_owned());
    for path in watched.difference(&desired) {
        let _ = watcher.unwatch(path);
    }
    for path in desired.difference(watched) {
        watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|error| format!("could not watch {}: {error}", path.display()))?;
    }
    *watched = desired;
    Ok(())
}

fn relevant(event: &notify::Result<Event>, common_dir: &Path) -> bool {
    let Ok(event) = event else {
        return true;
    };
    event.paths.iter().any(|path| {
        path.starts_with(common_dir)
            || !path
                .components()
                .any(|component| ignored_component(component))
    })
}

fn ignored_component(component: Component<'_>) -> bool {
    matches!(
        component.as_os_str().to_str(),
        Some(".git" | "node_modules" | "dist" | "target" | ".vite")
    )
}

#[cfg(test)]
mod tests {
    use super::{debounce_deadline, relevant, DEBOUNCE, MAX_DEBOUNCE};
    use notify::{Event, EventKind};
    use std::{path::Path, time::Instant};

    #[test]
    fn debounce_coalesces_bursts_without_unbounded_delay() {
        let first = Instant::now();
        assert_eq!(debounce_deadline(first, first), first + DEBOUNCE);
        assert_eq!(
            debounce_deadline(first, first + MAX_DEBOUNCE),
            first + MAX_DEBOUNCE
        );
    }

    #[test]
    fn ignores_build_output_but_keeps_git_metadata() {
        let common = Path::new("/repo/.git");
        let mut build_event = Event::new(EventKind::Any);
        build_event.paths.push("/repo/dist/app.js".into());
        assert!(!relevant(&Ok(build_event), common));

        let mut git_event = Event::new(EventKind::Any);
        git_event.paths.push("/repo/.git/refs/heads/main".into());
        assert!(relevant(&Ok(git_event), common));
    }
}
