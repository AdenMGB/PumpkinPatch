use crate::error::{Error, Result};
use std::net::TcpListener;

/// Returns true when `host:port` can be bound (port appears free for TCP).
pub fn is_tcp_port_available(host: &str, port: u16) -> bool {
    TcpListener::bind((host, port)).is_ok()
}

pub fn ensure_ports_available(host: &str, ports: &[u16]) -> Result<()> {
    for port in ports {
        if !is_tcp_port_available(host, *port) {
            return Err(Error::InvalidState(format!(
                "port {port} on {host} is already in use — pick another hub port or stop the conflicting process"
            )));
        }
    }
    Ok(())
}
