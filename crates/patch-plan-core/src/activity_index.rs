//! Player Activity Index (PAI) — Plan §3.1

use std::f64::consts::FRAC_PI_2;

pub const DEFAULT_PLAYTIME_THRESHOLD_MS: f64 = 1_800_000.0; // 30 minutes
pub const ONE_WEEK_MS: i64 = 604_800_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActivityIndex {
    pub value: f64,
    pub group: ActivityGroup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityGroup {
    VeryActive,
    Active,
    Regular,
    Irregular,
    Inactive,
}

impl ActivityGroup {
    pub fn label(self) -> &'static str {
        match self {
            Self::VeryActive => "Very Active",
            Self::Active => "Active",
            Self::Regular => "Regular",
            Self::Irregular => "Irregular",
            Self::Inactive => "Inactive",
        }
    }

    pub fn color_hex(self) -> &'static str {
        match self {
            Self::VeryActive => "#2eab6f",
            Self::Active => "#82b346",
            Self::Regular => "#b5b336",
            Self::Irregular => "#d98b2b",
            Self::Inactive => "#777777",
        }
    }
}

impl ActivityIndex {
    pub fn calculate(
        playtime_w1_ms: u64,
        playtime_w2_ms: u64,
        playtime_w3_ms: u64,
        threshold_ms: f64,
    ) -> Self {
        let threshold_ms = if threshold_ms > 0.0 {
            threshold_ms
        } else {
            DEFAULT_PLAYTIME_THRESHOLD_MS
        };

        let calc_week = |playtime: u64| -> f64 {
            1.0 - (1.0 / (FRAC_PI_2 * (playtime as f64 / threshold_ms) + 1.0))
        };

        let y1 = calc_week(playtime_w1_ms);
        let y2 = calc_week(playtime_w2_ms);
        let y3 = calc_week(playtime_w3_ms);

        let value = 5.0 * ((3.0 * y1 + 2.0 * y2 + 1.0 * y3) / 6.0);
        let group = classify_activity_index(value);
        Self { value, group }
    }
}

pub fn classify_activity_index(value: f64) -> ActivityGroup {
    match value {
        v if v >= 3.75 => ActivityGroup::VeryActive,
        v if v >= 3.0 => ActivityGroup::Active,
        v if v >= 2.0 => ActivityGroup::Regular,
        v if v >= 1.0 => ActivityGroup::Irregular,
        _ => ActivityGroup::Inactive,
    }
}

/// Sum active playtime (session length minus AFK) per Plan week window.
pub fn active_playtime_in_weeks(
    sessions: &[(i64, i64, u64)],
    now_ms: i64,
) -> (u64, u64, u64) {
    let w1_start = now_ms - ONE_WEEK_MS;
    let w2_start = now_ms - 2 * ONE_WEEK_MS;
    let w3_start = now_ms - 3 * ONE_WEEK_MS;

    let mut p1 = 0u64;
    let mut p2 = 0u64;
    let mut p3 = 0u64;

    for &(start, end, afk_ms) in sessions {
        if end <= start {
            continue;
        }
        let overlap = |range_start: i64, range_end: i64| -> u64 {
            let s = start.max(range_start);
            let e = end.min(range_end);
            if e <= s {
                return 0;
            }
            let span = (e - s) as u64;
            let total = (end - start) as u64;
            if total == 0 {
                return 0;
            }
            let afk_share = (afk_ms * span) / total;
            span.saturating_sub(afk_share)
        };

        p1 += overlap(w1_start, now_ms);
        p2 += overlap(w2_start, w1_start);
        p3 += overlap(w3_start, w2_start);
    }

    (p1, p2, p3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_playtime_is_inactive() {
        let ai = ActivityIndex::calculate(0, 0, 0, DEFAULT_PLAYTIME_THRESHOLD_MS);
        assert_eq!(ai.value, 0.0);
        assert_eq!(ai.group, ActivityGroup::Inactive);
    }

    #[test]
    fn threshold_week_hits_expected_ratio() {
        let t = DEFAULT_PLAYTIME_THRESHOLD_MS as u64;
        let ai = ActivityIndex::calculate(t, 0, 0, DEFAULT_PLAYTIME_THRESHOLD_MS);
        let expected_y = 1.0 - (1.0 / (FRAC_PI_2 + 1.0));
        let expected = 5.0 * (3.0 * expected_y / 6.0);
        assert!((ai.value - expected).abs() < 0.001);
    }

    #[test]
    fn week_overlap_counts_active_time() {
        let now = 3 * ONE_WEEK_MS;
        let sessions = vec![(now - 86_400_000, now - 43_200_000, 0)];
        let (w1, w2, w3) = active_playtime_in_weeks(&sessions, now);
        assert!(w1 > 0);
        assert_eq!(w2, 0);
        assert_eq!(w3, 0);
    }
}
