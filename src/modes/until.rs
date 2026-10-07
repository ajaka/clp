use anyhow::Result;

use crate::cli::Source;
use crate::common;
use crate::modes::Selection;

/// Read up to the first of any delimiter. When `inclusive`, keep the delimiter too.
pub fn read(source: &Source, delimiters: &[String], inclusive: bool) -> Result<Selection> {
    let full = common::read_source_text(source)?;
    let content = common::cut(&full, delimiters, inclusive);

    let names: Vec<&str> = delimiters.iter().map(|s| s.as_str()).collect();
    let word = if inclusive { "until" } else { "before" };
    let taken = content.len();
    Ok(Selection {
        content,
        label: format!(
            "{word} any of {} ({taken} of {} bytes)",
            names.join(", "),
            full.len()
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &crate::testutil::TempDir, name: &str, body: &str) -> Source {
        Source::Path(crate::testutil::write_file(dir.path(), name, body))
    }

    fn dels(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn exclusive_stops_before_delimiter() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "a.txt", "Hello World\nThis is a test\nEND\nMore\n");

        let sel = read(&s, &dels(&["END"]), false).unwrap();
        assert_eq!(sel.content, "Hello World\nThis is a test\n");
    }

    #[test]
    fn inclusive_keeps_delimiter() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "b.txt", "Hello World\nThis is a test\nEND\nMore\n");

        let sel = read(&s, &dels(&["END"]), true).unwrap();
        assert_eq!(sel.content, "Hello World\nThis is a test\nEND");
    }

    #[test]
    fn multiple_delimiters_pick_earliest() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "c.txt", "Hello World\nSTOP\nMore\nEND\n");

        let sel = read(&s, &dels(&["END", "STOP"]), false).unwrap();
        assert_eq!(sel.content, "Hello World\n");
    }

    #[test]
    fn delimiter_not_found_returns_everything() {
        let d = crate::testutil::tempfile();
        let body = "Hello World\n";
        let s = write(&d, "e.txt", body);

        let sel = read(&s, &dels(&["NOPE"]), false).unwrap();
        assert_eq!(sel.content, body);
    }

    #[test]
    fn empty_delimiter_list_returns_everything() {
        let d = crate::testutil::tempfile();
        let body = "Hello\n";
        let s = write(&d, "f.txt", body);

        let sel = read(&s, &[], false).unwrap();
        assert_eq!(sel.content, body);
    }

    #[test]
    fn delimiter_at_start_yields_empty() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "g.txt", "END\nHello\n");

        let sel = read(&s, &dels(&["END"]), false).unwrap();
        assert_eq!(sel.content, "");
    }

    #[test]
    fn mid_line_delimiter_cuts_at_byte_offset() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "h.txt", "prefixSTOPsuffix");

        let sel = read(&s, &dels(&["STOP"]), false).unwrap();
        assert_eq!(sel.content, "prefix");
    }

    #[test]
    fn multibyte_safe_with_utf8_delimiter() {
        let d = crate::testutil::tempfile();
        let s = write(&d, "i.txt", "前中後");

        let sel = read(&s, &dels(&["中"]), false).unwrap();
        assert_eq!(sel.content, "前");

        let sel = read(&s, &dels(&["中"]), true).unwrap();
        assert_eq!(sel.content, "前中");
    }
}
