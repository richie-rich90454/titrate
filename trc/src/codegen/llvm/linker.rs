//! System linker invocation.
//!
//! Links the LLVM object file with `libtitrate_native` (and libc) to produce
//! a runnable executable.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Link the object file into a native executable.
///
/// `object_path`  – the `.o` / `.obj` file produced by the LLVM backend
/// `output_exe`   – the final executable path
/// `native_lib_dir` – directory containing `libtitrate_native.a` /
///                    `titrate_native.lib`
pub fn link(
    object_path: &Path,
    output_exe: &Path,
    native_lib_dir: &Path,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        link_windows(object_path, output_exe, native_lib_dir)
    }
    #[cfg(not(windows))]
    {
        link_unix(object_path, output_exe, native_lib_dir)
    }
}

#[cfg(windows)]
fn link_windows(
    object_path: &Path,
    output_exe: &Path,
    native_lib_dir: &Path,
) -> Result<(), String> {
    // Prefer clang (it drives link.exe transparently and is shipped with LLVM),
    // then fall back to link.exe, then to gcc.
    if let Some(clang) = find_executable("clang") {
        return link_with_clang_windows(&clang, object_path, output_exe, native_lib_dir);
    }
    if let Some(link_exe) = find_executable("link") {
        return link_with_link_exe(&link_exe, object_path, output_exe, native_lib_dir);
    }
    if let Some(gcc) = find_executable("gcc") {
        return link_with_gcc_windows(&gcc, object_path, output_exe, native_lib_dir);
    }
    Err("linker: no suitable linker found (tried clang, link, gcc)".to_string())
}

#[cfg(windows)]
fn link_with_clang_windows(
    clang: &Path,
    object_path: &Path,
    output_exe: &Path,
    native_lib_dir: &Path,
) -> Result<(), String> {
    let mut cmd = Command::new(clang);
    cmd.arg(object_path)
        .arg("-o").arg(output_exe)
        .arg(format!("-L{}", native_lib_dir.display()))
        .arg("-ltitrate_native");

    run_link_command(cmd)
}

#[cfg(windows)]
fn link_with_link_exe(
    link_exe: &Path,
    object_path: &Path,
    output_exe: &Path,
    native_lib_dir: &Path,
) -> Result<(), String> {
    let mut cmd = Command::new(link_exe);
    cmd.arg(object_path)
        .arg(format!("/OUT:{}", output_exe.display()))
        .arg(format!("/LIBPATH:{}", native_lib_dir.display()))
        .arg("titrate_native.lib")
        // Default Windows runtime + C runtime.
        .arg("kernel32.lib")
        .arg("libcmt.lib");

    run_link_command(cmd)
}

#[cfg(windows)]
fn link_with_gcc_windows(
    gcc: &Path,
    object_path: &Path,
    output_exe: &Path,
    native_lib_dir: &Path,
) -> Result<(), String> {
    let mut cmd = Command::new(gcc);
    cmd.arg(object_path)
        .arg("-o").arg(output_exe)
        .arg(format!("-L{}", native_lib_dir.display()))
        .arg("-ltitrate_native");

    run_link_command(cmd)
}

#[cfg(not(windows))]
fn link_unix(
    object_path: &Path,
    output_exe: &Path,
    native_lib_dir: &Path,
) -> Result<(), String> {
    let cc = find_executable("cc")
        .or_else(|| find_executable("clang"))
        .or_else(|| find_executable("gcc"))
        .ok_or("linker: no cc/clang/gcc found")?;

    let mut cmd = Command::new(&cc);
    cmd.arg(object_path)
        .arg("-o").arg(output_exe)
        .arg(format!("-L{}", native_lib_dir.display()))
        .arg("-ltitrate_native")
        .arg("-lm")
        .arg("-lpthread");

    run_link_command(cmd)
}

fn run_link_command(mut cmd: Command) -> Result<(), String> {
    let output = cmd
        .output()
        .map_err(|e| format!("failed to invoke linker: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "linker failed (exit {:?}):\nstdout: {}\nstderr: {}",
            output.status.code(),
            stdout,
            stderr
        ));
    }
    Ok(())
}

/// Search PATH for an executable.
fn find_executable(name: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let exe_name = if name.ends_with(".exe") {
            name.to_string()
        } else {
            format!("{}.exe", name)
        };
        which(&exe_name)
    }
    #[cfg(not(windows))]
    {
        which(name)
    }
}

fn which(name: &str) -> Option<PathBuf> {
    let path_env = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_env) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
