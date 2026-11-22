//! Parity tests for networking natives: deterministic HTTP fixtures,
//! loopback-backed TCP sockets, and the no-op paths used by the shipped
//! TcpClient/UdpSocket wrappers. No test reaches an external network.

mod wasm_common;

use wasm_common::{assert_parity, assert_parity_with_root, start_echo_server};

#[test]
fn http_native_fixture_parity() {
    assert_parity(
        r#"
class Http {}
public fn main(): void {
    io::println(Http.get("http://titrate.invalid/fixture"));
    io::println(Http.post("http://titrate.invalid/fixture", "body", "text/plain"));
    io::println(Http.put("http://titrate.invalid/fixture", "body", "text/plain"));
    io::println(Http.delete("http://titrate.invalid/fixture"));
    io::println(Http.patch("http://titrate.invalid/fixture", "body", "text/plain"));
    io::println(Http.head("http://titrate.invalid/fixture"));
    Http.setTimeout(1000);
    Http.setFollowRedirects(0);
}
"#,
    );
}

#[test]
fn socket_native_roundtrip_parity() {
    let port = start_echo_server();
    let src = format!(
        r#"
public fn main(): void {{
    let h: long = Socket.new();
    Socket.connect(h, "127.0.0.1", {port});
    Socket.setTimeout(h, 2000);
    io::println(Integer.toString(Socket.send(h, "ping")));
    io::println(Socket.recv(h, 64));
    Socket.close(h);
    io::println("socket closed");
}}
"#
    );
    assert_parity(&src);
}

#[test]
fn socket_wrapper_state_noop_parity() {
    assert_parity(
        r#"
class LocalSocket {
    private long handle;
    private bool closed;
    public fn init() { this.handle = -1; this.closed = false; }
    public fn close(): void {
        if (!this.closed) {
            Socket.close(this.handle);
            this.closed = true;
        }
    }
    public fn setTimeout(ms: int): void { Socket.setTimeout(this.handle, ms); }
    public fn isClosed(): bool { return this.closed; }
}
public fn main(): void {
    let s = new LocalSocket();
    io::println(Boolean.toString(s.isClosed()));
    s.close();
    s.close();
    s.setTimeout(1000);
    io::println(Boolean.toString(s.isClosed()));
}
"#,
    );
}

#[test]
fn tcpclient_wrapper_state_noop_parity() {
    assert_parity(
        r#"
class Net {}
class LocalTcpClient {
    private long handle;
    public fn init() { this.handle = -1; }
    public fn setHandle(h: long): void { this.handle = h; }
    public fn close(): void {
        if (this.handle >= 0) { Net.close(this.handle); this.handle = -1; }
    }
    public fn setTimeout(ms: int): void { Net.setTimeout(this.handle, ms); }
    public fn getLocalAddress(): string { return Net.getLocalAddress(this.handle); }
    public fn getRemoteAddress(): string { return Net.getRemoteAddress(this.handle); }
    public fn isConnected(): bool { return this.handle >= 0; }
}
public fn main(): void {
    let c = new LocalTcpClient();
    io::println(Boolean.toString(c.isConnected()));
    c.setHandle(1);
    c.setTimeout(2500);
    io::println(Boolean.toString(c.isConnected()));
    io::println(c.getLocalAddress());
    io::println(c.getRemoteAddress());
    c.close();
    c.close();
    io::println(Boolean.toString(c.isConnected()));
}
"#,
    );
}

#[test]
fn udpsocket_wrapper_state_noop_parity() {
    assert_parity_with_root(
        r#"
import tt::net::UdpSocket;
class LocalUdpSocket {
    private long handle;
    private bool closed;
    public fn init() { this.handle = -1; this.closed = false; }
    public fn close(): void {
        if (!this.closed) { UdpSocket.close(this.handle); this.closed = true; }
    }
    public fn setTimeout(ms: int): void { UdpSocket.setTimeout(this.handle, ms); }
    public fn isClosed(): bool { return this.closed; }
}
public fn main(): void {
    let s = new LocalUdpSocket();
    io::println(Boolean.toString(s.isClosed()));
    s.setTimeout(1500);
    s.close();
    s.close();
    io::println(Boolean.toString(s.isClosed()));
}
"#,
        "../lib",
    );
}
