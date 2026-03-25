use std::net::TcpListener;

/// Picks an available TCP port for testing.
///
/// Note: There is a small race condition between picking and using,
/// but it is usually sufficient for local integration tests.
pub fn pick_unused_port() -> Option<u16> {
    TcpListener::bind("127.0.0.1:0")
        .ok()?
        .local_addr()
        .ok()?
        .port()
        .into()
}
