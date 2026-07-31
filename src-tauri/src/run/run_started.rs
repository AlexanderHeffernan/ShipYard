use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunStarted {
    pub(super) run_id: String,
}
