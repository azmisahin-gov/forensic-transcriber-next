//! Time primitives.
//!
//! All wall-clock timestamps are stored as milliseconds since the Unix epoch
//! (UTC) in a signed 64-bit integer. The type is deliberately not a `SystemTime`
//! so it serialises deterministically and can be rendered without a heavyweight
//! calendar dependency.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Milliseconds since 1970-01-01T00:00:00Z.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(i64);

impl Timestamp {
    /// The current wall-clock time.
    pub fn now() -> Self {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        Timestamp(millis)
    }

    /// Construct from an explicit epoch-millisecond value (used when reading
    /// persisted rows and in tests).
    pub fn from_millis(millis: i64) -> Self {
        Timestamp(millis)
    }

    /// The underlying epoch-millisecond value.
    pub fn as_millis(self) -> i64 {
        self.0
    }

    /// RFC 3339 / ISO 8601 rendering in UTC, e.g. `2026-10-16T09:30:00.000Z`.
    pub fn to_rfc3339(self) -> String {
        let (days, ms_of_day) = div_floor(self.0, 86_400_000);
        let (year, month, day) = civil_from_days(days);
        let ms = ms_of_day as u64;
        let (hour, minute, second, milli) = (
            ms / 3_600_000,
            (ms / 60_000) % 60,
            (ms / 1000) % 60,
            ms % 1000,
        );
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{milli:03}Z")
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_rfc3339())
    }
}

/// Floor division that keeps the remainder non-negative, so pre-1970 values and
/// negative remainders behave correctly.
fn div_floor(a: i64, b: i64) -> (i64, i64) {
    let q = a.div_euclid(b);
    let r = a.rem_euclid(b);
    (q, r)
}

/// Days since epoch to (year, month, day). Hinnant's `civil_from_days`
/// algorithm (<https://howardhinnant.github.io/date_algorithms.html>), public
/// domain.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_zero_renders_correctly() {
        assert_eq!(
            Timestamp::from_millis(0).to_rfc3339(),
            "1970-01-01T00:00:00.000Z"
        );
    }

    #[test]
    fn known_instant_renders_correctly() {
        // 2026-10-16T09:30:00.000Z == 1792143000000 ms (verified independently).
        assert_eq!(
            Timestamp::from_millis(1_792_143_000_000).to_rfc3339(),
            "2026-10-16T09:30:00.000Z"
        );
    }

    #[test]
    fn pre_epoch_renders_correctly() {
        // 1969-12-31T23:59:59.999Z
        assert_eq!(
            Timestamp::from_millis(-1).to_rfc3339(),
            "1969-12-31T23:59:59.999Z"
        );
    }

    #[test]
    fn now_is_monotonic_enough_to_be_positive() {
        assert!(Timestamp::now().as_millis() > 1_600_000_000_000);
    }
}
