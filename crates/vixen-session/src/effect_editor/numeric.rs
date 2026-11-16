//! What the number fields and sliders do with their values: F24, F25, F26, F32. Everything here works in shown units; `Display` converts
//! before and after (F33).

use super::Mods;

/// Travel before a press on the strip becomes a drag. `RepeatEditor.DragTolerance`.
const DRAG_TOLERANCE: f64 = 2.0;

/// A whole-number field's limits. `IntegerEditor`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct IntField {
    pub min: i128,
    pub max: i128,
    pub step: f64,
}

impl IntField {
    /// No `[NumberRange]`: the editor's minimum of 0, and the type's limit as the maximum.
    pub fn unranged(type_max: i128) -> IntField {
        IntField { min: 0, max: type_max, step: 1.0 }
    }

    /// `EnforceLimitsAndPrecision`'s reading of a `[NumberRange]`: both ends clamp to the type, and a minimum above the maximum wins.
    pub fn ranged(min: f64, max: f64, step: f64, bounds: (i128, i128)) -> IntField {
        let min = (min as i128).clamp(bounds.0, bounds.1);
        let max = (max as i128).clamp(bounds.0, bounds.1);
        IntField { min, max: max.max(min), step }
    }

    /// `decimal.Clamp(decimal.Truncate(value), minimum, maximum)`, on a value already truncated.
    pub fn enforce(self, value: i128) -> i128 {
        value.clamp(self.min, self.max)
    }

    /// `decimal.TryParse(text, NumberStyles.Integer)` then the limits: an optional sign and surrounding spaces, no decimals. `None` means revert.
    pub fn parse(self, text: &str) -> Option<i128> {
        let trimmed = text.trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c'));
        let digits = trimmed.strip_prefix(['+', '-']).unwrap_or(trimmed);
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        trimmed.parse::<i128>().ok().map(|v| self.enforce(v))
    }
}

/// A decimal field's limits. `DoubleEditor`. With no `[NumberRange]` the precision is 0 places, so such a double rounds to a whole number (KEEP, F25).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DoubleField {
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub precision: u32,
}

impl DoubleField {
    pub fn unranged() -> DoubleField {
        DoubleField { min: 0.0, max: f64::MAX, step: 1.0, precision: 0 }
    }

    /// `Math.Round(Math.Max(Minimum, Math.Min(Maximum, value)), MaxPrecision)`.
    pub fn enforce(self, value: f64) -> f64 {
        round_places(self.min.max(self.max.min(value)), self.precision)
    }

    pub fn parse(self, text: &str) -> Option<f64> {
        let cleaned: String = text.trim().chars().filter(|c| *c != ',').collect();
        let value = cleaned.parse::<f64>().ok()?;
        value.is_finite().then(|| self.enforce(value))
    }
}

/// `Math.Round(x, places)` in Banker's rounding.
fn round_places(value: f64, places: u32) -> f64 {
    if places == 0 {
        return value.round_ties_even();
    }
    let scale = 10f64.powi(places as i32);
    (value * scale).round_ties_even() / scale
}

/// One number field's limits.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum NumberField {
    Int(IntField),
    Double(DoubleField),
}

impl NumberField {
    fn step(self) -> f64 {
        match self {
            NumberField::Int(f) => f.step,
            NumberField::Double(f) => f.step,
        }
    }

    /// The text a field shows after this text is typed in, or `None` to revert.
    pub fn normalized(self, text: &str) -> Option<String> {
        match self {
            NumberField::Int(f) => f.parse(text).map(|v| v.to_string()),
            NumberField::Double(f) => f.parse(text).map(double_text),
        }
    }
}

/// `double.ToString()`.
pub fn double_text(value: f64) -> String {
    format!("{value}")
}

/// One press on a field's drag strip. Lines up with `IntegerEditor`'s and `DoubleEditor`'s mouse handlers, with y growing downwards.
#[derive(Clone, Debug)]
pub struct NumberDrag {
    field: NumberField,
    drag_start: (f64, f64),
    last: (f64, f64),
    dragging: bool,
    value_text: String,
    change_offset: f64,
    hold: String,
}

impl NumberDrag {
    /// `OnMouseDown`.
    pub fn press(field: NumberField, value_text: &str, x: f64, y: f64) -> NumberDrag {
        NumberDrag {
            field,
            drag_start: (x, y),
            last: (x, y),
            dragging: false,
            value_text: value_text.to_string(),
            change_offset: 0.0,
            hold: value_text.to_string(),
        }
    }

    /// `OnMouseMove`. Returns the new text when this move changed it. Right and up increase, left and down decrease.
    pub fn moved(&mut self, x: f64, y: f64, mods: Mods) -> Option<String> {
        if !self.dragging {
            let (dx, dy) = (x - self.drag_start.0, y - self.drag_start.1);
            if (dx * dx + dy * dy).sqrt() > DRAG_TOLERANCE {
                self.dragging = true;
                self.drag_start = (x, y);
                self.last = (x, y);
                self.change_offset = 0.0;
            }
            return None;
        }
        let (dx, dy) = (x - self.last.0, y - self.last.1);
        let length = (dx * dx + dy * dy).sqrt().round_ties_even();
        if length == 0.0 {
            return None;
        }
        self.last = (x, y);
        Some(self.apply_step(if dx > dy { length } else { -length }, mods))
    }

