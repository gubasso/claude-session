//! Narrow wall-clock boundary for account metadata.

use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) trait Clock {
    fn now(&self) -> SystemTime;
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

/// Formats a system instant as RFC 3339 UTC seconds.
pub(crate) fn rfc3339_utc(instant: SystemTime) -> String {
    let seconds = instant
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .cast_signed();
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        day_seconds / 3600,
        day_seconds / 60 % 60,
        day_seconds % 60
    )
}

/// Reads an RFC 3339 UTC seconds timestamp back as a system instant.
///
/// The inverse of [`rfc3339_utc`], and the arithmetic half only: the shape,
/// range, and leap-year validation already happened in `RecordedAt::parse`, so
/// a value that reaches here is well-formed by construction. Timestamps before
/// the epoch return `None` rather than a clamped instant, because the only
/// consumers are an age and an expiry estimate and both would be nonsense.
pub(crate) fn parse_rfc3339_utc(value: &str) -> Option<SystemTime> {
    let bytes = value.as_bytes();
    if bytes.len() != 20 {
        return None;
    }
    let number = |range: std::ops::Range<usize>| -> Option<i64> {
        std::str::from_utf8(bytes.get(range)?).ok()?.parse().ok()
    };
    let (year, month, day, hour, minute, second) = (
        number(0..4)?,
        number(5..7)?,
        number(8..10)?,
        number(11..13)?,
        number(14..16)?,
        number(17..19)?,
    );
    let seconds = days_from_civil(year, month, day)
        .checked_mul(86_400)?
        .checked_add(hour * 3600 + minute * 60 + second)?;
    UNIX_EPOCH.checked_add(std::time::Duration::from_secs(u64::try_from(seconds).ok()?))
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn exact_utc_shape_covers_calendar_boundaries() {
        assert_eq!(rfc3339_utc(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        assert_eq!(
            rfc3339_utc(UNIX_EPOCH + std::time::Duration::from_hours(264_384)),
            "2000-02-29T00:00:00Z"
        );
    }

    /// Round-tripping is the assertion that matters, because the two halves
    /// share no code and an era-arithmetic sign error shows up only here.
    #[test]
    fn every_rendered_instant_parses_back_to_itself() {
        for rendered in [
            "1970-01-01T00:00:00Z",
            "2000-02-29T00:00:00Z",
            "2026-08-11T15:30:20Z",
            "2027-12-31T23:59:59Z",
            "2100-03-01T12:00:00Z",
        ] {
            let instant = parse_rfc3339_utc(rendered).expect("a rendered instant parses");
            assert_eq!(rfc3339_utc(instant), rendered);
        }
    }

    /// A pre-epoch mint time is refused rather than clamped, since the only
    /// consumers are an age and an expiry estimate.
    #[test]
    fn a_malformed_or_pre_epoch_timestamp_has_no_instant() {
        for rendered in ["1969-12-31T23:59:59Z", "2026-08-11T15:30:20", ""] {
            assert!(
                parse_rfc3339_utc(rendered).is_none(),
                "{rendered} must have no instant"
            );
        }
    }
}
