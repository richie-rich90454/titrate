// Titrate Alpha 0.2 – bytecode virtual machine: time natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::rc::Rc;
use chrono::{Datelike, Timelike};

pub(crate) fn native_time_now(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let epoch_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("Time_now: {}", e))?
        .as_millis() as i64;
    Ok(Value::Long(epoch_ms))
}

pub(crate) fn native_time_sleep(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Time_sleep: expected 1 argument (milliseconds)".to_string());
    }
    let ms = args[0].to_i64().unwrap_or(0);
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    Ok(Value::Void)
}

pub(crate) fn native_time_format(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Time_format: expected 2 arguments (epoch_ms, format)".to_string());
    }
    let epoch_ms = args[0].to_i64().unwrap_or(0);
    let fmt = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Time_format: expected String format".to_string()),
    };
    // Simple format: support yyyy, MM, dd, HH, mm, ss placeholders
    let secs = epoch_ms / 1000;
    let datetime = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    let formatted = datetime.format(&fmt).to_string();
    Ok(Value::String(Rc::new(formatted)))
}

pub(crate) fn native_time_get_year(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Time_getYear: expected 1 argument (epoch_ms)".to_string());
    }
    let epoch_ms = args[0].to_i64().unwrap_or(0);
    let secs = epoch_ms / 1000;
    let datetime = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    Ok(Value::Int(datetime.year() as i32))
}

pub(crate) fn native_time_get_month(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Time_getMonth: expected 1 argument (epoch_ms)".to_string());
    }
    let epoch_ms = args[0].to_i64().unwrap_or(0);
    let secs = epoch_ms / 1000;
    let datetime = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    Ok(Value::Int(datetime.month() as i32))
}

pub(crate) fn native_time_get_day(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Time_getDay: expected 1 argument (epoch_ms)".to_string());
    }
    let epoch_ms = args[0].to_i64().unwrap_or(0);
    let secs = epoch_ms / 1000;
    let datetime = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    Ok(Value::Int(datetime.day() as i32))
}

pub(crate) fn native_time_get_hour(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Time_getHour: expected 1 argument (epoch_ms)".to_string());
    }
    let epoch_ms = args[0].to_i64().unwrap_or(0);
    let secs = epoch_ms / 1000;
    let datetime = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    Ok(Value::Int(datetime.hour() as i32))
}

pub(crate) fn native_time_get_minute(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Time_getMinute: expected 1 argument (epoch_ms)".to_string());
    }
    let epoch_ms = args[0].to_i64().unwrap_or(0);
    let secs = epoch_ms / 1000;
    let datetime = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    Ok(Value::Int(datetime.minute() as i32))
}

pub(crate) fn native_time_get_second(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Time_getSecond: expected 1 argument (epoch_ms)".to_string());
    }
    let epoch_ms = args[0].to_i64().unwrap_or(0);
    let secs = epoch_ms / 1000;
    let datetime = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    Ok(Value::Int(datetime.second() as i32))
}

pub(crate) fn native_time_day_of_week(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Time_dayOfWeek: expected 1 argument (epoch_ms)".to_string());
    }
    let epoch_ms = args[0].to_i64().unwrap_or(0);
    let secs = epoch_ms / 1000;
    let datetime = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    // chrono: 0=Mon, 6=Sun via .weekday().num_days_from_monday()
    Ok(Value::Int(datetime.weekday().num_days_from_monday() as i32))
}

pub(crate) fn native_time_day_of_year(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Time_dayOfYear: expected 1 argument (epoch_ms)".to_string());
    }
    let epoch_ms = args[0].to_i64().unwrap_or(0);
    let secs = epoch_ms / 1000;
    let datetime = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    Ok(Value::Int(datetime.ordinal() as i32))
}

pub(crate) fn native_time_monotonic(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    use std::time::Instant;
    // Return nanoseconds from an arbitrary epoch (process start)
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let start = START.get_or_init(Instant::now);
    let ns = start.elapsed().as_nanos() as i64;
    Ok(Value::Long(ns))
}

pub(crate) fn native_time_perf_counter(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    // High-resolution performance counter using monotonic clock
    use std::time::Instant;
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let start = START.get_or_init(Instant::now);
    let ns = start.elapsed().as_nanos() as i64;
    Ok(Value::Long(ns))
}

pub(crate) fn native_time_epoch_seconds(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    // Return Unix timestamp as double with sub-second precision
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("Time_epochSeconds: {}", e))?;
    let secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1_000_000_000.0;
    Ok(Value::Double(secs))
}

pub(crate) fn native_time_nanos(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    // Return current time as nanoseconds since Unix epoch
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("Time_nanos: {}", e))?;
    let ns = duration.as_secs() as i64 * 1_000_000_000 + duration.subsec_nanos() as i64;
    Ok(Value::Long(ns))
}
