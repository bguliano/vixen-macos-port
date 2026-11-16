//! `TimeControl`, the window's time box: `mm:ss.fff`, one digit highlighted, typed over or rotated. F379 to F382, F394.

use vixen_core::TimeSpan;

/// The caret's last place.
const LAST_POSITION: usize = 8;

fn clamp(t: TimeSpan, min: TimeSpan, max: TimeSpan) -> TimeSpan {
    if t > max {
        max
    } else if t < min {
        min
    } else {
        t
    }
}

/// `value.ToString(@"mm\:ss\.fff")`: the minutes part alone, so an hour or more wraps.
pub fn format_time(t: TimeSpan) -> String {
    let ticks = t.0.abs();
    let ms = ticks / 10_000 % 1000;
    let total_seconds = ticks / 10_000_000;
    format!("{:02}:{:02}.{ms:03}", total_seconds / 60 % 60, total_seconds % 60)
}

/// `TryParseExact` with that format. Failing leaves the default, zero.
pub fn parse_time(text: &str) -> TimeSpan {
    let bytes = text.as_bytes();
    let good = bytes.len() == 9
        && bytes[2] == b':'
        && bytes[5] == b'.'
        && [0, 1, 3, 4, 6, 7, 8].iter().all(|i| bytes[*i].is_ascii_digit())
        && bytes[3] <= b'5';
    if !good {
        return TimeSpan::ZERO;
    }
    let number = |range: std::ops::Range<usize>| text[range].parse::<i64>().unwrap_or(0);
    let minutes = number(0..2);
    if minutes > 59 {
        return TimeSpan::ZERO;
    }
    TimeSpan((minutes * 60 + number(3..5)) * 10_000_000 + number(6..9) * 10_000)
}

/// `ValidateTime`.
pub fn validate(text: &str, min: TimeSpan, max: TimeSpan) -> String {
    format_time(clamp(parse_time(text), min, max))
}

/// `SelectPos(-1)`: the first non-zero digit, else the last place.
pub fn first_position(text: &str) -> usize {
    text.char_indices().find(|(i, c)| *i < LAST_POSITION && c.is_ascii_digit() && *c != '0').map_or(LAST_POSITION.min(text.len().saturating_sub(1)), |(i, _)| i)
}

/// `ReplaceChar` with the caret moved on: tens of minutes and seconds top out at 5.
pub fn replace_digit(text: &str, position: usize, digit: char, min: TimeSpan, max: TimeSpan) -> (String, usize) {
    if position >= text.len() || !digit.is_ascii_digit() {
        return (text.to_string(), position);
    }
    let digit = if (position == 0 || position == 3) && digit > '5' { '5' } else { digit };
    let mut chars: Vec<char> = text.chars().collect();
    chars[position] = digit;
    let next = match position + 1 {
        2 => 3,
        5 => 6,
        p if p >= LAST_POSITION => LAST_POSITION,
        p => p,
    };
    (validate(&chars.iter().collect::<String>(), min, max), next)
}

/// `RotateValue`: ±1 moves this digit; ⌘ makes that ±9, and ⌥ fills every digit after this one with its highest or with zero.
pub fn rotate(text: &str, position: usize, delta: i32, min: TimeSpan, max: TimeSpan) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    if position >= chars.len() {
        return text.to_string();
    }
    if delta < -9 || delta > 9 {
        for (i, c) in chars.iter_mut().enumerate().skip(position + 1) {
            if c.is_ascii_digit() {
                *c = if delta < 0 {
                    '0'
                } else if i == 0 || i == 3 {
                    '5'
                } else {
                    '9'
                };
            }
        }
    } else {
        let mut value = (chars[position] as i32 + delta).clamp('0' as i32, '9' as i32);
        if (position == 0 || position == 3) && value >= '6' as i32 {
            value = '5' as i32;
        }
        chars[position] = char::from_u32(value as u32).unwrap_or('0');
    }
    validate(&chars.iter().collect::<String>(), min, max)
}

