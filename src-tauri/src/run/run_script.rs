use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunScript {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(super) file_name: String,
    pub(super) file_path: String,
    pub(crate) content: String,
}
