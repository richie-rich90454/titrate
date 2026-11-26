// Titrate Alpha 0.2 – bytecode virtual machine: net natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use super::NativeContext;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::net::{TcpStream, ToSocketAddrs};
use std::rc::Rc;
use std::time::Duration;

/// Offline HTTP fixture: both the bytecode VM and the WASM host return this
/// exact body for the fixture URL, so parity tests never touch the network.
pub(crate) const HTTP_FIXTURE_URL: &str = "http://titrate.invalid/fixture";
const DEFAULT_HTTP_TIMEOUT_MS: i64 = 30_000;
const MAX_HTTP_REDIRECTS: usize = 8;

thread_local! {
    static HTTP_TIMEOUT_MS: Cell<i64> = const { Cell::new(DEFAULT_HTTP_TIMEOUT_MS) };
    static HTTP_FOLLOW_REDIRECTS: Cell<bool> = const { Cell::new(true) };
}

fn http_fixture(url: &str, method: &str) -> Option<Value> {
    if url.starts_with(HTTP_FIXTURE_URL) {
        Some(Value::String(Rc::new(format!("fixture:{}", method))))
    } else {
        None
    }
}

fn http_timeout_duration() -> Duration {
    let ms = HTTP_TIMEOUT_MS.with(|c| c.get());
    if ms <= 0 {
        Duration::from_secs(30)
    } else {
        Duration::from_millis(ms as u64)
    }
}

fn http_connect(addr: &str, what: &str) -> Result<TcpStream, String> {
    let timeout = http_timeout_duration();
    let mut last_error = None;
    for target in addr
        .to_socket_addrs()
        .map_err(|e| format!("{}: failed to resolve '{}': {}", what, addr, e))?
    {
        match TcpStream::connect_timeout(&target, timeout) {
            Ok(stream) => {
                let _ = stream.set_read_timeout(Some(timeout));
                let _ = stream.set_write_timeout(Some(timeout));
                return Ok(stream);
            }
            Err(e) => last_error = Some(e),
        }
    }
    Err(format!(
        "{}: connection failed: {}",
        what,
        last_error
            .map(|e| e.to_string())
            .unwrap_or_else(|| "no addresses".to_string())
    ))
}

/// Send one request and return (status code, Location header, body).
fn http_send_once(
    ctx: &NativeContext,
    method: &str,
    url: &str,
    body: Option<(&str, &str)>,
) -> Result<(u16, Option<String>, String), String> {
    let what = format!("Http_{}", method.to_lowercase());
    let (host, port, path) = parse_http_url(url)?;
    let addr = format!("{}:{}", host, port);
    let mut stream = http_connect(&addr, &what)?;

    let request = match body {
        Some((data, content_type)) => format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            method, path, host, content_type, data.len(), data
        ),
        None => format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            method, path, host
        ),
    };
    use std::io::Write;
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("{}: write failed: {}", what, e))?;

    let response = super::read_bounded_ctx(ctx, &mut stream, &what)?;
    let response_str = String::from_utf8_lossy(&response);
    let header_end = response_str.find("\r\n\r\n").unwrap_or(response_str.len());
    let headers = &response_str[..header_end];
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    let location = headers.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key.eq_ignore_ascii_case("location") {
            Some(value.trim().to_string())
        } else {
            None
        }
    });
    let response_body = if header_end < response_str.len() {
        response_str[header_end + 4..].to_string()
    } else {
        response_str.to_string()
    };
    Ok((status, location, response_body))
}

fn http_resolve_location(current: &str, location: &str) -> Result<String, String> {
    if location.starts_with("http://") || location.starts_with("https://") {
        return Ok(location.to_string());
    }
    let is_https = current.starts_with("https://");
    let (host, port, _) = parse_http_url(current)?;
    let scheme = if is_https { "https://" } else { "http://" };
    let authority = if (is_https && port == 443) || (!is_https && port == 80) {
        host
    } else {
        format!("{}:{}", host, port)
    };
    if location.starts_with('/') {
        Ok(format!("{}{}{}", scheme, authority, location))
    } else {
        let base = current.rsplit_once('/').map(|(b, _)| b).unwrap_or(current);
        Ok(format!("{}{}{}/{}", scheme, authority, base, location))
    }
}

