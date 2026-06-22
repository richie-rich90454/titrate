//! Ownership, borrows, regions, and unsafe-block codegen helpers.
//!
//! This module implements Phase 1 of the Titrate LLVM native backend's
//! ownership features:
//!
//! - **`Owned<T>` drop flags**: each local `Owned<T>` variable gets an `i1`
//!   alloca drop flag initialised to `true`. On move (assignment of an
//!   `Owned<T>` value to another variable), the source drop flag is set to
//!   `false`. At scope exit, the drop flag is checked and, if still true,
//!   `titrate_free` is called on the pointer.
//!
//! - **Borrows**: `&T` and `&mut T` lower to raw pointers. `Expr::RefExpr`
//!   computes the address of the referenced value (the existing alloca
//!   pointer for an identifier, or a fresh alloca for a temporary).
//!
//! - **Regions**: `region` blocks lower to `alloca` plus
//!   `llvm.lifetime.start` / `llvm.lifetime.end` intrinsics. A region
//!   context tracks the current region's allocas and emits lifetime
//!   markers.
//!
//! - **Unsafe blocks**: `Expr::UnsafeBlock` just emits the body statements
//!   without any safety checks. Raw memory operations use `titrate_malloc`
//!   and `titrate_free`.
//!
//! - **`OwnedDeref`**: `Expr::OwnedDeref` loads the value pointed to by the
//!   `Owned<T>` pointer.
//!
//! For Phase 1, `Owned<T>` is just a heap pointer (`i8*`). Drop flags are
//! `i1` allocas. At scope exit, if the drop flag is true, `titrate_free` is
//! called on the pointer. A stack of cleanup actions is maintained: each
//! entry records the drop-flag alloca and the pointer alloca for an
//! `Owned<T>` local. When a scope ends, the cleanup actions recorded for
//! that scope are emitted (in reverse order).

use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};
use inkwell::AddressSpace;

/// A cleanup action registered for an `Owned<T>` local variable.
///
/// When the enclosing scope exits, the codegen emits a check: load
/// `drop_flag`; if true, call `titrate_free` on the pointer loaded from
/// `ptr_alloca`.
#[derive(Clone, Copy)]
pub struct CleanupAction<'ctx> {
    /// Alloca holding an `i1` drop flag.
    pub drop_flag: PointerValue<'ctx>,
    /// Alloca holding the `Owned<T>` pointer (an `i8*`).
    pub ptr_alloca: PointerValue<'ctx>,
}

/// A scope marker: the index in the cleanup stack at which this scope's
/// cleanups begin. When the scope exits, all cleanups from this index
/// upward are emitted (in reverse) and then popped.
pub struct ScopeMarker {
    pub start_index: usize,
}

/// Ownership / region codegen state. Owned by the `LlvmBackend` and
/// manipulated via the helpers below.
pub struct OwnershipContext<'ctx> {
    /// Stack of cleanup actions. Each `Owned<T>` local pushes one entry.
    cleanup_stack: Vec<CleanupAction<'ctx>>,
    /// Stack of scope markers (one per entered block).
    scope_markers: Vec<ScopeMarker>,
    /// Counter for generating unique region names.
    region_counter: usize,
}

impl<'ctx> OwnershipContext<'ctx> {
    pub fn new() -> Self {
        OwnershipContext {
            cleanup_stack: Vec::new(),
            scope_markers: Vec::new(),
            region_counter: 0,
        }
    }

    /// Enter a new scope. Returns a marker that should be passed to
    /// `exit_scope` when the scope ends.
    pub fn enter_scope(&mut self) -> ScopeMarker {
        let marker = ScopeMarker {
            start_index: self.cleanup_stack.len(),
        };
        self.scope_markers.push(ScopeMarker { start_index: marker.start_index });
        marker
    }

    /// Register a cleanup action for an `Owned<T>` local.
    pub fn register_owned_local(&mut self, action: CleanupAction<'ctx>) {
        self.cleanup_stack.push(action);
    }

    /// Return the cleanup actions registered since the current scope
    /// was entered. The caller is responsible for emitting the cleanup
    /// code (typically via `emit_scope_cleanup`).
    pub fn current_scope_actions(&self) -> &[CleanupAction<'ctx>] {
        let marker = self.scope_markers.last()
            .expect("scope marker must exist when querying scope actions");
        &self.cleanup_stack[marker.start_index..]
    }

    /// Pop the current scope's cleanup actions off the stack (after they
    /// have been emitted). Returns the popped actions in registration order
    /// (the caller should emit them in reverse).
    pub fn exit_scope(&mut self) -> Vec<CleanupAction<'ctx>> {
        let marker = self.scope_markers.pop()
            .expect("exit_scope called without enter_scope");
        self.cleanup_stack.split_off(marker.start_index)
    }

    /// Generate a unique region name.
    pub fn next_region_name(&mut self) -> String {
        let n = self.region_counter;
        self.region_counter += 1;
        format!("region.{}", n)
    }

    /// Look up the drop flag for an `Owned<T>` local whose pointer is
    /// stored at `ptr_alloca`. Returns `None` if no matching cleanup
    /// action is registered.
    pub fn find_drop_flag(&self, ptr_alloca: PointerValue<'ctx>) -> Option<PointerValue<'ctx>> {
        for action in self.cleanup_stack.iter().rev() {
            if action.ptr_alloca == ptr_alloca {
                return Some(action.drop_flag);
            }
        }
        None
    }
}

