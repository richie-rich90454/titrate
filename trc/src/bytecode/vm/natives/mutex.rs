// Titrate Alpha 0.2 – bytecode virtual machine: mutex natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, LazyLock, Mutex as StdMutex, MutexGuard};

// ---------------------------------------------------------------------------
// Plain Mutex registry
// ---------------------------------------------------------------------------

static MUTEX_REGISTRY: LazyLock<StdMutex<HashMap<i64, Arc<StdMutex<()>>>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static MUTEX_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

pub(crate) fn native_mutex_new(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let handle = MUTEX_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let mutex = Arc::new(StdMutex::new(()));
    let mut registry = MUTEX_REGISTRY.lock().unwrap();
    registry.insert(handle, mutex);
    Ok(Value::Long(handle))
}

pub(crate) fn native_mutex_lock(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Mutex_lock: expected an Int/Long handle".to_string()),
    };
    let mutex = {
        let registry = MUTEX_REGISTRY.lock().unwrap();
        registry
            .get(&handle)
            .cloned()
            .ok_or_else(|| "Mutex_lock: invalid handle".to_string())?
    };
    // We lock and immediately drop the guard (release) since we can't hold it across calls.
    // In a real implementation, the lock would be held until unlock is called.
    // This requires a more sophisticated approach; for now we lock-unlock as a unit.
    let _guard: MutexGuard<()> = mutex.lock().unwrap();
    Ok(Value::Null)
}

pub(crate) fn native_mutex_unlock(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Mutex_unlock: expected an Int/Long handle".to_string()),
    };
    let registry = MUTEX_REGISTRY.lock().unwrap();
    if !registry.contains_key(&handle) {
        return Err("Mutex_unlock: invalid handle".to_string());
    }
    // Lock is released when the guard from native_mutex_lock is dropped.
    Ok(Value::Null)
}

pub(crate) fn native_mutex_try_lock(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Mutex_tryLock: expected an Int/Long handle".to_string()),
    };
    let mutex = {
        let registry = MUTEX_REGISTRY.lock().unwrap();
        registry
            .get(&handle)
            .cloned()
            .ok_or_else(|| "Mutex_tryLock: invalid handle".to_string())?
    };
    let result = mutex.try_lock();
    let ok = result.is_ok();
    // Drop the guard immediately if acquired (lock-unlock as a unit for now)
    drop(result);
    Ok(Value::Bool(ok))
}

// ---------------------------------------------------------------------------
// Recursive Mutex registry
// Uses Mutex<(thread_id, lock_count)> to track reentrant locks.
// ---------------------------------------------------------------------------

static RECURSIVE_MUTEX_REGISTRY: LazyLock<
    StdMutex<HashMap<i64, Arc<StdMutex<(i64, u32)>>>>,
> = LazyLock::new(|| StdMutex::new(HashMap::new()));
static RECURSIVE_MUTEX_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

fn current_thread_id_as_i64() -> i64 {
    let tid = format!("{:?}", std::thread::current().id());
    tid.chars().fold(0i64, |acc, c| acc.wrapping_mul(31).wrapping_add(c as i64))
}

pub(crate) fn native_recursive_mutex_new(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let handle = RECURSIVE_MUTEX_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let mutex = Arc::new(StdMutex::new((0i64, 0u32))); // (owner_thread_id, lock_count)
    let mut registry = RECURSIVE_MUTEX_REGISTRY.lock().unwrap();
    registry.insert(handle, mutex);
    Ok(Value::Long(handle))
}

