//! Port of VixenModules.Effect.CountDown: a countdown (or count-up) number drawn as gradient text, with optional
//! movement, fade, size change and a "film spinner" overlay.
//!
//! The embedded fonts in Vixen.Core/Fonts are never used by this effect (it always draws with the user's font),
//! so none are bundled.

use crate::generated::count_down as gen;
use crate::text::{adjusted_font, blend_mask, blend_text};
use crate::textdraw::{measure, Brush, Canvas, GradMode};
use vixen_core::gradient::ColorPoint;
use vixen_core::{ColorGradient, Xyz};
use vixen_effects::gfx::Graphics;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    param_changed: Some(param_changed),
    supports_location: true,
    force_visual: true,
    visual: Some(visual),
    ported: true,
    ..gen::BASE
};

// CountDownDirection ordinals: Left, Right, Up, Down, Rotate, None, Random.
const D_LEFT: i32 = 0;
const D_RIGHT: i32 = 1;
const D_UP: i32 = 2;
const D_DOWN: i32 = 3;
const D_ROTATE: i32 = 4;
const D_NONE: i32 = 5;
const D_RANDOM: i32 = 6;

fn direction_ord(name: &str) -> i32 {
    match name {
        "Left" => D_LEFT,
        "Right" => D_RIGHT,
        "Up" => D_UP,
        "Down" => D_DOWN,
        "Rotate" => D_ROTATE,
        "None" => D_NONE,
        "Random" => D_RANDOM,
        _ => D_LEFT,
    }
}

/// The `CountDownTime` setter: anything but a whole number becomes "0".
fn param_changed(name: &str, p: &mut Params) {
    if name == "CountDownTime" && p.string("CountDownTime").trim().parse::<i32>().is_err() {
        p.set("CountDownTime", ParamValue::Str("0".to_string()));
    }
}

/// `UpdateAllAttributes`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let dir = direction_ord(p.enum_name("Direction"));
    v.set("XOffsetCurve", !(dir == D_LEFT || dir == D_RIGHT));
    v.set("YOffsetCurve", !(dir == D_UP || dir == D_DOWN));
    v.set("AngleCurve", dir == D_ROTATE);
    let moving = dir != D_NONE && dir != D_ROTATE;
    v.set("Speed", moving);
    v.set("CenterStop", moving);
    v.set("PerIteration", moving);
    v.set("CountDownTime", p.enum_name("CountDownType") != "Effect");
    v.set("FontScaleCurve", p.enum_name("SizeMode") == "None");
    let spinner = p.boolean("Spinner");
    v.set("SpinnerColors", spinner);
    v.set("SpinnerGradientMode", spinner);
    v.set("TargetPositioning", true);
    v.set("StringOrientation", p.enum_name("TargetPositioning") == "Strings");
}

fn grad_mode_of(name: &str) -> (GradMode, bool) {
    match name {
        "AcrossText" => (GradMode::Horizontal, false),
        "AcrossElement" => (GradMode::Horizontal, true),
        "VerticalAcrossText" => (GradMode::Vertical, false),
        "VerticalAcrossElement" => (GradMode::Vertical, true),
        "DiagonalAcrossText" => (GradMode::ForwardDiagonal, false),
        "DiagonalAcrossElement" => (GradMode::ForwardDiagonal, true),
        "BackwardDiagonalAcrossText" => (GradMode::BackwardDiagonal, false),
        "BackwardDiagonalAcrossElement" => (GradMode::BackwardDiagonal, true),
        _ => (GradMode::Horizontal, true),
    }
}

/// `CreateCrossHashBrush`: the mode follows the gradient mode, with backward diagonal as the fallback.
fn cross_hash_mode(name: &str) -> GradMode {
    match name {
        "AcrossElement" | "AcrossText" => GradMode::Horizontal,
        "VerticalAcrossElement" | "VerticalAcrossText" => GradMode::Vertical,
        "DiagonalAcrossElement" | "DiagonalAcrossText" => GradMode::ForwardDiagonal,
        _ => GradMode::BackwardDiagonal,
    }
}

#[derive(Copy, Clone, PartialEq)]
enum Fade {
    None,
    In,
    Out,
    InOut,
}

#[derive(Copy, Clone, PartialEq)]
enum SizeMode {
    None,
    Shrink,
    Grow,
}

#[derive(Copy, Clone, PartialEq)]
enum CountType {
    Effect,
    CountDown,
    CountUp,
}

