//! Port of VixenModules.Effect.Pattern (originally the Weave effect): a repeating weave or brick tile that is
//! scrolled, expanded or compressed across the display element and optionally rotated.
//!
//! Several properties are aliases of other data members in the C# (the Brick properties reuse the Weave
//! members), and the table follows the C# data contract:
//! - `BrickHeight` is `WeaveThickness`, `MortarHeight` is the `BrickHeight` member, `BrickColors` is
//!   `HorizontalColors` and `RotateBrick` is `TransposeTile`.
//! - The unused `MortarHeight` data member is kept as `MortarHeightData` so files round trip.

use crate::generated::pattern as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    param_changed: Some(param_changed),
    param_changed_ctx: Some(param_changed_ctx),
    targets_changed: Some(hydrate_aliases),
    supports_location: true,
    ported: true,
    ..gen::BASE
};

/// These C# getters read the same data members. Hydrate only the UI aliases,
/// without replaying setters such as AdvancedSizing that overwrite saved values.
fn hydrate_aliases(p: &mut Params, _: &TargetInfo) {
    let height = p.get("WeaveThickness").clone();
    let colors = p.get("HorizontalColors").clone();
    p.set("BrickHeight", height);
    p.set("BrickColors", colors);
}

fn param_changed_ctx(change: &ParamChange, p: &mut Params) {
    if change.name == MODULE_DATA {
        hydrate_aliases(p, change.info);
    }
}

/// `InitAllAttributes` and its `Update*Attributes` helpers, evaluated together.
/// The C# applies them as separate events, so the same property can be set by two of them; the combined
/// state here is what each event intends.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let weave = p.enum_name("PatternType") == "Weave";
    let brick = !weave;
    let advanced = p.boolean("AdvancedSizing");
    let locations = p.enum_name("TargetPositioning") == "Locations";

    v.set("StringOrientation", !locations);
    v.set("TargetPositioning", true);
    v.set("HighlightPercentage", p.boolean("Highlight") && locations);

    v.set("AdvancedSizing", weave);
    v.set("WeaveThickness", weave && !advanced);
    v.set("WeaveSpacing", weave && !advanced);
    v.set("WeaveHorizontalSpacing", weave && advanced);
    v.set("WeaveVerticalSpacing", weave && advanced);
    v.set("WeaveHorizontalThickness", weave && advanced);
    v.set("WeaveVerticalThickness", weave && advanced);
    v.set("HorizontalColors", weave);
    v.set("VerticalColors", weave);

    v.set("BrickHeight", brick);
    v.set("BrickWidth", brick);
    v.set("MortarHeight", brick);
    v.set("MortarColor", brick);
    v.set("RotateBrick", brick);
    v.set("BrickColors", brick);
}

/// The side effects of the property setters: enabling `AdvancedSizing` copies the shared size to the
/// individual sliders, and the Brick aliases write through to the members they stand for.
fn param_changed(name: &str, p: &mut Params) {
    match name {
        "AdvancedSizing" => {
            if p.boolean("AdvancedSizing") {
                let (spacing, thickness) = (p.int("WeaveSpacing"), p.int("WeaveThickness"));
                p.set("WeaveVerticalSpacing", ParamValue::Int(spacing));
                p.set("WeaveVerticalThickness", ParamValue::Int(thickness));
                p.set("WeaveHorizontalSpacing", ParamValue::Int(spacing));
                p.set("WeaveHorizontalThickness", ParamValue::Int(thickness));
            }
        }
        "BrickHeight" => {
            let v = p.int("BrickHeight");
            p.set("WeaveThickness", ParamValue::Int(v));
        }
        "WeaveThickness" => {
            let v = p.int("WeaveThickness");
            p.set("BrickHeight", ParamValue::Int(v));
        }
        "BrickColors" => {
            let v = p.gradients("BrickColors");
            p.set("HorizontalColors", ParamValue::GradientList(v));
        }
        "HorizontalColors" => {
            let v = p.gradients("HorizontalColors");
            p.set("BrickColors", ParamValue::GradientList(v));
        }
        _ => {}
    }
}

#[derive(Copy, Clone, PartialEq)]
enum Dir {
    Up,
    Down,
    Expand,
    Compress,
    Left,
    Right,
    HorizontalExpand,
    HorizontalCompress,
    CenterCompress,
    CenterExpand,
}

#[derive(Copy, Clone, PartialEq)]
enum Orient {
    Vertical,
    Horizontal,
}

/// `ConvertFromVerticalLocationToStringCoordinates` or the flipped horizontal one.
#[derive(Copy, Clone, PartialEq)]
enum Convert {
    Vertical,
    HorizontalFlip,
}

struct Pattern {
    brick: bool,
    direction: Dir,
    rotation: Curve,
    speed: Curve,
    level_curve: Curve,
    highlight: bool,
    highlight_percentage: i32,
    show_3d: bool,
    advanced_sizing: bool,
    weave_thickness: i32,
    weave_spacing: i32,
    weave_h_thickness: i32,
    weave_v_thickness: i32,
    weave_h_spacing: i32,
    weave_v_spacing: i32,
    horizontal_colors: Vec<ColorGradient>,
    vertical_colors: Vec<ColorGradient>,
    brick_width: i32,
    brick_height: i32,
    mortar_height: i32,
    mortar_color: ColorGradient,
    rotate_brick: bool,
    locations: bool,

