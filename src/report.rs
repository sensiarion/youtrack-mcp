use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use serde_json::{json, Value};

use crate::error::{AppError, Result};
use crate::youtrack::YouTrack;

const EXPECTED_MIN: i64 = 480;
const PRE_HOLIDAY_NUM: i64 = 7;
const PRE_HOLIDAY_DEN: i64 = 8;

pub async fn workitems_report(
    yt: &Arc<YouTrack>,
    author: Option<&str>,
    start: &str,
    end: &str,
) -> Result<Value> {
    let start_d = NaiveDate::parse_from_str(start, "%Y-%m-%d")
        .map_err(|_| AppError::Bad(format!("bad startDate '{start}'")))?;
    let end_d = NaiveDate::parse_from_str(end, "%Y-%m-%d")
        .map_err(|_| AppError::Bad(format!("bad endDate '{end}'")))?;
    if end_d < start_d {
        return Err(AppError::Bad("endDate before startDate".into()));
    }

    let items = yt
        .workitems_list(author, Some(start), Some(end), None, 1000, 0)
        .await?;
    let empty = vec![];
    let arr = items.as_array().unwrap_or(&empty);

    let mut by_date: HashMap<String, i64> = HashMap::new();
    let mut total_minutes = 0i64;
    for w in arr {
        let mins = w
            .get("duration")
            .and_then(|d| d.get("minutes"))
            .and_then(|x| x.as_i64())
            .unwrap_or(0);
        let ms = w.get("date").and_then(|x| x.as_i64()).unwrap_or(0);
        let iso = yt.epoch_ms_to_iso(ms);
        *by_date.entry(iso).or_insert(0) += mins;
        total_minutes += mins;
    }

    let mut days = vec![];
    let mut invalid = vec![];
    let mut total_expected = 0i64;
    let mut work_days = 0i64;

    let mut d = start_d;
    while d <= end_d {
        let iso = d.format("%Y-%m-%d").to_string();
        let is_weekend = matches!(d.weekday(), Weekday::Sat | Weekday::Sun);
        if is_weekend || yt.cfg.holidays.contains(&d) {
            d += Duration::days(1);
            continue;
        }
        let expected = if yt.cfg.pre_holidays.contains(&d) {
            EXPECTED_MIN * PRE_HOLIDAY_NUM / PRE_HOLIDAY_DEN
        } else {
            EXPECTED_MIN
        };
        let actual = by_date.get(&iso).copied().unwrap_or(0);
        let diff = actual - expected;
        let percent = if expected == 0 {
            0.0
        } else {
            (actual as f64 / expected as f64 * 1000.0).round() / 10.0
        };
        let entry = json!({
            "date": iso,
            "expected": expected,
            "actual": actual,
            "diff": diff,
            "percent": percent
        });
        if diff != 0 {
            invalid.push(entry.clone());
        }
        days.push(entry);
        total_expected += expected;
        work_days += 1;
        d += Duration::days(1);
    }

    let avg = if work_days == 0 {
        0.0
    } else {
        (total_minutes as f64 / 60.0 / work_days as f64 * 100.0).round() / 100.0
    };

    Ok(json!({
        "summary": {
            "totalMinutes": total_minutes,
            "totalHours": (total_minutes as f64 / 60.0 * 100.0).round() / 100.0,
            "expectedMinutes": total_expected,
            "workDays": work_days,
            "avgHoursPerDay": avg
        },
        "period": {"startDate": start, "endDate": end},
        "days": days,
        "invalidDays": invalid
    }))
}
