//! Command-line parsing for `clp`.
//!
//! A single pass over argv produces a [`Cli`] with one [`Operation`], any number of
//! [`Modifier`]s, and an explicit [`Source`]. Operations are mutually exclusive;
//! modifiers compose with any operation.

use anyhow::{Result, bail};
use std::fmt;
use std::path::PathBuf;

/// Where input content comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Stdin,
    Path(PathBuf),
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::Stdin => write!(f, "<stdin>"),
            Source::Path(p) => write!(f, "{}", p.display()),
        }
    }
}

/// Flags that change *how* content is transformed or written. These compose freely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    /// Append to the existing clipboard content instead of replacing it.
    Append,
    /// Print a summary of what was copied.
    Verbose,
    /// Trim leading and trailing whitespace.
    Trim,
}

impl fmt::Display for Modifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Modifier::Append => "-a",
            Modifier::Verbose => "-v",
            Modifier::Trim => "-t",
        };
        f.write_str(s)
    }
}

/// The single content-producing (or clipboard-consuming) operation requested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// Whole file or stdin (default).
    Copy,
    /// Copy up to the first of any delimiter. `inclusive` selects `-i` over `-u`.
    Until {
        delimiters: Vec<String>,
        inclusive: bool,
    },
    /// Copy the first `count` lines.
    Lines { count: usize },
    /// Copy an inclusive 1-indexed line range.
    Range { start: usize, end: usize },
    /// Write the clipboard to stdout.
    Paste,
    /// Empty the clipboard.
    Clear,
}

/// Parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cli {
    pub operation: Operation,
    pub modifiers: Vec<Modifier>,
    pub source: Source,
}

impl Cli {
    pub fn has(&self, m: Modifier) -> bool {
        self.modifiers.contains(&m)
    }
}

/// Set the operation, erroring if one was already chosen.
///
/// The error names both flags actually passed, rather than a fixed example list.
fn set_operation(
    slot: &mut Option<(&'static str, Operation)>,
    flag: &'static str,
    op: Operation,
) -> Result<()> {
    if let Some((prev, _)) = slot {
        bail!("Flags {prev} and {flag} are mutually exclusive; use only one");
    }
    *slot = Some((flag, op));
    Ok(())
}

/// Read the value that follows a flag, rejecting a following flag as the value.
fn take_value<'a>(args: &'a [String], i: &mut usize, flag: &str, hint: &str) -> Result<&'a String> {
    let v = args
        .get(*i + 1)
        .ok_or_else(|| anyhow::anyhow!("{flag} requires {hint}"))?;
    if v.starts_with('-') && v != "-" {
        bail!("{flag} requires {hint}, but got the flag {v:?}");
    }
    *i += 1;
    Ok(v)
}

/// Parse an inclusive line range like `5:10`.
///
/// Accepts open-ended forms: `5:` means 5 to end, `:10` means start to 10.
pub fn parse_range(spec: &str) -> Result<(usize, usize)> {
    let (l, r) = spec.split_once(':').ok_or_else(|| {
        anyhow::anyhow!("Invalid range {spec:?}: expected <start>:<end>, e.g. 5:10")
    })?;

    let parse = |s: &str, which: &str| -> Result<usize> {
        s.trim()
            .parse::<usize>()
            .map_err(|e| anyhow::anyhow!("Invalid {which} line {s:?}: {e}"))
            .and_then(|n| {
                if n == 0 {
                    bail!("{which} line must be greater than zero")
                }
                Ok(n)
            })
    };

    let start = if l.trim().is_empty() {
        1
    } else {
        parse(l, "start")?
    };
    let end = if r.trim().is_empty() {
        usize::MAX
    } else {
        parse(r, "end")?
    };

    if end < start {
        bail!("End line {end} is before start line {start}");
    }
    Ok((start, end))
}