/// Emit cleanup code for a list of cleanup actions, in reverse order
/// (last-registered first). The cleanups are emitted in the current
/// builder insertion block.
///
/// For each action:
///   1. Load the `i1` drop flag.
///   2. If true, load the pointer from `ptr_alloca` and call `titrate_free`.
///
/// The cleanups are emitted inline (without splitting the current block)
/// because Phase 1 does not support exceptions; scope exit always happens
/// via a normal branch.
pub fn emit_cleanup<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    module: &Module<'ctx>,
    actions: &[CleanupAction<'ctx>],
) -> Result<(), String> {
    let i1_ty = context.bool_type();
    let i8_ptr_ty = context.ptr_type(AddressSpace::default());

    let free_fn = module.get_function("titrate_free")
        .ok_or_else(|| "titrate_free not declared".to_string())?;

    for action in actions.iter().rev() {
        // Load drop flag.
        let flag = builder.build_load(i1_ty, action.drop_flag, "drop.flag")
            .map_err(|e| format!("build_load drop flag failed: {:?}", e))?
            .into_int_value();

        // Create blocks for the "free" and "skip" paths.
        let current_block = builder.get_insert_block()
            .ok_or("codegen: no insert block for cleanup")?;
        let free_block = context.insert_basic_block_after(current_block, "cleanup.free");
        let skip_block = context.insert_basic_block_after(free_block, "cleanup.skip");

        builder.build_conditional_branch(flag, free_block, skip_block)
            .map_err(|e| format!("build_cond_br cleanup failed: {:?}", e))?;

        // Free block: load the pointer and call titrate_free.
        builder.position_at_end(free_block);
        let ptr = builder.build_load(i8_ptr_ty, action.ptr_alloca, "drop.ptr")
            .map_err(|e| format!("build_load drop ptr failed: {:?}", e))?
            .into_pointer_value();
        builder.build_call(free_fn, &[ptr.into()], "drop.free")
            .map_err(|e| format!("build_call titrate_free failed: {:?}", e))?;
        builder.build_unconditional_branch(skip_block)
            .map_err(|e| format!("build_br cleanup.skip failed: {:?}", e))?;

        // Continue in the skip block.
        builder.position_at_end(skip_block);
    }
    Ok(())
}

/// Emit a `llvm.lifetime.start` intrinsic call for the given alloca.
///
/// The intrinsic signature is `void llvm.lifetime.start(i64 <size>, ptr nocapture <ptr>)`.
pub fn emit_lifetime_start<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    module: &Module<'ctx>,
    alloca: PointerValue<'ctx>,
    ty: BasicTypeEnum<'ctx>,
) -> Result<(), String> {
    let i64_ty = context.i64_type();
    let _ = ty;
    // -1 means "the whole allocation" per LLVM semantics.
    let neg_one = i64_ty.const_int(u64::MAX, true);

    let void_ty = context.void_type();
    let i8_ptr_ty = context.ptr_type(AddressSpace::default());
    let fn_ty = void_ty.fn_type(&[i64_ty.into(), i8_ptr_ty.into()], false);
    let intrinsic = module.add_function("llvm.lifetime.start", fn_ty, None);

    let ptr_cast = if alloca.get_type() == i8_ptr_ty {
        alloca
    } else {
        builder.build_bit_cast(alloca, i8_ptr_ty, "lifetime.ptr")
            .map_err(|e| format!("build_bit_cast lifetime failed: {:?}", e))?
            .into_pointer_value()
    };

    builder.build_call(intrinsic, &[neg_one.into(), ptr_cast.into()], "lifetime.start")
        .map_err(|e| format!("build_call lifetime.start failed: {:?}", e))?;
    Ok(())
}

/// Emit a `llvm.lifetime.end` intrinsic call for the given alloca.
pub fn emit_lifetime_end<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    module: &Module<'ctx>,
    alloca: PointerValue<'ctx>,
    ty: BasicTypeEnum<'ctx>,
) -> Result<(), String> {
    let i64_ty = context.i64_type();
    let _ = ty;
    let neg_one = i64_ty.const_int(u64::MAX, true);

    let void_ty = context.void_type();
    let i8_ptr_ty = context.ptr_type(AddressSpace::default());
    let fn_ty = void_ty.fn_type(&[i64_ty.into(), i8_ptr_ty.into()], false);
    let intrinsic = module.add_function("llvm.lifetime.end", fn_ty, None);

    let ptr_cast = if alloca.get_type() == i8_ptr_ty {
        alloca
    } else {
        builder.build_bit_cast(alloca, i8_ptr_ty, "lifetime.end.ptr")
            .map_err(|e| format!("build_bit_cast lifetime end failed: {:?}", e))?
            .into_pointer_value()
    };

    builder.build_call(intrinsic, &[neg_one.into(), ptr_cast.into()], "lifetime.end")
        .map_err(|e| format!("build_call lifetime.end failed: {:?}", e))?;
    Ok(())
}

