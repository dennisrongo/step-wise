// Google Health API v4 client. Steps (daily roll-up + intraday hourly) are the
// core metric. Distance and active minutes ride the same activity_and_fitness
// scope; resting HR and sleep need their own scopes (health_metrics, sleep) and
// so only populate after the user reconnects to grant them. Every metric except
// steps is best-effort: a per-metric failure degrades to `None` (shown as "—")
// and is logged — it never blanks the panel. We never fabricate a value.
use std::collections::HashMap;

use chrono::{DateTime, Datelike, Duration, FixedOffset, Local, NaiveDate, Timelike};
use serde_json::{json, Value};

use super::{fill_deltas, label_for, DaySummary, HealthError, HourBucket, WeekSummary, GOAL};
use crate::oauth;

const API_BASE: &str = "https://health.googleapis.com/v4";

// Where a user finishes linking their account to Google Health. Used as a
// fallback if the ACCOUNT_NOT_LINKED error ever omits its redirect_uri.
const DEFAULT_HEALTH_SETUP_URL: &str = "https://fitbit.google.com/auth/signup";

const METERS_PER_MILE: f64 = 1609.344;
const MM_PER_MILE: f64 = 1_609_344.0;

pub async fn fetch_week(
    http: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<WeekSummary, HealthError> {
    let token = oauth::refresh(http, client_id, client_secret, refresh_token)
        .await?
        .access_token;

    let today = Local::now().date_naive();
    let start = today - Duration::days(6);

    // Steps are required — if this fails the whole fetch fails (and the UI shows
    // the real error). Every other metric is best-effort below.
    let daily = fetch_daily_steps(http, &token, start, today).await?;

    let distance = optional("distance", fetch_daily_distance(http, &token, start, today).await);
    let active = optional(
        "active minutes",
        fetch_daily_active(http, &token, start, today).await,
    );
    let resting_hr = optional(
        "resting heart rate",
        fetch_daily_resting_hr(http, &token).await,
    );
    let sleep = optional("sleep", fetch_sleep(http, &token, start, today).await);

    let hourly_today = optional("intraday hourly steps", fetch_today_hourly(http, &token).await);
    let cur_hour = Local::now().hour();

    let mut days = Vec::with_capacity(7);
    for i in 0..7i64 {
        let date = start + Duration::days(i);
        let is_today = date == today;
        let steps = *daily.get(&date).unwrap_or(&0);
        let hourly = if is_today {
            (0..=cur_hour)
                .map(|h| HourBucket {
                    hour: h,
                    steps: *hourly_today.get(&h).unwrap_or(&0),
                })
                .collect()
        } else {
            Vec::new()
        };
        days.push(DaySummary {
            date: date.format("%Y-%m-%d").to_string(),
            label: label_for(date),
            is_today,
            steps,
            goal: GOAL,
            hourly,
            resting_hr: resting_hr.get(&date).copied(),
            sleep_minutes: sleep.get(&date).copied(),
            distance_mi: distance.get(&date).copied(),
            active_minutes: active.get(&date).copied(),
            resting_hr_delta: None,
            sleep_minutes_delta: None,
            distance_mi_delta: None,
            active_minutes_delta: None,
        });
    }
    fill_deltas(&mut days);
    Ok(WeekSummary { days })
}

/// Unwrap a best-effort metric: keep the data on success, or log and fall back
/// to an empty map. A missing scope (HR/sleep before reconnect), a transient
/// error, or an unexpected shape thus shows "—" instead of failing the panel.
fn optional<T: Default>(label: &str, result: Result<T, HealthError>) -> T {
    match result {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("{label} unavailable (reconnect if it needs a new permission): {e}");
            T::default()
        }
    }
}

fn parse_count(v: &Value) -> u64 {
    if let Some(n) = v.as_u64() {
        n
    } else if let Some(f) = v.as_f64() {
        f.max(0.0) as u64
    } else if let Some(s) = v.as_str() {
        s.parse().unwrap_or(0)
    } else {
        0
    }
}

