//! Local TCP control plane for Java Velocity `pumpkinpatch-velocity-bridge` plugin.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use vineyard_protocol::RouteWireSpec;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum BridgeCommand {
    RegisterRoute { spec: RouteWireSpec },
    ListRoutes { proxy_id: uuid::Uuid },
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeResponse {
    pub ok: bool,
    pub message: String,
}

pub struct VelocityBridge {
    routes: Arc<RwLock<Vec<RouteWireSpec>>>,
}

impl VelocityBridge {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn apply(&self, spec: RouteWireSpec) {
        let mut routes = self.routes.write().await;
        routes.retain(|r| r.backend_server_id != spec.backend_server_id);
        routes.push(spec);
    }

    pub async fn list(&self, proxy_id: uuid::Uuid) -> Vec<RouteWireSpec> {
        self.routes
            .read()
            .await
            .iter()
            .filter(|r| r.proxy_id == proxy_id)
            .cloned()
            .collect()
    }

    pub async fn serve(self, bind: SocketAddr) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(bind).await?;
        let bridge = Arc::new(self);
        loop {
            let (socket, _) = listener.accept().await?;
            let bridge = bridge.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_client(socket, bridge).await {
                    eprintln!("velocity-bridge client error: {e}");
                }
            });
        }
    }
}

async fn handle_client(
    socket: TcpStream,
    bridge: Arc<VelocityBridge>,
) -> Result<(), std::io::Error> {
    let (reader, mut writer) = socket.into_split();
    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await? {
        let response = match serde_json::from_str::<BridgeCommand>(&line) {
            Ok(BridgeCommand::Ping) => BridgeResponse {
                ok: true,
                message: "pumpkinpatch-velocity-bridge".into(),
            },
            Ok(BridgeCommand::RegisterRoute { spec }) => {
                bridge.apply(spec).await;
                BridgeResponse {
                    ok: true,
                    message: "route registered".into(),
                }
            }
            Ok(BridgeCommand::ListRoutes { proxy_id }) => {
                let routes = bridge.list(proxy_id).await;
                BridgeResponse {
                    ok: true,
                    message: serde_json::to_string(&routes).unwrap_or_default(),
                }
            }
            Err(e) => BridgeResponse {
                ok: false,
                message: e.to_string(),
            },
        };
        writer
            .write_all(format!("{}\n", serde_json::to_string(&response)?).as_bytes())
            .await?;
    }
    Ok(())
}