    // Per render.
    tile: PixelFrameBuffer,
    tile_valid: bool,
    scale_value: i32,
    buffer_ht: i32,
    buffer_wi: i32,
    length: i32,
    height_of_tile: i32,
    width_of_tile: i32,
    virtual_fb: Option<PixelFrameBuffer>,
    nodes: Vec<ElementLocation>,
    // Cached from the PixelInfo.
    bh: i32,
    bw: i32,
    bh_off: i32,
    bw_off: i32,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let direction = match p.enum_name("Direction") {
        "Down" => Dir::Down,
        "Expand" => Dir::Expand,
        "Compress" => Dir::Compress,
        "Left" => Dir::Left,
        "Right" => Dir::Right,
        "HorizontalExpand" => Dir::HorizontalExpand,
        "HorizontalCompress" => Dir::HorizontalCompress,
        "CenterCompress" => Dir::CenterCompress,
        "CenterExpand" => Dir::CenterExpand,
        _ => Dir::Up,
    };
    let mut e = Pattern {
        brick: p.enum_name("PatternType") == "Brick",
        direction,
        rotation: p.curve("RotationAngle"),
        speed: p.curve("SpeedCurve"),
        level_curve: p.curve("LevelCurve"),
        highlight: p.boolean("Highlight"),
        highlight_percentage: p.int("HighlightPercentage"),
        show_3d: p.boolean("Show3D"),
        advanced_sizing: p.boolean("AdvancedSizing"),
        weave_thickness: p.int("WeaveThickness"),
        weave_spacing: p.int("WeaveSpacing"),
        weave_h_thickness: p.int("WeaveHorizontalThickness"),
        weave_v_thickness: p.int("WeaveVerticalThickness"),
        weave_h_spacing: p.int("WeaveHorizontalSpacing"),
        weave_v_spacing: p.int("WeaveVerticalSpacing"),
        horizontal_colors: p.gradients("HorizontalColors"),
        vertical_colors: p.gradients("VerticalColors"),
        brick_width: p.int("BrickWidth"),
        // The Brick properties are aliases of Weave members.
        brick_height: p.int("WeaveThickness"),
        mortar_height: p.int("MortarHeight"),
        mortar_color: p.gradient("MortarColor"),
        rotate_brick: p.boolean("RotateBrick"),
        locations: TargetPositioning::from_name(p.enum_name("TargetPositioning")) == TargetPositioning::Locations,
        tile: PixelFrameBuffer::new(0, 0),
        tile_valid: false,
        scale_value: 0,
        buffer_ht: 0,
        buffer_wi: 0,
        length: 0,
        height_of_tile: 0,
        width_of_tile: 0,
        virtual_fb: None,
        nodes: Vec::new(),
        bh: 0,
        bw: 0,
        bh_off: 0,
        bw_off: 0,
    };
    render_pixel_effect(ctx, p, &mut e)
}

/// `PixelFrameBuffer.GetColorAt`, which throws outside the buffer in C#. Nothing is drawn there instead.
#[inline]
fn safe_get(fb: &PixelFrameBuffer, x: i32, y: i32) -> Color {
    if x >= 0 && y >= 0 && x < fb.width() && y < fb.height() {
        fb.get_color_at(x, y)
    } else {
        Color::EMPTY
    }
}

/// `AdjustIntensity` with the level already sampled for the frame.
#[inline]
fn adjust(color: Color, level: f64) -> Color {
    if level < 1.0 && color != Color::TRANSPARENT {
        let mut hsv = Hsv::from_color(color);
        hsv.set_v(hsv.v * level);
        hsv.to_color()
    } else {
        color
    }
}

/// `UpdateWeaveTileYPosition`.
fn update_tile_y(increment: bool, mut tile_y: i32, height_of_tile: i32) -> i32 {
    if increment {
        tile_y += 1;
        if tile_y == height_of_tile {
            tile_y = 0;
        }
    } else {
        tile_y -= 1;
        if tile_y < 0 {
            tile_y = height_of_tile - 1;
        }
    }
    tile_y
}

/// `CalculateWeaveYTilePosition`. `h` is positive.
fn calc_weave_y_tile(incrementing: bool, y: i32, movement_y: i32, h: i32, y_offset: i32) -> i32 {
    let mut y_tile = if incrementing { (y + movement_y) % h } else { y - y_offset - movement_y };
    if y_tile < 0 {
        y_tile += h;
    }
    y_tile % h
}

/// What one frame of rendering needs.
#[derive(Copy, Clone)]
struct Frame {
    frame: i32,
    level: f64,
    /// `GetWeaveYValue(frame)`.
    weave_y: i32,
}

impl Pattern {
    fn pos(info: &PixelInfo, frame: i32) -> f64 {
        info.interval_position(frame)
    }

    fn rotation_angle(&self, info: &PixelInfo, frame: i32) -> f64 {
        scale_curve_to_value(self.rotation.get_value(Pattern::pos(info, frame) * 100.0), 180.0, -180.0)
    }

    fn weave_y_value(&self, info: &PixelInfo, frame: i32) -> i32 {
        let pos = Pattern::pos(info, frame) * 100.0;
        let new_scale = (self.scale_value as f64 * 0.20) as i32;
        let speed = scale_curve_to_value(self.speed.get_value(pos), new_scale as f64, 0.0);
        (frame as f64 * (info.frame_time as f64 / 50.0) * speed) as i32
    }

    fn frame_ctx(&self, info: &PixelInfo, frame: i32) -> Frame {
        let level = self.level_curve.get_value(Pattern::pos(info, frame) * 100.0) / 100.0;
        Frame { frame, level, weave_y: self.weave_y_value(info, frame) }
    }

    #[inline]
    fn tile_get(&self, x: i32, y: i32) -> Color {
        safe_get(&self.tile, x, y)
    }

    // ---- Tile construction: weave

    fn weave_spacing_of(percent: i32, scale: i32) -> i32 {
        (percent as f64 / 100.0 * scale as f64) as i32
    }

    fn weave_thickness_of(percent: i32, scale: i32) -> i32 {
        let t = (percent as f64 / 100.0 * scale as f64) as i32;
        if t == 0 {
            1
        } else {
            t
        }
    }