struct CountDown {
    gradient_colors: Vec<ColorGradient>,
    spinner_colors: Vec<ColorGradient>,
    level_curve: Curve,
    direction_param: i32,
    minutes: bool,
    count_down_time: Option<i16>,
    speed: i32,
    center_stop: bool,
    per_iteration: bool,
    grad: (GradMode, bool),
    cross_hash_mode: GradMode,
    spinner_mode: GradMode,
    x_offset_curve: Curve,
    y_offset_curve: Curve,
    angle_curve: Curve,
    font_scale_curve: Curve,
    font: FontDesc,
    count_type: CountType,
    fade_mode: Fade,
    size_mode: SizeMode,
    spinner: bool,
    locations: bool,
    // Per group
    string_count: i32,
    // State
    base_font: FontDesc,
    new_font: FontDesc,
    text: String,
    direction_position: f64,
    fade: f64,
    level: f64,
    iteration: i32,
    previous_number: i32,
    direction: i32,
    size_adjust: i32,
    fps: f64,
    max_text_size: i32,
    buffer_wi: i32,
    buffer_ht: i32,
    rand: Rand,
}

/// Font-free diagnostic adapter only. Calls the actual drawing helpers and
/// preserves Canvas::fill_shape's production alpha threshold and brush paint.
#[cfg(test)]
pub(crate) fn raster_diagnostic(w: i32, h: i32, rect: (i32, i32, i32, i32), spinner: Option<(i32, i32)>) -> vixen_effects::gfx::Bitmap {
    let mut canvas = Canvas::new(w, h);
    if let Some((length, angle)) = spinner {
        let mut effect = from_params(&Params::defaults(DESCRIPTOR.params));
        effect.gradient_colors = vec![ColorGradient::solid(Color::WHITE)];
        effect.level = 1.0;
        effect.fade_mode = Fade::None;
        effect.draw_film_spinner(&mut canvas, rect, length, angle);
    } else {
        let brush = Brush::new(rect, GradMode::Horizontal, ColorGradient::solid(Color::WHITE)).unwrap();
        CountDown::draw_cross_hash(&mut canvas, &brush, rect);
    }
    let mut rgba = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let c = canvas.get(x, y);
            rgba.extend_from_slice(&[c.r, c.g, c.b, c.a]);
        }
    }
    let bitmap = vixen_effects::gfx::Bitmap::from_rgba(w, h, &rgba);
    assert_eq!(bitmap.to_rgba(), rgba, "diagnostic serialization changed Canvas pixels");
    bitmap
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    render_pixel_effect(ctx, p, &mut from_params(p))
}

fn from_params(p: &Params) -> CountDown {
    CountDown {
        gradient_colors: p.gradients("GradientColors"),
        spinner_colors: p.gradients("SpinnerColors"),
        level_curve: p.curve("LevelCurve"),
        direction_param: direction_ord(p.enum_name("Direction")),
        minutes: p.enum_name("TimeFormat") == "Minutes",
        // `Convert.ToInt16(string)` throws on bad input, which leaves the effect dark.
        count_down_time: p.string("CountDownTime").trim().parse::<i16>().ok(),
        speed: p.int("Speed"),
        center_stop: p.boolean("CenterStop"),
        per_iteration: p.boolean("PerIteration"),
        grad: grad_mode_of(p.enum_name("GradientMode")),
        cross_hash_mode: cross_hash_mode(p.enum_name("GradientMode")),
        spinner_mode: match p.enum_name("SpinnerGradientMode") {
            "Horizontal" => GradMode::Horizontal,
            "Vertical" => GradMode::Vertical,
            "Diagonal" => GradMode::ForwardDiagonal,
            _ => GradMode::BackwardDiagonal,
        },
        x_offset_curve: p.curve("XOffsetCurve"),
        y_offset_curve: p.curve("YOffsetCurve"),
        angle_curve: p.curve("AngleCurve"),
        font_scale_curve: p.curve("FontScaleCurve"),
        font: p.font("Font"),
        count_type: match p.enum_name("CountDownType") {
            "CountDown" => CountType::CountDown,
            "CountUp" => CountType::CountUp,
            _ => CountType::Effect,
        },
        fade_mode: match p.enum_name("CountDownFade") {
            "In" => Fade::In,
            "Out" => Fade::Out,
            "InOut" => Fade::InOut,
            _ => Fade::None,
        },
        size_mode: match p.enum_name("SizeMode") {
            "Shrink" => SizeMode::Shrink,
            "Grow" => SizeMode::Grow,
            _ => SizeMode::None,
        },
        spinner: p.boolean("Spinner"),
        locations: TargetPositioning::from_name(p.enum_name("TargetPositioning")) == TargetPositioning::Locations,
        string_count: 0,
        base_font: FontDesc::new("Arial", 20.0),
        new_font: FontDesc::new("Arial", 20.0),
        text: String::new(),
        direction_position: 0.0,
        fade: 1.0,
        level: 0.0,
        iteration: -1,
        previous_number: 0,
        direction: D_NONE,
        size_adjust: 0,
        fps: 20.0,
        max_text_size: 0,
        buffer_wi: 0,
        buffer_ht: 0,
        rand: Rand::default(),
    }
}

