use anyhow::Result;

use crate::common;

/// Write the clipboard contents to stdout.
///
/// The content is written verbatim; no trailing newline is added, so piping
/// yields exact bytes. Interactive use may leave the cursor mid-line if the
/// clipboard has no trailing newline.
pub fn handle_paste() -> Result<()> {
    let content = common::paste_from_clipboard()?;
    print!("{content}");
    Ok(())
}