    /// `CalculateValue`. `SmallChange` and `LargeChange` are never set, so ⌘ and ⇧ both mean ×1, not × tick (KEEP, F24).
    fn apply_step(&mut self, change: f64, mods: Mods) -> String {
        let per_step = if mods.command || mods.shift { 1.0 } else { self.field.step() };
        self.change_offset += change * per_step;
        self.hold = match self.field {
            NumberField::Int(f) => {
                let base = self.value_text.trim().parse::<i128>().unwrap_or(0);
                f.enforce(add_truncated(base, self.change_offset)).to_string()
            }
            NumberField::Double(f) => {
                let base = self.value_text.trim().parse::<f64>().unwrap_or(0.0);
                double_text(f.enforce(base + self.change_offset))
            }
        };
        self.hold.clone()
    }

    /// `_holdValue`: what the 300 ms and 500 ms timers and the release commit.
    pub fn hold_text(&self) -> &str {
        &self.hold
    }

    pub fn is_dragging(&self) -> bool {
        self.dragging
    }
}

/// `decimal.Truncate(_changeValue + _changeOffset)`. A double keeps a 64-bit value's low digits only when the base is small, so a huge base adds the offset's whole part.
fn add_truncated(base: i128, offset: f64) -> i128 {
    if base.abs() < (1i128 << 52) {
        (base as f64 + offset).trunc() as i128
    } else {
        base + offset.trunc() as i128
    }
}

/// F32's four sliders.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SliderKind {
    /// `SliderEditor`, an int shown as a double. Snaps to ticks.
    Int,
    /// `SliderDoubleEditor`.
    Double,
    /// `LevelEditor`, a 0..1 value on a 0..100 slider that snaps.
    Level,
    /// `SliderPercentageEditor`, a 0..1 value on a 0..100 slider that doesn't.
    Percent,
}

impl SliderKind {
    pub fn snaps(self) -> bool {
        matches!(self, SliderKind::Int | SliderKind::Level)
    }
}