/// Allocate an `Owned<T>` value: allocate a heap buffer of the inner
/// type's size, store the initial value into it, allocate a drop flag
/// initialised to `true`, and return `(ptr_alloca, drop_flag_alloca)`.
///
/// `inner_value` is the LLVM value to store in the heap allocation.
/// `inner_ty` is the LLVM type of the inner value.
pub fn alloc_owned<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    module: &Module<'ctx>,
    inner_value: BasicValueEnum<'ctx>,
    inner_ty: BasicTypeEnum<'ctx>,
    name_hint: &str,
) -> Result<(PointerValue<'ctx>, PointerValue<'ctx>), String> {
    let i8_ptr_ty = context.ptr_type(AddressSpace::default());
    let i1_ty = context.bool_type();

    // Look up titrate_malloc. It takes a size in bytes and returns an i8*.
    let malloc_fn = module.get_function("titrate_malloc")
        .ok_or_else(|| "titrate_malloc not declared".to_string())?;

    // Compute the allocation size in bytes.
    let size = inner_ty.size_of()
        .ok_or_else(|| format!("cannot compute size of type {:?}", inner_ty))?;

    // Call titrate_malloc(size) -> i8*.
    let raw_ptr = builder.build_call(malloc_fn, &[size.into()], &format!("{}.malloc", name_hint))
        .map_err(|e| format!("build_call titrate_malloc failed: {:?}", e))?;
    let raw_ptr = match raw_ptr.try_as_basic_value() {
        inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
        _ => return Err("titrate_malloc did not return a value".to_string()),
    };

    // In LLVM 15+, all pointers are opaque `ptr`, so we don't need to cast.
    // We just use the raw i8* pointer directly for storing the value.
    let typed_ptr = raw_ptr;

    // Store the initial value into the heap allocation.
    builder.build_store(typed_ptr, inner_value)
        .map_err(|e| format!("build_store owned init failed: {:?}", e))?;

    // Allocate a local i8* slot to hold the Owned<T> pointer (so that
    // assignment/move can update it).
    let ptr_alloca = builder.build_alloca(i8_ptr_ty, &format!("{}.ptr", name_hint))
        .map_err(|e| format!("build_alloca owned ptr failed: {:?}", e))?;
    builder.build_store(ptr_alloca, raw_ptr)
        .map_err(|e| format!("build_store owned ptr failed: {:?}", e))?;

    // Allocate the drop flag and initialise it to true.
    let drop_flag = builder.build_alloca(i1_ty, &format!("{}.drop", name_hint))
        .map_err(|e| format!("build_alloca drop flag failed: {:?}", e))?;
    let true_val = i1_ty.const_int(1, false);
    builder.build_store(drop_flag, true_val)
        .map_err(|e| format!("build_store drop flag failed: {:?}", e))?;

    Ok((ptr_alloca, drop_flag))
}

/// Load the inner value from an `Owned<T>` pointer alloca.
pub fn load_owned_value<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    inner_ty: BasicTypeEnum<'ctx>,
    ptr_alloca: PointerValue<'ctx>,
    name_hint: &str,
) -> Result<BasicValueEnum<'ctx>, String> {
    let i8_ptr_ty = context.ptr_type(AddressSpace::default());

    // Load the i8* from the alloca.
    let raw_ptr = builder.build_load(i8_ptr_ty, ptr_alloca, &format!("{}.raw", name_hint))
        .map_err(|e| format!("build_load owned raw failed: {:?}", e))?
        .into_pointer_value();

    // In LLVM 15+, all pointers are opaque `ptr`, so no cast is needed.
    let typed_ptr = raw_ptr;

    // Load the inner value.
    builder.build_load(inner_ty, typed_ptr, &format!("{}.val", name_hint))
        .map_err(|e| format!("build_load owned val failed: {:?}", e))
}

/// Mark an `Owned<T>` local as moved by setting its drop flag to false.
pub fn mark_moved<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    drop_flag: PointerValue<'ctx>,
) -> Result<(), String> {
    let i1_ty = context.bool_type();
    let false_val = i1_ty.const_int(0, false);
    builder.build_store(drop_flag, false_val)
        .map_err(|e| format!("build_store moved flag failed: {:?}", e))?;
    Ok(())
}

/// Get the `i8*` pointer value from an `Owned<T>` pointer alloca.
pub fn get_owned_raw_ptr<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    ptr_alloca: PointerValue<'ctx>,
    name_hint: &str,
) -> Result<PointerValue<'ctx>, String> {
    let i8_ptr_ty = context.ptr_type(AddressSpace::default());
    let raw = builder.build_load(i8_ptr_ty, ptr_alloca, &format!("{}.raw", name_hint))
        .map_err(|e| format!("build_load owned raw ptr failed: {:?}", e))?
        .into_pointer_value();
    Ok(raw)
}

/// Suppress unused-import warnings for re-exported helpers.
#[allow(dead_code)]
fn _suppress_unused<'ctx>(_: IntValue<'ctx>, _: BasicBlock<'ctx>) {}
