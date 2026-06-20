// Realistic placeholder data, mirroring the design and the frontend mockData.
// Used when STEPWISE_DEMO=1 so the Connected view is fully viewable without
// wiring Google.
use chrono::{Duration, Local};

use super::{fill_deltas, label_for, DaySummary, HourBucket, WeekSummary, GOAL};

const CURRENT_HOUR: u32 = 14; // 2 PM

const BASE_HOURLY: [f64; 24] = [
    0.2, 0.1, 0.05, 0.05, 0.1, 0.4, 1.2, 2.6, 3.1, 2.2, 1.8, 2.0, 3.3, 2.6, 2.5, 2.2, 2.7, 3.5,
    3.1, 2.4, 1.8, 1.2, 0.8, 0.4,
];

struct Seed {
    steps: u64,
    hr: u32,
    sleep: u32,
    dist: f64,
    active: u32,
}

// Oldest → today.
const SEEDS: [Seed; 7] = [
    Seed { steps: 7540, hr: 60, sleep: 408, dist: 3.4, active: 35 },
    Seed { steps: 12030, hr: 56, sleep: 475, dist: 5.4, active: 64 },
    Seed { steps: 9980, hr: 57, sleep: 490, dist: 4.5, active: 51 },
    Seed { steps: 9120, hr: 59, sleep: 422, dist: 4.1, active: 47 },
    Seed { steps: 11340, hr: 57, sleep: 450, dist: 5.1, active: 58 },
    Seed { steps: 6890, hr: 61, sleep: 380, dist: 3.1, active: 28 },
    Seed { steps: 8427, hr: 58, sleep: 432, dist: 3.8, active: 42 }, // today
];

fn hourly(steps: u64, is_today: bool) -> Vec<HourBucket> {
    let range = if is_today { (CURRENT_HOUR + 1) as usize } else { 24 };
    let weights = &BASE_HOURLY[..range];
    let sum: f64 = weights.iter().sum();
    let scale = steps as f64 / sum.max(1.0);
    weights
        .iter()
        .enumerate()
        .map(|(h, w)| HourBucket {
            hour: h as u32,
            steps: (w * scale).round() as u64,
        })
        .collect()
}

pub fn week() -> WeekSummary {
    let today = Local::now().date_naive();
    let mut days: Vec<DaySummary> = SEEDS
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let is_today = i == SEEDS.len() - 1;
            let date = today - Duration::days((SEEDS.len() - 1 - i) as i64);
            DaySummary {
                date: date.format("%Y-%m-%d").to_string(),
                label: label_for(date),
                is_today,
                steps: s.steps,
                goal: GOAL,
                hourly: hourly(s.steps, is_today),
                resting_hr: Some(s.hr),
                sleep_minutes: Some(s.sleep),
                distance_mi: Some(s.dist),
                active_minutes: Some(s.active),
                resting_hr_delta: None,
                sleep_minutes_delta: None,
                distance_mi_delta: None,
                active_minutes_delta: None,
            }
        })
        .collect();
    fill_deltas(&mut days);
    WeekSummary { days }
}
