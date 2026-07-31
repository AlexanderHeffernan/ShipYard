use super::application_kind::ApplicationKind;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApplication {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) kind: ApplicationKind,
    pub(super) app_path: String,
    pub(super) available: bool,
}
