//! Port of VixenModules.Effect.Border: a simple or advanced border, or a marching "marquee" of colour bands.

use crate::generated::border as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    supports_location: true,
    ported: true,
    ..gen::BASE
};

#[derive(Copy, Clone, PartialEq)]
enum Mode {
    Simple,
    Advanced,
    Marquee,
}

#[derive(Copy, Clone, PartialEq)]
enum GradMode {
    OverTime,
    AcrossElement,
    VerticalAcrossElement,
    DiagonalBottomTopElement,
    DiagonalTopBottomElement,
}

#[derive(Copy, Clone, PartialEq)]
enum Level {
    Level0,
    Level1,
    Level2,
}

fn mode_of(p: &Params) -> Mode {
    match p.enum_name("BorderMode") {
        "Advanced" => Mode::Advanced,
        "Marquee" => Mode::Marquee,
        _ => Mode::Simple,
    }
}

fn level_of(p: &Params) -> Level {
    match p.enum_name("RenderLevel") {
        "Level1" => Level::Level1,
        "Level2" => Level::Level2,
        _ => Level::Level0,
    }
}

/// `InitAllAttributes`, `UpdateBorderControlAttribute` and `UpdateMarqueeModeControlAttributes`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let mode = mode_of(p);
    let single = p.enum_name("BorderType") == "Single";
    let level0 = level_of(p) == Level::Level0;
    let locations = p.enum_name("TargetPositioning") == "Locations";
    let marquee = mode == Mode::Marquee;
    let advanced = mode == Mode::Advanced;
    let simple = mode == Mode::Simple;

    v.set("SimpleBorderWidth", simple);
    v.set("ThicknessCurve", advanced && single);
    v.set("TopThicknessCurve", advanced && !single);
    v.set("BottomThicknessCurve", advanced && !single);
    v.set("LeftThicknessCurve", advanced && !single);
    v.set("RightThicknessCurve", advanced && !single);
    v.set("BorderSizeCurve", advanced);
    v.set("BorderHeightCurve", advanced);
    v.set("BorderType", advanced);
    v.set("YOffsetCurve", advanced || marquee);
    v.set("XOffsetCurve", advanced || marquee);
    v.set("GradientMode", !marquee);
    v.set("Color", !marquee);
    v.set("Colors", marquee);
    v.set("RenderLevel", marquee);
    v.set("Thickness", marquee && level0);
    v.set("Stagger", marquee && level0 && p.int("Thickness") > 1);
    v.set("BandLength", marquee);
    v.set("SkipLength", marquee);
    v.set("Speed", marquee);
    v.set("Reverse", marquee);
    v.set("XScale", marquee);
    v.set("YScale", marquee);
    v.set("WrapX", marquee);
    v.set("WrapY", marquee);
    v.set("RenderScaleFactor", marquee && locations);
    // The constructor calls EnableTargetPositioning(true, true), and UpdateStringOrientationAttributes.
    v.set("TargetPositioning", true);
    v.set("StringOrientation", p.enum_name("TargetPositioning") == "Strings");
}

