//! Port of `src/core/dates.ts`: small, dependency-free date helpers for the
//! anti-rot machinery. All dates are plain ISO calendar dates (`YYYY-MM-DD`),
//! compared in UTC so results do not depend on the runner's timezone.
//!
//! `today_iso`/`journal_month` (this module) use UTC days-since-epoch math
//! from `SystemTime`, no timezone involved. `local_timestamp`, used by
//! `commands/log.rs` for journal entries, lives here too and calls
//! `localtime_r` via a direct `extern "C"` binding (see the module doc at the
//! bottom) - the one place this crate leaves std for a platform C function,
//! per `docs/rust-port.md`'s dependency policy.

use std::time::{SystemTime, UNIX_EPOCH};

/// Days in each month of a given (possibly leap) year, 1-indexed by month.
fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Convert a UTC (year, month 1-12, day 1-31) civil date to days since the
/// Unix epoch (1970-01-01), using Howard Hinnant's `days_from_civil`
/// algorithm (proleptic Gregorian, no external crate needed).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = (month as i64 + 9) % 12; // [0, 11], Mar=0 .. Feb=11
    let doy = (153 * mp + 2) / 5 + day as i64 - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146097 + doe - 719468
}

/// Inverse of `days_from_civil`: days since epoch -> (year, month, day).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let year = if month <= 2 { y + 1 } else { y };
    (year, month, day)
}

fn now_days_since_epoch() -> i64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    secs.div_euclid(86_400)
}

/// `todayIso()`: today's UTC calendar date as `YYYY-MM-DD`.
pub fn today_iso() -> String {
    let (y, m, d) = civil_from_days(now_days_since_epoch());
    format!("{y:04}-{m:02}-{d:02}")
}

/// `isValidIsoDate`: true only for a well-formed, real calendar date in
/// `YYYY-MM-DD` form.
pub fn is_valid_iso_date(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() != 10 {
        return false;
    }
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let digits_ok = |slice: &[u8]| slice.iter().all(|b| b.is_ascii_digit());
    if !digits_ok(&bytes[0..4]) || !digits_ok(&bytes[5..7]) || !digits_ok(&bytes[8..10]) {
        return false;
    }
    let y: i64 = s[0..4].parse().unwrap();
    let m: u32 = s[5..7].parse().unwrap();
    let d: u32 = s[8..10].parse().unwrap();
    if !(1..=12).contains(&m) {
        return false;
    }
    if d < 1 || d > days_in_month(y, m) {
        return false;
    }
    true
}

/// `daysBetween`: whole days from `a` to `b` (positive when `b` is later).
/// Assumes valid `YYYY-MM-DD` input.
pub fn days_between(a: &str, b: &str) -> i64 {
    let (ay, am, ad) = (
        a[0..4].parse().unwrap(),
        a[5..7].parse().unwrap(),
        a[8..10].parse().unwrap(),
    );
    let (by, bm, bd) = (
        b[0..4].parse().unwrap(),
        b[5..7].parse().unwrap(),
        b[8..10].parse().unwrap(),
    );
    days_from_civil(by, bm, bd) - days_from_civil(ay, am, ad)
}

/// `YYYY-MM` for the current UTC month, used for `journal/YYYY-MM.md`.
pub fn journal_month() -> String {
    today_iso()[..7].to_string()
}

#[repr(C)]
struct Tm {
    tm_sec: i32,
    tm_min: i32,
    tm_hour: i32,
    tm_mday: i32,
    tm_mon: i32,
    tm_year: i32,
    tm_wday: i32,
    tm_yday: i32,
    tm_isdst: i32,
    // macOS and glibc both carry these two extra fields with this same
    // leading layout; we only target those two platforms (see
    // `docs/rust-port.md`).
    tm_gmtoff: i64,
    tm_zone: *const std::os::raw::c_char,
}

extern "C" {
    fn localtime_r(timer: *const i64, result: *mut Tm) -> *mut Tm;
}

/// `YYYY-MM-DD HH:MM` in local time, exactly as `src/commands/log.ts` writes
/// journal entries. Uses `localtime_r` directly rather than pulling in a
/// timezone crate.
pub fn local_timestamp() -> String {
    let secs: i64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let mut tm: Tm = unsafe { std::mem::zeroed() };
    let ok = unsafe { !localtime_r(&secs, &mut tm).is_null() };
    if !ok {
        // Fall back to UTC rather than panicking - this should not happen on
        // darwin/linux, but a journal entry is more useful with a UTC
        // timestamp than with a crashed `log` command.
        let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
        let rem = secs.rem_euclid(86_400);
        return format!(
            "{y:04}-{m:02}-{d:02} {:02}:{:02}",
            rem / 3600,
            (rem % 3600) / 60
        );
    }
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid_iso_date_accepts_real_dates_and_rejects_malformed_ones() {
        assert!(is_valid_iso_date("2026-07-21"));
        assert!(!is_valid_iso_date("2026-02-29")); // not a leap year
        assert!(!is_valid_iso_date("2026-13-01"));
        assert!(!is_valid_iso_date("2026-7-1")); // needs zero-padding
        assert!(!is_valid_iso_date("not-a-date"));
    }

    #[test]
    fn is_valid_iso_date_accepts_leap_day() {
        assert!(is_valid_iso_date("2024-02-29"));
    }

    #[test]
    fn days_between_is_signed_and_timezone_independent() {
        assert_eq!(days_between("2026-07-01", "2026-07-21"), 20);
        assert_eq!(days_between("2026-07-21", "2026-07-01"), -20);
        assert_eq!(days_between("2026-07-21", "2026-07-21"), 0);
    }

    #[test]
    fn days_between_crosses_a_year_boundary() {
        assert_eq!(days_between("2025-12-31", "2026-01-01"), 1);
    }

    #[test]
    fn today_iso_yields_a_valid_iso_date() {
        assert!(is_valid_iso_date(&today_iso()));
    }

    #[test]
    fn civil_conversion_round_trips_across_a_wide_range() {
        for days in [-800000i64, -1, 0, 1, 19723, 400000] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
    }

    #[test]
    fn local_timestamp_has_the_expected_shape() {
        let ts = local_timestamp();
        // "YYYY-MM-DD HH:MM"
        assert_eq!(ts.len(), 16);
        assert_eq!(ts.as_bytes()[10], b' ');
        assert!(is_valid_iso_date(&ts[..10]));
    }
}
