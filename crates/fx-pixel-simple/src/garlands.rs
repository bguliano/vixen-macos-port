//! Port of VixenModules.Effect.Garlands.

use crate::generated::garlands as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    supports_location: true,
    ported: true,
    ..gen::BASE
};

/// `UpdateMovementTypeAttribute`, `UpdateStringOrientationAttributes` and `EnableTargetPositioning`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let speed = p.enum_name("MovementType") == "Speed";
    v.set("SpeedCurve", speed);
    v.set("Iterations", !speed);
    v.set("StringOrientation", p.enum_name("TargetPositioning") == "Strings");
    v.set("TargetPositioning", true);
}

#[derive(Copy, Clone, PartialEq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

struct Garlands {
    direction: Direction,
    movement_speed: bool,
    iterations: i32,
    garland_type: i32,
    colors: Vec<ColorGradient>,
    level_curve: Curve,
    speed_curve: Curve,
    spacing_curve: Curve,
    speed: i32,
    frames: i32,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    // Indexing the empty list throws in the C#, and a pixel effect that throws has no intents at all.
    if p.gradients("Colors").is_empty() {
        return EffectIntents::new();
    }
    let mut e = Garlands {
        direction: match p.enum_name("Direction") {
            "Down" => Direction::Down,
            "Left" => Direction::Left,
            "Right" => Direction::Right,
            _ => Direction::Up,
        },
        movement_speed: p.enum_name("MovementType") == "Speed",
        iterations: p.int("Iterations"),
        garland_type: p.int("Type"),
        colors: p.gradients("Colors"),
        level_curve: p.curve("LevelCurve"),
        speed_curve: p.curve("SpeedCurve"),
        spacing_curve: p.curve("SpacingCurve"),
        speed: 0,
        frames: 0,
    };
    render_pixel_effect(ctx, p, &mut e)
}

#[inline]
fn dim(col: Color, level: f64) -> Color {
    if level < 1.0 {
        let mut hsv = Hsv::from_color(col);
        hsv.set_v(hsv.v * level);
        hsv.to_color()
    } else {
        col
    }
}

/// `PatternIncrement`: cycles through the increments, staying on the last until it wraps.
struct PatternIncrement<'a> {
    increments: &'a [i32],
    index: usize,
}

impl PatternIncrement<'_> {
    fn next(&mut self) -> i32 {
        let inc = self.increments[self.index];
        self.index = if self.index < self.increments.len() - 1 { self.index + 1 } else { 0 };
        inc
    }
}

impl Garlands {
    fn calculate_speed(&self, interval_pos: f64, info: &PixelInfo) -> i32 {
        let value = (scale_curve_to_value(self.speed_curve.get_value(interval_pos), 100.0, 1.0) * info.frame_time as f64 / 50.0).round_ties_even() as i32;
        value.max(1)
    }

    fn calculate_spacing(&self, interval_pos: f64) -> i32 {
        let value = scale_curve_to_value(self.spacing_curve.get_value(interval_pos), 20.0, 1.0).round_ties_even() as i32;
        value.max(1)
    }

    fn channel_blend(c1: i32, c2: i32, ratio: f64) -> i32 {
        c1 + (ratio * (c2 - c1) as f64 + 0.5).floor() as i32
    }

    fn gradient_at(&self, idx: usize, pos: f64) -> Color {
        match self.colors.get(idx) {
            Some(g) => g.get_color_at(pos),
            None => Color::EMPTY,
        }
    }

    fn two_color_blend(&self, idx1: usize, idx2: usize, ratio: f64, frame: i32, info: &PixelInfo) -> Color {
        let pos = info.interval_position(frame);
        let c1 = self.gradient_at(idx1, pos);
        let c2 = self.gradient_at(idx2, pos);
        Color::from_rgb_i32(
            Self::channel_blend(c1.r as i32, c2.r as i32, ratio),
            Self::channel_blend(c1.g as i32, c2.g as i32, ratio),
            Self::channel_blend(c1.b as i32, c2.b as i32, ratio),
        )
    }

    fn multi_color_blend(&self, mut n: f64, circular: bool, frame: i32, info: &PixelInfo) -> Color {
        let count = self.colors.len();
        if count <= 1 {
            return self.gradient_at(0, info.interval_position(frame));
        }
        if n >= 1.0 {
            n = 0.99999;
        } else if n < 0.0 {
            n = 0.0;
        }
        let real_idx = if circular { n * count as f64 } else { n * (count - 1) as f64 };
        let idx1 = real_idx.floor() as i32;
        let idx2 = (idx1 + 1) % count as i32;
        let ratio = real_idx - idx1 as f64;
        self.two_color_blend(idx1 as usize, idx2 as usize, ratio, frame, info)
    }

