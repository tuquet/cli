use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, info, warn};

/// Pure Rust embedded HTTP CONNECT -> SOCKS5 bridge server.
/// Eliminates the external sing-box dependency completely.
pub struct HttpToSocks5Bridge {
    listen_port: u16,
    socks5_port: u16,
    running: Arc<AtomicBool>,
}

impl HttpToSocks5Bridge {
    pub fn new(listen_port: u16, socks5_port: u16) -> Self {
        Self {
            listen_port,
            socks5_port,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Spawns the HTTP proxy server on a background Tokio task.
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.listen_port));
        let listener = TcpListener::bind(addr).await?;
        self.running.store(true, Ordering::SeqCst);

        let socks_port = self.socks5_port;
        let is_running = self.running.clone();

        info!(
            "Embedded HTTP-to-SOCKS5 Bridge listening on 127.0.0.1:{} -> forwarding to SOCKS5 127.0.0.1:{}",
            self.listen_port, socks_port
        );

        tokio::spawn(async move {
            while is_running.load(Ordering::SeqCst) {
                match listener.accept().await {
                    Ok((client_stream, client_addr)) => {
                        tokio::spawn(async move {
                            if let Err(e) = handle_http_client(client_stream, socks_port).await {
                                debug!("Connection from {} closed: {}", client_addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        if is_running.load(Ordering::SeqCst) {
                            warn!("Accept error on HTTP proxy listener: {}", e);
                        }
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

async fn handle_http_client(
    mut client: TcpStream,
    socks_port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; 8192];
    let n = client.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }

    let req_str = String::from_utf8_lossy(&buf[..n]);
    let first_line = req_str.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    if parts.is_empty() {
        return Ok(());
    }

    let method = parts[0];
    let (target_host, target_port) = if method.eq_ignore_ascii_case("CONNECT") {
        if parts.len() < 2 {
            return Ok(());
        }
        parse_host_port(parts[1], 443)
    } else {
        // Plain HTTP: look for Host header
        let host_header = req_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("host:"))
            .and_then(|l| l.split_once(':').map(|(_, h)| h.trim()))
            .unwrap_or("127.0.0.1");
        parse_host_port(host_header, 80)
    };

    // 1. Connect to local SOCKS5 proxy (e.g. 127.0.0.1:1080)
    let socks_addr = SocketAddr::from(([127, 0, 0, 1], socks_port));
    let mut socks = match TcpStream::connect(socks_addr).await {
        Ok(s) => s,
        Err(e) => {
            let _ = client.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
            return Err(format!("Cannot connect to local SOCKS5 proxy on port {}: {}", socks_port, e).into());
        }
    };

    // 2. Perform SOCKS5 handshake (RFC 1928)
    // Send greeting: [VER=5, NMETHODS=1, METHOD=0 (No Auth)]
    socks.write_all(&[0x05, 0x01, 0x00]).await?;
    let mut greeting_resp = [0u8; 2];
    socks.read_exact(&mut greeting_resp).await?;
    if greeting_resp[0] != 0x05 || greeting_resp[1] != 0x00 {
        let _ = client.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
        return Err("SOCKS5 server rejected unauthenticated connection".into());
    }

    // 3. Send SOCKS5 CONNECT request
    let mut req = Vec::new();
    req.push(0x05); // VER
    req.push(0x01); // CMD: CONNECT
    req.push(0x00); // RSV

    if let Ok(ip) = target_host.parse::<std::net::Ipv4Addr>() {
        req.push(0x01); // ATYP: IPv4
        req.extend_from_slice(&ip.octets());
    } else {
        req.push(0x03); // ATYP: Domain name
        req.push(target_host.len() as u8);
        req.extend_from_slice(target_host.as_bytes());
    }

    req.push((target_port >> 8) as u8);
    req.push((target_port & 0xFF) as u8);

    socks.write_all(&req).await?;

    // Read SOCKS5 reply
    let mut resp_header = [0u8; 4];
    socks.read_exact(&mut resp_header).await?;
    if resp_header[0] != 0x05 || resp_header[1] != 0x00 {
        let _ = client.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
        return Err(format!("SOCKS5 connect failed with status code {}", resp_header[1]).into());
    }

    // Skip bound address in reply
    match resp_header[3] {
        0x01 => {
            let mut skip = [0u8; 4 + 2];
            socks.read_exact(&mut skip).await?;
        }
        0x03 => {
            let mut len = [0u8; 1];
            socks.read_exact(&mut len).await?;
            let mut skip = vec![0u8; len[0] as usize + 2];
            socks.read_exact(&mut skip).await?;
        }
        0x04 => {
            let mut skip = [0u8; 16 + 2];
            socks.read_exact(&mut skip).await?;
        }
        _ => {}
    }

    // 4. Respond to client and start bi-directional streaming
    if method.eq_ignore_ascii_case("CONNECT") {
        client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;
    } else {
        // Forward initial request for plain HTTP
        socks.write_all(&buf[..n]).await?;
    }

    let _ = tokio::io::copy_bidirectional(&mut client, &mut socks).await;

    Ok(())
}

fn parse_host_port(target: &str, default_port: u16) -> (String, u16) {
    let target = target.trim();
    if target.starts_with('[') {
        if let Some(close_bracket) = target.find(']') {
            let host = &target[1..close_bracket];
            let rest = &target[close_bracket + 1..];
            let port = if let Some(colon) = rest.find(':') {
                rest[colon + 1..].parse::<u16>().unwrap_or(default_port)
            } else {
                default_port
            };
            return (host.to_string(), port);
        }
    }
    if let Some(pos) = target.rfind(':') {
        let host = &target[..pos];
        let port = target[pos + 1..].parse::<u16>().unwrap_or(default_port);
        (host.to_string(), port)
    } else {
        (target.to_string(), default_port)
    }
}
