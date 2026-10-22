use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct GourdRouter {
    pub forwarding_secret: String,
    pub default_backend: SocketAddr,
    pub routes: Arc<RwLock<HashMap<String, SocketAddr>>>,
}

impl GourdRouter {
    pub fn new(forwarding_secret: String, default_backend: SocketAddr) -> Self {
        Self {
            forwarding_secret,
            default_backend,
            routes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_route(&self, host: &str, backend: SocketAddr) {
        self.routes
            .write()
            .await
            .insert(host.to_lowercase(), backend);
    }

    pub async fn resolve_backend(&self, virtual_host: &str) -> SocketAddr {
        let key = virtual_host.split('\0').next().unwrap_or(virtual_host).to_lowercase();
        self.routes
            .read()
            .await
            .get(&key)
            .copied()
            .unwrap_or(self.default_backend)
    }
}