fn http_request(
    ctx: &NativeContext,
    method: &str,
    url: &str,
    body: Option<(&str, &str)>,
) -> Result<String, String> {
    let mut current = url.to_string();
    let mut current_method = method.to_string();
    let mut current_body = body;
    for _ in 0..=MAX_HTTP_REDIRECTS {
        let (status, location, response) =
            http_send_once(ctx, &current_method, &current, current_body)?;
        let follow = HTTP_FOLLOW_REDIRECTS.with(|c| c.get());
        if follow && matches!(status, 301 | 302 | 303 | 307 | 308) {
            if let Some(location) = location {
                current = http_resolve_location(&current, &location)?;
                if status == 303 {
                    current_method = "GET".to_string();
                    current_body = None;
                }
                continue;
            }
        }
        return Ok(response);
    }
    Err(format!("Http_{}: too many redirects", method.to_lowercase()))
}

pub(crate) fn native_net_connect(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Net_connect: expected 2 arguments (host, port)".to_string());
    }
    let host = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Net_connect: expected String host".to_string()),
    };
    let port = args[1].to_i64().unwrap_or(0);
    let addr = format!("{}:{}", host, port);
    match http_connect(&addr, "Net_connect") {
        Ok(stream) => Ok(Value::Socket(Rc::new(RefCell::new(Some(stream))))),
        Err(e) => Err(e),
    }
}

pub(crate) fn native_net_send(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Net_send: expected 2 arguments (socket, data)".to_string());
    }
    match (&args[0], &args[1]) {
        (Value::Socket(socket_rc), Value::String(data)) => {
            let mut socket_opt = socket_rc.borrow_mut();
            match socket_opt.as_mut() {
                Some(stream) => {
                    use std::io::Write;
                    match stream.write_all(data.as_bytes()) {
                        Ok(()) => Ok(Value::Int(data.len() as i32)),
                        Err(e) => Err(format!("Net_send: {}", e)),
                    }
                }
                None => Err("Net_send: socket is closed".to_string()),
            }
        }
        _ => Err("Net_send: expected (Socket, String)".to_string()),
    }
}

pub(crate) fn native_net_receive_ctx(ctx: &NativeContext, args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Net_receive: expected 2 arguments (socket, maxBytes)".to_string());
    }
    let max_bytes = args[1].to_i64().unwrap_or(4096);
    let max_bytes = super::checked_len_ctx(ctx, max_bytes, "Net_receive")?;
    match &args[0] {
        Value::Socket(socket_rc) => {
            let mut socket_opt = socket_rc.borrow_mut();
            match socket_opt.as_mut() {
                Some(stream) => {
                    use std::io::Read;
                    let mut buf = vec![0u8; max_bytes];
                    match stream.read(&mut buf) {
                        Ok(n) => {
                            let s = String::from_utf8_lossy(&buf[..n]).to_string();
                            Ok(Value::String(Rc::new(s)))
                        }
                        Err(e) => Err(format!("Net_receive: {}", e)),
                    }
                }
                None => Err("Net_receive: socket is closed".to_string()),
            }
        }
        _ => Err("Net_receive: expected Socket argument".to_string()),
    }
}

pub(crate) fn native_net_bind(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Net_bind: expected 1 argument (port)".to_string());
    }
    let port = args[0].to_i64().unwrap_or(0);
    let addr = format!("0.0.0.0:{}", port);
    match std::net::TcpListener::bind(&addr) {
        Ok(listener) => Ok(Value::Listener(Rc::new(RefCell::new(Some(listener))))),
        Err(e) => Err(format!("Net_bind: failed to bind to port {}: {}", port, e)),
    }
}

