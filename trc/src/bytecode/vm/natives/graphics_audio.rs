// VM-side graphics and audio natives.
//
// This module owns the single store behind both dispatch layers: the bytecode
// natives here and the C-ABI host primitives in titrate_native, which call
// these natives through `lookup_builtin_native`. Each layer only converts
// arguments and results to its own representation.

use super::super::super::value::Value;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

const MAX_PIXELS: usize = 1 << 24;
const MAX_FRAMES: i64 = 1 << 24;

const STATUS_OK: i64 = 0;
const STATUS_BACKEND_UNAVAILABLE: i64 = -1;
const STATUS_INVALID_ARGUMENT: i64 = -2;

const WINDOW_BACKEND_UNAVAILABLE: &str = "windowing backend unavailable";
const INVALID_WINDOW_SIZE: &str = "invalid window size";
const AUDIO_OUTPUT_UNAVAILABLE: &str = "audio output unavailable";

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

fn window_create(width: i64, height: i64) -> i64 {
    if width <= 0 || height <= 0 {
        STATUS_INVALID_ARGUMENT
    } else {
        STATUS_BACKEND_UNAVAILABLE
    }
}

fn window_backend_available() -> bool {
    false
}

fn input_push(code: i64) {
    INPUT_QUEUE.with(|queue| queue.borrow_mut().push_back(code));
}

fn input_poll() -> i64 {
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

fn buffer_create(width: i64, height: i64) -> i64 {
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
    BUFFERS.with(|buffers| {
        buffers.borrow_mut().insert(
            handle,
            PixelBuffer {
                width,
                height,
                pixels: vec![0; count],
            },
        );
    });
    handle
}

fn buffer_destroy(handle: i64) -> i64 {
    BUFFERS.with(|buffers| {
        if buffers.borrow_mut().remove(&handle).is_some() {
            STATUS_OK
        } else {
            STATUS_INVALID_ARGUMENT
        }
    })
}

fn buffer_set(handle: i64, x: i64, y: i64, rgba: i64) -> i64 {
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

fn buffer_get(handle: i64, x: i64, y: i64) -> i64 {
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

fn buffer_clear(handle: i64, rgba: i64) -> i64 {
    let cleared = with_buffer_mut(handle, |buffer| {
        buffer.pixels.fill(rgba as u32);
    });
    match cleared {
        Some(()) => STATUS_OK,
        None => STATUS_INVALID_ARGUMENT,
    }
}

fn buffer_width(handle: i64) -> i64 {
    with_buffer(handle, |buffer| buffer.width as i64).unwrap_or(STATUS_INVALID_ARGUMENT)
}

fn buffer_height(handle: i64) -> i64 {
    with_buffer(handle, |buffer| buffer.height as i64).unwrap_or(STATUS_INVALID_ARGUMENT)
}

fn frame_count(frames: i64) -> usize {
    if frames <= 0 {
        return 0;
    }
    (frames as usize).min(MAX_FRAMES as usize)
}

fn amplitude(amp: f64) -> f64 {
    if !amp.is_finite() {
        return 0.0;
    }
    amp.abs().min(1.0)
}

fn frequency(freq: f64) -> f64 {
    if freq.is_finite() && freq > 0.0 {
        freq
    } else {
        0.0
    }
}

fn sample_rate(rate: f64) -> f64 {
    if rate.is_finite() && rate > 0.0 {
        rate
    } else {
        44100.0
    }
}

fn sine(freq: f64, rate: f64, frames: i64, amp: f64) -> Vec<f64> {
    let n = frame_count(frames);
    let amp = amplitude(amp);
    let step = std::f64::consts::TAU * frequency(freq) / sample_rate(rate);
    let mut phase: f64 = 0.0;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(amp * phase.sin());
        phase = (phase + step) % std::f64::consts::TAU;
    }
    out
}

fn square(freq: f64, rate: f64, frames: i64, amp: f64, duty: f64) -> Vec<f64> {
    let n = frame_count(frames);
    let amp = amplitude(amp);
    let mut duty = if duty.is_finite() { duty } else { 0.5 };
    if duty <= 0.0 || duty >= 1.0 {
        duty = 0.5;
    }
    let step = frequency(freq) / sample_rate(rate);
    let mut phase = 0.0;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(if phase < duty { amp } else { -amp });
        phase = (phase + step) % 1.0;
    }
    out
}

fn saw(freq: f64, rate: f64, frames: i64, amp: f64) -> Vec<f64> {
    let n = frame_count(frames);
    let amp = amplitude(amp);
    let step = frequency(freq) / sample_rate(rate);
    let mut phase = 0.0;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(amp * (2.0 * phase - 1.0));
        phase = (phase + step) % 1.0;
    }
    out
}

