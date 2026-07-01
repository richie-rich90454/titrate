//! System linker invocation.
//!
//! Links the LLVM object file with `libtitrate_native` (and libc) to produce
//! a runnable executable.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Link the object file into a native executable.
///
/// `object_path`   – the `.o` / `.obj` file produced by the LLVM backend
/// `output_exe`    – the final executable path
/// `native_lib_dir` – directory containing `libtitrate_native.a` /
///                    `titrate_native.lib`
/// `extra_link_libs` – extra libraries to link, taken from the `[native]`
///                    section of `Titrate.toml`. Each entry is rendered as
///                    `-l<lib>` on Unix (clang/gcc) or `<lib>.lib` for
///                    `link.exe` on Windows.
/// `extra_link_args` – raw linker flags appended verbatim (e.g.
///                    `-L/usr/local/lib`).
pub fn link(
    object_path: &Path,
    output_exe: &Path,
    native_lib_dir: &Path,
    extra_link_libs: &[String],
    extra_link_args: &[String],
) -> Result<(), String> {
    #[cfg(windows)]
    {
        link_windows(
            object_path,
            output_exe,
            native_lib_dir,
            extra_link_libs,
            extra_link_args,
        )
    }
    #[cfg(not(windows))]
    {
        link_unix(
            object_path,
            output_exe,
            native_lib_dir,
            extra_link_libs,
            extra_link_args,
        )
    }
}

#[cfg(windows)]
fn link_windows(
    object_path: &Path,
    output_exe: &Path,
    native_lib_dir: &Path,
    extra_link_libs: &[String],
    extra_link_args: &[String],
) -> Result<(), String> {
    // Prefer clang (it drives link.exe transparently and is shipped with LLVM),
    // then fall back to link.exe, then to gcc.
    if let Some(clang) = find_executable("clang") {
        return link_with_clang_windows(
            &clang,
            object_path,
            output_exe,
            native_lib_dir,
            extra_link_libs,
            extra_link_args,
        );
    }
    if let Some(link_exe) = find_executable("link") {
        return link_with_link_exe(
            &link_exe,
            object_path,
            output_exe,
            native_lib_dir,
            extra_link_libs,
            extra_link_args,
        );
    }
    if let Some(gcc) = find_executable("gcc") {
        return link_with_gcc_windows(
            &gcc,
            object_path,
            output_exe,
            native_lib_dir,
            extra_link_libs,
            extra_link_args,
        );
    }
    Err("linker: no suitable linker found (tried clang, link, gcc)".to_string())
}

#[cfg(windows)]
fn link_with_clang_windows(
    clang: &Path,
    object_path: &Path,
    output_exe: &Path,
    native_lib_dir: &Path,
    extra_link_libs: &[String],
    extra_link_args: &[String],
) -> Result<(), String> {
    let args = build_gcc_style_args(
        object_path,
        output_exe,
        native_lib_dir,
        extra_link_libs,
        extra_link_args,
        &[], // clang on Windows gets no implicit -lm/-lpthread
    );
    let mut cmd = Command::new(clang);
    cmd.args(&args);
    run_link_command(cmd)
}

#[cfg(windows)]
fn link_with_link_exe(
    link_exe: &Path,
    object_path: &Path,
    output_exe: &Path,
    native_lib_dir: &Path,
    extra_link_libs: &[String],
    extra_link_args: &[String],
) -> Result<(), String> {
    let args = build_link_exe_args(
        object_path,
        output_exe,
        native_lib_dir,
        extra_link_libs,
        extra_link_args,
    );
    let mut cmd = Command::new(link_exe);
    cmd.args(&args);
    run_link_command(cmd)
}

#[cfg(windows)]
fn link_with_gcc_windows(
    gcc: &Path,
    object_path: &Path,
    output_exe: &Path,
    native_lib_dir: &Path,
    extra_link_libs: &[String],
    extra_link_args: &[String],
) -> Result<(), String> {
    let args = build_gcc_style_args(
        object_path,
        output_exe,
        native_lib_dir,
        extra_link_libs,
        extra_link_args,
        &[],
    );
    let mut cmd = Command::new(gcc);
    cmd.args(&args);
    run_link_command(cmd)
}

#[cfg(not(windows))]
fn link_unix(
    object_path: &Path,
    output_exe: &Path,
    native_lib_dir: &Path,
    extra_link_libs: &[String],
    extra_link_args: &[String],
) -> Result<(), String> {
    let cc = find_executable("cc")
        .or_else(|| find_executable("clang"))
        .or_else(|| find_executable("gcc"))
        .ok_or("linker: no cc/clang/gcc found")?;

    let args = build_gcc_style_args(
        object_path,
        output_exe,
        native_lib_dir,
        extra_link_libs,
        extra_link_args,
        &["titrate_native", "m", "pthread"],
    );
    let mut cmd = Command::new(&cc);
    cmd.args(&args);
    run_link_command(cmd)
}

// ---- Linker argument construction ----------------------------------------
//
// These builders are pure functions returning `Vec<String>` so that the
// command construction can be unit-tested without actually invoking the
// system linker. `build_gcc_style_args` is shared by the Unix `cc`/`clang`/
// `gcc` driver and the Windows `clang`/`gcc` driver — only the implicit
// built-in libraries differ (Unix links `m` and `pthread` by default).

