// Titrate Alpha 0.2 – bytecode virtual machine: semaphore natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Condvar, LazyLock, Mutex as StdMutex};

struct CustomSemaphore {
    permits: StdMutex<usize>,
    condvar: Condvar,
}

static SEM_REGISTRY: LazyLock<StdMutex<HashMap<i64, Arc<CustomSemaphore>>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static SEM_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

pub(crate) fn native_sem_new(args: &[Value]) -> Result<Value, String> {
    let initial = match args.first() {
        Some(Value::Long(v)) => *v as usize,
        Some(Value::Int(v)) => *v as usize,
        _ => return Err("Semaphore_new: expected an Int initial permit count".to_string()),
    };
    let handle = SEM_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let sem = Arc::new(CustomSemaphore {
        permits: StdMutex::new(initial),
        condvar: Condvar::new(),
    });
    let mut registry = SEM_REGISTRY.lock().unwrap();
    registry.insert(handle, sem);
    Ok(Value::Long(handle))
}

pub(crate) fn native_sem_acquire(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Semaphore_acquire: expected an Int/Long handle".to_string()),
    };
    let sem = {
        let registry = SEM_REGISTRY.lock().unwrap();
        registry
            .get(&handle)
            .cloned()
            .ok_or_else(|| "Semaphore_acquire: invalid handle".to_string())?
    };
    let mut permits = sem.permits.lock().unwrap();
    while *permits == 0 {
        permits = sem.condvar.wait(permits).unwrap();
    }
    *permits -= 1;
    Ok(Value::Null)
}

pub(crate) fn native_sem_release(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Semaphore_release: expected an Int/Long handle".to_string()),
    };
    let sem = {
        let registry = SEM_REGISTRY.lock().unwrap();
        registry
            .get(&handle)
            .cloned()
            .ok_or_else(|| "Semaphore_release: invalid handle".to_string())?
    };
    {
        let mut permits = sem.permits.lock().unwrap();
        *permits += 1;
    }
    sem.condvar.notify_one();
    Ok(Value::Null)
}

pub(crate) fn native_sem_try_acquire(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Semaphore_tryAcquire: expected an Int/Long handle".to_string()),
    };
    let sem = {
        let registry = SEM_REGISTRY.lock().unwrap();
        registry
            .get(&handle)
            .cloned()
            .ok_or_else(|| "Semaphore_tryAcquire: invalid handle".to_string())?
    };
    let mut permits = sem.permits.lock().unwrap();
    if *permits > 0 {
        *permits -= 1;
        Ok(Value::Bool(true))
    } else {
        Ok(Value::Bool(false))
    }
}

pub(crate) fn native_sem_available_permits(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Semaphore_availablePermits: expected an Int/Long handle".to_string()),
    };
    let sem = {
        let registry = SEM_REGISTRY.lock().unwrap();
        registry
            .get(&handle)
            .cloned()
            .ok_or_else(|| "Semaphore_availablePermits: invalid handle".to_string())?
    };
    let permits = sem.permits.lock().unwrap();
    Ok(Value::Long(*permits as i64))
}
