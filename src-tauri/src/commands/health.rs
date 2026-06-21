// Health + connection commands. Errors are stringified at this IPC boundary;
// internal modules use `thiserror` enums. The AppState lock is held only to
// read/clone config — never across a network `.await`.
use chrono::{DateTime, Utc};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex;

use crate::encryption;
use crate::health::{self, ActiveMode, DaySummary, HealthError, SyncStatus, WeekSummary};
use crate::oauth;
use crate::state::AppState;

// Space-separated scopes. activity_and_fitness covers steps, distance and active
// minutes; resting HR needs health_metrics_and_measurements; sleep needs sleep.
// Adding scopes requires the user to reconnect (re-consent) before the new
// metrics return data.
const SCOPE: &str = "https://www.googleapis.com/auth/googlehealth.activity_and_fitness.readonly \
    https://www.googleapis.com/auth/googlehealth.health_metrics_and_measurements.readonly \
    https://www.googleapis.com/auth/googlehealth.sleep.readonly";

fn humanize_since(rfc3339: &str) -> Option<String> {
    let dt = DateTime::parse_from_rfc3339(rfc3339).ok()?.with_timezone(&Utc);
    let mins = (Utc::now() - dt).num_minutes().max(0);
    Some(if mins < 1 {
        "just now".to_string()
    } else if mins < 60 {
        format!("{mins} min ago")
    } else {
        format!("{}h ago", mins / 60)
    })
}

fn detail_since(rfc3339: &str) -> Option<String> {
    let dt = DateTime::parse_from_rfc3339(rfc3339).ok()?;
    Some(dt.format("%b %e at %l:%M %p").to_string())
}

fn status_from(st: &AppState) -> SyncStatus {
    let connected = st.connected();
    let (label, detail) = if st.demo {
        (Some("3 min ago".to_string()), None)
    } else if let Some(ts) = &st.settings.last_synced_at {
        (humanize_since(ts), detail_since(ts))
    } else {
        (None, None)
    };
    SyncStatus {
        state: if connected { "connected" } else { "reconnect" }.to_string(),
        connected,
        syncing: st.syncing,
        demo: st.demo,
        last_synced_label: if connected { label } else { None },
        last_synced_detail: detail,
    }
}

type Creds = (bool, reqwest::Client, Option<String>, Option<String>, Option<String>);

// Lock, decrypt the refresh token, clone what we need, drop the guard.
async fn gather(state: &State<'_, Mutex<AppState>>) -> Result<Creds, String> {
    let st = state.lock().await;
    let token = match &st.settings.google_refresh_token {
        Some(secret) => Some(encryption::decrypt(secret).map_err(|e| e.to_string())?),
        None => None,
    };
    Ok((
        st.demo,
        st.http.clone(),
        st.client_id.clone(),
        st.client_secret.clone(),
        token,
    ))
}

fn require(
    cid: Option<String>,
    csec: Option<String>,
    token: Option<String>,
) -> Result<(String, String, String), String> {
    match (cid, csec, token) {
        (Some(a), Some(b), Some(c)) => Ok((a, b, c)),
        _ => Err(HealthError::NotConnected.to_string()),
    }
}

async fn build_week(
    state: &State<'_, Mutex<AppState>>,
    active_mode: ActiveMode,
    goal: u64,
) -> Result<WeekSummary, String> {
    let (demo, http, cid, csec, token) = gather(state).await?;
    if demo {
        return Ok(health::demo::week(goal));
    }
    let (cid, csec, token) = require(cid, csec, token)?;
    health::google::fetch_week(&http, &cid, &csec, &token, active_mode, goal)
        .await
        .map_err(|e| {
            // Surface the real reason in the console: this is what otherwise turns
            // into a silent spinner in the UI.
            tracing::error!("week summary fetch failed: {e}");
            e.to_string()
        })
}

#[tauri::command]
pub async fn get_sync_status(state: State<'_, Mutex<AppState>>) -> Result<SyncStatus, String> {
    let st = state.lock().await;
    Ok(status_from(&st))
}

