use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RunFinished {
    pub(super) run_id: String,
    pub(super) exit_code: Option<i32>,
    pub(super) success: bool,
}