fn parse_f64(v: &Value) -> Option<f64> {
    if let Some(f) = v.as_f64() {
        Some(f)
    } else if let Some(u) = v.as_u64() {
        Some(u as f64)
    } else if let Some(s) = v.as_str() {
        s.parse().ok()
    } else {
        None
    }
}

fn civil(date: NaiveDate) -> Value {
    json!({
        "date": { "year": date.year(), "month": date.month(), "day": date.day() },
        "time": { "hours": 0, "minutes": 0, "seconds": 0 }
    })
}

/// The civil (calendar) date a rollup data point covers.
fn point_date(p: &Value) -> Option<NaiveDate> {
    let d = p.get("civilStartTime")?.get("date")?;
    let y = d.get("year")?.as_i64()? as i32;
    let m = d.get("month")?.as_i64()? as u32;
    let day = d.get("day")?.as_i64()? as u32;
    NaiveDate::from_ymd_opt(y, m, day)
}

/// Turn an HTTP status + raw body into JSON, **reading the body before branching
/// on status**. On a non-2xx, Google's body may be HTML, plain text, or empty —
/// not JSON — so eagerly calling `.json()` would surface a misleading "error
/// decoding response body" and destroy the real message (e.g. "Health API has
/// not been used in project … or it is disabled"). Mirror `oauth::parse_token`.
fn interpret(status: reqwest::StatusCode, body: &str) -> Result<Value, HealthError> {
    if !status.is_success() {
        return Err(classify_error(status, body));
    }
    serde_json::from_str(body)
        .map_err(|e| HealthError::Api(format!("could not parse Google response: {e}")))
}

/// Map a non-2xx Google response to a typed error. The actionable
/// `ACCOUNT_NOT_LINKED` precondition (a valid grant but no Google Health profile)
/// gets its own variant so the UI can guide setup; everything else is surfaced
/// verbatim with its status code.
fn classify_error(status: reqwest::StatusCode, body: &str) -> HealthError {
    if let Ok(v) = serde_json::from_str::<Value>(body) {
        if let Some(details) = v.pointer("/error/details").and_then(Value::as_array) {
            for d in details {
                if d.get("reason").and_then(Value::as_str) == Some("ACCOUNT_NOT_LINKED") {
                    let signup_url = d
                        .pointer("/metadata/redirect_uri")
                        .and_then(Value::as_str)
                        .unwrap_or(DEFAULT_HEALTH_SETUP_URL)
                        .to_string();
                    return HealthError::AccountNotLinked { signup_url };
                }
            }
        }
    }
    let msg = body.trim();
    let msg = if msg.is_empty() { "(empty response body)" } else { msg };
    HealthError::Api(format!("HTTP {} — {}", status.as_u16(), msg))
}

async fn read_json(resp: reqwest::Response) -> Result<Value, HealthError> {
    let status = resp.status();
    let body = resp.text().await?;
    interpret(status, &body)
}

