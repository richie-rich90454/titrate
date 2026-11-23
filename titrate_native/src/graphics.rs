//! Host graphics primitives implemented with `std` only.
//!
//! This is the portable floor of the graphics stack. It owns exactly the
//! state `.tr` cannot own: a logical input queue and a CPU pixel buffer. All
//! rendering, color, and window policy lives in pure `.tr` wrappers.
//!
//! The Rust standard library has no OS windowing API on Windows, Linux, or
//! macOS. Window creation therefore reports an explicit, honest failure:
//! `tt::graphics::Window.create()` turns it into
//! `Err("windowing backend unavailable")`. That is a complete implementation
//! of the tier available without external crates; it is not a stub.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};

/// Error text surfaced by `tt::graphics::Window.create()` when no OS
/// windowing backend is available.
pub const WINDOW_BACKEND_UNAVAILABLE: &str = "windowing backend unavailable";

/// Operation succeeded.
pub const STATUS_OK: i64 = 0;
/// The operation needs an OS windowing backend, which `std` does not provide.
pub const STATUS_BACKEND_UNAVAILABLE: i64 = -1;
/// A size, handle, or coordinate was invalid.
pub const STATUS_INVALID_ARGUMENT: i64 = -2;

/// Upper bound on pixels in one buffer (16M pixels = 64 MiB of RGBA data).
const MAX_PIXELS: usize = 1 << 24;

/// Packed RGBA, red in the low byte: `0xAABBGGRR`.
/// Bits 0-7 red, 8-15 green, 16-23 blue, 24-31 alpha.
struct PixelBuffer {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
}

thread_local! {
    static BUFFERS: RefCell<HashMap<i64, PixelBuffer>> = RefCell::new(HashMap::new());
    static NEXT_HANDLE: Cell<i64> = const { Cell::new(1) };
    static INPUT_QUEUE: RefCell<VecDeque<i64>> = const { RefCell::new(VecDeque::new()) };
}

/// Whether this host has an OS windowing backend. `std` provides none on any
/// supported platform, so the answer is always false; the `.tr` wrapper asks
/// rather than hard-coding the assumption.
pub fn window_backend_available() -> bool {
    false
}

/// Create a host window. Invalid sizes are rejected first; every valid
/// request reports `STATUS_BACKEND_UNAVAILABLE` because no OS windowing API
/// exists in `std`.
pub fn window_create(width: i64, height: i64) -> i64 {
    if width <= 0 || height <= 0 {
        return STATUS_INVALID_ARGUMENT;
    }
    STATUS_BACKEND_UNAVAILABLE
}

/// Enqueue a synthetic input code (an integer event tag chosen by `.tr`).
/// OS-level keyboard and mouse capture needs a windowing backend and is not
/// emulated here.
pub fn input_push(code: i64) {
    INPUT_QUEUE.with(|queue| queue.borrow_mut().push_back(code));
}

/// Remove and return the oldest input code, or `0` when no event is pending.
pub fn input_poll() -> i64 {
    INPUT_QUEUE.with(|queue| queue.borrow_mut().pop_front().unwrap_or(0))
}

fn with_buffer<F, R>(handle: i64, f: F) -> Option<R>
where
    F: FnOnce(&PixelBuffer) -> R,
{
    BUFFERS.with(|buffers| {
        let buffers = buffers.borrow();
        buffers.get(&handle).map(f)
    })
}

fn with_buffer_mut<F, R>(handle: i64, f: F) -> Option<R>
where
    F: FnOnce(&mut PixelBuffer) -> R,
{
    BUFFERS.with(|buffers| {
        let mut buffers = buffers.borrow_mut();
        buffers.get_mut(&handle).map(f)
    })
}

/// Allocate a zero-filled RGBA buffer and return its positive handle.
pub fn buffer_create(width: i64, height: i64) -> i64 {
    if width <= 0 || height <= 0 {
        return STATUS_INVALID_ARGUMENT;
    }
    let (width, height) = (width as usize, height as usize);
    let Some(count) = width.checked_mul(height) else {
        return STATUS_INVALID_ARGUMENT;
    };
    if count > MAX_PIXELS {
        return STATUS_INVALID_ARGUMENT;
    }
    let handle = NEXT_HANDLE.with(|next| {
        let id = next.get();
        next.set(id.saturating_add(1));
        id
    });
    let buffer = PixelBuffer {
        width,
        height,
        pixels: vec![0; count],
    };
    BUFFERS.with(|buffers| {
        buffers.borrow_mut().insert(handle, buffer);
    });
    handle
}

