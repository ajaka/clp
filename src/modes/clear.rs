use anyhow::Result;

use crate::common;

/// Empty the clipboard.
pub fn handle_clear() -> Result<()> {
    common::clear_clipboard()?;
    println!("Clipboard cleared");
    Ok(())
}
