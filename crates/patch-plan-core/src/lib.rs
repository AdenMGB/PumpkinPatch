//! Plan-compatible analytics core for Pumpkin Patch (native Path A).

mod activity_index;
mod data_extension;
mod geolocation;
mod join_address;
mod session;

pub use activity_index::{
    active_playtime_in_weeks, classify_activity_index, ActivityGroup, ActivityIndex,
    DEFAULT_PLAYTIME_THRESHOLD_MS, ONE_WEEK_MS,
};
pub use data_extension::{ExtensionIcon, ExtensionValue, PlanDataExtension};
pub use geolocation::geolocation_from_ip;
pub use join_address::{normalize_join_address, JoinAddressOptions};
pub use session::{
    ActiveSession, AfkTracker, GameMode, WorldTimes, DEFAULT_AFK_THRESHOLD_MS,
};
