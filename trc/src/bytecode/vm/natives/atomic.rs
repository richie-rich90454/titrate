// Titrate Alpha 0.2 – bytecode virtual machine: atomic natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{LazyLock, Mutex as StdMutex};
use std::rc::Rc;

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

pub(crate) fn native_atomic_int_fetch_or(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicInt_fetchOr: expected handle and value".to_string()),
    };
    let val = match args.get(1) {
        Some(Value::Long(v)) => *v,
        Some(Value::Int(v)) => *v as i64,
        _ => return Err("AtomicInt_fetchOr: expected an Int/Long value".to_string()),
    };
    let registry = ATOMIC_INT_REGISTRY.lock().unwrap();
    let atomic = registry
        .get(&handle)
        .ok_or_else(|| "AtomicInt_fetchOr: invalid handle".to_string())?;
    let old = atomic.fetch_or(val, Ordering::SeqCst);
    Ok(Value::Long(old))
}

pub(crate) fn native_atomic_int_fetch_and(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicInt_fetchAnd: expected handle and value".to_string()),
    };
    let val = match args.get(1) {
        Some(Value::Long(v)) => *v,
        Some(Value::Int(v)) => *v as i64,
        _ => return Err("AtomicInt_fetchAnd: expected an Int/Long value".to_string()),
    };
    let registry = ATOMIC_INT_REGISTRY.lock().unwrap();
    let atomic = registry
        .get(&handle)
        .ok_or_else(|| "AtomicInt_fetchAnd: invalid handle".to_string())?;
    let old = atomic.fetch_and(val, Ordering::SeqCst);
    Ok(Value::Long(old))
}

pub(crate) fn native_atomic_int_fetch_xor(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicInt_fetchXor: expected handle and value".to_string()),
    };
    let val = match args.get(1) {
        Some(Value::Long(v)) => *v,
        Some(Value::Int(v)) => *v as i64,
        _ => return Err("AtomicInt_fetchXor: expected an Int/Long value".to_string()),
    };
    let registry = ATOMIC_INT_REGISTRY.lock().unwrap();
    let atomic = registry
        .get(&handle)
        .ok_or_else(|| "AtomicInt_fetchXor: invalid handle".to_string())?;
    let old = atomic.fetch_xor(val, Ordering::SeqCst);
    Ok(Value::Long(old))
}

pub(crate) fn native_atomic_int_exchange(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicInt_exchange: expected handle and value".to_string()),
    };
    let val = match args.get(1) {
        Some(Value::Long(v)) => *v,
        Some(Value::Int(v)) => *v as i64,
        _ => return Err("AtomicInt_exchange: expected an Int/Long value".to_string()),
    };
    let registry = ATOMIC_INT_REGISTRY.lock().unwrap();
    let atomic = registry
        .get(&handle)
        .ok_or_else(|| "AtomicInt_exchange: invalid handle".to_string())?;
    let old = atomic.swap(val, Ordering::SeqCst);
    Ok(Value::Long(old))
}

// ---------------------------------------------------------------------------
// AtomicLong registry
// ---------------------------------------------------------------------------

static ATOMIC_LONG_REGISTRY: LazyLock<StdMutex<HashMap<i64, AtomicI64>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static ATOMIC_LONG_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

pub(crate) fn native_atomic_long_new(args: &[Value]) -> Result<Value, String> {
    let val = match args.first() {
        Some(Value::Long(v)) => *v,
        Some(Value::Int(v)) => *v as i64,
        _ => return Err("AtomicLong_new: expected an Int/Long initial value".to_string()),
    };
    let handle = ATOMIC_LONG_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let mut registry = ATOMIC_LONG_REGISTRY.lock().unwrap();
    registry.insert(handle, AtomicI64::new(val));
    Ok(Value::Long(handle))
}

pub(crate) fn native_atomic_long_get(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicLong_get: expected an Int/Long handle".to_string()),
    };
    let registry = ATOMIC_LONG_REGISTRY.lock().unwrap();
    let atomic = registry
        .get(&handle)
        .ok_or_else(|| "AtomicLong_get: invalid handle".to_string())?;
    Ok(Value::Long(atomic.load(Ordering::SeqCst)))
}