/// `ConvertMinutesToString`: `m:ss`, or `h:mm:ss` from an hour up.
fn minutes_to_string(count: i64) -> String {
    if count < 3600 {
        format!("{}:{:02}", count / 60, count % 60)
    } else {
        format!("{}:{:02}:{:02}", count / 3600, (count % 3600) / 60, count % 60)
    }
}

impl CountDown {
    fn calculate_x_offset(&self, pos: f64) -> i32 {
        scale_curve_to_value(self.x_offset_curve.get_value(pos), self.buffer_wi as f64, -(self.buffer_wi as f64)) as i32
    }

    fn calculate_y_offset(&self, pos: f64, maxht: i32) -> i32 {
        let h = self.buffer_ht + maxht / 2;
        scale_curve_to_value(self.y_offset_curve.get_value(pos), h as f64, -(h as f64)) as i32
    }

    fn calculate_angle(&self, pos: f64) -> i32 {
        scale_curve_to_value(self.angle_curve.get_value(pos), 360.0, 0.0) as i32
    }

    fn calculate_font_scale(&self, pos: f64) -> f32 {
        scale_curve_to_value(self.font_scale_curve.get_value(pos), 100.0, 1.0) as f32
    }

    /// `GetNewGolorGradient` / the plain copy of the gradient for the current count iteration.
    fn gradient_for(&self, list: &[ColorGradient]) -> Option<ColorGradient> {
        if list.is_empty() {
            return None;
        }
        // A negative iteration index would throw in the C#.
        let cg = &list[(self.iteration.max(0) as usize) % list.len()];
        if self.level < 1.0 || self.fade_mode != Fade::None {
            let points: Vec<ColorPoint> = cg
                .colors()
                .iter()
                .map(|pt| {
                    let mut hsv = Hsv::from_color(pt.color.to_color());
                    if self.fade_mode != Fade::None {
                        hsv.set_v(hsv.v * self.fade);
                    }
                    hsv.set_v(hsv.v * self.level);
                    ColorPoint::from_xyz(Xyz::from_color(hsv.to_color()), pt.focus, pt.position)
                })
                .collect();
            Some(ColorGradient::from_parts(points, cg.alphas().to_vec()))
        } else {
            Some(cg.clone())
        }
    }

    fn brush_for(&self, list: &[ColorGradient], mode: GradMode, rect: (i32, i32, i32, i32)) -> Option<Brush> {
        Brush::new(rect, mode, self.gradient_for(list)?)
    }

    /// `GetCountDownTime`.
    fn count_down_time(&mut self, frame: i32, info: &PixelInfo) -> String {
        let ct = self.count_down_time.unwrap_or(0) as f64;
        let mut number = match self.count_type {
            CountType::CountDown => {
                let mut n = ((ct * 1000.0 - (frame as f64 * info.frame_time as f64)) / 1000.0).ceil() as i32;
                if self.fade_mode == Fade::In {
                    n -= 1;
                }
                n
            }
            CountType::CountUp => {
                let mut n = ((ct * 1000.0 + (frame as f64 * info.frame_time as f64)) / 1000.0).floor() as i32;
                if self.fade_mode == Fade::In {
                    n += 1;
                }
                n
            }
            CountType::Effect => (info.remaining_time(frame) / 1000.0).ceil() as i32,
        };
        if self.previous_number != number {
            self.previous_number = number;
            self.iteration += 1;
            if self.direction_param == D_RANDOM {
                self.direction = self.rand.next_range(0, 6);
            }
            self.size_adjust = 0;
        }
        if number > 60 && self.minutes {
            return minutes_to_string(number as i64);
        }
        if number == i32::MIN {
            number = 0;
        }
        number.to_string()
    }

