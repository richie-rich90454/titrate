// Titrate Alpha 0.2 – bytecode virtual machine: FTP natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{LazyLock, Mutex as StdMutex};
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT_MS: i64 = 30_000;
const MAX_TIMEOUT_MS: i64 = 86_400_000;
const ACCEPT_POLL_MS: u64 = 10;

enum FtpHandle {
    Stream(BufReader<TcpStream>),
    Listener { listener: TcpListener, timeout: Duration },
}

static FTP_REGISTRY: LazyLock<StdMutex<HashMap<i64, FtpHandle>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static FTP_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

fn new_handle() -> i64 {
    FTP_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst)
}

// Titrate strings carry bytes opaquely; Latin-1 keeps binary transfers lossless.
fn bytes_to_latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

fn latin1_to_bytes(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| if (c as u32) <= 0xff { c as u8 } else { b'?' })
        .collect()
}

fn timeout_duration(ms: i64) -> Duration {
    let ms = if ms <= 0 { DEFAULT_TIMEOUT_MS } else { ms.min(MAX_TIMEOUT_MS) };
    Duration::from_millis(ms as u64)
}

fn handle_of(v: &Value) -> Option<i64> {
    match v {
        Value::Long(h) => Some(*h),
        Value::Int(h) => Some(*h as i64),
        _ => None,
    }
}

fn arg_handle(args: &[Value], op: &str) -> Result<i64, String> {
    match args.first().and_then(handle_of) {
        Some(h) => Ok(h),
        None => Err(format!("Ftp_{}: expected an Int/Long handle", op)),
    }
}

fn arg_string<'a>(v: &'a Value, op: &str) -> Result<&'a str, String> {
    match v {
        Value::String(s) => Ok(s.as_str()),
        _ => Err(format!("Ftp_{}: expected a String argument", op)),
    }
}

fn with_stream<T, F>(args: &[Value], op: &str, f: F) -> Result<T, String>
where
    F: FnOnce(&mut BufReader<TcpStream>) -> Result<T, String>,
{
    let handle = arg_handle(args, op)?;
    let mut registry = FTP_REGISTRY.lock().unwrap();
    let entry = registry
        .get_mut(&handle)
        .ok_or_else(|| format!("Ftp_{}: invalid handle {}", op, handle))?;
    match entry {
        FtpHandle::Stream(reader) => f(reader),
        FtpHandle::Listener { .. } => Err(format!("Ftp_{}: handle is not a stream", op)),
    }
}

fn write_stream(args: &[Value], op: &str) -> Result<Value, String> {
    if args.len() < 2 {
        return Err(format!("Ftp_{}: expected (handle, string)", op));
    }
    let data = arg_string(&args[1], op)?.to_string();
    let bytes = latin1_to_bytes(&data);
    with_stream(args, op, |reader| {
        reader
            .get_mut()
            .write_all(&bytes)
            .map_err(|e| format!("Ftp_{}: {}", op, e))?;
        reader
            .get_mut()
            .flush()
            .map_err(|e| format!("Ftp_{}: {}", op, e))?;
        Ok(Value::Bool(true))
    })
}

pub(crate) fn native_ftp_connect(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Ftp_connect: expected (host, port, timeoutMs)".to_string());
    }
    let host = arg_string(&args[0], "connect")?.to_string();
    let port = args[1]
        .to_i64()
        .ok_or_else(|| "Ftp_connect: expected a numeric port".to_string())?;
    if port < 0 || port > u16::MAX as i64 {
        return Ok(Value::Long(-1));
    }
    let timeout = timeout_duration(args.get(2).and_then(|v| v.to_i64()).unwrap_or(0));

    let addrs = match (host.as_str(), port as u16).to_socket_addrs() {
        Ok(addrs) => addrs.collect::<Vec<_>>(),
        Err(_) => return Ok(Value::Long(-1)),
    };
    for addr in addrs {
        if let Ok(stream) = TcpStream::connect_timeout(&addr, timeout) {
            let _ = stream.set_read_timeout(Some(timeout));
            let _ = stream.set_write_timeout(Some(timeout));
            let handle = new_handle();
            FTP_REGISTRY
                .lock()
                .unwrap()
                .insert(handle, FtpHandle::Stream(BufReader::new(stream)));
            return Ok(Value::Long(handle));
        }
    }
    Ok(Value::Long(-1))
}

pub(crate) fn native_ftp_close(args: &[Value]) -> Result<Value, String> {
    let handle = arg_handle(args, "close")?;
    let removed = FTP_REGISTRY.lock().unwrap().remove(&handle).is_some();
    Ok(Value::Bool(removed))
}

pub(crate) fn native_ftp_write(args: &[Value]) -> Result<Value, String> {
    write_stream(args, "write")
}

pub(crate) fn native_ftp_read_line(args: &[Value]) -> Result<Value, String> {
    with_stream(args, "readLine", |reader| {
        let mut buf = Vec::new();
        let n = reader
            .read_until(b'\n', &mut buf)
            .map_err(|e| format!("Ftp_readLine: {}", e))?;
        if n == 0 {
            return Ok(Value::String(Rc::new(String::new())));
        }
        if buf.last() == Some(&b'\n') {
            buf.pop();
        }
        if buf.last() == Some(&b'\r') {
            buf.pop();
        }
        Ok(Value::String(Rc::new(bytes_to_latin1(&buf))))
    })
}