pub(crate) fn native_atomic_long_set(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicLong_set: expected handle and value".to_string()),
    };
    let val = match args.get(1) {
        Some(Value::Long(v)) => *v,
        Some(Value::Int(v)) => *v as i64,
        _ => return Err("AtomicLong_set: expected an Int/Long value".to_string()),
    };
    let registry = ATOMIC_LONG_REGISTRY.lock().unwrap();
    let atomic = registry
        .get(&handle)
        .ok_or_else(|| "AtomicLong_set: invalid handle".to_string())?;
    atomic.store(val, Ordering::SeqCst);
    Ok(Value::Null)
}

pub(crate) fn native_atomic_long_fetch_add(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicLong_fetchAdd: expected handle and delta".to_string()),
    };
    let delta = match args.get(1) {
        Some(Value::Long(v)) => *v,
        Some(Value::Int(v)) => *v as i64,
        _ => return Err("AtomicLong_fetchAdd: expected an Int/Long delta".to_string()),
    };
    let registry = ATOMIC_LONG_REGISTRY.lock().unwrap();
    let atomic = registry
        .get(&handle)
        .ok_or_else(|| "AtomicLong_fetchAdd: invalid handle".to_string())?;
    let old = atomic.fetch_add(delta, Ordering::SeqCst);
    Ok(Value::Long(old))
}

pub(crate) fn native_atomic_long_fetch_sub(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicLong_fetchSub: expected handle and delta".to_string()),
    };
    let delta = match args.get(1) {
        Some(Value::Long(v)) => *v,
        Some(Value::Int(v)) => *v as i64,
        _ => return Err("AtomicLong_fetchSub: expected an Int/Long delta".to_string()),
    };
    let registry = ATOMIC_LONG_REGISTRY.lock().unwrap();
    let atomic = registry
        .get(&handle)
        .ok_or_else(|| "AtomicLong_fetchSub: invalid handle".to_string())?;
    let old = atomic.fetch_sub(delta, Ordering::SeqCst);
    Ok(Value::Long(old))
}

pub(crate) fn native_atomic_long_compare_and_swap(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicLong_compareAndSwap: expected handle, expected, new".to_string()),
    };
    let expected = match args.get(1) {
        Some(Value::Long(v)) => *v,
        Some(Value::Int(v)) => *v as i64,
        _ => return Err("AtomicLong_compareAndSwap: expected an Int/Long expected value".to_string()),
    };
    let new_val = match args.get(2) {
        Some(Value::Long(v)) => *v,
        Some(Value::Int(v)) => *v as i64,
        _ => return Err("AtomicLong_compareAndSwap: expected an Int/Long new value".to_string()),
    };
    let registry = ATOMIC_LONG_REGISTRY.lock().unwrap();
    let atomic = registry
        .get(&handle)
        .ok_or_else(|| "AtomicLong_compareAndSwap: invalid handle".to_string())?;
    let result = atomic.compare_exchange(expected, new_val, Ordering::SeqCst, Ordering::SeqCst);
    Ok(Value::Bool(result.is_ok()))
}

// ---------------------------------------------------------------------------
// AtomicRef registry (handle-based generic atomic reference)
// ---------------------------------------------------------------------------

// Since Value contains Rc<> which is not Send, we cannot store Value directly
// in a LazyLock (which requires Send). Instead, we store a boxed trait object
// that erases the type. For simplicity, we store string representations and
// reconstruct on get. A production implementation would need a more robust approach.
use std::any::Any;

static ATOMIC_REF_REGISTRY: LazyLock<StdMutex<HashMap<i64, Box<dyn Any + Send>>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static ATOMIC_REF_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

// Helper: convert Value to a Send-safe representation
fn value_to_sendable(val: &Value) -> Box<dyn Any + Send> {
    match val {
        Value::Null => Box::new(Option::<i64>::None),
        Value::Bool(b) => Box::new(*b),
        Value::Int(i) => Box::new(*i),
        Value::Long(l) => Box::new(*l),
        Value::Double(d) => Box::new(*d),
        Value::String(s) => Box::new(s.as_str().to_string()),
        _ => Box::new(format!("{:?}", val)), // fallback: debug representation
    }
}

