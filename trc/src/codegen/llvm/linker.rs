//! System linker invocation.
//!
//! Links the LLVM object file with `libtitrate_native` (and libc) to produce
//! a runnable executable.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Windows system libraries required when linking `titrate_native.lib`.
///
/// `titrate_native.lib` is a static archive that embeds the entire Rust
/// standard library (because the `titrate_native` crate depends on `std`).
/// When Rust's `rustc` links a normal binary it adds these system libraries
/// automatically, but our custom linker must list them explicitly so that
/// the symbols referenced by std's object files (networking, NT API,
/// cryptography, user environment, C runtime) are resolved.
///
/// - `titrate_native` – the Titrate native runtime (always required).
/// - `ws2_32`         – Winsock (recv, send, bind, listen, connect, WSA*).
/// - `ntdll`          – NT API (NtOpenFile, NtCreateNamedPipeFile, NtWriteFile).
/// - `advapi32`       – Advanced API (security, registry).
/// - `userenv`        – User environment (Rust std env lookup).
/// - `bcrypt`         – Cryptography (Rust std randomness).
/// - `kernel32`       – Kernel (CreateFile, HeapAlloc, etc.).
/// - `ucrt`           – Universal C Runtime import library (realloc, strcspn,
///   _beginthreadex, _localtime64_s, _dclass, memcmp, ...).
/// - `vcruntime`      – Dynamic VC Runtime import library (__C_specific_handler,
///   _CxxThrowException, memcpy, memset, ...). Provides the
///   import thunks for symbols exported by vcruntime140.dll.
/// - `libcmt`         – Combined static C runtime. Provides the compiler-
///   runtime DATA symbols (`__security_cookie`, `_tls_used`,
///   `_fltused`, `__chkstk`, `__GSHandlerCheck`,
///   `__report_rangecheckfailure`, `type_info` vftable)
///   that are NOT exported from vcruntime140.dll and thus
///   absent from the dynamic `vcruntime.lib` import library.
///   Linking both `libcmt.lib` (static UCRT+VCRT) and
///   `ucrt.lib`/`vcruntime.lib` (dynamic import thunks)
///   causes LNK2005 multiply-defined symbol errors on
///   `raise`/`signal`/`_msize`/`memcpy` — we resolve these
///   with `/FORCE:MULTIPLE` (downgrades LNK2005 to a
///   warning, letting the linker pick the first definition).
///   This is necessary because `titrate_native.lib` embeds
///   Rust std (compiled with `/MT`, static CRT) alongside
///   bundled SQLite (compiled with `/MD`, dynamic CRT) —
///   a fundamental CRT mismatch that cannot be resolved at
///   the linker level without `/FORCE`.
#[cfg(windows)]
const WINDOWS_STDLIB_DEPS: &[&str] = &[
    "titrate_native",
    "ws2_32",
    "ntdll",
    "advapi32",
    "userenv",
    "bcrypt",
    "kernel32",
    "ucrt",
    "vcruntime",
    "libcmt",
];

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
    let mut args = build_gcc_style_args(
        object_path,
        output_exe,
        native_lib_dir,
        extra_link_libs,
        extra_link_args,
        // clang on Windows gets titrate_native (always required) plus the
        // Windows system libraries that titrate_native.lib's embedded Rust
        // std depends on (networking, NT API, cryptography, C runtime).
        // clang drives link.exe which finds these in the Windows SDK / MSVC
        // lib directories set up via the LIB environment variable.
        WINDOWS_STDLIB_DEPS,
    );
    // Add explicit /LIBPATH entries for the MSVC VC Tools and Windows Kits
    // lib directories so the linker can find vcruntime.lib, ucrt.lib,
    // kernel32.lib, etc. when the LIB env var is not set (i.e. when not
    // running inside a Developer Command Prompt).
    append_windows_libpath_gcc_args(&mut args);
    // titrate_native.lib embeds inkwell code that references LLVM-C symbols;
    // link LLVM-C.dll's import library so those symbols resolve.
    append_llvm_c_gcc_args(&mut args);
    // Suppress static CRT default libraries to resolve LNK2005 conflicts
    // between the static UCRT (Rust std, /MT) and the dynamic UCRT (bundled
    // SQLite, /MD). See `append_windows_crt_nodefault_gcc_args` for details.
    append_windows_crt_nodefault_gcc_args(&mut args);
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
    let mut args = build_link_exe_args(
        object_path,
        output_exe,
        native_lib_dir,
        extra_link_libs,
        extra_link_args,
    );
    // Add explicit /LIBPATH entries for the MSVC VC Tools and Windows Kits
    // lib directories so the linker can find vcruntime.lib, ucrt.lib,
    // kernel32.lib, etc. when the LIB env var is not set (i.e. when not
    // running inside a Developer Command Prompt).
    append_windows_libpath_msvc_args(&mut args);
    // titrate_native.lib embeds inkwell code that references LLVM-C symbols;
    // link LLVM-C.lib so those symbols resolve.
    append_llvm_c_msvc_args(&mut args);
    // Suppress static CRT default libraries to resolve LNK2005 conflicts
    // between the static UCRT (Rust std, /MT) and the dynamic UCRT (bundled
    // SQLite, /MD). See `append_windows_crt_nodefault_msvc_args` for details.
    append_windows_crt_nodefault_msvc_args(&mut args);
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
    let mut args = build_gcc_style_args(
        object_path,
        output_exe,
        native_lib_dir,
        extra_link_libs,
        extra_link_args,
        WINDOWS_STDLIB_DEPS,
    );
    // Add explicit /LIBPATH entries for the MSVC VC Tools and Windows Kits
    // lib directories so the linker can find vcruntime.lib, ucrt.lib,
    // kernel32.lib, etc. when the LIB env var is not set (i.e. when not
    // running inside a Developer Command Prompt).
    append_windows_libpath_gcc_args(&mut args);
    // titrate_native.lib embeds inkwell code that references LLVM-C symbols;
    // link LLVM-C.dll's import library so those symbols resolve.
    append_llvm_c_gcc_args(&mut args);
    // Suppress static CRT default libraries to resolve LNK2005 conflicts
    // between the static UCRT (Rust std, /MT) and the dynamic UCRT (bundled
    // SQLite, /MD). See `append_windows_crt_nodefault_gcc_args` for details.
    append_windows_crt_nodefault_gcc_args(&mut args);
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

    let mut args = build_gcc_style_args(
        object_path,
        output_exe,
        native_lib_dir,
        extra_link_libs,
        extra_link_args,
        &["titrate_native", "m", "pthread"],
    );
    // libtitrate_native.a embeds inkwell code that references LLVM-C symbols;
    // link libLLVM-C.so so those symbols resolve.
    append_llvm_c_gcc_args(&mut args);
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
/// The built-in libraries mirror `WINDOWS_STDLIB_DEPS` (rendered as `.lib`
/// files for link.exe): `titrate_native` plus the Windows system libraries
/// and Universal C Runtime import library that `titrate_native.lib`'s
/// embedded Rust std depends on.
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
    // Built-in Windows system libraries required by titrate_native.lib's
    // embedded Rust std (networking, NT API, cryptography, C runtime).
    for lib in WINDOWS_STDLIB_DEPS {
        args.push(format!("{}.lib", lib));
    }
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

