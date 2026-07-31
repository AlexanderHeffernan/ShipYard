use super::project_watch::ProjectWatch;
use crate::git;
use std::{collections::HashMap, sync::Mutex};
use tauri::AppHandle;

#[derive(Default)]
pub struct WatchManager {
    watches: Mutex<HashMap<String, ProjectWatch>>,
}

impl WatchManager {
    pub fn start(&self, app: AppHandle, path: &str) -> Result<(), String> {
        let (root, common_dir) = git::resolve(path)?;
        let project_id = common_dir.to_string_lossy().into_owned();
        let mut watches = self.watches.lock().map_err(|error| error.to_string())?;
        if watches.contains_key(&project_id) {
            return Ok(());
        }

        let project_watch = ProjectWatch::start(app, project_id.clone(), root, common_dir)?;
        watches.insert(project_id, project_watch);
        Ok(())
    }

    pub fn stop(&self, project_id: &str) -> Result<(), String> {
        self.watches
            .lock()
            .map_err(|error| error.to_string())?
            .remove(project_id);
        Ok(())
    }

    pub fn stop_all(&self) {
        if let Ok(mut watches) = self.watches.lock() {
            watches.clear();
        }
    }
}
