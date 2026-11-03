//! The value of `timestamp` (docs/reference/syntax.md, "型と値"): an instant in
//! UTC, held as WIT holds it (docs/reference/contract.md, "WIT との対応"):
//! seconds since 1970-01-01T00:00:00Z and nanoseconds within that second.

/// An instant. Ordering is the order of instants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    seconds: i64,
    nanoseconds: u32,
}

const NANOS_PER_SECOND: u32 = 1_000_000_000;
const SECONDS_PER_DAY: i64 = 86_400;

impl Timestamp {
    /// `None` when `nanoseconds` is not below 10^9.
    pub fn new(seconds: i64, nanoseconds: u32) -> Option<Self> {
        (nanoseconds < NANOS_PER_SECOND).then_some(Self {
            seconds,
            nanoseconds,
        })
    }

    pub fn seconds(self) -> i64 {
        self.seconds
    }

    pub fn nanoseconds(self) -> u32 {
        self.nanoseconds
    }

    /// Reads RFC 3339 with `Z`, as contracts write timestamps:
    /// `YYYY-MM-DDTHH:MM:SS` with an optional fraction of 1 to 9 digits, then
    /// `Z`. Offsets, lowercase `t` / `z` and leap seconds are not read.
    pub fn parse(text: &str) -> Option<Self> {
        if !text.is_ascii() || text.len() < 20 {
            return None;
        }
        let (date_time, rest) = text.split_at(19);
        let d = date_time.as_bytes();
        if d[4] != b'-' || d[7] != b'-' || d[10] != b'T' || d[13] != b':' || d[16] != b':' {
            return None;
        }
        let year = number(&d[0..4])?;
        let month = number(&d[5..7])?;
        let day = number(&d[8..10])?;
        let hour = number(&d[11..13])?;
        let minute = number(&d[14..16])?;
        let second = number(&d[17..19])?;
        let nanoseconds = match rest.strip_suffix('Z')? {
            "" => 0,
            fraction => {
                let digits = fraction.strip_prefix('.')?;
                if digits.is_empty() || digits.len() > 9 {
                    return None;
                }
                let scale = 10_i64.pow(9 - u32::try_from(digits.len()).ok()?);
                u32::try_from(number(digits.as_bytes())? * scale).ok()?
            }
        };
        if !(1..=12).contains(&month)
            || !(1..=days_in_month(year, month)).contains(&day)
            || hour > 23
            || minute > 59
            || second > 59
        {
            return None;
        }
        let seconds = days_from_civil(year, month, day) * SECONDS_PER_DAY
            + hour * 3600
            + minute * 60
            + second;
        Self::new(seconds, nanoseconds)
    }

    /// Writes RFC 3339 with `Z`. The fraction is left out when it is zero and
    /// has no trailing zeros otherwise, so each instant has one text. `None`
    /// when the year is not within 0000 to 9999.
    pub fn format(self) -> Option<String> {
        let days = self.seconds.div_euclid(SECONDS_PER_DAY);
        let time = self.seconds.rem_euclid(SECONDS_PER_DAY);
        let (year, month, day) = civil_from_days(days);
        if !(0..=9999).contains(&year) {
            return None;
        }
        let mut text = format!(
            "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
            time / 3600,
            time % 3600 / 60,
            time % 60
        );
        if self.nanoseconds > 0 {
            let fraction = format!("{:09}", self.nanoseconds);
            text.push('.');
            text.push_str(fraction.trim_end_matches('0'));
        }
        text.push('Z');
        Some(text)
    }
}

