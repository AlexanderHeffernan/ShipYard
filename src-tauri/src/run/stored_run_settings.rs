use super::stored_run_script::StoredRunScript;
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StoredRunSettings {
    pub(super) default_script_id: Option<String>,
    pub(super) scripts: Vec<StoredRunScript>,
}