pub(crate) fn native_net_accept(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Net_accept: expected 1 argument (listener)".to_string());
    }
    match &args[0] {
        Value::Listener(listener_rc) => {
            let mut listener_opt = listener_rc.borrow_mut();
            match listener_opt.as_mut() {
                Some(listener) => {
                    let (stream, _addr) = listener.accept()
                        .map_err(|e| format!("Net_accept: {}", e))?;
                    Ok(Value::Socket(Rc::new(RefCell::new(Some(stream)))))
                }
                None => Err("Net_accept: listener is closed".to_string()),
            }
        }
        _ => Err("Net_accept: expected Listener argument".to_string()),
    }
}

pub(crate) fn native_net_close(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Net_close: expected 1 argument (socket or listener)".to_string());
    }
    match &args[0] {
        Value::Socket(socket_rc) => {
            let mut socket_opt = socket_rc.borrow_mut();
            *socket_opt = None;
            Ok(Value::Void)
        }
        Value::Listener(listener_rc) => {
            let mut listener_opt = listener_rc.borrow_mut();
            *listener_opt = None;
            Ok(Value::Void)
        }
        // Tolerate non-Socket/Listener args (e.g. uninitialized handle = -1)
        // as no-ops so closing a fresh TcpServer/TcpClient does not raise.
        _ => Ok(Value::Void),
    }
}

pub(crate) fn native_http_get_ctx(ctx: &NativeContext, args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Http_get: expected 1 argument (url)".to_string());
    }
    let url = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_get: expected String url".to_string()),
    };
    if let Some(v) = http_fixture(url, "GET") {
        return Ok(v);
    }
    Ok(Value::String(Rc::new(http_request(ctx, "GET", url, None)?)))
}

pub(crate) fn native_http_post_ctx(ctx: &NativeContext, args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("Http_post: expected 3 arguments (url, body, contentType)".to_string());
    }
    let url = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_post: expected String url".to_string()),
    };
    let body = match &args[1] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_post: expected String body".to_string()),
    };
    let content_type = match &args[2] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_post: expected String contentType".to_string()),
    };
    if let Some(v) = http_fixture(url, "POST") {
        return Ok(v);
    }
    Ok(Value::String(Rc::new(http_request(
        ctx,
        "POST",
        url,
        Some((body, content_type)),
    )?)))
}

pub(crate) fn native_http_put_ctx(ctx: &NativeContext, args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("Http_put: expected 3 arguments (url, body, contentType)".to_string());
    }
    let url = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_put: expected String url".to_string()),
    };
    let body = match &args[1] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_put: expected String body".to_string()),
    };
    let content_type = match &args[2] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_put: expected String contentType".to_string()),
    };
    if let Some(v) = http_fixture(url, "PUT") {
        return Ok(v);
    }
    Ok(Value::String(Rc::new(http_request(
        ctx,
        "PUT",
        url,
        Some((body, content_type)),
    )?)))
}

pub(crate) fn native_http_delete_ctx(ctx: &NativeContext, args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Http_delete: expected 1 argument (url)".to_string());
    }
    let url = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_delete: expected String url".to_string()),
    };
    if let Some(v) = http_fixture(url, "DELETE") {
        return Ok(v);
    }
    Ok(Value::String(Rc::new(http_request(ctx, "DELETE", url, None)?)))
}

pub(crate) fn native_http_patch_ctx(ctx: &NativeContext, args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("Http_patch: expected 3 arguments (url, body, contentType)".to_string());
    }
    let url = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_patch: expected String url".to_string()),
    };
    let body = match &args[1] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_patch: expected String body".to_string()),
    };
    let content_type = match &args[2] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_patch: expected String contentType".to_string()),
    };
    if let Some(v) = http_fixture(url, "PATCH") {
        return Ok(v);
    }
    Ok(Value::String(Rc::new(http_request(
        ctx,
        "PATCH",
        url,
        Some((body, content_type)),
    )?)))
}

