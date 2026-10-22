use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Auto-scaling pool lifecycle (plangem.md §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendPoolState {
    WarmingUp,
    /// Registered on the proxy but not advertised to the public join balancer.
    StandbyHiddenReady,
    #[default]
    Active,
    /// Match in progress — no new player joins.
    InGameLocked,
    Draining,
}

impl BackendPoolState {
    pub fn accepts_new_players(self) -> bool {
        matches!(self, BackendPoolState::Active)
    }
}

/// How many hidden-ready backends to promote when CCU exceeds active pool capacity (plangem.md §3.3).
pub fn predictive_scale_recommendation(
    current_ccu: u32,
    active_backends: u32,
    players_per_backend: u32,
    hidden_ready: u32,
) -> u32 {
    if active_backends == 0 || players_per_backend == 0 {
        return 0;
    }
    let capacity = active_backends.saturating_mul(players_per_backend);
    if current_ccu <= capacity {
        return 0;
    }
    let deficit = current_ccu - capacity;
    let need = (deficit + players_per_backend - 1) / players_per_backend;
    need.min(hidden_ready)
}

pub fn parse_pool_state(raw: Option<&str>) -> BackendPoolState {
    let s = raw.map(str::trim).unwrap_or("").to_ascii_lowercase();
    match s.as_str() {
        "" | "active" => BackendPoolState::Active,
        "warming_up" | "warming" => BackendPoolState::WarmingUp,
        "hidden_ready" | "standby" | "hot_spare" => BackendPoolState::StandbyHiddenReady,
        "in_game_locked" | "locked" => BackendPoolState::InGameLocked,
        "draining" => BackendPoolState::Draining,
        _ => BackendPoolState::Active,
    }
}

#[derive(Debug, Clone)]
struct BackendRoute {
    addr: SocketAddr,
    state: BackendPoolState,
}

#[derive(Clone)]
pub struct GourdRouter {
    pub forwarding_secret: String,
    pub default_backend: SocketAddr,
    routes: Arc<RwLock<HashMap<String, BackendRoute>>>,
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
        self.register_route_with_state(alias, backend, BackendPoolState::Active)
            .await;
    }

    pub async fn register_route_with_state(
        &self,
        alias: &str,
        backend: SocketAddr,
        state: BackendPoolState,
    ) {
        self.routes.write().await.insert(
            alias.to_lowercase(),
            BackendRoute {
                addr: backend,
                state,
            },
        );
    }

    pub async fn set_backend_state(&self, alias: &str, state: BackendPoolState) {
        let key = alias.to_lowercase();
        let mut routes = self.routes.write().await;
        if let Some(route) = routes.get_mut(&key) {
            route.state = state;
        }
    }

    pub async fn backend_state(&self, alias: &str) -> Option<BackendPoolState> {
        self.routes
            .read()
            .await
            .get(&alias.to_lowercase())
            .map(|r| r.state)
    }

    /// Promotes up to `limit` `hidden_ready` backends to `active` (plangem.md §3.3).
    pub async fn promote_hidden_ready(&self, limit: u32) -> Vec<String> {
        if limit == 0 {
            return Vec::new();
        }
        let mut routes = self.routes.write().await;
        let mut hidden: Vec<String> = routes
            .iter()
            .filter(|(_, r)| r.state == BackendPoolState::StandbyHiddenReady)
            .map(|(alias, _)| alias.clone())
            .collect();
        hidden.sort();
        let mut promoted = Vec::new();
        for alias in hidden.into_iter().take(limit as usize) {
            if let Some(route) = routes.get_mut(&alias) {
                route.state = BackendPoolState::Active;
                promoted.push(alias);
            }
        }
        promoted
    }

    pub async fn register_forced_host(&self, host: &str, velocity_alias: &str) {
        self.forced_hosts
            .write()
            .await
            .insert(host.to_lowercase(), velocity_alias.to_lowercase());
    }

    pub async fn resolve_backend(&self, virtual_host: &str) -> SocketAddr {
        self.resolve_backend_for_join(virtual_host, true).await
    }

    /// When `for_player_join` is true, backends in draining / hidden / warming states are skipped.
    pub async fn resolve_backend_for_join(
        &self,
        virtual_host: &str,
        for_player_join: bool,
    ) -> SocketAddr {
        let key = virtual_host
            .split('\0')
            .next()
            .unwrap_or(virtual_host)
            .to_lowercase();
        let routes = self.routes.read().await;
        let forced = self.forced_hosts.read().await;

        let pick = |alias: &str| -> Option<SocketAddr> {
            routes.get(alias).and_then(|route| {
                if for_player_join && !route.state.accepts_new_players() {
                    None
                } else {
                    Some(route.addr)
                }
            })
        };

        if let Some(alias) = forced.get(&key) {
            if let Some(addr) = pick(alias) {
                return addr;
            }
        }
        if let Some(addr) = pick(&key) {
            return addr;
        }
        self.default_backend
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

    #[tokio::test]
    async fn draining_backend_skipped_for_join() {
        let lobby: SocketAddr = "127.0.0.1:25566".parse().unwrap();
        let minigame: SocketAddr = "127.0.0.1:25567".parse().unwrap();
        let router = GourdRouter::new("secret".into(), lobby);
        router
            .register_route_with_state("bedwars", minigame, BackendPoolState::Draining)
            .await;
        router.register_forced_host("bw.example.com", "bedwars").await;
        let resolved = router.resolve_backend_for_join("bw.example.com", true).await;
        assert_eq!(resolved, lobby);
    }

    #[tokio::test]
    async fn hidden_ready_skipped_for_join() {
        let lobby: SocketAddr = "127.0.0.1:25566".parse().unwrap();
        let spare: SocketAddr = "127.0.0.1:25568".parse().unwrap();
        let router = GourdRouter::new("secret".into(), lobby);
        router
            .register_route_with_state("lobby-spare", spare, BackendPoolState::StandbyHiddenReady)
            .await;
        let resolved = router.resolve_backend_for_join("lobby-spare", true).await;
        assert_eq!(resolved, lobby);
    }

    #[test]
    fn parse_pool_state_aliases() {
        assert_eq!(parse_pool_state(Some("hidden_ready")), BackendPoolState::StandbyHiddenReady);
        assert_eq!(parse_pool_state(Some("DRAINING")), BackendPoolState::Draining);
    }

    #[test]
    fn predictive_scale_caps_at_hidden_ready() {
        assert_eq!(predictive_scale_recommendation(95, 2, 40, 1), 1);
        assert_eq!(predictive_scale_recommendation(50, 2, 40, 3), 0);
        assert_eq!(predictive_scale_recommendation(200, 1, 40, 2), 2);
    }

    #[tokio::test]
    async fn promote_hidden_ready_activates_spares() {
        let lobby: SocketAddr = "127.0.0.1:25566".parse().unwrap();
        let spare: SocketAddr = "127.0.0.1:25568".parse().unwrap();
        let router = GourdRouter::new("secret".into(), lobby);
        router
            .register_route_with_state("lobby-spare", spare, BackendPoolState::StandbyHiddenReady)
            .await;
        let promoted = router.promote_hidden_ready(1).await;
        assert_eq!(promoted, vec!["lobby-spare"]);
        assert_eq!(
            router.backend_state("lobby-spare").await,
            Some(BackendPoolState::Active)
        );
        assert_eq!(router.resolve_backend_for_join("lobby-spare", true).await, spare);
    }
}
