// VM-side graphics and audio natives.
//
// These mirror the host primitives in titrate_native so that the same .tr
// source works on the bytecode VM and through the native C-ABI wrappers. The
// two registries are intentionally independent: each backend owns its own
// thread-local state.

use super::super::super::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    static BUFFERS: RefCell<HashMap<i64, Vec<i32>>> = RefCell::new(HashMap::new());
    static NEXT_BUFFER: std::cell::Cell<i64> = const { std::cell::Cell::new(1) };
    static INPUT: RefCell<Vec<i64>> = const { RefCell::new(Vec::new()) };
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

fn ints_from(args: &[Value], index: usize) -> Vec<i64> {
    match args.get(index) {
        Some(Value::Array { elements }) => elements.iter().map(|v| v.to_i64().unwrap_or(0)).collect(),
        _ => Vec::new(),
    }
}

pub(crate) fn native_graphics_window_create(args: &[Value]) -> Result<Value, String> {
    let w = int_arg(args, 0);
    let h = int_arg(args, 1);
    if w <= 0 || h <= 0 {
        return Ok(Value::String(Rc::new("invalid window size".to_string())));
    }
    Ok(Value::String(Rc::new("windowing backend unavailable".to_string())))
}

pub(crate) fn native_graphics_window_is_available(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(false))
}

pub(crate) fn native_graphics_input_push(args: &[Value]) -> Result<Value, String> {
    INPUT.with(|q| q.borrow_mut().push(int_arg(args, 0)));
    Ok(Value::Void)
}

pub(crate) fn native_graphics_input_poll(_args: &[Value]) -> Result<Value, String> {
    let event = INPUT.with(|q| q.borrow_mut().first().copied().unwrap_or(0));
    if event != 0 {
        INPUT.with(|q| {
            let mut q = q.borrow_mut();
            if !q.is_empty() {
                q.remove(0);
            }
        });
    }
    Ok(Value::Int(event as i32))
}

pub(crate) fn native_graphics_buffer_create(args: &[Value]) -> Result<Value, String> {
    let w = int_arg(args, 0);
    let h = int_arg(args, 1);
    if w <= 0 || h <= 0 || w.checked_mul(h).is_none() {
        return Ok(Value::Int(-2));
    }
    let len = match usize::try_from(w.checked_mul(h).unwrap_or(0)) {
        Ok(n) if n <= (1 << 28) => n,
        _ => return Ok(Value::Int(-2)),
    };
    let handle = NEXT_BUFFER.with(|c| {
        let h = c.get();
        c.set(h + 1);
        h
    });
    BUFFERS.with(|m| {
        m.borrow_mut()
            .insert(handle, vec![0; len]);
    });
    Ok(Value::Long(handle))
}

pub(crate) fn native_graphics_buffer_destroy(args: &[Value]) -> Result<Value, String> {
    let handle = int_arg(args, 0);
    let removed = BUFFERS.with(|m| m.borrow_mut().remove(&handle).is_some());
    Ok(Value::Bool(removed))
}

pub(crate) fn native_graphics_buffer_set(args: &[Value]) -> Result<Value, String> {
    let handle = int_arg(args, 0);
    let x = int_arg(args, 1);
    let y = int_arg(args, 2);
    let color = int_arg(args, 3);
    let result = BUFFERS.with(|m| {
        let mut m = m.borrow_mut();
        match m.get_mut(&handle) {
            Some(buf) => {
                let (w, h) = (int_arg(args, 4), int_arg(args, 5));
                let w = if w > 0 { w as usize } else { 0 };
                let h = if h > 0 { h as usize } else { 0 };
                if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
                    return false;
                }
                let idx = y as usize * w + x as usize;
                if idx >= buf.len() {
                    return false;
                }
                buf[idx] = color as i32;
                true
            }
            None => false,
        }
    });
    Ok(Value::Bool(result))
}

