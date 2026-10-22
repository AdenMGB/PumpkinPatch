//! In-memory session, AFK tracker, and world×gamemode time — Plan §2.2

use std::collections::HashMap;

pub const DEFAULT_AFK_THRESHOLD_MS: i64 = 180_000;

#[derive(Debug, Clone)]
pub struct ActiveSession {
    pub user_uuid: String,
    pub session_start_ms: i64,
    pub afk_time_ms: i64,
    pub mob_kills: u32,
    pub deaths: u32,
    pub join_address: String,
    pub world_times: WorldTimes,
}

#[derive(Debug, Clone, Default)]
pub struct WorldTimes {
    pub current_world: String,
    pub current_gamemode: GameMode,
    pub last_state_change_ms: i64,
    /// (world, gamemode) -> milliseconds
    pub durations_ms: HashMap<(String, GameMode), u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameMode {
    #[default]
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl GameMode {
    pub fn from_label(label: &str) -> Self {
        match label.to_ascii_lowercase().as_str() {
            "creative" => Self::Creative,
            "adventure" => Self::Adventure,
            "spectator" => Self::Spectator,
            _ => Self::Survival,
        }
    }
}

impl WorldTimes {
    pub fn begin(world: String, gamemode: GameMode, now_ms: i64) -> Self {
        Self {
            current_world: world,
            current_gamemode: gamemode,
            last_state_change_ms: now_ms,
            durations_ms: HashMap::new(),
        }
    }

    pub fn add_elapsed(&mut self, now_ms: i64) {
        let delta = (now_ms - self.last_state_change_ms).max(0) as u64;
        if delta == 0 {
            return;
        }
        let key = (self.current_world.clone(), self.current_gamemode);
        *self.durations_ms.entry(key).or_insert(0) += delta;
        self.last_state_change_ms = now_ms;
    }

    pub fn change_world(&mut self, new_world: String, now_ms: i64) {
        self.add_elapsed(now_ms);
        self.current_world = new_world;
    }

    pub fn change_gamemode(&mut self, new_mode: GameMode, now_ms: i64) {
        self.add_elapsed(now_ms);
        self.current_gamemode = new_mode;
    }

    pub fn finalize(&mut self, now_ms: i64) {
        self.add_elapsed(now_ms);
    }

    /// Collapse (world, gamemode) durations into per-world gamemode columns for export.
    pub fn export_slices(&self) -> Vec<(String, u64, u64, u64, u64)> {
        let mut per_world: HashMap<String, [u64; 4]> = HashMap::new();
        for ((world, mode), ms) in &self.durations_ms {
            let row = per_world.entry(world.clone()).or_insert([0, 0, 0, 0]);
            let idx = match mode {
                GameMode::Survival => 0,
                GameMode::Creative => 1,
                GameMode::Adventure => 2,
                GameMode::Spectator => 3,
            };
            row[idx] += *ms;
        }
        per_world
            .into_iter()
            .map(|(world, t)| (world, t[0], t[1], t[2], t[3]))
            .collect()
    }
}

pub struct AfkTracker {
    pub threshold_ms: i64,
    last_movement: HashMap<String, i64>,
}

impl AfkTracker {
    pub fn new(threshold_ms: i64) -> Self {
        Self {
            threshold_ms,
            last_movement: HashMap::new(),
        }
    }

    pub fn on_activity(
        &mut self,
        user_uuid: &str,
        now_ms: i64,
        ignore_afk: bool,
        session: &mut ActiveSession,
    ) {
        if ignore_afk {
            self.last_movement.insert(user_uuid.to_string(), -1);
            return;
        }
        if let Some(&last) = self.last_movement.get(user_uuid) {
            if last >= 0 {
                let delta = now_ms - last;
                if delta > self.threshold_ms {
                    session.afk_time_ms += delta;
                }
            }
        }
        self.last_movement.insert(user_uuid.to_string(), now_ms);
    }

    pub fn on_quit(&mut self, user_uuid: &str, now_ms: i64, session: &mut ActiveSession) {
        self.on_activity(user_uuid, now_ms, false, session);
        self.last_movement.remove(user_uuid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn afk_accumulates_after_threshold() {
        let mut tracker = AfkTracker::new(1_000);
        let mut session = ActiveSession {
            user_uuid: "u".into(),
            session_start_ms: 0,
            afk_time_ms: 0,
            mob_kills: 0,
            deaths: 0,
            join_address: String::new(),
            world_times: WorldTimes::begin("world".into(), GameMode::Survival, 0),
        };
        tracker.on_activity("u", 0, false, &mut session);
        tracker.on_activity("u", 5_000, false, &mut session);
        assert_eq!(session.afk_time_ms, 5_000);
    }
}
