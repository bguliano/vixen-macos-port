//! Port of VixenModules.Effect.Life.

use crate::generated::life as gen;
use vixen_effects::prelude::*;

// The C# constructor never calls `EnableTargetPositioning` and has no `RenderEffectByLocation`, so there
// is no visibility rule and no location support.
pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    ported: true,
    ..gen::BASE
};

struct Life {
    life_type: i32,
    cells_to_start: i32,
    colors: Vec<ColorGradient>,
    level_curve: Curve,
    speed_curve: Curve,
    /// `_tempbuf`, stored `[x][y]` in one allocation.
    tempbuf: Vec<Color>,
    last_life_state: i64,
    rand: Rand,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    // Indexing the empty list throws in the C#, and a pixel effect that throws has no intents at all.
    if p.gradients("Colors").is_empty() {
        return EffectIntents::new();
    }
    let mut e = Life {
        life_type: p.int("Type"),
        cells_to_start: p.int("CellsToStart"),
        colors: p.gradients("Colors"),
        level_curve: p.curve("LevelCurve"),
        speed_curve: p.curve("SpeedCurve"),
        tempbuf: Vec::new(),
        last_life_state: 0,
        rand: Rand::default(),
    };
    render_pixel_effect(ctx, p, &mut e)
}

#[inline]
fn dim(color: Color, level: f64) -> Color {
    if level < 1.0 {
        let mut hsv = Hsv::from_color(color);
        hsv.set_v(hsv.v * level);
        hsv.to_color()
    } else {
        color
    }
}

/// `Color != Color.Transparent`. The C# also tests `!= Color.Black`, but .NET compares the known-colour
/// name too, so a computed black (`Color.FromArgb(0, 0, 0)`) never equals `Color.Black` and that test is
/// always true. Only transparent cells are dead.
#[inline]
fn is_live(c: Color) -> bool {
    c != Color::TRANSPARENT
}

impl Life {
    fn gradient_at(&self, idx: usize, pos: f64) -> Color {
        match self.colors.get(idx) {
            Some(g) => g.get_color_at(pos),
            None => Color::EMPTY,
        }
    }

    fn multi_color_blend(&self, mut n: f64, frame: i32, info: &PixelInfo) -> Color {
        let count = self.colors.len();
        if count <= 1 {
            return self.gradient_at(0, info.interval_position(frame));
        }
        if n >= 1.0 {
            n = 0.99999;
        }
        if n < 0.0 {
            n = 0.0;
        }
        let idx1 = (n * count as f64).floor() as i32;
        let idx2 = (idx1 + 1) % count as i32;
        let ratio = n * count as f64 - idx1 as f64;
        let pos = info.interval_position(frame);
        let c1 = self.gradient_at(idx1 as usize, pos);
        let c2 = self.gradient_at(idx2 as usize, pos);
        Color::from_rgb_i32(
            Self::channel_blend_round(c1.r as i32, c2.r as i32, ratio),
            Self::channel_blend_round(c1.g as i32, c2.g as i32, ratio),
            Self::channel_blend_round(c1.b as i32, c2.b as i32, ratio),
        )
    }

    /// `GetMultiColorBlend(RandDouble(), frame)`.
    fn random_color(&mut self, frame: i32, info: &PixelInfo) -> Color {
        let n = self.rand.next_double();
        self.multi_color_blend(n, frame, info)
    }

    /// `ChannelBlend`: `c1 + (int)Math.Floor(ratio * (c2 - c1) + 0.5)`.
    fn channel_blend_round(c1: i32, c2: i32, ratio: f64) -> i32 {
        c1 + (ratio * (c2 - c1) as f64 + 0.5).floor() as i32
    }

    fn calculate_speed(&self, interval_pos: f64, info: &PixelInfo) -> f64 {
        let value = scale_curve_to_value(self.speed_curve.get_value(interval_pos), 20.0, 1.0) * info.frame_time as f64 / 50.0;
        if value < 1.0 {
            1.0
        } else {
            value
        }
    }

    /// `GetTempPixel`.
    #[inline]
    fn temp_pixel(&self, x: i32, y: i32, wi: i32, ht: i32) -> Color {
        if x >= 0 && x < wi && y >= 0 && y < ht {
            self.tempbuf[x as usize * ht as usize + y as usize]
        } else {
            Color::TRANSPARENT
        }
    }

    /// `Life_CountNeighbors`.
    fn count_neighbors(&self, x0: i32, y0: i32, wi: i32, ht: i32) -> i32 {
        const N_X: [i32; 8] = [-1, -1, -1, 0, 1, 1, 1, 0];
        const N_Y: [i32; 8] = [-1, 0, 1, 1, 1, 0, -1, -1];
        let mut cnt = 0;
        for i in 0..8 {
            let mut x = (x0 + N_X[i]) % wi;
            let mut y = (y0 + N_Y[i]) % ht;
            if x < 0 {
                x += wi;
            }
            if y < 0 {
                y += ht;
            }
            if self.temp_pixel(x, y, wi, ht) != Color::TRANSPARENT {
                cnt += 1;
            }
        }
        cnt
    }

    fn set_frame_pixel(buf: &mut PixelFrameBuffer, color: Color, level: f64, x: i32, y: i32) {
        buf.set_pixel(x, y, dim(color, level));
    }
}

