//! Pure-Rust PCM sample generation and mixing for the audio host primitives.
//!
//! `std` exposes no audio output device, so this module never guesses at a
//! playback backend. It generates deterministic sample buffers and reports
//! output availability explicitly; the `.tr` layer does the sequencing.

/// Largest sample buffer a single primitive call may produce (16M frames).
pub const MAX_FRAMES: usize = 1 << 24;

/// Error text surfaced by the audio output wrapper when no device exists.
pub const AUDIO_OUTPUT_UNAVAILABLE: &str = "audio output unavailable";

/// Status codes returned by the raw C-ABI primitives.
pub const STATUS_OK: i32 = 0;
pub const STATUS_INVALID_ARG: i32 = -1;
pub const STATUS_UNAVAILABLE: i32 = -2;
pub const STATUS_ALLOC_FAILED: i32 = -3;

fn frame_count(frames: i64) -> usize {
    if frames <= 0 {
        return 0;
    }
    (frames as usize).min(MAX_FRAMES)
}

fn amplitude(amplitude: f64) -> f64 {
    if !amplitude.is_finite() {
        return 0.0;
    }
    amplitude.abs().min(1.0)
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

/// Generate samples following a sine wave at `freq` Hz.
pub fn sine(freq: f64, rate: f64, frames: i64, amp: f64) -> Vec<f64> {
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

/// Generate samples following a square wave at `freq` Hz with duty `duty`.
pub fn square(freq: f64, rate: f64, frames: i64, amp: f64, duty: f64) -> Vec<f64> {
    let n = frame_count(frames);
    let amp = amplitude(amp);
    let mut d = if duty.is_finite() { duty } else { 0.5 };
    if d <= 0.0 || d >= 1.0 {
        d = 0.5;
    }
    let step = frequency(freq) / sample_rate(rate);
    let mut phase = 0.0;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(if phase < d { amp } else { -amp });
        phase = (phase + step) % 1.0;
    }
    out
}

/// Generate samples following a rising sawtooth at `freq` Hz.
pub fn saw(freq: f64, rate: f64, frames: i64, amp: f64) -> Vec<f64> {
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

/// Generate deterministic white noise using xorshift64* (std only).
pub fn noise(seed: i64, frames: i64, amp: f64) -> Vec<f64> {
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

/// Mix two sample buffers index by index, clamping the result to PCM range.
/// A missing sample in the shorter buffer is treated as silence.
pub fn mix(a: &[f64], b: &[f64]) -> Vec<f64> {
    let n = a.len().max(b.len());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(0.0);
        let y = b.get(i).copied().unwrap_or(0.0);
        out.push((x + y).clamp(-1.0, 1.0));
    }
    out
}

/// `std` has no audio output device API; report that honestly instead of
/// returning a device handle that could never produce sound.
pub fn output_available() -> bool {
    false
}

pub fn output_status() -> i32 {
    STATUS_UNAVAILABLE
}

pub fn output_reason() -> &'static str {
    AUDIO_OUTPUT_UNAVAILABLE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_starts_at_zero_and_reaches_peak_at_quarter_period() {
        let s = sine(1000.0, 4000.0, 4, 1.0);
        assert_eq!(s.len(), 4);
        assert!(s[0].abs() < 1e-12);
        assert!((s[1] - 1.0).abs() < 1e-12);
        assert!(s[2].abs() < 1e-12);
        assert!((s[3] + 1.0).abs() < 1e-12);
    }

    #[test]
    fn square_saw_and_noise_stay_in_range() {
        for v in square(440.0, 44100.0, 64, 0.5, 0.25) {
            assert!(v == 0.5 || v == -0.5);
        }
        for v in saw(440.0, 44100.0, 64, 0.8) {
            assert!(v >= -0.8 && v < 0.8);
        }
        let a = noise(42, 64, 0.9);
        let b = noise(42, 64, 0.9);
        assert_eq!(a, b);
        assert!(a.iter().all(|v| v.abs() <= 0.9));
        assert_ne!(noise(43, 64, 0.9), a);
    }

    #[test]
    fn mix_adds_and_clamps_to_pcm_range() {
        let a = vec![0.25, 0.75];
        let b = vec![0.25, 0.75, -0.5];
        let m = mix(&a, &b);
        assert_eq!(m.len(), 3);
        assert!((m[0] - 0.5).abs() < 1e-12);
        assert!(m[1] <= 1.0);
        assert!((m[2] + 0.5).abs() < 1e-12);
    }

    #[test]
    fn output_is_explicitly_unavailable() {
        assert!(!output_available());
        assert_eq!(output_status(), STATUS_UNAVAILABLE);
        assert!(!output_reason().is_empty());
    }
}
