use anyhow::Result;

use crate::cli::Source;
use crate::common;
use crate::modes::Selection;

/// Read the first `count` lines, preserving their original terminators.
pub fn read(source: &Source, count: usize) -> Result<Selection> {
    let content = common::read_lines(source, count)?;
    let actual = content.split_inclusive('\n').count();
    Ok(Selection {
        content,
        // Report what was actually copied, not what was requested (finding 012).
        label: format!("{actual} of {count} lines"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &crate::testutil::TempDir, name: &str, body: &str) -> Source {
        Source::Path(crate::testutil::write_file(dir.path(), name, body))
    }

    #[test]
    fn takes_first_n_lines() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "a.txt", "L1\nL2\nL3\nL4\nL5\n");

        let sel = read(&s, 3).unwrap();
        assert_eq!(sel.content, "L1\nL2\nL3\n");
        assert_eq!(sel.label, "3 of 3 lines");
    }

    #[test]
    fn reports_actual_when_fewer_lines_exist() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "b.txt", "L1\nL2\n");

        let sel = read(&s, 100).unwrap();
        assert_eq!(sel.content, "L1\nL2\n");
        // Regression for finding 012: must not claim 100 lines.
        assert_eq!(sel.label, "2 of 100 lines");
    }

    #[test]
    fn preserves_trailing_newline() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "c.txt", "L1\nL2\n");

        let sel = read(&s, 1).unwrap();
        assert_eq!(sel.content, "L1\n");
    }

    #[test]
    fn preserves_crlf() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "d.txt", "a\r\nb\r\nc\r\n");

        let sel = read(&s, 2).unwrap();
        // Regression for finding 015: CRLF must survive.
        assert_eq!(sel.content, "a\r\nb\r\n");
    }

    #[test]
    fn unterminated_final_line_is_included() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "e.txt", "L1\nL2");

        let sel = read(&s, 2).unwrap();
        assert_eq!(sel.content, "L1\nL2");
    }

    #[test]
    fn zero_count_is_rejected() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "f.txt", "L1\n");

        assert!(read(&s, 0).is_err());
    }

    #[test]
    fn empty_file_yields_empty_selection() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "g.txt", "");

        let sel = read(&s, 5).unwrap();
        assert_eq!(sel.content, "");
        assert_eq!(sel.label, "0 of 5 lines");
    }
}
