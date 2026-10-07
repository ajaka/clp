use anyhow::Result;

use crate::cli::Source;
use crate::common;
use crate::modes::Selection;

/// Read an inclusive 1-indexed line range, preserving line terminators.
pub fn read(source: &Source, start: usize, end: usize) -> Result<Selection> {
    let content = common::read_line_range(source, start, end)?;
    let actual = content.split_inclusive('\n').count();
    let end_label = if end == usize::MAX {
        format!("{start}-end")
    } else {
        format!("{start}-{end}")
    };
    Ok(Selection {
        content,
        // Report what was actually copied, not what was requested (finding 012).
        label: format!("{actual} of lines {end_label}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &crate::testutil::TempDir, name: &str, body: &str) -> Source {
        Source::Path(crate::testutil::write_file(dir.path(), name, body))
    }

    #[test]
    fn reads_inclusive_range() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "a.txt", "L1\nL2\nL3\nL4\nL5\n");

        let sel = read(&s, 2, 4).unwrap();
        assert_eq!(sel.content, "L2\nL3\nL4\n");
        assert_eq!(sel.label, "3 of lines 2-4");
    }

    #[test]
    fn single_line_range() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "b.txt", "L1\nL2\nL3\n");

        let sel = read(&s, 2, 2).unwrap();
        assert_eq!(sel.content, "L2\n");
    }

    #[test]
    fn range_past_end_clamps_and_reports_actual() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "c.txt", "L1\nL2\n");

        let sel = read(&s, 1, 100).unwrap();
        assert_eq!(sel.content, "L1\nL2\n");
        // Regression for finding 012: must not claim 100 lines.
        assert_eq!(sel.label, "2 of lines 1-100");
    }

    #[test]
    fn open_ended_range_runs_to_end() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "d.txt", "L1\nL2\nL3\n");

        let sel = read(&s, 2, usize::MAX).unwrap();
        assert_eq!(sel.content, "L2\nL3\n");
        assert_eq!(sel.label, "2 of lines 2-end");
    }

    #[test]
    fn start_beyond_end_yields_empty() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "e.txt", "L1\nL2\n");

        let sel = read(&s, 99, 100).unwrap();
        assert_eq!(sel.content, "");
        assert_eq!(sel.label, "0 of lines 99-100");
    }

    #[test]
    fn start_zero_is_rejected_not_coerced() {
        // Regression for finding 011: library and CLI must agree that 0 is invalid.
        let d = crate::testutil::tempfile();
        let s = write(&d, "f.txt", "L1\nL2\n");

        assert!(read(&s, 0, 2).is_err());
    }

    #[test]
    fn inverted_range_is_rejected() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "g.txt", "L1\nL2\n");

        assert!(read(&s, 3, 1).is_err());
    }

    #[test]
    fn preserves_crlf() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "h.txt", "a\r\nb\r\nc\r\n");

        let sel = read(&s, 1, 2).unwrap();
        assert_eq!(sel.content, "a\r\nb\r\n");
    }
}