pub(crate) fn native_recursive_mutex_lock(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("RecursiveMutex_lock: expected an Int/Long handle".to_string()),
    };
    let mutex = {
        let registry = RECURSIVE_MUTEX_REGISTRY.lock().unwrap();
        registry
            .get(&handle)
            .cloned()
            .ok_or_else(|| "RecursiveMutex_lock: invalid handle".to_string())?
    };
    let tid = current_thread_id_as_i64();
    // Check if we already hold the lock
    {
        let inner = mutex.lock().unwrap();
        if inner.0 == tid && inner.1 > 0 {
            // Reentrant: just increment count
            drop(inner);
            let mut inner = mutex.lock().unwrap();
            inner.1 += 1;
            return Ok(Value::Null);
        }
    }
    // Not held by us: block until we acquire
    // Spin-lock approach: try to acquire when count is 0
    loop {
        {
            let mut inner = mutex.lock().unwrap();
            if inner.1 == 0 {
                inner.0 = tid;
                inner.1 = 1;
                return Ok(Value::Null);
            }
            if inner.0 == tid {
                inner.1 += 1;
                return Ok(Value::Null);
            }
        }
        std::thread::yield_now();
    }
}

pub(crate) fn native_recursive_mutex_unlock(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("RecursiveMutex_unlock: expected an Int/Long handle".to_string()),
    };
    let mutex = {
        let registry = RECURSIVE_MUTEX_REGISTRY.lock().unwrap();
        registry
            .get(&handle)
            .cloned()
            .ok_or_else(|| "RecursiveMutex_unlock: invalid handle".to_string())?
    };
    let tid = current_thread_id_as_i64();
    let mut inner = mutex.lock().unwrap();
    if inner.0 != tid {
        return Err("RecursiveMutex_unlock: current thread does not hold the lock".to_string());
    }
    inner.1 -= 1;
    if inner.1 == 0 {
        inner.0 = 0;
    }
    Ok(Value::Null)
}

pub(crate) fn native_recursive_mutex_try_lock(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("RecursiveMutex_tryLock: expected an Int/Long handle".to_string()),
    };
    let mutex = {
        let registry = RECURSIVE_MUTEX_REGISTRY.lock().unwrap();
        registry
            .get(&handle)
            .cloned()
            .ok_or_else(|| "RecursiveMutex_tryLock: invalid handle".to_string())?
    };
    let tid = current_thread_id_as_i64();
    let mut inner = mutex.lock().unwrap();
    if inner.1 == 0 {
        inner.0 = tid;
        inner.1 = 1;
        Ok(Value::Bool(true))
    } else if inner.0 == tid {
        inner.1 += 1;
        Ok(Value::Bool(true))
    } else {
        Ok(Value::Bool(false))
    }
}

// ---------------------------------------------------------------------------
// SharedMutex (reader-writer lock) registry
// ---------------------------------------------------------------------------

static SHARED_MUTEX_REGISTRY: LazyLock<StdMutex<HashMap<i64, Arc<StdMutex<()>>>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static SHARED_MUTEX_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

pub(crate) fn native_shared_mutex_new(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let handle = SHARED_MUTEX_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let mutex = Arc::new(StdMutex::new(()));
    let mut registry = SHARED_MUTEX_REGISTRY.lock().unwrap();
    registry.insert(handle, mutex);
    Ok(Value::Long(handle))
}

pub(crate) fn native_shared_mutex_shared_lock(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    // Stub: in a real implementation, this would acquire a shared (reader) lock
    Ok(Value::Null)
}

pub(crate) fn native_shared_mutex_shared_unlock(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Null)
}

pub(crate) fn native_shared_mutex_unique_lock(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    // Stub: in a real implementation, this would acquire a unique (writer) lock
    Ok(Value::Null)
}

pub(crate) fn native_shared_mutex_unique_unlock(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Null)
}

pub(crate) fn native_shared_mutex_try_shared_lock(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Bool(true))
}

pub(crate) fn native_shared_mutex_try_unique_lock(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Bool(true))
}

// ---------------------------------------------------------------------------
// OnceFlag registry
// ---------------------------------------------------------------------------

pub(crate) fn native_once_flag_new(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Long(0))
}

pub(crate) fn native_once_flag_call_once(args: &[Value]) -> Result<Value, String> {
    // In a real implementation, this would ensure the function is called exactly once
    // For now, the .tr wrapper already guards with a `called` flag
    Ok(Value::Null)
}