/// POST a `dailyRollUp` for one data type over [start, end] (civil days) and
/// return the parsed response. Shared by every daily metric.
async fn daily_rollup(
    http: &reqwest::Client,
    token: &str,
    data_type: &str,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Value, HealthError> {
    let url = format!("{API_BASE}/users/me/dataTypes/{data_type}/dataPoints:dailyRollUp");
    let body = json!({
        "range": { "start": civil(start), "end": civil(end + Duration::days(1)) },
        "windowSizeDays": 1
    });
    let resp = http.post(url).bearer_auth(token).json(&body).send().await?;
    let value = read_json(resp).await?;
    // Log the count + first point so a silently-empty metric (no data synced vs.
    // a field-name mismatch) can be told apart from live data.
    if tracing::enabled!(tracing::Level::DEBUG) {
        let pts = value.get("rollupDataPoints").and_then(Value::as_array);
        tracing::debug!(
            "{data_type}: {} rollup points; first = {}",
            pts.map(|a| a.len()).unwrap_or(0),
            pts.and_then(|a| a.first())
                .map(|v| v.to_string())
                .unwrap_or_else(|| "none".into())
        );
    }
    Ok(value)
}

fn rollup_points(value: &Value) -> &[Value] {
    value
        .get("rollupDataPoints")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// GET the `list` endpoint for a data type, following pagination, and collect all
/// data points. Used for types that don't support dailyRollUp (resting HR, sleep).
async fn list_points(
    http: &reqwest::Client,
    token: &str,
    data_type: &str,
    filter: &str,
    page_size: u32,
) -> Result<Vec<Value>, HealthError> {
    let mut out = Vec::new();
    let mut page = String::new();
    loop {
        let mut url = url::Url::parse(&format!("{API_BASE}/users/me/dataTypes/{data_type}/dataPoints"))
            .map_err(|e| HealthError::Api(e.to_string()))?;
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("filter", filter);
            qp.append_pair("pageSize", &page_size.to_string());
            if !page.is_empty() {
                qp.append_pair("pageToken", &page);
            }
        }
        let resp = http.get(url).bearer_auth(token).send().await?;
        let value = read_json(resp).await?;
        if let Some(points) = value.get("dataPoints").and_then(Value::as_array) {
            out.extend(points.iter().cloned());
        }
        match value.get("nextPageToken").and_then(Value::as_str) {
            Some(t) if !t.is_empty() => page = t.to_string(),
            _ => break,
        }
    }
    Ok(out)
}

/// GET the first page of the `list` endpoint with **no filter**, returning its
/// data points. Responses are ordered by time descending, so this yields the
/// most recent `page_size` points — enough for low-volume daily summaries like
/// resting HR, and it sidesteps that type's finicky filter-member path.
async fn list_recent(
    http: &reqwest::Client,
    token: &str,
    data_type: &str,
    page_size: u32,
) -> Result<Vec<Value>, HealthError> {
    let url = format!("{API_BASE}/users/me/dataTypes/{data_type}/dataPoints?pageSize={page_size}");
    let resp = http.get(url).bearer_auth(token).send().await?;
    let value = read_json(resp).await?;
    Ok(value
        .get("dataPoints")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

/// Extract a `date -> step count` map from a `dailyRollUp` response.
fn parse_daily_steps(value: &Value) -> HashMap<NaiveDate, u64> {
    let mut map = HashMap::new();
    for p in rollup_points(value) {
        if let Some(date) = point_date(p) {
            let steps = p
                .get("steps")
                .and_then(|s| s.get("countSum"))
                .map(parse_count)
                .unwrap_or(0);
            map.insert(date, steps);
        }
    }
    map
}

/// `date -> distance in miles`. The API reports `millimetersSum` (millimeters);
/// older sources may use `metersSum`/`distanceMetersSum`, so handle both units.
fn parse_daily_distance(value: &Value) -> HashMap<NaiveDate, f64> {
    let mut map = HashMap::new();
    for p in rollup_points(value) {
        let Some(date) = point_date(p) else { continue };
        let Some(dist) = p.get("distance") else { continue };
        let miles = if let Some(mm) = dist.get("millimetersSum").and_then(parse_f64) {
            mm / MM_PER_MILE
        } else if let Some(m) = dist
            .get("metersSum")
            .or_else(|| dist.get("distanceMetersSum"))
            .and_then(parse_f64)
        {
            m / METERS_PER_MILE
        } else {
            continue;
        };
        map.insert(date, (miles * 10.0).round() / 10.0);
    }
    map
}

/// `date -> active minutes`, summed across activity levels (light/moderate/
/// vigorous). Falls back to a flat `activeMinutesSum` if present.
fn parse_daily_active(value: &Value) -> HashMap<NaiveDate, u32> {
    let mut map = HashMap::new();
    for p in rollup_points(value) {
        let Some(date) = point_date(p) else { continue };
        let Some(active) = p.get("activeMinutes") else { continue };
        let by_level = active
            .get("activeMinutesRollupByActivityLevel")
            .and_then(Value::as_array)
            .map(|levels| {
                levels
                    .iter()
                    .filter_map(|l| l.get("activeMinutesSum").map(parse_count))
                    .sum::<u64>()
            })
            .unwrap_or(0);
        let total = if by_level > 0 {
            by_level
        } else {
            active.get("activeMinutesSum").map(parse_count).unwrap_or(0)
        };
        map.insert(date, total as u32);
    }
    map
}

/// `date -> resting heart rate (bpm)` from a `list` response on
/// `daily-resting-heart-rate` (which has no dailyRollUp). The bpm field name
/// isn't well documented, so try the likely direct fields, then fall back to the
/// midpoint of a personal range. Unknown shapes stay absent (shown as "—").
fn parse_resting_hr(points: &[Value]) -> HashMap<NaiveDate, u32> {
    let mut map = HashMap::new();
    for p in points {
        let Some(rhr) = p.get("dailyRestingHeartRate") else { continue };
        let Some(date) = daily_point_date(p, "dailyRestingHeartRate") else { continue };
        if let Some(bpm) = extract_bpm(rhr) {
            map.insert(date, bpm);
        }
    }
    map
}

fn extract_bpm(rhr: &Value) -> Option<u32> {
    for key in ["beatsPerMinute", "restingHeartRateBpm", "bpm", "value"] {
        if let Some(v) = rhr.get(key).and_then(parse_f64) {
            return Some(v.round() as u32);
        }
    }
    // Older/range shape: surface the midpoint of the personal range.
    let range = rhr.get("restingHeartRatePersonalRange").unwrap_or(rhr);
    let min = range.get("beatsPerMinuteMin").and_then(parse_f64);
    let max = range.get("beatsPerMinuteMax").and_then(parse_f64);
    match (min, max) {
        (Some(a), Some(b)) => Some(((a + b) / 2.0).round() as u32),
        (Some(a), None) | (None, Some(a)) => Some(a.round() as u32),
        _ => None,
    }
}

/// Date for a daily-summary `list` point: prefer `civilStartTime.date`, else the
/// member's own `date` object.
fn daily_point_date(p: &Value, member: &str) -> Option<NaiveDate> {
    point_date(p).or_else(|| {
        let d = p.get(member)?.get("date")?;
        let y = d.get("year")?.as_i64()? as i32;
        let m = d.get("month")?.as_i64()? as u32;
        let day = d.get("day")?.as_i64()? as u32;
        NaiveDate::from_ymd_opt(y, m, day)
    })
}

/// Fold one page of intraday step intervals into `map` keyed by start hour.
fn accumulate_hourly(value: &Value, map: &mut HashMap<u32, u64>) {
    if let Some(points) = value.get("dataPoints").and_then(Value::as_array) {
        for dp in points {
            if let Some(steps) = dp.get("steps") {
                let hour = steps
                    .get("interval")
                    .and_then(|i| i.get("civilStartTime"))
                    .and_then(|c| c.get("time"))
                    .and_then(|t| t.get("hours"))
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32;
                let count = steps.get("count").map(parse_count).unwrap_or(0);
                *map.entry(hour).or_insert(0) += count;
            }
        }
    }
}

/// Sum sleep minutes per night from a `list` response. Each point is a full
/// session whose `summary.minutesAsleep` is the time actually asleep (awake time
/// already excluded); attribute it to the local wake date (session end, in the
/// data's own UTC offset). Falls back to summing non-awake stage segments.
fn parse_sleep(points: &[Value]) -> HashMap<NaiveDate, u32> {
    let mut map: HashMap<NaiveDate, u32> = HashMap::new();
    for p in points {
        let Some(sleep) = p.get("sleep") else { continue };
        let minutes = sleep
            .get("summary")
            .and_then(|s| s.get("minutesAsleep"))
            .and_then(parse_f64)
            .map(|m| m.round() as u32)
            .or_else(|| asleep_minutes_from_stages(sleep));
        let Some(minutes) = minutes.filter(|m| *m > 0) else { continue };
        let Some(date) = session_wake_date(sleep) else { continue };
        *map.entry(date).or_insert(0) += minutes;
    }
    map
}

/// The local calendar date a session ends on, using the session's own UTC offset
/// so it matches the user's timezone regardless of where this app runs.
fn session_wake_date(sleep: &Value) -> Option<NaiveDate> {
    let interval = sleep.get("interval")?;
    let end = interval
        .get("endTime")
        .and_then(Value::as_str)
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())?;
    match interval
        .get("endUtcOffset")
        .and_then(Value::as_str)
        .and_then(parse_offset_seconds)
        .and_then(FixedOffset::east_opt)
    {
        Some(tz) => Some(end.with_timezone(&tz).date_naive()),
        None => Some(end.with_timezone(&Local).date_naive()),
    }
}

/// Sum non-awake stage durations — fallback when there's no `summary.minutesAsleep`.
fn asleep_minutes_from_stages(sleep: &Value) -> Option<u32> {
    let stages = sleep.get("stages").and_then(Value::as_array)?;
    let mut total = 0u32;
    for st in stages {
        let kind = st.get("type").and_then(Value::as_str).unwrap_or("");
        if kind.eq_ignore_ascii_case("AWAKE") {
            continue;
        }
        let start = st
            .get("startTime")
            .and_then(Value::as_str)
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok());
        let end = st
            .get("endTime")
            .and_then(Value::as_str)
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok());
        if let (Some(s), Some(e)) = (start, end) {
            total += (e - s).num_minutes().max(0) as u32;
        }
    }
    (total > 0).then_some(total)
}

