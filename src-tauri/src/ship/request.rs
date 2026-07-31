use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShipRequest {
    pub(crate) project_id: String,
    pub(crate) script_id: String,
    pub(crate) work_item_id: String,
    pub(crate) source_path: String,
    pub(crate) source_branch: Option<String>,
    pub(crate) source_sha: String,
    pub(crate) default_branch: String,
}
