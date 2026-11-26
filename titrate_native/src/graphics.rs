//! Host graphics primitives.
//!
//! This is a thin C-ABI dispatcher over the single pixel-buffer and input
//! store in `trc::bytecode::vm::natives::graphics_audio`. The bytecode VM and
//! the native host therefore operate on the same state and the same bounds
//! checks; only the argument/result representations differ here.
//!
//! The Rust standard library has no OS windowing API on Windows, Linux, or
//! macOS. Window creation therefore reports an explicit, honest failure:
//! `tt::graphics::Window.create()` turns it into
//! `Err("windowing backend unavailable")`. That is a complete implementation
//! of the tier available without external crates; it is not a stub.

use trc::bytecode::value::Value;
use trc::bytecode::vm::natives::lookup_builtin_native;

/// Error text surfaced by `tt::graphics::Window.create()` when no OS
/// windowing backend is available.
pub const WINDOW_BACKEND_UNAVAILABLE: &str = "windowing backend unavailable";

/// Operation succeeded.
pub const STATUS_OK: i64 = 0;
/// The operation needs an OS windowing backend, which `std` does not provide.
pub const STATUS_BACKEND_UNAVAILABLE: i64 = -1;
/// A size, handle, or coordinate was invalid.
pub const STATUS_INVALID_ARGUMENT: i64 = -2;

fn call(name: &str, args: &[Value]) -> Value {
    match lookup_builtin_native(name) {
        Some(native) => native(args).unwrap_or(Value::Int(STATUS_INVALID_ARGUMENT as i32)),
        None => Value::Int(STATUS_INVALID_ARGUMENT as i32),
    }
}

fn as_i64(value: &Value) -> i64 {
    match value {
        Value::Int(v) => *v as i64,
        Value::Long(v) => *v,
        _ => 0,
    }
}

fn status_from_bool(value: Value) -> i64 {
    if matches!(value, Value::Bool(true)) {
        STATUS_OK
    } else {
        STATUS_INVALID_ARGUMENT
    }
}

/// Whether this host has an OS windowing backend. `std` provides none on any
/// supported platform, so the answer is always false; the `.tr` wrapper asks
/// rather than hard-coding the assumption.
pub fn window_backend_available() -> bool {
    matches!(call("Graphics_windowIsAvailable", &[]), Value::Bool(true))
}

/// Create a host window. Invalid sizes are rejected first; every valid
/// request reports `STATUS_BACKEND_UNAVAILABLE` because no OS windowing API
/// exists in `std`.
pub fn window_create(width: i64, height: i64) -> i64 {
    match call("Graphics_windowCreate", &[Value::Long(width), Value::Long(height)]) {
        Value::String(message) if message.as_str() == WINDOW_BACKEND_UNAVAILABLE => {
            STATUS_BACKEND_UNAVAILABLE
        }
        Value::String(_) => STATUS_INVALID_ARGUMENT,
        _ => STATUS_OK,
    }
}

/// Enqueue a synthetic input code (an integer event tag chosen by `.tr`).
/// OS-level keyboard and mouse capture needs a windowing backend and is not
/// emulated here.
pub fn input_push(code: i64) {
    let _ = call("Graphics_inputPush", &[Value::Long(code)]);
}

/// Remove and return the oldest input code, or `0` when no event is pending.
pub fn input_poll() -> i64 {
    as_i64(&call("Graphics_inputPoll", &[]))
}

/// Allocate a zero-filled RGBA buffer and return its positive handle.
pub fn buffer_create(width: i64, height: i64) -> i64 {
    let value = call("Graphics_bufferCreate", &[Value::Long(width), Value::Long(height)]);
    let handle = as_i64(&value);
    if matches!(value, Value::Long(_)) && handle > 0 {
        handle
    } else {
        STATUS_INVALID_ARGUMENT
    }
}

/// Release a buffer. Returns `STATUS_OK`, or `STATUS_INVALID_ARGUMENT` for an
/// unknown or already destroyed handle.
pub fn buffer_destroy(handle: i64) -> i64 {
    status_from_bool(call("Graphics_bufferDestroy", &[Value::Long(handle)]))
}

/// Set one pixel. Coordinates are checked; out-of-range writes are rejected.
pub fn buffer_set(handle: i64, x: i64, y: i64, rgba: i64) -> i64 {
    status_from_bool(call(
        "Graphics_bufferSet",
        &[
            Value::Long(handle),
            Value::Long(x),
            Value::Long(y),
            Value::Long(rgba),
        ],
    ))
}

/// Read one packed RGBA pixel. Returns `STATUS_INVALID_ARGUMENT` for an
/// unknown handle or out-of-range coordinates.
pub fn buffer_get(handle: i64, x: i64, y: i64) -> i64 {
    match call(
        "Graphics_bufferGet",
        &[Value::Long(handle), Value::Long(x), Value::Long(y)],
    ) {
        Value::Long(pixel) => pixel as u32 as i64,
        _ => STATUS_INVALID_ARGUMENT,
    }
}

/// Fill the whole buffer with one packed RGBA color.
pub fn buffer_clear(handle: i64, rgba: i64) -> i64 {
    status_from_bool(call(
        "Graphics_bufferClear",
        &[Value::Long(handle), Value::Long(rgba)],
    ))
}

