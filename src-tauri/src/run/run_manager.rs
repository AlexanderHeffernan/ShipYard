use super::run_session::RunSession;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc, Mutex},
};

pub struct RunManager {
    pub(super) sessions: Arc<Mutex<HashMap<String, Arc<RunSession>>>>,
    pub(super) next_id: AtomicU64,
}

impl Default for RunManager {
    fn default() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            next_id: AtomicU64::new(1),
        }
    }
}
