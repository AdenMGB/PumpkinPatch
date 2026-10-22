use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct GourdRouter {
    pub forwarding_secret: String,
    pub default_backend: SocketAddr,
    routes: Arc<RwLock<HashMap<String, SocketAddr>>>,
    /// Forced host header → velocity server alias (same semantics as Velocity `forced-hosts`).
    forced_hosts: Arc<RwLock<HashMap<String, String>>>,
}

impl GourdRouter {
    pub fn new(forwarding_secret: String, default_backend: SocketAddr) -> Self {
        Self {
            forwarding_secret,
            default_backend,
            routes: Arc::new(RwLock::new(HashMap::new())),
            forced_hosts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_route(&self, alias: &str, backend: SocketAddr) {
        self.routes
            .write()
            .await
            .insert(alias.to_lowercase(), backend);
    }

    pub async fn register_forced_host(&self, host: &str, velocity_alias: &str) {
        self.forced_hosts
            .write()
            .await
            .insert(host.to_lowercase(), velocity_alias.to_lowercase());
    }

    pub async fn resolve_backend(&self, virtual_host: &str) -> SocketAddr {
        let key = virtual_host
            .split('\0')
            .next()
            .unwrap_or(virtual_host)
            .to_lowercase();
        let routes = self.routes.read().await;
        let forced = self.forced_hosts.read().await;
        if let Some(alias) = forced.get(&key) {
            if let Some(addr) = routes.get(alias) {
                return *addr;
            }
        }
        routes.get(&key).copied().unwrap_or(self.default_backend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn forced_host_resolves_via_alias() {
        let lobby: SocketAddr = "127.0.0.1:25566".parse().unwrap();
        let survival: SocketAddr = "127.0.0.1:25567".parse().unwrap();
        let router = GourdRouter::new("secret".into(), lobby);
        router.register_route("survival", survival).await;
        router.register_forced_host("play.example.com", "survival").await;
        let resolved = router.resolve_backend("play.example.com").await;
        assert_eq!(resolved, survival);
    }
}
