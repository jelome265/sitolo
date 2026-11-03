//! Timestamp conversion at the PostgreSQL boundary.
//!
//! Event and audit types carry `SystemTime` so the domain and application layers
//! do not depend on a database driver's timestamp crate. `SQLx` binds
//! `chrono::DateTime<Utc>`, so conversion happens here, once, with no panics.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};

/// Converts a `SystemTime` into a UTC `DateTime` without panicking.
///
/// Returns `None` when the instant is outside chrono's representable range.
pub(crate) fn to_utc(value: SystemTime) -> Option<DateTime<Utc>> {
    let (seconds, nanos) = match value.duration_since(UNIX_EPOCH) {
        Ok(after) => (i64::try_from(after.as_secs()).ok()?, after.subsec_nanos()),
        Err(before) => {
            let delta = before.duration();
            let seconds = i64::try_from(delta.as_secs()).ok()?;
            let nanos = delta.subsec_nanos();
            if nanos == 0 {
                (seconds.checked_neg()?, 0)
            } else {
                (
                    seconds.checked_neg()?.checked_sub(1)?,
                    1_000_000_000_u32.saturating_sub(nanos),
                )
            }
        }
    };

    DateTime::from_timestamp(seconds, nanos)
}

/// Converts a UTC `DateTime` back into `SystemTime` without panicking.
///
/// PostgreSQL's `timestamptz` range is wider than `SystemTime` on some
/// platforms. Returns `None` instead of overflowing when a persisted value
/// falls outside the platform's representable range.
pub(crate) fn from_utc(value: DateTime<Utc>) -> Option<SystemTime> {
    let seconds = value.timestamp();
    let nanos = value.timestamp_subsec_nanos();

    if seconds >= 0 {
        UNIX_EPOCH.checked_add(Duration::new(u64::try_from(seconds).ok()?, nanos))
    } else {
        // chrono timestamps are floor-based: -0.5s is represented as
        // `timestamp() == -1` plus 500_000_000 nanoseconds. Subtract the
        // whole seconds and add the positive fractional part back.
        UNIX_EPOCH
            .checked_sub(Duration::new(seconds.unsigned_abs(), 0))?
            .checked_add(Duration::new(0, nanos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    fn test_error(message: &str) -> io::Error {
        io::Error::other(message.to_owned())
    }

    #[test]
    fn epoch_round_trips() -> Result<(), Box<dyn std::error::Error>> {
        let utc = to_utc(UNIX_EPOCH).ok_or_else(|| test_error("epoch is representable"))?;
        assert_eq!(from_utc(utc), Some(UNIX_EPOCH));
        Ok(())
    }

    #[test]
    fn pre_epoch_instant_round_trips() -> Result<(), Box<dyn std::error::Error>> {
        let instant = UNIX_EPOCH
            .checked_sub(Duration::from_millis(500))
            .ok_or_else(|| test_error("test instant is representable"))?;
        let utc =
            to_utc(instant).ok_or_else(|| test_error("pre-epoch instant is representable"))?;
        assert_eq!(from_utc(utc), Some(instant));
        Ok(())
    }

    #[test]
    fn fractional_future_instant_round_trips() -> Result<(), Box<dyn std::error::Error>> {
        let instant = UNIX_EPOCH
            .checked_add(Duration::new(1_700_000_000, 123_456_789))
            .ok_or_else(|| test_error("test instant is representable"))?;
        let utc = to_utc(instant).ok_or_else(|| test_error("future instant is representable"))?;
        assert_eq!(from_utc(utc), Some(instant));
        Ok(())
    }
}
