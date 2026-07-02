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

fn main() {
    // Allow the user to override the LLVM prefix via LLVM_SYS_221_PREFIX.
    let prefix = env::var_os("LLVM_SYS_221_PREFIX")
        .map(PathBuf::from)
        .or_else(|| {
            // Fall back to the well-known Windows install location.
            #[cfg(windows)]
            {
                let candidate = PathBuf::from(r"C:\Program Files\LLVM");
                if candidate.join("lib").exists() {
                    return Some(candidate);
                }
            }
            None
        });

    if let Some(prefix) = prefix {
        let libdir = prefix.join("lib");
        println!("cargo:rustc-link-search=native={}", libdir.display());

        // Link the combined C API library. On Windows this is an import
        // library for LLVM-C.dll; on Unix it is libLLVM-C.so / libLLVM-C.dylib.
        // All three platforms use the same library name.
        println!("cargo:rustc-link-lib=dylib=LLVM-C");
    } else {
        // No LLVM found – emit a cfg so the codegen module can fall back to a
        // text-based IR backend that only needs clang/llc at link time.
        println!("cargo:rustc-cfg=trc_llvm_unavailable");
        println!("cargo:warning=LLVM development files not found; native codegen will use the text-IR fallback");
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
