// Titrate Alpha 0.2 – bytecode virtual machine: multiprocessing natives
// Precision in every step – richie-rich90454, 2026
//
// Native support for the tt::concurrent::Multiprocessing stdlib module.
// Processes are real OS processes spawned via std::process::Command and
// tracked by integer handle. Queues are in-process shared queues backed by a
// thread_local registry (stub: real cross-process IPC requires serialization
// of Value which contains non-Send Rc<> pointers; the queue API is functional
// within a single VM thread, satisfying the native binding contract).

use super::super::super::value::Value;
use std::collections::VecDeque;
use std::process::Child;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{LazyLock, Mutex as StdMutex};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Process registry — OS child processes keyed by handle
// ---------------------------------------------------------------------------

static PROCESS_REGISTRY: LazyLock<StdMutex<HashMap<i64, Child>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static PROCESS_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

/// Process_spawn(program: String, [args...]) -> Long handle
///
/// Spawns a real OS process using `program` as the executable and any
/// additional string arguments as argv. The returned Long is a handle that
/// can be passed to Process_join or Process_terminate.
pub(crate) fn native_process_spawn(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Process_spawn: expected at least 1 argument (program)".to_string());
    }
    let program = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Process_spawn: expected a String program argument".to_string()),
    };
    let mut cmd = std::process::Command::new(&program);
    for arg in &args[1..] {
        match arg {
            Value::String(s) => {
                cmd.arg(s.as_str());
            }
            other => {
                cmd.arg(format!("{:?}", other));
            }
        }
    }
    let child = cmd
        .spawn()
        .map_err(|e| format!("Process_spawn: failed to spawn '{}': {}", program, e))?;
    let handle = PROCESS_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let mut registry = PROCESS_REGISTRY.lock().unwrap();
    registry.insert(handle, child);
    Ok(Value::Long(handle))
}

/// Process_join(handle: Long) -> Int exit_code
///
/// Blocks until the referenced child process exits and returns its exit code.
/// Returns -1 if the exit code cannot be determined (e.g. terminated by signal).
pub(crate) fn native_process_join(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Process_join: expected an Int/Long handle".to_string()),
    };
    let mut child = {
        let mut registry = PROCESS_REGISTRY.lock().unwrap();
        registry
            .remove(&handle)
            .ok_or_else(|| "Process_join: invalid handle or already joined".to_string())?
    };
    let status = child
        .wait()
        .map_err(|e| format!("Process_join: failed to wait for child: {}", e))?;
    Ok(Value::Int(status.code().unwrap_or(-1)))
}

/// Process_terminate(handle: Long) -> Null
///
/// Kills the referenced child process. The handle is removed from the
/// registry; subsequent calls with the same handle return an error.
pub(crate) fn native_process_terminate(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Process_terminate: expected an Int/Long handle".to_string()),
    };
    let mut child = {
        let mut registry = PROCESS_REGISTRY.lock().unwrap();
        registry
            .remove(&handle)
            .ok_or_else(|| "Process_terminate: invalid handle or already joined".to_string())?
    };
    let _ = child.kill();
    let _ = child.wait();
    Ok(Value::Null)
}

// ---------------------------------------------------------------------------
// Queue registry — in-process FIFO queue keyed by handle
// ---------------------------------------------------------------------------

struct QueueState {
    items: VecDeque<Value>,
    closed: bool,
}

thread_local! {
    static QUEUE_REGISTRY: std::cell::RefCell<HashMap<i64, QueueState>> =
        std::cell::RefCell::new(HashMap::new());
}
static QUEUE_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

/// Queue_new() -> Long handle
///
/// Creates a new in-process FIFO queue and returns a handle.
pub(crate) fn native_queue_new(_args: &[Value]) -> Result<Value, String> {
    let handle = QUEUE_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    QUEUE_REGISTRY.with(|r| {
        r.borrow_mut().insert(
            handle,
            QueueState {
                items: VecDeque::new(),
                closed: false,
            },
        );
    });
    Ok(Value::Long(handle))
}

/// Queue_send(handle: Long, value) -> Null
///
/// Appends `value` to the back of the queue. Returns an error if the queue
/// has been closed.
pub(crate) fn native_queue_send(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Queue_send: expected 2 arguments (handle, value)".to_string());
    }
    let handle = match &args[0] {
        Value::Long(h) => *h,
        Value::Int(h) => *h as i64,
        _ => return Err("Queue_send: expected an Int/Long handle".to_string()),
    };
    let value = args[1].clone();
    QUEUE_REGISTRY.with(|r| {
        let mut registry = r.borrow_mut();
        let state = registry
            .get_mut(&handle)
            .ok_or_else(|| "Queue_send: invalid queue handle".to_string())?;
        if state.closed {
            return Err("Queue_send: queue is closed".to_string());
        }
        state.items.push_back(value);
        Ok(Value::Null)
    })
}

/// Queue_recv(handle: Long) -> value
///
/// Removes and returns the front element of the queue. Returns Null if the
/// queue is empty (even if closed) so callers can poll without raising.
pub(crate) fn native_queue_recv(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Queue_recv: expected an Int/Long handle".to_string()),
    };
    QUEUE_REGISTRY.with(|r| {
        let mut registry = r.borrow_mut();
        let state = registry
            .get_mut(&handle)
            .ok_or_else(|| "Queue_recv: invalid queue handle".to_string())?;
        Ok(state.items.pop_front().unwrap_or(Value::Null))
    })
}

/// Queue_close(handle: Long) -> Null
///
/// Marks the queue as closed. Subsequent Queue_send calls return an error.
/// The handle is removed from the registry so its storage is reclaimed.
pub(crate) fn native_queue_close(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Queue_close: expected an Int/Long handle".to_string()),
    };
    QUEUE_REGISTRY.with(|r| {
        let mut registry = r.borrow_mut();
        if let Some(state) = registry.get_mut(&handle) {
            state.closed = true;
        }
        registry.remove(&handle);
    });
    Ok(Value::Null)
}
