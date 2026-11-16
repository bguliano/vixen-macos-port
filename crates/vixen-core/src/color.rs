//! Port of System.Drawing.Color (the parts Vixen uses) and Vixen.Core/Common/ColorSpaces.cs.

/// ARGB colour, value-compatible with System.Drawing.Color.
///
/// .NET equality also compares a "known colour" name; Vixen never relies on that distinction
/// except `Color.Empty` vs. `Color.FromArgb(0, 0, 0, 0)`, which are the same value here.
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct Color {
    pub a: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const EMPTY: Color = Color { a: 0, r: 0, g: 0, b: 0 };
    /// `Color.Transparent` in .NET is white with zero alpha, not black.
    pub const TRANSPARENT: Color = Color { a: 0, r: 255, g: 255, b: 255 };
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const RED: Color = Color::rgb(255, 0, 0);
    /// `Color.Lime` is (0,255,0); `Color.Green` in .NET is (0,128,0).
    pub const LIME: Color = Color::rgb(0, 255, 0);
    pub const GREEN: Color = Color::rgb(0, 128, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const YELLOW: Color = Color::rgb(255, 255, 0);
    pub const CYAN: Color = Color::rgb(0, 255, 255);
    pub const MAGENTA: Color = Color::rgb(255, 0, 255);
    pub const ORANGE: Color = Color::rgb(255, 165, 0);
    pub const GRAY: Color = Color::rgb(128, 128, 128);
    pub const DARK_GRAY: Color = Color::rgb(169, 169, 169);
    pub const DIM_GRAY: Color = Color::rgb(105, 105, 105);

    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { a: 255, r, g, b }
    }

    #[inline]
    pub const fn argb(a: u8, r: u8, g: u8, b: u8) -> Color {
        Color { a, r, g, b }
    }

    /// `Color.FromArgb(int, int, int)`. .NET throws when out of range; ported effects never hit
    /// that on valid data, so this clamps rather than panicking mid-render.
    #[inline]
    pub fn from_rgb_i32(r: i32, g: i32, b: i32) -> Color {
        Color { a: 255, r: clamp_u8(r), g: clamp_u8(g), b: clamp_u8(b) }
    }

    /// `Color.FromArgb(int, int, int, int)`.
    #[inline]
    pub fn from_argb_i32(a: i32, r: i32, g: i32, b: i32) -> Color {
        Color { a: clamp_u8(a), r: clamp_u8(r), g: clamp_u8(g), b: clamp_u8(b) }
    }

    /// `Color.FromArgb(int alpha, Color baseColor)`.
    #[inline]
    pub fn with_alpha(self, a: i32) -> Color {
        Color { a: clamp_u8(a), ..self }
    }

    /// `Color.ToArgb()`.
    #[inline]
    pub fn to_argb(self) -> i32 {
        i32::from_ne_bytes(self.to_argb_u32().to_ne_bytes())
    }

    #[inline]
    pub fn to_argb_u32(self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
    }

    /// `Color.FromArgb(int argb)`.
    #[inline]
    pub fn from_argb_u32(v: u32) -> Color {
        Color { a: (v >> 24) as u8, r: (v >> 16) as u8, g: (v >> 8) as u8, b: v as u8 }
    }

    /// Port of `Extensions.Combine`: highest-wins per component, opaque result.
    #[inline]
    pub fn combine(self, other: Color) -> Color {
        Color::rgb(self.r.max(other.r), self.g.max(other.g), self.b.max(other.b))
    }

    /// `Color.GetBrightness()` (HSL lightness).
    pub fn brightness(self) -> f32 {
        let r = self.r as i32;
        let g = self.g as i32;
        let b = self.b as i32;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        (max + min) as f32 / (255 * 2) as f32
    }

    /// `Color.GetHue()` in degrees.
    pub fn hue(self) -> f32 {
        let (r, g, b) = (self.r as i32, self.g as i32, self.b as i32);
        if r == g && g == b {
            return 0.0;
        }
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = (max - min) as f32;
        let mut hue = if r == max {
            (g - b) as f32 / delta
        } else if g == max {
            (b - r) as f32 / delta + 2.0
        } else {
            (r - g) as f32 / delta + 4.0
        };
        hue *= 60.0;
        if hue < 0.0 {
            hue += 360.0;
        }
        hue
    }

    /// `Color.GetSaturation()` (HSL saturation).
    pub fn saturation(self) -> f32 {
        let (r, g, b) = (self.r as i32, self.g as i32, self.b as i32);
        if r == g && g == b {
            return 0.0;
        }
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let div = if max + min <= 255 { max + min } else { 255 * 2 - max - min };
        (max - min) as f32 / div as f32
    }
}

