use super::{request::ShipRequest, safety};
use std::path::PathBuf;

#[derive(Clone)]
pub(crate) struct ShipContext {
    pub(crate) project_id: String,
    pub(crate) work_item_id: String,
    pub(crate) source_path: PathBuf,
    pub(crate) source_branch: Option<String>,
    pub(crate) source_sha: String,
    pub(crate) default_branch: String,
    pub(crate) target_path: PathBuf,
}

impl ShipContext {
    pub(crate) fn validated(request: ShipRequest) -> Result<Self, String> {
        safety::validate(&request)
    }
}
