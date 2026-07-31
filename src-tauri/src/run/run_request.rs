use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
    pub(super) project_id: String,
    pub(super) script_id: String,
    pub(super) working_directory: String,
}