/// Parse argv (excluding argv[0]).
pub fn parse(args: &[String]) -> Result<Cli> {
    let mut op: Option<(&'static str, Operation)> = None;
    let mut modifiers: Vec<Modifier> = Vec::new();
    let mut positionals: Vec<String> = Vec::new();
    let mut after_sep = false;
    let mut saw_any_sep = false;
    // Resolved into the operation after the loop, once all positionals are known.
    let mut pending_until: Option<(bool, Vec<String>)> = None;
    let mut i = 0;

    while i < args.len() {
        let a = args[i].as_str();

        if !after_sep && a == "--" {
            after_sep = true;
            i += 1;
            continue;
        }

        let is_flag = !after_sep && a.starts_with('-') && a.len() > 1;
        if !is_flag {
            positionals.push(args[i].clone());
            i += 1;
            continue;
        }

        // Number of argv slots this flag occupies, including the flag itself.
        // Advanced after the match so each branch only states its own width.
        let flag_idx = i;
        let mut consumed = 1usize;

        match a {
            "-u" | "-i" => {
                // Delimiters run until `--` or the next flag. Which of these is the
                // file cannot be decided here, because a later positional or flag
                // may still follow; it is resolved after the loop.
                let mut delimiters = Vec::new();
                i += 1;
                while i < args.len() && args[i] != "--" && !args[i].starts_with('-') {
                    delimiters.push(args[i].clone());
                    i += 1;
                }
                if i < args.len() && args[i] == "--" {
                    saw_any_sep = true;
                    i += 1;
                    after_sep = true;
                }
                consumed = i - flag_idx;
                pending_until = Some((a == "-i", delimiters));
            }

            "-l" => {
                let v = take_value(args, &mut i, "-l", "a line count")?;
                let count: usize = v
                    .parse()
                    .map_err(|e| anyhow::anyhow!("Invalid line count {v:?}: {e}"))?;
                if count == 0 {
                    bail!("Line count must be greater than zero");
                }
                consumed = 2;
                set_operation(&mut op, "-l", Operation::Lines { count })?;
            }

            "-r" => {
                let v = take_value(args, &mut i, "-r", "a range like <start>:<end>")?;
                let (start, end) = parse_range(v)?;
                consumed = 2;
                set_operation(&mut op, "-r", Operation::Range { start, end })?;
            }

            "-p" => set_operation(&mut op, "-p", Operation::Paste)?,
            "-c" => set_operation(&mut op, "-c", Operation::Clear)?,

            "-a" => modifiers.push(Modifier::Append),
            "-v" => modifiers.push(Modifier::Verbose),
            "-t" => modifiers.push(Modifier::Trim),

            other => bail!("Unknown flag: {other:?}\nRun `clp --help` for usage"),
        }

        // -u/-i already walked `i` past their delimiters, so honour where it ended.
        i = flag_idx + consumed;
    }

    if positionals.len() > 1 {
        let extra: Vec<&str> = positionals[1..].iter().map(|s| s.as_str()).collect();
        bail!(
            "Expected at most one file, got {}: {}\nRun `clp --help` for usage",
            positionals.len(),
            extra.join(", ")
        );
    }

    // With -u/-i and no `--`, the final argument is the file rather than a delimiter,
    // so `clp -u END f.txt` reads delimiter "END" from "f.txt".
    if let Some((inclusive, mut delimiters)) = pending_until {
        if !saw_any_sep && positionals.is_empty() {
            if let Some(file) = delimiters.pop() {
                positionals.push(file);
            }
        }
        if delimiters.is_empty() {
            bail!(
                "{} requires at least one delimiter argument",
                if inclusive { "-i" } else { "-u" }
            );
        }
        op = Some((
            if inclusive { "-i" } else { "-u" },
            Operation::Until {
                delimiters,
                inclusive,
            },
        ));
    }

    // -p and -c address the clipboard directly and take no input.
    if let Some((flag, _)) = &op {
        if matches!(
            op.as_ref().map(|(_, o)| o),
            Some(Operation::Paste) | Some(Operation::Clear)
        ) && !positionals.is_empty()
        {
            bail!("{flag} does not take a file argument");
        }
    }
    // -a (append) makes no sense against -c (clear): there is nothing to append to.
    if modifiers.contains(&Modifier::Append)
        && matches!(op.as_ref().map(|(_, o)| o), Some(Operation::Clear))
    {
        bail!("Flags -c and -a are mutually exclusive; use only one");
    }

    let source = match positionals.first() {
        Some(p) => Source::Path(PathBuf::from(p)),
        None => Source::Stdin,
    };

    Ok(Cli {
        operation: op.map(|(_, o)| o).unwrap_or(Operation::Copy),
        modifiers,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_args_is_copy_from_stdin() {
        let c = parse(&[]).unwrap();
        assert_eq!(c.operation, Operation::Copy);
        assert_eq!(c.source, Source::Stdin);
        assert!(c.modifiers.is_empty());
    }

    #[test]
    fn single_file_is_copy() {
        let c = parse(&v(&["f.txt"])).unwrap();
        assert_eq!(c.operation, Operation::Copy);
        assert_eq!(c.source, Source::Path(PathBuf::from("f.txt")));
    }

    #[test]
    fn rejects_unknown_flag() {
        let e = parse(&v(&["-L", "2", "f.txt"])).unwrap_err();
        assert!(e.to_string().contains("Unknown flag"), "{e}");
    }

    #[test]
    fn rejects_multiple_files() {
        let e = parse(&v(&["a.txt", "b.txt"])).unwrap_err();
        assert!(e.to_string().contains("at most one file"), "{e}");
    }

    #[test]
    fn conflict_message_names_both_real_flags() {
        let e = parse(&v(&["-r", "1:2", "-l", "5", "f.txt"])).unwrap_err();
        let msg = e.to_string();
        assert!(msg.contains("-r") && msg.contains("-l"), "{msg}");
        assert!(!msg.contains("-u"), "{msg}");
    }

    #[test]
    fn until_takes_delims_then_sep_then_file() {
        let c = parse(&v(&["-u", "END", "STOP", "--", "f.txt"])).unwrap();
        assert_eq!(
            c.operation,
            Operation::Until {
                delimiters: v(&["END", "STOP"]),
                inclusive: false
            }
        );
        assert_eq!(c.source, Source::Path(PathBuf::from("f.txt")));
    }

    #[test]
    fn until_without_sep_treats_last_arg_as_file() {
        let c = parse(&v(&["-u", "END", "f.txt"])).unwrap();
        assert_eq!(
            c.operation,
            Operation::Until {
                delimiters: v(&["END"]),
                inclusive: false
            }
        );
        assert_eq!(c.source, Source::Path(PathBuf::from("f.txt")));
    }

    #[test]
    fn inclusive_is_distinct_from_until() {
        let c = parse(&v(&["-i", "END", "f.txt"])).unwrap();
        assert_eq!(
            c.operation,
            Operation::Until {
                delimiters: v(&["END"]),
                inclusive: true
            }
        );
    }

    #[test]
    fn modifiers_compose_with_operations() {
        let c = parse(&v(&["-u", "END", "-a", "-t", "f.txt"])).unwrap();
        assert!(matches!(c.operation, Operation::Until { .. }));
        assert!(c.has(Modifier::Append));
        assert!(c.has(Modifier::Trim));
        assert!(!c.has(Modifier::Verbose));
    }

    #[test]
    fn modifier_before_operation_no_longer_breaks_source() {
        // Regression for the old `args.get(1)` bug: -t used to be read as the filename.
        let c = parse(&v(&["-t", "-a", "f.txt"])).unwrap();
        assert_eq!(c.source, Source::Path(PathBuf::from("f.txt")));
        assert!(c.has(Modifier::Trim) && c.has(Modifier::Append));
    }

    #[test]
    fn lines_parses_count() {
        let c = parse(&v(&["-l", "7", "f.txt"])).unwrap();
        assert_eq!(c.operation, Operation::Lines { count: 7 });
    }

    #[test]
    fn lines_rejects_zero() {
        assert!(parse(&v(&["-l", "0", "f.txt"])).is_err());
    }

    #[test]
    fn lines_rejects_flag_as_count() {
        let e = parse(&v(&["-l", "-t", "f.txt"])).unwrap_err();
        assert!(e.to_string().contains("requires a line count"), "{e}");
    }

    #[test]
    fn range_parses() {
        let c = parse(&v(&["-r", "2:5", "f.txt"])).unwrap();
        assert_eq!(c.operation, Operation::Range { start: 2, end: 5 });
    }

    #[test]
    fn range_open_ended() {
        assert_eq!(parse_range("5:").unwrap(), (5, usize::MAX));
        assert_eq!(parse_range(":10").unwrap(), (1, 10));
    }

    #[test]
    fn range_rejects_zero_and_inverted() {
        assert!(parse_range("0:5").is_err());
        assert!(parse_range("5:1").is_err());
    }

    #[test]
    fn paste_and_clear_take_no_file() {
        let c = parse(&v(&["-p"])).unwrap();
        assert_eq!(c.operation, Operation::Paste);
        assert!(parse(&v(&["-p", "f.txt"])).is_err());
        assert!(parse(&v(&["-c", "f.txt"])).is_err());
    }

    #[test]
    fn clear_conflicts_with_append() {
        assert!(parse(&v(&["-c", "-a"])).is_err());
    }

    #[test]
    fn missing_flag_value_is_clear() {
        assert!(parse(&v(&["-l"])).is_err());
        assert!(parse(&v(&["-r"])).is_err());
        assert!(parse(&v(&["-u"])).is_err());
    }

    #[test]
    fn double_dash_allows_leading_dash_filename() {
        let c = parse(&v(&["--", "-weird.txt"])).unwrap();
        assert_eq!(c.source, Source::Path(PathBuf::from("-weird.txt")));
    }
}
