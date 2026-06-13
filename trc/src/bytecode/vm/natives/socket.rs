// Titrate Alpha 0.2 – bytecode virtual machine: socket natives (TCP + UDP)
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{LazyLock, Mutex as StdMutex};
use std::time::Duration;

// ---------------------------------------------------------------------------
// TCP Socket handle
// ---------------------------------------------------------------------------

enum TcpHandle {
    Listener(TcpListener),
    Stream(TcpStream),
}

static TCP_REGISTRY: LazyLock<StdMutex<HashMap<i64, TcpHandle>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static TCP_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

pub(crate) fn native_socket_new(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let handle = TCP_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    // Placeholder: actual socket created on connect/bind
    Ok(Value::Long(handle))
}

pub(crate) fn native_socket_connect(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Socket_connect: expected handle, host, port".to_string()),
    };
    let host = match args.get(1) {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("Socket_connect: expected a String host".to_string()),
    };
    let port = match args.get(2) {
        Some(Value::Long(p)) => *p,
        Some(Value::Int(p)) => *p as i64,
        _ => return Err("Socket_connect: expected an Int/Long port".to_string()),
    };
    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(&addr)
        .map_err(|e| format!("Socket_connect: {}", e))?;
    let mut registry = TCP_REGISTRY.lock().unwrap();
    registry.insert(handle, TcpHandle::Stream(stream));
    Ok(Value::Null)
}

pub(crate) fn native_socket_bind(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Socket_bind: expected handle, host, port".to_string()),
    };
    let host = match args.get(1) {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("Socket_bind: expected a String host".to_string()),
    };
    let port = match args.get(2) {
        Some(Value::Long(p)) => *p,
        Some(Value::Int(p)) => *p as i64,
        _ => return Err("Socket_bind: expected an Int/Long port".to_string()),
    };
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr)
        .map_err(|e| format!("Socket_bind: {}", e))?;
    let mut registry = TCP_REGISTRY.lock().unwrap();
    registry.insert(handle, TcpHandle::Listener(listener));
    Ok(Value::Null)
}

pub(crate) fn native_socket_listen(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Socket_listen: expected handle and backlog".to_string()),
    };
    let mut registry = TCP_REGISTRY.lock().unwrap();
    let tcp_handle = registry
        .get_mut(&handle)
        .ok_or_else(|| "Socket_listen: invalid handle".to_string())?;
    match tcp_handle {
        TcpHandle::Listener(listener) => {
            listener
                .set_nonblocking(false)
                .map_err(|e| format!("Socket_listen: {}", e))?;
            Ok(Value::Null)
        }
        TcpHandle::Stream(_) => Err("Socket_listen: handle is not a listener".to_string()),
    }
}

pub(crate) fn native_socket_accept(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Socket_accept: expected an Int/Long handle".to_string()),
    };
    // Clone the listener before releasing the lock to avoid holding mutex during blocking accept()
    let cloned_listener = {
        let registry = TCP_REGISTRY.lock().unwrap();
        let tcp_handle = registry
            .get(&handle)
            .ok_or_else(|| "Socket_accept: invalid handle".to_string())?;
        match tcp_handle {
            TcpHandle::Listener(listener) => listener
                .try_clone()
                .map_err(|e| format!("Socket_accept: failed to clone listener: {}", e))?,
            TcpHandle::Stream(_) => return Err("Socket_accept: handle is not a listener".to_string()),
        }
    };
    // Now perform the blocking accept on the cloned listener (mutex is released)
    let (stream, _addr) = cloned_listener
        .accept()
        .map_err(|e| format!("Socket_accept: {}", e))?;
    let client_handle = TCP_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let mut registry = TCP_REGISTRY.lock().unwrap();
    registry.insert(client_handle, TcpHandle::Stream(stream));
    Ok(Value::Long(client_handle))
}

pub(crate) fn native_socket_send(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Socket_send: expected handle and data".to_string()),
    };
    let data = match args.get(1) {
        Some(Value::String(s)) => s.as_bytes().to_vec(),
        _ => return Err("Socket_send: expected a String data argument".to_string()),
    };
    let mut registry = TCP_REGISTRY.lock().unwrap();
    let tcp_handle = registry
        .get_mut(&handle)
        .ok_or_else(|| "Socket_send: invalid handle".to_string())?;
    match tcp_handle {
        TcpHandle::Stream(stream) => {
            let written = stream
                .write(&data)
                .map_err(|e| format!("Socket_send: {}", e))?;
            Ok(Value::Long(written as i64))
        }
        TcpHandle::Listener(_) => Err("Socket_send: handle is not a stream".to_string()),
    }
}