pub(crate) fn native_http_head_ctx(ctx: &NativeContext, args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Http_head: expected 1 argument (url)".to_string());
    }
    let url = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_head: expected String url".to_string()),
    };
    if let Some(v) = http_fixture(url, "HEAD") {
        return Ok(v);
    }
    Ok(Value::String(Rc::new(http_request(ctx, "HEAD", url, None)?)))
}

pub(crate) fn native_http_set_timeout(args: &[Value]) -> Result<Value, String> {
    let ms = match args.first() {
        Some(v) => v.to_i64().unwrap_or(-1),
        None => return Err("Http_setTimeout: expected a timeout in milliseconds".to_string()),
    };
    if ms < 0 {
        return Err(format!("Http_setTimeout: negative timeout {}", ms));
    }
    HTTP_TIMEOUT_MS.with(|c| c.set(ms));
    Ok(Value::Void)
}

pub(crate) fn native_http_set_follow_redirects(args: &[Value]) -> Result<Value, String> {
    let enabled = match args.first() {
        Some(Value::Bool(b)) => *b,
        Some(v) => v.to_i64().unwrap_or(0) != 0,
        None => {
            return Err("Http_setFollowRedirects: expected a Bool flag".to_string())
        }
    };
    HTTP_FOLLOW_REDIRECTS.with(|c| c.set(enabled));
    Ok(Value::Void)
}

pub(crate) fn parse_http_url(url: &str) -> Result<(String, u16, String), String> {
    let is_https = url.starts_with("https://");
    let url = url.strip_prefix("http://").unwrap_or(url);
    let url = url.strip_prefix("https://").unwrap_or(url);

    let (host_port, path) = match url.find('/') {
        Some(idx) => (&url[..idx], &url[idx..]),
        None => (url, "/"),
    };

    let default_port: u16 = if is_https { 443 } else { 80 };
    let (host, port) = if host_port.contains(':') {
        let parts: Vec<&str> = host_port.splitn(2, ':').collect();
        let port: u16 = parts[1].parse().unwrap_or(default_port);
        (parts[0].to_string(), port)
    } else {
        (host_port.to_string(), default_port)
    };

    Ok((host, port, path.to_string()))
}

pub(crate) fn native_dns_lookup(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Dns_lookup: expected 2 arguments (hostname, recordType)".to_string());
    }
    let hostname = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Dns_lookup: expected String hostname".to_string()),
    };
    let record_type = match &args[1] {
        Value::String(s) => s.as_str().to_uppercase(),
        _ => return Err("Dns_lookup: expected String recordType".to_string()),
    };

    use std::net::ToSocketAddrs;
    let addr_str = format!("{}:0", hostname);
    let mut records: Vec<Value> = Vec::new();
    match addr_str.to_socket_addrs() {
        Ok(iter) => {
            for addr in iter {
                let ip = addr.ip();
                let include = match record_type.as_str() {
                    "A" => ip.is_ipv4(),
                    "AAAA" => ip.is_ipv6(),
                    _ => true,
                };
                if include {
                    let mut fields = HashMap::new();
                    fields.insert("name".to_string(), Value::String(Rc::new(hostname.clone())));
                    fields.insert("recordType".to_string(), Value::String(Rc::new(record_type.clone())));
                    fields.insert("value".to_string(), Value::String(Rc::new(ip.to_string())));
                    fields.insert("ttl".to_string(), Value::Int(0));
                    records.push(Value::ClassInstance {
                        class_name: "DnsRecord".to_string(),
                        fields: Rc::new(RefCell::new(fields)),
                        vtable: HashMap::new(),
                    });
                }
            }
        }
        Err(e) => return Err(format!("Dns_lookup: failed to resolve '{}': {}", hostname, e)),
    }
    Ok(Value::Array { elements: records })
}

pub(crate) fn native_dns_reverse_lookup(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Dns_reverseLookup: expected 1 argument (ip)".to_string());
    }
    let ip = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Dns_reverseLookup: expected String ip".to_string()),
    };
    // std does not provide reverse DNS; return the IP as a fallback.
    Ok(Value::String(Rc::new(ip)))
}

