use std::rc::Rc;

use trc::bytecode::value::Value;

use crate::{titrate_to_value, value_to_titrate, TitrateValue};

/// Read the uniform C-ABI argument list into VM values.
unsafe fn arg_values(args: *const TitrateValue, arg_count: usize) -> Vec<Value> {
    if args.is_null() || arg_count == 0 {
        return Vec::new();
    }
    (0..arg_count)
        .map(|i| titrate_to_value(&std::ptr::read_unaligned(args.add(i))))
        .collect()
}

fn as_f64(value: Option<&Value>, default: f64) -> f64 {
    match value {
        Some(Value::Double(d)) => *d,
        Some(Value::Float(f)) => *f as f64,
        Some(Value::Int(i)) => *i as f64,
        Some(Value::Long(l)) => *l as f64,
        _ => default,
    }
}

fn as_i64(value: Option<&Value>, default: i64) -> i64 {
    match value {
        Some(Value::Int(i)) => *i as i64,
        Some(Value::Long(l)) => *l,
        Some(Value::Double(d)) => *d as i64,
        _ => default,
    }
}

fn as_doubles(value: Option<&Value>) -> Vec<f64> {
    match value {
        Some(Value::Array { elements }) => elements.iter().map(|e| as_f64(Some(e), 0.0)).collect(),
        _ => Vec::new(),
    }
}

fn double_array(samples: Vec<f64>) -> TitrateValue {
    let elements = samples.into_iter().map(Value::Double).collect();
    value_to_titrate(&Value::Array { elements })
}

/// Empty string on success, exact unavailable text on failure. The `.tr`
/// wrapper turns this into `Err(message)`, matching the graphics host pattern.
fn output_message() -> TitrateValue {
    if crate::audio::output_available() {
        value_to_titrate(&Value::String(Rc::new(String::new())))
    } else {
        value_to_titrate(&Value::String(Rc::new(
            crate::audio::output_reason().to_string(),
        )))
    }
}

// ---------------------------------------------------------------------------
// AudioDevice — output capability is reported, never fabricated
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn titrate_AudioDevice_isAvailable(
    _args: *const TitrateValue,
    _arg_count: usize,
) -> TitrateValue {
    value_to_titrate(&Value::Bool(crate::audio::output_available()))
}

#[no_mangle]
pub unsafe extern "C" fn titrate_AudioDevice_status(
    _args: *const TitrateValue,
    _arg_count: usize,
) -> TitrateValue {
    value_to_titrate(&Value::Int(crate::audio::output_status()))
}

/// Playback always reports the unavailable device on a std-only host; the
/// buffer is consumed only so callers get the same error shape for any input.
#[no_mangle]
pub unsafe extern "C" fn titrate_AudioDevice_play(
    args: *const TitrateValue,
    arg_count: usize,
) -> TitrateValue {
    let values = arg_values(args, arg_count);
    let samples = as_doubles(values.first());
    if samples.is_empty() {
        return value_to_titrate(&Value::String(Rc::new(
            crate::audio::output_reason().to_string(),
        )));
    }
    output_message()
}

// ---------------------------------------------------------------------------
// Synth — primitive PCM sample generation (sequencing stays in .tr)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn titrate_Synth_sine(
    args: *const TitrateValue,
    arg_count: usize,
) -> TitrateValue {
    let a = arg_values(args, arg_count);
    double_array(crate::audio::sine(
        as_f64(a.first(), 440.0),
        as_f64(a.get(1), 44100.0),
        as_i64(a.get(2), 0),
        as_f64(a.get(3), 1.0),
    ))
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Synth_square(
    args: *const TitrateValue,
    arg_count: usize,
) -> TitrateValue {
    let a = arg_values(args, arg_count);
    double_array(crate::audio::square(
        as_f64(a.first(), 440.0),
        as_f64(a.get(1), 44100.0),
        as_i64(a.get(2), 0),
        as_f64(a.get(3), 1.0),
        as_f64(a.get(4), 0.5),
    ))
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Synth_saw(
    args: *const TitrateValue,
    arg_count: usize,
) -> TitrateValue {
    let a = arg_values(args, arg_count);
    double_array(crate::audio::saw(
        as_f64(a.first(), 440.0),
        as_f64(a.get(1), 44100.0),
        as_i64(a.get(2), 0),
        as_f64(a.get(3), 1.0),
    ))
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Synth_noise(
    args: *const TitrateValue,
    arg_count: usize,
) -> TitrateValue {
    let a = arg_values(args, arg_count);
    double_array(crate::audio::noise(
        as_i64(a.first(), 0),
        as_i64(a.get(1), 0),
        as_f64(a.get(2), 1.0),
    ))
}

#[no_mangle]
pub unsafe extern "C" fn titrate_Synth_mix(
    args: *const TitrateValue,
    arg_count: usize,
) -> TitrateValue {
    let a = arg_values(args, arg_count);
    let left = as_doubles(a.first());
    let right = as_doubles(a.get(1));
    double_array(crate::audio::mix(&left, &right))
}
