//! Port of VixenModules.Effect.Wave. Each waveform keeps a queue of pixel columns that is advanced at a
//! fixed 20 Hz and drawn into the frame buffer in the waveform's direction.

use crate::generated::wave as gen;
use std::collections::VecDeque;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    param_changed_ctx: Some(param_changed_ctx),
    supports_location: true,
    ported: true,
    ..gen::BASE
};

const MAX_WAVE_VELOCITY: i32 = 10;
const MAX_WAVE_THICKNESS: i32 = 100;
const MAX_WAVE_FREQUENCY: i32 = 3600;
const MIN_WAVE_FREQUENCY: i32 = 180;
const MAX_WAVE_HEIGHT: i32 = 100;
const MIN_WAVE_Y_OFFSET: i32 = -250;
const MAX_WAVE_Y_OFFSET: i32 = 250;
const PI_180: f64 = std::f64::consts::PI / 180.0;

/// `UpdateStringOrientationAttributes` and `EnableTargetPositioning(true, true)`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    v.set("StringOrientation", p.enum_name("TargetPositioning") == "Strings");
    v.set("TargetPositioning", true);
}

/// `Waveform.InitAllAttributes` and the setters' `UpdateWaveTypeAttributes`, `UpdateMovementTypeAttributes` and
/// `UpdateUseMarkAttributes`, for an editor showing one wave.
pub(crate) fn waveform_visibility(wave: &Params, _effect: &Params, v: &mut Visibility) {
    let wave_type = wave.enum_name("WaveType");
    let fractal = wave_type == "FractalIvey";
    let decaying = wave_type == "DecayingSine";
    v.set("Frequency", !fractal);
    v.set("Height", !fractal);
    v.set("YOffset", !fractal);
    v.set("PrimeWave", !fractal && !decaying);
    v.set("MovementType", !fractal && !decaying);
    v.set("PhaseShift", !fractal && !decaying);
    v.set("UseMarks", decaying);
    v.set("MarkCollectionId", decaying && wave.boolean("UseMarks"));
    v.set("WindowPercentage", wave.enum_name("MovementType") == "Snake");
}

