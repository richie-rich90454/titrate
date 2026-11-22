//! Shared helpers for test working directories.
//!
//! Several test programs read and write files. Running them with a working
//! directory inside the repository mutates tracked fixtures on every run and
//! leaves stray artifacts behind. These helpers copy the small fixture tree
//! into the system temporary directory and return that isolated path instead.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

fn copy_tree(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("create sandbox {}: {}", dst.display(), e))?;
    for entry in fs::read_dir(src).map_err(|e| format!("read {}: {}", src.display(), e))? {
        let entry = entry.map_err(|e| format!("read directory entry: {}", e))?;
        let name = entry.file_name();
        let target = dst.join(&name);
        let kind = entry
            .file_type()
            .map_err(|e| format!("file type {}: {}", entry.path().display(), e))?;
        if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)
                .map_err(|e| format!("copy {}: {}", entry.path().display(), e))?;
        }
    }
    Ok(())
}

/// Create an isolated copy of `fixture_dir` under the system temp directory.
pub fn isolated_copy(fixture_dir: &Path, label: &str) -> Result<PathBuf, String> {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    // Keep sandboxes under the workspace's ignored `target/` tree. Besides
    // avoiding tracked directories, this preserves the repository-relative
    // `lib/tt/data` probes used by the standard library's DataFile loader.
    let dir = Path::new("../target/test_sandboxes").join(format!(
        "{}_{}_{}",
        std::process::id(),
        label,
        id
    ));
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| format!("clear sandbox: {}", e))?;
    }
    copy_tree(fixture_dir, &dir)?;
    Ok(fs::canonicalize(&dir).map_err(|e| format!("canonicalize sandbox: {}", e))?)
}

/// Remove a sandbox created by `isolated_copy`.
pub fn cleanup(path: &Path) {
    let _ = fs::remove_dir_all(path);
}
