// Titrate Alpha 0.2 – bytecode virtual machine: math natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;

pub(crate) fn native_math_sin(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_sin: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.sin()))
}

pub(crate) fn native_math_cos(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_cos: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.cos()))
}

pub(crate) fn native_math_tan(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_tan: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.tan()))
}

pub(crate) fn native_math_asin(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_asin: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.asin()))
}

pub(crate) fn native_math_acos(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_acos: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.acos()))
}

pub(crate) fn native_math_atan(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_atan: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.atan()))
}

pub(crate) fn native_math_atan2(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 { return Err("Math_atan2: expected 2 arguments (y, x)".to_string()); }
    let y = args[0].to_f64().unwrap_or(0.0);
    let x = args[1].to_f64().unwrap_or(0.0);
    Ok(Value::Double(y.atan2(x)))
}

pub(crate) fn native_math_ln(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_ln: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.ln()))
}

pub(crate) fn native_math_log10(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_log10: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.log10()))
}

pub(crate) fn native_math_log2(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_log2: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.log2()))
}

pub(crate) fn native_math_exp(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_exp: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.exp()))
}

pub(crate) fn native_math_pow(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 { return Err("Math_pow: expected 2 arguments (base, exp)".to_string()); }
    let base = args[0].to_f64().unwrap_or(0.0);
    let exp = args[1].to_f64().unwrap_or(0.0);
    Ok(Value::Double(base.powf(exp)))
}

pub(crate) fn native_math_sqrt(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_sqrt: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.sqrt()))
}

pub(crate) fn native_math_cbrt(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_cbrt: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.cbrt()))
}

pub(crate) fn native_math_abs(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_abs: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.abs()))
}

pub(crate) fn native_math_abs_int(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_absInt: expected 1 argument".to_string()); }
    let x = args[0].to_i64().unwrap_or(0);
    Ok(Value::Long(x.abs()))
}

pub(crate) fn native_math_floor(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_floor: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.floor()))
}

pub(crate) fn native_math_ceil(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_ceil: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.ceil()))
}

pub(crate) fn native_math_round(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_round: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Long(x.round() as i64))
}

pub(crate) fn native_math_inf(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Double(f64::INFINITY))
}

pub(crate) fn native_math_nan(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Double(f64::NAN))
}

pub(crate) fn native_math_max_double(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Double(f64::MAX))
}

pub(crate) fn native_math_min_double(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Double(f64::MIN))
}

pub(crate) fn native_math_max_int(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Long(i64::MAX))
}

pub(crate) fn native_math_min_int(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Long(i64::MIN))
}

pub(crate) fn native_math_next_up(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_nextUp: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.next_up()))
}

pub(crate) fn native_math_next_down(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_nextDown: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    Ok(Value::Double(x.next_down()))
}

pub(crate) fn native_math_ulp(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_ulp: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0).abs();
    let ulp = if x == 0.0 {
        f64::MIN_POSITIVE
    } else {
        let exp = x.log2().floor() as i32;
        f64::powf(2.0, exp as f64) * f64::EPSILON
    };
    Ok(Value::Double(ulp))
}

pub(crate) fn native_math_get_exponent(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() { return Err("Math_getExponent: expected 1 argument".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    if x == 0.0 || !x.is_finite() {
        return Ok(Value::Long(i64::MIN));
    }
    let exp = x.abs().log2().floor() as i64;
    Ok(Value::Long(exp))
}

pub(crate) fn native_math_scalb(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 { return Err("Math_scalb: expected 2 arguments (x, scaleFactor)".to_string()); }
    let x = args[0].to_f64().unwrap_or(0.0);
    let scale = args[1].to_i64().unwrap_or(0) as i32;
    Ok(Value::Double(x * 2.0_f64.powi(scale)))
}

pub(crate) fn native_math_random(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Math_random: {}", e))?
        .as_nanos() as u64;
    // Simple xorshift64 for a quick random double in [0, 1)
    let mut s = seed;
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    let result = (s >> 11) as f64 / (1u64 << 53) as f64;
    Ok(Value::Double(result))
}

pub(crate) fn native_math_neg_inf(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Double(f64::NEG_INFINITY))
}
