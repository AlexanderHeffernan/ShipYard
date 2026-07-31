use super::run_session::RunSession;
use portable_pty::Child;
use std::{io::Read, sync::Arc};

pub(super) struct SpawnedTerminal {
    pub(super) child: Box<dyn Child + Send + Sync>,
    pub(super) reader: Box<dyn Read + Send>,
    pub(super) session: Arc<RunSession>,
}
