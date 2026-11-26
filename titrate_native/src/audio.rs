//! Pure-Rust PCM sample generation and mixing for the audio host primitives.
//!
//! This module is a C-ABI dispatcher over the single PCM implementation in
//! `trc::bytecode::vm::natives::graphics_audio`. The bytecode VM and the
//! native host generate identical samples from identical arguments; only the
//! argument/result representations differ here.
//!
//! `std` exposes no audio output device, so this module never guesses at a
//! playback backend. It reports output availability explicitly; the `.tr`
//! layer does the sequencing.

use trc::bytecode::value::Value;
use trc::bytecode::vm::natives::lookup_builtin_native;

/// Largest sample buffer a single primitive call may produce (16M frames).
pub const MAX_FRAMES: usize = 1 << 24;

/// Error text surfaced by the audio output wrapper when no device exists.
pub const AUDIO_OUTPUT_UNAVAILABLE: &str = "audio output unavailable";

/// Status codes returned by the raw C-ABI primitives.
pub const STATUS_OK: i32 = 0;
pub const STATUS_INVALID_ARG: i32 = -1;
pub const STATUS_UNAVAILABLE: i32 = -2;
pub const STATUS_ALLOC_FAILED: i32 = -3;

fn call(name: &str, args: &[Value]) -> Value {
    match lookup_builtin_native(name) {
        Some(native) => native(args).unwrap_or(Value::Int(STATUS_INVALID_ARG)),
        None => Value::Int(STATUS_INVALID_ARG),
    }
}

