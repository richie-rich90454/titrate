//! Rust wrappers for the `LLVMInitializeAll*` / `LLVMInitializeNative*` functions.
//!
//! These functions are declared `static inline` in `llvm-c/Target.h`, so they
//! are not real symbols in `LLVM-C.lib` / `LLVM-C.dll`. The `llvm-sys` crate
//! normally ships C wrappers that call the inline functions, but when the
//! `disable-alltargets-init` feature is enabled (which we use to avoid
//! llvm-config version coupling), those wrappers are not compiled.
//!
//! inkwell's `Target::initialize_all` / `Target::initialize_native` methods
//! still call `LLVM_InitializeAllTargets` etc. via FFI, so we provide the
//! symbols here. Each wrapper calls the individual target initialization
//! functions (which ARE exported from `LLVM-C.dll`) directly.
//!
//! On x86 / x86_64 we only need the X86 target. Other targets are omitted to
//! keep the symbol table small; add them here if the `target-*` inkwell
//! features are enabled in the future.

// Individual target initialization functions exported by LLVM-C.dll.
extern "C" {
    fn LLVMInitializeX86TargetInfo();
    fn LLVMInitializeX86Target();
    fn LLVMInitializeX86TargetMC();
    fn LLVMInitializeX86AsmPrinter();
    fn LLVMInitializeX86AsmParser();
    fn LLVMInitializeX86Disassembler();
}

/// `LLVMInitializeAllTargetInfos` wrapper – initialize all target infos.
/// Phase 0 only enables the X86 target.
#[no_mangle]
pub extern "C" fn LLVM_InitializeAllTargetInfos() {
    unsafe { LLVMInitializeX86TargetInfo() };
}

/// `LLVMInitializeAllTargets` wrapper – initialize all targets.
#[no_mangle]
pub extern "C" fn LLVM_InitializeAllTargets() {
    unsafe { LLVMInitializeX86Target() };
}

/// `LLVMInitializeAllTargetMCs` wrapper – initialize all target MCs.
#[no_mangle]
pub extern "C" fn LLVM_InitializeAllTargetMCs() {
    unsafe { LLVMInitializeX86TargetMC() };
}

/// `LLVMInitializeAllAsmPrinters` wrapper – initialize all asm printers.
#[no_mangle]
pub extern "C" fn LLVM_InitializeAllAsmPrinters() {
    unsafe { LLVMInitializeX86AsmPrinter() };
}

/// `LLVMInitializeAllAsmParsers` wrapper – initialize all asm parsers.
#[no_mangle]
pub extern "C" fn LLVM_InitializeAllAsmParsers() {
    unsafe { LLVMInitializeX86AsmParser() };
}

/// `LLVMInitializeAllDisassemblers` wrapper – initialize all disassemblers.
#[no_mangle]
pub extern "C" fn LLVM_InitializeAllDisassemblers() {
    unsafe { LLVMInitializeX86Disassembler() };
}

/// `LLVMInitializeNativeTarget` wrapper. Returns 0 on success, non-zero on
/// failure (matching `LLVMBool`).
#[no_mangle]
pub extern "C" fn LLVM_InitializeNativeTarget() -> i32 {
    unsafe { LLVMInitializeX86Target() };
    0
}

/// `LLVMInitializeNativeAsmParser` wrapper.
#[no_mangle]
pub extern "C" fn LLVM_InitializeNativeAsmParser() -> i32 {
    unsafe { LLVMInitializeX86AsmParser() };
    0
}

/// `LLVMInitializeNativeAsmPrinter` wrapper.
#[no_mangle]
pub extern "C" fn LLVM_InitializeNativeAsmPrinter() -> i32 {
    unsafe { LLVMInitializeX86AsmPrinter() };
    0
}

/// `LLVMInitializeNativeDisassembler` wrapper.
#[no_mangle]
pub extern "C" fn LLVM_InitializeNativeDisassembler() -> i32 {
    unsafe { LLVMInitializeX86Disassembler() };
    0
}

/// Initialize the X86 target directly. This bypasses the `#[no_mangle]`
/// `LLVM_InitializeNativeTarget` indirection (which inkwell calls via
/// llvm-sys) because that symbol resolution can fail when the inkwell
/// feature version (e.g. `llvm22-1`) does not exactly match the installed
/// LLVM version (e.g. 23.0.0). Calling the individual
/// `LLVMInitializeX86*` functions directly avoids the version-coupled FFI
/// path and works as long as the X86 target is linked into `LLVM-C.dll`.
pub fn initialize_x86() {
    // SAFETY: these are plain initialisation functions with no preconditions
    // and no meaningful return value. They are idempotent and thread-safe by
    // LLVM contract. Calling them more than once is a no-op.
    unsafe {
        LLVMInitializeX86TargetInfo();
        LLVMInitializeX86Target();
        LLVMInitializeX86TargetMC();
        LLVMInitializeX86AsmPrinter();
        LLVMInitializeX86AsmParser();
    }
}