pub(crate) fn native_socket_recv(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Socket_recv: expected handle and bufsize".to_string()),
    };
    let bufsize = match args.get(1) {
        Some(Value::Long(n)) => *n as usize,
        Some(Value::Int(n)) => *n as usize,
        _ => return Err("Socket_recv: expected an Int/Long buffer size".to_string()),
    };
    let mut buf = vec![0u8; bufsize];
    let mut registry = TCP_REGISTRY.lock().unwrap();
    let tcp_handle = registry
        .get_mut(&handle)
        .ok_or_else(|| "Socket_recv: invalid handle".to_string())?;
    match tcp_handle {
        TcpHandle::Stream(stream) => {
            let n = stream
                .read(&mut buf)
                .map_err(|e| format!("Socket_recv: {}", e))?;
            let s = String::from_utf8_lossy(&buf[..n]).to_string();
            Ok(Value::String(std::rc::Rc::new(s)))
        }
        TcpHandle::Listener(_) => Err("Socket_recv: handle is not a stream".to_string()),
    }
}

pub(crate) fn native_socket_close(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Socket_close: expected an Int/Long handle".to_string()),
    };
    let mut registry = TCP_REGISTRY.lock().unwrap();
    registry
        .remove(&handle)
        .ok_or_else(|| "Socket_close: invalid handle".to_string())?;
    Ok(Value::Null)
}

pub(crate) fn native_socket_set_timeout(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Socket_setTimeout: expected handle and ms".to_string()),
    };
    let ms = match args.get(1) {
        Some(Value::Long(ms)) => *ms,
        Some(Value::Int(ms)) => *ms as i64,
        _ => return Err("Socket_setTimeout: expected an Int/Long timeout".to_string()),
    };
    let duration = Some(Duration::from_millis(ms as u64));
    let mut registry = TCP_REGISTRY.lock().unwrap();
    let tcp_handle = registry
        .get_mut(&handle)
        .ok_or_else(|| "Socket_setTimeout: invalid handle".to_string())?;
    match tcp_handle {
        TcpHandle::Stream(stream) => {
            stream
                .set_read_timeout(duration)
                .map_err(|e| format!("Socket_setTimeout: {}", e))?;
            stream
                .set_write_timeout(duration)
                .map_err(|e| format!("Socket_setTimeout: {}", e))?;
            Ok(Value::Null)
        }
        TcpHandle::Listener(listener) => {
            // For listeners, there's no direct timeout; ignore
            let _ = listener;
            Ok(Value::Null)
        }
    }
}

pub(crate) fn native_socket_set_no_delay(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Socket_setNoDelay: expected handle and flag".to_string()),
    };
    let flag = match args.get(1) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Long(v)) => *v != 0,
        Some(Value::Int(v)) => *v != 0,
        _ => return Err("Socket_setNoDelay: expected a Bool flag".to_string()),
    };
    let mut registry = TCP_REGISTRY.lock().unwrap();
    let tcp_handle = registry
        .get_mut(&handle)
        .ok_or_else(|| "Socket_setNoDelay: invalid handle".to_string())?;
    match tcp_handle {
        TcpHandle::Stream(stream) => {
            stream
                .set_nodelay(flag)
                .map_err(|e| format!("Socket_setNoDelay: {}", e))?;
            Ok(Value::Null)
        }
        TcpHandle::Listener(_) => Err("Socket_setNoDelay: handle is not a stream".to_string()),
    }
}

// ---------------------------------------------------------------------------
// UDP Socket handle
// ---------------------------------------------------------------------------

struct UdpHandle {
    socket: UdpSocket,
    last_sender_host: String,
    last_sender_port: u16,
}

static UDP_REGISTRY: LazyLock<StdMutex<HashMap<i64, UdpHandle>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static UDP_NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

pub(crate) fn native_udp_socket_new(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let handle = UDP_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    // Placeholder: actual socket created on bind
    Ok(Value::Long(handle))
}

pub(crate) fn native_udp_socket_bind(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("UdpSocket_bind: expected handle, host, port".to_string()),
    };
    let host = match args.get(1) {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("UdpSocket_bind: expected a String host".to_string()),
    };
    let port = match args.get(2) {
        Some(Value::Long(p)) => *p,
        Some(Value::Int(p)) => *p as i64,
        _ => return Err("UdpSocket_bind: expected an Int/Long port".to_string()),
    };
    let addr = format!("{}:{}", host, port);
    let socket = UdpSocket::bind(&addr)
        .map_err(|e| format!("UdpSocket_bind: {}", e))?;
    let udp_handle = UdpHandle {
        socket,
        last_sender_host: String::new(),
        last_sender_port: 0,
    };
    let mut registry = UDP_REGISTRY.lock().unwrap();
    registry.insert(handle, udp_handle);
    Ok(Value::Null)
}

