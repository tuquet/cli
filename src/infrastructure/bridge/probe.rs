use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// Fast, zero-overhead probe to check if a local port is actively listening (< 1ms).
pub fn probe_port(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(50)).is_ok()
}

/// Polls a port asynchronously with 50ms intervals until it accepts connections or reaches timeout.
pub async fn wait_for_port(port: u16, max_wait: Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < max_wait {
        if probe_port(port) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}
