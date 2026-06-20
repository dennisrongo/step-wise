// Google Health API v4 client. Ported from health-steps.mjs (daily roll-up) and
// live-steps.mjs (intraday step intervals → hourly shape). Steps are fully
// wired; resting HR / sleep / distance / active need additional scopes and are
// left as honest `None` for now.
use std::collections::HashMap;

use chrono::{Datelike, Duration, Local, NaiveDate, Timelike};
use serde_json::{json, Value};

use super::{fill_deltas, label_for, DaySummary, HealthError, HourBucket, WeekSummary, GOAL};
use crate::oauth;

const API_BASE: &str = "https://health.googleapis.com/v4";

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

    let daily = fetch_daily_steps(http, &token, start, today).await?;
    let hourly_today = fetch_today_hourly(http, &token).await.unwrap_or_default();
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
            resting_hr: None,
            sleep_minutes: None,
            distance_mi: None,
            active_minutes: None,
            resting_hr_delta: None,
            sleep_minutes_delta: None,
            distance_mi_delta: None,
            active_minutes_delta: None,
        });
    }
    fill_deltas(&mut days);
    Ok(WeekSummary { days })
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

fn civil(date: NaiveDate) -> Value {
    json!({
        "date": { "year": date.year(), "month": date.month(), "day": date.day() },
        "time": { "hours": 0, "minutes": 0, "seconds": 0 }
    })
}

async fn fetch_daily_steps(
    http: &reqwest::Client,
    token: &str,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<HashMap<NaiveDate, u64>, HealthError> {
    let url = format!("{API_BASE}/users/me/dataTypes/steps/dataPoints:dailyRollUp");
    let body = json!({
        "range": { "start": civil(start), "end": civil(end + Duration::days(1)) },
        "windowSizeDays": 1
    });

    let resp = http.post(url).bearer_auth(token).json(&body).send().await?;
    let status = resp.status();
    let value: Value = resp.json().await?;
    if !status.is_success() {
        return Err(HealthError::Api(value.to_string()));
    }

    let mut map = HashMap::new();
    if let Some(points) = value.get("rollupDataPoints").and_then(Value::as_array) {
        for p in points {
            let date = p
                .get("civilStartTime")
                .and_then(|c| c.get("date"))
                .and_then(|d| {
                    let y = d.get("year")?.as_i64()? as i32;
                    let m = d.get("month")?.as_i64()? as u32;
                    let day = d.get("day")?.as_i64()? as u32;
                    NaiveDate::from_ymd_opt(y, m, day)
                });
            if let Some(date) = date {
                let steps = p
                    .get("steps")
                    .and_then(|s| s.get("countSum"))
                    .map(parse_count)
                    .unwrap_or(0);
                map.insert(date, steps);
            }
        }
    }
    Ok(map)
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
        let status = resp.status();
        let value: Value = resp.json().await?;
        if !status.is_success() {
            return Err(HealthError::Api(value.to_string()));
        }

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

        match value.get("nextPageToken").and_then(Value::as_str) {
            Some(t) if !t.is_empty() => page = t.to_string(),
            _ => break,
        }
    }
    Ok(map)
}