    /// `SetFadePositionLevel`.
    fn set_fade_position_level(&mut self, frame: i32, info: &PixelInfo) {
        let total_frames = info.number_frames();
        let mut start_frame = 0;
        if self.count_type == CountType::Effect {
            let tf = total_frames as f64;
            start_frame = ((tf / self.fps - (tf / self.fps).floor()) * self.fps) as i32;
        }
        if self.per_iteration {
            self.direction_position = if self.count_type != CountType::Effect {
                (1.0 / total_frames as f64 * info.time_span.total_secs() * frame as f64) % 1.0
            } else {
                1.0 / self.fps * (((frame - start_frame - 1) as f64) % self.fps)
            };
        } else {
            self.direction_position = (info.interval_position(frame) * self.speed as f64) % 1.0;
        }

        self.text = self.count_down_time(frame, info);

        match self.fade_mode {
            Fade::In => {
                self.fade = 1.0 / self.fps * (((frame - start_frame) as f64) % self.fps);
                if self.fade < 0.0 {
                    self.fade += 1.0;
                }
            }
            Fade::Out => {
                self.fade = 1.0 - 1.0 / self.fps * (((frame - start_frame - 1) as f64) % self.fps);
                if self.fade > 1.0 {
                    self.fade -= 1.0;
                }
            }
            Fade::InOut => {
                self.fade = ((frame - start_frame) as f64) % self.fps;
                if self.fade < 0.0 {
                    self.fade = -self.fade;
                }
                if self.fade < 10.0 {
                    self.fade = 1.0 / self.fps * self.fade;
                    if self.fade < 0.0 {
                        self.fade += 1.0;
                    }
                } else {
                    self.fade = 1.0 - 1.0 / self.fps * self.fade;
                    if self.fade > 1.0 {
                        self.fade -= 1.0;
                    }
                }
            }
            Fade::None => self.fade = 1.0,
        }
    }

    // ---- Film spinner ----

    fn max_cross_hash_width(&self, ht: i32, wi: i32) -> i32 {
        // A 2 pixel margin at the top and bottom of the matrix.
        ht.min(wi) - 4
    }

    fn cross_hash_width(&self, ht: i32, wi: i32) -> i32 {
        let max_w = self.max_cross_hash_width(ht, wi);
        let mut min_w = max_w - 20;
        if min_w < 0 {
            min_w = 1;
        }
        match self.size_mode {
            SizeMode::Grow => min_w + self.size_adjust,
            SizeMode::Shrink => max_w - self.size_adjust,
            SizeMode::None => max_w,
        }
    }

