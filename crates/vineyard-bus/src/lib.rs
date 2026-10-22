//! Local Vineyard bus: broadcast telemetry frames and in-memory routing CRDT (phase 1).

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;
use vineyard_protocol::{pack_frame, RouteWireSpec, TickTelemetryFrame};

#[derive(Debug, thiserror::Error)]
pub enum VineyardError {
    #[error("no telemetry subscribers")]
    NoSubscribers,
}

#[derive(Clone)]
pub struct VineyardBus {
    telemetry: broadcast::Sender<Vec<u8>>,
    routes: Arc<RwLock<HashMap<Uuid, Vec<RouteWireSpec>>>>,
    server_aliases: Arc<RwLock<HashMap<Uuid, String>>>,
}

impl VineyardBus {
    pub fn new() -> Self {
        let (telemetry, _) = broadcast::channel(256);
        Self {
            telemetry,
            routes: Arc::new(RwLock::new(HashMap::new())),
            server_aliases: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_server(&self, server_id: Uuid, alias: String) {
        self.server_aliases.write().await.insert(server_id, alias);
    }

    pub fn publish_frame(&self, frame: &TickTelemetryFrame) -> Result<(), VineyardError> {
        let bytes = pack_frame(frame);
        self.publish_raw(bytes)
    }

    pub fn publish_raw(&self, bytes: Vec<u8>) -> Result<(), VineyardError> {
        self.telemetry
            .send(bytes)
            .map_err(|_| VineyardError::NoSubscribers)?;
        Ok(())
    }

    pub fn subscribe_frames(&self) -> broadcast::Receiver<Vec<u8>> {
        self.telemetry.subscribe()
    }

    pub async fn subscribe_server_frames(
        &self,
        server_id: Uuid,
    ) -> broadcast::Receiver<Vec<u8>> {
        if !self.server_aliases.read().await.contains_key(&server_id) {
            self.server_aliases
                .write()
                .await
                .insert(server_id, server_id.to_string());
        }
        self.telemetry.subscribe()
    }

    pub async fn publish_route_mutation(&self, spec: RouteWireSpec) -> Result<(), VineyardError> {
        let mut routes = self.routes.write().await;
        routes
            .entry(spec.proxy_id)
            .or_default()
            .retain(|r| r.backend_server_id != spec.backend_server_id);
        routes.entry(spec.proxy_id).or_default().push(spec);
        Ok(())
    }

    pub async fn list_routes(&self, proxy_id: Uuid) -> Vec<RouteWireSpec> {
        self.routes
            .read()
            .await
            .get(&proxy_id)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn total_route_count(&self) -> usize {
        self.routes
            .read()
            .await
            .values()
            .map(|v| v.len())
            .sum()
    }

    pub async fn registered_server_count(&self) -> usize {
        self.server_aliases.read().await.len()
    }
}

impl Default for VineyardBus {
    fn default() -> Self {
        Self::new()
    }
}
