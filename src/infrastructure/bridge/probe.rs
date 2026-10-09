use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// Fast, zero-overhead probe to check if a local port is actively listening (< 1ms).
pub fn probe_port(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(50)).is_ok()
}

/// Fast, asynchronous non-blocking probe to check if a local port is actively listening.
pub async fn probe_port_async(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tokio::time::timeout(Duration::from_millis(50), tokio::net::TcpStream::connect(&addr))
        .await
        .map(|res| res.is_ok())
        .unwrap_or(false)
}

/// Polls a port asynchronously with 50ms intervals until it accepts connections or reaches timeout.
pub async fn wait_for_port(port: u16, max_wait: Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < max_wait {
        if probe_port_async(port).await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}
