use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current date as `YYYY-MM-DD`.
pub(super) fn today_str() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (year, month, day) = unix_secs_to_ymd(secs);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Returns the previous date as `YYYY-MM-DD`.
#[allow(dead_code)]
pub(super) fn yesterday_str() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .saturating_sub(86_400);
    let (year, month, day) = unix_secs_to_ymd(secs);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Current HH:MM:SS timestamp.
#[allow(dead_code)]
pub(super) fn timestamp_hms() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    format!("{h:02}:{m:02}:{s:02}")
}

/// Minimal no-dep unix timestamp → (year, month, day).
pub(crate) fn unix_secs_to_ymd(mut secs: u64) -> (u32, u32, u32) {
    secs /= 86_400; // days since unix epoch
    let mut year = 1970u32;
    loop {
        let leap = if year.is_multiple_of(400) {
            366u64
        } else if year.is_multiple_of(100) {
            365
        } else if year.is_multiple_of(4) {
            366
        } else {
            365
        };
        if secs < leap {
            break;
        }
        secs -= leap;
        year += 1;
    }
    let leap_year =
        year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100));
    let days_in_month = [
        31u32,
        if leap_year { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0;
    let mut remaining = secs as u32;
    for (i, &d) in days_in_month.iter().enumerate() {
        if remaining < d {
            month = i as u32 + 1;
            break;
        }
        remaining -= d;
    }
    if month == 0 {
        month = 12;
    }
    (year, month, remaining + 1)
}
