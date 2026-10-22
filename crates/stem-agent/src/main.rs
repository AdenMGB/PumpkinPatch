//! `stem-agent` — lightweight headless daemon for remote Vineyard nodes.

mod port_pool;

use clap::Parser;
use gourd_proxy::predictive_scale_recommendation;
use port_pool::PortPool;
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use vineyard_bus::VineyardBus;
use vineyard_protocol::RouteWireSpec;

#[derive(Deserialize)]
struct ScaleRecommendBody {
    current_ccu: u32,
    active_backends: u32,
    players_per_backend: u32,
    hidden_ready: u32,
}

#[derive(Parser, Debug)]
#[command(name = "stem-agent", about = "Pumpkin Patch stem-agent (Vineyard phase-1)")]
struct Args {
    #[arg(long, default_value = "127.0.0.1:9876")]
    listen: SocketAddr,
    #[arg(long, default_value_t = 25600)]
    port_min: u16,
    #[arg(long, default_value_t = 25700)]
    port_max: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let bus = VineyardBus::new();
    let ports = PortPool::new(args.port_min, args.port_max);
    let listener = TcpListener::bind(args.listen).await?;
    eprintln!(
        "stem-agent listening on {} (Vineyard bus ready, port pool {}-{})",
        args.listen,
        args.port_min,
        args.port_max
    );

    loop {
        let (mut socket, peer) = listener.accept().await?;
        let bus = bus.clone();
        let ports = ports.clone();
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
            } else if req.starts_with("POST /allocate-port") {
                let port = ports.allocate().unwrap_or(0);
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{{\"port\":{port}}}\n"
                )
            } else if req.contains("POST /scale/recommend") {
                let body_start = req.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
                let json = req[body_start..].trim();
                let body = if let Ok(body) = serde_json::from_str::<ScaleRecommendBody>(json) {
                    let promote = predictive_scale_recommendation(
                        body.current_ccu,
                        body.active_backends,
                        body.players_per_backend,
                        body.hidden_ready,
                    );
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{{\"promote\":{promote}}}\n"
                    )
                } else {
                    "HTTP/1.1 400 Bad Request\r\n\r\n".to_string()
                };
                body
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