    fn radial_line_angle(&self, frame: i32, info: &PixelInfo) -> i32 {
        // C# casts the integral milliseconds to decimal before dividing. Binary floats can
        // make an exact degree boundary slightly too large (550 ms -> 198.000...03).
        let scaled = (info.remaining_time(frame) % 1000.0).ceil() as i32 * 360;
        scaled.div_euclid(1000) + i32::from(scaled.rem_euclid(1000) != 0)
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_film_spinner_swipe(
        &self,
        canvas: &mut Canvas,
        wi: i32,
        ht: i32,
        move_x: i32,
        move_y: i32,
        text_angle: i32,
        radial_angle: i32,
    ) {
        let swipe_radius = ((wi * wi + ht * ht) as f64).sqrt() as i32;
        let swipe_radius_int = swipe_radius;
        let max_cross = self.max_cross_hash_width(ht, wi);
        // The circle must cover the matrix even when the cross hash is slightly off it.
        let circle = 2 * swipe_radius_int + max_cross;
        let nx = ((wi - circle) as f64 / 2.0).ceil() as i32;
        let ny = ((ht - circle) as f64 / 2.0).ceil() as i32;
        let (sx, sy) = (nx + move_x, ny + move_y);
        let Some(brush) = self.brush_for(&self.spinner_colors, self.spinner_mode, (0, 0, wi, ht)) else { return };
        let rotate = if self.direction == D_ROTATE { text_angle } else { 0 };
        // Zero is along the Y axis.
        let start = -90 + rotate;
        let sweep = 360 - radial_angle;
        canvas.fill_shape(&brush, |g| g.fill_pie(Color::WHITE, sx as f32, sy as f32, circle as f32, circle as f32, start as f32, sweep as f32));
    }

    fn draw_cross_hash(canvas: &mut Canvas, brush: &Brush, rect: (i32, i32, i32, i32)) {
        let (l, t, w, h) = rect;
        // Preserve the integer GDI coordinates used by DrawFilmSpinnerCrossHash.
        // Graphics applies its own device-pixel alignment; an extra half pixel shifts the mask.
        canvas.fill_shape(brush, |g| {
            g.draw_ellipse(Color::WHITE, 1.0, l as f32, t as f32, w as f32, h as f32);
            g.draw_line(Color::WHITE, 1.0, (l + w / 2) as f32, t as f32, (l + w / 2) as f32, (t + h) as f32);
            g.draw_line(Color::WHITE, 1.0, l as f32, (t + h / 2) as f32, (l + w) as f32, (t + h / 2) as f32);
        });
    }

    fn draw_film_spinner(&self, canvas: &mut Canvas, rect: (i32, i32, i32, i32), line_length: i32, radial_angle: i32) {
        let brush_rect = (rect.0 - 1, rect.1 - 1, rect.2 + 2, rect.3 + 2);
        let Some(brush) = self.brush_for(&self.gradient_colors, self.cross_hash_mode, brush_rect) else { return };
        Self::draw_cross_hash(canvas, &brush, rect);
        // Adding 90 degrees brings 0 back to the X axis.
        let radians = (radial_angle + 90) as f64 * (std::f64::consts::PI / 180.0);
        let (sx, sy) = (rect.0 + rect.2 / 2, rect.1 + rect.3 / 2);
        let pt_x = sx as f64 + line_length as f64 * radians.cos();
        let pt_y = sy as f64 - 1.0 * line_length as f64 * radians.sin();
        canvas.fill_shape(&brush, |g| g.draw_line(Color::WHITE, 1.0, sx as f32, sy as f32, pt_x as i32 as f32, pt_y as i32 as f32));
    }

    // ---- Text ----

    fn draw_text(&self, text: &str, canvas: &mut Canvas, p: (i32, i32)) {
        let (mode, across_element) = self.grad;
        let size = measure(text, &self.new_font);
        let offset = self.max_text_size - size.0 as i32;
        let offset_point = (p.0 + offset / 2, p.1);
        let rect = if across_element {
            (0, 0, self.buffer_wi, self.buffer_ht)
        } else {
            (offset_point.0, p.1, self.max_text_size, size.1 as i32)
        };
        if let Some(brush) = self.brush_for(&self.gradient_colors, mode, rect) {
            canvas.draw_string(text, &self.new_font, &brush, offset_point.0, offset_point.1);
        }
    }

    fn initial_render(&mut self, frame: i32, info: &PixelInfo) -> Canvas {
        let (buffer_ht, buffer_wi) = (info.buffer_ht(), info.buffer_wi());
        let mut canvas = Canvas::new(buffer_wi, buffer_ht);
        let pos = info.interval_position(frame) * 100.0;
        let text_angle = self.calculate_angle(pos);

        self.set_fade_position_level(frame, info);

        // Font size from the size mode.
        self.size_adjust += 1;
        let base_pts = self.base_font.size;
        let new_size = match self.size_mode {
            SizeMode::Grow => base_pts / self.fps as f32 * self.size_adjust as f32,
            SizeMode::Shrink => base_pts / self.fps as f32 * (21 - self.size_adjust) as f32,
            SizeMode::None => base_pts * (self.calculate_font_scale(pos) / 100.0),
        };
        // GDI+ throws for a font size of 0 or less; a tiny size draws nothing visible instead.
        self.new_font = FontDesc { name: self.font.name.clone(), size: new_size.max(0.1), bold: self.font.bold, italic: self.font.italic };

        let mut textsize = (0.0f32, 0.0f32);
        if !self.text.is_empty() {
            let size = measure(&self.text, &self.new_font);
            if size.0 > textsize.0 {
                textsize = size;
            }
        }
        self.max_text_size = (textsize.0 as f64 * 0.95).round_ties_even() as i32;
        let maxht = (textsize.1 as f64).round_ties_even() as i32;

        let mut x_offset = self.calculate_x_offset(pos);
        let mut y_offset = self.calculate_y_offset(pos, maxht);
        match self.direction {
            D_LEFT | D_RIGHT => x_offset = 0,
            D_UP | D_DOWN => y_offset = 0,
            _ => {}
        }
        let offset_left = (((buffer_wi - self.max_text_size) / 2) * 2 + x_offset) / 2;
        let offset_top = (((buffer_ht - maxht) / 2) * 2 + y_offset) / 2;
        let (tw, th) = (textsize.0, textsize.1);
        let point = match self.direction {
            D_LEFT => {
                let left_x = buffer_wi - (self.direction_position * (tw as f64 + buffer_wi as f64)) as i32;
                (if self.center_stop { left_x.max((buffer_wi - tw as i32) / 2) } else { left_x }, offset_top)
            }
            D_RIGHT => {
                let right_x = -self.max_text_size + (self.direction_position * (self.max_text_size as f64 + buffer_wi as f64)) as i32;
                (if self.center_stop { right_x.min((buffer_wi - tw as i32) / 2) } else { right_x }, offset_top)
            }
            D_UP => {
                let up_y = buffer_ht - ((th as f64 + buffer_ht as f64) * self.direction_position) as i32;
                (offset_left, if self.center_stop { up_y.max((buffer_ht - th as i32) / 2) } else { up_y })
            }
            D_DOWN => {
                let down_y = -(th as i32) + ((th as f64 + buffer_ht as f64) * self.direction_position) as i32;
                (offset_left, if self.center_stop { down_y.min((buffer_ht - th as i32) / 2) } else { down_y })
            }
            _ => ((buffer_wi - self.max_text_size) / 2 + x_offset, offset_top),
        };

        let center_of_text = (point.0 + tw as i32 / 2, point.1 + th as i32 / 2);
        let (mut move_x, mut move_y, mut radial_angle) = (0, 0, 0);
        if self.spinner {
            move_x = center_of_text.0 - buffer_wi / 2;
            move_y = center_of_text.1 - buffer_ht / 2;
            radial_angle = self.radial_line_angle(frame, info);
            // The swipe is drawn before the rotation so its gradient fits the matrix.
            self.draw_film_spinner_swipe(&mut canvas, buffer_wi, buffer_ht, move_x, move_y, text_angle, radial_angle);
        }

        if self.direction == D_ROTATE {
            canvas.set_rotation(text_angle as f32, buffer_wi as f32 / 2.0 + x_offset as f32, buffer_ht as f32 / 2.0 + (y_offset as f32 / 2.0));
        }

        if self.spinner {
            let cross_w = self.cross_hash_width(buffer_ht, buffer_wi);
            let left = (buffer_wi - cross_w) / 2 + move_x;
            let top = (buffer_ht - cross_w) / 2 + move_y;
            let line_len = ((buffer_wi * buffer_wi + buffer_ht * buffer_ht) as f64).sqrt().ceil() as i32;
            if cross_w > 0 {
                self.draw_film_spinner(&mut canvas, (left, top, cross_w, cross_w), line_len, radial_angle);
            }
        }
        let text = self.text.clone();
        self.draw_text(&text, &mut canvas, point);
        canvas
    }
}

impl PixelEffect for CountDown {
    fn begin_group(&mut self, targets: &[NodeId], ctx: &RenderCtx) {
        self.string_count = ctx.tree.leaf_parents(targets).len() as i32;
    }