fn noise(seed: i64, frames: i64, amp: f64) -> Vec<f64> {
    let n = frame_count(frames);
    let amp = amplitude(amp);
    let mut state = ((seed as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)) | 1;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let value = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        out.push((value as f64 / u64::MAX as f64) * 2.0 * amp - amp);
    }
    out
}

fn mix(a: &[f64], b: &[f64]) -> Vec<f64> {
    let n = a.len().max(b.len());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(0.0);
        let y = b.get(i).copied().unwrap_or(0.0);
        out.push((x + y).clamp(-1.0, 1.0));
    }
    out
}

fn int_arg(args: &[Value], index: usize) -> i64 {
    match args.get(index) {
        Some(Value::Int(v)) => *v as i64,
        Some(Value::Long(v)) => *v,
        _ => 0,
    }
}

fn f64_arg(args: &[Value], index: usize) -> f64 {
    match args.get(index) {
        Some(Value::Double(v)) => *v,
        Some(Value::Float(v)) => *v as f64,
        Some(Value::Int(v)) => *v as f64,
        Some(Value::Long(v)) => *v as f64,
        _ => 0.0,
    }
}

pub(crate) fn native_graphics_window_create(args: &[Value]) -> Result<Value, String> {
    let message = match window_create(int_arg(args, 0), int_arg(args, 1)) {
        STATUS_BACKEND_UNAVAILABLE => WINDOW_BACKEND_UNAVAILABLE,
        STATUS_INVALID_ARGUMENT => INVALID_WINDOW_SIZE,
        _ => "",
    };
    Ok(Value::String(Rc::new(message.to_string())))
}

pub(crate) fn native_graphics_window_is_available(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(window_backend_available()))
}

pub(crate) fn native_graphics_input_push(args: &[Value]) -> Result<Value, String> {
    input_push(int_arg(args, 0));
    Ok(Value::Void)
}

pub(crate) fn native_graphics_input_poll(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Int(input_poll() as i32))
}

pub(crate) fn native_graphics_buffer_create(args: &[Value]) -> Result<Value, String> {
    let handle = buffer_create(int_arg(args, 0), int_arg(args, 1));
    Ok(if handle > 0 {
        Value::Long(handle)
    } else {
        Value::Int(handle as i32)
    })
}

pub(crate) fn native_graphics_buffer_destroy(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(buffer_destroy(int_arg(args, 0)) == STATUS_OK))
}

pub(crate) fn native_graphics_buffer_set(args: &[Value]) -> Result<Value, String> {
    let ok = buffer_set(
        int_arg(args, 0),
        int_arg(args, 1),
        int_arg(args, 2),
        int_arg(args, 3),
    ) == STATUS_OK;
    Ok(Value::Bool(ok))
}

pub(crate) fn native_graphics_buffer_get(args: &[Value]) -> Result<Value, String> {
    let pixel = buffer_get(int_arg(args, 0), int_arg(args, 1), int_arg(args, 2));
    Ok(if pixel < 0 {
        Value::Int(-1)
    } else {
        // .tr stores colors as i32; keep the sign-extended shape.
        Value::Long(pixel as i32 as i64)
    })
}