struct Border {
    mode: Mode,
    single_border: bool,
    grad_mode: GradMode,
    level: Level,
    locations: bool,
    simple_width: i32,
    thickness_curve: Curve,
    top_curve: Curve,
    bottom_curve: Curve,
    left_curve: Curve,
    right_curve: Curve,
    border_size_curve: Curve,
    border_height_curve: Curve,
    x_offset_curve: Curve,
    y_offset_curve: Curve,
    level_curve: Curve,
    color: ColorGradient,
    colors: Vec<ColorGradient>,
    marquee_thickness: i32,
    stagger: i32,
    band_length: i32,
    skip_length: i32,
    speed: Curve,
    reverse: bool,
    x_scale: i32,
    y_scale: i32,
    wrap_x: bool,
    wrap_y: bool,
    render_scale_factor_param: i32,
    // State
    min_buffer_size: f64,
    max_buffer_size: i32,
    buffer_ht: i32,
    buffer_wi: i32,
    render_scale_factor: i32,
    position_of_marquee: f64,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut e = Border {
        mode: mode_of(p),
        single_border: p.enum_name("BorderType") == "Single",
        grad_mode: match p.enum_name("GradientMode") {
            "AcrossElement" => GradMode::AcrossElement,
            "VerticalAcrossElement" => GradMode::VerticalAcrossElement,
            "DiagonalBottomTopElement" => GradMode::DiagonalBottomTopElement,
            "DiagonalTopBottomElement" => GradMode::DiagonalTopBottomElement,
            _ => GradMode::OverTime,
        },
        level: level_of(p),
        locations: TargetPositioning::from_name(p.enum_name("TargetPositioning")) == TargetPositioning::Locations,
        simple_width: p.int("SimpleBorderWidth"),
        thickness_curve: p.curve("ThicknessCurve"),
        top_curve: p.curve("TopThicknessCurve"),
        bottom_curve: p.curve("BottomThicknessCurve"),
        left_curve: p.curve("LeftThicknessCurve"),
        right_curve: p.curve("RightThicknessCurve"),
        border_size_curve: p.curve("BorderSizeCurve"),
        border_height_curve: p.curve("BorderHeightCurve"),
        x_offset_curve: p.curve("XOffsetCurve"),
        y_offset_curve: p.curve("YOffsetCurve"),
        level_curve: p.curve("LevelCurve"),
        color: p.gradient("Color"),
        colors: p.gradients("Colors"),
        marquee_thickness: p.int("Thickness"),
        stagger: p.int("Stagger"),
        band_length: p.int("BandLength"),
        skip_length: p.int("SkipLength"),
        speed: p.curve("Speed"),
        reverse: p.boolean("Reverse"),
        x_scale: p.int("XScale"),
        y_scale: p.int("YScale"),
        wrap_x: p.boolean("WrapX"),
        wrap_y: p.boolean("WrapY"),
        render_scale_factor_param: p.int("RenderScaleFactor"),
        min_buffer_size: 0.0,
        max_buffer_size: 0,
        buffer_ht: 0,
        buffer_wi: 0,
        render_scale_factor: 4,
        position_of_marquee: 0.0,
    };
    render_pixel_effect(ctx, p, &mut e)
}

/// The thickness and offset values `CalculatePixel` needs, computed once per frame.
struct Frame {
    thickness: i32,
    top: i32,
    bottom: i32,
    left: i32,
    right: i32,
    border_width: i32,
    border_height: i32,
    x_offset_adj: i32,
    y_offset_adj: i32,
    level: f64,
    color: Color,
}

impl Border {
    fn calculate_border_size(&self, pos: f64, info: &PixelInfo) -> f64 {
        scale_curve_to_value(self.border_size_curve.get_value(pos), 0.0, info.buffer_wi() as f64)
    }

