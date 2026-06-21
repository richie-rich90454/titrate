// Titrate Alpha 0.3 – bytecode virtual machine: SSL/TLS native functions
// Precision in every step – richie-rich90454, 2026
//
// Real SSL/TLS support using the `native-tls` crate.
// TlsConnector and TlsStream objects are stored in thread_local registries
// keyed by integer handle. Binary data is exchanged via Latin-1 encoded strings.

use super::super::super::value::Value;
use native_tls::{TlsConnector, TlsStream};
use std::cell::RefCell;
use std::collections::HashMap;
use std::net::TcpStream;
use std::rc::Rc;

/// Wraps a connected TLS stream and its underlying TCP stream so the TCP
/// stream is dropped together with the TLS layer.
struct TlsConnection {
    _tcp: TcpStream,
    stream: TlsStream<TcpStream>,
}

thread_local! {
    static CONNECTOR_REGISTRY: RefCell<HashMap<i64, TlsConnector>> = RefCell::new(HashMap::new());
    static STREAM_REGISTRY: RefCell<HashMap<i64, TlsConnection>> = RefCell::new(HashMap::new());
    static NEXT_HANDLE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
}

fn get_handle() -> i64 {
    NEXT_HANDLE.with(|h| h.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
}

fn bytes_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

fn string_to_bytes(s: &str) -> Vec<u8> {
    s.chars().map(|c| c as u8).collect()
}

pub(crate) fn native_ssl_context_new(_args: &[Value]) -> Result<Value, String> {
    let connector = TlsConnector::builder()
        .build()
        .map_err(|e| format!("Ssl_contextNew: failed to build connector: {}", e))?;

    let handle = get_handle();
    CONNECTOR_REGISTRY.with(|r| {
        r.borrow_mut().insert(handle, connector);
    });
    Ok(Value::Long(handle))
}

pub(crate) fn native_ssl_connect(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("Ssl_connect: expected 3 arguments (ctxHandle, host, port)".to_string());
    }
    let ctx_handle = args[0].to_i64().unwrap_or(0);
    let host = match &args[1] {
        Value::String(s) => s.as_str().to_string(),
        _ => return Err("Ssl_connect: expected a string host argument".to_string()),
    };
    let port = args[2].to_i64().unwrap_or(0) as u16;

    let connector = CONNECTOR_REGISTRY.with(|r| {
        let registry = r.borrow();
        registry.get(&ctx_handle).cloned()
    }).ok_or_else(|| "Ssl_connect: invalid SSL context handle".to_string())?;

    let addr = format!("{}:{}", host, port);
    let tcp = TcpStream::connect(&addr)
        .map_err(|e| format!("Ssl_connect: failed to connect to '{}': {}", addr, e))?;

    // Use a clone of the TCP stream for the TLS handshake; keep the original
    // so it is not closed prematurely. TcpStream::try_clone() duplicates the
    // file descriptor, and the underlying socket stays open until both halves
    // are dropped.
    let tls_tcp = tcp.try_clone()
        .map_err(|e| format!("Ssl_connect: failed to clone tcp stream: {}", e))?;

    let stream = connector.connect(&host, tls_tcp)
        .map_err(|e| format!("Ssl_connect: TLS handshake to '{}' failed: {}", host, e))?;

    let handle = get_handle();
    STREAM_REGISTRY.with(|r| {
        r.borrow_mut().insert(handle, TlsConnection { _tcp: tcp, stream });
    });
    Ok(Value::Long(handle))
}

pub(crate) fn native_ssl_send(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Ssl_send: expected 2 arguments (handle, data)".to_string());
    }
    let handle = args[0].to_i64().unwrap_or(0);
    let data = match &args[1] {
        Value::String(s) => string_to_bytes(s.as_str()),
        _ => return Err("Ssl_send: expected a string data argument".to_string()),
    };

    STREAM_REGISTRY.with(|r| {
        let mut registry = r.borrow_mut();
        let conn = registry.get_mut(&handle)
            .ok_or_else(|| "Ssl_send: invalid connection handle".to_string())?;
        use std::io::Write;
        conn.stream.write_all(&data)
            .map_err(|e| format!("Ssl_send: write failed: {}", e))?;
        Ok(Value::Int(data.len() as i32))
    })
}

pub(crate) fn native_ssl_recv(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("Ssl_recv: expected 2 arguments (handle, bufferSize)".to_string());
    }
    let handle = args[0].to_i64().unwrap_or(0);
    let buf_size = args[1].to_i64().unwrap_or(0) as usize;

    STREAM_REGISTRY.with(|r| {
        let mut registry = r.borrow_mut();
        let conn = registry.get_mut(&handle)
            .ok_or_else(|| "Ssl_recv: invalid connection handle".to_string())?;
        use std::io::Read;
        let mut buf = vec![0u8; buf_size];
        let n = conn.stream.read(&mut buf)
            .map_err(|e| format!("Ssl_recv: read failed: {}", e))?;
        buf.truncate(n);
        Ok(Value::String(Rc::new(bytes_to_string(&buf))))
    })
}

pub(crate) fn native_ssl_close(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Ssl_close: expected a handle argument".to_string()),
    };

    STREAM_REGISTRY.with(|r| {
        r.borrow_mut().remove(&handle);
    });
    Ok(Value::Void)
}

pub(crate) fn native_ssl_peer_certificate(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Ssl_peerCertificate: expected a handle argument".to_string()),
    };

    STREAM_REGISTRY.with(|r| {
        let mut registry = r.borrow_mut();
        let conn = registry.get_mut(&handle)
            .ok_or_else(|| "Ssl_peerCertificate: invalid connection handle".to_string())?;

        let cert = conn.stream.peer_certificate()
            .map_err(|e| format!("Ssl_peerCertificate: failed to get certificate: {}", e))?;

        match cert {
            None => Ok(Value::Null),
            Some(cert) => {
                let der = cert.to_der()
                    .map_err(|e| format!("Ssl_peerCertificate: failed to encode certificate: {}", e))?;
                Ok(Value::String(Rc::new(bytes_to_string(&der))))
            }
        }
    })
}

pub(crate) fn native_ssl_context_close(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Ssl_contextClose: expected a handle argument".to_string()),
    };

    CONNECTOR_REGISTRY.with(|r| {
        r.borrow_mut().remove(&handle);
    });
    Ok(Value::Void)
}

pub(crate) fn native_ssl_get_peer_cert_hash(args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Long(h)) => *h,
        Some(Value::Int(h)) => *h as i64,
        _ => return Err("Ssl_getPeerCertHash: expected a handle argument".to_string()),
    };

    STREAM_REGISTRY.with(|r| {
        let mut registry = r.borrow_mut();
        let conn = registry.get_mut(&handle)
            .ok_or_else(|| "Ssl_getPeerCertHash: invalid connection handle".to_string())?;

        let cert = conn.stream.peer_certificate()
            .map_err(|e| format!("Ssl_getPeerCertHash: failed to get certificate: {}", e))?;

        match cert {
            None => Ok(Value::String(Rc::new(String::new()))),
            Some(cert) => {
                let der = cert.to_der()
                    .map_err(|e| format!("Ssl_getPeerCertHash: failed to encode certificate: {}", e))?;

                // SHA-256 hash of the DER-encoded certificate.
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(&der);
                let hash = hasher.finalize();
                Ok(Value::String(Rc::new(bytes_to_string(&hash))))
            }
        }
    })
}
