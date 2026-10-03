use crate::error::{Error, Result};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};

pub async fn wait_for_tcp_port(host: &str, port: u16, max_wait: Duration) -> Result<()> {
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .map_err(|e| Error::Other(format!("invalid wait address {host}:{port}: {e}")))?;
    let deadline = tokio::time::Instant::now() + max_wait;
    let mut attempt = 0u32;
    while tokio::time::Instant::now() < deadline {
        attempt += 1;
        if timeout(Duration::from_millis(500), TcpStream::connect(addr))
            .await
            .is_ok_and(|r| r.is_ok())
        {
            return Ok(());
        }
        let backoff = Duration::from_millis(200 + (attempt.min(20) as u64) * 100);
        sleep(backoff).await;
    }
    Err(Error::Other(format!(
        "timed out waiting for {host}:{port} to accept connections after {max_wait:?}"
    )))
}
