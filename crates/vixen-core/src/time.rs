//! System.TimeSpan: 100 ns ticks, with xs:duration text as written by DataContractSerializer.

use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

pub const TICKS_PER_MS: i64 = 10_000;
pub const TICKS_PER_SEC: i64 = 10_000_000;
pub const TICKS_PER_MIN: i64 = 60 * TICKS_PER_SEC;
pub const TICKS_PER_HOUR: i64 = 60 * TICKS_PER_MIN;
pub const TICKS_PER_DAY: i64 = 24 * TICKS_PER_HOUR;

#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct TimeSpan(pub i64);

impl TimeSpan {
    pub const ZERO: TimeSpan = TimeSpan(0);
    pub const MIN: TimeSpan = TimeSpan(i64::MIN);
    pub const MAX: TimeSpan = TimeSpan(i64::MAX);

    #[inline]
    pub const fn from_ticks(t: i64) -> TimeSpan {
        TimeSpan(t)
    }

    /// `new TimeSpan(0, 0, 0, 0, ms)`.
    #[inline]
    pub const fn from_ms_i(ms: i64) -> TimeSpan {
        TimeSpan(ms * TICKS_PER_MS)
    }

    /// `TimeSpan.FromMilliseconds(double)`. Modern .NET truncates to a whole tick, it does not round.
    #[inline]
    pub fn from_ms(ms: f64) -> TimeSpan {
        TimeSpan((ms * TICKS_PER_MS as f64) as i64)
    }

    /// `TimeSpan.FromSeconds(double)`, which truncates to a whole tick the same way.
    #[inline]
    pub fn from_secs(s: f64) -> TimeSpan {
        TimeSpan((s * TICKS_PER_SEC as f64) as i64)
    }

    #[inline]
    pub const fn ticks(self) -> i64 {
        self.0
    }

    #[inline]
    pub fn total_ms(self) -> f64 {
        self.0 as f64 / TICKS_PER_MS as f64
    }

    #[inline]
    pub fn total_secs(self) -> f64 {
        self.0 as f64 / TICKS_PER_SEC as f64
    }

    /// Parses xs:duration (`P1DT2H3M4.5S`, optional leading `-`). Returns None on malformed text.
    pub fn parse_xml(text: &str) -> Option<TimeSpan> {
        let text = text.trim();
        let (negative, rest) = match text.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, text),
        };
        let rest = rest.strip_prefix('P')?;
        let mut ticks: i128 = 0;
        let mut in_time = false;
        let mut num = String::new();
        let mut seen_any = false;
        for ch in rest.chars() {
            match ch {
                '0'..='9' | '.' => num.push(ch),
                'T' => in_time = true,
                'D' | 'H' | 'M' | 'S' => {
                    let scale: i128 = match (ch, in_time) {
                        ('D', false) => TICKS_PER_DAY as i128,
                        ('H', true) => TICKS_PER_HOUR as i128,
                        ('M', true) => TICKS_PER_MIN as i128,
                        ('S', true) => TICKS_PER_SEC as i128,
                        _ => return None,
                    };
                    ticks += parse_scaled(&num, scale)?;
                    num.clear();
                    seen_any = true;
                }
                _ => return None,
            }
        }
        if !num.is_empty() || !seen_any {
            return None;
        }
        let ticks = if negative { -ticks } else { ticks };
        Some(TimeSpan(ticks.clamp(i64::MIN as i128, i64::MAX as i128) as i64))
    }

    /// Writes xs:duration the way `XmlConvert.ToString(TimeSpan)` does.
    pub fn to_xml(self) -> String {
        if self.0 == 0 {
            return "PT0S".to_string();
        }
        let negative = self.0 < 0;
        let mut t = (self.0 as i128).unsigned_abs();
        let mut out = String::new();
        if negative {
            out.push('-');
        }
        out.push('P');
        let day = TICKS_PER_DAY as u128;
        let days = t / day;
        t %= day;
        if days > 0 {
            out.push_str(&format!("{days}D"));
        }
        if t > 0 {
            out.push('T');
            let hours = t / TICKS_PER_HOUR as u128;
            t %= TICKS_PER_HOUR as u128;
            let mins = t / TICKS_PER_MIN as u128;
            t %= TICKS_PER_MIN as u128;
            if hours > 0 {
                out.push_str(&format!("{hours}H"));
            }
            if mins > 0 {
                out.push_str(&format!("{mins}M"));
            }
            if t > 0 {
                let secs = t / TICKS_PER_SEC as u128;
                let frac = t % TICKS_PER_SEC as u128;
                if frac == 0 {
                    out.push_str(&format!("{secs}S"));
                } else {
                    let f = format!("{frac:07}");
                    out.push_str(&format!("{secs}.{}S", f.trim_end_matches('0')));
                }
            }
        }
        out
    }
}