    /// `garlandsState`, which both render paths compute the same way.
    fn garlands_state(&mut self, limit: i32, interval_pos_factor: f64, info: &PixelInfo) -> i32 {
        let state;
        if !self.movement_speed {
            let n = info.number_frames();
            let per_frame = if n == 0 { 0 } else { limit / n };
            state = limit.wrapping_sub(self.frames.wrapping_mul(per_frame.wrapping_mul(self.iterations))) / 4;
            if state <= 0 {
                self.frames = 0;
            }
        } else {
            self.speed = self.speed.wrapping_add(self.calculate_speed(interval_pos_factor, info));
            state = if limit == 0 { 0 } else { (limit - self.speed % limit) / 4 };
        }
        self.frames += 1;
        state
    }

    /// Per-column drop of a ring in the string render, by garland type.
    fn type_offset(&self, x: i32) -> i32 {
        match self.garland_type {
            1 => match x % 5 {
                2 => 2,
                1 | 3 => 1,
                _ => 0,
            },
            2 => match x % 5 {
                2 => 4,
                1 | 3 => 2,
                _ => 0,
            },
            3 => match x % 6 {
                3 => 6,
                2 | 4 => 4,
                1 | 5 => 2,
                _ => 0,
            },
            4 => match x % 5 {
                1 | 3 => 2,
                _ => 0,
            },
            _ => 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_ring<B: PixelBuffer>(
        &self,
        start: i32,
        mut y: i32,
        y_limit: i32,
        height: i32,
        matrix: &[(i32, Vec<i32>)],
        buf: &mut B,
        col: Color,
        increments: &[i32],
    ) {
        if y < y_limit {
            y = y_limit;
        }
        if y >= height {
            return;
        }
        let row_index = if matches!(self.direction, Direction::Up | Direction::Left) { height - y - 1 } else { y };
        let Some((key, items)) = matrix.get(row_index.max(0) as usize) else { return };
        let mut pattern = PatternIncrement { increments, index: 0 };
        let mut x = start;
        while (x as usize) < items.len() && x >= 0 {
            if matches!(self.direction, Direction::Up | Direction::Down) {
                buf.set_pixel(items[x as usize], *key, col);
            } else {
                buf.set_pixel(*key, items[x as usize], col);
            }
            x += pattern.next();
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_pattern<B: PixelBuffer>(&self, matrix: &[(i32, Vec<i32>)], buf: &mut B, y: i32, y_limit: i32, height: i32, col: Color) {
        match self.garland_type {
            0 => self.draw_ring(0, y, y_limit, height, matrix, buf, col, &[1]),
            1 => {
                self.draw_ring(0, y, y_limit, height, matrix, buf, col, &[4, 1]);
                self.draw_ring(1, y - 1, y_limit, height, matrix, buf, col, &[2, 3]);
                self.draw_ring(2, y - 2, y_limit, height, matrix, buf, col, &[5]);
            }
            2 => {
                self.draw_ring(0, y, y_limit, height, matrix, buf, col, &[4, 1]);
                self.draw_ring(1, y - 2, y_limit, height, matrix, buf, col, &[2, 3]);
                self.draw_ring(2, y - 4, y_limit, height, matrix, buf, col, &[5]);
            }
            3 => {
                self.draw_ring(0, y, y_limit, height, matrix, buf, col, &[6]);
                self.draw_ring(1, y - 2, y_limit, height, matrix, buf, col, &[4, 2]);
                self.draw_ring(2, y - 4, y_limit, height, matrix, buf, col, &[2, 4]);
                self.draw_ring(3, y - 6, y_limit, height, matrix, buf, col, &[6]);
            }
            4 => {
                self.draw_ring(0, y, y_limit, height, matrix, buf, col, &[2, 2, 1]);
                self.draw_ring(1, y - 2, y_limit, height, matrix, buf, col, &[2, 3]);
            }
            _ => {} // The C# asserts here and draws nothing.
        }
    }

    /// `GetSparseMatrix`: rows of locations, keyed by Y (Up/Down) or X (Left/Right).
    fn sparse_matrix(&self, locs: &[ElementLocation]) -> Vec<(i32, Vec<i32>)> {
        let by_y = matches!(self.direction, Direction::Up | Direction::Down);
        let mut sorted = locs.to_vec();
        if by_y {
            sorted.sort_by_key(|l| (l.y, l.x));
        } else {
            sorted.sort_by_key(|l| (l.x, l.y));
        }
        let mut matrix: Vec<(i32, Vec<i32>)> = Vec::new();
        for l in sorted {
            let (key, val) = if by_y { (l.y, l.x) } else { (l.x, l.y) };
            match matrix.last_mut() {
                Some((k, items)) if *k == key => items.push(val),
                _ => matrix.push((key, vec![val])),
            }
        }
        matrix
    }
}

impl PixelEffect for Garlands {
    fn setup_render(&mut self, _info: &PixelInfo) {
        self.frames = 0;
        self.speed = 0;
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let (buf_wi, buf_ht) = (info.buffer_wi(), info.buffer_ht());
        let interval_pos_factor = info.interval_position(frame) * 100.0;
        let level = self.level_curve.get_value(interval_pos_factor) / 100.0;
        let pixel_spacing = self.calculate_spacing(interval_pos_factor) * buf_ht / 100 + 3;
        let (width, height) = if matches!(self.direction, Direction::Up | Direction::Down) { (buf_wi, buf_ht) } else { (buf_ht, buf_wi) };
        let limit = height.wrapping_mul(pixel_spacing).wrapping_mul(4);
        let state = self.garlands_state(limit, interval_pos_factor, info);

        for ring in 0..height {
            let ratio = ring as f64 / height as f64;
            let col = dim(self.multi_color_blend(ratio, false, frame, info), level);
            let y = state - ring * pixel_spacing;
            let y_limit = height - ring - 1;
            for x in 0..width {
                let mut yadj = y - self.type_offset(x);
                if yadj < y_limit {
                    yadj = y_limit;
                }
                match self.direction {
                    Direction::Down => {
                        if yadj < buf_ht {
                            buf.set_pixel(x, buf_ht - 1 - yadj, col);
                        }
                    }
                    Direction::Up => {
                        if yadj < buf_ht {
                            buf.set_pixel(x, yadj, col);
                        }
                    }
                    Direction::Left => {
                        if yadj < buf_wi {
                            buf.set_pixel(buf_wi - 1 - yadj, x, col);
                        }
                    }
                    Direction::Right => {
                        if yadj < buf_wi {
                            buf.set_pixel(yadj, x, col);
                        }
                    }
                }
            }
        }
    }

    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        let matrix = self.sparse_matrix(buf.element_locations());
        let height = matrix.len() as i32;
        if height == 0 {
            return;
        }
        for frame in 0..num_frames {
            buf.current_frame = frame;
            let interval_pos_factor = info.interval_position(frame) * 100.0;
            let level = self.level_curve.get_value(interval_pos_factor) / 100.0;
            let pixel_spacing = self.calculate_spacing(interval_pos_factor) * height / 100 + 3;
            let limit = height.wrapping_mul(pixel_spacing).wrapping_mul(4);
            let state = self.garlands_state(limit, interval_pos_factor, info);

            for ring in 0..height {
                let ratio = ring as f64 / height as f64;
                let col = dim(self.multi_color_blend(ratio, false, frame, info), level);
                let mut y = state - ring * pixel_spacing;
                let y_limit = height - ring - 1;
                if y < y_limit {
                    y = y_limit;
                }
                self.draw_pattern(&matrix, buf, y, y_limit, height, col);
            }
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
    fn speed_mode_hides_iterations() {
        let mut p = DESCRIPTOR.new_params();
        p.set("MovementType", ParamValue::Enum("Speed".to_string()));
        let visible: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(visible.contains(&"SpeedCurve"));
        assert!(!visible.contains(&"Iterations"));
    }

    fn garlands(colors: Vec<ColorGradient>) -> Garlands {
        Garlands {
            direction: Direction::Up,
            movement_speed: false,
            iterations: 1,
            garland_type: 0,
            colors,
            level_curve: Curve::flat100(),
            speed_curve: Curve::flat(30.0),
            spacing_curve: Curve::flat(10.0),
            speed: 0,
            frames: 0,
        }
    }

    #[test]
    fn multi_color_blend_interpolates_between_gradients() {
        let g = garlands(vec![ColorGradient::solid(Color::rgb(255, 0, 0)), ColorGradient::solid(Color::rgb(0, 0, 255))]);
        let info = crate::test_util::info(2, 10, 1000);
        assert_eq!(g.multi_color_blend(0.0, false, 0, &info), Color::rgb(255, 0, 0));
        // ratio 0.5: red 255 + floor(-127.5 + 0.5) = 128, blue 0 + floor(127.5 + 0.5) = 128.
        assert_eq!(g.multi_color_blend(0.5, false, 0, &info), Color::rgb(128, 0, 128));
    }

    #[test]
    fn pattern_increment_cycles() {
        let mut p = PatternIncrement { increments: &[2, 2, 1], index: 0 };
        let seq: Vec<i32> = (0..5).map(|_| p.next()).collect();
        assert_eq!(seq, vec![2, 2, 1, 2, 2]);
    }

    #[test]
    fn type_one_offsets_follow_the_five_column_cycle() {
        let mut g = garlands(vec![ColorGradient::solid(Color::WHITE)]);
        g.garland_type = 1;
        let offsets: Vec<i32> = (0..5).map(|x| g.type_offset(x)).collect();
        assert_eq!(offsets, vec![0, 1, 2, 1, 0]);
    }
}
