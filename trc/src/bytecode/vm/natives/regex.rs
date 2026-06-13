// Titrate Alpha 0.2 – bytecode virtual machine: regex natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::rc::Rc;

pub(crate) fn native_regex_match(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Regex_match: expected 2 arguments (pattern, input)".to_string());
    }
    let pattern = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_match: expected String pattern".to_string()),
    };
    let input = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_match: expected String input".to_string()),
    };
    let re = regex::Regex::new(&pattern)
        .map_err(|e| format!("Regex_match: invalid pattern '{}': {}", pattern, e))?;
    Ok(Value::Bool(re.is_match(&input)))
}

pub(crate) fn native_regex_find(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Regex_find: expected 2 arguments (pattern, input)".to_string());
    }
    let pattern = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_find: expected String pattern".to_string()),
    };
    let input = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_find: expected String input".to_string()),
    };
    let re = regex::Regex::new(&pattern)
        .map_err(|e| format!("Regex_find: invalid pattern '{}': {}", pattern, e))?;
    match re.find(&input) {
        Some(m) => {
            // Return "start,end,matched_text"
            let result = format!("{},{},{}", m.start(), m.end(), m.as_str());
            Ok(Value::String(Rc::new(result)))
        }
        None => Ok(Value::String(Rc::new(String::new()))),
    }
}

pub(crate) fn native_regex_replace(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("Regex_replace: expected 3 arguments (pattern, input, replacement)".to_string());
    }
    let pattern = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_replace: expected String pattern".to_string()),
    };
    let input = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_replace: expected String input".to_string()),
    };
    let replacement = match &args[2] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_replace: expected String replacement".to_string()),
    };
    let re = regex::Regex::new(&pattern)
        .map_err(|e| format!("Regex_replace: invalid pattern '{}': {}", pattern, e))?;
    let result = re.replace_all(&input, &replacement).to_string();
    Ok(Value::String(Rc::new(result)))
}

pub(crate) fn native_regex_group_count(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Regex_groupCount: expected 1 argument (pattern)".to_string());
    }
    let pattern = match args.first() {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("Regex_groupCount: expected String pattern".to_string()),
    };
    let re = regex::Regex::new(&pattern)
        .map_err(|e| format!("Regex_groupCount: invalid pattern '{}': {}", pattern, e))?;
    Ok(Value::Int(re.captures_len() as i32 - 1))
}

pub(crate) fn native_regex_find_groups(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Regex_findGroups: expected 2 arguments (pattern, input)".to_string());
    }
    let pattern = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_findGroups: expected String pattern".to_string()),
    };
    let input = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_findGroups: expected String input".to_string()),
    };
    let re = regex::Regex::new(&pattern)
        .map_err(|e| format!("Regex_findGroups: invalid pattern '{}': {}", pattern, e))?;
    match re.captures(&input) {
        Some(caps) => {
            // Format: "group0_start,group0_end,group0_text;group1_start,group1_end,group1_text;..."
            let parts: Vec<String> = caps.iter().enumerate().map(|(i, m)| {
                match m {
                    Some(m) => format!("{},{},{}", m.start(), m.end(), m.as_str()),
                    None => format!("-1,-1,"),
                }
            }).collect();
            let result = parts.join(";");
            Ok(Value::String(Rc::new(result)))
        }
        None => Ok(Value::String(Rc::new(String::new()))),
    }
}

/// Build a regex pattern with inline flags, e.g. "(?im)" + pattern
fn build_pattern_with_flags(pattern: &str, flags: &str) -> String {
    if flags.is_empty() {
        pattern.to_string()
    } else {
        format!("(?{}){}", flags, pattern)
    }
}

pub(crate) fn native_regex_find_with_flags(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("Regex_findWithFlags: expected 3 arguments (pattern, input, flags)".to_string());
    }
    let pattern = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_findWithFlags: expected String pattern".to_string()),
    };
    let input = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_findWithFlags: expected String input".to_string()),
    };
    let flags = match &args[2] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_findWithFlags: expected String flags".to_string()),
    };
    let full_pattern = build_pattern_with_flags(&pattern, &flags);
    let re = regex::Regex::new(&full_pattern)
        .map_err(|e| format!("Regex_findWithFlags: invalid pattern '{}': {}", full_pattern, e))?;
    match re.find(&input) {
        Some(m) => {
            let result = format!("{},{},{}", m.start(), m.end(), m.as_str());
            Ok(Value::String(Rc::new(result)))
        }
        None => Ok(Value::String(Rc::new(String::new()))),
    }
}

pub(crate) fn native_regex_match_with_flags(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("Regex_matchWithFlags: expected 3 arguments (pattern, input, flags)".to_string());
    }
    let pattern = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_matchWithFlags: expected String pattern".to_string()),
    };
    let input = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_matchWithFlags: expected String input".to_string()),
    };
    let flags = match &args[2] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Regex_matchWithFlags: expected String flags".to_string()),
    };
    let full_pattern = build_pattern_with_flags(&pattern, &flags);
    let re = regex::Regex::new(&full_pattern)
        .map_err(|e| format!("Regex_matchWithFlags: invalid pattern '{}': {}", full_pattern, e))?;
    Ok(Value::Bool(re.is_match(&input)))
}
