use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum WorkStatus {
    Working,
    Ready,
    Shipped,
    #[serde(rename = "mergeConflict")]
    MergeConflict,
}
