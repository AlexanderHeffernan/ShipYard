#[derive(Default)]
pub(crate) struct ShipStates {
    pub(crate) conflicts: Vec<(String, String)>,
    pub(crate) shipped: Vec<(String, String)>,
}
