//! CrowSwarm — configurable synthetic player ramp (scaffold).

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmPlan {
    pub target_bots: u32,
    pub ramp_seconds: u32,
    pub join_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmReport {
    pub attempted: u32,
    pub simulated_ok: u32,
}

pub async fn run_swarm(plan: SwarmPlan) -> SwarmReport {
    let step = Duration::from_millis(
        (plan.ramp_seconds.max(1) as u64 * 1000) / plan.target_bots.max(1) as u64,
    );
    let mut simulated_ok = 0u32;
    for i in 0..plan.target_bots {
        tokio::time::sleep(step).await;
        simulated_ok = i + 1;
    }
    SwarmReport {
        attempted: plan.target_bots,
        simulated_ok,
    }
}