#[inline]
fn clamp_u8(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

/// `XYZ.ClipValue`.
#[inline]
pub fn clip_value(value: f64, min: f64, max: f64) -> f64 {
    if value.is_nan() || value == f64::NEG_INFINITY || value < min {
        return min;
    }
    if value == f64::INFINITY || value > max {
        return max;
    }
    value
}

/// CIE XYZ, as stored by gradient colour points.
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct Xyz {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Xyz {
    pub fn new(x: f64, y: f64, z: f64) -> Xyz {
        Xyz {
            x: clip_value(x, 0.0, 95.047),
            y: clip_value(y, 0.0, 100.000),
            z: clip_value(z, 0.0, 108.883),
        }
    }

    pub fn from_rgb(value: Rgb) -> Xyz {
        let r = gamma_correction(value.r);
        let g = gamma_correction(value.g);
        let b = gamma_correction(value.b);
        Xyz::new(
            r * 41.24 + g * 35.76 + b * 18.05,
            r * 21.26 + g * 71.52 + b * 7.22,
            r * 1.93 + g * 11.92 + b * 95.05,
        )
    }

    pub fn from_color(c: Color) -> Xyz {
        Xyz::from_rgb(Rgb::from_color(c))
    }

    pub fn to_rgb(self) -> Rgb {
        let r = invert_gamma(self.x * 0.032406 + self.y * -0.015372 + self.z * -0.004986);
        let g = invert_gamma(self.x * -0.009689 + self.y * 0.018758 + self.z * 0.000415);
        let b = invert_gamma(self.x * 0.000557 + self.y * -0.002040 + self.z * 0.010570);
        Rgb::new(r, g, b)
    }

    /// `xyz.ToRGB().ToArgb()`, used everywhere gradients compare colours.
    pub fn to_color(self) -> Color {
        self.to_rgb().to_color()
    }
}

#[inline]
fn gamma_correction(value: f64) -> f64 {
    if value > 0.04045 {
        ((value + 0.055) / 1.055).powf(2.4)
    } else {
        value / 12.92
    }
}

#[inline]
fn invert_gamma(value: f64) -> f64 {
    if value > 0.0031308 {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    } else {
        12.92 * value
    }
}

/// Floating point RGB in 0..1.
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct Rgb {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl Rgb {
    pub fn new(r: f64, g: f64, b: f64) -> Rgb {
        Rgb {
            r: clip_value(r, 0.0, 1.0),
            g: clip_value(g, 0.0, 1.0),
            b: clip_value(b, 0.0, 1.0),
        }
    }

    pub fn from_color(c: Color) -> Rgb {
        Rgb::new(c.r as f64 / 255.0, c.g as f64 / 255.0, c.b as f64 / 255.0)
    }

    /// `RGB.ToArgb()`. C# `Math.Round` defaults to banker's rounding.
    pub fn to_color(self) -> Color {
        Color::rgb(
            (255.0 * self.r).round_ties_even() as u8,
            (255.0 * self.g).round_ties_even() as u8,
            (255.0 * self.b).round_ties_even() as u8,
        )
    }
}

/// HSV with every component in 0..1.
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct Hsv {
    pub h: f64,
    pub s: f64,
    pub v: f64,
}

impl Hsv {
    pub fn new(h: f64, s: f64, v: f64) -> Hsv {
        Hsv {
            h: clip_value(h, 0.0, 1.0),
            s: clip_value(s, 0.0, 1.0),
            v: clip_value(v, 0.0, 1.0),
        }
    }

    /// `HSV.VFromRgb`.
    #[inline]
    pub fn v_from_color(c: Color) -> f64 {
        c.r.max(c.g).max(c.b) as f64 / 255.0
    }

    /// `HSV.VFromRgb(r, b, g)`.
    #[inline]
    pub fn v_from_rgb(r: f64, g: f64, b: f64) -> f64 {
        r.max(g).max(b) / 255.0
    }

    #[inline]
    pub fn from_color(c: Color) -> Hsv {
        Hsv::from_rgb(c.r as f64, c.g as f64, c.b as f64)
    }

    /// `HSV.FromRGB(double, double, double)` with components on a 255 scale.
    /// The result is deliberately not clipped, matching the C# field initialiser.
    pub fn from_rgb(mut r: f64, mut g: f64, mut b: f64) -> Hsv {
        let mut k = 0.0;
        if g < b {
            std::mem::swap(&mut g, &mut b);
            k = -1.0;
        }
        let mut min_gb = b;
        if r < g {
            std::mem::swap(&mut r, &mut g);
            k = -0.3333333333333 - k;
            min_gb = g.min(b);
        }
        let chroma = r - min_gb;
        Hsv {
            v: r / 255.0,
            h: (k + (g - b) / (6.0 * chroma + 1e-20)).abs(),
            s: chroma / (r + 1e-20),
        }
    }

    /// `hsv.V = value` (the property setter clamps).
    #[inline]
    pub fn set_v(&mut self, v: f64) {
        self.v = clamp(v, 0.0, 1.0);
    }

    #[inline]
    pub fn set_s(&mut self, s: f64) {
        self.s = clamp(s, 0.0, 1.0);
    }

    #[inline]
    pub fn set_h(&mut self, h: f64) {
        self.h = clamp(h, 0.0, 1.0);
    }

    #[inline]
    pub fn to_color(self) -> Color {
        Hsv::to_rgb(self.h, self.s, self.v)
    }

    /// `HSV.ToRGB(hue, saturation, value)`.
    pub fn to_rgb(hue: f64, saturation: f64, value: f64) -> Color {
        if saturation == 0.0 {
            let c = (value * 255.0) as i32;
            return Color::from_rgb_i32(c, c, c);
        }
        let mut h = hue * 6.0;
        if h == 6.0 {
            h = 0.0;
        }
        let i = h.floor() as i32;
        let f = h - i as f64;
        let var1 = value * (1.0 - saturation);
        let (r, g, b) = match i {
            0 => (value, value * (1.0 - saturation * (1.0 - f)), var1),
            1 => (value * (1.0 - saturation * f), value, var1),
            2 => (var1, value, value * (1.0 - saturation * (1.0 - f))),
            3 => (var1, value * (1.0 - saturation * f), value),
            4 => (value * (1.0 - saturation * (1.0 - f)), var1, value),
            _ => (value, var1, value * (1.0 - saturation * f)),
        };
        Color::from_rgb_i32((r * 255.0) as i32, (g * 255.0) as i32, (b * 255.0) as i32)
    }
}

/// `HSV.Clamp`.
#[inline]
pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xyz_round_trips_primary_colours() {
        for c in [Color::rgb(255, 0, 0), Color::rgb(0, 255, 0), Color::rgb(0, 0, 255), Color::WHITE, Color::BLACK] {
            assert_eq!(Xyz::from_color(c).to_color(), c);
        }
    }

    #[test]
    fn hsv_round_trip() {
        let c = Color::rgb(200, 100, 50);
        let hsv = Hsv::from_color(c);
        let back = hsv.to_color();
        assert!((back.r as i32 - c.r as i32).abs() <= 1);
        assert!((back.g as i32 - c.g as i32).abs() <= 1);
        assert!((back.b as i32 - c.b as i32).abs() <= 1);
    }

    #[test]
    fn dotnet_named_colour_values() {
        assert_eq!(Color::TRANSPARENT.to_argb_u32(), 0x00FF_FFFF);
        assert_eq!(Color::EMPTY.to_argb_u32(), 0);
        assert_eq!(Color::RED.to_argb_u32(), 0xFFFF_0000);
    }

    #[test]
    fn combine_is_highest_wins() {
        assert_eq!(Color::rgb(10, 200, 30).combine(Color::rgb(20, 100, 30)), Color::rgb(20, 200, 30));
    }
}
