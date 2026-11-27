// trc build script.
//
// Links the LLVM C API shared library (LLVM-C.dll on Windows, libLLVM-C.so on
// Unix) so that the inkwell-based codegen module can call into LLVM at runtime.
//
// We rely on inkwell's `no-llvm-linking` + llvm-sys's `disable-alltargets-init`
// features to skip llvm-config entirely and link the single combined C API
// library directly. This avoids version-mismatch issues between the llvm-sys
// crate's expected LLVM version and the system-installed LLVM.
//
// The `LLVMInitializeAll*` / `LLVMInitializeNative*` functions are `static
// inline` in `llvm-c/Target.h`, so they are not real symbols in LLVM-C.lib.
// llvm-sys normally provides C wrappers, but skips them when
// `disable-alltargets-init` is enabled. We provide Rust wrappers in
// `src/codegen/llvm/target_wrappers.rs` that call the individual target
// initialization functions (which ARE exported from LLVM-C.dll) directly.

use std::env;
use std::path::PathBuf;

fn windows_llvm_prefix() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let candidate = PathBuf::from(r"C:\Program Files\LLVM");
        if candidate.join("lib").exists() {
            return Some(candidate);
        }
    }
    None
}

fn main() {
    // Allow the user to override the LLVM prefix via LLVM_SYS_221_PREFIX.
    let prefix = env::var_os("LLVM_SYS_221_PREFIX")
        .map(PathBuf::from)
        .or_else(windows_llvm_prefix);

    if let Some(prefix) = prefix {
        let libdir = prefix.join("lib");
        println!("cargo:rustc-link-search=native={}", libdir.display());

        // Link the combined C API library. On Windows this is an import
        // library for LLVM-C.dll; on Unix it is libLLVM-C.so / libLLVM-C.dylib.
        // All three platforms use the same library name.
        println!("cargo:rustc-link-lib=dylib=LLVM-C");
    } else {
        // Native codegen links against LLVM-C directly; without it the link
        // step fails, so stop here with an actionable message instead.
        panic!(
            "LLVM development files not found (LLVM 22.1 or newer; LLVM 23.0.0git is also supported \
             because trc links LLVM-C directly). Install LLVM and set LLVM_SYS_221_PREFIX to its \
             installation directory (on Windows the default is C:\\Program Files\\LLVM) so that its \
             lib directory contains the LLVM-C library, then rebuild trc."
        );
    }

    // The titrate_native static library is needed at link time when producing
    // native binaries, but not for building trc itself. We still record the
    // path so the codegen module can locate it.
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let workspace_root = PathBuf::from(manifest_dir)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        println!(
            "cargo:rustc-env=TITRATE_WORKSPACE_ROOT={}",
            workspace_root.display()
        );
    }
}
