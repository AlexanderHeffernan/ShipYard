use super::application_kind::ApplicationKind;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StoredOpenApplication {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) kind: ApplicationKind,
    pub(super) app_path: String,
}
