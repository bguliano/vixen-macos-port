//! Port of Vixen.Core/Data/Value: the values intents carry.

use crate::color::{clamp, clip_value, Color, Hsv};
use std::sync::Arc;

/// `RGBValue`: a brightness-compensated colour.
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct RgbValue {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbValue {
    pub const BLACK: RgbValue = RgbValue { r: 0, g: 0, b: 0 };

    /// `new RGBValue(Color)`: zero alpha means off, regardless of the RGB components.
    #[inline]
    pub fn new(c: Color) -> RgbValue {
        if c.a != 0 {
            RgbValue { r: c.r, g: c.g, b: c.b }
        } else {
            RgbValue::BLACK
        }
    }

    /// `FullColor`: opaque, intensity already applied.
    #[inline]
    pub fn full_color(self) -> Color {
        Color::rgb(self.r, self.g, self.b)
    }

    /// `FullColorWithAlpha`: full-brightness colour with intensity moved to alpha.
    pub fn full_color_with_alpha(self) -> Color {
        let i = Hsv::v_from_rgb(self.r as f64, self.g as f64, self.b as f64);
        if i > 0.0 {
            if i < 1.0 {
                let mult = 1.0 / i;
                return Color::from_argb_i32(
                    (255.0 * i) as i32,
                    ((self.r as f64 * mult) as i32).min(255),
                    ((self.g as f64 * mult) as i32).min(255),
                    ((self.b as f64 * mult) as i32).min(255),
                );
            }
            return Color::argb(255, self.r, self.g, self.b);
        }
        Color::EMPTY
    }

    #[inline]
    pub fn intensity(self) -> f64 {
        Hsv::v_from_rgb(self.r as f64, self.g as f64, self.b as f64)
    }

    /// `RGBValue.ConvertToGrayscale`.
    pub fn convert_to_grayscale(c: Color) -> Color {
        let level = (c.r as f64 * 0.3 + c.g as f64 * 0.59 + c.b as f64 * 0.11) as u8;
        Color::rgb(level, level, level)
    }

    /// `RGBValue.GetGrayscaleLevel`.
    pub fn grayscale_level(c: Color) -> u8 {
        RgbValue::convert_to_grayscale(c).r
    }
}

/// `LightingValue`: a colour plus a separate intensity.
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct LightingValue {
    pub color: Color,
    pub intensity: f64,
}

impl LightingValue {
    /// `new LightingValue(Color, double)`: clamps the intensity.
    #[inline]
    pub fn new(color: Color, intensity: f64) -> LightingValue {
        LightingValue { color, intensity: clamp(intensity, 0.0, 1.0) }
    }

    /// `FullColor`.
    #[inline]
    pub fn full_color(self) -> Color {
        Color::from_rgb_i32(
            (self.color.r as f64 * self.intensity) as i32,
            (self.color.g as f64 * self.intensity) as i32,
            (self.color.b as f64 * self.intensity) as i32,
        )
    }

    /// `FullColorWithAlpha`.
    #[inline]
    pub fn full_color_with_alpha(self) -> Color {
        self.color.with_alpha((self.intensity * 255.0) as i32)
    }
}

/// `DiscreteValue`: a fixed colour on a single-colour channel, with intensity.
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct DiscreteValue {
    pub color: Color,
    pub intensity: f64,
}

impl DiscreteValue {
    /// `new DiscreteValue(Color, double)`: clips the intensity.
    #[inline]
    pub fn new(color: Color, intensity: f64) -> DiscreteValue {
        DiscreteValue { color, intensity: clip_value(intensity, 0.0, 1.0) }
    }

    /// `FullColor`.
    pub fn full_color(self) -> Color {
        let mut hsv = Hsv::from_color(self.color);
        hsv.set_v(hsv.v * self.intensity);
        hsv.to_color()
    }

    /// `FullColorWithAplha` (sic).
    #[inline]
    pub fn full_color_with_alpha(self) -> Color {
        Color::argb((self.intensity * 255.0) as u8, self.color.r, self.color.g, self.color.b)
    }
}

/// `FunctionIdentity`: which fixture function a range value drives.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum FunctionIdentity {
    Pan,
    Tilt,
    Zoom,
    Dim,
    Shutter,
    Gobo,
    Frost,
    Prism,
    OpenClosePrism,
    SpinColorWheel,
    Custom,
}

/// `FixtureIndexType`.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub enum FixtureIndexType {
    #[default]
    Custom,
    ShutterOpen,
    ShutterClosed,
    LampOn,
    LampOff,
    ColorWheel,
    Prism,
    PrismOpen,
    PrismClose,
    Strobe,
}

/// `Named8BitCommand<FixtureIndexType>`: an 8-bit value saying which fixture function it drives.
#[derive(Clone, PartialEq, Debug)]
pub struct NamedCommand {
    pub value: u8,
    pub index_type: FixtureIndexType,
    pub tag: Arc<str>,
    pub label: Arc<str>,
    pub range_minimum: u8,
    pub range_maximum: u8,
}

/// The `ICommand` types: raw values sent to a controller output.
#[derive(Clone, PartialEq, Debug)]
pub enum CommandValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Color(crate::Color),
    Str(Arc<str>),
    Named8(NamedCommand),
}

impl CommandValue {
    /// The 8-bit value of an 8-bit command, named or not.
    pub fn as_u8(&self) -> Option<u8> {
        match self {
            CommandValue::U8(v) => Some(*v),
            CommandValue::Named8(c) => Some(c.value),
            _ => None,
        }
    }
}

/// `RangeValue<FunctionIdentity>`: a 0..1 position for a fixture function.
#[derive(Clone, PartialEq, Debug)]
pub struct RangeValue {
    pub tag_type: FunctionIdentity,
    pub tag: Arc<str>,
    pub value: f64,
    pub label: Arc<str>,
}

impl RangeValue {
    pub fn new(tag_type: FunctionIdentity, tag: &str, value: f64, label: &str) -> RangeValue {
        RangeValue { tag_type, tag: Arc::from(tag), value, label: Arc::from(label) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_alpha_rgb_value_is_black() {
        assert_eq!(RgbValue::new(Color::TRANSPARENT), RgbValue::BLACK);
    }

    #[test]
    fn lighting_value_applies_intensity_with_truncation() {
        assert_eq!(LightingValue::new(Color::WHITE, 0.5).full_color(), Color::rgb(127, 127, 127));
        assert_eq!(LightingValue::new(Color::WHITE, 2.0).intensity, 1.0);
    }

    #[test]
    fn rgb_full_colour_with_alpha_restores_full_brightness() {
        let c = RgbValue { r: 128, g: 0, b: 0 }.full_color_with_alpha();
        assert_eq!(c.r, 255);
        assert_eq!(c.a, 128);
    }

    #[test]
    fn black_rgb_with_alpha_is_empty() {
        assert_eq!(RgbValue::BLACK.full_color_with_alpha(), Color::EMPTY);
    }
}
