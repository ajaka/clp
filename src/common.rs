use anyhow::{Context, Result};
use std::fs;

use crate::cli::Source;

/// Interface for system clipboard interactions.
pub trait Clipboard {
    fn copy(&mut self, content: &str) -> Result<()>;
    fn paste(&mut self) -> Result<String>;
    fn clear(&mut self) -> Result<()>;
}

/// Clipboard backend backed by `arboard`.
struct ArboardClipboard(arboard::Clipboard);

impl ArboardClipboard {
    fn new() -> Result<Self> {
        let cb = arboard::Clipboard::new().with_context(|| "Failed to initialize clipboard")?;
        Ok(Self(cb))
    }
}

impl Clipboard for ArboardClipboard {
    fn copy(&mut self, content: &str) -> Result<()> {
        self.0
            .set_text(content)
            .with_context(|| "Failed to set clipboard content")
    }

    fn paste(&mut self) -> Result<String> {
        self.0
            .get_text()
            .with_context(|| "Failed to get clipboard content")
    }

    fn clear(&mut self) -> Result<()> {
        self.0
            .set_text("")
            .with_context(|| "Failed to clear clipboard")
    }
}

#[cfg(target_os = "linux")]
struct XclipClipboard;

#[cfg(target_os = "linux")]
impl Clipboard for XclipClipboard {
    fn copy(&mut self, content: &str) -> Result<()> {
        use std::io::Write;
        use std::process::Command;

        let mut child = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .with_context(|| "Failed to spawn xclip (is it installed?)")?;

        child
            .stdin
            .as_mut()
            .with_context(|| "Failed to open xclip stdin")?
            .write_all(content.as_bytes())
            .with_context(|| "Failed to write to xclip stdin")?;

        let output = child
            .wait_with_output()
            .with_context(|| "Failed to wait for xclip")?;

        if !output.status.success() {
            anyhow::bail!(
                "xclip failed with exit code: {}",
                output.status.code().unwrap_or(-1)
            );
        }
        Ok(())
    }

    fn paste(&mut self) -> Result<String> {
        use std::process::Command;

        let output = Command::new("xclip")
            .args(["-selection", "clipboard", "-o"])
            .output()
            .with_context(|| "Failed to execute xclip (is it installed?)")?;

        if !output.status.success() {
            anyhow::bail!(
                "xclip failed with exit code: {}",
                output.status.code().unwrap_or(-1)
            );
        }

        String::from_utf8(output.stdout)
            .with_context(|| "Clipboard content does not contain valid UTF-8 text")
    }

    fn clear(&mut self) -> Result<()> {
        self.copy("")
    }
}

/// Retrieve the active clipboard backend.
///
/// On Linux:
/// - Prefers `xclip` when in pure X11 (DISPLAY is set and WAYLAND_DISPLAY is unset) if xclip is installed.
/// - Otherwise uses `arboard` (which supports Wayland data-control).
///
/// On non-Linux platforms, uses `arboard`.
pub fn get_clipboard() -> Result<Box<dyn Clipboard>> {
    #[cfg(target_os = "linux")]
    {
        let has_display = std::env::var_os("DISPLAY").is_some();

        if has_display
            && std::process::Command::new("xclip")
                .arg("-version")
                .output()
                .is_ok()
        {
            return Ok(Box::new(XclipClipboard));
        }
        Ok(Box::new(ArboardClipboard::new()?))
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok(Box::new(ArboardClipboard::new()?))
    }
}

/// Copy text content to the system clipboard.
pub fn copy_to_clipboard(content: &str) -> Result<()> {
    get_clipboard()?.copy(content)
}

/// Clear the system clipboard.
pub fn clear_clipboard() -> Result<()> {
    get_clipboard()?.clear()
}

/// Paste text content from the system clipboard.
pub fn paste_from_clipboard() -> Result<String> {
    get_clipboard()?.paste()
}

/// Append text content to the system clipboard.
///
/// Inserts a newline between existing content and new content if the existing
/// content is not empty and does not end with a newline.
pub fn append_to_clipboard(content: &str) -> Result<()> {
    let mut clipboard = get_clipboard()?;
    let existing = clipboard.paste().unwrap_or_default();
    let new_content = if existing.is_empty() {
        content.to_string()
    } else if existing.ends_with('\n') {
        format!("{}{}", existing, content)
    } else {
        format!("{}\n{}", existing, content)
    };
    clipboard
        .copy(&new_content)
        .context("Failed to copy appended content to clipboard")
}

/// Trim leading and trailing whitespace from content.
pub fn trim_whitespace(content: &str) -> String {
    content.trim().to_string()
}

/// Read the entire content of `source`.
///
/// Prompts with a hint if stdin is an interactive terminal, and produces clear
/// errors when reading invalid UTF-8 files.
pub fn read_source_text(source: &Source) -> Result<String> {
    match source {
        Source::Stdin => {
            use std::io::{IsTerminal, Read};
            if std::io::stdin().is_terminal() {
                eprintln!("Reading from stdin... (press Ctrl+D to end)");
            }
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .with_context(|| "Failed to read from stdin")?;
            Ok(buf)
        }
        Source::Path(p) => {
            let bytes = fs::read(p).with_context(|| format!("Failed to read {}", p.display()))?;
            String::from_utf8(bytes)
                .map_err(|e| anyhow::anyhow!("File {} contains non-UTF-8 data: {e}", p.display()))
        }
    }
}

