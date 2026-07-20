// Titrate Alpha 0.2 – bytecode virtual machine: net natives
// Precision in every step – richie-rich90454, 2026

use super::super::super::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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
    match std::net::TcpStream::connect(&addr) {
        Ok(stream) => Ok(Value::Socket(Rc::new(RefCell::new(Some(stream))))),
        Err(e) => Err(format!("Net_connect: failed to connect to {}: {}", addr, e)),
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

pub(crate) fn native_net_receive(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Net_receive: expected 2 arguments (socket, maxBytes)".to_string());
    }
    let max_bytes = args[1].to_i64().unwrap_or(4096) as usize;
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

pub(crate) fn native_http_get(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Http_get: expected 1 argument (url)".to_string());
    }
    let url = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_get: expected String url".to_string()),
    };

    // Parse URL manually
    let url_str = url.to_string();
    let (host, port, path) = parse_http_url(&url_str)?;

    let addr = format!("{}:{}", host, port);
    let mut stream = std::net::TcpStream::connect(&addr)
        .map_err(|e| format!("Http_get: connection failed: {}", e))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))
        .map_err(|e| format!("Http_get: {}", e))?;

    let request = format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", path, host);
    use std::io::Write;
    stream.write_all(request.as_bytes())
        .map_err(|e| format!("Http_get: write failed: {}", e))?;

    use std::io::Read;
    let mut response = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(e) => {
                if response.is_empty() {
                    return Err(format!("Http_get: read failed: {}", e));
                }
                break;
            }
        }
    }

    let response_str = String::from_utf8_lossy(&response).to_string();
    // Strip HTTP headers
    if let Some(idx) = response_str.find("\r\n\r\n") {
        Ok(Value::String(Rc::new(response_str[idx + 4..].to_string())))
    } else {
        Ok(Value::String(Rc::new(response_str)))
    }
}

pub(crate) fn native_http_post(args: &[Value]) -> Result<Value, String> {
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

    let url_str = url.to_string();
    let (host, port, path) = parse_http_url(&url_str)?;

    let addr = format!("{}:{}", host, port);
    let mut stream = std::net::TcpStream::connect(&addr)
        .map_err(|e| format!("Http_post: connection failed: {}", e))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))
        .map_err(|e| format!("Http_post: {}", e))?;

    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path, host, content_type, body.len(), body
    );
    use std::io::Write;
    stream.write_all(request.as_bytes())
        .map_err(|e| format!("Http_post: write failed: {}", e))?;

    use std::io::Read;
    let mut response = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(e) => {
                if response.is_empty() {
                    return Err(format!("Http_post: read failed: {}", e));
                }
                break;
            }
        }
    }

    let response_str = String::from_utf8_lossy(&response).to_string();
    if let Some(idx) = response_str.find("\r\n\r\n") {
        Ok(Value::String(Rc::new(response_str[idx + 4..].to_string())))
    } else {
        Ok(Value::String(Rc::new(response_str)))
    }
}

pub(crate) fn native_http_put(args: &[Value]) -> Result<Value, String> {
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

    let url_str = url.to_string();
    let (host, port, path) = parse_http_url(&url_str)?;

    let addr = format!("{}:{}", host, port);
    let mut stream = std::net::TcpStream::connect(&addr)
        .map_err(|e| format!("Http_put: connection failed: {}", e))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))
        .map_err(|e| format!("Http_put: {}", e))?;

    let request = format!(
        "PUT {} HTTP/1.1\r\nHost: {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path, host, content_type, body.len(), body
    );
    use std::io::Write;
    stream.write_all(request.as_bytes())
        .map_err(|e| format!("Http_put: write failed: {}", e))?;

    use std::io::Read;
    let mut response = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(e) => {
                if response.is_empty() {
                    return Err(format!("Http_put: read failed: {}", e));
                }
                break;
            }
        }
    }

    let response_str = String::from_utf8_lossy(&response).to_string();
    if let Some(idx) = response_str.find("\r\n\r\n") {
        Ok(Value::String(Rc::new(response_str[idx + 4..].to_string())))
    } else {
        Ok(Value::String(Rc::new(response_str)))
    }
}

