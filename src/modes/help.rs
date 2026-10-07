//! Help text rendering.
//!
//! One table drives both the colourised and plain variants so the two cannot drift.

/// Whether ANSI colour should be emitted.
///
/// Honours the `NO_COLOR` convention (any value, including empty, disables
/// colour), plus `CLICOLOR=0`, `CLICOLOR_FORCE` and `TERM=dumb`.
fn colour_enabled() -> bool {
    // https://no-color.org: presence alone disables colour, even when empty.
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if std::env::var("CLICOLOR").as_deref() == Ok("0") {
        return false;
    }
    if std::env::var("TERM").as_deref() == Ok("dumb") {
        return false;
    }
    if std::env::var("CLICOLOR_FORCE").as_deref() == Ok("1") {
        return true;
    }
    true
}

/// A flag and its description.
struct Flag(&'static str, &'static str);

const FLAGS: &[Flag] = &[
    Flag("-p", "Write the clipboard to stdout"),
    Flag("-c", "Clear the clipboard"),
    Flag(
        "-u DELIM...",
        "Copy up to the first of any delimiter (delimiter excluded)",
    ),
    Flag(
        "-i DELIM...",
        "Copy up to the first of any delimiter (delimiter included)",
    ),
    Flag("-l N", "Copy the first N lines"),
    Flag("-r START:END", "Copy an inclusive 1-indexed line range"),
    Flag("-a", "Append to the existing clipboard content"),
    Flag("-v", "Report what was copied"),
    Flag("-t", "Trim leading and trailing whitespace"),
    Flag("--help, -help", "Show this help"),
];

const EXAMPLES: &[(&str, &str)] = &[
    ("echo hello | clp", "Copy from stdin"),
    ("clp file.txt", "Copy an entire file"),
    ("clp -p", "Write the clipboard to stdout"),
    ("clp -c", "Clear the clipboard"),
    ("clp -u END file.txt", "Copy up to \"END\""),
    (
        "clp -i END STOP file.txt",
        "Copy up to whichever comes first",
    ),
    ("clp -l 10 file.txt", "Copy the first 10 lines"),
    ("clp -r 5:15 file.txt", "Copy lines 5 through 15"),
    (
        "clp -u END -a file.txt",
        "Combine: copy up to \"END\", then append",
    ),
    ("clp -t file.txt", "Copy without surrounding whitespace"),
    ("clp -v file.txt", "Copy and report a summary"),
];

const NOTES: &[&str] = &[
    "With no FILE, clp reads from stdin.",
    "Operations (-u, -i, -l, -r, -p, -c) are mutually exclusive.",
    "Modifiers (-a, -v, -t) combine with any operation.",
    "-u and -i take several delimiters; use -- to end them: clp -u END STOP -- f.txt",
    "-r accepts open ranges: 5: is 5 to end, :10 is start to 10.",
    "-r is 1-indexed and inclusive; -l must be greater than zero.",
    "Line endings are preserved exactly as they appear in the input.",
];

/// Longest flag string, used to align the description column.
const FLAG_WIDTH: usize = 20;

/// Display usage information.
pub fn handle_help() -> anyhow::Result<()> {
    if colour_enabled() {
        print_colour();
    } else {
        print_plain();
    }
    Ok(())
}

fn print_colour() {
    use colored::*;

    println!(
        "{}\n",
        "clp - copy parts of files to the clipboard"
            .bright_cyan()
            .bold()
    );

    println!("{}", "USAGE".yellow().bold());
    println!("    {}\n", "clp [OPERATION] [MODIFIERS] [FILE]".white());

    println!("{}", "OPERATIONS & MODIFIERS".yellow().bold());
    for Flag(flag, desc) in FLAGS {
        println!(
            "    {:<width$}  {}",
            flag.green().bold(),
            desc.white(),
            width = FLAG_WIDTH
        );
    }
    println!();

    println!("{}", "EXAMPLES".yellow().bold());
    for (cmd, note) in EXAMPLES {
        println!(
            "    {:<30}  {}",
            cmd.bright_blue(),
            format!("# {note}").bright_black()
        );
    }
    println!();

    println!("{}", "NOTES".yellow().bold());
    for note in NOTES {
        println!("    {} {}", "•".bright_magenta(), note.white());
    }
}

fn print_plain() {
    println!("clp - copy parts of files to the clipboard\n");

    println!("USAGE");
    println!("    clp [OPERATION] [MODIFIERS] [FILE]\n");

    println!("OPERATIONS & MODIFIERS");
    for Flag(flag, desc) in FLAGS {
        println!("    {flag:<FLAG_WIDTH$}  {desc}");
    }
    println!();

    println!("EXAMPLES");
    for (cmd, note) in EXAMPLES {
        println!("    {cmd:<30}  # {note}");
    }
    println!();

    println!("NOTES");
    for note in NOTES {
        println!("    - {note}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_flag_has_a_description() {
        assert!(!FLAGS.is_empty());
        for Flag(flag, desc) in FLAGS {
            assert!(!flag.is_empty());
            assert!(!desc.is_empty(), "{flag} has no description");
        }
    }

    #[test]
    fn every_example_uses_a_documented_flag() {
        // Guards against help text drifting from the parser: any short flag
        // mentioned in an example must exist in FLAGS.
        for (cmd, _) in EXAMPLES {
            for tok in cmd.split_whitespace() {
                if let Some(rest) = tok.strip_prefix('-') {
                    if rest.is_empty() || rest.starts_with('-') {
                        continue;
                    }
                    let needle = format!("-{rest}");
                    let known = FLAGS
                        .iter()
                        .any(|Flag(f, _)| f.starts_with(&needle) || f.contains(&needle));
                    assert!(known, "example {cmd:?} uses undocumented flag {needle:?}");
                }
            }
        }
    }

    #[test]
    fn flags_fit_the_alignment_column() {
        for Flag(flag, _) in FLAGS {
            assert!(
                flag.len() <= FLAG_WIDTH,
                "{flag:?} is {} chars, wider than the {FLAG_WIDTH}-char column",
                flag.len()
            );
        }
    }

    #[test]
    fn both_renderers_produce_output_without_panicking() {
        print_plain();
        print_colour();
    }

    #[test]
    fn notes_document_open_ranges_and_separator() {
        assert!(NOTES.iter().any(|n| n.contains("5:")));
        assert!(NOTES.iter().any(|n| n.contains("--")));
    }
}
