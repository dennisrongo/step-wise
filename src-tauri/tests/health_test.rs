use stepwise_lib::health;

#[test]
fn demo_week_has_seven_days_with_today_last() {
    let week = health::demo::week();
    assert_eq!(week.days.len(), 7);
    assert!(week.days.last().unwrap().is_today);
    assert!(week.days.iter().take(6).all(|d| !d.is_today));
}

#[test]
fn demo_today_matches_the_design() {
    let week = health::demo::week();
    let today = week.days.last().unwrap();
    assert_eq!(today.steps, 8_427);
    assert_eq!(today.goal, 10_000);
    // midnight → current hour (14) inclusive = 15 buckets
    assert_eq!(today.hourly.len(), 15);
}

#[test]
fn deltas_are_relative_to_the_previous_day() {
    let week = health::demo::week();
    assert!(
        week.days[0].resting_hr_delta.is_none(),
        "first day has no previous day"
    );
    assert!(
        week.days[1].resting_hr_delta.is_some(),
        "later days carry a trend vs. yesterday"
    );
}

#[test]
fn labels_are_two_letter_weekdays() {
    let week = health::demo::week();
    for day in &week.days {
        assert_eq!(day.label.chars().count(), 2, "label should be 2 letters");
    }
}