pub(crate) fn native_http_delete(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Http_delete: expected 1 argument (url)".to_string());
    }
    let url = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_delete: expected String url".to_string()),
    };

    let url_str = url.to_string();
    let (host, port, path) = parse_http_url(&url_str)?;

    let addr = format!("{}:{}", host, port);
    let mut stream = std::net::TcpStream::connect(&addr)
        .map_err(|e| format!("Http_delete: connection failed: {}", e))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))
        .map_err(|e| format!("Http_delete: {}", e))?;

    let request = format!("DELETE {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", path, host);
    use std::io::Write;
    stream.write_all(request.as_bytes())
        .map_err(|e| format!("Http_delete: write failed: {}", e))?;

    use std::io::Read;
    let mut response = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(e) => {
                if response.is_empty() {
                    return Err(format!("Http_delete: read failed: {}", e));
                }
                break;
            }
        }
    }

    let response_str = String::from_utf8_lossy(&response).to_string();
    if let Some(idx) = response_str.find("\r\n\r\n") {
        Ok(Value::String(Rc::new(response_str[idx + 4..].to_string())))
    } else {
        Ok(Value::String(Rc::new(response_str)))
    }
}

pub(crate) fn native_http_patch(args: &[Value]) -> Result<Value, String> {
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

    let url_str = url.to_string();
    let (host, port, path) = parse_http_url(&url_str)?;

    let addr = format!("{}:{}", host, port);
    let mut stream = std::net::TcpStream::connect(&addr)
        .map_err(|e| format!("Http_patch: connection failed: {}", e))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))
        .map_err(|e| format!("Http_patch: {}", e))?;

    let request = format!(
        "PATCH {} HTTP/1.1\r\nHost: {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path, host, content_type, body.len(), body
    );
    use std::io::Write;
    stream.write_all(request.as_bytes())
        .map_err(|e| format!("Http_patch: write failed: {}", e))?;

    use std::io::Read;
    let mut response = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(e) => {
                if response.is_empty() {
                    return Err(format!("Http_patch: read failed: {}", e));
                }
                break;
            }
        }
    }

    let response_str = String::from_utf8_lossy(&response).to_string();
    if let Some(idx) = response_str.find("\r\n\r\n") {
        Ok(Value::String(Rc::new(response_str[idx + 4..].to_string())))
    } else {
        Ok(Value::String(Rc::new(response_str)))
    }
}

pub(crate) fn native_http_head(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Http_head: expected 1 argument (url)".to_string());
    }
    let url = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Http_head: expected String url".to_string()),
    };

    let url_str = url.to_string();
    let (host, port, path) = parse_http_url(&url_str)?;

    let addr = format!("{}:{}", host, port);
    let mut stream = std::net::TcpStream::connect(&addr)
        .map_err(|e| format!("Http_head: connection failed: {}", e))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))
        .map_err(|e| format!("Http_head: {}", e))?;

    let request = format!("HEAD {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", path, host);
    use std::io::Write;
    stream.write_all(request.as_bytes())
        .map_err(|e| format!("Http_head: write failed: {}", e))?;

    use std::io::Read;
    let mut response = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(e) => {
                if response.is_empty() {
                    return Err(format!("Http_head: read failed: {}", e));
                }
                break;
            }
        }
    }

    let response_str = String::from_utf8_lossy(&response).to_string();
    if let Some(idx) = response_str.find("\r\n\r\n") {
        Ok(Value::String(Rc::new(response_str[idx + 4..].to_string())))
    } else {
        Ok(Value::String(Rc::new(response_str)))
    }
}

pub(crate) fn native_http_set_timeout(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    // Stub: timeout is stored in the HttpClient .tr object and applied per-request
    Ok(Value::Void)
}

pub(crate) fn native_http_set_follow_redirects(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    // Stub: redirect policy is stored in the HttpClient .tr object
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
    let ms = args[1].to_i64().unwrap_or(0) as u64;
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
