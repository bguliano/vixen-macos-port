//! Local wall-clock dates, in the forms `Settings.xml` and the log files use.

use std::time::{SystemTime, UNIX_EPOCH};

/// A local date and time, ordered by when it happened. `DateTime` as these files use it, with no time zone.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalDateTime {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub millisecond: u16,
}

impl LocalDateTime {
    /// `DateTime.Now`.
    pub fn now() -> LocalDateTime {
        LocalDateTime::from_system_time(SystemTime::now())
    }

    /// That moment in the local time zone.
    pub fn from_system_time(time: SystemTime) -> LocalDateTime {
        let since_epoch = time.duration_since(UNIX_EPOCH).unwrap_or_default();
        let seconds = since_epoch.as_secs() as libc::time_t;
        let mut parts: libc::tm = unsafe { std::mem::zeroed() };
        // SAFETY: both pointers refer to live values for the length of the call.
        unsafe { libc::localtime_r(&seconds, &mut parts) };
        LocalDateTime {
            year: parts.tm_year + 1900,
            month: (parts.tm_mon + 1) as u8,
            day: parts.tm_mday as u8,
            hour: parts.tm_hour as u8,
            minute: parts.tm_min as u8,
            second: parts.tm_sec as u8,
            millisecond: since_epoch.subsec_millis() as u16,
        }
    }

    /// `M/d/yyyy h:mm:ss a` with or without the AM/PM, as `Convert.ToString(DateTime)` wrote it, or ISO 8601.
    pub fn parse(text: &str) -> Option<LocalDateTime> {
        let text = text.trim();
        if text.contains('/') {
            Self::parse_us(text)
        } else {
            Self::parse_iso(text)
        }
    }

    fn parse_us(text: &str) -> Option<LocalDateTime> {
        let mut words = text.split_whitespace();
        let date = words.next()?;
        let mut date_parts = date.split('/');
        let month: u8 = date_parts.next()?.parse().ok()?;
        let day: u8 = date_parts.next()?.parse().ok()?;
        let year: i32 = date_parts.next()?.parse().ok()?;
        if date_parts.next().is_some() {
            return None;
        }
        let (mut hour, minute, second) = match words.next() {
            Some(time) => parse_clock(time)?.0,
            None => (0, 0, 0),
        };
        match words.next().map(str::to_ascii_uppercase).as_deref() {
            None => {}
            Some("AM") => {
                if hour == 12 {
                    hour = 0;
                }
            }
            Some("PM") => {
                if hour < 12 {
                    hour += 12;
                }
            }
            Some(_) => return None,
        }
        if words.next().is_some() {
            return None;
        }
        Self::checked(year, month, day, hour, minute, second, 0)
    }

    fn parse_iso(text: &str) -> Option<LocalDateTime> {
        let (date, time) = match text.split_once(['T', ' ']) {
            Some((date, time)) => (date, Some(time)),
            None => (text, None),
        };
        let mut date_parts = date.split('-');
        let year: i32 = date_parts.next()?.parse().ok()?;
        let month: u8 = date_parts.next()?.parse().ok()?;
        let day: u8 = date_parts.next()?.parse().ok()?;
        if date_parts.next().is_some() {
            return None;
        }
        let ((hour, minute, second), millisecond) = match time {
            // A zone suffix is ignored: every value here is a local wall-clock time.
            Some(time) => parse_clock(time.trim_end_matches('Z').split(['+', '-']).next()?)?,
            None => ((0, 0, 0), 0),
        };
        Self::checked(year, month, day, hour, minute, second, millisecond)
    }

    fn checked(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8, millisecond: u16) -> Option<LocalDateTime> {
        let valid = (1..=12).contains(&month) && (1..=31).contains(&day) && hour < 24 && minute < 60 && second < 60;
        valid.then_some(LocalDateTime { year, month, day, hour, minute, second, millisecond })
    }

    /// `12/20/2025 5:43:19 PM`.
    pub fn settings_text(&self) -> String {
        let (hour, suffix) = match self.hour {
            0 => (12, "AM"),
            1..=11 => (self.hour, "AM"),
            12 => (12, "PM"),
            _ => (self.hour - 12, "PM"),
        };
        format!("{}/{}/{} {}:{:02}:{:02} {}", self.month, self.day, self.year, hour, self.minute, self.second, suffix)
    }

