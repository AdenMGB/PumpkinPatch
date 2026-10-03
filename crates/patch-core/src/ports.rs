use crate::error::{Error, Result};
use std::net::{SocketAddr, TcpListener};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Returns true if nothing is listening on `host:port` (bind succeeds).
pub fn is_port_free(host: &str, port: u16) -> Result<bool> {
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .map_err(|e| Error::Other(format!("invalid address {host}:{port}: {e}")))?;
    Ok(TcpListener::bind(addr).is_ok())
}

pub fn assert_ports_free(host: &str, ports: &[u16]) -> Result<()> {
    let mut busy = Vec::new();
    for port in ports {
        if !is_port_free(host, *port)? {
            busy.push(*port);
        }
    }
    if !busy.is_empty() {
        return Err(Error::InvalidState(format!(
            "ports already in use on {host}: {}. Change hub port or stop conflicting processes.",
            busy.iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    Ok(())
}

pub async fn can_connect_tcp(host: &str, port: u16, wait: Duration) -> bool {
    let addr = format!("{host}:{port}");
    timeout(wait, TcpStream::connect(&addr))
        .await
        .ok()
        .and_then(|r| r.ok())
        .is_some()
}
