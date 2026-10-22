//! `stem-agent` — lightweight headless daemon for remote Vineyard nodes.

use clap::Parser;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use vineyard_bus::VineyardBus;
use vineyard_protocol::RouteWireSpec;

#[derive(Parser, Debug)]
#[command(name = "stem-agent", about = "Pumpkin Patch stem-agent (Vineyard phase-1)")]
struct Args {
    #[arg(long, default_value = "127.0.0.1:9876")]
    listen: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let bus = VineyardBus::new();
    let listener = TcpListener::bind(args.listen).await?;
    eprintln!("stem-agent listening on {} (Vineyard bus ready)", args.listen);

    loop {
        let (mut socket, peer) = listener.accept().await?;
        let bus = bus.clone();
        tokio::spawn(async move {
            let mut buf = [0u8; 512];
            let n = socket.read(&mut buf).await.unwrap_or(0);
            if n == 0 {
                return;
            }
            let req = String::from_utf8_lossy(&buf[..n]);
            let body: String = if req.starts_with("GET /health") {
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nok\n".into()
            } else if req.starts_with("GET /vineyard") {
                let routes = bus.total_route_count().await;
                let servers = bus.registered_server_count().await;
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{{\"peer\":\"{peer}\",\"routes\":{routes},\"servers\":{servers}}}\n"
                )
            } else if req.contains("POST /vineyard/route") {
                let body_start = req.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
                let json = req[body_start..].trim();
                let resp = if let Ok(spec) = serde_json::from_str::<RouteWireSpec>(json) {
                    let _ = bus.publish_route_mutation(spec).await;
                    "HTTP/1.1 204 No Content\r\n\r\n"
                } else {
                    "HTTP/1.1 400 Bad Request\r\n\r\n"
                };
                resp.to_string()
            } else {
                "HTTP/1.1 404 Not Found\r\n\r\n".into()
            };
            let _ = socket.write_all(body.as_bytes()).await;
            let _ = bus;
        });
    }
}