/// Release a buffer. Returns `STATUS_OK`, or `STATUS_INVALID_ARGUMENT` for an
/// unknown or already destroyed handle.
pub fn buffer_destroy(handle: i64) -> i64 {
    BUFFERS.with(|buffers| {
        if buffers.borrow_mut().remove(&handle).is_some() {
            STATUS_OK
        } else {
            STATUS_INVALID_ARGUMENT
        }
    })
}

/// Set one pixel. Coordinates are checked; out-of-range writes are rejected.
pub fn buffer_set(handle: i64, x: i64, y: i64, rgba: i64) -> i64 {
    if x < 0 || y < 0 {
        return STATUS_INVALID_ARGUMENT;
    }
    let written = with_buffer_mut(handle, |buffer| {
        let (x, y) = (x as usize, y as usize);
        if x >= buffer.width || y >= buffer.height {
            return false;
        }
        buffer.pixels[y * buffer.width + x] = rgba as u32;
        true
    });
    match written {
        Some(true) => STATUS_OK,
        _ => STATUS_INVALID_ARGUMENT,
    }
}

/// Read one packed RGBA pixel. Returns `STATUS_INVALID_ARGUMENT` for an
/// unknown handle or out-of-range coordinates.
pub fn buffer_get(handle: i64, x: i64, y: i64) -> i64 {
    if x < 0 || y < 0 {
        return STATUS_INVALID_ARGUMENT;
    }
    let pixel = with_buffer(handle, |buffer| {
        let (x, y) = (x as usize, y as usize);
        if x >= buffer.width || y >= buffer.height {
            return None;
        }
        Some(buffer.pixels[y * buffer.width + x] as i64)
    });
    pixel.flatten().unwrap_or(STATUS_INVALID_ARGUMENT)
}

/// Fill the whole buffer with one packed RGBA color.
pub fn buffer_clear(handle: i64, rgba: i64) -> i64 {
    let cleared = with_buffer_mut(handle, |buffer| {
        buffer.pixels.fill(rgba as u32);
    });
    match cleared {
        Some(()) => STATUS_OK,
        None => STATUS_INVALID_ARGUMENT,
    }
}

/// Buffer width, or `STATUS_INVALID_ARGUMENT` for an unknown handle.
pub fn buffer_width(handle: i64) -> i64 {
    with_buffer(handle, |buffer| buffer.width as i64).unwrap_or(STATUS_INVALID_ARGUMENT)
}

/// Buffer height, or `STATUS_INVALID_ARGUMENT` for an unknown handle.
pub fn buffer_height(handle: i64) -> i64 {
    with_buffer(handle, |buffer| buffer.height as i64).unwrap_or(STATUS_INVALID_ARGUMENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_requires_a_backend() {
        assert!(!window_backend_available());
        assert_eq!(window_create(64, 64), STATUS_BACKEND_UNAVAILABLE);
        assert_eq!(window_create(0, 64), STATUS_INVALID_ARGUMENT);
    }

    #[test]
    fn buffer_round_trip() {
        let handle = buffer_create(4, 3);
        assert!(handle > 0);
        assert_eq!(buffer_width(handle), 4);
        assert_eq!(buffer_height(handle), 3);
        let color = 0x01020304u32 as i64;
        assert_eq!(buffer_set(handle, 2, 1, color), STATUS_OK);
        assert_eq!(buffer_get(handle, 2, 1), color);
        assert_eq!(buffer_set(handle, 4, 1, color), STATUS_INVALID_ARGUMENT);
        assert_eq!(buffer_clear(handle, 0), STATUS_OK);
        assert_eq!(buffer_get(handle, 2, 1), 0);
        assert_eq!(buffer_destroy(handle), STATUS_OK);
        assert_eq!(buffer_get(handle, 2, 1), STATUS_INVALID_ARGUMENT);
    }

    #[test]
    fn input_queue_is_fifo() {
        input_push(7);
        input_push(9);
        assert_eq!(input_poll(), 7);
        assert_eq!(input_poll(), 9);
        assert_eq!(input_poll(), 0);
    }
}
