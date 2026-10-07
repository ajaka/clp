//! Shared test helpers.
//!
//! One `tempfile`-backed temporary directory per test, cleaned up automatically
//! (including on panic), and no hardcoded `/tmp` literals so the suite runs on
//! Windows and macOS as well as Linux.

use std::path::{Path, PathBuf};

/// A self-cleaning temporary directory.
pub struct TempDir(tempfile::TempDir);

impl TempDir {
    pub fn path(&self) -> &Path {
        self.0.path()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // tempfile::TempDir removes the directory recursively on drop, including
        // when the test panics.
    }
}

/// Create a temporary directory that is removed when dropped.
pub fn tempfile() -> TempDir {
    TempDir(tempfile::TempDir::new().expect("failed to create temp dir"))
}

/// Write `content` to `name` inside `dir`, returning the full path.
pub fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, content).expect("failed to write temp file");
    p
}

/// A path inside `dir` that is guaranteed not to exist.
#[allow(dead_code)]
pub fn nonexistent(dir: &Path) -> PathBuf {
    dir.join("this-file-does-not-exist.txt")
}