    fn initialize_weave_tile(&mut self) {
        let (h_spacing, v_spacing, h_thickness, v_thickness);
        if self.advanced_sizing {
            h_spacing = Pattern::weave_spacing_of(self.weave_h_spacing, self.scale_value);
            v_spacing = Pattern::weave_spacing_of(self.weave_v_spacing, self.scale_value);
            h_thickness = Pattern::weave_thickness_of(self.weave_h_thickness, self.scale_value);
            v_thickness = Pattern::weave_thickness_of(self.weave_v_thickness, self.scale_value);
        } else {
            h_spacing = Pattern::weave_spacing_of(self.weave_spacing, self.scale_value);
            v_spacing = h_spacing;
            h_thickness = Pattern::weave_thickness_of(self.weave_thickness, self.scale_value);
            v_thickness = h_thickness;
        }
        let tile_size = |thickness: i32, spacing: i32, colors: usize| (2 * thickness + 2 * spacing).wrapping_mul(colors as i32);
        self.height_of_tile = tile_size(h_thickness, h_spacing, self.horizontal_colors.len());
        self.width_of_tile = tile_size(v_thickness, v_spacing, self.vertical_colors.len());
        self.tile_valid = self.height_of_tile > 0 && self.width_of_tile > 0;
        self.tile = PixelFrameBuffer::new(self.width_of_tile.max(0), self.height_of_tile.max(0));
        if !self.tile_valid {
            return;
        }
        let (height, width) = (self.height_of_tile, self.width_of_tile);

        // `DrawWeave`: the vertical bars, then the horizontal bars.
        let mut tile = std::mem::replace(&mut self.tile, PixelFrameBuffer::new(0, 0));
        self.draw_bars(width, v_thickness, h_thickness, v_spacing, h_spacing, height, &mut tile, &self.vertical_colors, Orient::Vertical, false);
        self.draw_bars(height, h_thickness, v_thickness, h_spacing, v_spacing, width, &mut tile, &self.horizontal_colors, Orient::Horizontal, true);
        self.tile = tile;
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_bars(
        &self,
        dimension_length: i32,
        bar_thickness: i32,
        other_bar_thickness: i32,
        bar_spacing: i32,
        other_bar_spacing: i32,
        bar_length: i32,
        fb: &mut PixelFrameBuffer,
        colors: &[ColorGradient],
        axis: Orient,
        odd: bool,
    ) {
        let mut d = 0i32;
        let mut bar_counter = 0i32;
        let bar_counter_check = if odd { 1 } else { 0 };
        let mut color_index = 0usize;
        while d < dimension_length {
            if d + bar_thickness <= dimension_length {
                let on = bar_counter % 2 == bar_counter_check;
                self.draw_line(d, bar_length, fb, colors, on, on, bar_thickness, other_bar_thickness, other_bar_spacing, color_index, axis);
                bar_counter += 1;
                color_index += 1;
                if color_index + 1 > colors.len() {
                    color_index = 0;
                }
            }
            d = d.saturating_add(bar_thickness).saturating_add(bar_spacing);
            if bar_thickness + bar_spacing <= 0 {
                break; // C# would loop forever
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_line(
        &self,
        d_start: i32,
        d2_length: i32,
        fb: &mut PixelFrameBuffer,
        colors: &[ColorGradient],
        over_or_under: bool,
        on_off: bool,
        bar_thickness: i32,
        other_bar_thickness: i32,
        bar_spacing: i32,
        color_index: usize,
        axis: Orient,
    ) {
        let period = other_bar_thickness + bar_spacing;
        for d1 in 0..bar_thickness {
            let mut off_counter = 0;
            let mut pen_on = on_off;
            let mut over = over_or_under;
            let color = self.bar_color(color_index, d1, bar_thickness, colors);
            for d2 in 0..d2_length {
                if pen_on {
                    match axis {
                        Orient::Vertical => fb.set_pixel(d1 + d_start, d2, color),
                        Orient::Horizontal => fb.set_pixel(d2, d1 + d_start, color),
                    }
                } else {
                    off_counter += 1;
                }
                if off_counter == other_bar_thickness {
                    pen_on = true;
                }
                if period > 0 && (d2 + 1) % period == 0 {
                    over = !over;
                    off_counter = 0;
                    if !over {
                        pen_on = false;
                    }
                }
            }
        }
    }

    fn highlight_color(gradient: &ColorGradient) -> Color {
        let mut hsv = Hsv::from_color(gradient.get_color_at(0.0));
        hsv.set_s(0.0);
        hsv.to_color()
    }

    fn color_3d(gradient: &ColorGradient, current: i32, thickness: i32) -> Color {
        let color = gradient.get_color_at(current as f64 / thickness as f64);
        let mut hsv = Hsv::from_color(color);
        hsv.set_v(hsv.v * (((thickness - current) as f32 / thickness as f32) as f64));
        hsv.to_color()
    }

    /// `GetBarColor` and `GetBrickColor`, which are the same calculation.
    fn bar_color(&self, color_index: usize, current: i32, thickness: i32, colors: &[ColorGradient]) -> Color {
        match colors.get(color_index) {
            Some(g) => self.shade(g, current, thickness),
            None => Color::EMPTY,
        }
    }

    fn shade(&self, gradient: &ColorGradient, current: i32, thickness: i32) -> Color {
        let mut highlight_row = 0;
        if self.locations {
            // Make the first % pixels white.
            highlight_row = (thickness as f64 * self.highlight_percentage as f64 / 100.0) as i32;
        }
        if self.highlight && current <= highlight_row {
            Pattern::highlight_color(gradient)
        } else if self.show_3d {
            Pattern::color_3d(gradient, current, thickness)
        } else {
            gradient.get_color_at(current as f64 / thickness as f64)
        }
    }

    // ---- Tile construction: brick

    fn brick_size(percent: i32, scale: i32) -> i32 {
        ((percent as f64 / 100.0 * scale as f64) as i32).max(1)
    }

    fn initialize_brick_tile(&mut self, info: &PixelInfo, frame: i32) {
        let brick_width = Pattern::brick_size(self.brick_width, self.scale_value);
        let brick_height = Pattern::brick_size(self.brick_height, self.scale_value);
        let mortar_height = Pattern::brick_size(self.mortar_height, self.scale_value);
        let n = self.horizontal_colors.len() as i32;
        self.width_of_tile = n * brick_width + n * mortar_height;
        self.height_of_tile = 2 * brick_height + 2 * mortar_height;
        self.tile = PixelFrameBuffer::new(self.width_of_tile.max(0), self.height_of_tile.max(0));
        self.tile_valid = self.width_of_tile > 0 && self.height_of_tile > 0;
        if !self.tile_valid {
            return;
        }
        let mortar = self.mortar_color.get_color_at(Pattern::pos(info, frame));
        self.setup_render_brick(brick_width, brick_height, mortar_height, mortar);

        if self.rotate_brick {
            // `TransposeTile`.
            let mut transposed = PixelFrameBuffer::new(self.height_of_tile, self.width_of_tile);
            for x in 0..self.width_of_tile {
                for y in 0..self.height_of_tile {
                    transposed.set_pixel(self.height_of_tile - y - 1, x, self.tile_get(x, y));
                }
            }
            self.tile = transposed;
            std::mem::swap(&mut self.width_of_tile, &mut self.height_of_tile);
        }
    }

    fn setup_render_brick(&mut self, brick_width: i32, brick_height: i32, mortar_height: i32, mortar: Color) {
        let colors: Vec<ColorGradient> = if self.horizontal_colors.len() < 2 {
            vec![self.horizontal_colors[0].clone(), self.horizontal_colors[0].clone()]
        } else {
            self.horizontal_colors.clone()
        };
        let mut skip_y = 0;
        for color_index in 0..2 {
            let mut skip_x = 0;
            if color_index % 2 == 0 {
                for gradient in &colors {
                    self.draw_brick_and_mortar(skip_x, skip_y, brick_width, brick_height, gradient, mortar_height, mortar);
                    skip_x += brick_width + mortar_height;
                }
            } else {
                let mut second_half = brick_width / 2;
                if brick_width % 2 != 0 {
                    second_half += 1;
                }
                let first = self.horizontal_colors[0].clone();
                self.draw_half_brick(0, skip_y, brick_height, brick_width / 2, &first);
                self.draw_mortar(skip_x + brick_width / 2, skip_y, mortar_height, brick_height, mortar);
                skip_x = brick_width / 2 + mortar_height;
                for gradient in colors.iter().skip(1) {
                    self.draw_brick_and_mortar(skip_x, skip_y, brick_width, brick_height, gradient, mortar_height, mortar);
                    skip_x += brick_width + mortar_height;
                }
                self.draw_half_brick(skip_x, skip_y, brick_height, second_half, &first);
            }
            // Mortar the length of the tile between brick rows.
            let w = self.width_of_tile;
            self.draw_mortar(0, skip_y + brick_height, w, mortar_height, mortar);
            skip_y += brick_height + mortar_height;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_brick_and_mortar(&mut self, x: i32, y: i32, brick_width: i32, brick_height: i32, gradient: &ColorGradient, mortar_height: i32, mortar: Color) {
        self.draw_half_brick(x, y, brick_height, brick_width, gradient); // a full brick
        self.draw_mortar(x + brick_width, y, mortar_height, brick_height, mortar);
    }

    fn draw_mortar(&mut self, x: i32, y: i32, mortar_width: i32, brick_height: i32, mortar: Color) {
        for h in 0..brick_height {
            for w in 0..mortar_width {
                self.tile.set_pixel(x + w, y + h, mortar);
            }
        }
    }

    /// `DrawFullBrick` and `DrawHalfBrick`.
    fn draw_half_brick(&mut self, x: i32, y: i32, brick_height: i32, width: i32, gradient: &ColorGradient) {
        for h in 0..brick_height {
            let color = self.shade(gradient, h, brick_height);
            for w in 0..width {
                self.tile.set_pixel(x + w, y + h, color);
            }
        }
    }

    fn initialize_tile(&mut self, info: &PixelInfo, frame: i32) {
        if self.brick {
            self.initialize_brick_tile(info, frame);
        } else {
            self.initialize_weave_tile();
        }
    }

    // ---- Rotation

    /// `GetRotatedPosition`: a WPF `RotateTransform(-angle, center, center)` about the square buffer's centre.
    fn rotated_position(&self, x: i32, y: i32, angle: f64, swap_xy: bool) -> (i32, i32) {
        let (y_offset, x_offset) = if swap_xy {
            ((self.length - self.bw) / 2, (self.length - self.bh) / 2)
        } else {
            ((self.length - self.bh) / 2, (self.length - self.bw) / 2)
        };
        let center = (self.length - 1) as f64 / 2.0;
        let radians = (-angle) * (std::f64::consts::PI / 180.0);
        let (sin, cos) = radians.sin_cos();
        let px = (x + x_offset) as f64 - center;
        let py = (y + y_offset) as f64 - center;
        let tx = center + px * cos - py * sin;
        let ty = center + px * sin + py * cos;
        (tx.round_ties_even() as i32, ty.round_ties_even() as i32)
    }

    // ---- String rendering

    /// `SetPixelVertical` and `SetPixelHorizontal`.
    #[inline]
    fn put(&self, fb: &mut PixelFrameBuffer, f: &Frame, orient: Orient, x: i32, y: i32, tile_x: i32, tile_y: i32) {
        let c = adjust(self.tile_get(tile_x, tile_y), f.level);
        match orient {
            Orient::Vertical => fb.set_pixel(x, y, c),
            Orient::Horizontal => fb.set_pixel(y, x, c),
        }
    }

    /// `RenderEffectWeave`: the moving weave.
    fn string_weave(&self, fb: &mut PixelFrameBuffer, f: &Frame, orient: Orient, move_right: bool, y_length: i32, x_length: i32) {
        let h = self.height_of_tile;
        let w = self.width_of_tile;
        let mut tile_y = f.weave_y % h;
        for y in 0..y_length {
            for x in 0..x_length {
                self.put(fb, f, orient, x, y, x % w, tile_y);
            }
            tile_y = update_tile_y(move_right, tile_y, h);
        }
    }

    /// `RenderEffectWeaveExpandCompress`.
    fn string_expand_compress(&self, fb: &mut PixelFrameBuffer, f: &Frame, orient: Orient, expand: bool, y_length: i32, x_length: i32) {
        let h = self.height_of_tile;
        let w = self.width_of_tile;
        let mut tile_y = f.weave_y % h;
        let mut y = y_length / 2 - 1;
        while y >= 0 {
            for x in 0..x_length {
                self.put(fb, f, orient, x, y, x % w, tile_y);
            }
            tile_y = update_tile_y(!expand, tile_y, h);
            y -= 1;
        }
        tile_y = f.weave_y % h;
        for y in y_length / 2..y_length {
            for x in 0..x_length {
                self.put(fb, f, orient, x, y, x % w, tile_y);
            }
            tile_y = update_tile_y(!expand, tile_y, h);
        }
    }

    /// `DrawQuadrantRow`.
    #[allow(clippy::too_many_arguments)]
    fn quadrant_row(&self, fb: &mut PixelFrameBuffer, f: &Frame, orient: Orient, expand: bool, y: i32, x_length: i32, start_tile_x: i32, tile_y: i32, decrementing: bool) {
        let w = self.width_of_tile;
        for x in 0..x_length / 2 {
            let tile_x = if expand { (x + start_tile_x) % w } else { (x + x_length - start_tile_x) % w };
            let px = if decrementing { x_length - x - 1 } else { x };
            self.put(fb, f, orient, px, y, tile_x, tile_y);
        }
    }

    /// `RenderEffectWeaveExpandCompressCenter`.
    fn string_expand_compress_center(&self, fb: &mut PixelFrameBuffer, f: &Frame, orient: Orient, expand: bool, y_length: i32, x_length: i32) {
        let h = self.height_of_tile;
        let start_tile_x = f.weave_y % self.width_of_tile;
        let mut tile_y = f.weave_y % h;
        let mut y = y_length / 2 - 1;
        while y >= 0 {
            self.quadrant_row(fb, f, orient, expand, y, x_length, start_tile_x, tile_y, false);
            self.quadrant_row(fb, f, orient, expand, y, x_length, start_tile_x, tile_y, true);
            tile_y = update_tile_y(!expand, tile_y, h);
            y -= 1;
        }
        tile_y = f.weave_y % h;
        for y in y_length / 2..y_length {
            self.quadrant_row(fb, f, orient, expand, y, x_length, start_tile_x, tile_y, false);
            self.quadrant_row(fb, f, orient, expand, y, x_length, start_tile_x, tile_y, true);
            tile_y = update_tile_y(!expand, tile_y, h);
        }
    }

    /// `RenderEffectStringsInternal` and `RenderEffectStringsWeave`.
    fn render_strings_internal(&mut self, info: &PixelInfo, f: &Frame, fb: &mut PixelFrameBuffer) {
        if self.brick {
            self.initialize_tile(info, f.frame);
        }
        if !self.tile_valid {
            return; // the C# divides by the empty tile and fails
        }
        let (bt, bw) = (self.buffer_ht, self.buffer_wi);
        use Orient::{Horizontal as H, Vertical as V};
        match self.direction {
            Dir::Up | Dir::Down => self.string_weave(fb, f, V, self.direction == Dir::Down, bt, bw),
            Dir::Left | Dir::Right => self.string_weave(fb, f, H, self.direction == Dir::Left, bw, bt),
            Dir::Expand => self.string_expand_compress(fb, f, V, true, bt, bw),
            Dir::Compress => self.string_expand_compress(fb, f, V, false, bt, bw),
            Dir::HorizontalExpand => self.string_expand_compress(fb, f, H, true, bw, bt),
            Dir::HorizontalCompress => self.string_expand_compress(fb, f, H, false, bw, bt),
            Dir::CenterCompress => self.string_expand_compress_center(fb, f, V, false, bt, bw),
            Dir::CenterExpand => self.string_expand_compress_center(fb, f, V, true, bt, bw),
        }
    }

    // ---- Location rendering

    fn convert(&self, kind: Convert, x: i32, y: i32) -> (i32, i32) {
        match kind {
            Convert::Vertical => {
                let y2 = ((self.bh_off - y) + (self.buffer_ht - 1 + self.bh_off)).abs();
                (x - self.bw_off, y2 - self.bh_off)
            }
            Convert::HorizontalFlip => {
                // `ConvertFromHorizontalLocationToStringCoordinates(y, x)`.
                let y2 = ((self.bw_off - x) + (self.buffer_wi - 1 + self.bw_off)).abs();
                (y - self.bh_off, y2 - self.bw_off)
            }
        }
    }

    /// `RenderEffectByLocation` with `CalculateMovingColor`.
    fn location_moving(&self, buf: &mut PixelLocationFrameBuffer, f: &Frame, info: &PixelInfo, swap_xy: bool, moves_right: bool, kind: Convert) {
        let mut angle = self.rotation_angle(info, f.frame);
        if swap_xy {
            angle = -angle;
        }
        let h = self.height_of_tile;
        let movement_y = f.weave_y % h;
        for node in &self.nodes {
            let (mut x, mut y) = self.convert(kind, node.x, node.y);
            if angle != 0.0 {
                (x, y) = self.rotated_position(x, y, angle, swap_xy);
            }
            let y_tile = calc_weave_y_tile(moves_right, y, movement_y, h, 0);
            let x_tile = x % self.width_of_tile;
            let color = if moves_right { self.tile_get(x_tile, y_tile) } else { self.tile_get(x_tile, (h - 1) - y_tile) };
            buf.set_pixel(node.x, node.y, adjust(color, f.level));
        }
    }

    /// `RenderEffectLocationExpandCompress`.
    #[allow(clippy::too_many_arguments)]
    fn location_expand_compress(&self, buf: &mut PixelLocationFrameBuffer, f: &Frame, down_increases: bool, angle: f64, swap_xy: bool, kind: Convert, height: i32) {
        let h = self.height_of_tile;
        let w = self.width_of_tile;
        let movement_y = f.weave_y % h;
        let mut height = height;
        let compressing = matches!(self.direction, Dir::Compress | Dir::HorizontalCompress);
        for node in &self.nodes {
            let (mut x, mut y) = self.convert(kind, node.x, node.y);
            if angle != 0.0 {
                (x, y) = self.rotated_position(x, y, angle, swap_xy);
                height = self.length;
            }
            let down = y < height / 2;
            let mut y_tile = calc_weave_y_tile(if down_increases { down } else { !down }, y, movement_y, h, height / 2);
            let repeating_x = x % w;
            if down {
                y_tile = (height / 2 - y_tile) % h;
                if y_tile < 0 {
                    y_tile += h;
                }
            }
            let c = if compressing { self.tile_get(repeating_x, y_tile) } else { self.tile_get(repeating_x, (h - 1) - y_tile) };
            buf.set_pixel(node.x, node.y, adjust(c, f.level));
        }
    }

    /// `RenderEffectLocationExpandCompressCenter`.
    #[allow(clippy::too_many_arguments)]
    fn location_expand_compress_center(&self, buf: &mut PixelLocationFrameBuffer, f: &Frame, down_increases: bool, angle: f64, swap_xy: bool, kind: Convert, height: i32, width: i32) {
        let h = self.height_of_tile;
        let w = self.width_of_tile;
        let movement_y = f.weave_y % h;
        let movement_x = f.weave_y % w;
        let (mut height, mut width) = (height, width);
        let compressing = matches!(self.direction, Dir::Compress | Dir::HorizontalCompress);
        for node in &self.nodes {
            let (mut x, mut y) = self.convert(kind, node.x, node.y);
            if angle != 0.0 {
                (x, y) = self.rotated_position(x, y, angle, swap_xy);
                height = self.length;
                width = self.length;
            }
            let down = y < height / 2;
            let x_down = x < width / 2;
            let mut y_tile = calc_weave_y_tile(if down_increases { down } else { !down }, y, movement_y, h, height / 2);
            let x_tile = calc_weave_y_tile(if down_increases { x_down } else { !x_down }, x, movement_x, w, width / 2);
            let mut repeating_x = x_tile % w;
            if x_down {
                repeating_x = (width / 2 - repeating_x) % w;
                if repeating_x < 0 {
                    repeating_x += w;
                }
            }
            if down {
                y_tile = (height / 2 - y_tile) % h;
                if y_tile < 0 {
                    y_tile += h;
                }
            }
            let c = if compressing { self.tile_get(repeating_x, y_tile) } else { self.tile_get(repeating_x, (h - 1) - y_tile) };
            buf.set_pixel(node.x, node.y, adjust(c, f.level));
        }
    }

    /// `RenderEffectByLocationWeave`: dispatch on the direction.
    fn render_location_frame(&self, buf: &mut PixelLocationFrameBuffer, info: &PixelInfo, f: &Frame) {
        let angle = self.rotation_angle(info, f.frame);
        let (bt, bw) = (self.buffer_ht, self.buffer_wi);
        match self.direction {
            Dir::Up | Dir::Down => self.location_moving(buf, f, info, false, self.direction == Dir::Down, Convert::Vertical),
            Dir::Left | Dir::Right => self.location_moving(buf, f, info, true, self.direction == Dir::Right, Convert::HorizontalFlip),
            Dir::Expand => self.location_expand_compress(buf, f, true, angle, false, Convert::Vertical, bt),
            Dir::Compress => self.location_expand_compress(buf, f, false, angle, false, Convert::Vertical, bt),
            Dir::HorizontalExpand => self.location_expand_compress(buf, f, true, -angle, true, Convert::HorizontalFlip, bw),
            Dir::HorizontalCompress => self.location_expand_compress(buf, f, false, -angle, true, Convert::HorizontalFlip, bw),
            Dir::CenterCompress => self.location_expand_compress_center(buf, f, false, angle, false, Convert::Vertical, bt, bw),
            Dir::CenterExpand => self.location_expand_compress_center(buf, f, true, angle, false, Convert::Vertical, bt, bw),
        }
    }
}

impl PixelEffect for Pattern {
    fn setup_render(&mut self, info: &PixelInfo) {
        self.bh = info.buffer_ht();
        self.bw = info.buffer_wi();
        self.bh_off = info.buffer_ht_offset();
        self.bw_off = info.buffer_wi_offset();
        // Plus 2 is an extra margin so a rotation never lands on a negative location.
        let diagonal = (((self.bh as i64 * self.bh as i64) + (self.bw as i64 * self.bw as i64)) as f64).sqrt();
        self.length = diagonal.round() as i32 + 2;

        if info.target_positioning == TargetPositioning::Strings {
            let pts = &self.rotation.points;
            let angle_is_zero = pts.len() == 2 && pts[0] == (0.0, 50.0) && pts[1] == (100.0, 50.0);
            if angle_is_zero {
                self.buffer_wi = self.bw;
                self.buffer_ht = self.bh;
            } else {
                self.buffer_wi = self.length;
                self.buffer_ht = self.length;
            }
        } else {
            self.buffer_wi = self.bw;
            self.buffer_ht = self.bh;
        }
        self.scale_value = self.bh.min(self.bw);
        self.virtual_fb = None;
        self.tile_valid = false;
        if !self.brick {
            self.initialize_weave_tile();
        }
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let f = self.frame_ctx(info, frame);
        let angle = self.rotation_angle(info, frame);
        if angle == 0.0 {
            self.render_strings_internal(info, &f, buf);
            return;
        }
        // A virtual frame buffer large enough to rotate the real one inside it.
        let mut virtual_fb = match self.virtual_fb.take() {
            Some(mut v) if v.width() == self.buffer_wi && v.height() == self.buffer_ht => {
                v.fill(Color::EMPTY);
                v
            }
            _ => PixelFrameBuffer::new(self.buffer_wi, self.buffer_ht),
        };
        self.render_strings_internal(info, &f, &mut virtual_fb);
        if self.tile_valid {
            for x in 0..self.bw {
                for y in 0..self.bh {
                    let (rx, ry) = self.rotated_position(x, y, angle, false);
                    buf.set_pixel(x, y, safe_get(&virtual_fb, rx, ry));
                }
            }
        }
        self.virtual_fb = Some(virtual_fb);
    }

    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        self.nodes = buf.element_locations().to_vec();
        for frame in 0..num_frames {
            if self.brick {
                self.initialize_tile(info, frame);
            }
            buf.current_frame = frame;
            if !self.tile_valid {
                continue; // the C# divides by the empty tile and fails
            }
            let f = self.frame_ctx(info, frame);
            self.render_location_frame(buf, info, &f);
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

    fn set_enum(p: &mut Params, name: &str, v: &str) {
        p.set(name, ParamValue::Enum(v.to_string()));
    }

    #[test]
    fn every_direction_and_pattern_renders_in_both_modes() {
        let mut tree = ElementTree::new();
        let g = testing::grid(&mut tree, "G", 9, 21);
        let single = tree.add_string("S", 1);
        let located = testing::grid(&mut tree, "L", 4, 10);
        testing::add_locations(&mut tree, located);
        let rotated = Curve::from_points(vec![(0.0, 20.0), (50.0, 80.0), (100.0, 50.0)]);
        for pattern in ["Weave", "Brick"] {
            for dir in ["Up", "Down", "Expand", "Compress", "Left", "Right", "HorizontalExpand", "HorizontalCompress", "CenterCompress", "CenterExpand"] {
                for rot in [false, true] {
                    for extra in 0..4 {
                        let edit = |p: &mut Params| {
                            set_enum(p, "PatternType", pattern);
                            set_enum(p, "Direction", dir);
                            p.set("Highlight", ParamValue::Bool(extra & 1 == 1));
                            p.set("Show3D", ParamValue::Bool(extra & 2 == 2));
                            p.set("RotateBrick", ParamValue::Bool(extra & 1 == 1));
                            p.set("AdvancedSizing", ParamValue::Bool(extra & 2 == 2));
                            if rot {
                                p.set("RotationAngle", ParamValue::Curve(rotated.clone()));
                            }
                        };
                        for target in [g, single] {
                            testing::render_with(&DESCRIPTOR, &tree, &[target], 700, edit);
                        }
                        testing::render_with(&DESCRIPTOR, &tree, &[located], 700, |p| {
                            edit(p);
                            set_enum(p, "TargetPositioning", "Locations");
                        });
                    }
                }
            }
        }
        // Empty colour lists leave an empty tile, which must not fail.
        testing::render_with(&DESCRIPTOR, &tree, &[g], 300, |p| {
            p.set("HorizontalColors", ParamValue::GradientList(Vec::new()));
        });
        testing::render_with(&DESCRIPTOR, &tree, &[g], 300, |p| {
            set_enum(p, "PatternType", "Brick");
            p.set("HorizontalColors", ParamValue::GradientList(Vec::new()));
        });
    }

    #[test]
    fn rotated_position_is_identity_at_zero_and_a_quarter_turn_swaps_axes() {
        let mut e = test_pattern();
        e.length = 11;
        e.bh = 11;
        e.bw = 11;
        assert_eq!(e.rotated_position(3, 7, 0.0, false), (3, 7));
        // -angle is passed to the WPF transform, which turns clockwise: 90 -> (cx - (y-cy), cy + (x-cx)).
        assert_eq!(e.rotated_position(3, 7, -90.0, false), (3, 3));
    }

    fn test_pattern() -> Pattern {
        Pattern {
            brick: false,
            direction: Dir::Up,
            rotation: Curve::flat(50.0),
            speed: Curve::flat(10.0),
            level_curve: Curve::flat100(),
            highlight: false,
            highlight_percentage: 5,
            show_3d: false,
            advanced_sizing: false,
            weave_thickness: 5,
            weave_spacing: 5,
            weave_h_thickness: 5,
            weave_v_thickness: 5,
            weave_h_spacing: 5,
            weave_v_spacing: 5,
            horizontal_colors: vec![ColorGradient::solid(Color::RED)],
            vertical_colors: vec![ColorGradient::solid(Color::BLUE)],
            brick_width: 15,
            brick_height: 5,
            mortar_height: 5,
            mortar_color: ColorGradient::solid(Color::BLACK),
            rotate_brick: false,
            locations: false,
            tile: PixelFrameBuffer::new(0, 0),
            tile_valid: false,
            scale_value: 10,
            buffer_ht: 10,
            buffer_wi: 10,
            length: 0,
            height_of_tile: 0,
            width_of_tile: 0,
            virtual_fb: None,
            nodes: Vec::new(),
            bh: 10,
            bw: 10,
            bh_off: 0,
            bw_off: 0,
        }
    }

    /// Weave tile for a scale of 100 with the default 5% thickness and spacing: bars 5 thick, 5 apart, so
    /// the tile is (2*5 + 2*5) * colours = 20 wide and 20 high with one colour each way.
    #[test]
    fn weave_tile_size_and_first_bars() {
        let mut e = test_pattern();
        e.scale_value = 100;
        e.initialize_weave_tile();
        assert!(e.tile_valid);
        assert_eq!((e.width_of_tile, e.height_of_tile), (20, 20));
        // The first vertical (blue) bar is on for y 0..10, off for 10..15 (it goes under the horizontal bar
        // that starts at y = 10) and on again from 15. Its pixel at (0, 7) has no horizontal bar over it.
        assert_eq!(e.tile_get(0, 7), Color::BLUE);
        assert_eq!(e.tile_get(0, 12), Color::RED);
        // The first horizontal (red) bar starts off for the width of a vertical bar and is on from x = 5.
        assert_eq!(e.tile_get(2, 2), Color::BLUE);
        assert_eq!(e.tile_get(12, 0), Color::RED);
        // Between bars (x = 7, y = 7) nothing is drawn.
        assert_eq!(e.tile_get(7, 7), Color::EMPTY);
    }

    /// Brick tile: width = colours * (brick + mortar), height = 2 * (brick + mortar), the mortar row
    /// follows the first brick row, and the second row starts with half a brick.
    #[test]
    fn brick_tile_layout() {
        let mut e = test_pattern();
        e.brick = true;
        e.scale_value = 100;
        e.brick_width = 10; // 10 px
        e.brick_height = 4;
        e.mortar_height = 2;
        e.horizontal_colors = vec![ColorGradient::solid(Color::RED), ColorGradient::solid(Color::BLUE)];
        let info = pixel_info_for_test();
        e.initialize_brick_tile(&info, 0);
        assert_eq!((e.width_of_tile, e.height_of_tile), (2 * (10 + 2), 2 * (4 + 2)));
        assert_eq!(e.tile_get(0, 0), Color::RED);
        assert_eq!(e.tile_get(10, 0), Color::BLACK); // mortar after the first brick
        assert_eq!(e.tile_get(12, 0), Color::BLUE);
        assert_eq!(e.tile_get(0, 4), Color::BLACK); // mortar row between brick rows
        assert_eq!(e.tile_get(0, 6), Color::RED); // second row: half brick first
        assert_eq!(e.tile_get(5, 6), Color::BLACK); // mortar after the half brick
    }

    fn pixel_info_for_test() -> PixelInfo {
        // The brick mortar only samples the gradient at the interval position, so any info works.
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 2);
        let captured = std::sync::Mutex::new(None);
        struct Grab<'a>(&'a std::sync::Mutex<Option<PixelInfo>>);
        impl PixelEffect for Grab<'_> {
            fn setup_render(&mut self, info: &PixelInfo) {
                *self.0.lock().unwrap() = Some(info.clone());
            }
            fn render_effect(&mut self, _f: i32, _i: &PixelInfo, _b: &mut PixelFrameBuffer) {}
        }
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let targets = [s];
        let ctx = RenderCtx { video_cache: None,
            tree: &tree,
            targets: &targets,
            start_time: TimeSpan::ZERO,
            time_span: TimeSpan::from_ms_i(500),
            frame_ms: 50,
            marks: &[],
            audio: None,
            module_data_root: None,
            cancel: &cancel,
        };
        render_pixel(&ctx, StringOrientation::Vertical, TargetPositioning::Strings, &mut Grab(&captured));
        let info = captured.lock().unwrap().take().unwrap();
        info
    }

    #[test]
    fn adjust_intensity_scales_value_only_below_full_level() {
        let c = Color::rgb(200, 100, 0);
        assert_eq!(adjust(c, 1.0), c);
        let dim = adjust(c, 0.5);
        assert!(dim.r < c.r && dim.r > 0);
        assert_eq!(adjust(Color::TRANSPARENT, 0.5), Color::TRANSPARENT);
    }

    #[test]
    fn visibility_follows_pattern_type_and_sizing() {
        let names = |p: &Params| -> Vec<&'static str> { DESCRIPTOR.visible_params(p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect() };
        let mut p = DESCRIPTOR.new_params();
        let v = names(&p);
        assert!(v.contains(&"WeaveThickness") && v.contains(&"HorizontalColors"));
        assert!(!v.contains(&"BrickWidth") && !v.contains(&"WeaveHorizontalSpacing") && !v.contains(&"HighlightPercentage"));
        p.set("AdvancedSizing", ParamValue::Bool(true));
        let v = names(&p);
        assert!(v.contains(&"WeaveHorizontalSpacing") && !v.contains(&"WeaveSpacing"));
        set_enum(&mut p, "PatternType", "Brick");
        let v = names(&p);
        assert!(v.contains(&"BrickColors") && v.contains(&"MortarColor") && !v.contains(&"VerticalColors"));
        p.set("Highlight", ParamValue::Bool(true));
        set_enum(&mut p, "TargetPositioning", "Locations");
        let v = names(&p);
        assert!(v.contains(&"HighlightPercentage") && !v.contains(&"StringOrientation"));
    }

    #[test]
    fn advanced_sizing_copies_shared_values_and_brick_aliases_write_through() {
        let mut p = DESCRIPTOR.new_params();
        p.set("WeaveSpacing", ParamValue::Int(12));
        p.set("WeaveThickness", ParamValue::Int(9));
        p.set("AdvancedSizing", ParamValue::Bool(true));
        param_changed("AdvancedSizing", &mut p);
        assert_eq!(p.int("WeaveVerticalSpacing"), 12);
        assert_eq!(p.int("WeaveHorizontalThickness"), 9);
        p.set("BrickHeight", ParamValue::Int(33));
        param_changed("BrickHeight", &mut p);
        assert_eq!(p.int("WeaveThickness"), 33);
    }
}
