use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptInput {
    pub(super) id: Option<String>,
    pub(super) label: String,
    pub(super) content: String,
    pub(super) make_default: bool,
}