    fn calculate_border_thickness(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.thickness_curve.get_value(pos), self.min_buffer_size, 2.0)
    }

    fn calculate_border_height(&self, pos: f64, info: &PixelInfo) -> f64 {
        scale_curve_to_value(self.border_height_curve.get_value(pos), 0.0, info.buffer_ht() as f64)
    }

    fn calculate_x_offset(&self, pos: f64) -> i32 {
        scale_curve_to_value(self.x_offset_curve.get_value(pos), 100.0, -100.0).round_ties_even() as i32
    }

    fn calculate_y_offset(&self, pos: f64) -> i32 {
        scale_curve_to_value(self.y_offset_curve.get_value(pos), 100.0, -100.0).round_ties_even() as i32
    }

    /// The per-frame values of `RenderEffectSimpleOrAdvancedBorder` (`by_location` false) and
    /// `RenderEffectSimpleOrAdvancedByLocation` (true), which differ in rounding and in the level scale.
    fn simple_frame(&self, frame: i32, info: &PixelInfo, by_location: bool) -> Frame {
        let interval_pos = info.interval_position(frame);
        let pos = interval_pos * 100.0;
        let (wi, ht) = (info.buffer_wi(), info.buffer_ht());

        // Deliberate quirk: the location version samples the level curve at 0..1 without dividing by 100.
        let level = if by_location { self.level_curve.get_value(interval_pos) } else { self.level_curve.get_value(pos) / 100.0 };
        let mut thickness = (self.calculate_border_thickness(pos) / 2.0).round_ties_even() as i32;
        let side = |curve: &Curve, dim: i32| -> i32 {
            let v = curve.get_value(pos) * dim as f64 / 100.0;
            if by_location {
                v as i32
            } else {
                v.round_ties_even() as i32
            }
        };
        let mut top = side(&self.top_curve, ht);
        let mut bottom = side(&self.bottom_curve, ht);
        let mut left = side(&self.left_curve, wi);
        let mut right = side(&self.right_curve, wi);
        let mut border_height = (self.calculate_border_height(pos, info) as i32) / 2;
        let mut border_width = (self.calculate_border_size(pos, info) / 2.0) as i32;
        let mut x_offset_adj = self.calculate_x_offset(pos).wrapping_mul(wi - border_width) / 100;
        let mut y_offset_adj = self.calculate_y_offset(pos).wrapping_mul(ht - border_height) / 100;
        let color = self.color.get_color_at(interval_pos);

        if self.mode == Mode::Simple {
            thickness = self.simple_width;
            border_width = 0;
            border_height = 0;
            if by_location {
                x_offset_adj = 0;
                y_offset_adj = 0;
            }
        } else if self.single_border {
            right = thickness;
            top = thickness;
            left = thickness;
            bottom = thickness;
        }
        Frame { thickness, top, bottom, left, right, border_width, border_height, x_offset_adj, y_offset_adj, level, color }
    }

    fn calculate_pixel<B: PixelBuffer>(&self, x_in: i32, y_in: i32, f: &Frame, info: &PixelInfo, buf: &mut B) {
        let (y_coord, x_coord) = (y_in, x_in);
        let (mut x, mut y) = (x_in, y_in);
        let (buffer_wi, buffer_ht) = (info.buffer_wi(), info.buffer_ht());
        if self.locations {
            // Flip and offset the coordinates so the location version acts like the string version.
            y = ((info.buffer_ht_offset() - y) + (buffer_ht - 1 + info.buffer_ht_offset())).abs();
            y -= info.buffer_ht_offset();
            x -= info.buffer_wi_offset();
        }
        x -= f.x_offset_adj;
        y -= f.y_offset_adj;

        let (bw, bh) = (f.border_width, f.border_height);
        let inside = x >= bw && y < buffer_ht - bh && y >= bh && x < buffer_wi - bw;
        let on_border = if self.single_border || self.mode == Mode::Simple {
            y < bh + f.thickness || y >= buffer_ht - bh - f.thickness || x < bw + f.thickness || x >= buffer_wi - bw - f.thickness
        } else {
            y < bh + f.bottom || y >= buffer_ht - bh - f.top || x < bw + f.left || x >= buffer_wi - bw - f.right
        };
        if on_border && inside {
            let c = self.get_color(x, y, f.color, f.level, buffer_ht, buffer_wi);
            buf.set_pixel(x_coord, y_coord, c);
        }
    }

    /// `GetColor`, for Simple and Advanced mode.
    fn get_color(&self, x: i32, y: i32, mut color: Color, level: f64, ht: i32, wi: i32) -> Color {
        let (xf, yf, htf, wif) = (x as f64, y as f64, ht as f64, wi as f64);
        match self.grad_mode {
            GradMode::OverTime => {}
            GradMode::AcrossElement => color = self.color.get_color_at(100.0 / wif * xf / 100.0),
            GradMode::VerticalAcrossElement => color = self.color.get_color_at(100.0 / htf * (htf - yf) / 100.0),
            GradMode::DiagonalTopBottomElement => {
                color = self.color.get_color_at((100.0 / htf * (htf - yf) + 100.0 / wif * xf) / 200.0)
            }
            GradMode::DiagonalBottomTopElement => color = self.color.get_color_at((100.0 / htf * yf + 100.0 / wif * xf) / 200.0),
        }
        if level < 1.0 {
            let mut hsv = Hsv::from_color(color);
            hsv.set_v(hsv.v * level);
            color = hsv.to_color();
        }
        color
    }

    // ---- Marquee ----

    fn marquee_band_length(&self) -> i32 {
        let size = ((self.max_buffer_size as f64 * self.band_length as f64) / 100.0) as i32;
        if size == 0 {
            1
        } else {
            size
        }
    }

    fn marquee_skip_length(&self) -> i32 {
        (self.max_buffer_size as f64 * self.skip_length as f64 / 100.0) as i32
    }

    fn marquee_thickness(&self) -> i32 {
        let t = (self.max_buffer_size as f64 * self.marquee_thickness as f64 / 100.0) as i32;
        if t == 0 {
            1
        } else {
            t
        }
    }

    fn marquee_stagger(&self) -> i32 {
        (self.max_buffer_size as f64 * self.stagger as f64 / 100.0) as i32
    }

    fn color_intensity(intensity: f64, color: Color) -> Color {
        if intensity < 1.0 {
            let mut hsv = Hsv::from_color(color);
            hsv.set_v(hsv.v * intensity);
            return hsv.to_color();
        }
        color
    }

    /// `UpdateMarqueeColor`.
    fn update_marquee_color(position: &mut i32, band_color: &mut i32, colorcnt: i32, color_size: i32, shift: i32) {
        if shift == 0 {
            return;
        }
        if shift > 0 {
            let mut index = 0;
            while index < shift {
                *position += 1;
                if *position >= color_size {
                    *band_color += 1;
                    *band_color %= colorcnt;
                    *position = 0;
                }
                index += 1;
            }
        } else {
            let mut index = 0;
            while index > shift {
                *position -= 1;
                if *position < 0 {
                    *band_color += 1;
                    *band_color %= colorcnt;
                    *position = color_size - 1;
                }
                index -= 1;
            }
        }
    }

    /// `ProcessPixel`. Wrapping uses the display element's size, even when drawing into a virtual buffer.
    fn process_pixel(buf: &mut PixelFrameBuffer, info: &PixelInfo, x_pos: i32, y_pos: i32, color: Color, wrap_x: bool, wrap_y: bool) {
        let mut xv = x_pos;
        if wrap_x && info.buffer_wi() != 0 {
            xv %= info.buffer_wi();
            xv = if xv >= 0 { xv } else { info.buffer_wi() + xv };
        }
        let mut yv = y_pos;
        if wrap_y && info.buffer_ht() != 0 {
            yv %= info.buffer_ht();
            yv = if yv >= 0 { yv } else { info.buffer_ht() + yv };
        }
        buf.set_pixel(xv, yv, color);
    }

    /// `RenderEffectMarquee`.
    fn render_marquee(&mut self, frame: i32, buf: &mut PixelFrameBuffer, info: &PixelInfo, buffer_wi: i32, buffer_ht: i32) {
        let interval_pos = info.interval_position(frame);
        let pos = interval_pos * 100.0;
        // Deliberate quirk: the level is sampled at 0..1, not 0..100.
        let intensity = self.level_curve.get_value(interval_pos) / 100.0;

        let band_size = self.marquee_band_length();
        let skip_size = self.marquee_skip_length();
        let thickness = self.marquee_thickness();
        let stagger = self.marquee_stagger();
        // Scaled so movement stays constant across update intervals.
        let m_speed = scale_curve_to_value(self.speed.get_value(pos), 25.0, 0.0) * info.frame_time as f64 / 50.0;
        let m_start = 0;

        let (mut x_scale, mut y_scale) = (self.x_scale, self.y_scale);
        let mut xc_adj = scale_curve_to_value(self.x_offset_curve.get_value(pos), 100.0, -100.0).round_ties_even() as i32;
        let mut yc_adj = scale_curve_to_value(self.y_offset_curve.get_value(pos), 100.0, -100.0).round_ties_even() as i32;
        let reverse_dir = self.reverse;
        let (mut wrap_x, mut wrap_y) = (self.wrap_x, self.wrap_y);

        if self.level != Level::Level0 {
            xc_adj = 0;
            yc_adj = 0;
            x_scale = 100;
            y_scale = 100;
            wrap_x = false;
            wrap_y = false;
        }

        let colorcnt = self.colors.len() as i32;
        let color_size = band_size + skip_size;
        let repeat_size = color_size.wrapping_mul(colorcnt);
        // The C# divides by repeat_size, which throws with no colours; the effect stays dark.
        if repeat_size <= 0 || color_size <= 0 {
            return;
        }

        self.position_of_marquee += m_speed;
        let x = (self.position_of_marquee / 5.0) as i32;

        let mut corner_x1 = 0;
        let mut corner_y1 = 0;
        let mut corner_x2 = (buffer_wi as f64 * x_scale as f64 / 100.0 - 1.0).round_ties_even() as i32;
        let mut corner_y2 = (buffer_ht as f64 * y_scale as f64 / 100.0 - 1.0).round_ties_even() as i32;
        let sign = if reverse_dir { -1 } else { 1 };

        let xoffset_adj = ((xc_adj as f64 * buffer_wi as f64) / 100.0) as i32;
        let yoffset_adj = ((yc_adj as f64 * buffer_ht as f64) / 100.0) as i32;

        // Sampling once per colour keeps the per-pixel loops free of gradient lookups.
        let sampled: Vec<Color> =
            self.colors.iter().map(|g| Self::color_intensity(intensity, g.get_color_at(interval_pos))).collect();
        let pick = |current_pos: i32, current_color: i32| -> Color {
            if current_pos < band_size {
                sampled.get(current_color as usize).copied().unwrap_or(Color::TRANSPARENT)
            } else {
                Color::TRANSPARENT
            }
        };

        for thick in 0..thickness {
            let mut current_color = ((x + m_start) % repeat_size) / color_size;
            let mut current_pos = ((x + m_start) % repeat_size) % color_size;
            if sign < 0 {
                current_color = colorcnt - current_color - 1;
            }

            if corner_y2 != corner_y1 {
                Self::update_marquee_color(&mut current_pos, &mut current_color, colorcnt, color_size, thick * (stagger + 1) * sign);
                for x_pos in corner_x1..=corner_x2 {
                    let color = pick(current_pos, current_color);
                    Self::process_pixel(buf, info, x_pos + xoffset_adj, corner_y2 + yoffset_adj, color, wrap_x, wrap_y);
                    Self::update_marquee_color(&mut current_pos, &mut current_color, colorcnt, color_size, sign);
                }
                Self::update_marquee_color(&mut current_pos, &mut current_color, colorcnt, color_size, thick * 2 * sign);
                let mut y_pos = corner_y2;
                while y_pos >= corner_y1 {
                    let color = pick(current_pos, current_color);
                    Self::process_pixel(buf, info, corner_x2 + xoffset_adj, y_pos + yoffset_adj, color, wrap_x, wrap_y);
                    Self::update_marquee_color(&mut current_pos, &mut current_color, colorcnt, color_size, sign);
                    y_pos -= 1;
                }
            }
            Self::update_marquee_color(&mut current_pos, &mut current_color, colorcnt, color_size, thick * 2 * sign);
            let mut x_pos = corner_x2;
            while x_pos >= corner_x1 {
                let color = pick(current_pos, current_color);
                Self::process_pixel(buf, info, x_pos + xoffset_adj, corner_y1 + yoffset_adj, color, wrap_x, wrap_y);
                Self::update_marquee_color(&mut current_pos, &mut current_color, colorcnt, color_size, sign);
                x_pos -= 1;
            }

            if corner_y2 != corner_y1 {
                Self::update_marquee_color(&mut current_pos, &mut current_color, colorcnt, color_size, thick * 2 * sign);
                for y_pos in corner_y1..corner_y2 {
                    let color = pick(current_pos, current_color);
                    Self::process_pixel(buf, info, corner_x1 + xoffset_adj, y_pos + yoffset_adj, color, wrap_x, wrap_y);
                    Self::update_marquee_color(&mut current_pos, &mut current_color, colorcnt, color_size, sign);
                }
            }

            corner_x1 += 1;
            corner_y1 += 1;
            corner_x2 -= 1;
            corner_y2 -= 1;
        }
    }

    /// `RenderEffectMarqueeSingleString`: renders one long strand and lays it out zig-zag or typewriter style.
    fn render_marquee_single_string(&mut self, frame: i32, buf: &mut PixelFrameBuffer, info: &PixelInfo) {
        let buffer_wi = (self.buffer_wi as f64 * self.x_scale as f64 / 100.0).round_ties_even() as i32;
        let buffer_ht = (self.buffer_ht as f64 * self.y_scale as f64 / 100.0).round_ties_even() as i32;
        let length = buffer_wi.wrapping_mul(buffer_ht).max(0);
        let mut strand = PixelFrameBuffer::new(length, 1);
        self.render_marquee(frame, &mut strand, info, length, 1);

        let pos = info.interval_position(frame) * 100.0;
        let x_off = self.calculate_x_offset(pos).wrapping_mul(self.buffer_wi) / 100;
        let y_off = self.calculate_y_offset(pos).wrapping_mul(self.buffer_ht) / 100;
        let get = |i: i32| if i >= 0 && i < strand.width() { strand.get_color_at(i, 0) } else { Color::EMPTY };

        let (wx, wy) = (self.wrap_x, self.wrap_y);
        if self.level == Level::Level1 {
            // Zig zag: even rows left to right, odd rows right to left.
            let mut index = 0;
            for y in 0..buffer_ht {
                for x in 0..buffer_wi {
                    if y % 2 == 0 {
                        Self::process_pixel(buf, info, x + x_off, y + y_off, get(index), wx, wy);
                    } else {
                        Self::process_pixel(buf, info, buffer_wi - x - 1 + x_off, y + y_off, get(index), wx, wy);
                    }
                    index += 1;
                }
            }
        } else {
            // Typewriter: returns to the left side for each row.
            let mut index = 0;
            for x in 0..buffer_wi {
                for y in 0..buffer_ht {
                    Self::process_pixel(buf, info, x + x_off, y + y_off, get(index), wx, wy);
                    index += 1;
                }
            }
        }
    }
}