    fn setup_render(&mut self, info: &PixelInfo) {
        self.fps = 1000.0 / info.frame_time as f64;
        self.iteration = -1;
        self.direction = self.direction_param;
        self.size_adjust = 0;
        self.previous_number = 0;
        self.buffer_wi = info.buffer_wi();
        self.buffer_ht = info.buffer_ht();
        if self.locations {
            // Keeps the text the same visual size when switching between strings and locations.
            let sc = self.string_count.max(1) as f64;
            let fs = self.font.size as f64;
            let new_size = (sc - ((sc - fs) / 100.0) * 100.0) * (info.buffer_ht() as f64 / sc);
            self.base_font = FontDesc { name: self.font.name.clone(), size: (new_size as i32).max(1) as f32, bold: self.font.bold, italic: self.font.italic };
        } else {
            self.base_font = self.font.clone();
        }
        self.new_font = self.base_font.clone();
    }

    fn cleanup_render(&mut self) {}

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        if self.count_down_time.is_none() && self.count_type != CountType::Effect {
            return;
        }
        let (ht, wi) = (info.buffer_ht(), info.buffer_wi());
        let canvas = self.initial_render(frame, info);
        // Deliberate quirk: the string version samples the level after drawing, so a frame uses the previous
        // frame's level (and the very first frame draws with level 0).
        self.level = self.level_curve.get_value(info.interval_position(frame) * 100.0) / 100.0;
        for x in 0..wi {
            for y in 0..ht {
                let c = canvas.get(x, ht - y - 1);
                if c.a != 0 {
                    buf.set_pixel(x, y, c);
                }
            }
        }
    }

    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        if self.count_down_time.is_none() && self.count_type != CountType::Effect {
            return;
        }
        let ht = info.buffer_ht();
        let mut nodes = buf.element_locations().to_vec();
        nodes.sort_by_key(|l| (l.x, l.y));
        for frame in 0..num_frames {
            buf.current_frame = frame;
            self.level = self.level_curve.get_value(info.interval_position(frame) * 100.0) / 100.0;
            let canvas = self.initial_render(frame, info);
            for l in &nodes {
                // Flip and offset the coordinates so the location version acts like the string version.
                let mut y = ((info.buffer_ht_offset() - l.y) + (ht - 1 + info.buffer_ht_offset())).abs();
                y -= info.buffer_ht_offset();
                let x = l.x - info.buffer_wi_offset();
                let c = canvas.get(x, ht - y - 1);
                if c.a != 0 {
                    buf.set_pixel(l.x, l.y, c);
                } else {
                    buf.set_pixel(l.x, l.y, Color::TRANSPARENT);
                }
            }
        }
    }
}