/// The value of ASCII digits; `None` if any byte is not a digit.
fn number(digits: &[u8]) -> Option<i64> {
    digits.iter().try_fold(0_i64, |n, &c| {
        c.is_ascii_digit().then(|| n * 10 + i64::from(c - b'0'))
    })
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

// The two conversions below are the proleptic Gregorian calendar algorithms
// of Howard Hinnant ("chrono-Compatible Low-Level Date Algorithms"), counting
// days from 1970-01-01.

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_from_march = (month + 9) % 12;
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    let year = year_of_era + era * 400;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Seconds from GNU date: date -u -d <text> +%s.
    const KNOWN: [(&str, i64); 9] = [
        ("1970-01-01T00:00:00Z", 0),
        ("2026-11-01T10:00:00Z", 1_793_527_200),
        ("2026-11-01T10:15:00Z", 1_793_528_100),
        ("2000-02-29T23:59:59Z", 951_868_799),
        ("1969-12-31T23:59:59Z", -1),
        ("1900-03-01T00:00:00Z", -2_203_891_200),
        ("0000-01-01T00:00:00Z", -62_167_219_200),
        ("9999-12-31T23:59:59Z", 253_402_300_799),
        ("2024-03-01T00:00:00Z", 1_709_251_200),
    ];

    #[test]
    fn reads_and_writes_known_instants() {
        for (text, seconds) in KNOWN {
            let ts = Timestamp::parse(text);
            assert_eq!(ts, Timestamp::new(seconds, 0), "{text}");
            assert_eq!(ts.and_then(Timestamp::format).as_deref(), Some(text));
            assert_eq!(ts.map(Timestamp::seconds), Some(seconds), "{text}");
            assert_eq!(ts.map(Timestamp::nanoseconds), Some(0), "{text}");
        }
    }

    #[test]
    fn reads_a_fraction_of_a_second() {
        let ts = Timestamp::parse("1970-01-01T00:00:01.5Z");
        assert_eq!(ts, Timestamp::new(1, 500_000_000));
        assert_eq!(ts.map(Timestamp::nanoseconds), Some(500_000_000));
        assert_eq!(
            Timestamp::parse("1969-12-31T23:59:59.000000001Z"),
            Timestamp::new(-1, 1)
        );
        assert_eq!(
            Timestamp::parse("1970-01-01T00:00:00.123456789Z"),
            Timestamp::new(0, 123_456_789)
        );
    }

    #[test]
    fn writes_the_fraction_without_trailing_zeros() {
        let format = |s, n| Timestamp::new(s, n).and_then(Timestamp::format);
        assert_eq!(
            format(1, 500_000_000).as_deref(),
            Some("1970-01-01T00:00:01.5Z")
        );
        assert_eq!(
            format(-1, 1).as_deref(),
            Some("1969-12-31T23:59:59.000000001Z")
        );
        assert_eq!(
            Timestamp::parse("1970-01-01T00:00:00.100Z").and_then(Timestamp::format),
            Some("1970-01-01T00:00:00.1Z".to_owned())
        );
    }

    #[test]
    fn refuses_what_is_not_rfc_3339_with_z() {
        for text in [
            "2026-02-29T00:00:00Z",
            "1900-02-29T00:00:00Z",
            "2000-02-30T00:00:00Z",
            "2026-04-31T00:00:00Z",
            "2026-00-01T00:00:00Z",
            "2026-13-01T00:00:00Z",
            "2026-01-00T00:00:00Z",
            "2026-01-01T24:00:00Z",
            "2026-01-01T00:60:00Z",
            "2026-01-01T23:59:60Z",
            "2026-01-01T00:00:00z",
            "2026-01-01t00:00:00Z",
            "2026-01-01 00:00:00Z",
            "2026-01-01T00:00:00+09:00",
            "2026-01-01T00:00:00",
            "2026-01-01T00:00:00.Z",
            "2026-01-01T00:00:00.1234567890Z",
            "2026-01-01T00:00:00ZZ",
            "2026/01/01T00:00:00Z",
            "2026/01-01T00:00:00Z",
            "2026-01/01T00:00:00Z",
            "2026-01-01T00-00:00Z",
            "2026-01-01T00:00-00Z",
            "+026-01-01T00:00:00Z",
            "2026-01-01T00:00:0\u{0661}Z",
            "",
        ] {
            assert_eq!(Timestamp::parse(text), None, "{text}");
        }
    }

    #[test]
    fn refuses_nanoseconds_of_a_whole_second() {
        assert_eq!(Timestamp::new(0, 1_000_000_000), None);
        assert!(Timestamp::new(0, 999_999_999).is_some());
    }

    #[test]
    fn does_not_write_years_outside_four_digits() {
        let before = Timestamp::new(-62_167_219_201, 0);
        let after = Timestamp::new(253_402_300_800, 0);
        assert_eq!(before.and_then(Timestamp::format), None);
        assert_eq!(after.and_then(Timestamp::format), None);
    }

    #[test]
    fn orders_by_instant() {
        let earlier = Timestamp::parse("1969-12-31T23:59:59.9Z");
        let later = Timestamp::parse("1970-01-01T00:00:00Z");
        assert!(earlier < later);
    }
}
