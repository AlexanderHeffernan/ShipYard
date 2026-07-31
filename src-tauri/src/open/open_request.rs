use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenRequest {
    pub(super) application_id: String,
    pub(super) project_id: String,
    pub(super) checkout_path: String,
}