pub(crate) fn native_graphics_buffer_clear(args: &[Value]) -> Result<Value, String> {
    let ok = buffer_clear(int_arg(args, 0), int_arg(args, 1)) == STATUS_OK;
    Ok(Value::Bool(ok))
}

pub(crate) fn native_graphics_buffer_width(args: &[Value]) -> Result<Value, String> {
    let width = buffer_width(int_arg(args, 0));
    Ok(Value::Long(if width < 0 { -1 } else { width }))
}

pub(crate) fn native_graphics_buffer_height(args: &[Value]) -> Result<Value, String> {
    let height = buffer_height(int_arg(args, 0));
    Ok(Value::Int(if height < 0 { -1 } else { height as i32 }))
}

fn wave(args: &[Value], kind: &str) -> Result<Value, String> {
    let freq = f64_arg(args, 0);
    let rate = f64_arg(args, 1);
    let frames = int_arg(args, 2);
    let amp = f64_arg(args, 3);
    let duty = if kind == "square" { f64_arg(args, 4) } else { 0.5 };
    if rate <= 0.0 || frames <= 0 || frames > MAX_FRAMES {
        return Err("Synth: invalid wave parameters".to_string());
    }
    let samples = match kind {
        "sine" => sine(freq, rate, frames, amp),
        "square" => square(freq, rate, frames, amp, duty),
        "saw" => saw(freq, rate, frames, amp),
        _ => vec![0.0; frame_count(frames)],
    };
    Ok(Value::Array {
        elements: samples.into_iter().map(Value::Double).collect(),
    })
}

pub(crate) fn native_synth_sine(args: &[Value]) -> Result<Value, String> {
    wave(args, "sine")
}

pub(crate) fn native_synth_square(args: &[Value]) -> Result<Value, String> {
    wave(args, "square")
}

pub(crate) fn native_synth_saw(args: &[Value]) -> Result<Value, String> {
    wave(args, "saw")
}

pub(crate) fn native_synth_noise(args: &[Value]) -> Result<Value, String> {
    let seed = int_arg(args, 0);
    let frames = int_arg(args, 1);
    let amp = f64_arg(args, 2);
    if frames <= 0 || frames > MAX_FRAMES {
        return Err("Synth: invalid frame count".to_string());
    }
    Ok(Value::Array {
        elements: noise(seed, frames, amp)
            .into_iter()
            .map(Value::Double)
            .collect(),
    })
}

fn doubles_from(args: &[Value], index: usize) -> Vec<f64> {
    match args.get(index) {
        Some(Value::Array { elements }) => elements.iter().map(|v| v.to_f64().unwrap_or(0.0)).collect(),
        Some(Value::ClassInstance { fields, .. }) => {
            let borrowed = fields.borrow();
            match borrowed.get("_elements") {
                Some(Value::Array { elements }) => {
                    elements.iter().map(|v| v.to_f64().unwrap_or(0.0)).collect()
                }
                _ => Vec::new(),
            }
        }
        _ => Vec::new(),
    }
}

pub(crate) fn native_synth_mix(args: &[Value]) -> Result<Value, String> {
    let a = doubles_from(args, 0);
    let b = doubles_from(args, 1);
    let cap = MAX_FRAMES as usize;
    let samples = mix(&a[..a.len().min(cap)], &b[..b.len().min(cap)]);
    Ok(Value::Array {
        elements: samples.into_iter().map(Value::Double).collect(),
    })
}

pub(crate) fn native_audio_device_is_available(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(audio_output_available()))
}

pub(crate) fn native_audio_device_status(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Int(STATUS_BACKEND_UNAVAILABLE as i32))
}

pub(crate) fn native_audio_device_play(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::String(Rc::new(AUDIO_OUTPUT_UNAVAILABLE.to_string())))
}

fn audio_output_available() -> bool {
    false
}