// ---- Timeline thumbnail (GenerateVisualRepresentation) ----

/// `VixenSystem.DefaultUpdateInterval`, for the thumbnail: `FrameTime` isn't part of `TargetInfo`, and the only thing it
/// decides here is how a fraction of a second at the start of an Effect count lines up.
const VISUAL_FRAME_MS: f64 = 50.0;

/// `GenerateVisualRepresentation`: every number the effect counts through, one per second of its length.
fn visual(p: &Params, t: &TargetInfo, g: &mut Graphics, rect: (i32, i32, i32, i32)) {
    let colors = p.gradients("GradientColors");
    if colors.is_empty() {
        return;
    }
    let mode = match p.enum_name("GradientMode") {
        "VerticalAcrossElement" | "VerticalAcrossText" => GradMode::Vertical,
        "DiagonalAcrossElement" | "DiagonalAcrossText" => GradMode::ForwardDiagonal,
        "BackwardDiagonalAcrossElement" | "BackwardDiagonalAcrossText" => GradMode::BackwardDiagonal,
        _ => GradMode::Horizontal,
    };
    // `Convert.ToInt32(CountDownTime)` throws for anything else, and so does `i % secondTicks` for 0 below.
    let Ok(mut count) = p.string("CountDownTime").trim().parse::<i32>() else { return };
    let second_ticks = (rect.2 as f64 / t.time_span.ticks() as i32 as f64 * 10_000_000.0) as i32;
    if second_ticks == 0 {
        return;
    }
    let minutes = p.enum_name("TimeFormat") == "Minutes";
    let label = |n: i32| if n > 60 && minutes { minutes_to_string(n as i64) } else { n.to_string() };
    let fade_in = p.enum_name("CountDownFade") == "In";
    let mut iteration = 0usize;
    let mut draw = |g: &mut Graphics, text: &str, x: i32| {
        draw_visual_text(p, g, rect, &colors, mode, text, x, iteration);
        iteration += 1;
    };
    match p.enum_name("CountDownType") {
        "CountDown" => {
            if fade_in {
                count -= 1;
            }
            for i in (0..rect.2).filter(|i| i % second_ticks == 0) {
                draw(g, &label(count), i);
                count -= 1;
            }
        }
        "CountUp" => {
            if fade_in {
                count += 1;
            }
            for i in (0..rect.2).filter(|i| i % second_ticks == 0) {
                draw(g, &label(count), i);
                count += 1;
            }
        }
        _ => {
            let total_frames = (t.time_span.total_ms() / VISUAL_FRAME_MS) as i32 as f64;
            let fps = 1000.0 / VISUAL_FRAME_MS;
            let seconds = total_frames / fps;
            let is_int = (seconds % 1.0).abs() <= f64::EPSILON * 100.0;
            let start_tick = (rect.2 as f64 / total_frames * ((seconds - seconds.floor()) * fps) as i32 as f64) as i32;
            count = seconds.ceil() as i32;
            if !is_int {
                count -= 1;
            }
            for i in (0..rect.2).filter(|i| i % second_ticks == 0) {
                if i == 0 && start_tick > 2 {
                    draw(g, &label(count + 1), i);
                }
                draw(g, &label(count), i + start_tick);
                count -= 1;
            }
        }
    }
}

