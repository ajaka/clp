pub mod clear;
pub mod default;
pub mod help;
pub mod lines;
pub mod paste;
pub mod range;
pub mod until;
pub mod verbose;

use anyhow::Result;
use colored::*;

use crate::cli::{Cli, Operation};
use crate::common;

/// A chunk of content selected from the input, with a description of what was selected.
pub struct Selection {
    pub content: String,
    /// Human-readable summary, e.g. `lines 1-2` or `until "END"`.
    pub label: String,
}

/// Read the content selected by `cli`'s operation.
///
/// Only called for content-producing operations; `Paste` and `Clear` are handled
/// by the caller because they address the clipboard directly.
pub fn select(cli: &Cli) -> Result<Selection> {
    match &cli.operation {
        Operation::Copy => default::read(&cli.source),
        Operation::Until {
            delimiters,
            inclusive,
        } => until::read(&cli.source, delimiters, *inclusive),
        Operation::Lines { count } => lines::read(&cli.source, *count),
        Operation::Range { start, end } => range::read(&cli.source, *start, *end),
        Operation::Paste | Operation::Clear => {
            unreachable!("Paste and Clear are handled by the caller")
        }
    }
}

/// Write `content` to the clipboard, honouring the Append modifier, then report.
pub fn deliver(cli: &Cli, selection: &Selection) -> Result<()> {
    let content = if cli.has(crate::cli::Modifier::Trim) {
        common::trim_whitespace(&selection.content)
    } else {
        selection.content.clone()
    };

    if cli.has(crate::cli::Modifier::Append) {
        common::append_to_clipboard(&content)?;
        if cli.has(crate::cli::Modifier::Verbose) {
            verbose::report(&content, "Appended to clipboard");
        } else {
            println!("Appended to clipboard ({})", selection.label);
        }
    } else {
        common::copy_to_clipboard(&content)?;
        if cli.has(crate::cli::Modifier::Verbose) {
            verbose::report(&content, "Copied to clipboard");
        } else {
            println!("Copied to clipboard ({})", selection.label);
        }
    }
    Ok(())
}

/// Print the guidance shown when no clipboard manager is available.
pub fn no_manager_message() -> String {
    use std::fmt::Write as _;
    let mut s = String::new();
    let _ = writeln!(s, "{}", "No clipboard manager detected.".red().bold());
    let _ = writeln!(s, "{}", "Install one of the following:".white());
    for m in crate::detect_platform::KNOWN_MANAGERS
        .iter()
        .filter(|m| m.is_wayland)
    {
        let _ = writeln!(s, "  - {}", m.display_name.white());
    }
    let _ = writeln!(s, "\n{}", "X11:".yellow().bold());
    for m in crate::detect_platform::KNOWN_MANAGERS
        .iter()
        .filter(|m| !m.is_wayland)
    {
        let _ = writeln!(s, "  - {}", m.display_name.white());
    }
    let _ = writeln!(
        s,
        "\n{}",
        "Or use a desktop environment with built-in clipboard support (KDE, GNOME, XFCE, MATE, Cinnamon).".white()
    );
    s
}