pub(crate) fn native_udp_socket_send_to(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("UdpSocket_sendTo: expected handle, data, host, port".to_string()),
    };
    let data = match args.get(1) {
        Some(Value::String(s)) => s.as_bytes().to_vec(),
        _ => return Err("UdpSocket_sendTo: expected a String data argument".to_string()),
    };
    let host = match args.get(2) {
        Some(Value::String(s)) => s.as_str().to_string(),
        _ => return Err("UdpSocket_sendTo: expected a String host".to_string()),
    };
    let port = match args.get(3) {
        Some(Value::Long(p)) => *p,
        Some(Value::Int(p)) => *p as i64,
        _ => return Err("UdpSocket_sendTo: expected an Int/Long port".to_string()),
    };
    let addr = format!("{}:{}", host, port);
    let mut registry = UDP_REGISTRY.lock().unwrap();
    let udp_handle = registry
        .get_mut(&handle)
        .ok_or_else(|| "UdpSocket_sendTo: invalid handle".to_string())?;
    let sent = udp_handle
        .socket
        .send_to(&data, &addr)
        .map_err(|e| format!("UdpSocket_sendTo: {}", e))?;
    Ok(Value::Long(sent as i64))
}

pub(crate) fn native_udp_socket_recv_from(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("UdpSocket_recvFrom: expected handle and bufsize".to_string()),
    };
    let bufsize = match args.get(1) {
        Some(Value::Long(n)) => *n as usize,
        Some(Value::Int(n)) => *n as usize,
        _ => return Err("UdpSocket_recvFrom: expected an Int/Long buffer size".to_string()),
    };
    let mut buf = vec![0u8; bufsize];
    let mut registry = UDP_REGISTRY.lock().unwrap();
    let udp_handle = registry
        .get_mut(&handle)
        .ok_or_else(|| "UdpSocket_recvFrom: invalid handle".to_string())?;
    let (n, sender) = udp_handle
        .socket
        .recv_from(&mut buf)
        .map_err(|e| format!("UdpSocket_recvFrom: {}", e))?;
    let s = String::from_utf8_lossy(&buf[..n]).to_string();
    udp_handle.last_sender_host = sender.ip().to_string();
    udp_handle.last_sender_port = sender.port();
    Ok(Value::String(std::rc::Rc::new(s)))
}

pub(crate) fn native_udp_socket_close(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("UdpSocket_close: expected an Int/Long handle".to_string()),
    };
    let mut registry = UDP_REGISTRY.lock().unwrap();
    registry
        .remove(&handle)
        .ok_or_else(|| "UdpSocket_close: invalid handle".to_string())?;
    Ok(Value::Null)
}

pub(crate) fn native_udp_socket_set_timeout(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("UdpSocket_setTimeout: expected handle and ms".to_string()),
    };
    let ms = match args.get(1) {
        Some(Value::Long(ms)) => *ms,
        Some(Value::Int(ms)) => *ms as i64,
        _ => return Err("UdpSocket_setTimeout: expected an Int/Long timeout".to_string()),
    };
    let duration = Some(Duration::from_millis(ms as u64));
    let mut registry = UDP_REGISTRY.lock().unwrap();
    let udp_handle = registry
        .get_mut(&handle)
        .ok_or_else(|| "UdpSocket_setTimeout: invalid handle".to_string())?;
    udp_handle
        .socket
        .set_read_timeout(duration)
        .map_err(|e| format!("UdpSocket_setTimeout: {}", e))?;
    udp_handle
        .socket
        .set_write_timeout(duration)
        .map_err(|e| format!("UdpSocket_setTimeout: {}", e))?;
    Ok(Value::Null)
}

pub(crate) fn native_udp_socket_last_sender_host(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("UdpSocket_lastSenderHost: expected an Int/Long handle".to_string()),
    };
    let registry = UDP_REGISTRY.lock().unwrap();
    let udp_handle = registry
        .get(&handle)
        .ok_or_else(|| "UdpSocket_lastSenderHost: invalid handle".to_string())?;
    Ok(Value::String(std::rc::Rc::new(udp_handle.last_sender_host.clone())))
}

pub(crate) fn native_udp_socket_last_sender_port(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("UdpSocket_lastSenderPort: expected an Int/Long handle".to_string()),
    };
    let registry = UDP_REGISTRY.lock().unwrap();
    let udp_handle = registry
        .get(&handle)
        .ok_or_else(|| "UdpSocket_lastSenderPort: invalid handle".to_string())?;
    Ok(Value::Long(udp_handle.last_sender_port as i64))
}

