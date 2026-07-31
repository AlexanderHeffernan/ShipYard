use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RunEvent {
    pub(super) run_id: String,
    pub(super) data: Vec<u8>,
}