/// Discover the LLVM C-API library directory.
///
/// Mirrors the logic in `trc/build.rs`: consult `LLVM_SYS_221_PREFIX` first,
/// then fall back to the well-known Windows install location. Returns the
/// `lib` directory containing `LLVM-C.lib` (Windows) / `libLLVM-C.so` (Unix).
///
/// The native linker needs this so it can resolve the LLVM-C symbols
/// referenced by `titrate_native`'s embedded inkwell code —
/// `titrate_native.lib` is built with `inkwell` as a dependency, so its
/// object files contain references to `LLVMInstructionRemoveFromParent`,
/// `LLVMSetNSW`, etc. that only `LLVM-C.lib` can satisfy.
fn find_llvm_lib_dir() -> Option<PathBuf> {
    if let Some(prefix) = std::env::var_os("LLVM_SYS_221_PREFIX") {
        let libdir = PathBuf::from(prefix).join("lib");
        if libdir.is_dir() {
            return Some(libdir);
        }
    }
    #[cfg(windows)]
    {
        let candidate = PathBuf::from(r"C:\Program Files\LLVM\lib");
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

/// Discover the Windows system library directories needed when the `LIB`
/// environment variable is not configured (e.g. when running outside a
/// Visual Studio "Developer Command Prompt").
///
/// Returns the MSVC VC Tools `lib\x64` directory (which contains
/// `vcruntime.lib`, `libcmt.lib`, etc.) plus the Windows Kits UCRT and UM
/// `lib\x64` directories (which contain `ucrt.lib`, `kernel32.lib`, etc.).
///
/// When the `LIB` env var is already set (Developer Command Prompt), this
/// function returns an empty vector — the linker uses `LIB` directly and
/// adding redundant `/LIBPATH` entries would be harmless but noisy.
#[cfg(windows)]
fn find_windows_system_lib_dirs() -> Vec<PathBuf> {
    if std::env::var_os("LIB").is_some() {
        return Vec::new();
    }
    let mut dirs = Vec::new();
    if let Some(d) = find_msvc_vc_tools_lib_dir() {
        dirs.push(d);
    }
    if let Some(d) = find_windows_kits_lib_dir("ucrt") {
        dirs.push(d);
    }
    if let Some(d) = find_windows_kits_lib_dir("um") {
        dirs.push(d);
    }
    dirs
}

/// Find the latest MSVC VC Tools `lib\x64` directory.
///
/// Searches Visual Studio 18/17/16 (2026/2022/2019) across the Community,
/// Professional, Enterprise, and BuildTools editions, picking the highest
/// installed MSVC toolchain version. Returns `None` if no installation is
/// found.
#[cfg(windows)]
fn find_msvc_vc_tools_lib_dir() -> Option<PathBuf> {
    let base = PathBuf::from(r"C:\Program Files\Microsoft Visual Studio");
    for edition in &["18", "17", "16"] {
        for flavor in &["Community", "Professional", "Enterprise", "BuildTools"] {
            let vc_tools = base.join(edition).join(flavor).join("VC").join("Tools").join("MSVC");
            if !vc_tools.is_dir() {
                continue;
            }
            let mut versions: Vec<String> = std::fs::read_dir(&vc_tools)
                .ok()?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                .collect();
            versions.sort_by(|a, b| b.cmp(a));
            for ver in &versions {
                let candidate = vc_tools.join(ver).join("lib").join("x64");
                if candidate.is_dir() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// Find the latest Windows Kits `Lib\<ver>\<subdir>\x64` directory.
///
/// `subdir` is `"ucrt"` (for `ucrt.lib`) or `"um"` (for `kernel32.lib`,
/// `ws2_32.lib`, etc.). Returns the highest-versioned kit directory that
/// actually exists.
#[cfg(windows)]
fn find_windows_kits_lib_dir(subdir: &str) -> Option<PathBuf> {
    let base = PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\Lib");
    if !base.is_dir() {
        return None;
    }
    let mut versions: Vec<String> = std::fs::read_dir(&base)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .collect();
    versions.sort_by(|a, b| b.cmp(a));
    for ver in &versions {
        let candidate = base.join(ver).join(subdir).join("x64");
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

/// Append `-Wl,/LIBPATH:<dir>` for each discovered Windows system library
/// directory. The `-Wl,` prefix passes the flag through the clang/gcc driver
/// to the underlying `link.exe`.
#[cfg(windows)]
fn append_windows_libpath_gcc_args(args: &mut Vec<String>) {
    for dir in find_windows_system_lib_dirs() {
        args.push(format!("-Wl,/LIBPATH:{}", dir.display()));
    }
}

/// Append raw `/LIBPATH:<dir>` for each discovered Windows system library
/// directory, for the MSVC `link.exe` driver.
#[cfg(windows)]
fn append_windows_libpath_msvc_args(args: &mut Vec<String>) {
    for dir in find_windows_system_lib_dirs() {
        args.push(format!("/LIBPATH:{}", dir.display()));
    }
}

/// Append `-L<llvm_lib_dir>` and `-lLLVM-C` to a gcc/clang-style arg list
/// so the linker can resolve LLVM-C symbols referenced by titrate_native.
///
/// `LLVM-C` is appended AFTER the built-in `titrate_native` library because
/// static link order requires the consumer (`titrate_native`, which references
/// LLVM-C) to appear before the provider (`LLVM-C`).
fn append_llvm_c_gcc_args(args: &mut Vec<String>) {
    if let Some(dir) = find_llvm_lib_dir() {
        args.push(format!("-L{}", dir.display()));
        args.push("-lLLVM-C".to_string());
    }
}

/// Append `/LIBPATH:<llvm_lib_dir>` and `LLVM-C.lib` to an MSVC `link.exe`
/// arg list so the linker can resolve LLVM-C symbols referenced by
/// titrate_native.
#[cfg(windows)]
fn append_llvm_c_msvc_args(args: &mut Vec<String>) {
    if let Some(dir) = find_llvm_lib_dir() {
        args.push(format!("/LIBPATH:{}", dir.display()));
        args.push("LLVM-C.lib".to_string());
    }
}

/// Append `/NODEFAULTLIB` flags (with `-Wl,` prefix for the gcc/clang
/// driver) plus `/FORCE:MULTIPLE` to resolve the static/dynamic CRT
/// mixing conflict in `titrate_native.lib`.
///
/// `titrate_native.lib` embeds two kinds of object files with conflicting
/// CRT expectations:
/// - Rust's `std` (compiled with `/MT`, static CRT) emits `/DEFAULTLIB`
///   directives that pull in `libucrt.lib` (static UCRT) and `libcmt.lib`
///   (combined static CRT).
/// - Bundled SQLite (compiled by the `cc` crate with the default `/MD`,
///   dynamic CRT) emits `/DEFAULTLIB` directives that pull in `ucrt.lib`
///   (dynamic UCRT import lib) and `vcruntime.lib` (dynamic VC runtime).
///
/// We explicitly link `libcmt.lib` (for compiler-runtime data symbols like
/// `__security_cookie`, `_tls_used`) alongside the dynamic `ucrt.lib` and
/// `vcruntime.lib` (for SQLite's import references). This causes LNK2005
/// multiply-defined symbol errors on `raise`/`signal`/`_msize`/`memcpy`
/// (defined in both static and dynamic CRTs).
///
/// `/FORCE:MULTIPLE` downgrades LNK2005 to a warning, letting the linker
/// pick the first definition. We suppress `libucrt.lib` (the standalone
/// static UCRT, redundant with `libcmt.lib`'s UCRT) and `MSVCRT` (the old
/// pre-2015 runtime) to minimise the number of multiply-defined symbols.
///
/// The `-Wl,` prefix tells clang/gcc to pass the flag through to link.exe
/// verbatim; without it clang treats `/NODEFAULTLIB:...` as an input file
/// path and fails with "no such file or directory".
#[cfg(windows)]
fn append_windows_crt_nodefault_gcc_args(args: &mut Vec<String>) {
    args.push("-Wl,/NODEFAULTLIB:libucrt.lib".to_string());
    args.push("-Wl,/NODEFAULTLIB:MSVCRT".to_string());
    // Allow multiply-defined symbols (LNK2005 → warning) so that libcmt.lib
    // (static CRT, for startup data symbols) can coexist with ucrt.lib and
    // vcruntime.lib (dynamic import libs, for SQLite's /MD references).
    args.push("-Wl,/FORCE:MULTIPLE".to_string());
}

/// Append raw `/NODEFAULTLIB` flags for the MSVC `link.exe` driver.
///
/// See `append_windows_crt_nodefault_gcc_args` for the rationale. This
/// variant omits the `-Wl,` prefix because `link.exe` accepts the flags
/// directly.
#[cfg(windows)]
fn append_windows_crt_nodefault_msvc_args(args: &mut Vec<String>) {
    args.push("/NODEFAULTLIB:libucrt.lib".to_string());
    args.push("/NODEFAULTLIB:MSVCRT".to_string());
    // Allow multiply-defined symbols (LNK2005 → warning) — see
    // `append_windows_crt_nodefault_gcc_args` for the rationale.
    args.push("/FORCE:MULTIPLE".to_string());
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

    /// B.3.5: a `[native]` section with `link_libs = ["m"]` must cause the
    /// linker invocation to include `-lm`. Exercises the gcc/clang-style arg
    /// builder used by both the Unix `cc` driver and the Windows
    /// `clang`/`gcc` driver.
    #[test]
    fn gcc_style_args_render_extra_lib_as_l_flag() {
        let obj = Path::new("a.o");
        let exe = Path::new("out");
        let lib_dir = Path::new("/x/lib");
        let extra_libs = vec!["m".to_string()];
        let args = build_gcc_style_args(obj, exe, lib_dir, &extra_libs, &[], &["titrate_native"]);
        // The user-supplied "m" must render as "-lm".
        assert!(
            args.iter().any(|a| a == "-lm"),
            "expected -lm in args: {:?}",
            args
        );
        // Sanity: it should appear after the built-in -ltitrate_native.
        let native_idx = args
            .iter()
            .position(|a| a == "-ltitrate_native")
            .expect("built-in -ltitrate_native present");
        let extra_m_idx = args
            .iter()
            .rposition(|a| a == "-lm")
            .expect("extra -lm present");
        assert!(
            extra_m_idx > native_idx,
            "extra -lm should follow -ltitrate_native"
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

    /// B.3.5 (Windows/MSVC): `[native] link_libs = ["m"]` becomes `m.lib`
    /// when the linker driver is MSVC `link.exe`.
    #[cfg(windows)]
    #[test]
    fn link_exe_args_render_native_m_as_dot_lib() {
        let obj = Path::new("a.obj");
        let exe = Path::new("out.exe");
        let lib_dir = Path::new("C:\\lib");
        let extra_libs = vec!["m".to_string()];
        let args = build_link_exe_args(obj, exe, lib_dir, &extra_libs, &[]);
        assert!(
            args.iter().any(|a| a == "m.lib"),
            "expected m.lib in args: {:?}",
            args
        );
    }

    /// `find_llvm_lib_dir` must return a directory that actually exists when
    /// LLVM is installed (the build itself would fail without LLVM, so this
    /// test machine has it). If LLVM is genuinely absent the test is a no-op.
    #[test]
    fn find_llvm_lib_dir_returns_existing_directory_or_none() {
        match find_llvm_lib_dir() {
            Some(dir) => assert!(dir.is_dir(), "find_llvm_lib_dir returned non-existent dir: {}", dir.display()),
            None => { /* LLVM not installed — acceptable for this smoke test */ }
        }
    }

    /// `append_llvm_c_gcc_args` must add `-L<dir>` and `-lLLVM-C` in that
    /// order when LLVM is available, and must leave the args untouched when
    /// it is not. The `-lLLVM-C` flag must come after any existing
    /// `-ltitrate_native` to satisfy static link order (consumer before
    /// provider).
    #[test]
    fn append_llvm_c_gcc_args_adds_libpath_and_libname_when_llvm_present() {
        let mut args = vec!["-ltitrate_native".to_string()];
        append_llvm_c_gcc_args(&mut args);
        match find_llvm_lib_dir() {
            Some(_) => {
                let l_idx = args.iter().position(|a| a.starts_with("-L"));
                let llvmc_idx = args.iter().position(|a| a == "-lLLVM-C");
                assert!(l_idx.is_some(), "expected -L flag after append_llvm_c_gcc_args, got: {:?}", args);
                assert!(llvmc_idx.is_some(), "expected -lLLVM-C flag, got: {:?}", args);
                // -L must come before -lLLVM-C.
                assert!(l_idx.unwrap() < llvmc_idx.unwrap());
                // -lLLVM-C must come after -ltitrate_native (consumer before provider).
                let native_idx = args.iter().position(|a| a == "-ltitrate_native").unwrap();
                assert!(native_idx < llvmc_idx.unwrap(), "LLVM-C must follow titrate_native in link order");
            }
            None => {
                // No LLVM — args must be unchanged.
                assert_eq!(args, vec!["-ltitrate_native".to_string()]);
            }
        }
    }

    /// `append_llvm_c_msvc_args` must add `/LIBPATH:<dir>` and `LLVM-C.lib`
    /// when LLVM is available. Windows-only.
    #[cfg(windows)]
    #[test]
    fn append_llvm_c_msvc_args_adds_libpath_and_libname_when_llvm_present() {
        let mut args = vec!["titrate_native.lib".to_string()];
        append_llvm_c_msvc_args(&mut args);
        match find_llvm_lib_dir() {
            Some(_) => {
                let libpath_idx = args.iter().position(|a| a.starts_with("/LIBPATH:"));
                let llvmc_idx = args.iter().position(|a| a == "LLVM-C.lib");
                assert!(libpath_idx.is_some(), "expected /LIBPATH: flag, got: {:?}", args);
                assert!(llvmc_idx.is_some(), "expected LLVM-C.lib, got: {:?}", args);
                assert!(libpath_idx.unwrap() < llvmc_idx.unwrap());
            }
            None => {
                assert_eq!(args, vec!["titrate_native.lib".to_string()]);
            }
        }
    }
}
