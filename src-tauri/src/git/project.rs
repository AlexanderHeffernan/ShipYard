use super::work_item::WorkItem;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub(crate) id: String,
    pub(super) name: String,
    pub(super) path: String,
    pub(super) default_branch: Option<String>,
    pub(crate) work_items: Vec<WorkItem>,
    pub(crate) github_repository: Option<String>,
    pub(crate) github_error: Option<String>,
}
