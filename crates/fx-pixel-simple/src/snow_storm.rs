//! Port of VixenModules.Effect.SnowStorm.

use crate::generated::snow_storm as gen;
use vixen_effects::prelude::*;

// The C# constructor never calls `EnableTargetPositioning` and has no `RenderEffectByLocation`.
pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    ported: true,
    ..gen::BASE
};

/// `UpdateColorAttribute`: the gradients are unused by the Rainbow colour type.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    v.set("Colors", p.enum_name("ColorType") != "RainBow");
}

#[derive(Copy, Clone, PartialEq)]
enum ColorType {
    RainBow,
    Range,
    Palette,
    Gradient,
}

/// `SnowstormClass`.
struct Item {
    points: Vec<(i32, i32)>,
    hsv: Hsv,
    idx: i32,
    ss_decay: i32,
    color: usize,
    expired: bool,
}

struct SnowStorm {
    color_type: ColorType,
    colors: Vec<ColorGradient>,
    count_curve: Curve,
    length_curve: Curve,
    level_curve: Curve,
    speed_curve: Curve,
    reverse: bool,
    gradient_position: f64,
    items: Vec<Item>,
    rand: Rand,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    // RainBow doesn't use the list. Otherwise a modulo by its empty count throws in the C#, and a pixel effect that throws
    // has no intents at all.
    if p.enum_name("ColorType") != "RainBow" && p.gradients("Colors").is_empty() {
        return EffectIntents::new();
    }
    let mut e = SnowStorm {
        color_type: match p.enum_name("ColorType") {
            "RainBow" => ColorType::RainBow,
            "Range" => ColorType::Range,
            "Gradient" => ColorType::Gradient,
            _ => ColorType::Palette,
        },
        colors: p.gradients("Colors"),
        count_curve: p.curve("CountCurve"),
        length_curve: p.curve("LengthCurve"),
        level_curve: p.curve("LevelCurve"),
        speed_curve: p.curve("SpeedCurve"),
        reverse: p.boolean("ReverseDirection"),
        gradient_position: 0.0,
        items: Vec::new(),
        rand: Rand::default(),
    };
    render_pixel_effect(ctx, p, &mut e)
}

/// `SnowstormVector`.
fn snowstorm_vector(idx: i32) -> (i32, i32) {
    match idx {
        0 => (-1, 0),
        1 => (-1, -1),
        2 => (0, -1),
        3 => (1, -1),
        4 => (1, 0),
        5 => (1, 1),
        6 => (0, 1),
        _ => (-1, 1),
    }
}

impl SnowStorm {
    fn calculate_length(&self, interval_pos: f64) -> f64 {
        scale_curve_to_value(self.length_curve.get_value(interval_pos), 20.0, 1.0)
    }

    fn calculate_count(&self, interval_pos: f64) -> f64 {
        scale_curve_to_value(self.count_curve.get_value(interval_pos), 100.0, 1.0)
    }

    fn calculate_speed(&self, interval_pos: f64, info: &PixelInfo) -> f64 {
        let value = scale_curve_to_value(self.speed_curve.get_value(interval_pos), 100.0, 1.0) * info.frame_time as f64 / 50.0;
        if value < 1.0 {
            1.0
        } else {
            value
        }
    }

    /// `RandomRange`.
    fn random_range(&mut self, num1: f64, num2: f64) -> f64 {
        let (lo, hi) = if num1 < num2 { (num1, num2) } else { (num2, num1) };
        (self.rand.next_double() * (hi - lo) + lo) as f32 as f64
    }

    /// `SetRangeColor`.
    fn set_range_color(&mut self, a: Hsv, b: Hsv) -> Hsv {
        let h = self.random_range(a.h, b.h);
        let s = self.random_range(a.s, b.s);
        Hsv::new(h, s, 1.0)
    }

    fn random_color(&mut self, pos: f64) -> Hsv {
        let n = self.colors.len() as i32;
        let i = self.rand.next() % n;
        Hsv::from_color(self.colors[i as usize].get_color_at(pos))
    }

    /// `SnowstormAdvance`.
    fn advance(&mut self, item_index: usize, wi: i32, ht: i32) {
        const CNT: usize = 8;
        const ARR: [i32; 16] = [30, 20, 10, 5, 0, 5, 10, 20, 20, 15, 10, 10, 10, 10, 10, 15];
        let mut adv = snowstorm_vector(7);
        let idx = self.items[item_index].idx;
        let i0 = if idx % 7 <= 4 { 0 } else { CNT };
        let r = self.rand.next() % 100;
        let mut val = 0;
        for i in 0..CNT {
            val += ARR[i0 + i];
            if r < val {
                adv = snowstorm_vector(i as i32);
                break;
            }
        }
        if idx % 3 == 0 {
            adv.0 *= 2;
            adv.1 *= 2;
        }
        let item = &mut self.items[item_index];
        let Some(&(px, py)) = item.points.last() else { return };
        let mut x = (px + adv.0) % wi;
        let mut y = (py + adv.1) % ht;
        if x < 0 {
            x += wi;
        }
        if y < 0 {
            y += ht;
        }
        item.points.push((x, y));
    }
}

impl PixelEffect for SnowStorm {
    fn setup_render(&mut self, _info: &PixelInfo) {
        self.items = Vec::new();
    }

