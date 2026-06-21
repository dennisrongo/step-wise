//! Live freshness probe against the real Google Health API. IGNORED by default
//! (needs creds + a connected account), so `cargo test --all` / CI skip it.
//!
//! It answers one question empirically: when the panel looks stale, is it
//! upstream lag (Google's cloud genuinely has no newer data) or a bug in our
//! code? For TODAY it prints, side by side:
//!   1. what our app code yields  (health::google::fetch_week -> today steps)
//!   2. the raw dailyRollUp total (Google's ground-truth daily number)
//!   3. the raw intraday points   (sum, count, and the timestamp of the LATEST
//!      point vs now -> how stale Google's freshest data actually is)
//! and loops a few times so you can walk around and watch what moves.
//!
//! Run (PowerShell/bash, from src-tauri/):
//!   cargo test --test api_probe -- --ignored --nocapture
//! Walk-and-watch (longer):
//!   PROBE_ITERS=8 PROBE_INTERVAL_SECS=30 cargo test --test api_probe -- --ignored --nocapture
//!
//! Reading the output:
//!   - app today == raw rollup            -> our parse is faithful (no code bug in the total)
//!   - app today != raw rollup            -> a bug in OUR code (parse/date/timezone)
//!   - latest intraday point is hours old -> upstream lag (phone hasn't synced to cloud)
//!   - you walk, raw numbers DON'T move   -> upstream lag (cloud stale), not our client
//!   - raw intraday moves, rollup lags    -> Google's async rollup recompute (upstream, expected)

use std::path::PathBuf;

use chrono::{Datelike, Duration, Local, NaiveDate, Timelike};
use serde_json::{json, Value};

use stepwise_lib::encryption::{self, EncryptedSecret};
use stepwise_lib::health::{self, ActiveMode};
use stepwise_lib::oauth;

const API_BASE: &str = "https://health.googleapis.com/v4";

fn settings_path() -> PathBuf {
    if let Ok(p) = std::env::var("STEPWISE_SETTINGS") {
        return PathBuf::from(p);
    }
    let base = std::env::var("APPDATA").expect("APPDATA not set — pass STEPWISE_SETTINGS=<path to settings.json>");
    PathBuf::from(base).join("com.dennisrongo.stepwise").join("settings.json")
}

/// Decrypt the refresh token the app stored, using the same machine-bound key.
fn load_refresh_token() -> String {
    let path = settings_path();
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read settings {}: {e}", path.display()));
    let v: Value = serde_json::from_str(&raw).expect("parse settings.json");
    let secret: EncryptedSecret = serde_json::from_value(v["googleRefreshToken"].clone())
        .expect("settings.json has no googleRefreshToken — connect the app first");
    encryption::decrypt(&secret).expect("decrypt refresh token (same machine as the app?)")
}

fn civil(date: NaiveDate) -> Value {
    json!({
        "date": { "year": date.year(), "month": date.month(), "day": date.day() },
        "time": { "hours": 0, "minutes": 0, "seconds": 0 }
    })
}

/// Raw dailyRollUp for [day, day+1) — Google's ground-truth daily total.
async fn raw_rollup_today(http: &reqwest::Client, token: &str, today: NaiveDate) -> u64 {
    let url = format!("{API_BASE}/users/me/dataTypes/steps/dataPoints:dailyRollUp");
    let body = json!({
        "range": { "start": civil(today), "end": civil(today + Duration::days(1)) },
        "windowSizeDays": 1
    });
    let resp = http.post(url).bearer_auth(token).json(&body).send().await.expect("rollup send");
    let status = resp.status();
    let text = resp.text().await.expect("rollup body");
    let v: Value = serde_json::from_str(&text).unwrap_or_else(|_| panic!("rollup non-JSON ({status}): {text}"));
    v.get("rollupDataPoints")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(|p| p.get("steps"))
        .and_then(|s| s.get("countSum"))
        .and_then(|c| c.as_u64().or_else(|| c.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0)
}

/// Raw intraday points for today. Returns (sum, count, latest point's
/// start-of-interval as (hour, minute), and the latest point's JSON).
async fn raw_intraday_today(
    http: &reqwest::Client,
    token: &str,
    today: NaiveDate,
) -> (u64, usize, Option<(u32, u32)>, Option<Value>) {
    let tomorrow = today + Duration::days(1);
    let filter = format!(
        "steps.interval.civil_start_time >= \"{}\" AND steps.interval.civil_start_time < \"{}\"",
        today.format("%Y-%m-%d"),
        tomorrow.format("%Y-%m-%d")
    );
    let mut sum = 0u64;
    let mut count = 0usize;
    let mut latest: Option<(u32, u32)> = None;
    let mut latest_pt: Option<Value> = None;
    let mut page = String::new();
    loop {
        let mut url = url::Url::parse(&format!("{API_BASE}/users/me/dataTypes/steps/dataPoints")).unwrap();
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("filter", &filter);
            qp.append_pair("pageSize", "10000");
            if !page.is_empty() {
                qp.append_pair("pageToken", &page);
            }
        }
        let resp = http.get(url).bearer_auth(token).send().await.expect("intraday send");
        let status = resp.status();
        let text = resp.text().await.expect("intraday body");
        let v: Value = serde_json::from_str(&text)
            .unwrap_or_else(|_| panic!("intraday non-JSON ({status}): {text}"));
        if let Some(points) = v.get("dataPoints").and_then(Value::as_array) {
            for dp in points {
                let Some(steps) = dp.get("steps") else { continue };
                count += 1;
                sum += steps.get("count")
                    .and_then(|c| c.as_u64().or_else(|| c.as_str().and_then(|s| s.parse().ok())))
                    .unwrap_or(0);
                let t = steps.get("interval").and_then(|i| i.get("civilStartTime")).and_then(|c| c.get("time"));
                let h = t.and_then(|t| t.get("hours")).and_then(Value::as_u64).unwrap_or(0) as u32;
                let m = t.and_then(|t| t.get("minutes")).and_then(Value::as_u64).unwrap_or(0) as u32;
                if latest.map_or(true, |(lh, lm)| (h, m) > (lh, lm)) {
                    latest = Some((h, m));
                    latest_pt = Some(dp.clone());
                }
            }
        }
        match v.get("nextPageToken").and_then(Value::as_str) {
            Some(t) if !t.is_empty() => page = t.to_string(),
            _ => break,
        }
    }
    (sum, count, latest, latest_pt)
}

