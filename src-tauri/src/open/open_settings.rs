use super::open_application::OpenApplication;
use serde::Serialize;

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenSettings {
    pub(super) default_application_id: Option<String>,
    pub(super) applications: Vec<OpenApplication>,
}
