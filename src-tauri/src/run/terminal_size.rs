use portable_pty::PtySize;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSize {
    columns: u16,
    rows: u16,
}

impl TerminalSize {
    pub(super) fn into_pty_size(self) -> Result<PtySize, String> {
        if self.columns == 0 || self.rows == 0 {
            return Err("terminal dimensions must be greater than zero".to_owned());
        }
        Ok(PtySize {
            rows: self.rows,
            cols: self.columns,
            pixel_width: 0,
            pixel_height: 0,
        })
    }
}
