
use crate::Timestamp;

impl DateTime {
    /// The minimum representable [`Timestamp`] as a `DateTime`.
    pub(crate) const MIN: DateTime = DateTime {
        year: -292_277_022_657,
        month: 1,
        day: 27,
        hour: 8,
        minute: 29,
        second: 52,
        nanos: 0,
    };

    /// The maximum representable [`Timestamp`] as a `DateTime`.
    pub(crate) const MAX: DateTime = DateTime {
        year: 292_277_026_596,
        month: 12,
        day: 4,
        hour: 15,
        minute: 30,
        second: 7,
        nanos: 999_999_999,
    };

    /// Returns `true` if the `DateTime` is a valid calendar date.
    pub(crate) fn is_valid(&self) -> bool {
        self >= &DateTime::MIN
            && self <= &DateTime::MAX
            && self.month > 0
            && self.month <= 12
            && self.day > 0
            && self.day <= days_in_month(self.year, self.month)
            && self.hour < 24
            && self.minute < 60
            && self.second < 60
            && self.nanos < 1_000_000_000
    }
}

/// A point in time, represented as a date and time in the UTC timezone.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DateTime {
    /// The year.
    pub(crate) year: i64,
    /// The month of the year, from 1 to 12, inclusive.
    pub(crate) month: u8,
    /// The day of the month, from 1 to 31, inclusive.
    pub(crate) day: u8,
    /// The hour of the day, from 0 to 23, inclusive.
    pub(crate) hour: u8,
    /// The minute of the hour, from 0 to 59, inclusive.
    pub(crate) minute: u8,
    /// The second of the minute, from 0 to 59, inclusive.
    pub(crate) second: u8,
    /// The nanoseconds, from 0 to 999_999_999, inclusive.
    pub(crate) nanos: u32,
}


/// Returns the offset in seconds from the Unix epoch of the start of a year.
///
/// musl's [`__year_to_secs`][1] converted to Rust via c2rust and then cleaned up by hand.
///
/// Returns an i128 because the start of the earliest supported year underflows i64.
///
/// [1]: https://git.musl-libc.org/cgit/musl/tree/src/time/__year_to_secs.c
pub(crate) fn year_to_seconds(year: i64) -> (i128, bool) {
    let is_leap;
    let year = year - 1900;

    // Fast path for years 1900 - 2038.
    if year as u64 <= 138 {
        let mut leaps: i64 = (year - 68) >> 2;
        if (year - 68).trailing_zeros() >= 2 {
            leaps -= 1;
            is_leap = true;
        } else {
            is_leap = false;
        }
        return (
            i128::from(31_536_000 * (year - 70) + 86400 * leaps),
            is_leap,
        );
    }

    let centuries: i64;
    let mut leaps: i64;

    let mut cycles: i64 = (year - 100) / 400;
    let mut rem: i64 = (year - 100) % 400;

    if rem < 0 {
        cycles -= 1;
        rem += 400
    }
    if rem == 0 {
        is_leap = true;
        centuries = 0;
        leaps = 0;
    } else {
        if rem >= 200 {
            if rem >= 300 {
                centuries = 3;
                rem -= 300;
            } else {
                centuries = 2;
                rem -= 200;
            }
        } else if rem >= 100 {
            centuries = 1;
            rem -= 100;
        } else {
            centuries = 0;
        }
        if rem == 0 {
            is_leap = false;
            leaps = 0;
        } else {
            leaps = rem / 4;
            rem %= 4;
            is_leap = rem == 0;
        }
    }
    leaps += 97 * cycles + 24 * centuries - i64::from(is_leap);

    (
        i128::from((year - 100) * 31_536_000) + i128::from(leaps * 86400 + 946_684_800 + 86400),
        is_leap,
    )
}


/// Returns the number of days in the month.
fn days_in_month(year: i64, month: u8) -> u8 {
    const DAYS_IN_MONTH: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let (_, is_leap) = year_to_seconds(year);
    DAYS_IN_MONTH[usize::from(month - 1)] + u8::from(is_leap && month == 2)
}


/// Returns the number of seconds in the year prior to the start of the provided month.
///
/// This is musl's [`__month_to_secs`][1] converted to Rust via c2rust and then cleaned up by hand.
///
/// [1]: https://git.musl-libc.org/cgit/musl/tree/src/time/__month_to_secs.c
fn month_to_seconds(month: u8, is_leap: bool) -> u32 {
    const SECS_THROUGH_MONTH: [u32; 12] = [
        0,
        31 * 86400,
        59 * 86400,
        90 * 86400,
        120 * 86400,
        151 * 86400,
        181 * 86400,
        212 * 86400,
        243 * 86400,
        273 * 86400,
        304 * 86400,
        334 * 86400,
    ];
    let t = SECS_THROUGH_MONTH[usize::from(month - 1)];
    if is_leap && month > 2 {
        t + 86400
    } else {
        t
    }
}


/// Returns the offset in seconds from the Unix epoch of the date time.
///
/// This is musl's [`__tm_to_secs`][1] converted to Rust via [c2rust[2] and then cleaned up by
/// hand.
///
/// [1]: https://git.musl-libc.org/cgit/musl/tree/src/time/__tm_to_secs.c
/// [2]: https://c2rust.com/
fn date_time_to_seconds(tm: &DateTime) -> i64 {
    let (start_of_year, is_leap) = year_to_seconds(tm.year);

    let seconds_within_year = month_to_seconds(tm.month, is_leap)
        + 86400 * u32::from(tm.day - 1)
        + 3600 * u32::from(tm.hour)
        + 60 * u32::from(tm.minute)
        + u32::from(tm.second);

    (start_of_year + i128::from(seconds_within_year)) as i64
}

impl From<DateTime> for Timestamp {
    fn from(date_time: DateTime) -> Timestamp {
        let seconds = date_time_to_seconds(&date_time);
        let nanos = date_time.nanos;
        Timestamp {
            seconds,
            nanos: nanos as i32,
        }
    }
}