/// `DrawText`.
#[allow(clippy::too_many_arguments)]
fn draw_visual_text(
    p: &Params,
    g: &mut Graphics,
    rect: (i32, i32, i32, i32),
    colors: &[ColorGradient],
    mode: GradMode,
    display: &str,
    start_x: i32,
    iteration: usize,
) {
    let local = (0, 0, rect.2, rect.3);
    let font = adjusted_font(display, local, &p.font("Font"), 48, 10);
    let size = measure(display, &font);
    let gradient = &colors[iteration % colors.len()];
    if let Some(b) = Brush::new((start_x, 2, size.0 as i32, size.1 as i32), mode, gradient.clone()) {
        blend_text(g, rect, display, &font, &b, start_x, 2);
    }
    if p.boolean("Spinner") {
        let sq = (size.0 as i32).min(size.1 as i32);
        let r = (start_x, 2 + (size.1 as i32 - sq) / 2, sq, sq);
        if let Some(b) = Brush::new(r, cross_hash_mode(p.enum_name("GradientMode")), colors[0].clone()) {
            let mut m = Graphics::new(rect.2.max(1), rect.3.max(1));
            let (l, t, w, h) = r;
            m.draw_ellipse(Color::WHITE, 1.0, l as f32, t as f32, w as f32, h as f32);
            m.draw_line(Color::WHITE, 1.0, (l + w / 2) as f32, t as f32, (l + w / 2) as f32, (t + h) as f32);
            m.draw_line(Color::WHITE, 1.0, l as f32, (t + h / 2) as f32, (l + w) as f32, (t + h / 2) as f32);
            blend_mask(g, rect, m.bitmap(), &b);
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
    fn minutes_format() {
        assert_eq!(minutes_to_string(65), "1:05");
        assert_eq!(minutes_to_string(600), "10:00");
        assert_eq!(minutes_to_string(3725), "1:02:05");
    }

    #[test]
    fn all_directions_and_spinner_render() {
        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 20, 30);
        for dir in ["Left", "Right", "Up", "Down", "Rotate", "None", "Random"] {
            for spinner in [false, true] {
                let out = testing::render_with(&DESCRIPTOR, &tree, &[m], 3000, |p| {
                    p.set("Direction", ParamValue::Enum(dir.into()));
                    p.set("Spinner", ParamValue::Bool(spinner));
                    p.set("Font", ParamValue::Font(FontDesc::new("Arial", 10.0)));
                });
                let mut any = false;
                for ms in (0..3000).step_by(100) {
                    any |= lit_count(&tree, m, &out, ms) > 0;
                }
                assert!(any, "{dir} spinner={spinner} drew nothing");
            }
        }
    }

    #[test]
    fn count_down_type_uses_the_start_time() {
        // "Down from time" 5 with no fade shows 5 at the start; the text is drawn, so some pixels are lit.
        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 20, 30);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[m], 2000, |p| {
            p.set("CountDownType", ParamValue::Enum("CountDown".into()));
            p.set("CountDownTime", ParamValue::Str("5".into()));
            p.set("CountDownFade", ParamValue::Enum("None".into()));
            p.set("Direction", ParamValue::Enum("None".into()));
            p.set("Font", ParamValue::Font(FontDesc::new("Arial", 10.0)));
        });
        assert!(lit_count(&tree, m, &out, 500) > 0);
        // Bad text for the time leaves the effect dark.
        let bad = testing::render_with(&DESCRIPTOR, &tree, &[m], 2000, |p| {
            p.set("CountDownType", ParamValue::Enum("CountDown".into()));
            p.set("CountDownTime", ParamValue::Str("abc".into()));
        });
        assert_eq!(lit_count(&tree, m, &bad, 500), 0);
    }

    #[test]
    fn first_frame_uses_level_zero() {
        // The string version computes the level after drawing, so frame 0 is drawn black.
        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 20, 30);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[m], 1000, |p| {
            p.set("Direction", ParamValue::Enum("None".into()));
            p.set("CountDownFade", ParamValue::Enum("None".into()));
            p.set("Font", ParamValue::Font(FontDesc::new("Arial", 10.0)));
        });
        // Frame 0 has pixels set to black (not lit), frame 1 has red text.
        assert_eq!(lit_count(&tree, m, &out, 0), 0);
        assert!(lit_count(&tree, m, &out, 100) > 0);
    }

    #[test]
    fn thumbnail_draws_number() {
        let mut g = Graphics::new(80, 24);
        let p = DESCRIPTOR.new_params();
        // One number a second over a two second effect.
        let mut tree = ElementTree::new();
        let n = tree.add_element("E");
        let targets = [n];
        let t = TargetInfo { time_span: TimeSpan::from_ms_i(2000), ..testing::target_info(&tree, &targets) };
        visual(&p, &t, &mut g, (0, 0, 80, 24));
        assert!((40..80).any(|x| (0..24).any(|y| g.bitmap().get_pixel(x, y).a > 0)), "the second number");
        let lit = (0..24).flat_map(|y| (0..80).map(move |x| (x, y))).filter(|(x, y)| g.bitmap().get_pixel(*x, *y).a > 0).count();
        assert!(lit > 10);
    }

    #[test]
    fn visibility_rules() {
        let mut p = DESCRIPTOR.new_params();
        p.set("Spinner", ParamValue::Bool(true));
        p.set("CountDownType", ParamValue::Enum("CountDown".into()));
        let vis: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(vis.contains(&"SpinnerColors") && vis.contains(&"CountDownTime"));
        assert!(!vis.contains(&"Speed")); // Direction None
    }
}

#[cfg(test)]
#[path = "count_down_golden.rs"]
mod golden;