pub(crate) fn native_graphics_buffer_get(args: &[Value]) -> Result<Value, String> {
    let handle = int_arg(args, 0);
    let x = int_arg(args, 1);
    let y = int_arg(args, 2);
    let w = int_arg(args, 3);
    let value = BUFFERS.with(|m| {
        let m = m.borrow();
        match m.get(&handle) {
            Some(buf) if w > 0 && x >= 0 && y >= 0 => {
                let idx = y as usize * w as usize + x as usize;
                buf.get(idx).copied().map(|v| v as i64)
            }
            _ => None,
        }
    });
    Ok(value.map(Value::Long).unwrap_or(Value::Int(-1)))
}

pub(crate) fn native_graphics_buffer_clear(args: &[Value]) -> Result<Value, String> {
    let handle = int_arg(args, 0);
    let color = int_arg(args, 1) as i32;
    let ok = BUFFERS.with(|m| {
        let mut m = m.borrow_mut();
        if let Some(buf) = m.get_mut(&handle) {
            for v in buf.iter_mut() {
                *v = color;
            }
            true
        } else {
            false
        }
    });
    Ok(Value::Bool(ok))
}

pub(crate) fn native_graphics_buffer_width(args: &[Value]) -> Result<Value, String> {
    let handle = int_arg(args, 0);
    let n = BUFFERS.with(|m| m.borrow().get(&handle).map(|b| b.len() as i64).unwrap_or(-1));
    Ok(Value::Long(n))
}

pub(crate) fn native_graphics_buffer_height(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Int(1))
}

fn wave(args: &[Value], kind: &str) -> Result<Value, String> {
    let freq = f64_arg(args, 0);
    let rate = f64_arg(args, 1);
    let frames = int_arg(args, 2);
    let amp = f64_arg(args, 3);
    let duty = if kind == "square" { f64_arg(args, 4) } else { 0.5 };
    if rate <= 0.0 || frames <= 0 || frames > (1 << 22) {
        return Err("Synth: invalid wave parameters".to_string());
    }
    let mut samples = Vec::with_capacity(frames as usize);
    for i in 0..frames {
        let phase = (i as f64 * freq / rate).fract();
        let value = match kind {
            "sine" => (std::f64::consts::TAU * phase).sin(),
            "square" => {
                if phase < duty { 1.0 } else { -1.0 }
            }
            "saw" => 2.0 * phase - 1.0,
            _ => 0.0,
        };
        samples.push(Value::Double(value * amp));
    }
    Ok(Value::Array { elements: samples })
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
    let mut seed = int_arg(args, 0) as u64;
    let frames = int_arg(args, 1);
    let amp = f64_arg(args, 2);
    if frames <= 0 || frames > (1 << 22) {
        return Err("Synth: invalid frame count".to_string());
    }
    let mut samples = Vec::with_capacity(frames as usize);
    for _ in 0..frames {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let unit = (seed as f64) / (u64::MAX as f64);
        samples.push(Value::Double((2.0 * unit - 1.0) * amp));
    }
    Ok(Value::Array { elements: samples })
}

pub(crate) fn native_synth_mix(args: &[Value]) -> Result<Value, String> {
    let a = ints_from(args, 0);
    let b = ints_from(args, 1);
    let n = a.len().max(b.len()).min(1 << 22);
    let mut samples = Vec::with_capacity(n);
    for i in 0..n {
        let av = a.get(i).copied().unwrap_or(0) as f64;
        let bv = b.get(i).copied().unwrap_or(0) as f64;
        samples.push(Value::Double((av + bv).clamp(-1.0, 1.0)));
    }
    Ok(Value::Array { elements: samples })
}

pub(crate) fn native_audio_device_is_available(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(false))
}

pub(crate) fn native_audio_device_status(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Int(-2))
}

pub(crate) fn native_audio_device_play(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::String(Rc::new("audio output unavailable".to_string())))
}
