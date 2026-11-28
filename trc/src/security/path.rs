//! Canonical, symlink-safe path confinement.
//!
//! A requested path is interpreted relative to the sandbox root (absolute
//! prefixes are dropped), `.`/`..` are normalized lexically, and the result is
//! canonicalized so that symlinks are resolved before the containment check.
//! Both lexical escapes and symlink escapes are rejected.

use std::ffi::OsString;
use std::fmt;
use std::io;
use std::path::{Component, Path, PathBuf};

/// Why a path could not be confined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathEscape {
    /// The sandbox root does not exist or cannot be canonicalized.
    InvalidRoot,
    /// The request climbed above its own root before joining the sandbox.
    ParentTraversal,
    /// The joined path could not be resolved.
    Unresolved(PathBuf),
    /// The canonical path resolved outside the sandbox root.
    OutsideRoot {
        canonical: PathBuf,
        root: PathBuf,
    },
}

impl fmt::Display for PathEscape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathEscape::InvalidRoot => write!(f, "sandbox root does not exist"),
            PathEscape::ParentTraversal => write!(f, "path escapes its own root via '..'"),
            PathEscape::Unresolved(path) => {
                write!(f, "path could not be resolved: {}", path.display())
            }
            PathEscape::OutsideRoot { canonical, root } => write!(
                f,
                "path '{}' resolves outside sandbox root '{}'",
                canonical.display(),
                root.display()
            ),
        }
    }
}

impl std::error::Error for PathEscape {}

/// Resolve `requested` inside `root`, rejecting any escape.
///
/// The returned path is canonical for existing entries; for a not-yet-created
/// leaf, the nearest existing ancestor is canonicalized and the missing suffix
/// is appended, which keeps writes inside the sandbox.
pub fn confine_path(root: &Path, requested: &Path) -> Result<PathBuf, PathEscape> {
    let root_real = std::fs::canonicalize(root).map_err(|_| PathEscape::InvalidRoot)?;
    let relative = normalize_relative(requested)?;
    let candidate = root_real.join(relative);
    let canonical = canonicalize_with_missing(&candidate)
        .map_err(|_| PathEscape::Unresolved(candidate.clone()))?;
    if canonical.starts_with(&root_real) {
        Ok(canonical)
    } else {
        Err(PathEscape::OutsideRoot {
            canonical,
            root: root_real,
        })
    }
}

/// Drop drive/root prefixes and fold `.`/`..` lexically. A `..` that would pop
/// past the first segment is an escape rather than something to pass through
/// to the filesystem.
fn normalize_relative(requested: &Path) -> Result<PathBuf, PathEscape> {
    let mut parts: Vec<OsString> = Vec::new();
    for component in requested.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::CurDir => {}
            Component::ParentDir => {
                if parts.pop().is_none() {
                    return Err(PathEscape::ParentTraversal);
                }
            }
            Component::Normal(segment) => parts.push(segment.to_os_string()),
        }
    }
    let mut normalized = PathBuf::new();
    for part in parts {
        normalized.push(part);
    }
    Ok(normalized)
}

/// Canonicalize a path whose leaf (or trailing components) may not exist yet.
fn canonicalize_with_missing(path: &Path) -> io::Result<PathBuf> {
    match std::fs::canonicalize(path) {
        Ok(canonical) => Ok(canonical),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            let parent = match path.parent() {
                Some(parent) if parent != path => parent,
                _ => return Err(err),
            };
            let mut resolved = canonicalize_with_missing(parent)?;
            match path.file_name() {
                Some(name) => {
                    resolved.push(name);
                    Ok(resolved)
                }
                None => Err(err),
            }
        }
        Err(err) => Err(err),
    }
}