// Helper: reconstruct Value from Send-safe representation
fn sendable_to_value(stored: &Box<dyn Any + Send>) -> Value {
    if let Some(&b) = stored.downcast_ref::<bool>() {
        return Value::Bool(b);
    }
    if let Some(&i) = stored.downcast_ref::<i32>() {
        return Value::Int(i);
    }
    if let Some(&l) = stored.downcast_ref::<i64>() {
        return Value::Long(l);
    }
    if let Some(&d) = stored.downcast_ref::<f64>() {
        return Value::Double(d);
    }
    if let Some(s) = stored.downcast_ref::<String>() {
        return Value::String(Rc::new(s.clone()));
    }
    if stored.downcast_ref::<Option<i64>>().is_some() {
        return Value::Null;
    }
    Value::Null
}

fn sendable_equals(a: &Box<dyn Any + Send>, b: &Box<dyn Any + Send>) -> bool {
    if let (Some(av), Some(bv)) = (a.downcast_ref::<bool>(), b.downcast_ref::<bool>()) {
        return av == bv;
    }
    if let (Some(av), Some(bv)) = (a.downcast_ref::<i32>(), b.downcast_ref::<i32>()) {
        return av == bv;
    }
    if let (Some(av), Some(bv)) = (a.downcast_ref::<i64>(), b.downcast_ref::<i64>()) {
        return av == bv;
    }
    if let (Some(av), Some(bv)) = (a.downcast_ref::<f64>(), b.downcast_ref::<f64>()) {
        return av == bv;
    }
    if let (Some(av), Some(bv)) = (a.downcast_ref::<String>(), b.downcast_ref::<String>()) {
        return av == bv;
    }
    // Both None (Null)?
    if a.downcast_ref::<Option<i64>>().is_some() && b.downcast_ref::<Option<i64>>().is_some() {
        return true;
    }
    false
}

pub(crate) fn native_atomic_ref_new(args: &[Value]) -> Result<Value, String> {
    let val = args.first().cloned().unwrap_or(Value::Null);
    let handle = ATOMIC_REF_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let mut registry = ATOMIC_REF_REGISTRY.lock().unwrap();
    registry.insert(handle, value_to_sendable(&val));
    Ok(Value::Long(handle))
}

pub(crate) fn native_atomic_ref_get(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicRef_get: expected an Int/Long handle".to_string()),
    };
    let registry = ATOMIC_REF_REGISTRY.lock().unwrap();
    let stored = registry
        .get(&handle)
        .ok_or_else(|| "AtomicRef_get: invalid handle".to_string())?;
    Ok(sendable_to_value(stored))
}

pub(crate) fn native_atomic_ref_set(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicRef_set: expected handle and value".to_string()),
    };
    let val = args.get(1).cloned().unwrap_or(Value::Null);
    let mut registry = ATOMIC_REF_REGISTRY.lock().unwrap();
    let cell = registry
        .get_mut(&handle)
        .ok_or_else(|| "AtomicRef_set: invalid handle".to_string())?;
    *cell = value_to_sendable(&val);
    Ok(Value::Null)
}

pub(crate) fn native_atomic_ref_compare_and_swap(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("AtomicRef_compareAndSwap: expected handle, expected, new".to_string()),
    };
    let expected = args.get(1).cloned().unwrap_or(Value::Null);
    let new_val = args.get(2).cloned().unwrap_or(Value::Null);
    let mut registry = ATOMIC_REF_REGISTRY.lock().unwrap();
    let current = registry
        .get(&handle)
        .ok_or_else(|| "AtomicRef_compareAndSwap: invalid handle".to_string())?;
    let expected_sendable = value_to_sendable(&expected);
    if sendable_equals(current, &expected_sendable) {
        let cell = registry.get_mut(&handle).unwrap();
        *cell = value_to_sendable(&new_val);
        Ok(Value::Bool(true))
    } else {
        Ok(Value::Bool(false))
    }
}

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
