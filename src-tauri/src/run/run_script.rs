use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunScript {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) file_name: String,
    pub(super) file_path: String,
    pub(super) content: String,
}
