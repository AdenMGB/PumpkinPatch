//! Plan §5.1 reduced refresh barrier — 15s JSON response cache for expensive aggregations.

use crate::error::{Error, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Plan "Reduced Refresh Barrier" TTL.
pub const REFRESH_BARRIER_MS: u64 = 15_000;

pub struct PlanJsonCache {
    entries: RwLock<HashMap<String, CachedJson>>,
}

struct CachedJson {
    created: Instant,
    body: String,
}

impl PlanJsonCache {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            entries: RwLock::new(HashMap::new()),
        })
    }

    pub fn cache_key(endpoint: &str, network_id: &str, server_uuid: Option<&str>, extra: &str) -> String {
        format!(
            "{}|{}|{}|{}",
            endpoint,
            network_id,
            server_uuid.unwrap_or("-"),
            extra
        )
    }

    pub async fn get_json(&self, key: &str) -> Option<String> {
        let ttl = Duration::from_millis(REFRESH_BARRIER_MS);
        let guard = self.entries.read().await;
        guard.get(key).and_then(|entry| {
            if entry.created.elapsed() < ttl {
                Some(entry.body.clone())
            } else {
                None
            }
        })
    }

    pub async fn clear(&self) {
        self.entries.write().await.clear();
    }

    pub async fn set_json(&self, key: String, body: String) {
        let ttl = Duration::from_millis(REFRESH_BARRIER_MS);
        let mut guard = self.entries.write().await;
        guard.retain(|_, e| e.created.elapsed() < ttl);
        guard.insert(
            key,
            CachedJson {
                created: Instant::now(),
                body,
            },
        );
    }
}

pub async fn with_plan_cache<T, F, Fut>(
    cache: &PlanJsonCache,
    key: String,
    bypass: bool,
    compute: F,
) -> Result<T>
where
    T: Serialize + DeserializeOwned,
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    if !bypass {
        if let Some(hit) = cache.get_json(&key).await {
            return serde_json::from_str(&hit).map_err(Error::from);
        }
    }
    let value = compute().await?;
    let json = serde_json::to_string(&value)?;
    cache.set_json(key, json).await;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_cached_value_within_ttl() {
        let cache = PlanJsonCache::new();
        cache.set_json("k".into(), "\"hello\"".into()).await;
        assert_eq!(cache.get_json("k").await.as_deref(), Some("\"hello\""));
    }
}