fn parse_scaled(num: &str, scale: i128) -> Option<i128> {
    if num.is_empty() {
        return None;
    }
    match num.split_once('.') {
        None => Some(num.parse::<i128>().ok()? * scale),
        Some((whole, frac)) => {
            let w: i128 = if whole.is_empty() { 0 } else { whole.parse().ok()? };
            let mut result = w * scale;
            let mut unit = scale;
            for c in frac.chars() {
                unit /= 10;
                if unit == 0 {
                    break;
                }
                result += c.to_digit(10)? as i128 * unit;
            }
            Some(result)
        }
    }
}

impl Add for TimeSpan {
    type Output = TimeSpan;
    #[inline]
    fn add(self, o: TimeSpan) -> TimeSpan {
        TimeSpan(self.0.saturating_add(o.0))
    }
}

impl Sub for TimeSpan {
    type Output = TimeSpan;
    #[inline]
    fn sub(self, o: TimeSpan) -> TimeSpan {
        TimeSpan(self.0.saturating_sub(o.0))
    }
}

impl AddAssign for TimeSpan {
    #[inline]
    fn add_assign(&mut self, o: TimeSpan) {
        *self = *self + o;
    }
}

impl SubAssign for TimeSpan {
    #[inline]
    fn sub_assign(&mut self, o: TimeSpan) {
        *self = *self - o;
    }
}

impl Neg for TimeSpan {
    type Output = TimeSpan;
    #[inline]
    fn neg(self) -> TimeSpan {
        TimeSpan(-self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_round_trip() {
        for (text, ticks) in [
            ("PT0S", 0),
            ("PT1S", TICKS_PER_SEC),
            ("PT1M30.5S", 90 * TICKS_PER_SEC + 5 * TICKS_PER_SEC / 10),
            ("P1DT2H", TICKS_PER_DAY + 2 * TICKS_PER_HOUR),
            ("-PT0.05S", -500_000),
        ] {
            let t = TimeSpan::parse_xml(text).unwrap();
            assert_eq!(t.ticks(), ticks, "{text}");
            assert_eq!(t.to_xml(), text);
        }
    }

    #[test]
    fn from_ms_truncates_to_a_tick_like_dotnet() {
        // 1000.0 * (0.35 - 0.2) is 149.99999999999997, so .NET yields 1499999 ticks, not 1500000.
        assert_eq!(TimeSpan::from_ms(1000.0 * (0.35 - 0.2)).ticks(), 1_499_999);
        assert_eq!(TimeSpan::from_ms(0.00009).ticks(), 0);
        assert_eq!(TimeSpan::from_ms(33.33333333).ticks(), 333_333);
        assert_eq!(TimeSpan::from_ms(-0.00009).ticks(), 0);
    }

    #[test]
    fn min_value_parses() {
        // TimeSpan.MinValue as DataContractSerializer writes it for an unset TimePerPixel.
        let t = TimeSpan::parse_xml("-P10675199DT2H48M5.4775808S").unwrap();
        assert_eq!(t, TimeSpan::MIN);
        assert_eq!(t.to_xml(), "-P10675199DT2H48M5.4775808S");
    }

    #[test]
    fn rejects_garbage() {
        assert!(TimeSpan::parse_xml("").is_none());
        assert!(TimeSpan::parse_xml("P").is_none());
        assert!(TimeSpan::parse_xml("hello").is_none());
    }
}