impl PixelEffect for Border {
    fn setup_render(&mut self, info: &PixelInfo) {
        let (wi, ht) = (info.buffer_wi(), info.buffer_ht());
        self.max_buffer_size = ht.max(wi);
        self.min_buffer_size = ht.min(wi) as f64;
        self.position_of_marquee = 0.0;
        if self.locations {
            self.render_scale_factor = self.render_scale_factor_param;
            // A scale factor of 0 would divide by zero in the C#; treat it as 1.
            let f = self.render_scale_factor.max(1);
            self.buffer_wi = wi / f;
            self.buffer_ht = ht / f;
            if ht % f != 0 {
                self.buffer_ht += 1;
            }
            if wi % f != 0 {
                self.buffer_wi += 1;
            }
            self.render_scale_factor = f;
        } else {
            self.buffer_wi = wi;
            self.buffer_ht = ht;
        }
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        if self.mode != Mode::Marquee {
            let f = self.simple_frame(frame, info, false);
            for x in 0..info.buffer_wi() {
                for y in 0..info.buffer_ht() {
                    self.calculate_pixel(x, y, &f, info, buf);
                }
            }
        } else if self.level == Level::Level0 {
            let (wi, ht) = (self.buffer_wi, self.buffer_ht);
            self.render_marquee(frame, buf, info, wi, ht);
        } else {
            self.render_marquee_single_string(frame, buf, info);
        }
    }

    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        if self.mode != Mode::Marquee {
            let locations = buf.element_locations().to_vec();
            for frame in 0..num_frames {
                buf.current_frame = frame;
                let f = self.simple_frame(frame, info, true);
                for l in &locations {
                    self.calculate_pixel(l.x, l.y, &f, info, buf);
                }
            }
        } else {
            let locations = buf.element_locations().to_vec();
            for frame in 0..num_frames {
                buf.current_frame = frame;
                let mut virt = PixelFrameBuffer::new(self.buffer_wi, self.buffer_ht);
                self.render_effect(frame, info, &mut virt);
                for l in &locations {
                    // Flip and offset, then scale the location down to the virtual buffer.
                    let ht = info.buffer_ht();
                    let mut y = ((info.buffer_ht_offset() - l.y) + (ht - 1 + info.buffer_ht_offset())).abs();
                    y -= info.buffer_ht_offset();
                    let x = l.x - info.buffer_wi_offset();
                    let (sx, sy) = (x / self.render_scale_factor, y / self.render_scale_factor);
                    let c = if sx >= 0 && sy >= 0 && sx < virt.width() && sy < virt.height() {
                        virt.get_color_at(sx, sy)
                    } else {
                        Color::EMPTY
                    };
                    buf.set_pixel(l.x, l.y, c);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use vixen_effects::testing;

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        testing::assert_produces_light(&DESCRIPTOR);
    }

    #[test]
    fn marquee_smoke() {
        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 8, 12);
        for level in ["Level0", "Level1", "Level2"] {
            let out = testing::render_with(&DESCRIPTOR, &tree, &[m], 1000, |p| {
                p.set("BorderMode", ParamValue::Enum("Marquee".into()));
                p.set("RenderLevel", ParamValue::Enum(level.into()));
            });
            assert!(lit_count(&tree, m, &out, 0) > 0, "{level}");
        }
    }

    #[test]
    fn simple_border_one_pixel_outline() {
        // Simple mode, width 1, on a 6x6 matrix: only the outer ring is lit, in the gradient's colour
        // at the start of the effect (red).
        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 6, 6);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[m], 500, |_| {});
        assert_eq!(color_at(&tree, m, &out, 0, 3, 0), (255, 0, 0));
        assert_eq!(color_at(&tree, m, &out, 5, 0, 0), (255, 0, 0));
        assert_eq!(color_at(&tree, m, &out, 3, 3, 0), (0, 0, 0));
        assert_eq!(lit_count(&tree, m, &out, 0), 20);
    }

    #[test]
    fn simple_mode_visibility() {
        let p = DESCRIPTOR.new_params();
        let vis: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(vis.contains(&"SimpleBorderWidth"));
        assert!(!vis.contains(&"Colors"));
        let mut p = DESCRIPTOR.new_params();
        p.set("BorderMode", ParamValue::Enum("Marquee".into()));
        let vis: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(vis.contains(&"Colors") && !vis.contains(&"SimpleBorderWidth"));
    }

    #[test]
    fn marquee_band_colours_follow_palette() {
        // Level 0 marquee, thickness 1 on a 1x? strip is awkward, so check the update helper directly:
        // moving 3 pixels with a band of 3 and no skip changes to the next colour.
        let (mut pos, mut col) = (0, 0);
        Border::update_marquee_color(&mut pos, &mut col, 3, 3, 3);
        assert_eq!((pos, col), (0, 1));
        Border::update_marquee_color(&mut pos, &mut col, 3, 3, -1);
        assert_eq!((pos, col), (2, 2));
    }
}
