use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StoredRunScript {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) file_name: String,
}
