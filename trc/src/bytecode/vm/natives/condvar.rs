// Titrate Alpha 0.2 – bytecode virtual machine: condvar natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Condvar, LazyLock, Mutex as StdMutex};
use std::time::Duration;

struct CondvarPair {
    condvar: Condvar,
    flag: StdMutex<bool>,
}

static CV_REGISTRY: LazyLock<StdMutex<HashMap<i64, Arc<CondvarPair>>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static CV_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

pub(crate) fn native_cv_new(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let handle = CV_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let pair = Arc::new(CondvarPair {
        condvar: Condvar::new(),
        flag: StdMutex::new(false),
    });
    let mut registry = CV_REGISTRY.lock().unwrap();
    registry.insert(handle, pair);
    Ok(Value::Long(handle))
}

pub(crate) fn native_cv_wait(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("CondVar_wait: expected an Int/Long handle".to_string()),
    };
    let pair = {
        let registry = CV_REGISTRY.lock().unwrap();
        registry
            .get(&handle)
            .cloned()
            .ok_or_else(|| "CondVar_wait: invalid handle".to_string())?
    };
    let mut flag = pair.flag.lock().unwrap();
    *flag = false;
    while !*flag {
        flag = pair.condvar.wait(flag).unwrap();
    }
    Ok(Value::Null)
}

pub(crate) fn native_cv_wait_for(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("CondVar_waitFor: expected handle and timeout".to_string()),
    };
    let ms = match args.get(1) {
        Some(Value::Long(ms)) => *ms,
        Some(Value::Int(ms)) => *ms as i64,
        _ => return Err("CondVar_waitFor: expected timeout in milliseconds".to_string()),
    };
    let pair = {
        let registry = CV_REGISTRY.lock().unwrap();
        registry
            .get(&handle)
            .cloned()
            .ok_or_else(|| "CondVar_waitFor: invalid handle".to_string())?
    };
    let mut flag = pair.flag.lock().unwrap();
    *flag = false;
    let result = pair
        .condvar
        .wait_timeout(flag, Duration::from_millis(ms as u64))
        .unwrap();
    Ok(Value::Bool(*result.0))
}

pub(crate) fn native_cv_notify_one(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("CondVar_notifyOne: expected an Int/Long handle".to_string()),
    };
    let pair = {
        let registry = CV_REGISTRY.lock().unwrap();
        registry
            .get(&handle)
            .cloned()
            .ok_or_else(|| "CondVar_notifyOne: invalid handle".to_string())?
    };
    {
        let mut flag = pair.flag.lock().unwrap();
        *flag = true;
    }
    pair.condvar.notify_one();
    Ok(Value::Null)
}

pub(crate) fn native_cv_notify_all(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("CondVar_notifyAll: expected an Int/Long handle".to_string()),
    };
    let pair = {
        let registry = CV_REGISTRY.lock().unwrap();
        registry
            .get(&handle)
            .cloned()
            .ok_or_else(|| "CondVar_notifyAll: invalid handle".to_string())?
    };
    {
        let mut flag = pair.flag.lock().unwrap();
        *flag = true;
    }
    pair.condvar.notify_all();
    Ok(Value::Null)
}
