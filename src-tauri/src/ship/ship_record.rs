use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShipRecord {
    pub(crate) work_item_id: String,
    pub(crate) source_sha: String,
    pub(crate) default_branch: String,
    pub(crate) source_path: String,
    pub(crate) target_path: String,
    #[serde(default)]
    pub(crate) successful: bool,
}