fn samples(name: &str, args: &[Value]) -> Vec<f64> {
    match call(name, args) {
        Value::Array { elements } => elements
            .iter()
            .map(|value| match value {
                Value::Double(v) => *v,
                Value::Float(v) => *v as f64,
                Value::Int(v) => *v as f64,
                Value::Long(v) => *v as f64,
                _ => 0.0,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn dispatch_frames(frames: i64) -> Option<i64> {
    if frames < 0 {
        None
    } else {
        Some(frames.min(MAX_FRAMES as i64))
    }
}

/// Generate samples following a sine wave at `freq` Hz.
pub fn sine(freq: f64, rate: f64, frames: i64, amp: f64) -> Vec<f64> {
    let Some(frames) = dispatch_frames(frames) else {
        return Vec::new();
    };
    samples(
        "Synth_sine",
        &[
            Value::Double(freq),
            Value::Double(rate),
            Value::Long(frames),
            Value::Double(amp),
        ],
    )
}

/// Generate samples following a square wave at `freq` Hz with duty `duty`.
pub fn square(freq: f64, rate: f64, frames: i64, amp: f64, duty: f64) -> Vec<f64> {
    let Some(frames) = dispatch_frames(frames) else {
        return Vec::new();
    };
    samples(
        "Synth_square",
        &[
            Value::Double(freq),
            Value::Double(rate),
            Value::Long(frames),
            Value::Double(amp),
            Value::Double(duty),
        ],
    )
}

/// Generate samples following a rising sawtooth at `freq` Hz.
pub fn saw(freq: f64, rate: f64, frames: i64, amp: f64) -> Vec<f64> {
    let Some(frames) = dispatch_frames(frames) else {
        return Vec::new();
    };
    samples(
        "Synth_saw",
        &[
            Value::Double(freq),
            Value::Double(rate),
            Value::Long(frames),
            Value::Double(amp),
        ],
    )
}

/// Generate deterministic white noise using xorshift64* (std only).
pub fn noise(seed: i64, frames: i64, amp: f64) -> Vec<f64> {
    let Some(frames) = dispatch_frames(frames) else {
        return Vec::new();
    };
    samples(
        "Synth_noise",
        &[Value::Long(seed), Value::Long(frames), Value::Double(amp)],
    )
}

/// Mix two sample buffers index by index, clamping the result to PCM range.
/// A missing sample in the shorter buffer is treated as silence.
pub fn mix(a: &[f64], b: &[f64]) -> Vec<f64> {
    let array = |values: &[f64]| Value::Array {
        elements: values.iter().map(|v| Value::Double(*v)).collect(),
    };
    samples("Synth_mix", &[array(a), array(b)])
}

/// `std` has no audio output device API; report that honestly instead of
/// returning a device handle that could never produce sound.
pub fn output_available() -> bool {
    matches!(call("AudioDevice_isAvailable", &[]), Value::Bool(true))
}

pub fn output_status() -> i32 {
    match call("AudioDevice_status", &[]) {
        Value::Int(status) => status,
        _ => STATUS_UNAVAILABLE,
    }
}

pub fn output_reason() -> &'static str {
    AUDIO_OUTPUT_UNAVAILABLE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vm(name: &str, args: &[Value]) -> Value {
        (lookup_builtin_native(name).unwrap())(args).unwrap()
    }

    fn vm_samples(value: &Value) -> Vec<f64> {
        match value {
            Value::Array { elements } => elements
                .iter()
                .map(|v| match v {
                    Value::Double(d) => *d,
                    _ => panic!("expected a double sample"),
                })
                .collect(),
            _ => panic!("expected an array result"),
        }
    }

    fn vm_int(value: &Value) -> i64 {
        match value {
            Value::Int(v) => *v as i64,
            Value::Long(v) => *v,
            _ => panic!("expected an integer result"),
        }
    }

    #[test]
    fn wave_generation_matches_vm_samples() {
        let sine_args = [
            Value::Double(1000.0),
            Value::Double(4000.0),
            Value::Long(4),
            Value::Double(1.0),
        ];
        let s = sine(1000.0, 4000.0, 4, 1.0);
        assert_eq!(s, vm_samples(&vm("Synth_sine", &sine_args)));
        assert!(s[0].abs() < 1e-12);
        assert!((s[1] - 1.0).abs() < 1e-12);

        let square_args = [
            Value::Double(1000.0),
            Value::Double(4000.0),
            Value::Long(4),
            Value::Double(1.0),
            Value::Double(0.5),
        ];
        let sq = square(1000.0, 4000.0, 4, 1.0, 0.5);
        assert_eq!(sq, vm_samples(&vm("Synth_square", &square_args)));
        assert_eq!(sq, vec![1.0, 1.0, -1.0, -1.0]);

        let saw_args = [
            Value::Double(1000.0),
            Value::Double(4000.0),
            Value::Long(4),
            Value::Double(1.0),
        ];
        let sw = saw(1000.0, 4000.0, 4, 1.0);
        assert_eq!(sw, vm_samples(&vm("Synth_saw", &saw_args)));
        assert_eq!(sw.len(), 4);
        assert!((sw[0] + 1.0).abs() < 1e-12);
        assert!((sw[1] + 0.5).abs() < 1e-12);
        assert!(sw[2].abs() < 1e-12);
        assert!((sw[3] - 0.5).abs() < 1e-12);

        let n = noise(42, 8, 0.9);
        assert_eq!(
            n,
            vm_samples(&vm(
                "Synth_noise",
                &[Value::Long(42), Value::Long(8), Value::Double(0.9)]
            ))
        );
        assert_eq!(n, noise(42, 8, 0.9));
        assert_ne!(n, noise(43, 8, 0.9));
        assert!(n.iter().all(|v| v.abs() <= 0.9));
    }

    #[test]
    fn mix_matches_vm_and_clamps_to_pcm_range() {
        let a = vec![0.25, 0.75];
        let b = vec![0.25, 0.75, -0.5];
        let array = |values: &[f64]| Value::Array {
            elements: values.iter().map(|v| Value::Double(*v)).collect(),
        };
        let mixed = mix(&a, &b);
        assert_eq!(mixed, vm_samples(&vm("Synth_mix", &[array(&a), array(&b)])));
        assert_eq!(mixed, vec![0.5, 1.0, -0.5]);
    }

    #[test]
    fn device_state_matches_vm() {
        assert!(!output_available());
        assert!(matches!(vm("AudioDevice_isAvailable", &[]), Value::Bool(false)));
        // After unification the shared VM primitive is the single source of
        // truth and reports its backend-unavailable code (-1) directly.
        let unavailable = vm_int(&vm("AudioDevice_status", &[]));
        assert_eq!(output_status() as i64, unavailable);
        assert!(unavailable < 0);
        match vm("AudioDevice_play", &[]) {
            Value::String(message) => assert_eq!(message.as_str(), output_reason()),
            _ => panic!("expected a string result"),
        }
        assert_eq!(output_reason(), AUDIO_OUTPUT_UNAVAILABLE);
    }

    #[test]
    fn invalid_frame_count_matches_vm_error() {
        assert!(sine(1000.0, 4000.0, 0, 1.0).is_empty());
        assert!(noise(42, 0, 0.9).is_empty());
        let err = (lookup_builtin_native("Synth_sine").unwrap())(
            &[
                Value::Double(1000.0),
                Value::Double(4000.0),
                Value::Long(0),
                Value::Double(1.0),
            ],
        )
        .unwrap_err();
        assert_eq!(err, "Synth: invalid wave parameters");
        let err = (lookup_builtin_native("Synth_noise").unwrap())(
            &[Value::Long(42), Value::Long(0), Value::Double(0.9)],
        )
        .unwrap_err();
        assert_eq!(err, "Synth: invalid frame count");
    }
}