/// The arrow keys and a press on a separator: the caret stays on digits. `direction` is -1 or +1.
pub fn move_position(position: usize, direction: i32) -> usize {
    if direction > 0 {
        match position {
            1 => 3,
            4 => 6,
            p if p >= LAST_POSITION => LAST_POSITION,
            p => p + 1,
        }
    } else {
        match position {
            0 => 0,
            3 => 1,
            6 => 4,
            p => p - 1,
        }
    }
}

/// `textBox_MouseDown`: a press lands on a digit.
pub fn snap_position(position: usize) -> usize {
    match position {
        2 => 3,
        5 => 6,
        p if p > LAST_POSITION => LAST_POSITION,
        p => p,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX: TimeSpan = TimeSpan(10 * 60 * 10_000_000);

    #[test]
    fn formats_and_parses_mm_ss_fff() {
        assert_eq!(format_time(TimeSpan::from_secs(83.25)), "01:23.250");
        assert_eq!(parse_time("01:23.250"), TimeSpan::from_secs(83.25));
        // Only the minutes part shows: an hour wraps.
        assert_eq!(format_time(TimeSpan::from_secs(3661.0)), "01:01.000");
        // Exact format, or the default.
        assert_eq!(parse_time("1:23.250"), TimeSpan::ZERO);
        assert_eq!(parse_time("99:00.000"), TimeSpan::ZERO);
        assert_eq!(parse_time("00:61.000"), TimeSpan::ZERO);
    }

    #[test]
    fn validate_clamps() {
        assert_eq!(validate("09:00.000", TimeSpan::ZERO, TimeSpan::from_secs(60.0)), "01:00.000");
        assert_eq!(validate("00:00.000", TimeSpan::from_secs(1.0), MAX), "00:01.000");
    }

    #[test]
    fn typing_overwrites_and_skips_separators() {
        assert_eq!(replace_digit("00:00.000", 0, '7', TimeSpan::ZERO, TimeSpan(i64::MAX)), ("50:00.000".to_string(), 1));
        assert_eq!(replace_digit("00:00.000", 1, '3', TimeSpan::ZERO, MAX), ("03:00.000".to_string(), 3));
        assert_eq!(replace_digit("00:00.000", 4, '3', TimeSpan::ZERO, MAX), ("00:03.000".to_string(), 6));
        assert_eq!(replace_digit("00:00.000", 8, '3', TimeSpan::ZERO, MAX), ("00:00.003".to_string(), 8));
    }

    #[test]
    fn rotating_moves_one_digit_or_fills_the_rest() {
        assert_eq!(rotate("00:05.000", 4, 1, TimeSpan::ZERO, MAX), "00:06.000");
        assert_eq!(rotate("00:05.000", 4, -9, TimeSpan::ZERO, MAX), "00:00.000");
        assert_eq!(rotate("00:05.000", 4, 9, TimeSpan::ZERO, MAX), "00:09.000");
        assert_eq!(rotate("00:05.000", 3, 9, TimeSpan::ZERO, MAX), "00:55.000");
        assert_eq!(rotate("00:05.000", 1, 99, TimeSpan::ZERO, TimeSpan(i64::MAX)), "00:59.999");
        assert_eq!(rotate("01:05.123", 1, -99, TimeSpan::ZERO, MAX), "01:00.000");
    }

    #[test]
    fn the_caret_stays_on_digits() {
        assert_eq!(first_position("00:05.000"), 4);
        assert_eq!(first_position("00:00.000"), 8);
        assert_eq!(move_position(1, 1), 3);
        assert_eq!(move_position(3, -1), 1);
        assert_eq!(move_position(8, 1), 8);
        assert_eq!(move_position(0, -1), 0);
        assert_eq!(snap_position(2), 3);
        assert_eq!(snap_position(9), 8);
    }
}
