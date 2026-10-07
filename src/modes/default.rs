use anyhow::Result;

use crate::cli::Source;
use crate::common;
use crate::modes::Selection;

/// Read the whole input.
pub fn read(source: &Source) -> Result<Selection> {
    let content = common::read_source_text(source)?;
    let bytes = content.len();
    Ok(Selection {
        content,
        label: format!("{bytes} bytes"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_whole_file_and_labels_it() {
        let dir = crate::testutil::tempfile();
        let path = dir.path().join("f.txt");
        std::fs::write(&path, "hello\n").unwrap();

        let sel = read(&Source::Path(path)).unwrap();
        assert_eq!(sel.content, "hello\n");
        assert_eq!(sel.label, "6 bytes");
    }

    #[test]
    fn empty_file_reads_empty() {
        let dir = crate::testutil::tempfile();
        let path = dir.path().join("empty.txt");
        std::fs::write(&path, "").unwrap();

        let sel = read(&Source::Path(path)).unwrap();
        assert_eq!(sel.content, "");
        assert_eq!(sel.label, "0 bytes");
    }

    #[test]
    fn unicode_byte_count_is_reported() {
        let dir = crate::testutil::tempfile();
        let path = dir.path().join("u.txt");
        std::fs::write(&path, "世界").unwrap();

        let sel = read(&Source::Path(path)).unwrap();
        assert_eq!(sel.content, "世界");
        // 2 chars, 6 bytes
        assert_eq!(sel.label, "6 bytes");
    }

    #[test]
    fn missing_file_errors() {
        let r = read(&Source::Path(std::path::PathBuf::from(
            "/nonexistent/x.txt",
        )));
        assert!(r.is_err());
    }
}