impl PixelEffect for Life {
    fn setup_render(&mut self, info: &PixelInfo) {
        self.tempbuf = vec![Color::TRANSPARENT; (info.buffer_wi().max(0) as usize) * (info.buffer_ht().max(0) as usize)];
    }

    fn cleanup_render(&mut self) {
        self.tempbuf = Vec::new();
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let (wi, ht) = (info.buffer_wi(), info.buffer_ht());
        if wi <= 0 || ht <= 0 {
            return;
        }
        let level = self.level_curve.get_value(info.interval_position(frame) * 100.0) / 100.0;
        let interval_pos_factor = info.interval_position(frame) * 100.0;
        let state = frame as f64 * self.calculate_speed(interval_pos_factor, info);
        let count = wi.wrapping_mul(ht).wrapping_mul(self.cells_to_start) / 200 + 1;
        if frame == 0 {
            for _ in 0..count {
                let x = self.rand.next() % wi;
                let y = self.rand.next() % ht;
                let color = dim(self.random_color(frame, info), level);
                self.tempbuf[x as usize * ht as usize + y as usize] = color;
            }
        }
        let temp_state = ((state % 400.0) as i64) / 20;
        if temp_state == self.last_life_state {
            // CopyTempBufToPixels
            for x in 0..wi {
                for y in 0..ht {
                    buf.set_pixel(x, y, self.tempbuf[x as usize * ht as usize + y as usize]);
                }
            }
            return;
        }
        self.last_life_state = temp_state;

        for x in 0..wi {
            for y in 0..ht {
                let color = self.temp_pixel(x, y, wi, ht);
                let live = is_live(color);
                let cnt = self.count_neighbors(x, y, wi, ht);
                match self.life_type {
                    0 => {
                        // B3/S23
                        if live && (2..=3).contains(&cnt) {
                            Self::set_frame_pixel(buf, color, level, x, y);
                        } else if !live && cnt == 3 {
                            let c = self.random_color(frame, info);
                            Self::set_frame_pixel(buf, c, level, x, y);
                        }
                    }
                    1 => {
                        // B35/S236
                        if live && (cnt == 2 || cnt == 3 || cnt == 6) {
                            let c = self.random_color(frame, info);
                            Self::set_frame_pixel(buf, c, level, x, y);
                        } else if !live && (cnt == 3 || cnt == 5) {
                            let c = self.random_color(frame, info);
                            Self::set_frame_pixel(buf, c, level, x, y);
                        }
                    }
                    2 => {
                        // B357/S1358
                        if live && (cnt == 1 || cnt == 3 || cnt == 5 || cnt == 8) {
                            Self::set_frame_pixel(buf, color, level, x, y);
                        } else if !live && (cnt == 3 || cnt == 5 || cnt == 7) {
                            let c = self.random_color(frame, info);
                            Self::set_frame_pixel(buf, c, level, x, y);
                        }
                    }
                    3 => {
                        // B378/S235678
                        if live && (cnt == 2 || cnt == 3 || cnt >= 5) {
                            Self::set_frame_pixel(buf, color, level, x, y);
                        } else if !live && (cnt == 3 || cnt == 7 || cnt == 8) {
                            let c = self.random_color(frame, info);
                            Self::set_frame_pixel(buf, c, level, x, y);
                        }
                    }
                    4 => {
                        // B25678/S5678
                        if live && cnt >= 5 {
                            Self::set_frame_pixel(buf, color, level, x, y);
                        } else if !live && (cnt == 2 || cnt >= 5) {
                            let c = self.random_color(frame, info);
                            Self::set_frame_pixel(buf, c, level, x, y);
                        }
                    }
                    _ => {}
                }
            }
        }
        // CopyPixelsToTempBuf
        for x in 0..wi {
            for y in 0..ht {
                self.tempbuf[x as usize * ht as usize + y as usize] = buf.get_color_at(x, y);
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

    fn life(life_type: i32) -> Life {
        Life {
            life_type,
            cells_to_start: 50,
            colors: vec![ColorGradient::solid(Color::WHITE)],
            level_curve: Curve::flat100(),
            speed_curve: Curve::flat(25.0),
            tempbuf: Vec::new(),
            last_life_state: 0,
            rand: Rand::seeded(1),
        }
    }

    #[test]
    fn neighbours_wrap_around_the_edges() {
        let mut l = life(0);
        l.tempbuf = vec![Color::TRANSPARENT; 16];
        // A live cell at (3,3) is a neighbour of (0,0) on a 4x4 torus.
        l.tempbuf[3 * 4 + 3] = Color::WHITE;
        assert_eq!(l.count_neighbors(0, 0, 4, 4), 1);
        assert_eq!(l.count_neighbors(1, 1, 4, 4), 0);
    }

    #[test]
    fn frame_zero_only_seeds_cells() {
        let info = crate::test_util::info(6, 6, 1000);
        let mut l = life(0);
        l.setup_render(&info);
        let mut buf = PixelFrameBuffer::new(6, 6);
        buf.clear_buffer();
        l.render_effect(0, &info, &mut buf);
        // 6 * 6 * 50 / 200 + 1 = 10 seeds, some possibly overlapping.
        let lit = (0..6).flat_map(|x| (0..6).map(move |y| (x, y))).filter(|&(x, y)| buf.get_color_at(x, y) == Color::WHITE).count();
        assert!((1..=10).contains(&lit), "{lit}");
    }
}
