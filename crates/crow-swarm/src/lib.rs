//! CrowSwarm — configurable synthetic player ramp (scaffold).

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmPlan {
    pub target_bots: u32,
    pub ramp_seconds: u32,
    pub join_address: String,
    /// `login_storm`, `world_explorer`, or `combat_chaos` (plangem.md §6.6).
    #[serde(default)]
    pub behavior_profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmReport {
    pub attempted: u32,
    pub simulated_ok: u32,
}

pub async fn run_swarm(plan: SwarmPlan) -> SwarmReport {
    let _profile = if plan.behavior_profile.is_empty() {
        "login_storm"
    } else {
        plan.behavior_profile.as_str()
    };
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
