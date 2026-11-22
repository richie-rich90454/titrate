//! Shared runtime capability detection for native-codegen integration tests.
//!
//! Native tests no longer use `#[ignore]`. Each one calls `native_ready` at
//! the start. When LLVM development files or the required `titrate_native`
//! static library are missing, the helper prints a clear warning and the test
//! returns early. On a configured machine the full test runs and any mismatch
//! fails normally.

use std::env;
use std::path::{Path, PathBuf};

/// What a native test needs before it can run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibProfile {
    /// IR or object emission only: LLVM dev files, no linker or runtime lib.
    LlvmOnly,
    /// Plain `trc --native` links the debug runtime library.
    Debug,
    /// `trc --native --release` links the release runtime library.
    Release,
}

/// Locate the workspace root by walking up from `CARGO_MANIFEST_DIR`.
pub fn workspace_root() -> PathBuf {
    let manifest = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set");
    PathBuf::from(manifest)
        .parent()
        .map(|p| p.to_path_buf())
        .expect("trc should be inside the workspace")
}

fn native_lib_name() -> &'static str {
    if cfg!(windows) {
        "titrate_native.lib"
    } else {
        "libtitrate_native.a"
    }
}

fn native_lib_present(root: &Path, profile: LibProfile) -> bool {
    let dir = match profile {
        LibProfile::LlvmOnly => return true,
        LibProfile::Debug => "debug",
        LibProfile::Release => "release",
    };
    root.join("target").join(dir).join(native_lib_name()).is_file()
}

#[cfg(windows)]
fn default_llvm_prefix() -> Option<PathBuf> {
    let candidate = PathBuf::from(r"C:\Program Files\LLVM");
    if candidate.join("lib").is_dir() {
        Some(candidate)
    } else {
        None
    }
}

#[cfg(not(windows))]
fn default_llvm_prefix() -> Option<PathBuf> {
    None
}

/// Mirror `trc/build.rs`: the configured prefix, then the Windows default.
fn llvm_prefix() -> Option<PathBuf> {
    if let Some(value) = env::var_os("LLVM_SYS_221_PREFIX") {
        let candidate = PathBuf::from(value);
        if candidate.join("lib").is_dir() {
            return Some(candidate);
        }
    }
    default_llvm_prefix()
}

fn llvm_c_library_present(prefix: &Path) -> bool {
    let lib_dir = prefix.join("lib");
    if cfg!(windows) {
        lib_dir.join("LLVM-C.lib").is_file()
    } else {
        std::fs::read_dir(&lib_dir).is_ok_and(|entries| {
            entries.flatten().any(|entry| {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                name == "libLLVM-C.so"
                    || name == "libLLVM-C.dylib"
                    || name.starts_with("libLLVM-C.so.")
            })
        })
    }
}

/// Return true when the native toolchain is ready. Otherwise print a warning
/// naming every missing prerequisite and return false.
pub fn native_ready(profile: LibProfile) -> bool {
    let mut missing = Vec::new();

    match llvm_prefix() {
        Some(prefix) if llvm_c_library_present(&prefix) => {}
        _ => missing.push(
            "LLVM 22.1 development files not found (set LLVM_SYS_221_PREFIX or install LLVM)"
                .to_string(),
        ),
    }

    let lib_requirement = match profile {
        LibProfile::LlvmOnly => None,
        LibProfile::Debug => Some(("debug", "cargo build -p titrate_native")),
        LibProfile::Release => Some(("release", "cargo build -p titrate_native --release")),
    };
    if let Some((label, build_cmd)) = lib_requirement {
        if !native_lib_present(&workspace_root(), profile) {
            missing.push(format!(
                "titrate_native {} library not found (run `{}`)",
                label, build_cmd
            ));
        }
    }

    if missing.is_empty() {
        return true;
    }
    eprintln!("WARNING: skipping native test: {}", missing.join("; "));
    false
}