/// Parse a Google duration-offset string like "-25200s" into seconds.
fn parse_offset_seconds(s: &str) -> Option<i32> {
    s.trim_end_matches('s').parse::<i32>().ok()
}

async fn fetch_daily_steps(
    http: &reqwest::Client,
    token: &str,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<HashMap<NaiveDate, u64>, HealthError> {
    let value = daily_rollup(http, token, "steps", start, end).await?;
    Ok(parse_daily_steps(&value))
}

async fn fetch_daily_distance(
    http: &reqwest::Client,
    token: &str,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<HashMap<NaiveDate, f64>, HealthError> {
    let value = daily_rollup(http, token, "distance", start, end).await?;
    Ok(parse_daily_distance(&value))
}

async fn fetch_daily_active(
    http: &reqwest::Client,
    token: &str,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<HashMap<NaiveDate, u32>, HealthError> {
    let value = daily_rollup(http, token, "active-minutes", start, end).await?;
    Ok(parse_daily_active(&value))
}

async fn fetch_daily_resting_hr(
    http: &reqwest::Client,
    token: &str,
) -> Result<HashMap<NaiveDate, u32>, HealthError> {
    // daily-resting-heart-rate has no dailyRollUp and a finicky filter-member
    // path, so fetch the most recent points (one per day, newest-first); that
    // covers our week and the builder ignores any out-of-range dates.
    let points = list_recent(http, token, "daily-resting-heart-rate", 14).await?;
    if let Some(first) = points.first() {
        tracing::debug!("resting-hr raw point: {first}");
    }
    Ok(parse_resting_hr(&points))
}

async fn fetch_today_hourly(
    http: &reqwest::Client,
    token: &str,
) -> Result<HashMap<u32, u64>, HealthError> {
    let today = Local::now().date_naive();
    let tomorrow = today + Duration::days(1);
    let filter = format!(
        "steps.interval.civil_start_time >= \"{}\" AND steps.interval.civil_start_time < \"{}\"",
        today.format("%Y-%m-%d"),
        tomorrow.format("%Y-%m-%d")
    );

    let mut map: HashMap<u32, u64> = HashMap::new();
    let mut page = String::new();
    loop {
        let mut url = url::Url::parse(&format!("{API_BASE}/users/me/dataTypes/steps/dataPoints"))
            .map_err(|e| HealthError::Api(e.to_string()))?;
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("filter", &filter);
            qp.append_pair("pageSize", "10000");
            if !page.is_empty() {
                qp.append_pair("pageToken", &page);
            }
        }

        let resp = http.get(url).bearer_auth(token).send().await?;
        let value = read_json(resp).await?;
        accumulate_hourly(&value, &mut map);

        match value.get("nextPageToken").and_then(Value::as_str) {
            Some(t) if !t.is_empty() => page = t.to_string(),
            _ => break,
        }
    }
    Ok(map)
}

async fn fetch_sleep(
    http: &reqwest::Client,
    token: &str,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<HashMap<NaiveDate, u32>, HealthError> {
    // Sleep filters on the session END time (start_time isn't a valid member),
    // and its page size caps at 25.
    let filter = format!(
        "sleep.interval.civil_end_time >= \"{}\" AND sleep.interval.civil_end_time < \"{}\"",
        start.format("%Y-%m-%d"),
        (end + Duration::days(1)).format("%Y-%m-%d")
    );
    let points = list_points(http, token, "sleep", &filter, 25).await?;
    // Sleep's wire shape is the least documented; log the first raw point at
    // debug so the exact fields can be confirmed against live data.
    if let Some(first) = points.first() {
        tracing::debug!("sleep raw point: {first}");
    }
    Ok(parse_sleep(&points))
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn error_status_with_json_body_surfaces_googles_message() {
        // The shape Google returns when the Health API isn't enabled on the project.
        let body = r#"{"error":{"code":403,"message":"Health API has not been used in project 1234 before or it is disabled","status":"PERMISSION_DENIED"}}"#;
        let err = interpret(StatusCode::FORBIDDEN, body).unwrap_err();
        let s = err.to_string();
        assert!(s.contains("403"), "should include the status code: {s}");
        assert!(
            s.contains("PERMISSION_DENIED") && s.contains("disabled"),
            "should preserve Google's real message: {s}"
        );
    }

    #[test]
    fn error_status_with_non_json_body_is_not_masked_as_a_decode_error() {
        // Regression for the bug behind the silent spinner: previously
        // `resp.json().await?` on a non-JSON error body produced a useless
        // "error decoding response body" and threw away this text.
        let err = interpret(StatusCode::UNAUTHORIZED, "<html>401 Unauthorized</html>").unwrap_err();
        let s = err.to_string();
        assert!(s.contains("401"), "status preserved: {s}");
        assert!(s.contains("Unauthorized"), "raw body survives: {s}");
        assert!(
            !s.to_lowercase().contains("decoding"),
            "must not collapse into a decode error: {s}"
        );
    }

    #[test]
    fn empty_error_body_still_reports_the_status() {
        let err = interpret(StatusCode::INTERNAL_SERVER_ERROR, "   ").unwrap_err();
        assert!(err.to_string().contains("500"));
    }

    #[test]
    fn account_not_linked_is_classified_as_an_actionable_error() {
        // The exact 400 Google returns when the account has no Health profile yet.
        let body = r#"{"error":{"code":400,"message":"The account is not linked to Google Health.","status":"FAILED_PRECONDITION","details":[{"@type":"type.googleapis.com/google.rpc.ErrorInfo","reason":"ACCOUNT_NOT_LINKED","domain":"health.googleapis.com","metadata":{"redirect_uri":"https://fitbit.google.com/auth/signup"}}]}}"#;
        match interpret(StatusCode::BAD_REQUEST, body).unwrap_err() {
            HealthError::AccountNotLinked { signup_url } => {
                assert_eq!(signup_url, "https://fitbit.google.com/auth/signup");
            }
            other => panic!("expected AccountNotLinked, got {other:?}"),
        }
        // Display carries the stable token the frontend keys on.
        assert!(interpret(StatusCode::BAD_REQUEST, body)
            .unwrap_err()
            .to_string()
            .contains("ACCOUNT_NOT_LINKED"));
    }

    #[test]
    fn account_not_linked_without_redirect_uri_falls_back_to_default() {
        let body = r#"{"error":{"status":"FAILED_PRECONDITION","details":[{"reason":"ACCOUNT_NOT_LINKED"}]}}"#;
        match interpret(StatusCode::BAD_REQUEST, body).unwrap_err() {
            HealthError::AccountNotLinked { signup_url } => {
                assert_eq!(signup_url, DEFAULT_HEALTH_SETUP_URL)
            }
            other => panic!("expected AccountNotLinked, got {other:?}"),
        }
    }

    #[test]
    fn other_4xx_without_that_reason_stays_a_plain_api_error() {
        let body = r#"{"error":{"code":403,"status":"PERMISSION_DENIED","details":[{"reason":"SERVICE_DISABLED"}]}}"#;
        assert!(matches!(
            interpret(StatusCode::FORBIDDEN, body).unwrap_err(),
            HealthError::Api(_)
        ));
    }

    #[test]
    fn success_with_invalid_json_is_an_api_error_not_a_panic() {
        let err = interpret(StatusCode::OK, "definitely not json").unwrap_err();
        assert!(matches!(err, HealthError::Api(_)));
    }

    #[test]
    fn parse_daily_steps_extracts_steps_by_date() {
        let v = json!({
            "rollupDataPoints": [
                { "civilStartTime": { "date": { "year": 2026, "month": 6, "day": 18 } },
                  "steps": { "countSum": 8427 } },
                // Google sometimes returns counts as strings; parse_count handles it.
                { "civilStartTime": { "date": { "year": 2026, "month": 6, "day": 19 } },
                  "steps": { "countSum": "11340" } }
            ]
        });
        let map = parse_daily_steps(&v);
        assert_eq!(map.get(&d(2026, 6, 18)), Some(&8427));
        assert_eq!(map.get(&d(2026, 6, 19)), Some(&11340));
    }

    #[test]
    fn parse_daily_steps_tolerates_a_point_missing_its_count() {
        let v = json!({ "rollupDataPoints": [
            { "civilStartTime": { "date": { "year": 2026, "month": 6, "day": 18 } } }
        ] });
        assert_eq!(parse_daily_steps(&v).get(&d(2026, 6, 18)), Some(&0));
    }

    #[test]
    fn parse_daily_steps_is_empty_when_there_are_no_points() {
        assert!(parse_daily_steps(&json!({})).is_empty());
        assert!(parse_daily_steps(&json!({ "rollupDataPoints": [] })).is_empty());
    }

    #[test]
    fn parse_distance_converts_millimeters_to_miles() {
        // The real field/unit: 9,459,200 mm = 5.88 mi → 5.9 (string value).
        let v = json!({ "rollupDataPoints": [
            { "civilStartTime": { "date": { "year": 2026, "month": 6, "day": 20 } },
              "distance": { "millimetersSum": "9459200" } }
        ] });
        assert_eq!(parse_daily_distance(&v).get(&d(2026, 6, 20)), Some(&5.9));
    }

    #[test]
    fn parse_distance_converts_meters_to_miles() {
        let v = json!({ "rollupDataPoints": [
            { "civilStartTime": { "date": { "year": 2026, "month": 6, "day": 20 } },
              "distance": { "metersSum": 8046.72 } } // exactly 5.0 miles
        ] });
        assert_eq!(parse_daily_distance(&v).get(&d(2026, 6, 20)), Some(&5.0));
    }

    #[test]
    fn parse_distance_accepts_legacy_field_name() {
        let v = json!({ "rollupDataPoints": [
            { "civilStartTime": { "date": { "year": 2026, "month": 6, "day": 20 } },
              "distance": { "distanceMetersSum": 1609.344 } } // 1.0 mile
        ] });
        assert_eq!(parse_daily_distance(&v).get(&d(2026, 6, 20)), Some(&1.0));
    }

    #[test]
    fn parse_active_sums_across_activity_levels() {
        let v = json!({ "rollupDataPoints": [
            { "civilStartTime": { "date": { "year": 2026, "month": 6, "day": 20 } },
              "activeMinutes": { "activeMinutesRollupByActivityLevel": [
                  { "activityLevel": "MODERATE", "activeMinutesSum": 25 },
                  { "activityLevel": "VIGOROUS", "activeMinutesSum": 17 }
              ] } }
        ] });
        assert_eq!(parse_daily_active(&v).get(&d(2026, 6, 20)), Some(&42));
    }

    #[test]
    fn parse_active_falls_back_to_flat_sum() {
        let v = json!({ "rollupDataPoints": [
            { "civilStartTime": { "date": { "year": 2026, "month": 6, "day": 20 } },
              "activeMinutes": { "activeMinutesSum": 51 } }
        ] });
        assert_eq!(parse_daily_active(&v).get(&d(2026, 6, 20)), Some(&51));
    }

    #[test]
    fn parse_resting_hr_reads_a_direct_bpm_value() {
        // Daily summary points carry their own `date`; no civilStartTime needed.
        let points = [json!({
            "dailyRestingHeartRate": {
                "date": { "year": 2026, "month": 6, "day": 20 },
                "beatsPerMinute": 57
            }
        })];
        assert_eq!(parse_resting_hr(&points).get(&d(2026, 6, 20)), Some(&57));
    }

    #[test]
    fn parse_resting_hr_falls_back_to_personal_range_midpoint() {
        let points = [json!({
            "civilStartTime": { "date": { "year": 2026, "month": 6, "day": 20 } },
            "dailyRestingHeartRate": {
                "restingHeartRatePersonalRange": { "beatsPerMinuteMin": 54, "beatsPerMinuteMax": 62 }
            }
        })];
        assert_eq!(parse_resting_hr(&points).get(&d(2026, 6, 20)), Some(&58));
    }

    #[test]
    fn accumulate_hourly_sums_counts_within_an_hour() {
        let v = json!({
            "dataPoints": [
                { "steps": { "interval": { "civilStartTime": { "time": { "hours": 9 } } }, "count": 120 } },
                { "steps": { "interval": { "civilStartTime": { "time": { "hours": 9 } } }, "count": 80 } },
                { "steps": { "interval": { "civilStartTime": { "time": { "hours": 14 } } }, "count": 300 } }
            ]
        });
        let mut map = HashMap::new();
        accumulate_hourly(&v, &mut map);
        assert_eq!(map.get(&9), Some(&200), "two intervals in hour 9 sum");
        assert_eq!(map.get(&14), Some(&300));
        assert_eq!(map.get(&10), None);
    }

    #[test]
    fn parse_sleep_uses_minutes_asleep_from_summary() {
        // Real shape: a session with a summary. minutesAsleep (463) excludes the
        // awake time, unlike the raw in-bed interval (469). Date pinned by the
        // session's own offset (-7h): 15:48Z → 08:48 local on Jun 20.
        let points = [json!({ "sleep": {
            "interval": {
                "startTime": "2026-06-20T07:59:00Z", "startUtcOffset": "-25200s",
                "endTime": "2026-06-20T15:48:00Z", "endUtcOffset": "-25200s"
            },
            "summary": { "minutesAsleep": "463", "minutesInSleepPeriod": "469" }
        } })];
        assert_eq!(parse_sleep(&points).get(&d(2026, 6, 20)), Some(&463));
    }

    #[test]
    fn parse_sleep_falls_back_to_summing_non_awake_stages() {
        let points = [json!({ "sleep": {
            "interval": { "endTime": "2026-06-20T11:00:00Z", "endUtcOffset": "0s" },
            "stages": [
                { "type": "LIGHT", "startTime": "2026-06-20T08:00:00Z", "endTime": "2026-06-20T09:00:00Z" },
                { "type": "AWAKE", "startTime": "2026-06-20T09:00:00Z", "endTime": "2026-06-20T09:30:00Z" },
                { "type": "DEEP",  "startTime": "2026-06-20T09:30:00Z", "endTime": "2026-06-20T10:30:00Z" }
            ]
        } })];
        // 60 (light) + 60 (deep) asleep; 30 awake excluded.
        assert_eq!(parse_sleep(&points).get(&d(2026, 6, 20)), Some(&120));
    }
}