/// Find the earliest occurrence of any delimiter in the content.
/// Breaks ties by preferring the longer delimiter.
fn find_earliest<'a>(content: &'a str, delimiters: &[&'a str]) -> Option<(usize, &'a str)> {
    delimiters
        .iter()
        .filter(|d| !d.is_empty())
        .filter_map(|d| content.find(d).map(|pos| (pos, *d)))
        .min_by_key(|(pos, delim)| (*pos, std::cmp::Reverse(delim.len())))
}

/// Cut already-loaded `content` at the first occurrence of any delimiter.
///
/// With `inclusive`, the delimiter itself is kept. When no delimiter matches,
/// the content is returned unchanged.
pub fn cut(content: &str, delimiters: &[String], inclusive: bool) -> String {
    let candidates: Vec<&str> = delimiters
        .iter()
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .collect();

    if candidates.is_empty() {
        return content.to_string();
    }

    match find_earliest(content, &candidates) {
        Some((pos, delim)) => {
            let end = if inclusive { pos + delim.len() } else { pos };
            content[..end].to_string()
        }
        None => content.to_string(),
    }
}

/// Read content up to (but not including) the first occurrence of any delimiter.
pub fn read_until(source: &Source, delimiters: &[String]) -> Result<String> {
    Ok(cut(&read_source_text(source)?, delimiters, false))
}

/// Read content up to and including the first occurrence of any delimiter.
pub fn read_until_inclusive(source: &Source, delimiters: &[String]) -> Result<String> {
    Ok(cut(&read_source_text(source)?, delimiters, true))
}

/// Read the first `count` lines, preserving each line's original terminator
/// (so CRLF input stays CRLF and a trailing newline is retained).
pub fn read_lines(source: &Source, count: usize) -> Result<String> {
    if count == 0 {
        anyhow::bail!("Line count must be greater than zero");
    }
    let content = read_source_text(source)?;
    Ok(content.split_inclusive('\n').take(count).collect())
}

/// Read an inclusive 1-indexed line range, preserving line terminators.
/// A range extending past the end of the input yields the available lines.
pub fn read_line_range(source: &Source, start: usize, end: usize) -> Result<String> {
    if start == 0 {
        anyhow::bail!("Start line must be greater than zero");
    }
    if end < start {
        anyhow::bail!("End line must be greater than or equal to start line");
    }
    let content = read_source_text(source)?;
    Ok(content
        .split_inclusive('\n')
        .skip(start - 1)
        .take(end - start + 1)
        .collect())
}

#[cfg(test)]
pub struct MockClipboard {
    pub content: String,
}

#[cfg(test)]
impl MockClipboard {
    pub fn new(initial: &str) -> Self {
        Self {
            content: initial.to_string(),
        }
    }
}

#[cfg(test)]
impl Clipboard for MockClipboard {
    fn copy(&mut self, text: &str) -> Result<()> {
        self.content = text.to_string();
        Ok(())
    }

    fn paste(&mut self) -> Result<String> {
        Ok(self.content.clone())
    }

    fn clear(&mut self) -> Result<()> {
        self.content.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_whitespace() {
        assert_eq!(trim_whitespace("  hello  "), "hello");
        assert_eq!(trim_whitespace("\n\thello\n"), "hello");
        assert_eq!(trim_whitespace("hello"), "hello");
        assert_eq!(trim_whitespace(""), "");
        assert_eq!(trim_whitespace("   "), "");
    }

    #[test]
    fn test_trim_whitespace_multiline() {
        assert_eq!(trim_whitespace("  hello\nworld  "), "hello\nworld");
        assert_eq!(trim_whitespace("\nhello\nworld\n"), "hello\nworld");
    }

    #[test]
    fn test_mock_clipboard_roundtrip() {
        let mut cb = MockClipboard::new("initial");
        assert_eq!(cb.paste().unwrap(), "initial");

        cb.copy("updated").unwrap();
        assert_eq!(cb.paste().unwrap(), "updated");

        cb.clear().unwrap();
        assert_eq!(cb.paste().unwrap(), "");
    }

    #[test]
    fn test_cut_tie_breaking_prefers_longer_delimiter() {
        let text = "hello START_EXTRA world";
        let delims = vec!["START".to_string(), "START_EXTRA".to_string()];
        let cut_res = cut(text, &delims, false);
        assert_eq!(cut_res, "hello ");

        let cut_inc = cut(text, &delims, true);
        assert_eq!(cut_inc, "hello START_EXTRA");
    }

    #[test]
    fn test_non_utf8_file_reports_clear_error() {
        let dir = crate::testutil::tempfile();
        let path = dir.path().join("invalid.bin");
        // Invalid UTF-8 sequence
        std::fs::write(&path, [0xFF, 0xFE, 0xFD]).unwrap();

        let err = read_source_text(&Source::Path(path)).unwrap_err();
        assert!(err.to_string().contains("contains non-UTF-8 data"));
    }
}
