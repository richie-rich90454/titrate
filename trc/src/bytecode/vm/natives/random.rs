// Titrate Alpha 0.2 – bytecode virtual machine: random natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;

pub(crate) fn native_random_seed(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let epoch_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("Random_seed: {}", e))?
        .as_millis() as i64;
    Ok(Value::Long(epoch_ms))
}

pub(crate) fn native_random_next_long(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Random_nextLong: expected 2 arguments (state0, state1)".to_string());
    }
    let s0 = args[0].to_i64().unwrap_or(0) as u64;
    let mut s1 = args[1].to_i64().unwrap_or(0) as u64;

    // Xorshift128+
    s1 ^= s1 << 23;
    s1 ^= s1 >> 17;
    s1 ^= s0;
    s1 ^= s0 >> 26;
    let new_s0 = s1;
    let result = (new_s0.wrapping_add(s1)) as i64;
    let new_s1 = s0;

    Ok(Value::Array {
        elements: vec![
            Value::Long(new_s0 as i64),
            Value::Long(new_s1 as i64),
            Value::Long(result),
        ],
    })
}
