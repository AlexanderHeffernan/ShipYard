use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptInput {
    pub(crate) id: Option<String>,
    pub(crate) label: String,
    pub(crate) content: String,
    pub(crate) make_default: bool,
}