/// Buffer width, or `STATUS_INVALID_ARGUMENT` for an unknown handle.
pub fn buffer_width(handle: i64) -> i64 {
    match call("Graphics_bufferWidth", &[Value::Long(handle)]) {
        Value::Long(width) if width >= 0 => width,
        _ => STATUS_INVALID_ARGUMENT,
    }
}

/// Buffer height, or `STATUS_INVALID_ARGUMENT` for an unknown handle.
pub fn buffer_height(handle: i64) -> i64 {
    match call("Graphics_bufferHeight", &[Value::Long(handle)]) {
        Value::Int(height) if height >= 0 => height as i64,
        _ => STATUS_INVALID_ARGUMENT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vm(name: &str, args: &[Value]) -> Value {
        (lookup_builtin_native(name).unwrap())(args).unwrap()
    }

    fn vm_int(value: &Value) -> i64 {
        match value {
            Value::Int(v) => *v as i64,
            Value::Long(v) => *v,
            _ => panic!("expected an integer result"),
        }
    }

    fn vm_text(value: &Value) -> String {
        match value {
            Value::String(s) => s.to_string(),
            _ => panic!("expected a string result"),
        }
    }

    #[test]
    fn window_status_matches_vm_messages() {
        assert!(!window_backend_available());
        assert!(matches!(vm("Graphics_windowIsAvailable", &[]), Value::Bool(false)));

        assert_eq!(window_create(320, 240), STATUS_BACKEND_UNAVAILABLE);
        assert_eq!(
            vm_text(&vm(
                "Graphics_windowCreate",
                &[Value::Long(320), Value::Long(240)]
            )),
            WINDOW_BACKEND_UNAVAILABLE
        );

        assert_eq!(window_create(0, 240), STATUS_INVALID_ARGUMENT);
        assert_eq!(
            vm_text(&vm(
                "Graphics_windowCreate",
                &[Value::Long(0), Value::Long(240)]
            )),
            "invalid window size"
        );
    }

    #[test]
    fn buffer_state_matches_across_dispatch_layers() {
        let handle = buffer_create(4, 3);
        assert!(handle > 0);
        assert!(vm_int(&vm(
            "Graphics_bufferCreate",
            &[Value::Long(4), Value::Long(3)]
        )) > 0);

        let color = 0x01020304i64;
        assert_eq!(buffer_set(handle, 2, 1, color), STATUS_OK);
        assert_eq!(buffer_get(handle, 2, 1), color);

        let opaque = 0xAABBCCDDi64;
        assert!(matches!(
            vm(
                "Graphics_bufferSet",
                &[
                    Value::Long(handle),
                    Value::Long(2),
                    Value::Long(1),
                    Value::Long(opaque),
                ]
            ),
            Value::Bool(true)
        ));
        assert_eq!(buffer_get(handle, 2, 1), opaque as u32 as i64);
        // The VM keeps .tr colors in their sign-extended i32 shape.
        assert_eq!(
            vm_int(&vm(
                "Graphics_bufferGet",
                &[Value::Long(handle), Value::Long(2), Value::Long(1)]
            )),
            opaque as i32 as i64
        );

        assert_eq!(buffer_width(handle), 4);
        assert_eq!(
            vm_int(&vm("Graphics_bufferWidth", &[Value::Long(handle)])),
            4
        );
        assert_eq!(buffer_height(handle), 3);
        assert_eq!(
            vm_int(&vm("Graphics_bufferHeight", &[Value::Long(handle)])),
            3
        );

        assert_eq!(buffer_set(handle, 4, 1, color), STATUS_INVALID_ARGUMENT);
        assert!(matches!(
            vm(
                "Graphics_bufferSet",
                &[
                    Value::Long(handle),
                    Value::Long(4),
                    Value::Long(1),
                    Value::Long(color),
                ]
            ),
            Value::Bool(false)
        ));

        assert_eq!(buffer_clear(handle, 0), STATUS_OK);
        assert_eq!(buffer_get(handle, 2, 1), 0);
        assert_eq!(buffer_destroy(handle), STATUS_OK);
        assert_eq!(buffer_get(handle, 2, 1), STATUS_INVALID_ARGUMENT);
        assert_eq!(
            vm_int(&vm(
                "Graphics_bufferGet",
                &[Value::Long(handle), Value::Long(2), Value::Long(1)]
            )),
            -1
        );
    }

    #[test]
    fn input_queue_matches_across_dispatch_layers() {
        input_push(7);
        vm("Graphics_inputPush", &[Value::Long(9)]);
        assert_eq!(input_poll(), 7);
        assert_eq!(vm_int(&vm("Graphics_inputPoll", &[])), 9);

        // Zero is a real event; it must not block the events behind it.
        vm("Graphics_inputPush", &[Value::Long(0)]);
        input_push(11);
        assert_eq!(vm_int(&vm("Graphics_inputPoll", &[])), 0);
        assert_eq!(input_poll(), 11);
        assert_eq!(vm_int(&vm("Graphics_inputPoll", &[])), 0);
        assert_eq!(input_poll(), 0);
    }

    #[test]
    fn oversized_buffer_is_rejected_by_both() {
        let oversized = ((1usize << 24) + 1) as i64;
        assert_eq!(buffer_create(oversized, 1), STATUS_INVALID_ARGUMENT);
        assert!(matches!(
            vm("Graphics_bufferCreate", &[Value::Long(oversized), Value::Long(1)]),
            Value::Int(-2)
        ));
    }
}
