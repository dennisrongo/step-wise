// Health data types + shared helpers. The wire format is camelCase to match
// the frontend `types.ts`.
pub mod demo;
pub mod google;

use chrono::{Datelike, NaiveDate, Weekday};
use serde::Serialize;

/// The daily step goal used when the user hasn't chosen one. The Google Health
/// API has no goal/target data type, so the goal is a local preference (like
/// theme / active mode) passed in at fetch time — 10,000 is the conventional
/// default we fall back to.
pub const DEFAULT_GOAL: u64 = 10_000;
/// Clamp bounds for a user-chosen goal: keeps the ring math (steps / goal) from
/// dividing by zero or going absurd. Must stay in sync with the frontend bounds
/// in `src/goal.ts` (which snaps + clamps before sending) — change both together.
const MIN_GOAL: u64 = 1_000;
const MAX_GOAL: u64 = 100_000;

/// Resolve the effective step goal: fall back to [`DEFAULT_GOAL`] when unset, and
/// clamp any chosen value into the supported range.
pub fn resolve_goal(goal: Option<u64>) -> u64 {
    goal.unwrap_or(DEFAULT_GOAL).clamp(MIN_GOAL, MAX_GOAL)
}

/// Which activity levels count toward the "active minutes" metric. `Full` counts
/// light + moderate + vigorous (the default); `ModerateVigorous` excludes light.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveMode {
    Full,
    ModerateVigorous,
}

impl ActiveMode {
    /// Parse the wire value the frontend sends; anything unrecognized is `Full`.
    pub fn from_opt(s: Option<&str>) -> Self {
        match s {
            Some("intense") | Some("moderate-vigorous") | Some("moderateVigorous") => {
                ActiveMode::ModerateVigorous
            }
            _ => ActiveMode::Full,
        }
    }

    /// Whether an API activity-level label counts under this mode.
    pub fn counts(&self, level: &str) -> bool {
        match self {
            ActiveMode::Full => true,
            ActiveMode::ModerateVigorous => {
                level.eq_ignore_ascii_case("MODERATE") || level.eq_ignore_ascii_case("VIGOROUS")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HourBucket {
    pub hour: u32,
    pub steps: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaySummary {
    pub date: String,
    pub label: String,
    pub is_today: bool,
    pub steps: u64,
    pub goal: u64,
    pub hourly: Vec<HourBucket>,
    pub resting_hr: Option<u32>,
    pub sleep_minutes: Option<u32>,
    pub distance_mi: Option<f64>,
    pub active_minutes: Option<u32>,
    pub resting_hr_delta: Option<i32>,
    pub sleep_minutes_delta: Option<i32>,
    pub distance_mi_delta: Option<f64>,
    pub active_minutes_delta: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekSummary {
    pub days: Vec<DaySummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub state: String, // "connected" | "reconnect"
    pub connected: bool,
    pub syncing: bool,
    pub demo: bool,
    pub last_synced_label: Option<String>,
    pub last_synced_detail: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum HealthError {
    #[error("not connected to Google Health")]
    NotConnected,
    /// The stored refresh token can't be decrypted on this machine — almost
    /// always because the machine id changed (e.g. the Windows wmic→registry
    /// switch), so the AES-256-GCM key no longer matches. The token is
    /// permanently unusable, so it's cleared and the user must reconnect once.
    /// The stable `NEEDS_RECONNECT` token lets the frontend show a guided
    /// reconnect state instead of a cryptic cipher error with a dead-end retry.
    #[error("NEEDS_RECONNECT — your saved Google sign-in couldn't be read on this device; reconnect to continue")]
    NeedsReconnect,
    /// The OAuth grant is valid, but the account has no Google Health profile
    /// yet (FAILED_PRECONDITION / ACCOUNT_NOT_LINKED). Actionable: the user must
    /// finish setup at `signup_url`. The stable `ACCOUNT_NOT_LINKED` token lets
    /// the frontend show a guided state instead of raw JSON.
    #[error("ACCOUNT_NOT_LINKED — this Google account isn't linked to Google Health yet (set it up at {signup_url})")]
    AccountNotLinked { signup_url: String },
    #[error(transparent)]
    OAuth(#[from] crate::oauth::OAuthError),
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Google Health API error: {0}")]
    Api(String),
}

/// Two-letter weekday label, matching the design (Mo/Tu/We/Th/Fr/Sa/Su).
pub fn label_for(date: NaiveDate) -> String {
    match date.weekday() {
        Weekday::Mon => "Mo",
        Weekday::Tue => "Tu",
        Weekday::Wed => "We",
        Weekday::Thu => "Th",
        Weekday::Fri => "Fr",
        Weekday::Sat => "Sa",
        Weekday::Sun => "Su",
    }
    .to_string()
}

/// Fill each day's trend deltas relative to the previous day in the slice.
pub fn fill_deltas(days: &mut [DaySummary]) {
    for i in 1..days.len() {
        let prev_hr = days[i - 1].resting_hr;
        let prev_sleep = days[i - 1].sleep_minutes;
        let prev_dist = days[i - 1].distance_mi;
        let prev_active = days[i - 1].active_minutes;

        let d = &mut days[i];
        d.resting_hr_delta = match (d.resting_hr, prev_hr) {
            (Some(c), Some(p)) => Some(c as i32 - p as i32),
            _ => None,
        };
        d.sleep_minutes_delta = match (d.sleep_minutes, prev_sleep) {
            (Some(c), Some(p)) => Some(c as i32 - p as i32),
            _ => None,
        };
        d.distance_mi_delta = match (d.distance_mi, prev_dist) {
            (Some(c), Some(p)) => Some(((c - p) * 10.0).round() / 10.0),
            _ => None,
        };
        d.active_minutes_delta = match (d.active_minutes, prev_active) {
            (Some(c), Some(p)) => Some(c as i32 - p as i32),
            _ => None,
        };
    }
}
