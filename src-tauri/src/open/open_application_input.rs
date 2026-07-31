use super::application_kind::ApplicationKind;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApplicationInput {
    pub(super) id: Option<String>,
    pub(super) label: String,
    pub(super) kind: ApplicationKind,
    pub(super) app_path: String,
    pub(super) make_default: bool,
}
