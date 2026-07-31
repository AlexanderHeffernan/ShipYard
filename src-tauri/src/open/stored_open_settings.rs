use super::stored_open_application::StoredOpenApplication;
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StoredOpenSettings {
    pub(super) default_application_id: Option<String>,
    pub(super) applications: Vec<StoredOpenApplication>,
}
