//! Uniform C-ABI wrappers for the host graphics primitives.
//!
//! Every wrapper here has the same signature as the other VM native
//! wrappers:
//!   `TitrateValue titrate_Graphics_x(const TitrateValue* args, size_t count)`
//! and delegates to a scalar `titrate_graphics_*` host primitive, converting
//! the result back through `TitrateValue`. These names are additive to the
//! native registry used by `tt::graphics`.

use std::rc::Rc;

use trc::bytecode::value::Value;

use crate::{graphics, titrate_to_value, value_to_titrate, TitrateValue};

/// Read uniform C-ABI arguments with `read_unaligned`, matching
/// `wrappers::mod::native_wrapper` so LLVM's struct padding cannot corrupt
/// an argument.
unsafe fn read_args(args: *const TitrateValue, arg_count: usize) -> Vec<Value> {
    if args.is_null() || arg_count == 0 {
        return Vec::new();
    }
    let mut values = Vec::with_capacity(arg_count);
    for i in 0..arg_count {
        let tv = unsafe { std::ptr::read_unaligned(args.add(i)) };
        values.push(titrate_to_value(&tv));
    }
    values
}

fn arg_int(values: &[Value], index: usize) -> i64 {
    match values.get(index) {
        Some(Value::Int(v)) => *v as i64,
        Some(Value::Long(v)) => *v,
        Some(Value::Byte(v)) => *v as i64,
        Some(Value::Short(v)) => *v as i64,
        Some(Value::Bool(true)) => 1,
        Some(Value::Bool(false)) => 0,
        _ => 0,
    }
}

fn int_result(v: i64) -> TitrateValue {
    value_to_titrate(&Value::Int(v as i32))
}

fn bool_result(v: bool) -> TitrateValue {
    value_to_titrate(&Value::Bool(v))
}

/// `Graphics_windowCreate(title: string, width: int, height: int) -> string`
/// Returns an empty string on success, or the exact error text
/// `windowing backend unavailable`. `tt::graphics::Window.create()` turns the
/// latter into `Err("windowing backend unavailable")`.
#[no_mangle]
pub unsafe extern "C" fn titrate_Graphics_windowCreate(
    args: *const TitrateValue,
    arg_count: usize,
) -> TitrateValue {
    let values = unsafe { read_args(args, arg_count) };
    let status = crate::titrate_graphics_window_create(arg_int(&values, 1), arg_int(&values, 2));
    let message = if status == graphics::STATUS_BACKEND_UNAVAILABLE {
        graphics::WINDOW_BACKEND_UNAVAILABLE
    } else if status == graphics::STATUS_INVALID_ARGUMENT {
        "invalid window size"
    } else {
        ""
    };
    value_to_titrate(&Value::String(Rc::new(message.to_string())))
}

/// `Graphics_windowIsAvailable() -> bool`. Always false with a `std`-only
/// host, but read from the primitive instead of assumed in `.tr`.
#[no_mangle]
pub unsafe extern "C" fn titrate_Graphics_windowIsAvailable(
    args: *const TitrateValue,
    arg_count: usize,
) -> TitrateValue {
    let _ = (args, arg_count);
    bool_result(crate::titrate_graphics_window_backend_available() != 0)
}

/// `Graphics_inputPush(code: int) -> void`. Enqueue a synthetic input event.
#[no_mangle]
pub unsafe extern "C" fn titrate_Graphics_inputPush(
    args: *const TitrateValue,
    arg_count: usize,
) -> TitrateValue {
    let values = unsafe { read_args(args, arg_count) };
    crate::titrate_graphics_input_push(arg_int(&values, 0));
    value_to_titrate(&Value::Void)
}

/// `Graphics_inputPoll() -> int`. Returns the oldest event code, or 0 when
/// the queue is empty.
#[no_mangle]
pub unsafe extern "C" fn titrate_Graphics_inputPoll(
    args: *const TitrateValue,
    arg_count: usize,
) -> TitrateValue {
    let _ = (args, arg_count);
    int_result(crate::titrate_graphics_input_poll())
}

/// `Graphics_bufferCreate(width: int, height: int) -> int`. Returns a
/// positive handle, or a negative status on invalid size.
#[no_mangle]
pub unsafe extern "C" fn titrate_Graphics_bufferCreate(
    args: *const TitrateValue,
    arg_count: usize,
) -> TitrateValue {
    let values = unsafe { read_args(args, arg_count) };
    int_result(crate::titrate_graphics_buffer_create(
        arg_int(&values, 0),
        arg_int(&values, 1),
    ))
}

/// `Graphics_bufferDestroy(handle: int) -> bool`.
#[no_mangle]
pub unsafe extern "C" fn titrate_Graphics_bufferDestroy(
    args: *const TitrateValue,
    arg_count: usize,
) -> TitrateValue {
    let values = unsafe { read_args(args, arg_count) };
    bool_result(
        crate::titrate_graphics_buffer_destroy(arg_int(&values, 0))
            == graphics::STATUS_OK,
    )
}

/// `Graphics_bufferSet(handle: int, x: int, y: int, rgba: int) -> bool`.
#[no_mangle]
pub unsafe extern "C" fn titrate_Graphics_bufferSet(
    args: *const TitrateValue,
    arg_count: usize,
) -> TitrateValue {
    let values = unsafe { read_args(args, arg_count) };
    bool_result(
        crate::titrate_graphics_buffer_set(
            arg_int(&values, 0),
            arg_int(&values, 1),
            arg_int(&values, 2),
            arg_int(&values, 3),
        ) == graphics::STATUS_OK,
    )
}

/// `Graphics_bufferGet(handle: int, x: int, y: int) -> int`. Returns the
/// packed RGBA bits as a 32-bit integer; negative for invalid coordinates.
#[no_mangle]
pub unsafe extern "C" fn titrate_Graphics_bufferGet(
    args: *const TitrateValue,
    arg_count: usize,
) -> TitrateValue {
    let values = unsafe { read_args(args, arg_count) };
    int_result(crate::titrate_graphics_buffer_get(
        arg_int(&values, 0),
        arg_int(&values, 1),
        arg_int(&values, 2),
    ))
}

/// `Graphics_bufferClear(handle: int, rgba: int) -> bool`.
#[no_mangle]
pub unsafe extern "C" fn titrate_Graphics_bufferClear(
    args: *const TitrateValue,
    arg_count: usize,
) -> TitrateValue {
    let values = unsafe { read_args(args, arg_count) };
    bool_result(
        crate::titrate_graphics_buffer_clear(arg_int(&values, 0), arg_int(&values, 1))
            == graphics::STATUS_OK,
    )
}

/// `Graphics_bufferWidth(handle: int) -> int`.
#[no_mangle]
pub unsafe extern "C" fn titrate_Graphics_bufferWidth(
    args: *const TitrateValue,
    arg_count: usize,
) -> TitrateValue {
    let values = unsafe { read_args(args, arg_count) };
    int_result(crate::titrate_graphics_buffer_width(arg_int(&values, 0)))
}

/// `Graphics_bufferHeight(handle: int) -> int`.
#[no_mangle]
pub unsafe extern "C" fn titrate_Graphics_bufferHeight(
    args: *const TitrateValue,
    arg_count: usize,
) -> TitrateValue {
    let values = unsafe { read_args(args, arg_count) };
    int_result(crate::titrate_graphics_buffer_height(arg_int(&values, 0)))
}