/// `GetMarkCollectionSelections` and the wave's setters. An edited wave reaches this as the whole list.
fn param_changed_ctx(c: &ParamChange, p: &mut Params) {
    if !matches!(c.name, MARK_COLLECTIONS | "WaveformData") {
        return;
    }
    let Some(i) = p.index_of("WaveformData") else { return };
    let ParamValue::RecordList(waves) = &mut p.values[i] else { return };
    let old_waves: &[Params] = if let ParamValue::RecordList(old) = c.old { old } else { &[] };
    // Wave.OnWavesChildPropertyChanged activates every active selection only
    // for WaveType/UseMarks edits. Assigning an explicit missing/nil ID, or
    // editing another child property, keeps that ID until the next activation
    // or MarkCollections assignment. Add/remove changes collection membership,
    // not a child's mode; comparing shifted row indices would falsely activate.
    let normalize_marks = c.name == MARK_COLLECTIONS
        || (old_waves.len() == waves.len()
            && old_waves.iter().zip(waves.iter()).any(|(old, wave)| {
                old.enum_name("WaveType") != wave.enum_name("WaveType") || old.boolean("UseMarks") != wave.boolean("UseMarks")
            }));
    for (row, wave) in waves.iter_mut().enumerate() {
        // The `MovementType` setter primes the wave when it changes to a snake.
        let was_snake = old_waves.get(row).is_some_and(|o| o.enum_name("MovementType") == "Snake");
        if c.name == "WaveformData" && !was_snake && wave.enum_name("MovementType") == "Snake" {
            wave.set("PrimeWave", ParamValue::Bool(true));
        }
        let active = wave.enum_name("WaveType") == "DecayingSine" && wave.boolean("UseMarks");
        if normalize_marks && active && !(c.info.targets.is_empty() && c.info.marks.is_empty()) {
            let id = normalized_mark_selection(wave.guid("MarkCollectionId"), c.info.marks, None, true);
            wave.set("MarkCollectionId", ParamValue::Guid(id));
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum WaveType {
    Sine,
    Triangle,
    Square,
    DecayingSine,
    FractalIvey,
    Sawtooth,
}

#[derive(Copy, Clone, PartialEq)]
enum Direction {
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
}

#[derive(Copy, Clone, PartialEq)]
enum Movement {
    Continuous,
    Snake,
    GrowAndShrink,
}

type Column = Vec<(Color, i32)>;

/// An `IWaveform`: the settings plus the state the rendering keeps between columns.
struct Waveform {
    wave_type: WaveType,
    movement: Movement,
    direction: Direction,
    across: bool,
    mirror: bool,
    prime_wave: bool,
    window_percentage: i32,
    phase_shift: i32,
    speed: Curve,
    thickness: Curve,
    frequency: Curve,
    height: Curve,
    y_offset: Curve,
    color: ColorGradient,
    /// The selected mark collection, when its id matches one of the effect's collections.
    marks: Option<MarkCollection>,

    buffer_ht: i32,
    buffer_wi: i32,
    pixels: VecDeque<Column>,
    ivey_y_values: Vec<i32>,
    decay_factor: i32,
    saw_tooth_x: i32,
    saw_tooth_max_y: i32,
    saw_tooth_min_y: i32,
    last_saw_tooth_y: i32,
    last_last_saw_tooth_y: i32,
    degrees: f64,
    shrink: bool,
    window_start: i32,
    window_stop: i32,
    triangle_counter: i32,
    last_wave_thickness: i32,
}

impl Waveform {
    fn from_params(d: &Params, marks: &[MarkCollection]) -> Waveform {
        let movement = match d.enum_name("MovementType") {
            "Snake" => Movement::Snake,
            "GrowAndShrink" => Movement::GrowAndShrink,
            _ => Movement::Continuous,
        };
        let id = d.guid("MarkCollectionId");
        Waveform {
            wave_type: match d.enum_name("WaveType") {
                "Triangle" => WaveType::Triangle,
                "Square" => WaveType::Square,
                "DecayingSine" => WaveType::DecayingSine,
                "FractalIvey" => WaveType::FractalIvey,
                "Sawtooth" => WaveType::Sawtooth,
                _ => WaveType::Sine,
            },
            movement,
            direction: match d.enum_name("Direction") {
                "RightToLeft" => Direction::RightToLeft,
                "TopToBottom" => Direction::TopToBottom,
                "BottomToTop" => Direction::BottomToTop,
                _ => Direction::LeftToRight,
            },
            across: d.enum_name("ColorHandling") == "Across",
            mirror: d.boolean("Mirror"),
            // The MovementType setter forces PrimeWave on for Snake.
            prime_wave: d.boolean("PrimeWave") || movement == Movement::Snake,
            window_percentage: d.int("WindowPercentage"),
            phase_shift: d.int("PhaseShift"),
            speed: d.curve("Speed"),
            thickness: d.curve("Thickness"),
            frequency: d.curve("Frequency"),
            height: d.curve("Height"),
            y_offset: d.curve("YOffset"),
            color: d.gradient("Color"),
            marks: marks.iter().find(|m| m.id == id).cloned(),
            buffer_ht: 0,
            buffer_wi: 0,
            pixels: VecDeque::new(),
            ivey_y_values: Vec::new(),
            decay_factor: 0,
            saw_tooth_x: 0,
            saw_tooth_max_y: 0,
            saw_tooth_min_y: i32::MAX,
            last_saw_tooth_y: 0,
            last_last_saw_tooth_y: 0,
            degrees: 0.0,
            shrink: false,
            window_start: 0,
            window_stop: 0,
            triangle_counter: 0,
            last_wave_thickness: 0,
        }
    }

    /// `YC`.
    #[inline]
    fn yc(&self) -> f64 {
        self.buffer_ht as f64 / 2.0
    }

    /// `GetDegreesPerX`.
    #[inline]
    fn degrees_per_x(&self, number_of_waves: i32) -> f64 {
        number_of_waves as f64 / self.buffer_wi as f64
    }
}

/// The values `Wave.RenderWave` receives for one waveform and 50 ms step.
#[derive(Copy, Clone)]
struct Step {
    frame: i32,
    number_of_waves: i32,
    thickness: i32,
    height: i32,
    speed: i32,
    y_offset: i32,
    color: Color,
}

#[derive(Copy, Clone)]
enum ColumnKind {
    Sine,
    Square,
    Triangle,
    Sawtooth,
}

struct Wave {
    param_waves: Vec<Params>,
    marks: Vec<MarkCollection>,
    waves: Vec<Waveform>,
    rand: Rand,
    buffer_ht: i32,
    buffer_wi: i32,
    speed_increment: i32,
    frame_50ms: i32,
    frame_time: i32,
    max_pixels_per_string: i32,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut e = Wave {
        param_waves: p.records("WaveformData").to_vec(),
        marks: ctx.marks.to_vec(),
        waves: Vec::new(),
        rand: Rand::default(),
        buffer_ht: 0,
        buffer_wi: 0,
        speed_increment: 1,
        frame_50ms: 0,
        frame_time: 0,
        max_pixels_per_string: 0,
    };
    render_pixel_effect(ctx, p, &mut e)
}

#[inline]
fn sine(degree: f64) -> f64 {
    (degree * PI_180).sin()
}

/// `Math.Sign(double)`, which throws on NaN in C#. NaN reads as 0 here.
#[inline]
fn sign(v: f64) -> i32 {
    if v > 0.0 {
        1
    } else if v < 0.0 {
        -1
    } else {
        0
    }
}

fn calculate_bottom_of_wave(ystart: f64, r: f64, thickness: i32) -> i32 {
    (ystart - (r * (thickness as f64 / 100.0))).floor() as i32
}

fn calculate_top_of_wave(ystart: f64, r: f64, thickness: i32) -> i32 {
    (ystart + (r * (thickness as f64 / 100.0))).ceil() as i32
}

fn calculate_top_and_bottom(ystart: f64, r: f64, thickness: i32) -> (i32, i32) {
    let y1 = calculate_bottom_of_wave(ystart, r, thickness);
    let mut y2 = calculate_top_of_wave(ystart, r, thickness);
    if y2 <= y1 {
        y2 = y1 + 1; // minimum height
    }
    (y1, y2)
}

impl Waveform {
    /// `PopulateYPositions`.
    fn populate_y_positions(&self, y1: i32, y2: i32, rounded_offset: i32, column: &mut Column, fill: Color) {
        let mut fill = fill;
        let mut y = 0;
        let mut y_new = y1;
        while y_new < y2 {
            if self.across {
                fill = self.color.get_color_at_f32(y as f32 / (y2 - y1) as f32);
            }
            column.push((fill, y_new.wrapping_add(rounded_offset)));
            y_new += 1;
            y += 1;
        }
    }

    /// `PopulateYCoordinates`.
    fn populate_y_coordinates(&mut self, y1: i32, y2: i32, yoffset: i32, fill: Color) {
        let mut column: Column = Vec::new();
        let yc = self.yc();
        let wave_y_offset = (self.buffer_ht as f64 / 2.0) * (yoffset as f64 * 0.01);
        let rounded = wave_y_offset.round_ties_even() as i32;
        self.populate_y_positions(y1, y2, rounded, &mut column, fill);
        if self.mirror {
            let y1m = (yc + (yc - y1 as f64)) as i32;
            let y2m = (yc + (yc - y2 as f64)) as i32;
            let (a, b) = if y1m < y2m { (y1m, y2m) } else { (y2m, y1m) };
            self.populate_y_positions(a, b, rounded, &mut column, fill);
        }
        self.pixels.push_back(column);
    }

    /// `RenderVerticalLines`.
    fn render_vertical_lines(&mut self, y1: i32, y2: i32, yoffset: i32, lines: i32, fill: Color) {
        for _ in 0..lines.max(0) {
            self.populate_y_coordinates(y1, y2, yoffset, fill);
        }
    }

    /// `RenderPixelToFrameBuffer`. A `None` buffer is the priming pass.
    #[inline]
    fn render_pixel(&self, raw_index: i32, x: i32, y: i32, fill: Color, fb: &mut Option<&mut PixelFrameBuffer>) {
        if let Some(fb) = fb {
            if self.movement == Movement::Snake {
                if self.window_start <= raw_index && raw_index <= self.window_stop {
                    fb.set_pixel(x, y, fill);
                }
            } else {
                fb.set_pixel(x, y, fill);
            }
        }
    }

    /// `RenderColumns`.
    fn render_columns(&mut self, mut fb: Option<&mut PixelFrameBuffer>, wspeed: i32, update_window: bool) {
        while self.pixels.len() as i64 > self.buffer_wi as i64 {
            self.pixels.pop_front();
        }
        if update_window {
            self.window_start += wspeed;
            self.window_stop += wspeed;
            if self.window_start >= self.buffer_wi {
                self.window_start = 0;
                self.window_stop = (self.buffer_wi as f64 * (self.window_percentage as f64 / 100.0)) as i32;
            }
        }
        if fb.is_none() {
            return;
        }
        let direction = self.direction;
        let wi = self.buffer_wi;
        for (index, column) in self.pixels.iter().enumerate() {
            let index = index as i32;
            for &(color, y_pos) in column {
                match direction {
                    Direction::LeftToRight => self.render_pixel(index, index, y_pos, color, &mut fb),
                    Direction::BottomToTop => self.render_pixel(index, y_pos, index, color, &mut fb),
                    Direction::RightToLeft => self.render_pixel(index, wi - index - 1, y_pos, color, &mut fb),
                    Direction::TopToBottom => self.render_pixel(index, y_pos, wi - index - 1, color, &mut fb),
                }
            }
        }
    }

    /// `GrowAndShrink`, which every column-based wave type goes through.
    fn grow_and_shrink(&mut self, st: &Step, wspeed: i32, kind: ColumnKind, fb: Option<&mut PixelFrameBuffer>) {
        if self.movement == Movement::GrowAndShrink && self.pixels.len() as i64 >= self.buffer_wi as i64 {
            self.shrink = true;
        }
        if !self.shrink {
            for _ in 0..wspeed {
                match kind {
                    ColumnKind::Sine => self.column_sine(st),
                    ColumnKind::Square => self.square_column(st),
                    ColumnKind::Triangle => self.column_triangle(st),
                    ColumnKind::Sawtooth => self.saw_tooth_column(st),
                }
            }
        } else {
            for _ in 0..wspeed {
                if !self.pixels.is_empty() {
                    // `QueueExtension.Pop`: removes the last item added.
                    self.pixels.pop_back();
                }
            }
            if self.pixels.is_empty() {
                self.shrink = false;
            }
        }
        self.render_columns(fb, wspeed, true);
    }

    // ---- Sine

    fn column_sine(&mut self, st: &Step) {
        let sinrad = sine(self.degrees + self.phase_shift as f64);
        self.degrees += self.degrees_per_x(st.number_of_waves);
        let yc = self.yc();
        self.sine_column_at(sinrad, st, yc);
    }

    /// `RenderColumnSine(sinrad, ..., r, ...)`.
    fn sine_column_at(&mut self, sinrad: f64, st: &Step, r: f64) {
        let ystart = r * (st.height as f64 / 100.0) * sinrad + self.yc();
        let (y1, y2) = calculate_top_and_bottom(ystart, r, st.thickness);
        self.populate_y_coordinates(y1, y2, st.y_offset, st.color);
    }

    // ---- Square

    fn square_column(&mut self, st: &Step) {
        let sinrad_minus1 = sine(self.degrees + self.phase_shift as f64);
        self.degrees += self.degrees_per_x(st.number_of_waves);
        let sinrad = sine(self.degrees + self.phase_shift as f64);

        let yc = self.yc();
        let x = self.pixels.len() as i32;
        let (y1, y2, vertical) = self.square_y(st, sinrad, sinrad_minus1, yc, x);
        self.populate_y_coordinates(y1, y2, st.y_offset, st.color);
        if vertical {
            self.render_vertical_lines(y1, y2, st.y_offset, self.last_wave_thickness - 1, st.color);
        }
    }

    /// The overload of `RenderSquareWave` that calculates the Y coordinates.
    fn square_y(&mut self, st: &Step, sinrad: f64, sinrad_minus1: f64, yc: f64, x: i32) -> (i32, i32, bool) {
        let mut vertical = false;
        let (mut y1, mut y2);
        let h = st.height as f64 / 100.0;
        if sign(sinrad) != sign(sinrad_minus1) && x != 1 {
            y1 = (yc - yc * h) as i32;
            y2 = (yc + yc * h).ceil() as i32;
            vertical = true;
        } else if sinrad > 0.0 {
            y1 = (yc + 1.0 + yc * h * ((100.0 - st.thickness as f64) / 100.0)) as i32;
            y2 = (yc + yc * h).ceil() as i32;
        } else {
            y1 = (yc - yc * h) as i32;
            y2 = (yc - yc * h * ((100.0 - st.thickness as f64) / 100.0)) as i32;
        }
        if y1 < 0 {
            y1 = 0;
        }
        if y2 < 1 {
            y2 = 1;
        }
        if y1 > self.buffer_ht - 1 {
            y1 = self.buffer_ht - 1;
        }
        if y2 > self.buffer_ht {
            y2 = self.buffer_ht;
        }
        if y2 <= y1 {
            y2 = y1 + 1;
        }
        if !vertical {
            self.last_wave_thickness = y2 - y1;
        }
        (y1, y2, vertical)
    }

    // ---- Triangle

    fn column_triangle(&mut self, st: &Step) {
        let ystart = self.triangle_y_start(st.number_of_waves, st.height, self.triangle_counter.wrapping_add(self.phase_shift));
        self.triangle_counter += 1;
        let yc = self.yc();
        let (y1, y2) = calculate_top_and_bottom(ystart as f64, yc, st.thickness);
        self.populate_y_coordinates(y1, y2, st.y_offset, st.color);
    }

    fn triangle_y_start(&self, number_of_waves: i32, wave_height: i32, x: i32) -> i32 {
        let waves = number_of_waves as f64 / 180.0 / 5.0;
        let amp = self.buffer_ht.wrapping_mul(wave_height) / 100;
        let mut y_start;
        if amp == 0 {
            y_start = 0;
        } else {
            y_start = (self.buffer_ht - amp) / 2 + (((x as f64 * waves) as i32) % (2 * amp) - amp).wrapping_abs();
        }
        if y_start > self.buffer_ht - 1 {
            y_start = self.buffer_ht - 1;
        }
        y_start
    }

    // ---- Sawtooth

    fn saw_tooth_column(&mut self, st: &Step) {
        // `CalculateSlopeOfSawTooth`. The degrees per X use integer division, as in C#.
        let degree_per_x = (st.number_of_waves.checked_div(self.buffer_wi).unwrap_or(0)) as f64;
        let tooth_gap = (360.0 / degree_per_x).ceil() as i32;
        let yc = self.yc();
        let sy1 = (yc - yc * (st.height as f64 / 100.0)) as i32;
        let sy2 = (yc + yc * (st.height as f64 / 100.0)) as i32;
        let rise = (sy2 - sy1) as f64;
        let m = rise / tooth_gap as f64;

        let sinrad_minus1 = sine(self.degrees + self.phase_shift as f64);
        self.degrees += degree_per_x;
        let sinrad = sine(self.degrees + self.phase_shift as f64);

        let mut y1 = calculate_bottom_of_wave((yc as i32) as f64, yc, st.height);

        // `CalculateYPositionOnSawtooth`.
        let positive = sinrad >= 0.0;
        let minus1_negative = sinrad_minus1 < 0.0;
        if (positive && minus1_negative) || self.saw_tooth_x >= tooth_gap {
            self.saw_tooth_x = 1;
        } else {
            self.saw_tooth_x += 1;
        }
        let ystart = (y1 as f64 + m * self.saw_tooth_x as f64) as i32;

        let vertical = positive && minus1_negative;
        let y2;
        if vertical {
            y1 = self.saw_tooth_min_y;
            y2 = self.saw_tooth_max_y;
        } else {
            let (a, b) = calculate_top_and_bottom(ystart as f64, yc, st.thickness);
            y1 = a;
            y2 = b;
            self.last_wave_thickness = y2 - y1;
        }
        if self.saw_tooth_max_y < y2 {
            self.saw_tooth_max_y = y2;
        }
        if self.saw_tooth_min_y > y1 {
            self.saw_tooth_min_y = y1;
        }
        self.populate_y_coordinates(y1, y2, st.y_offset, st.color);
        if vertical {
            let lines = self.last_wave_thickness - (self.last_saw_tooth_y - self.last_last_saw_tooth_y);
            self.render_vertical_lines(y1, y2, st.y_offset, lines, st.color);
        }
        self.last_last_saw_tooth_y = self.last_saw_tooth_y;
        self.last_saw_tooth_y = ystart;
    }

    // ---- Fractal Ivey

    fn render_ivey(&mut self, st: &Step, wspeed: i32, fb: Option<&mut PixelFrameBuffer>) {
        let state = st.frame.wrapping_mul(wspeed);
        for x in 0..self.buffer_wi {
            // `RenderFractalIveyWave`.
            let eff_x = x + self.buffer_wi * (state / 2 / self.buffer_wi);
            if (eff_x as i64) >= st.number_of_waves as i64 * self.buffer_wi as i64 {
                break;
            }
            if eff_x > state / 2 {
                continue; // the ivy "grows"
            }
            // The C# indexes with the frequency at this moment, which can exceed the data made at setup and
            // throw. The effect would go dark; stop the columns here instead.
            let Some(&raw) = self.ivey_y_values.get(eff_x.max(0) as usize) else { break };
            let ystart = raw / 2;
            let yc = self.yc();
            let (y1, y2) = calculate_top_and_bottom(ystart as f64, yc, st.thickness);
            self.populate_y_coordinates_no_offset(y1, y2, st.color);
        }
        self.render_columns(fb, wspeed, true);
        self.pixels.clear();
    }

    /// `PopulateYCoordinates(y1, y2, yc, wave, 0, mirror, color)`.
    fn populate_y_coordinates_no_offset(&mut self, y1: i32, y2: i32, color: Color) {
        self.populate_y_coordinates(y1, y2, 0, color);
    }

    // ---- Decaying sine

    fn mark_name_present(&self) -> bool {
        self.marks.as_ref().is_some_and(|m| !m.name.is_empty())
    }

    /// `IsMarkActive`.
    fn is_mark_active(&self, frame: i32, x: &Timing) -> bool {
        let mut active = true;
        if let (true, Some(marks)) = (self.mark_name_present(), &self.marks) {
            let ms = x.frame_time.wrapping_mul(frame);
            let pos = x.start_time + TimeSpan::from_ms_i(ms as i64);
            let end = x.start_time + x.time_span;
            // The first mark `MarksInclusiveOfTime(pos, end)` returns.
            active = marks.marks_inclusive_of_time(pos, end).first().is_some_and(|m| m.start_time <= pos && pos <= m.end_time());
        }
        active
    }

    fn render_decaying_sine(&mut self, st: &Step, speed_increment: i32, timing: &Timing, fb: Option<&mut PixelFrameBuffer>) {
        let state = st.frame.wrapping_mul(st.speed);
        let mut r = self.yc();
        r -= self.decay_factor as f64;
        self.decay_factor = self.decay_factor.wrapping_add(speed_increment);
        if r <= 0.0 {
            if self.is_mark_active(st.frame, timing) {
                self.decay_factor = 0;
            } else {
                r = 0.0;
            }
        } else if self.mark_name_present() && self.is_mark_active(st.frame, timing) {
            self.decay_factor = 0;
        }
        for x in 0..self.buffer_wi {
            let sinrad = sine(x as f64 * self.degrees_per_x(st.number_of_waves) + state as f64 + self.phase_shift as f64);
            self.sine_column_at(sinrad, st, r);
        }
        self.render_columns(fb, st.speed, true);
        self.pixels.clear();
    }
}

/// The effect timing the decaying sine needs for its mark lookup.
struct Timing {
    frame_time: i32,
    start_time: TimeSpan,
    time_span: TimeSpan,
}

impl Wave {
    fn scale_i(curve: &Curve, pos: f64, max: i32, min: i32) -> i32 {
        scale_curve_to_value(curve.get_value(pos), max as f64, min as f64).round_ties_even() as i32
    }

    /// `CalculateWaveSpeed`.
    fn wave_speed(&self, w: &Waveform, pos: f64, allow_zero: bool) -> i32 {
        let mut speed = Wave::scale_i(&w.speed, pos, MAX_WAVE_VELOCITY.wrapping_mul(self.speed_increment), 0);
        if speed == 0 && !allow_zero {
            speed = 1;
        }
        speed
    }

    fn wave_thickness(w: &Waveform, pos: f64) -> i32 {
        Wave::scale_i(&w.thickness, pos, MAX_WAVE_THICKNESS, 0)
    }

    fn wave_frequency(w: &Waveform, pos: f64) -> i32 {
        let f = Wave::scale_i(&w.frequency, pos, MAX_WAVE_FREQUENCY, MIN_WAVE_FREQUENCY);
        if f == 0 {
            1
        } else {
            f
        }
    }

    fn wave_height(w: &Waveform, pos: f64) -> i32 {
        Wave::scale_i(&w.height, pos, MAX_WAVE_HEIGHT, 0)
    }

    fn wave_y_offset(w: &Waveform, pos: f64) -> i32 {
        Wave::scale_i(&w.y_offset, pos, MAX_WAVE_Y_OFFSET, MIN_WAVE_Y_OFFSET)
    }

    fn step(&self, w: &Waveform, frame: i32, interval_pos: f64, allow_zero: bool) -> Step {
        let factor = interval_pos * 100.0;
        Step {
            frame,
            number_of_waves: Wave::wave_frequency(w, factor),
            thickness: Wave::wave_thickness(w, factor),
            height: Wave::wave_height(w, factor),
            speed: self.wave_speed(w, factor, allow_zero),
            y_offset: Wave::wave_y_offset(w, factor),
            color: w.color.get_color_at(interval_pos),
        }
    }

    /// `RenderWave`.
    fn render_wave(w: &mut Waveform, st: &Step, speed_increment: i32, timing: &Timing, fb: Option<&mut PixelFrameBuffer>) {
        match w.wave_type {
            WaveType::Sine => w.grow_and_shrink(st, st.speed, ColumnKind::Sine, fb),
            WaveType::Square => w.grow_and_shrink(st, st.speed.wrapping_mul(2), ColumnKind::Square, fb),
            WaveType::Triangle => w.grow_and_shrink(st, st.speed, ColumnKind::Triangle, fb),
            WaveType::Sawtooth => w.grow_and_shrink(st, st.speed, ColumnKind::Sawtooth, fb),
            WaveType::FractalIvey => w.render_ivey(st, st.speed.wrapping_mul(2), fb),
            WaveType::DecayingSine => w.render_decaying_sine(st, speed_increment, timing, fb),
        }
    }

    /// `InitializeFractalIveyData`.
    fn init_ivey(&mut self, wi: usize) {
        let (buffer_wi, buffer_ht, yc) = (self.waves[wi].buffer_wi, self.waves[wi].buffer_ht, self.waves[wi].yc());
        let number_of_waves = Wave::wave_frequency(&self.waves[wi], 0.0);
        let total = (number_of_waves as i64 * buffer_wi as i64).clamp(0, 50_000_000) as usize;
        let mut delay = 0;
        let mut delta = 0;
        let mut values = vec![0i32; total];
        for x1 in 0..total {
            let was_positive = delay > 0;
            delay -= 1;
            values[x1] = if was_positive && x1 > 0 { values[x1 - 1] + delta } else { (2.0 * yc) as i32 };
            if values[x1] >= 2 * buffer_ht {
                delta = -2;
                values[x1] = 2 * buffer_ht - 1;
                if delay > 1 {
                    delay = 1;
                }
            }
            if values[x1] < 0 {
                delta = 2;
                values[x1] = 0;
                if delay > 1 {
                    delay = 1;
                }
            }
            if delay < 1 {
                delta = (self.rand.next() % 7) - 3;
                delay = 2 + (self.rand.next() % 3);
            }
        }
        self.waves[wi].ivey_y_values = values;
    }

    fn timing(&self, info: &PixelInfo) -> Timing {
        Timing { frame_time: info.frame_time, start_time: info.start_time, time_span: info.time_span }
    }

    fn render_frame(&mut self, frame_num: i32, info: &PixelInfo, fb: &mut PixelFrameBuffer) {
        let interval_pos = info.interval_position(frame_num);
        let timing = self.timing(info);
        self.frame_time += info.frame_time;
        let speed_increment = self.speed_increment;
        if self.frame_time >= 50 {
            let logical_frames = self.frame_time / 50;
            for index in 0..logical_frames {
                for wi in 0..self.waves.len() {
                    if self.waves[wi].prime_wave {
                        self.frame_50ms += self.waves[wi].buffer_wi;
                    }
                    let st = self.step(&self.waves[wi], self.frame_50ms, interval_pos, true);
                    // Only the last logical frame renders pixels; earlier ones just advance the wave.
                    let target = if index == logical_frames - 1 { Some(&mut *fb) } else { None };
                    Wave::render_wave(&mut self.waves[wi], &st, speed_increment, &timing, target);
                }
                self.frame_50ms += 1;
                self.frame_time -= 50;
            }
        } else {
            // A refresh faster than 20 Hz repeats the last frame of wave data.
            for wi in 0..self.waves.len() {
                let speed = self.wave_speed(&self.waves[wi], interval_pos * 100.0, true);
                self.waves[wi].render_columns(Some(&mut *fb), speed, false);
            }
        }
    }
}

impl PixelEffect for Wave {
    fn begin_group(&mut self, targets: &[NodeId], ctx: &RenderCtx) {
        // `MaxPixelsPerString` is computed for both positioning modes in C#; the framework only sets it for strings.
        let nodes = ctx.tree.leaf_parents(targets);
        self.max_pixels_per_string = nodes.iter().map(|n| ctx.tree.element_count(*n) as i32).max().unwrap_or(0).max(0);
    }

    fn setup_render(&mut self, info: &PixelInfo) {
        self.buffer_wi = info.buffer_wi();
        self.buffer_ht = info.buffer_ht();
        self.frame_50ms = 0;
        self.frame_time = 0;

        let matrix_width = if info.target_positioning == TargetPositioning::Strings {
            self.buffer_wi as f64
        } else {
            self.max_pixels_per_string as f64
        };
        // `Math.Round(x, MidpointRounding.AwayFromZero)`. A lone element has no leaf parents, so this can divide by zero: the
        // infinity saturates to int.MaxValue on both sides, and the wave's speed is then computed from what that wraps to.
        self.speed_increment = (self.buffer_wi as f64 / matrix_width).round() as i32;

        self.waves = self.param_waves.iter().map(|d| Waveform::from_params(d, &self.marks)).collect();
        let timing = self.timing(info);
        for wi in 0..self.waves.len() {
            {
                let w = &mut self.waves[wi];
                w.saw_tooth_min_y = i32::MAX;
                w.saw_tooth_max_y = 0;
                w.saw_tooth_x = 0;
                w.decay_factor = 0;
                w.degrees = 0.0;
                w.triangle_counter = 0;
                w.pixels.clear();
                if matches!(w.direction, Direction::RightToLeft | Direction::LeftToRight) {
                    w.buffer_ht = self.buffer_ht;
                    w.buffer_wi = self.buffer_wi;
                } else {
                    w.buffer_ht = self.buffer_wi;
                    w.buffer_wi = self.buffer_ht;
                }
            }
            if self.waves[wi].wave_type == WaveType::FractalIvey {
                self.init_ivey(wi);
            }
            let speed_increment = self.speed_increment;
            let w = &mut self.waves[wi];
            w.window_start = 0;
            w.window_stop = (w.buffer_wi as f64 * (w.window_percentage as f64 / 100.0)) as i32;
            if w.prime_wave {
                // Prime the wave with enough frames to fill the matrix.
                for prime_frame in 0..w.buffer_wi {
                    let pos = info.interval_position(prime_frame);
                    let factor = pos * 100.0;
                    let mut speed = Wave::scale_i(&w.speed, factor, MAX_WAVE_VELOCITY.wrapping_mul(speed_increment), 0);
                    if speed == 0 {
                        speed = 1;
                    }
                    let st = Step {
                        frame: prime_frame,
                        number_of_waves: Wave::wave_frequency(w, factor),
                        thickness: Wave::wave_thickness(w, factor),
                        height: Wave::wave_height(w, factor),
                        speed,
                        y_offset: Wave::wave_y_offset(w, factor),
                        color: w.color.get_color_at(pos),
                    };
                    Wave::render_wave(w, &st, speed_increment, &timing, None);
                }
            }
        }
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        self.render_frame(frame, info, buf);
    }

    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        let local_buffer_ht = info.buffer_ht();
        let mut virtual_buffer = PixelFrameBuffer::new(self.buffer_wi, self.buffer_ht);
        let locations = buf.element_locations().to_vec();
        for frame in 0..num_frames {
            buf.current_frame = frame;
            self.render_frame(frame, info, &mut virtual_buffer);
            for l in &locations {
                // Flip and offset the coordinates so the location version acts like the string version.
                let mut y = ((info.buffer_ht_offset() - l.y) + (local_buffer_ht - 1 + info.buffer_ht_offset())).abs();
                y -= info.buffer_ht_offset();
                let x = l.x - info.buffer_wi_offset();
                // C# indexes the virtual buffer unchecked, which throws for a location outside it. Skip those.
                if x >= 0 && y >= 0 && x < virtual_buffer.width() && y < virtual_buffer.height() {
                    buf.set_pixel(l.x, l.y, virtual_buffer.get_color_at(x, y));
                }
            }
            virtual_buffer.clear_buffer();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_effects::testing;

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        testing::assert_produces_light(&DESCRIPTOR);
    }

    #[test]
    fn defaults_have_two_waves_like_wave_data() {
        let p = DESCRIPTOR.new_params();
        let waves = p.records("WaveformData");
        assert_eq!(waves.len(), 2);
        assert_eq!(waves[1].enum_name("Direction"), "RightToLeft");
        assert_eq!(waves[1].int("PhaseShift"), 90);
        assert_eq!(waves[1].enum_name("ColorHandling"), "Across");
        assert_eq!(waves[0].enum_name("WaveType"), "Sine");
        assert_eq!(waves[0].int("WindowPercentage"), 25);
    }

    #[test]
    fn orientation_hidden_in_location_mode() {
        let mut p = DESCRIPTOR.new_params();
        p.set("TargetPositioning", ParamValue::Enum("Locations".to_string()));
        let visible: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(!visible.contains(&"StringOrientation"));
        assert!(visible.contains(&"TargetPositioning"));
    }

    #[test]
    fn a_wave_shows_what_its_type_uses() {
        let shown = |wave: &Params, name: &'static str| {
            let mut v = Visibility::default();
            (gen::WAVEFORM_DATA.visibility.unwrap())(wave, wave, &mut v);
            v.overrides.iter().find(|o| o.0 == name).map(|o| o.1).unwrap()
        };
        let mut w = Params::defaults(&gen::WAVEFORM_PARAMS);
        assert!(shown(&w, "Frequency") && shown(&w, "MovementType") && !shown(&w, "UseMarks") && !shown(&w, "MarkCollectionId"));
        w.set("WaveType", ParamValue::Enum("DecayingSine".to_string()));
        assert!(shown(&w, "UseMarks") && !shown(&w, "MovementType") && !shown(&w, "MarkCollectionId"));
        w.set("UseMarks", ParamValue::Bool(true));
        assert!(shown(&w, "MarkCollectionId"));
        w.set("WaveType", ParamValue::Enum("FractalIvey".to_string()));
        assert!(!shown(&w, "Frequency") && !shown(&w, "MarkCollectionId"));
    }

    #[test]
    fn editing_a_wave_applies_its_setters() {
        let marks = [MarkCollection::default()];
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 3);
        let targets = [s];
        let t = TargetInfo { marks: &marks, ..testing::target_info(&tree, &targets) };
        let mut w = Params::defaults(&gen::WAVEFORM_PARAMS);
        w.set("WaveType", ParamValue::Enum("DecayingSine".to_string()));
        w.set("UseMarks", ParamValue::Bool(true));
        let mut snake = Params::defaults(&gen::WAVEFORM_PARAMS);
        snake.set("MovementType", ParamValue::Enum("Snake".to_string()));
        snake.set("PrimeWave", ParamValue::Bool(false));
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_param(&mut p, "WaveformData", ParamValue::RecordList(vec![w, snake]), &t);
        assert_eq!(p.records("WaveformData")[0].guid("MarkCollectionId"), marks[0].id);
        assert!(p.records("WaveformData")[1].boolean("PrimeWave"));
    }

    #[test]
    fn explicit_wave_mark_edits_wait_for_activation_or_collection_assignment() {
        let marks = [MarkCollection::default()];
        let mut tree = ElementTree::new();
        let targets = [tree.add_string("S", 3)];
        let t = TargetInfo { marks: &marks, ..testing::target_info(&tree, &targets) };
        let mut p = DESCRIPTOR.new_params();
        let edit = |p: &mut Params, row: usize, field: &str, value: ParamValue| {
            let mut waves = p.records("WaveformData").to_vec();
            waves[row].set(field, value);
            DESCRIPTOR.set_param(p, "WaveformData", ParamValue::RecordList(waves), &t);
        };
        edit(&mut p, 1, "WaveType", ParamValue::Enum("DecayingSine".into()));
        edit(&mut p, 1, "UseMarks", ParamValue::Bool(true));
        assert_eq!(p.records("WaveformData")[1].guid("MarkCollectionId"), marks[0].id);
        for id in [Uuid::nil(), uuid::uuid!("00000000-0000-0000-0000-00000000dead")] {
            edit(&mut p, 1, "MarkCollectionId", ParamValue::Guid(id));
            edit(&mut p, 1, "Color", ParamValue::Gradient(ColorGradient::solid(Color::RED)));
            assert_eq!(p.records("WaveformData")[1].guid("MarkCollectionId"), id);
            // A mode edit on another row activates all active selections.
            edit(&mut p, 0, "UseMarks", ParamValue::Bool(true));
            assert_eq!(p.records("WaveformData")[1].guid("MarkCollectionId"), marks[0].id);
            edit(&mut p, 0, "UseMarks", ParamValue::Bool(false));
        }
        let missing = uuid::uuid!("00000000-0000-0000-0000-00000000dead");
        edit(&mut p, 1, "MarkCollectionId", ParamValue::Guid(missing));
        let remaining = p.records("WaveformData")[1].clone();
        DESCRIPTOR.set_param(&mut p, "WaveformData", ParamValue::RecordList(vec![remaining]), &t);
        assert_eq!(p.records("WaveformData")[0].guid("MarkCollectionId"), missing, "deletion is not a child mode edit");
        DESCRIPTOR.set_marks(&mut p, &t);
        assert_eq!(p.records("WaveformData")[0].guid("MarkCollectionId"), marks[0].id);
    }

    #[test]
    fn snake_forces_prime_wave() {
        let mut p = Params::defaults(&gen::WAVEFORM_PARAMS);
        p.set("MovementType", ParamValue::Enum("Snake".to_string()));
        let w = Waveform::from_params(&p, &[]);
        assert!(w.prime_wave);
    }

    #[test]
    fn column_helpers_match_csharp_formulas() {
        // bottom = floor(ystart - r * thickness/100), top = ceil(ystart + r * thickness/100), minimum height 1.
        assert_eq!(calculate_top_and_bottom(10.0, 10.0, 5), (9, 11));
        assert_eq!(calculate_top_and_bottom(10.0, 10.0, 0), (10, 11));
    }

    /// Sine of 0 degrees puts the first column on the centre row: ystart = 10, y1 = floor(9.5), y2 = ceil(10.5).
    #[test]
    fn first_sine_column_is_centred() {
        let mut p = Params::defaults(&gen::WAVEFORM_PARAMS);
        p.set("Mirror", ParamValue::Bool(false));
        let mut w = Waveform::from_params(&p, &[]);
        w.buffer_ht = 20;
        w.buffer_wi = 10;
        let st = Step { frame: 0, number_of_waves: 180, thickness: 5, height: 50, speed: 1, y_offset: 0, color: Color::BLUE };
        w.column_sine(&st);
        let ys: Vec<i32> = w.pixels[0].iter().map(|c| c.1).collect();
        assert_eq!(ys, vec![9, 10]);
        // 180 waves over 10 columns is 18 degrees a column; the next column is centred at 10 + 10 * 0.5 * sin(18).
        w.column_sine(&st);
        let ys: Vec<i32> = w.pixels[1].iter().map(|c| c.1).collect();
        let centre = 10.0 + 10.0 * 0.5 * (18.0f64 * PI_180).sin();
        assert_eq!(ys, vec![(centre - 0.5).floor() as i32, (centre + 0.5).ceil() as i32 - 1].into_iter().collect::<Vec<_>>());
    }

    #[test]
    fn pop_removes_the_newest_column_and_queue_is_capped() {
        let p = Params::defaults(&gen::WAVEFORM_PARAMS);
        let mut w = Waveform::from_params(&p, &[]);
        w.buffer_ht = 20;
        w.buffer_wi = 3;
        let st = Step { frame: 0, number_of_waves: 180, thickness: 5, height: 50, speed: 1, y_offset: 0, color: Color::BLUE };
        for _ in 0..5 {
            w.column_sine(&st);
        }
        w.render_columns(None, 1, false);
        assert_eq!(w.pixels.len(), 3);
    }

    #[test]
    fn every_wave_type_direction_and_movement_renders() {
        let mut tree = ElementTree::new();
        let g = testing::grid(&mut tree, "G", 12, 17);
        let single = tree.add_string("S", 1);
        let located = testing::grid(&mut tree, "L", 4, 10);
        testing::add_locations(&mut tree, located);
        for wt in ["Sine", "Triangle", "Square", "DecayingSine", "FractalIvey", "Sawtooth"] {
            for dir in ["LeftToRight", "RightToLeft", "TopToBottom", "BottomToTop"] {
                for mv in ["Continuous", "Snake", "GrowAndShrink"] {
                    for mirror in [false, true] {
                        for target in [g, single] {
                            testing::render_with(&DESCRIPTOR, &tree, &[target], 800, |p| {
                                let mut w = p.records("WaveformData")[0].clone();
                                w.set("WaveType", ParamValue::Enum(wt.to_string()));
                                w.set("Direction", ParamValue::Enum(dir.to_string()));
                                w.set("MovementType", ParamValue::Enum(mv.to_string()));
                                w.set("Mirror", ParamValue::Bool(mirror));
                                p.set("WaveformData", ParamValue::RecordList(vec![w]));
                            });
                        }
                    }
                }
            }
            testing::render_with(&DESCRIPTOR, &tree, &[located], 800, |p| {
                let mut w = p.records("WaveformData")[0].clone();
                w.set("WaveType", ParamValue::Enum(wt.to_string()));
                p.set("WaveformData", ParamValue::RecordList(vec![w]));
                p.set("TargetPositioning", ParamValue::Enum("Locations".to_string()));
            });
        }
        // No waveforms at all renders nothing but must not fail.
        testing::render_with(&DESCRIPTOR, &tree, &[g], 800, |p| {
            p.set("WaveformData", ParamValue::RecordList(Vec::new()));
        });
    }
}