/// Build a gcc/clang-style argument vector.
///
/// `builtin_libs` are the libraries always linked (e.g. `titrate_native`,
/// `m`, `pthread` on Unix) and are rendered as `-l<lib>` before the
/// user-supplied `extra_link_libs`. Each `extra_link_libs` entry becomes
/// `-l<lib>`; each `extra_link_args` entry is appended verbatim.
fn build_gcc_style_args(
    object_path: &Path,
    output_exe: &Path,
    native_lib_dir: &Path,
    extra_link_libs: &[String],
    extra_link_args: &[String],
    builtin_libs: &[&str],
) -> Vec<String> {
    let mut args = Vec::new();
    args.push(object_path.display().to_string());
    args.push("-o".to_string());
    args.push(output_exe.display().to_string());
    args.push(format!("-L{}", native_lib_dir.display()));
    for lib in builtin_libs {
        args.push(format!("-l{}", lib));
    }
    for lib in extra_link_libs {
        args.push(format!("-l{}", lib));
    }
    for arg in extra_link_args {
        args.push(arg.clone());
    }
    args
}

/// Build the argument vector for MSVC `link.exe`.
///
/// Extra libraries become `<lib>.lib` and extra args are appended verbatim.
#[cfg(windows)]
fn build_link_exe_args(
    object_path: &Path,
    output_exe: &Path,
    native_lib_dir: &Path,
    extra_link_libs: &[String],
    extra_link_args: &[String],
) -> Vec<String> {
    let mut args = Vec::new();
    args.push(object_path.display().to_string());
    args.push(format!("/OUT:{}", output_exe.display()));
    args.push(format!("/LIBPATH:{}", native_lib_dir.display()));
    args.push("titrate_native.lib".to_string());
    // Default Windows runtime + C runtime.
    args.push("kernel32.lib".to_string());
    args.push("libcmt.lib".to_string());
    for lib in extra_link_libs {
        args.push(format!("{}.lib", lib));
    }
    for arg in extra_link_args {
        args.push(arg.clone());
    }
    args
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gcc_style_args_include_built_in_libs() {
        let obj = Path::new("a.o");
        let exe = Path::new("out");
        let lib_dir = Path::new("/x/lib");
        let args = build_gcc_style_args(
            obj,
            exe,
            lib_dir,
            &[],
            &[],
            &["titrate_native", "m", "pthread"],
        );
        assert!(args.iter().any(|a| a == "-ltitrate_native"));
        assert!(args.iter().any(|a| a == "-lm"));
        assert!(args.iter().any(|a| a == "-lpthread"));
        assert!(args.iter().any(|a| a == "-o"));
        assert!(args.iter().any(|a| a == "-L/x/lib"));
    }

    #[test]
    fn gcc_style_args_append_extra_libs_and_args_verbatim() {
        let obj = Path::new("a.o");
        let exe = Path::new("out");
        let lib_dir = Path::new("/x/lib");
        let extra_libs = vec!["ssl".to_string(), "crypto".to_string()];
        let extra_args = vec![
            "-L/usr/local/lib".to_string(),
            "-Wl,-rpath,/opt/lib".to_string(),
        ];
        let args = build_gcc_style_args(obj, exe, lib_dir, &extra_libs, &extra_args, &[]);
        assert!(args.iter().any(|a| a == "-lssl"));
        assert!(args.iter().any(|a| a == "-lcrypto"));
        assert!(args.iter().any(|a| a == "-L/usr/local/lib"));
        assert!(args.iter().any(|a| a == "-Wl,-rpath,/opt/lib"));
        // Extra args come after extra libs.
        let crypto_idx = args
            .iter()
            .position(|a| a == "-lcrypto")
            .expect("-lcrypto present");
        let rpath_idx = args
            .iter()
            .position(|a| a == "-Wl,-rpath,/opt/lib")
            .expect("rpath arg present");
        assert!(rpath_idx > crypto_idx);
    }

    #[test]
    fn gcc_style_args_empty_extras_match_legacy_unix_behavior() {
        let obj = Path::new("a.o");
        let exe = Path::new("out");
        let lib_dir = Path::new("/x/lib");
        let args = build_gcc_style_args(
            obj,
            exe,
            lib_dir,
            &[],
            &[],
            &["titrate_native", "m", "pthread"],
        );
        // Without [native], only the built-in flags should be present.
        assert_eq!(
            args,
            vec![
                "a.o".to_string(),
                "-o".to_string(),
                "out".to_string(),
                "-L/x/lib".to_string(),
                "-ltitrate_native".to_string(),
                "-lm".to_string(),
                "-lpthread".to_string(),
            ]
        );
    }

    #[cfg(windows)]
    #[test]
    fn link_exe_args_render_extra_libs_as_dot_lib() {
        let obj = Path::new("a.obj");
        let exe = Path::new("out.exe");
        let lib_dir = Path::new("C:\\lib");
        let extra_libs = vec!["ws2_32".to_string()];
        let args = build_link_exe_args(obj, exe, lib_dir, &extra_libs, &[]);
        assert!(args.iter().any(|a| a == "ws2_32.lib"));
        assert!(args.iter().any(|a| a == "titrate_native.lib"));
    }
}