// ---------------------------------------------------------------------------
// DNS resolution
// ---------------------------------------------------------------------------

pub(crate) fn native_socket_get_addr_info(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Socket_getAddrInfo: expected 2 arguments (host, port)".to_string());
    }
    let host = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Socket_getAddrInfo: expected String host".to_string()),
    };
    let port = match &args[1] {
        Value::Int(p) => *p as u16,
        Value::Long(p) => *p as u16,
        _ => return Err("Socket_getAddrInfo: expected Int port".to_string()),
    };

    use std::rc::Rc;
    let addr_str = format!("{}:{}", host, port);
    match addr_str.to_socket_addrs() {
        Ok(addrs) => {
            let results: Vec<String> = addrs
                .map(|a: std::net::SocketAddr| a.to_string())
                .collect();
            // Return first resolved address as a string, or all joined by commas
            if results.is_empty() {
                Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
                    format!("Socket_getAddrInfo: no addresses found for {}", host)
                )))))
            } else {
                Ok(Value::ResultOk(Box::new(Value::String(Rc::new(results.join(","))))))
            }
        }
        Err(e) => Ok(Value::ResultErr(Box::new(Value::String(Rc::new(
            format!("Socket_getAddrInfo: {}", e)
        ))))),
    }
}

// ---------------------------------------------------------------------------
// Convenience socket functions
// ---------------------------------------------------------------------------

pub(crate) fn native_socket_inet_pton(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    // Stub: return empty string (IP address packing not yet implemented)
    Ok(Value::String(std::rc::Rc::new("".to_string())))
}

pub(crate) fn native_socket_inet_ntop(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    // Stub: return empty string
    Ok(Value::String(std::rc::Rc::new("".to_string())))
}

pub(crate) fn native_socket_create_connection(args: &[Value]) -> Result<Value, String> {
    // Create a client socket and connect to host:port
    if args.len() < 2 {
        return Err("Socket_createConnection: expected host and port".to_string());
    }
    let host = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Socket_createConnection: host must be a String".to_string()),
    };
    let port = match &args[1] {
        Value::Int(p) => *p as u16,
        Value::Long(p) => *p as u16,
        _ => return Err("Socket_createConnection: port must be an Int".to_string()),
    };
    // Create socket and connect
    let handle = TCP_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(&addr)
        .map_err(|e| format!("Socket_createConnection: {}", e))?;
    stream.set_nonblocking(false).ok();
    let mut registry = TCP_REGISTRY.lock().unwrap();
    registry.insert(handle, TcpHandle::Stream(stream));
    Ok(Value::Long(handle))
}

pub(crate) fn native_socket_create_server(args: &[Value]) -> Result<Value, String> {
    // Create a server socket bound to host:port with backlog
    if args.len() < 3 {
        return Err("Socket_createServer: expected host, port, and backlog".to_string());
    }
    let host = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Socket_createServer: host must be a String".to_string()),
    };
    let port = match &args[1] {
        Value::Int(p) => *p as u16,
        Value::Long(p) => *p as u16,
        _ => return Err("Socket_createServer: port must be an Int".to_string()),
    };
    let backlog = match &args[2] {
        Value::Int(b) => *b,
        Value::Long(b) => *b as i32,
        _ => return Err("Socket_createServer: backlog must be an Int".to_string()),
    };
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr)
        .map_err(|e| format!("Socket_createServer: {}", e))?;
    listener.set_nonblocking(false).ok();
    let handle = TCP_NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let mut registry = TCP_REGISTRY.lock().unwrap();
    registry.insert(handle, TcpHandle::Listener(listener));
    let _ = backlog; // backlog is handled by the OS TCP stack
    Ok(Value::Long(handle))
}

pub(crate) fn native_socket_get_local_address(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::String(std::rc::Rc::new("127.0.0.1".to_string())))
}

pub(crate) fn native_socket_get_remote_address(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Ok(Value::String(std::rc::Rc::new("0.0.0.0".to_string())))
}

pub(crate) fn native_socket_get_local_port(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Int(0))
}

pub(crate) fn native_socket_get_remote_port(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Int(0))
}

pub(crate) fn native_socket_set_reuse_addr(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Void)
}

pub(crate) fn native_socket_set_broadcast(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Void)
}

pub(crate) fn native_socket_set_keep_alive(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Void)
}

pub(crate) fn native_socket_set_linger(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Void)
}
