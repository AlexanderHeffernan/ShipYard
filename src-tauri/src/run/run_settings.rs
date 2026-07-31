use super::run_script::RunScript;
use serde::Serialize;

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSettings {
    pub(crate) default_script_id: Option<String>,
    pub(crate) scripts: Vec<RunScript>,
}
