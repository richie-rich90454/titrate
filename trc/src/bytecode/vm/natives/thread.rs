// Titrate Alpha 0.2 – bytecode virtual machine: thread natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{LazyLock, Mutex as StdMutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

static THREAD_REGISTRY: LazyLock<StdMutex<HashMap<i64, Option<JoinHandle<()>>>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static THREAD_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

pub(crate) fn native_thread_spawn(args: &[Value]) -> Result<Value, String> {
    let _ = args; // Full Titrate function execution in threads requires VM architecture changes
    let handle = THREAD_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let join_handle = thread::spawn(|| {
        // Placeholder: actual function execution would go here
    });
    let mut registry = THREAD_REGISTRY.lock().unwrap();
    registry.insert(handle, Some(join_handle));
    Ok(Value::Long(handle))
}

pub(crate) fn native_thread_join(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Thread_join: expected an Int/Long handle".to_string()),
    };
    let join_handle = {
        let mut registry = THREAD_REGISTRY.lock().unwrap();
        registry
            .get_mut(&handle)
            .and_then(|jh| jh.take())
            .ok_or_else(|| "Thread_join: invalid handle or already joined".to_string())?
    };
    join_handle
        .join()
        .map_err(|_| "Thread_join: thread panicked".to_string())?;
    Ok(Value::Null)
}

pub(crate) fn native_thread_sleep(args: &[Value]) -> Result<Value, String> {
    let ms = match args.first() {
        Some(Value::Long(ms)) => *ms,
        Some(Value::Int(ms)) => *ms as i64,
        _ => return Err("Thread_sleep: expected an Int argument".to_string()),
    };
    thread::sleep(Duration::from_millis(ms as u64));
    Ok(Value::Null)
}

pub(crate) fn native_thread_yield(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    thread::yield_now();
    Ok(Value::Null)
}

pub(crate) fn native_thread_get_id(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Thread_getId: expected an Int/Long handle".to_string()),
    };
    Ok(Value::Long(handle))
}

pub(crate) fn native_thread_current_id(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    // Use a hash of the thread id since as_u64() is unstable
    let tid = format!("{:?}", std::thread::current().id());
    let hash = tid.chars().fold(0i64, |acc, c| acc.wrapping_mul(31).wrapping_add(c as i64));
    Ok(Value::Long(hash))
}

pub(crate) fn native_thread_detach(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Thread_detach: expected an Int/Long handle".to_string()),
    };
    let mut registry = THREAD_REGISTRY.lock().unwrap();
    if let Some(jh) = registry.get_mut(&handle) {
        // Drop the JoinHandle without joining, effectively detaching the thread
        jh.take();
    }
    Ok(Value::Null)
}
