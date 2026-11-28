//! Native argument serialization harness: random TitrateValue byte buffers
//! are decoded by the ABI mirror, then the real C-ABI dispatch bridge is
//! checked against a deliberately malformed argument buffer.

use crate::abi;
use crate::common::Lcg;

const NATIVE_NAME: &[u8] = b"Math_abs";
const RESULT_CAP: usize = 512;

// The dispatch bridge takes a serialized Value stream ([i32 tag][payload]),
// not a 24-byte TitrateValue union; the abi mirror checks that layout instead.
const TAG_INT: i32 = 1;

fn call_bridge(args: &[u8]) -> (i32, String) {
    let mut result = [0u8; RESULT_CAP];
    let mut capacity = RESULT_CAP as i64;
    let code = unsafe {
        titrate_native::titrate_native_call_v2(
            NATIVE_NAME.as_ptr(),
            NATIVE_NAME.len() as i64,
            args.as_ptr(),
            1,
            args.len() as i64,
            result.as_mut_ptr(),
            &mut capacity,
        )
    };
    let written = (capacity.max(0) as usize).min(RESULT_CAP);
    (code, String::from_utf8_lossy(&result[..written]).into_owned())
}

fn check_real_bridge() -> Result<(), String> {
    let mut valid = Vec::from(TAG_INT.to_le_bytes());
    valid.extend_from_slice(&42i32.to_le_bytes());
    let (code, _) = call_bridge(&valid);
    if code != 0 {
        return Err(format!("well-formed native argument was rejected (code {})", code));
    }

    // An int tag with a truncated payload must fail with a descriptive reason.
    let malformed = [1u8, 0, 0, 0, 1, 0];
    let (code, message) = call_bridge(&malformed);
    if code != 1 {
        return Err("malformed native argument was accepted by titrate_native_call_v2".to_string());
    }
    if !message.contains("deserialize:") {
        return Err(format!(
            "native bridge rejected the buffer without a descriptive reason: {:?}",
            message
        ));
    }
    Ok(())
}

/// Fuzz the decoder and the production bridge. A structured `Err` is the
/// expected outcome for malformed buffers; panics are converted to failures by
/// the harness runner in lib.rs.
pub fn run(iterations: usize, seed: u64) -> Result<(), String> {
    let mut rng = Lcg::new(seed);
    let mut accepted = 0usize;
    for iteration in 0..iterations {
        let buffer = abi::random_buffer(&mut rng);
        match abi::decode(&buffer) {
            Ok(_) => accepted += 1,
            Err(error) => {
                if !abi::is_descriptive_error(&error) {
                    return Err(format!(
                        "iteration {}: decoder error is not descriptive: {}",
                        iteration, error
                    ));
                }
            }
        }
    }
    if accepted == 0 {
        return Err("generator produced no well-formed TitrateValue buffers".to_string());
    }

    for (label, bytes) in abi::malformed_cases() {
        match abi::decode(&bytes) {
            Ok(value) => {
                return Err(format!("malformed case '{}' decoded as {:?}", label, value))
            }
            Err(error) => {
                if !abi::is_descriptive_error(&error) {
                    return Err(format!("malformed case '{}': undescriptive error {}", label, error));
                }
            }
        }
    }

    check_real_bridge()
}
