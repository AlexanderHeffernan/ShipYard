use super::run_script::RunScript;
use serde::Serialize;

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSettings {
    pub(super) default_script_id: Option<String>,
    pub(super) scripts: Vec<RunScript>,
}
