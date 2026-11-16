//! Port of the `Whirl` class: one rectangular whirl (or a meteor travelling the whirl path, or concentric
//! rectangles) drawn into a region of the frame buffer. The C# spreads this over five partial files with
//! eight near-identical methods per direction; here each family is one function driven by a small table.

use vixen_effects::prelude::*;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Mode {
    Recurrent,
    Symmetrical,
    Meteor,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Start {
    TopLeft,
    BottomLeft,
    TopRight,
    BottomRight,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Dir {
    In,
    Out,
    InAndOut,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Rot {
    Clockwise,
    CounterClockwise,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ColorMode {
    GradientOverTime,
    LegColors,
    RectangularRings,
    Bands,
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

/// `WhirlVortexMetadata`.
#[derive(Clone, Default, Debug)]
struct Vortex {
    last_x: i32,
    last_y: i32,
    last_width: i32,
    last_height: i32,
    draw_top: bool,
    draw_right: bool,
    draw_left: bool,
    draw_bottom: bool,
}

impl Vortex {
    fn draws(&self, side: Side) -> bool {
        match side {
            Side::Left => self.draw_left,
            Side::Right => self.draw_right,
            Side::Top => self.draw_top,
            Side::Bottom => self.draw_bottom,
        }
    }
}

#[derive(Clone)]
pub struct Whirl {
    pub enabled: bool,
    pub mode: Mode,
    pub tail_length: i32,
    pub start: Start,
    pub direction: Dir,
    pub rotation: Rot,
    pub reverse_draw: bool,
    pub spacing: i32,
    pub thickness: i32,
    pub show_3d: bool,
    pub color_mode: ColorMode,
    pub band_length: i32,
    pub single_color: ColorGradient,
    pub colors: Vec<ColorGradient>,
    pub left_color: ColorGradient,
    pub right_color: ColorGradient,
    pub top_color: ColorGradient,
    pub bottom_color: ColorGradient,

    pub x: i32,
    pub y: i32,
    pub x_offset: i32,
    pub y_offset: i32,
    pub width: i32,
    pub height: i32,
    pub intensity: f64,
    pub pause_at_end: i32,
    /// The C# would have thrown a `DivideByZeroException` by now, which leaves the whole effect dark.
    pub threw: bool,

    number_of_frames: i32,
    total_number_of_pixels: f64,
    drawing_tail: bool,
    pixels_to_draw_per_frame: f64,
    number_of_pixels_to_draw: f64,
    frames_per_rectangle: i32,
    number_of_symmetric_rectangles: i32,
    color_index: i32,
    band_pixel_count: i32,
    internal_reverse_draw: bool,
    vortex: Vortex,
}

fn parse_mode(s: &str) -> Mode {
    match s {
        "SymmetricalWhirls" => Mode::Symmetrical,
        "Meteor" => Mode::Meteor,
        _ => Mode::Recurrent,
    }
}

fn parse_start(s: &str) -> Start {
    match s {
        "BottomLeft" => Start::BottomLeft,
        "TopRight" => Start::TopRight,
        "BottomRight" => Start::BottomRight,
        _ => Start::TopLeft,
    }
}

fn parse_dir(s: &str) -> Dir {
    match s {
        "Out" => Dir::Out,
        "InAndOut" => Dir::InAndOut,
        _ => Dir::In,
    }
}

fn parse_color_mode(s: &str) -> ColorMode {
    match s {
        "LegColors" => ColorMode::LegColors,
        "RectangularRings" => ColorMode::RectangularRings,
        "Bands" => ColorMode::Bands,
        _ => ColorMode::GradientOverTime,
    }
}

/// `DoubleThickness`.
#[inline]
fn double_thickness(length: i32, thickness: i32) -> bool {
    length - 2 * thickness >= 0
}

impl Whirl {
    /// `new Whirl()`. Used when the effect has no whirls at all.
    pub fn new() -> Whirl {
        Whirl {
            enabled: true,
            mode: Mode::Recurrent,
            tail_length: 10,
            start: Start::TopLeft,
            direction: Dir::In,
            rotation: Rot::Clockwise,
            reverse_draw: false,
            spacing: 1,
            thickness: 1,
            show_3d: false,
            color_mode: ColorMode::GradientOverTime,
            band_length: 10,
            single_color: ColorGradient::solid(Color::RED),
            colors: vec![ColorGradient::solid(Color::RED), ColorGradient::solid(Color::WHITE)],
            left_color: ColorGradient::solid(Color::BLUE),
            right_color: ColorGradient::solid(Color::rgb(255, 255, 0)),
            top_color: ColorGradient::solid(Color::RED),
            bottom_color: ColorGradient::solid(Color::rgb(0, 128, 0)),
            x: 0,
            y: 0,
            x_offset: 0,
            y_offset: 0,
            width: 0,
            height: 0,
            intensity: 1.0,
            pause_at_end: 0,
            threw: false,
            number_of_frames: 0,
            total_number_of_pixels: 0.0,
            drawing_tail: false,
            pixels_to_draw_per_frame: 0.0,
            number_of_pixels_to_draw: 0.0,
            frames_per_rectangle: 0,
            number_of_symmetric_rectangles: 0,
            color_index: 0,
            band_pixel_count: 0,
            internal_reverse_draw: false,
            vortex: Vortex::default(),
        }
    }

    /// `UpdateWhirlModel`: a whirl from a `WhirlData` record.
    pub fn from_record(d: &Params) -> Whirl {
        let mut w = Whirl::new();
        w.enabled = d.boolean("Enabled");
        w.x = d.int("X");
        w.y = d.int("Y");
        w.width = d.int("Width");
        w.height = d.int("Height");
        w.mode = parse_mode(d.enum_name("WhirlMode"));
        w.tail_length = d.int("TailLength");
        w.start = parse_start(d.enum_name("StartLocation"));
        w.direction = parse_dir(d.enum_name("WhirlDirection"));
        w.rotation = if d.enum_name("Rotation") == "CounterClockwise" { Rot::CounterClockwise } else { Rot::Clockwise };
        w.reverse_draw = d.boolean("ReverseDraw");
        w.spacing = d.int("Spacing");
        w.thickness = d.int("Thickness");
        w.show_3d = d.boolean("Show3D");
        w.color_mode = parse_color_mode(d.enum_name("ColorMode"));
        w.band_length = d.int("BandLength");
        w.left_color = d.gradient("LeftColor");
        w.right_color = d.gradient("RightColor");
        w.top_color = d.gradient("TopColor");
        w.bottom_color = d.gradient("BottomColor");
        w.single_color = d.gradient("SingleColor");
        w.colors = d.gradients("Colors");
        w
    }

    fn calc_spacing(&self, spacing: i32) -> i32 {
        (spacing as f64 / 100.0 * self.width.max(self.height) as f64) as i32
    }

    fn calc_thickness(&self, thickness: i32) -> i32 {
        let t = (thickness as f64 / 100.0 * self.width.max(self.height) as f64) as i32;
        t.max(1)
    }

    fn tail_len(&self) -> i32 {
        let t = (self.tail_length as f64 / 100.0 * self.width.max(self.height) as f64) as i32;
        t.max(1)
    }

    fn band_len(&self) -> i32 {
        let b = (self.band_length as f64 / 100.0 * self.width.max(self.height) as f64) as i32;
        b.max(1)
    }

    /// `ResetPixelNumber`.
    pub fn reset_pixel_number(&mut self) {
        self.number_of_pixels_to_draw = if self.internal_reverse_draw { self.total_number_of_pixels - 1.0 } else { 0.0 };
    }

    // ---- Setup

    /// `CalculateNumberOfPixelsXxx` for all eight corner and rotation variants. `swap` is the family whose
    /// first side is the height; `flags` are the (left, top, right, bottom) draw flags set by the three
    /// places the spiral can end.
    #[allow(clippy::too_many_arguments)]
    fn calc_pixels(&mut self, x: i32, y: i32, width: i32, height: i32, t: i32, s: i32, first: bool, swap: bool, flags: &[u8; 3]) -> i32 {
        let (a, b) = if swap { (height, width) } else { (width, height) };
        let v = &mut self.vortex;
        v.draw_left = true;
        v.draw_bottom = true;
        v.draw_top = true;
        v.draw_right = true;
        let mut n = if first { a - 1 } else { a + t + s - 1 };
        let set = |v: &mut Vortex, f: u8| {
            v.last_width = width;
            v.last_height = height;
            v.draw_left = f & 8 != 0;
            v.draw_top = f & 4 != 0;
            v.draw_right = f & 2 != 0;
            v.draw_bottom = f & 1 != 0;
            v.last_x = x;
            v.last_y = y;
        };
        if b - t >= t {
            n += b - 1;
            if a - t >= t && double_thickness(b, t) {
                n += a - 1;
                if b - t - s >= t && double_thickness(a, t) {
                    n += b - t - s - 1;
                } else {
                    n += 1;
                    set(&mut self.vortex, flags[0]);
                }
            } else {
                n += 1;
                set(&mut self.vortex, flags[1]);
            }
        } else {
            n += 1;
            set(&mut self.vortex, flags[2]);
        }
        let width2 = width - 2 * (t + s);
        let height2 = height - 2 * (t + s);
        if width2 >= t && height2 >= t {
            n += self.calc_pixels(x + t + s, y + t + s, width2, height2, t, s, false, swap, flags);
        } else {
            let v = &mut self.vortex;
            v.last_width = width2 + 2 * (t + s);
            v.last_height = height2 + 2 * (t + s);
            if v.draw_left && v.draw_bottom && v.draw_top && v.draw_right {
                n += 1;
            }
            v.last_x = x;
            v.last_y = y;
        }
        n
    }

    /// `SetupRenderTopLeft` and friends: (swap, flags as left/top/right/bottom bits) per corner and rotation.
    fn total_pixels_for(&mut self, t: i32, s: i32) -> f64 {
        const fn f(l: u8, t: u8, r: u8, b: u8) -> u8 {
            l * 8 + t * 4 + r * 2 + b
        }
        let (w, h) = (self.width, self.height);
        let (swap, flags): (bool, [u8; 3]) = match (self.start, self.rotation) {
            (Start::TopLeft, Rot::Clockwise) => (false, [f(0, 1, 1, 1), f(0, 1, 1, 0), f(0, 1, 0, 0)]),
            (Start::TopLeft, Rot::CounterClockwise) => (true, [f(1, 0, 1, 1), f(1, 0, 0, 1), f(1, 0, 0, 0)]),
            (Start::BottomLeft, Rot::Clockwise) => (true, [f(1, 1, 1, 0), f(1, 1, 0, 0), f(1, 0, 0, 0)]),
            (Start::BottomLeft, Rot::CounterClockwise) => (false, [f(0, 1, 1, 1), f(0, 0, 1, 1), f(0, 0, 0, 1)]),
            (Start::TopRight, Rot::Clockwise) => (true, [f(1, 0, 1, 1), f(0, 0, 1, 1), f(0, 0, 1, 0)]),
            (Start::TopRight, Rot::CounterClockwise) => (false, [f(1, 1, 0, 1), f(1, 1, 0, 0), f(0, 1, 0, 0)]),
            (Start::BottomRight, Rot::Clockwise) => (false, [f(1, 1, 0, 1), f(1, 0, 0, 1), f(0, 0, 0, 1)]),
            (Start::BottomRight, Rot::CounterClockwise) => (true, [f(1, 1, 1, 0), f(0, 1, 1, 0), f(0, 0, 1, 0)]),
        };
        self.calc_pixels(0, 0, w, h, t, s, true, swap, &flags) as f64
    }

    /// `CalculateNumberOfRectangles`.
    fn calc_rectangles(&mut self, x: i32, y: i32, width: i32, height: i32, t: i32, s: i32) -> i32 {
        let mut n = 0;
        if width >= t && height >= t {
            n = 1;
            let width2 = width - 2 * (t + s);
            let height2 = height - 2 * (t + s);
            if width2 >= t && height2 >= t {
                n += self.calc_rectangles(x + t + s, y + t + s, width2, height2, t, s);
            } else {
                self.vortex.last_width = width2 + 2 * (t + s);
                self.vortex.last_height = height2 + 2 * (t + s);
                self.vortex.last_x = x;
                self.vortex.last_y = y;
            }
        }
        n
    }

    /// `SetupRender`.
    pub fn setup_render(&mut self, number_of_frames: i32) {
        self.internal_reverse_draw = self.reverse_draw;
        self.drawing_tail = false;
        self.number_of_frames = number_of_frames;
        let frames_at_end = number_of_frames as f64 * self.pause_at_end as f64 / 100.0;
        let (t, s) = (self.calc_thickness(self.thickness), self.calc_spacing(self.spacing));
        if self.mode != Mode::Symmetrical {
            self.total_number_of_pixels = self.total_pixels_for(t, s);
            let mut n = self.total_number_of_pixels;
            if self.direction == Dir::InAndOut {
                n *= 2.0;
            }
            self.pixels_to_draw_per_frame = n / (number_of_frames as f64 - frames_at_end);
            self.reset_pixel_number();
        } else {
            self.number_of_symmetric_rectangles = self.calc_rectangles(0, 0, self.width, self.height, t, s);
            if self.number_of_symmetric_rectangles == 0 {
                self.threw = true;
                return;
            }
            self.frames_per_rectangle = (number_of_frames as f64 - frames_at_end) as i32 / self.number_of_symmetric_rectangles;
            if self.direction == Dir::InAndOut {
                self.frames_per_rectangle /= 2;
            }
        }
    }

    // ---- Colour

    fn update_selected_color_band(&mut self) {
        if self.color_mode == ColorMode::Bands {
            self.band_pixel_count += 1;
            if self.band_pixel_count >= self.band_len() {
                self.color_index += 1;
                if self.color_index > self.colors.len() as i32 - 1 {
                    self.color_index = 0;
                }
                self.band_pixel_count = 0;
            }
        }
    }

    /// `UpdateColor`: the next ring colour.
    fn update_color(&mut self) {
        if self.color_mode == ColorMode::RectangularRings {
            self.color_index += 1;
            if self.color_index > self.colors.len() as i32 - 1 {
                self.color_index = 0;
            }
        }
    }

    fn indexed_color(&self, interval_pos: f64) -> Option<Color> {
        // `Colors[_colorIndex]` throws for an empty list in C#; the pixel is not drawn here.
        usize::try_from(self.color_index).ok().and_then(|i| self.colors.get(i)).map(|g| g.get_color_at(interval_pos))
    }

    /// `GetColor`. `None` stands for the C# index exception.
    fn get_color(&mut self, side: Side, interval_pos: f64, whirl_index: i32, remaining: i32) -> Option<Color> {
        let mut color = Color::TRANSPARENT;
        match self.color_mode {
            ColorMode::GradientOverTime => {
                if self.mode == Mode::Meteor {
                    if remaining <= self.tail_len() {
                        let tail = self.tail_len() as f64;
                        let tail_interval = (tail - remaining as f64) / tail;
                        color = self.single_color.get_color_at(tail_interval);
                    }
                } else {
                    color = self.single_color.get_color_at(interval_pos);
                }
            }
            ColorMode::LegColors => {
                color = match side {
                    Side::Left => self.left_color.get_color_at(interval_pos),
                    Side::Right => self.right_color.get_color_at(interval_pos),
                    Side::Top => self.top_color.get_color_at(interval_pos),
                    Side::Bottom => self.bottom_color.get_color_at(interval_pos),
                };
            }
            ColorMode::Bands => {
                let c = self.indexed_color(interval_pos);
                self.update_selected_color_band();
                color = c?;
            }
            ColorMode::RectangularRings => {
                color = self.indexed_color(interval_pos)?;
            }
        }
        if self.show_3d {
            let thickness = self.calc_thickness(self.thickness);
            let mut hsv = Hsv::from_color(color);
            hsv.set_v(hsv.v * (((thickness - whirl_index) as f32 / thickness as f32) as f64));
            color = hsv.to_color();
        }
        let mut hsv = Hsv::from_color(color);
        hsv.set_v(hsv.v * self.intensity);
        Some(hsv.to_color())
    }

    // ---- Drawing primitives

    fn set_pixel(&self, fb: &mut PixelFrameBuffer, x: i32, y: i32, color: Color, remaining: i32) {
        let (xo, yo) = (self.x + self.x_offset, self.y + self.y_offset);
        if self.mode == Mode::Meteor {
            if remaining <= self.tail_len() {
                fb.set_pixel(x.wrapping_add(xo), y.wrapping_add(yo), color);
            }
        } else {
            fb.set_pixel(x.wrapping_add(xo), y.wrapping_add(yo), color);
        }
    }

    /// `CalcNumPixels`.
    fn calc_num_pixels(&self, pixels: i32, number_of_pixels: i32) -> i32 {
        if self.drawing_tail && pixels.wrapping_add(1) == number_of_pixels {
            pixels + 1
        } else {
            pixels
        }
    }

    /// `LeadingEdge`.
    fn leading_edge(&self, length: i32, number_of_pixels: i32, thickness: i32, index: i32) -> bool {
        number_of_pixels <= length - (thickness - index) || (number_of_pixels <= length && self.drawing_tail)
    }

    /// `GetPixelPosition`.
    #[allow(clippy::too_many_arguments)]
    fn pixel_position(&self, side: Side, x: i32, y: i32, pixel: i32, offset: i32, index: i32, width: i32, _height: i32) -> (i32, i32) {
        let mut rotation = self.rotation;
        if (self.direction == Dir::Out && !self.internal_reverse_draw) || (self.direction == Dir::In && self.reverse_draw) {
            rotation = if rotation == Rot::Clockwise { Rot::CounterClockwise } else { Rot::Clockwise };
        }
        let (w, h) = (self.width, self.height);
        if rotation == Rot::CounterClockwise {
            match side {
                Side::Bottom => ((x + pixel) + offset, y + index),
                Side::Left => (x + index, h - 1 - (y + pixel + offset)),
                Side::Right => (x + width - 1 - index, pixel + y + offset),
                Side::Top => (w - 1 - x - pixel + offset, h - 1 - y - index),
            }
        } else {
            match side {
                Side::Bottom => (w - 1 - (x + pixel) + offset, y + index),
                Side::Left => (x + index, (y + pixel) + offset),
                Side::Right => (x + width - 1 - index, h - 1 - (y + pixel) + offset),
                Side::Top => (x + pixel + offset, h - 1 - y - index),
            }
        }
    }

    /// `DrawSidePixels`.
    #[allow(clippy::too_many_arguments)]
    fn draw_side_pixels(&mut self, fb: &mut PixelFrameBuffer, side: Side, x: i32, y: i32, offset: i32, index: i32, width: i32, height: i32, interval_pos: f64, remaining: i32, first_pass: bool, num_of_pixels: i32) {
        let mut remaining = remaining;
        let mut pixel = if first_pass || self.mode == Mode::Meteor { 0 } else { index };
        while pixel < num_of_pixels {
            let (px, py) = self.pixel_position(side, x, y, pixel, offset, index, width, height);
            if let Some(color) = self.get_color(side, interval_pos, index, remaining) {
                self.set_pixel(fb, px, py, color, remaining);
            }
            remaining = remaining.wrapping_sub(1);
            pixel += 1;
        }
    }

    /// `DrawSide` (all eight `DrawXxxSideClockwise` wrappers call it).
    #[allow(clippy::too_many_arguments)]
    fn draw_side(&mut self, side: Side, fb: &mut PixelFrameBuffer, x: i32, y: i32, width: i32, height: i32, thickness: i32, number_of_pixels: i32, max_pixels: i32, interval_pos: f64, offset: i32, first_pass: bool, remaining: i32) {
        let saved_remaining = remaining;
        let mut next_color_index = 0;
        let mut next_band_pixel_count = 0;
        let color_index = self.color_index;
        let band_pixel_count = self.band_pixel_count;
        for index in 0..thickness {
            self.color_index = color_index;
            self.band_pixel_count = band_pixel_count;
            let leading = self.leading_edge(self.calc_num_pixels(max_pixels, number_of_pixels), number_of_pixels, thickness, index);
            if !first_pass {
                for _ in 0..index {
                    self.update_selected_color_band();
                }
            }
            let remaining = saved_remaining;
            if self.mode == Mode::Meteor && !leading {
                self.draw_side_pixels(fb, side, x, y, offset, index, width, height, interval_pos, remaining, first_pass, number_of_pixels);
            } else {
                let mut num = if self.drawing_tail {
                    self.calc_num_pixels(max_pixels, number_of_pixels)
                } else {
                    self.calc_num_pixels(max_pixels, number_of_pixels) - index
                };
                num = number_of_pixels.min(num);
                self.draw_side_pixels(fb, side, x, y, offset, index, width, height, interval_pos, remaining, first_pass, num);
            }
            if index == 0 {
                next_color_index = self.color_index;
                next_band_pixel_count = self.band_pixel_count;
            }
        }
        self.color_index = next_color_index;
        self.band_pixel_count = next_band_pixel_count;
    }

    /// `DrawIn`.
    #[allow(clippy::too_many_arguments)]
    fn draw_in(&mut self, fb: &mut PixelFrameBuffer, x: i32, y: i32, width: i32, height: i32, number_of_pixels: i32, t: i32, s: i32, interval_pos: f64, length1: i32, length2: i32, sides: [Side; 4], offset: i32, first_pass: bool) {
        let mut n = number_of_pixels;
        let (length, length_offset) = if first_pass { (length1 - 1, 0) } else { (length1 + t + s - 1, offset) };
        let take = self.calc_num_pixels(length, n).min(n);
        self.draw_side(sides[0], fb, x, y, width, height, t, take, length, interval_pos, length_offset, first_pass, n);
        n -= take;
        if n <= 0 {
            return;
        }
        let take = self.calc_num_pixels(length2 - 1, n).min(n);
        self.draw_side(sides[1], fb, x, y, width, height, t, take, length2 - 1, interval_pos, 0, false, n);
        n -= take;
        if n <= 0 {
            return;
        }
        let take = self.calc_num_pixels(length1 - 1, n).min(n);
        self.draw_side(sides[2], fb, x, y, width, height, t, take, length1 - 1, interval_pos, 0, false, n);
        n -= take;
        if n <= 0 {
            return;
        }
        let max4 = length2 - t - s - 1;
        let take = self.calc_num_pixels(max4, n).min(n);
        self.draw_side(sides[3], fb, x, y, width, height, t, take, max4, interval_pos, 0, false, n);
        n -= take;
        let shrink = 2 * (t + s);
        let (w, h) = (width - shrink, height - shrink);
        if w >= t && h >= t {
            self.update_color();
            self.draw_in(fb, x + t + s, y + t + s, w, h, n, t, s, interval_pos, length1 - shrink, length2 - shrink, sides, offset, false);
        }
    }

    /// `DrawOut`.
    #[allow(clippy::too_many_arguments)]
    fn draw_out(&mut self, fb: &mut PixelFrameBuffer, x: i32, y: i32, width: i32, height: i32, number_of_pixels: i32, t: i32, s: i32, interval_pos: f64, length1: i32, length2: i32, first_pass: bool, parts: [(bool, Side); 4], offset: i32) {
        let mut n = number_of_pixels;
        let (flag1, side1) = parts[0];
        if flag1 {
            let max = length1 - t - s - 1;
            let take = max.min(n);
            self.draw_side(side1, fb, x, y, width, height, t, take, max, interval_pos, offset, first_pass, n);
            n -= take;
        }
        if n <= 0 {
            return;
        }
        let (flag2, side2) = parts[1];
        if flag2 {
            let max = length2 - 1;
            let take = max.min(n);
            self.draw_side(side2, fb, x, y, width, height, t, take, max, interval_pos, 0, first_pass && !flag1, n);
            n -= take;
        }
        if n <= 0 {
            return;
        }
        let (flag3, side3) = parts[2];
        if flag3 {
            let max = length1 - 1;
            let take = max.min(n);
            self.draw_side(side3, fb, x, y, width, height, t, take, max, interval_pos, 0, first_pass && !flag1 && !flag2, n);
            n -= take;
        }
        if n > 0 {
            let (flag4, side4) = parts[3];
            if flag4 {
                let max = length2 + t + s - 1;
                let take = n.min(max);
                self.draw_side(side4, fb, x, y, width, height, t, take, max, interval_pos, 0, first_pass && !flag1 && !flag2 && !flag3, n);
                n -= take;
            }
        }
        if n > 0 {
            let grow = 2 * (t + s);
            let (w, h) = (width + grow, height + grow);
            if w <= self.width && h <= self.height {
                self.update_color();
                self.draw_out(fb, x - t - s, y - t - s, w, h, n, t, s, interval_pos, length1 + grow, length2 + grow, false, [(true, side1), (true, side2), (true, side3), (true, parts[3].1)], offset);
            }
        }
    }

    /// The `DrawXxxIn` family: which dimension leads, the order of the sides and the sign of the offset.
    fn draw_whirl_in(&mut self, fb: &mut PixelFrameBuffer, n: i32, t: i32, s: i32, interval_pos: f64) {
        use Side::*;
        let (w, h) = (self.width, self.height);
        let ts = t + s;
        // (length1, length2) is (width, height) or (height, width).
        let (swap, sides, offset): (bool, [Side; 4], i32) = match (self.start, self.rotation) {
            (Start::TopLeft, Rot::Clockwise) => (false, [Top, Right, Bottom, Left], -ts),
            (Start::BottomLeft, Rot::Clockwise) => (true, [Left, Top, Right, Bottom], -ts),
            (Start::BottomRight, Rot::CounterClockwise) => (true, [Right, Top, Left, Bottom], -ts),
            (Start::BottomLeft, Rot::CounterClockwise) => (false, [Bottom, Right, Top, Left], -ts),
            (Start::BottomRight, Rot::Clockwise) => (false, [Bottom, Left, Top, Right], ts),
            (Start::TopLeft, Rot::CounterClockwise) => (true, [Left, Bottom, Right, Top], -ts),
            (Start::TopRight, Rot::Clockwise) => (true, [Right, Bottom, Left, Top], ts),
            (Start::TopRight, Rot::CounterClockwise) => (false, [Top, Left, Bottom, Right], ts),
        };
        let (l1, l2) = if swap { (h, w) } else { (w, h) };
        self.draw_in(fb, 0, 0, w, h, n, t, s, interval_pos, l1, l2, sides, offset, true);
    }

    /// The `DrawXxxOut` family, starting from the last rectangle the setup found.
    fn draw_whirl_out(&mut self, fb: &mut PixelFrameBuffer, n: i32, t: i32, s: i32, interval_pos: f64) {
        use Side::*;
        let ts = s + t;
        let (swap, sides, offset): (bool, [Side; 4], i32) = match (self.start, self.rotation) {
            (Start::BottomRight, Rot::CounterClockwise) => (false, [Bottom, Left, Top, Right], -ts),
            (Start::TopRight, Rot::Clockwise) => (false, [Top, Left, Bottom, Right], -ts),
            (Start::TopLeft, Rot::Clockwise) => (true, [Left, Bottom, Right, Top], ts),
            (Start::BottomLeft, Rot::Clockwise) => (false, [Bottom, Right, Top, Left], ts),
            (Start::TopRight, Rot::CounterClockwise) => (true, [Right, Bottom, Left, Top], -ts),
            (Start::BottomRight, Rot::Clockwise) => (true, [Right, Top, Left, Bottom], ts),
            (Start::BottomLeft, Rot::CounterClockwise) => (true, [Left, Top, Right, Bottom], ts),
            (Start::TopLeft, Rot::CounterClockwise) => (false, [Top, Right, Bottom, Left], ts),
        };
        let v = self.vortex.clone();
        let (l1, l2) = if swap { (v.last_height, v.last_width) } else { (v.last_width, v.last_height) };
        let parts = [(v.draws(sides[0]), sides[0]), (v.draws(sides[1]), sides[1]), (v.draws(sides[2]), sides[2]), (v.draws(sides[3]), sides[3])];
        self.draw_out(fb, v.last_x, v.last_y, v.last_width, v.last_height, n, t, s, interval_pos, l1, l2, true, parts, offset);
    }

    // ---- Concentric whirls

    /// `GetSymmetricalPixelPosition`.
    fn symmetrical_position(&self, side: Side, x: i32, y: i32, pixel: i32, index: i32, width: i32) -> (i32, i32) {
        match side {
            Side::Bottom => (x + pixel, y + index),
            Side::Top => (self.width - 1 - x - pixel, self.height - 1 - y - index),
            Side::Left => (x + index, self.height - 1 - (y + pixel)),
            Side::Right => (x + width - 1 - index, y + pixel),
        }
    }

    /// `DrawSideSymmetryWhirl`.
    #[allow(clippy::too_many_arguments)]
    fn draw_side_symmetry(&mut self, fb: &mut PixelFrameBuffer, x: i32, y: i32, width: i32, thickness: i32, interval_pos: f64, length: i32, side: Side) {
        let color_index = self.color_index;
        let band_pixel_count = self.band_pixel_count;
        let mut next_color_index = 0;
        let mut next_band_pixel_count = 0;
        for index in 0..thickness {
            self.color_index = color_index;
            self.band_pixel_count = band_pixel_count;
            for _ in 0..index {
                self.update_selected_color_band();
            }
            let mut pixel = index;
            while pixel < length - index - 1 {
                let color = self.get_color(side, interval_pos, index, 0);
                let (px, py) = self.symmetrical_position(side, x, y, pixel, index, width);
                if let Some(color) = color {
                    self.set_pixel(fb, px, py, color, 0);
                }
                pixel += 1;
            }
            if index == 0 {
                next_color_index = self.color_index;
                next_band_pixel_count = self.band_pixel_count;
            }
        }
        self.color_index = next_color_index;
        self.band_pixel_count = next_band_pixel_count;
    }

    fn draw_four_symmetry(&mut self, fb: &mut PixelFrameBuffer, x: i32, y: i32, width: i32, height: i32, t: i32, interval_pos: f64) {
        self.draw_side_symmetry(fb, x, y, width, t, interval_pos, height, Side::Left);
        self.draw_side_symmetry(fb, x, y, width, t, interval_pos, width, Side::Bottom);
        self.draw_side_symmetry(fb, x, y, width, t, interval_pos, height, Side::Right);
        self.draw_side_symmetry(fb, x, y, width, t, interval_pos, width, Side::Top);
    }

    /// `DrawWhirlSymmetricalOut`.
    #[allow(clippy::too_many_arguments)]
    fn symmetrical_out(&mut self, fb: &mut PixelFrameBuffer, x: i32, y: i32, width: i32, height: i32, t: i32, s: i32, interval_pos: f64, levels: i32) {
        self.draw_four_symmetry(fb, x, y, width, height, t, interval_pos);
        let (w, h) = (width + 2 * (t + s), height + 2 * (t + s));
        if w <= self.width && h <= self.height && levels > 0 {
            self.update_color();
            self.symmetrical_out(fb, x - t - s, y - t - s, w, h, t, s, interval_pos, levels - 1);
        }
    }

    /// `DrawWhirlSymmetricalIn`.
    #[allow(clippy::too_many_arguments)]
    fn symmetrical_in(&mut self, fb: &mut PixelFrameBuffer, x: i32, y: i32, width: i32, height: i32, t: i32, s: i32, interval_pos: f64, levels: i32) {
        self.draw_four_symmetry(fb, x, y, width, height, t, interval_pos);
        let (w, h) = (width - 2 * (t + s), height - 2 * (t + s));
        if w >= t && h >= t && levels > 0 {
            self.update_color();
            self.symmetrical_in(fb, x + t + s, y + t + s, w, h, t, s, interval_pos, levels - 1);
        }
    }

    // ---- Per frame

    /// `RenderEffect`: `frame` is the frame within the iteration.
    pub fn render_effect(&mut self, frame: i32, fb: &mut PixelFrameBuffer, interval_pos: f64) {
        if self.direction == Dir::InAndOut && frame == self.number_of_frames / 2 {
            self.internal_reverse_draw = !self.reverse_draw;
            self.reset_pixel_number();
        }
        if self.internal_reverse_draw {
            self.number_of_pixels_to_draw -= self.pixels_to_draw_per_frame;
        } else {
            self.number_of_pixels_to_draw += self.pixels_to_draw_per_frame;
            if self.number_of_pixels_to_draw > self.total_number_of_pixels {
                self.number_of_pixels_to_draw = self.total_number_of_pixels;
            }
        }
        let (t, s) = (self.calc_thickness(self.thickness), self.calc_spacing(self.spacing));
        if self.number_of_pixels_to_draw >= self.total_number_of_pixels - t as f64 {
            self.drawing_tail = true;
        }
        self.color_index = 0;
        self.band_pixel_count = 0;

        if self.mode == Mode::Symmetrical {
            if self.frames_per_rectangle == 0 {
                self.threw = true;
                return;
            }
            let mut levels = frame / self.frames_per_rectangle;
            let (w, h) = (self.width, self.height);
            match self.direction {
                Dir::In => self.symmetrical_in(fb, 0, 0, w, h, t, s, interval_pos, levels),
                Dir::Out => {
                    let v = self.vortex.clone();
                    self.symmetrical_out(fb, v.last_x, v.last_y, v.last_width, v.last_height, t, s, interval_pos, levels);
                }
                Dir::InAndOut => {
                    if frame < self.number_of_frames / 2 {
                        self.symmetrical_in(fb, 0, 0, w, h, t, s, interval_pos, levels);
                    } else {
                        levels = self.number_of_symmetric_rectangles - (frame - self.number_of_frames / 2) / self.frames_per_rectangle;
                        if levels > 0 {
                            self.symmetrical_in(fb, 0, 0, w, h, t, s, interval_pos, levels);
                        }
                    }
                }
            }
        } else {
            let n = self.number_of_pixels_to_draw as i32;
            let draw_in = match self.direction {
                Dir::In => !self.reverse_draw,
                Dir::Out => self.reverse_draw,
                Dir::InAndOut => true,
            };
            if draw_in {
                self.draw_whirl_in(fb, n, t, s, interval_pos);
            } else {
                self.draw_whirl_out(fb, n, t, s, interval_pos);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(fb: &PixelFrameBuffer) -> i32 {
        let mut n = 0;
        for x in 0..fb.width() {
            for y in 0..fb.height() {
                if fb.get_color_at(x, y) != Color::EMPTY {
                    n += 1;
                }
            }
        }
        n
    }

    /// With 1 pixel thickness the number of pixels the setup counts for the whole spiral is exactly the
    /// number the drawing lights, for every corner and rotation. The two are separate code paths in the C#.
    #[test]
    fn counted_pixels_equal_drawn_pixels() {
        for (w, h) in [(10, 10), (13, 8), (7, 12), (9, 9), (20, 15)] {
            for spacing in 0..4 {
                for start in [Start::TopLeft, Start::BottomLeft, Start::TopRight, Start::BottomRight] {
                    for rotation in [Rot::Clockwise, Rot::CounterClockwise] {
                        let mut whirl = Whirl::new();
                        whirl.width = w;
                        whirl.height = h;
                        whirl.start = start;
                        whirl.rotation = rotation;
                        whirl.thickness = (100.0 / w.max(h) as f64 * 1.0).ceil() as i32; // one pixel
                        whirl.spacing = spacing * 100 / w.max(h);
                        let (t, s) = (whirl.calc_thickness(whirl.thickness), whirl.calc_spacing(whirl.spacing));
                        assert_eq!(t, 1);
                        whirl.setup_render(100);
                        let total = whirl.total_number_of_pixels as i32;
                        let mut fb = PixelFrameBuffer::new(w, h);
                        fb.fill(Color::EMPTY);
                        whirl.number_of_pixels_to_draw = total as f64;
                        whirl.drawing_tail = true;
                        whirl.draw_whirl_in(&mut fb, total, t, s, 0.5);
                        assert_eq!(lit(&fb), total, "{w}x{h} s={s} {start:?} {rotation:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn symmetrical_rectangle_count() {
        let mut whirl = Whirl::new();
        whirl.width = 20;
        whirl.height = 20;
        // thickness 1 and spacing 1: a rectangle fits every 4 pixels, 20 -> 16 -> ... -> 4.
        assert_eq!(whirl.calc_rectangles(0, 0, 20, 20, 1, 1), 5);
        assert_eq!((whirl.vortex.last_width, whirl.vortex.last_height), (4, 4));
    }
}