    /// `MM/dd/yyyy HH:mm:ss.fff`, the time at the start of a log line.
    pub fn log_text(&self) -> String {
        format!(
            "{:02}/{:02}/{:04} {:02}:{:02}:{:02}.{:03}",
            self.month, self.day, self.year, self.hour, self.minute, self.second, self.millisecond
        )
    }

    /// NLog's `${shortdate}`: `yyyy-MM-dd`.
    pub fn short_date(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// `h:mm:ss`, with `.fff` allowed on the seconds.
fn parse_clock(text: &str) -> Option<((u8, u8, u8), u16)> {
    let mut parts = text.split(':');
    let hour: u8 = parts.next()?.parse().ok()?;
    let minute: u8 = parts.next()?.parse().ok()?;
    let seconds = parts.next().unwrap_or("0");
    if parts.next().is_some() {
        return None;
    }
    let (whole, fraction) = seconds.split_once('.').unwrap_or((seconds, ""));
    let second: u8 = whole.parse().ok()?;
    let millisecond = if fraction.is_empty() {
        0
    } else {
        let digits: String = fraction.chars().chain("000".chars()).take(3).collect();
        digits.parse().ok()?
    };
    Some(((hour, minute, second), millisecond))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> LocalDateTime {
        LocalDateTime { year, month, day, hour, minute, second, millisecond: 0 }
    }

    #[test]
    fn reads_the_form_the_real_files_hold() {
        assert_eq!(LocalDateTime::parse("12/20/2025 5:43:19 PM"), Some(at(2025, 12, 20, 17, 43, 19)));
        assert_eq!(LocalDateTime::parse("1/2/2025 12:00:01 AM"), Some(at(2025, 1, 2, 0, 0, 1)));
        assert_eq!(LocalDateTime::parse("1/2/2025 12:00:01 PM"), Some(at(2025, 1, 2, 12, 0, 1)));
        assert_eq!(LocalDateTime::parse("1/2/2025 17:00:01"), Some(at(2025, 1, 2, 17, 0, 1)));
    }

    #[test]
    fn reads_iso_8601() {
        assert_eq!(LocalDateTime::parse("2025-12-20T17:43:19"), Some(at(2025, 12, 20, 17, 43, 19)));
        assert_eq!(LocalDateTime::parse("2025-12-20T17:43:19Z"), Some(at(2025, 12, 20, 17, 43, 19)));
        assert_eq!(
            LocalDateTime::parse("2025-12-20 17:43:19.5").map(|d| d.millisecond),
            Some(500),
        );
    }

    #[test]
    fn rejects_what_is_not_a_date() {
        for bad in ["", "yesterday", "13/40/2025 1:00:00 PM", "20.12.2025", "1/2/2025 1:00:00 XM", "2025-13-01"] {
            assert_eq!(LocalDateTime::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn writes_each_form() {
        let date = LocalDateTime { millisecond: 588, ..at(2025, 12, 20, 17, 43, 19) };
        assert_eq!(date.settings_text(), "12/20/2025 5:43:19 PM");
        assert_eq!(date.log_text(), "12/20/2025 17:43:19.588");
        assert_eq!(date.short_date(), "2025-12-20");
        assert_eq!(at(2025, 1, 2, 0, 5, 9).settings_text(), "1/2/2025 12:05:09 AM");
        assert_eq!(at(2025, 1, 2, 12, 5, 9).settings_text(), "1/2/2025 12:05:09 PM");
    }

    #[test]
    fn dates_order_by_time() {
        assert!(LocalDateTime::parse("1/1/2026 1:00:00 AM") > LocalDateTime::parse("12/31/2025 11:00:00 PM"));
    }

    #[test]
    fn now_is_a_plausible_date_that_round_trips_to_the_second() {
        let now = LocalDateTime::now();
        assert!(now.year >= 2024);
        let back = LocalDateTime::parse(&now.settings_text()).unwrap();
        assert_eq!(back, LocalDateTime { millisecond: 0, ..now });
    }
}
