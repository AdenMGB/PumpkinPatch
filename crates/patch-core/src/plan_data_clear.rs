//! APIv5 QueryService `subscribeDataClearEvent` subset (indepth.md §8).

use std::collections::VecDeque;
use std::sync::Mutex;

const MAX_EVENTS: usize = 512;

#[derive(Debug, Clone, serde::Serialize)]
pub struct PlanDataClearEvent {
    pub player_uuid: String,
    pub cleared_at_ms: i64,
}

static EVENTS: Mutex<VecDeque<PlanDataClearEvent>> = Mutex::new(VecDeque::new());

/// Record a GDPR / `/plan db remove` erasure for extension subscribers.
pub fn record_plan_player_data_cleared(player_uuid: &str) {
    let cleared_at_ms = chrono::Utc::now().timestamp_millis();
    let event = PlanDataClearEvent {
        player_uuid: player_uuid.to_string(),
        cleared_at_ms,
    };
    let mut guard = EVENTS.lock().expect("plan data clear lock");
    guard.push_back(event);
    while guard.len() > MAX_EVENTS {
        guard.pop_front();
    }
}

/// Events with `cleared_at_ms` strictly greater than `since_ms`.
pub fn plan_data_clear_events_since(since_ms: i64) -> Vec<PlanDataClearEvent> {
    let guard = EVENTS.lock().expect("plan data clear lock");
    guard
        .iter()
        .filter(|e| e.cleared_at_ms > since_ms)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_lists_events() {
        record_plan_player_data_cleared("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee");
        let events = plan_data_clear_events_since(0);
        assert!(events.iter().any(|e| e.player_uuid.contains("aaaa")));
    }
}
