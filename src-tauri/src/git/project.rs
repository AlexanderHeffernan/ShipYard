use super::work_item::WorkItem;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) path: String,
    pub(super) default_branch: Option<String>,
    pub(super) work_items: Vec<WorkItem>,
}
