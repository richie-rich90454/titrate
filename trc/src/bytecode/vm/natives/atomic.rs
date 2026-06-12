// Titrate Alpha 0.2 – bytecode virtual machine: atomic natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{LazyLock, Mutex as StdMutex};

// ---------------------------------------------------------------------------
// AtomicInt registry
// ---------------------------------------------------------------------------

static ATOMIC_INT_REGISTRY: LazyLock<StdMutex<HashMap<i64, AtomicI64>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static ATOMIC_INT_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

pub(crate) fn native_atomic_int_new(args: &[Value]) -> Result<Value, String> {
    let val = match args.first() {
        Some(Value::Long(v)) => *v,
        Some(Value::Int(v)) => *v as i64,
        _ => return Err("AtomicInt_new: expected an Int/Long initial value".to_string()),
    };
    let handle = ATOMIC_INT_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let mut registry = ATOMIC_INT_REGISTRY.lock().unwrap();
    registry.insert(handle, AtomicI64::new(val));
    Ok(Value::Long(handle))
}

pub(crate) fn native_atomic_int_get(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicInt_get: expected an Int/Long handle".to_string()),
    };
    let registry = ATOMIC_INT_REGISTRY.lock().unwrap();
    let atomic = registry
        .get(&handle)
        .ok_or_else(|| "AtomicInt_get: invalid handle".to_string())?;
    Ok(Value::Long(atomic.load(Ordering::SeqCst)))
}

pub(crate) fn native_atomic_int_set(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicInt_set: expected handle and value".to_string()),
    };
    let val = match args.get(1) {
        Some(Value::Long(v)) => *v,
        Some(Value::Int(v)) => *v as i64,
        _ => return Err("AtomicInt_set: expected an Int/Long value".to_string()),
    };
    let registry = ATOMIC_INT_REGISTRY.lock().unwrap();
    let atomic = registry
        .get(&handle)
        .ok_or_else(|| "AtomicInt_set: invalid handle".to_string())?;
    atomic.store(val, Ordering::SeqCst);
    Ok(Value::Null)
}

pub(crate) fn native_atomic_int_fetch_add(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicInt_fetchAdd: expected handle and delta".to_string()),
    };
    let delta = match args.get(1) {
        Some(Value::Long(v)) => *v,
        Some(Value::Int(v)) => *v as i64,
        _ => return Err("AtomicInt_fetchAdd: expected an Int/Long delta".to_string()),
    };
    let registry = ATOMIC_INT_REGISTRY.lock().unwrap();
    let atomic = registry
        .get(&handle)
        .ok_or_else(|| "AtomicInt_fetchAdd: invalid handle".to_string())?;
    let old = atomic.fetch_add(delta, Ordering::SeqCst);
    Ok(Value::Long(old))
}

pub(crate) fn native_atomic_int_fetch_sub(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicInt_fetchSub: expected handle and delta".to_string()),
    };
    let delta = match args.get(1) {
        Some(Value::Long(v)) => *v,
        Some(Value::Int(v)) => *v as i64,
        _ => return Err("AtomicInt_fetchSub: expected an Int/Long delta".to_string()),
    };
    let registry = ATOMIC_INT_REGISTRY.lock().unwrap();
    let atomic = registry
        .get(&handle)
        .ok_or_else(|| "AtomicInt_fetchSub: invalid handle".to_string())?;
    let old = atomic.fetch_sub(delta, Ordering::SeqCst);
    Ok(Value::Long(old))
}

pub(crate) fn native_atomic_int_compare_and_swap(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicInt_compareAndSwap: expected handle, expected, new".to_string()),
    };
    let expected = match args.get(1) {
        Some(Value::Long(v)) => *v,
        Some(Value::Int(v)) => *v as i64,
        _ => return Err("AtomicInt_compareAndSwap: expected an Int/Long expected value".to_string()),
    };
    let new_val = match args.get(2) {
        Some(Value::Long(v)) => *v,
        Some(Value::Int(v)) => *v as i64,
        _ => return Err("AtomicInt_compareAndSwap: expected an Int/Long new value".to_string()),
    };
    let registry = ATOMIC_INT_REGISTRY.lock().unwrap();
    let atomic = registry
        .get(&handle)
        .ok_or_else(|| "AtomicInt_compareAndSwap: invalid handle".to_string())?;
    let result = atomic.compare_exchange(expected, new_val, Ordering::SeqCst, Ordering::SeqCst);
    Ok(Value::Bool(result.is_ok()))
}

// ---------------------------------------------------------------------------
// AtomicBool registry
// ---------------------------------------------------------------------------

static ATOMIC_BOOL_REGISTRY: LazyLock<StdMutex<HashMap<i64, AtomicBool>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static ATOMIC_BOOL_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

pub(crate) fn native_atomic_bool_new(args: &[Value]) -> Result<Value, String> {
    let val = match args.first() {
        Some(Value::Bool(v)) => *v,
        Some(Value::Long(v)) => *v != 0,
        Some(Value::Int(v)) => *v != 0,
        _ => return Err("AtomicBool_new: expected a Bool initial value".to_string()),
    };
    let handle = ATOMIC_BOOL_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let mut registry = ATOMIC_BOOL_REGISTRY.lock().unwrap();
    registry.insert(handle, AtomicBool::new(val));
    Ok(Value::Long(handle))
}

pub(crate) fn native_atomic_bool_get(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicBool_get: expected an Int/Long handle".to_string()),
    };
    let registry = ATOMIC_BOOL_REGISTRY.lock().unwrap();
    let atomic = registry
        .get(&handle)
        .ok_or_else(|| "AtomicBool_get: invalid handle".to_string())?;
    Ok(Value::Bool(atomic.load(Ordering::SeqCst)))
}

pub(crate) fn native_atomic_bool_set(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicBool_set: expected handle and value".to_string()),
    };
    let val = match args.get(1) {
        Some(Value::Bool(v)) => *v,
        Some(Value::Long(v)) => *v != 0,
        Some(Value::Int(v)) => *v != 0,
        _ => return Err("AtomicBool_set: expected a Bool value".to_string()),
    };
    let registry = ATOMIC_BOOL_REGISTRY.lock().unwrap();
    let atomic = registry
        .get(&handle)
        .ok_or_else(|| "AtomicBool_set: invalid handle".to_string())?;
    atomic.store(val, Ordering::SeqCst);
    Ok(Value::Null)
}

pub(crate) fn native_atomic_bool_compare_and_swap(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicBool_compareAndSwap: expected handle, expected, new".to_string()),
    };
    let expected = match args.get(1) {
        Some(Value::Bool(v)) => *v,
        Some(Value::Long(v)) => *v != 0,
        Some(Value::Int(v)) => *v != 0,
        _ => return Err("AtomicBool_compareAndSwap: expected a Bool expected value".to_string()),
    };
    let new_val = match args.get(2) {
        Some(Value::Bool(v)) => *v,
        Some(Value::Long(v)) => *v != 0,
        Some(Value::Int(v)) => *v != 0,
        _ => return Err("AtomicBool_compareAndSwap: expected a Bool new value".to_string()),
    };
    let registry = ATOMIC_BOOL_REGISTRY.lock().unwrap();
    let atomic = registry
        .get(&handle)
        .ok_or_else(|| "AtomicBool_compareAndSwap: invalid handle".to_string())?;
    let result = atomic.compare_exchange(expected, new_val, Ordering::SeqCst, Ordering::SeqCst);
    Ok(Value::Bool(result.is_ok()))
}