pub(crate) fn native_net_get_local_port(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(Value::Int(0));
    }
    match &args[0] {
        Value::Socket(socket_rc) => {
            let socket_opt = socket_rc.borrow();
            match socket_opt.as_ref() {
                Some(stream) => {
                    match stream.local_addr() {
                        Ok(addr) => Ok(Value::Int(addr.port() as i32)),
                        Err(_) => Ok(Value::Int(0)),
                    }
                }
                None => Ok(Value::Int(0)),
            }
        }
        Value::Listener(listener_rc) => {
            let listener_opt = listener_rc.borrow();
            match listener_opt.as_ref() {
                Some(listener) => {
                    match listener.local_addr() {
                        Ok(addr) => Ok(Value::Int(addr.port() as i32)),
                        Err(_) => Ok(Value::Int(0)),
                    }
                }
                None => Ok(Value::Int(0)),
            }
        }
        _ => Ok(Value::Int(0)),
    }
}

pub(crate) fn native_net_get_local_address(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(Value::String(Rc::new("".to_string())));
    }
    match &args[0] {
        Value::Socket(socket_rc) => {
            let socket_opt = socket_rc.borrow();
            match socket_opt.as_ref() {
                Some(stream) => {
                    match stream.local_addr() {
                        Ok(addr) => Ok(Value::String(Rc::new(addr.ip().to_string()))),
                        Err(_) => Ok(Value::String(Rc::new("".to_string()))),
                    }
                }
                None => Ok(Value::String(Rc::new("".to_string()))),
            }
        }
        _ => Ok(Value::String(Rc::new("".to_string()))),
    }
}

pub(crate) fn native_net_get_remote_address(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(Value::String(Rc::new("".to_string())));
    }
    match &args[0] {
        Value::Socket(socket_rc) => {
            let socket_opt = socket_rc.borrow();
            match socket_opt.as_ref() {
                Some(stream) => {
                    match stream.peer_addr() {
                        Ok(addr) => Ok(Value::String(Rc::new(addr.ip().to_string()))),
                        Err(_) => Ok(Value::String(Rc::new("".to_string()))),
                    }
                }
                None => Ok(Value::String(Rc::new("".to_string()))),
            }
        }
        _ => Ok(Value::String(Rc::new("".to_string()))),
    }
}

pub(crate) fn native_net_set_timeout(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Net_setTimeout: expected 2 arguments (socket, ms)".to_string());
    }
    let ms = args[1].to_i64().unwrap_or(0);
    if ms < 0 {
        return Err(format!("Net_setTimeout: negative timeout {}", ms));
    }
    let ms = ms as u64;
    let timeout = if ms > 0 {
        Some(std::time::Duration::from_millis(ms))
    } else {
        None
    };
    match &args[0] {
        Value::Socket(socket_rc) => {
            let mut socket_opt = socket_rc.borrow_mut();
            if let Some(stream) = socket_opt.as_mut() {
                let _ = stream.set_read_timeout(timeout);
                let _ = stream.set_write_timeout(timeout);
            }
            Ok(Value::Void)
        }
        Value::Listener(listener_rc) => {
            let listener_opt = listener_rc.borrow();
            if let Some(listener) = listener_opt.as_ref() {
                let _ = listener.set_nonblocking(ms == 0);
            }
            Ok(Value::Void)
        }
        // Tolerate non-Socket/Listener args (e.g. uninitialized handle = -1)
        // as no-ops so setTimeout can be exercised on fresh servers/clients.
        _ => Ok(Value::Void),
    }
}

trusted_natives! {
    native_net_receive => native_net_receive_ctx,
    native_http_get => native_http_get_ctx,
    native_http_post => native_http_post_ctx,
    native_http_put => native_http_put_ctx,
    native_http_delete => native_http_delete_ctx,
    native_http_patch => native_http_patch_ctx,
    native_http_head => native_http_head_ctx,
}