#[tauri::command]
pub async fn get_week_summary(
    state: State<'_, Mutex<AppState>>,
    active_mode: Option<String>,
    goal: Option<u64>,
) -> Result<WeekSummary, String> {
    build_week(
        &state,
        ActiveMode::from_opt(active_mode.as_deref()),
        health::resolve_goal(goal),
    )
    .await
}

#[tauri::command]
pub async fn get_day_summary(
    state: State<'_, Mutex<AppState>>,
    date: Option<String>,
    active_mode: Option<String>,
    goal: Option<u64>,
) -> Result<DaySummary, String> {
    let week = build_week(
        &state,
        ActiveMode::from_opt(active_mode.as_deref()),
        health::resolve_goal(goal),
    )
    .await?;
    let day = match date {
        Some(d) => week.days.into_iter().find(|x| x.date == d),
        None => week.days.into_iter().find(|x| x.is_today),
    };
    day.ok_or_else(|| "No data for the requested day.".to_string())
}

#[tauri::command]
pub async fn connect_google_health(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<SyncStatus, String> {
    let (cid, csec) = {
        let st = state.lock().await;
        match (st.client_id.clone(), st.client_secret.clone()) {
            (Some(a), Some(b)) => (a, b),
            _ => {
                return Err(
                    "Set GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET (see .env.example).".to_string(),
                )
            }
        }
    };

    let flow = oauth::start_flow(&cid, SCOPE).map_err(|e| e.to_string())?;
    app.opener()
        .open_url(flow.auth_url.clone(), None::<&str>)
        .map_err(|e| e.to_string())?;

    // Block on the loopback redirect off the IPC runtime.
    let expected = flow.state.clone();
    let listener = flow.listener;
    let code = tokio::task::spawn_blocking(move || oauth::wait_for_code(listener, &expected))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    let http = { state.lock().await.http.clone() };
    let tokens = oauth::exchange_code(&http, &cid, &csec, &code, &flow.verifier, &flow.redirect_uri)
        .await
        .map_err(|e| e.to_string())?;
    let refresh = tokens
        .refresh_token
        .ok_or_else(|| "Google did not return a refresh token — try again.".to_string())?;

    let encrypted = encryption::encrypt(&refresh).map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    {
        let mut st = state.lock().await;
        st.settings.google_refresh_token = Some(encrypted);
        st.settings.last_synced_at = Some(now);
        st.settings.save(&app).map_err(|e| e.to_string())?;
    }

    let st = state.lock().await;
    Ok(status_from(&st))
}

#[tauri::command]
pub async fn disconnect(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<SyncStatus, String> {
    let mut st = state.lock().await;
    st.settings.google_refresh_token = None;
    st.settings.save(&app).map_err(|e| e.to_string())?;
    Ok(status_from(&st))
}

/// A refresh's fresh status **and** the week it just fetched. Returning both
/// lets the frontend apply the new numbers from this single fetch instead of
/// following up with a second `get_week_summary` (which would double every
/// auto-refresh tick's API cost).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResult {
    pub status: SyncStatus,
    pub week: WeekSummary,
}

#[tauri::command]
pub async fn refresh_now(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    active_mode: Option<String>,
    goal: Option<u64>,
) -> Result<RefreshResult, String> {
    {
        state.lock().await.syncing = true;
    }
    let result = build_week(
        &state,
        ActiveMode::from_opt(active_mode.as_deref()),
        health::resolve_goal(goal),
    )
    .await;
    let now = Utc::now().to_rfc3339();

    let mut st = state.lock().await;
    st.syncing = false;
    match result {
        Ok(week) => {
            if !st.demo {
                st.settings.last_synced_at = Some(now);
                let _ = st.settings.save(&app);
            }
            Ok(RefreshResult {
                status: status_from(&st),
                week,
            })
        }
        Err(e) => Err(e),
    }
}