/// `Slider.SnapToTick`: the closer of the two neighbouring ticks, and the upper one on a tie. A slider that doesn't snap, or with no tick, gives back its value.
pub fn snap_to_tick(value: f64, min: f64, max: f64, tick: f64, snap: bool) -> f64 {
    if !snap || tick <= 0.0 {
        return value;
    }
    let previous = min + ((value - min) / tick).round_ties_even() * tick;
    let next = max.min(previous + tick);
    if value >= (previous + next) * 0.5 || close(value, (previous + next) * 0.5) {
        next
    } else {
        previous
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// The slider's hover tooltip, from the value on the slider. The stored value's units differ per kind, and `slider_value` undoes them.
pub fn slider_tooltip(kind: SliderKind, slider_value: f64) -> String {
    let n = match kind {
        // `(int)(stored * 100)` truncates, and the slider already shows `stored * 100`, with its floating error: 0.29 gives 28.
        SliderKind::Level => slider_value.trunc() as i64,
        _ => slider_value.round_ties_even() as i64,
    };
    n.to_string()
}

/// `Slider`'s automatic tooltip while dragging: `Value.ToString("N")` with `AutoToolTipPrecision` of 0.
pub fn slider_drag_tip(slider_value: f64) -> String {
    let rounded = slider_value.round() as i64;
    let digits = rounded.unsigned_abs().to_string();
    let mut grouped = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    if rounded < 0 {
        grouped.insert(0, '-');
    }
    grouped
}

/// `PercentageValueConverter.ConvertBack`: `f` outside 0..1 becomes 1, not a clamp (KEEP, F32). The arithmetic is in `float`.
pub fn percent_from_slider(slider_value: f64) -> f64 {
    let f = slider_value as f32 / 100.0;
    if !(0.0..=1.0).contains(&f) {
        1.0
    } else {
        f as f64
    }
}

/// `Convert.ToInt32(double)`.
pub fn to_int32(value: f64) -> i32 {
    value.round_ties_even() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(min: i128, max: i128) -> IntField {
        IntField { min, max, step: 1.0 }
    }

    #[test]
    fn int_parsing_follows_number_styles_integer() {
        let f = int(0, 255);
        assert_eq!(f.parse("12.7"), None);
        assert_eq!(f.parse(" -5 "), Some(0));
        assert_eq!(f.parse(""), None);
        assert_eq!(f.parse("   "), None);
        assert_eq!(f.parse("300"), Some(255));
        assert_eq!(f.parse("+7"), Some(7));
        assert_eq!(f.parse("1e3"), None);
    }

    #[test]
    fn a_minimum_above_the_maximum_wins() {
        let f = IntField::ranged(10.0, 5.0, 1.0, (i32::MIN as i128, i32::MAX as i128));
        assert_eq!((f.min, f.max), (10, 10));
    }

    #[test]
    fn doubles_round_in_bankers_rounding_after_clamping() {
        let f = DoubleField { min: 0.0, max: 100.0, step: 1.0, precision: 0 };
        assert_eq!(f.enforce(2.5), 2.0);
        assert_eq!(f.enforce(3.5), 4.0);
        assert_eq!(f.enforce(100.5), 100.0);
        assert_eq!(f.parse("nan"), None);
        assert_eq!(f.parse("abc"), None);
        assert_eq!(DoubleField { precision: 2, ..f }.enforce(1.005), 1.0);
    }

    fn strip(start: &str) -> NumberDrag {
        let mut d = NumberDrag::press(NumberField::Int(int(0, 1000)), start, 0.0, 0.0);
        assert_eq!(d.moved(3.0, 0.0, Mods::default()), None);
        d
    }

    #[test]
    fn strip_directions() {
        let mods = Mods::default();
        let mut d = strip("100");
        // (5, -1): length rounds to 5, dx > dy, so it increases.
        assert_eq!(d.moved(8.0, -1.0, mods).as_deref(), Some("105"));
        let mut d = strip("100");
        // (-1, 5) decreases.
        assert_eq!(d.moved(2.0, 5.0, mods).as_deref(), Some("95"));
        let mut d = strip("100");
        // (3, 3): dx is not greater than dy, so it decreases by round(4.24) = 4.
        assert_eq!(d.moved(6.0, 3.0, mods).as_deref(), Some("96"));
    }

    #[test]
    fn strip_multiplies_by_the_tick_unless_a_modifier_is_held() {
        let field = NumberField::Int(IntField { min: 0, max: 1000, step: 5.0 });
        let mut d = NumberDrag::press(field, "100", 0.0, 0.0);
        d.moved(3.0, 0.0, Mods::default());
        assert_eq!(d.moved(5.0, 0.0, Mods::default()).as_deref(), Some("110"));
        assert_eq!(d.moved(7.0, 0.0, Mods { command: true, ..Mods::default() }).as_deref(), Some("112"));
        assert_eq!(d.moved(9.0, 0.0, Mods { shift: true, ..Mods::default() }).as_deref(), Some("114"));
        assert_eq!(d.hold_text(), "114");
    }

    #[test]
    fn strip_offset_keeps_counting_past_the_limit() {
        let mut d = NumberDrag::press(NumberField::Int(int(0, 10)), "9", 0.0, 0.0);
        d.moved(3.0, 0.0, Mods::default());
        assert_eq!(d.moved(8.0, 0.0, Mods::default()).as_deref(), Some("10"));
        // The offset is at 5, so coming back by 4 is still 10.
        assert_eq!(d.moved(4.0, 0.0, Mods::default()).as_deref(), Some("10"));
    }

    #[test]
    fn a_huge_unsigned_value_keeps_its_digits() {
        let field = NumberField::Int(IntField { min: 0, max: u64::MAX as i128, step: 1.0 });
        let mut d = NumberDrag::press(field, "18446744073709551610", 0.0, 0.0);
        d.moved(3.0, 0.0, Mods::default());
        assert_eq!(d.moved(5.0, 0.0, Mods::default()).as_deref(), Some("18446744073709551612"));
        assert_eq!(d.moved(30.0, 0.0, Mods::default()).as_deref(), Some("18446744073709551615"));
    }

    #[test]
    fn percent_write_back_is_one_outside_the_range() {
        assert_eq!(percent_from_slider(150.0), 1.0);
        assert_eq!(percent_from_slider(-5.0), 1.0);
        assert_eq!(percent_from_slider(50.0), 0.5);
        // In float, as the converter does.
        assert_eq!(percent_from_slider(33.0), (33.0f32 / 100.0) as f64);
    }

    #[test]
    fn slider_tooltips() {
        assert_eq!(slider_tooltip(SliderKind::Level, 0.29 * 100.0), "28");
        assert_eq!(slider_tooltip(SliderKind::Int, 2.5), "2");
        assert_eq!(slider_tooltip(SliderKind::Double, 3.5), "4");
        assert_eq!(slider_tooltip(SliderKind::Percent, 50.4), "50");
        assert_eq!(to_int32(2.5), 2);
        assert_eq!(slider_drag_tip(1234.0), "1,234");
    }

    #[test]
    fn snapping_chooses_the_closer_tick() {
        assert_eq!(snap_to_tick(10.4, 0.0, 100.0, 1.0, true), 10.0);
        assert_eq!(snap_to_tick(10.6, 0.0, 100.0, 1.0, true), 11.0);
        assert_eq!(snap_to_tick(10.5, 0.0, 100.0, 1.0, true), 11.0);
        assert_eq!(snap_to_tick(99.9, 0.0, 100.0, 1.0, true), 100.0);
        assert_eq!(snap_to_tick(37.0, 0.0, 200.0, 0.5, true), 37.0);
        assert_eq!(snap_to_tick(10.4, 0.0, 100.0, 1.0, false), 10.4);
    }
}