pub(crate) fn native_ftp_read_data(args: &[Value]) -> Result<Value, String> {
    let max_bytes = args
        .get(1)
        .and_then(|v| v.to_i64())
        .unwrap_or(4096);
    let max_bytes = if max_bytes > 0 { max_bytes as usize } else { 4096 };
    let mut buf = vec![0u8; max_bytes];
    with_stream(args, "readData", |reader| {
        match reader.read(&mut buf) {
            Ok(0) => Ok(Value::String(Rc::new(String::new()))),
            Ok(n) => Ok(Value::String(Rc::new(bytes_to_latin1(&buf[..n])))),
            Err(e) if e.kind() == ErrorKind::TimedOut || e.kind() == ErrorKind::WouldBlock => {
                Ok(Value::String(Rc::new(String::new())))
            }
            Err(e) => Err(format!("Ftp_readData: {}", e)),
        }
    })
}

pub(crate) fn native_ftp_write_data(args: &[Value]) -> Result<Value, String> {
    write_stream(args, "writeData")
}

pub(crate) fn native_ftp_listen(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Ftp_listen: expected (host, port, timeoutMs)".to_string());
    }
    let host = arg_string(&args[0], "listen")?.to_string();
    let port = args[1]
        .to_i64()
        .ok_or_else(|| "Ftp_listen: expected a numeric port".to_string())?;
    if port < 0 || port > u16::MAX as i64 {
        return Ok(Value::Long(-1));
    }
    let timeout = timeout_duration(args.get(2).and_then(|v| v.to_i64()).unwrap_or(0));

    let addrs = match (host.as_str(), port as u16).to_socket_addrs() {
        Ok(addrs) => addrs.collect::<Vec<_>>(),
        Err(_) => return Ok(Value::Long(-1)),
    };
    for addr in addrs {
        if let Ok(listener) = TcpListener::bind(addr) {
            // Accept has no OS-level timeout; the listener stays nonblocking
            // so Ftp_accept can poll it against a deadline.
            if listener.set_nonblocking(true).is_err() {
                return Ok(Value::Long(-1));
            }
            let handle = new_handle();
            FTP_REGISTRY
                .lock()
                .unwrap()
                .insert(handle, FtpHandle::Listener { listener, timeout });
            return Ok(Value::Long(handle));
        }
    }
    Ok(Value::Long(-1))
}

pub(crate) fn native_ftp_get_local_address(args: &[Value]) -> Result<Value, String> {
    let handle = arg_handle(args, "getLocalAddress")?;
    let registry = FTP_REGISTRY.lock().unwrap();
    let address = match registry.get(&handle) {
        Some(FtpHandle::Stream(reader)) => reader
            .get_ref()
            .local_addr()
            .map(|a| a.ip().to_string())
            .unwrap_or_default(),
        Some(FtpHandle::Listener { listener, .. }) => listener
            .local_addr()
            .map(|a| a.ip().to_string())
            .unwrap_or_default(),
        None => String::new(),
    };
    Ok(Value::String(Rc::new(address)))
}

pub(crate) fn native_ftp_get_local_port(args: &[Value]) -> Result<Value, String> {
    let handle = arg_handle(args, "getLocalPort")?;
    let registry = FTP_REGISTRY.lock().unwrap();
    let port = match registry.get(&handle) {
        Some(FtpHandle::Stream(reader)) => reader
            .get_ref()
            .local_addr()
            .map(|a| a.port())
            .unwrap_or(0),
        Some(FtpHandle::Listener { listener, .. }) => listener
            .local_addr()
            .map(|a| a.port())
            .unwrap_or(0),
        None => 0,
    };
    Ok(Value::Int(port as i32))
}

pub(crate) fn native_ftp_accept(args: &[Value]) -> Result<Value, String> {
    let handle = arg_handle(args, "accept")?;
    let (listener, listener_timeout) = {
        let registry = FTP_REGISTRY.lock().unwrap();
        match registry.get(&handle) {
            Some(FtpHandle::Listener { listener, timeout }) => (
                listener
                    .try_clone()
                    .map_err(|e| format!("Ftp_accept: {}", e))?,
                *timeout,
            ),
            Some(FtpHandle::Stream(_)) => {
                return Err("Ftp_accept: handle is not a listener".to_string());
            }
            None => return Ok(Value::Long(-1)),
        }
    };
    let override_ms = args.get(1).and_then(|v| v.to_i64()).unwrap_or(0);
    let timeout = if override_ms > 0 {
        timeout_duration(override_ms)
    } else {
        listener_timeout
    };
    let deadline = Instant::now() + timeout;
    loop {
        match listener.accept() {
            Ok((stream, _addr)) => {
                let _ = stream.set_read_timeout(Some(timeout));
                let _ = stream.set_write_timeout(Some(timeout));
                let new = new_handle();
                FTP_REGISTRY
                    .lock()
                    .unwrap()
                    .insert(new, FtpHandle::Stream(BufReader::new(stream)));
                return Ok(Value::Long(new));
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                if Instant::now() >= deadline {
                    return Ok(Value::Long(-1));
                }
                std::thread::sleep(Duration::from_millis(ACCEPT_POLL_MS));
            }
            Err(e) => return Err(format!("Ftp_accept: {}", e)),
        }
    }
}