#[tokio::test]
#[ignore = "live Google Health API probe; run manually with --ignored --nocapture"]
async fn probe_live_freshness() {
    let _ = dotenvy::dotenv();
    let cid = std::env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID (see .env)");
    let csec = std::env::var("GOOGLE_CLIENT_SECRET").expect("GOOGLE_CLIENT_SECRET (see .env)");
    let refresh = load_refresh_token();
    let http = reqwest::Client::new();

    let iters: u32 = std::env::var("PROBE_ITERS").ok().and_then(|s| s.parse().ok()).unwrap_or(4);
    let interval: u64 = std::env::var("PROBE_INTERVAL_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(30);

    println!("\n=== Google Health freshness probe — {iters} samples, {interval}s apart ===");
    println!("(app today = what the panel shows; rollup = Google's daily total; intraday = raw points)\n");

    for i in 0..iters {
        let now = Local::now();
        let today = now.date_naive();

        // 1) The exact value our app would display, via our real code path.
        let app_today = match health::google::fetch_week(&http, &cid, &csec, &refresh, ActiveMode::Full, health::DEFAULT_GOAL).await {
            Ok(week) => week.days.iter().find(|d| d.is_today).map(|d| d.steps),
            Err(e) => {
                println!("[{i}] fetch_week ERROR: {e}");
                None
            }
        };

        // 2) + 3) Raw ground truth (its own fresh access token).
        let token = oauth::refresh(&http, &cid, &csec, &refresh).await.expect("oauth refresh").access_token;
        let rollup = raw_rollup_today(&http, &token, today).await;
        let (intraday_sum, n_pts, latest, latest_pt) = raw_intraday_today(&http, &token, today).await;

        let lag = latest.map(|(h, m)| {
            let now_min = now.hour() as i64 * 60 + now.minute() as i64;
            let pt_min = h as i64 * 60 + m as i64;
            (now_min - pt_min).max(0)
        });

        println!("[{i}] {} (local today {today})", now.format("%H:%M:%S"));
        println!(
            "      app today steps : {}",
            app_today.map(|s| s.to_string()).unwrap_or_else(|| "—".into())
        );
        println!("      raw dailyRollUp : {rollup}");
        println!(
            "      raw intraday    : sum={intraday_sum}  points={n_pts}  latest={}  (~{} min ago)",
            latest.map(|(h, m)| format!("{h:02}:{m:02}")).unwrap_or_else(|| "none".into()),
            lag.map(|l| l.to_string()).unwrap_or_else(|| "?".into())
        );
        if i == 0 {
            if app_today != Some(rollup) {
                println!("      ⚠ app today != rollup -> our code diverges from the API value");
            }
            if let Some(pt) = &latest_pt {
                println!("      latest intraday point: {pt}");
            }
        }
        println!();

        if i + 1 < iters {
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
        }
    }

    println!("=== interpretation ===");
    println!("• 'app today' tracking 'raw dailyRollUp' exactly -> no bug in our total.");
    println!("• 'latest' many minutes behind now, and not advancing as you walk -> UPSTREAM lag");
    println!("  (the phone hasn't pushed newer data to Google's cloud yet).");
    println!("• intraday 'sum' rising while 'dailyRollUp' lags -> Google's async rollup recompute (upstream).");
}