    fn cleanup_render(&mut self) {
        self.items = Vec::new();
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let (wi, ht) = (info.buffer_wi(), info.buffer_ht());
        let color_count = self.colors.len() as i32;
        // The C# throws (a modulo by zero) with no gradients, unless the colours are not used.
        if wi <= 0 || ht <= 0 || (color_count == 0 && self.color_type != ColorType::RainBow) {
            return;
        }
        let interval_pos_factor = info.interval_position(frame) * 100.0;
        let adjust_speed = self.calculate_speed(interval_pos_factor, info);
        let count = (wi as f64 * ht as f64 * self.calculate_count(interval_pos_factor) / 2000.0).round_ties_even() as i32 + 1;
        let tail_length = ((wi as f64 * ht as f64 * self.calculate_length(interval_pos_factor) / 2000.0) + 2.0).round_ties_even() as i32;
        let tail_length = tail_length.max(1);
        let level = self.level_curve.get_value(interval_pos_factor) / 100.0;

        if frame == 0 {
            self.items.clear();
        }
        // Create snowstorm elements.
        for i in 0..count {
            if (self.items.len() as i32) < count {
                let mut item = Item { points: Vec::new(), hsv: Hsv::default(), idx: i, ss_decay: 0, color: 0, expired: false };
                match self.color_type {
                    ColorType::Range => {
                        // Two random colours from the list for each snowstorm.
                        let a = self.random_color(interval_pos_factor / 100.0);
                        let b = self.random_color(interval_pos_factor / 100.0);
                        item.hsv = self.set_range_color(a, b);
                    }
                    ColorType::Palette => {
                        item.hsv = self.random_color(interval_pos_factor / 100.0);
                    }
                    ColorType::Gradient => {
                        item.color = (self.rand.next() % color_count) as usize;
                        self.gradient_position = 100.0 / tail_length as f64 / 100.0;
                        item.hsv = Hsv::from_color(self.colors[item.color].get_color_at(0.0));
                    }
                    ColorType::RainBow => {}
                }
                // Start in a random state.
                let mut r = self.rand.next() % (2 * tail_length);
                if r > 0 {
                    let x = self.rand.next() % wi;
                    let y = self.rand.next() % ht;
                    item.points.push((x, y));
                }
                if r >= tail_length {
                    item.ss_decay = r - tail_length;
                    r = tail_length;
                }
                self.items.push(item);
                let last = self.items.len() - 1;
                for _ in 1..r {
                    self.advance(last, wi, ht);
                }
            }
        }

        // Render snowstorm items.
        for k in 0..self.items.len() {
            if self.items[k].points.is_empty() {
                let x = self.rand.next() % wi;
                let y = self.rand.next() % ht;
                self.items[k].points.push((x, y));
            } else if ((self.rand.next() % 20) as f64) < adjust_speed {
                self.advance(k, wi, ht);
            }
            let sz = self.items[k].points.len() as i32;
            for pt in 0..sz as usize {
                match self.color_type {
                    ColorType::RainBow => {
                        // Rainbow uses no user colours.
                        let h = ((self.rand.next() % 1000) as f32 / 1000.0f32) as f64;
                        let it = &mut self.items[k];
                        it.hsv.set_h(h);
                        it.hsv.set_s(1.0);
                        it.hsv.set_v(1.0);
                    }
                    ColorType::Gradient => {
                        let c = self.colors[self.items[k].color].get_color_at(self.gradient_position * pt as f64);
                        self.items[k].hsv = Hsv::from_color(c);
                    }
                    _ => {}
                }
                let it = &mut self.items[k];
                let mut hsv = it.hsv;
                hsv.set_v((1.0 - (sz - pt as i32 + it.ss_decay) as f64 / tail_length as f64) as f32 as f64);
                hsv.set_v(hsv.v * level);
                let (px, py) = it.points[pt];
                if px >= wi - 1 || py >= ht - 1 || px <= 1 || py <= 1 {
                    // Flags snowstorms that have reached the end of the grid as expired.
                    it.expired = true;
                    break;
                }
                if hsv.v < 0.0 {
                    hsv.set_v(0.0);
                }
                if !self.reverse {
                    buf.set_pixel_hsv(px, py, hsv);
                } else {
                    buf.set_pixel_hsv(wi - px, py, hsv);
                }
            }
        }
        // Delete old snowstorms.
        self.items.retain(|i| !i.expired);
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
    fn rainbow_hides_the_gradients() {
        let mut p = DESCRIPTOR.new_params();
        let shown = |p: &Params| DESCRIPTOR.visible_params(p).into_iter().any(|i| DESCRIPTOR.params[i].name == "Colors");
        assert!(shown(&p));
        p.set("ColorType", ParamValue::Enum("RainBow".to_string()));
        assert!(!shown(&p));
    }

    #[test]
    fn vectors_walk_the_eight_neighbours() {
        assert_eq!(snowstorm_vector(0), (-1, 0));
        assert_eq!(snowstorm_vector(5), (1, 1));
        assert_eq!(snowstorm_vector(7), (-1, 1));
        assert_eq!(snowstorm_vector(99), (-1, 1));
    }

    #[test]
    fn advance_wraps_inside_the_buffer() {
        let mut s = SnowStorm {
            color_type: ColorType::RainBow,
            colors: Vec::new(),
            count_curve: Curve::flat(10.0),
            length_curve: Curve::flat(25.0),
            level_curve: Curve::flat100(),
            speed_curve: Curve::flat(50.0),
            reverse: false,
            gradient_position: 0.0,
            items: vec![Item { points: vec![(0, 0)], hsv: Hsv::default(), idx: 1, ss_decay: 0, color: 0, expired: false }],
            rand: Rand::seeded(7),
        };
        for _ in 0..200 {
            s.advance(0, 5, 4);
        }
        assert!(s.items[0].points.iter().all(|&(x, y)| (0..5).contains(&x) && (0..4).contains(&y)));
        assert_eq!(s.items[0].points.len(), 201);
    }
}
