//! Port of VixenModules.Effect.Curtain.

use crate::generated::curtain as gen;
use std::collections::HashMap;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    supports_location: true,
    ported: true,
    ..gen::BASE
};

/// `UpdateMovementTypeAttribute`, `UpdateStringOrientationAttributes` and `EnableTargetPositioning`.
/// Quirk kept from the C#: it also sets a property named "Iterations", but the property is really called
/// `Speed` (display name "Iterations"), so `Speed` is always visible.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let position = p.enum_name("MovementType") == "Position";
    v.set("PositionCurve", position);
    v.set("Direction", !position);
    v.set("StringOrientation", p.enum_name("TargetPositioning") == "Strings");
    v.set("TargetPositioning", true);
}

#[derive(Copy, Clone, PartialEq)]
enum Direction {
    Open,
    Close,
    OpenClose,
    CloseOpen,
}

impl Direction {
    fn index(self) -> i32 {
        self as i32
    }
}

#[derive(Copy, Clone, PartialEq)]
enum Edge {
    Left,
    Center,
    Right,
    Bottom,
    Middle,
    Top,
}

struct Curtain {
    direction: Direction,
    edge: Edge,
    movement_iterations: bool,
    speed: i32,
    swag: i32,
    intensity_per_iteration: bool,
    position_curve: Curve,
    level_curve: Curve,
    color: ColorGradient,
    last_curtain_dir: i32,
    last_curtain_limit: i32,
    position: f64,
    /// Locations grouped by X, built once per render for the swag lookups.
    by_x: HashMap<i32, Vec<ElementLocation>>,
    by_y: HashMap<i32, Vec<ElementLocation>>,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut e = Curtain {
        direction: match p.enum_name("Direction") {
            "CurtainClose" => Direction::Close,
            "CurtainOpenClose" => Direction::OpenClose,
            "CurtainCloseOpen" => Direction::CloseOpen,
            _ => Direction::Open,
        },
        edge: match p.enum_name("Edge") {
            "Left" => Edge::Left,
            "Right" => Edge::Right,
            "Bottom" => Edge::Bottom,
            "Middle" => Edge::Middle,
            "Top" => Edge::Top,
            _ => Edge::Center,
        },
        movement_iterations: p.enum_name("MovementType") == "Iterations",
        speed: p.int("Speed"),
        swag: p.int("Swag"),
        intensity_per_iteration: p.boolean("IntensityPerIteration"),
        position_curve: p.curve("PositionCurve"),
        level_curve: p.curve("LevelCurve"),
        color: p.gradient("Color"),
        last_curtain_dir: 0,
        last_curtain_limit: 0,
        position: 0.0,
        by_x: HashMap::new(),
        by_y: HashMap::new(),
    };
    render_pixel_effect(ctx, p, &mut e)
}

/// `hsv.V *= level` applied to a colour when `level < 1`.
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

impl Curtain {
    fn calculate_position(&self, interval_pos: f64) -> f64 {
        scale_curve_to_value(self.position_curve.get_value(interval_pos), 0.0, 100.0)
    }

    /// `(frame * Speed) % GetNumberFrames()`, with the zero-frame throw avoided.
    fn iteration_frame(&self, frame: i32, info: &PixelInfo) -> i32 {
        let n = info.number_frames();
        if n == 0 {
            0
        } else {
            frame.wrapping_mul(self.speed) % n
        }
    }

    /// Position and level, the same in both render paths.
    fn position_and_level(&mut self, frame: i32, info: &PixelInfo) -> f64 {
        let time_interval_position = info.interval_position(frame);
        let iter_pos = info.interval_position(self.iteration_frame(frame, info)) * 100.0;
        self.position = if self.movement_iterations {
            (time_interval_position * self.speed as f64) % 1.0
        } else {
            self.calculate_position(iter_pos) / 100.0
        };
        if self.intensity_per_iteration {
            self.level_curve.get_value(info.interval_position(self.iteration_frame(frame, info)) * 100.0) / 100.0
        } else {
            self.level_curve.get_value(time_interval_position * 100.0) / 100.0
        }
    }

    /// The x and y limits, before the direction flip.
    fn limits(&self, swaglen: i32, wi: i32, ht: i32) -> (i32, i32) {
        let pos = self.position;
        let (wi_f, ht_f, sw) = (wi as f64, ht as f64, swaglen as f64);
        if !self.movement_iterations {
            return (((pos * wi_f) + (sw * pos * 2.0)) as i32, ((pos * ht_f) + (sw * pos * 2.0)) as i32);
        }
        match self.direction {
            Direction::Open => (((pos * wi_f) + (sw * pos * 2.0)) as i32, ((pos * ht_f) + (sw * pos * 2.0)) as i32),
            Direction::Close => (((pos * wi_f) - (sw * (1.0 - pos) * 2.0)) as i32, ((pos * ht_f) - (sw * (1.0 - pos) * 2.0)) as i32),
            Direction::OpenClose => {
                if pos <= 0.5 {
                    (((pos * 2.0 * wi_f) + (sw * pos * 4.0)) as i32, ((pos * 2.0 * ht_f) + (sw * pos * 4.0)) as i32)
                } else {
                    (((pos - 0.5) * 2.0 * wi_f - (sw * (1.0 - pos) * 4.0)) as i32, ((pos - 0.5) * 2.0 * ht_f - (sw * (1.0 - pos) * 4.0)) as i32)
                }
            }
            Direction::CloseOpen => {
                if pos <= 0.5 {
                    (((pos * 2.0 * wi_f) - (sw * (0.5 - pos) * 4.0)) as i32, ((pos * 2.0 * ht_f) - (sw * (0.5 - pos) * 4.0)) as i32)
                } else {
                    (((pos - 0.5) * 2.0 * wi_f + (sw * pos * 2.0)) as i32, ((pos - 0.5) * 2.0 * ht_f + (sw * pos * 2.0)) as i32)
                }
            }
        }
    }

    fn curtain_dir(&mut self, xlimit: i32, swaglen: i32) -> i32 {
        let dir = if matches!(self.direction, Direction::Open | Direction::Close) || !self.movement_iterations {
            self.direction.index() % 2
        } else if xlimit < self.last_curtain_limit.wrapping_sub(swaglen * 2) {
            1 - self.last_curtain_dir
        } else {
            self.last_curtain_dir
        };
        self.last_curtain_dir = dir;
        self.last_curtain_limit = xlimit;
        dir
    }

    fn draw_curtain(&self, left_edge: bool, xlimit: i32, swag: &[i32], buf: &mut PixelFrameBuffer, level: f64, width: i32, wi: i32, ht: i32) {
        let w = width as f64;
        for i in 0..xlimit.max(0) {
            let col = dim(self.color.get_color_at(i as f64 / w), level);
            let x = if left_edge { wi - i - 1 } else { i };
            for y in (0..ht).rev() {
                buf.set_pixel(x, y, col);
            }
        }
        for (i, &s) in swag.iter().enumerate() {
            let mut x = xlimit + i as i32;
            let col = dim(self.color.get_color_at(x as f64 / w), level);
            if left_edge {
                x = wi - x - 1;
            }
            let mut y = ht - 1;
            while y > s {
                buf.set_pixel(x, y, col);
                y -= 1;
            }
        }
    }

    fn draw_curtain_vertical(&self, top_edge: bool, ylimit: i32, swag: &[i32], buf: &mut PixelFrameBuffer, level: f64, width: i32, wi: i32, ht: i32) {
        let w = width as f64;
        for i in 0..ylimit.max(0) {
            let col = dim(self.color.get_color_at(i as f64 / w), level);
            let y = if top_edge { ht - i - 1 } else { i };
            for x in (0..wi).rev() {
                buf.set_pixel(x, y, col);
            }
        }
        for (i, &s) in swag.iter().enumerate() {
            let mut y = ylimit + i as i32;
            let col = dim(self.color.get_color_at(y as f64 / w), level);
            if top_edge {
                y = ht - y - 1;
            }
            let mut x = wi - 1;
            while x > s {
                buf.set_pixel(x, y, col);
                x -= 1;
            }
        }
    }

    fn location_color(&self, left_edge: bool, width: i32, l: &ElementLocation, info: &PixelInfo) -> Color {
        let wi = info.buffer_wi();
        let w = width as f64;
        if !left_edge || wi == width {
            let percent = (l.x - info.buffer_wi_offset()) as f64 / w;
            self.color.get_color_at(if left_edge { 1.0 - percent } else { percent })
        } else {
            let percent = (l.x - info.buffer_wi_offset() - (wi - width)) as f64 / w;
            self.color.get_color_at(1.0 - percent)
        }
    }

    fn vertical_location_color(&self, top_edge: bool, width: i32, l: &ElementLocation, info: &PixelInfo) -> Color {
        let ht = info.buffer_ht();
        let w = width as f64;
        if !top_edge || ht == width {
            let percent = (l.y - info.buffer_ht_offset()) as f64 / w;
            self.color.get_color_at(if top_edge { 1.0 - percent } else { percent })
        } else {
            let percent = (l.y - info.buffer_ht_offset() - (ht - width)) as f64 / w;
            self.color.get_color_at(1.0 - percent)
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_curtain_location(&self, left_edge: bool, xlimit: i32, swag: &[i32], buf: &mut PixelLocationFrameBuffer, level: f64, width: i32, info: &PixelInfo, locs: &[ElementLocation]) {
        let (wi, wi_off, ht, ht_off) = (info.buffer_wi(), info.buffer_wi_offset(), info.buffer_ht(), info.buffer_ht_offset());
        let right_buffer_limit = wi + wi_off;
        for l in locs {
            let inside = if left_edge { l.x >= right_buffer_limit - xlimit } else { l.x <= xlimit + wi_off };
            if inside {
                let col = dim(self.location_color(left_edge, width, l, info), level);
                buf.set_pixel(l.x, l.y, col);
            }
        }
        let n = swag.len() as i32;
        for (i, &s) in swag.iter().enumerate() {
            let i = i as i32;
            let x = if left_edge { right_buffer_limit - (xlimit + i) } else { xlimit + i + wi_off };
            let limit = ht - s + ht_off;
            if let Some(list) = self.by_x.get(&x) {
                for l in list {
                    let in_swag = if left_edge {
                        l.x <= right_buffer_limit - xlimit && l.x >= right_buffer_limit - (xlimit + n)
                    } else {
                        l.x >= xlimit + wi_off && l.x <= xlimit + n + wi_off
                    };
                    if in_swag && l.y < limit {
                        let col = dim(self.location_color(left_edge, width, l, info), level);
                        buf.set_pixel(l.x, l.y, col);
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_curtain_vertical_location(&self, top_edge: bool, ylimit: i32, swag: &[i32], buf: &mut PixelLocationFrameBuffer, level: f64, width: i32, info: &PixelInfo, locs: &[ElementLocation]) {
        let (ht, ht_off, wi_off) = (info.buffer_ht(), info.buffer_ht_offset(), info.buffer_wi_offset());
        let top_buffer_limit = ht + ht_off;
        for l in locs {
            let inside = if top_edge { l.y > top_buffer_limit - ylimit } else { l.y < ylimit + ht_off };
            if inside {
                let col = dim(self.vertical_location_color(top_edge, width, l, info), level);
                buf.set_pixel(l.x, l.y, col);
            }
        }
        let n = swag.len() as i32;
        for (i, &s) in swag.iter().enumerate() {
            let i = i as i32;
            let y = if top_edge { top_buffer_limit - (ylimit + i) } else { ylimit + i + ht_off };
            let limit = wi_off + s;
            if let Some(list) = self.by_y.get(&y) {
                for l in list {
                    let in_swag = if top_edge {
                        l.y <= top_buffer_limit - ylimit && l.y >= top_buffer_limit - (ylimit + n)
                    } else {
                        l.y >= ylimit + ht_off && l.y <= ylimit + n + ht_off
                    };
                    if in_swag && l.x > limit {
                        let col = dim(self.vertical_location_color(top_edge, width, l, info), level);
                        buf.set_pixel(l.x, l.y, col);
                    }
                }
            }
        }
    }
}

impl PixelEffect for Curtain {
    fn setup_render(&mut self, _info: &PixelInfo) {
        self.last_curtain_dir = if matches!(self.direction, Direction::Open | Direction::OpenClose) || !self.movement_iterations { 0 } else { 1 };
        self.last_curtain_limit = 0;
        self.position = 0.0;
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let (wi, ht) = (info.buffer_wi(), info.buffer_ht());
        let swaglen = if ht > 1 { self.swag * wi / 40 } else { 0 };
        let level = self.position_and_level(frame, info);

        let mut swag: Vec<i32> = Vec::new();
        if swaglen > 0 {
            let a = (ht - 1) as f64 / (swaglen * swaglen) as f64;
            for x in 0..swaglen {
                swag.push((a * x as f64 * x as f64) as i32);
            }
        }
        let (mut xlimit, mut ylimit) = self.limits(swaglen, wi, ht);
        let dir = self.curtain_dir(xlimit, swaglen);
        if dir == 0 {
            xlimit = wi - xlimit - 1;
            ylimit = ht - ylimit - 1;
        }
        match self.edge {
            Edge::Left => self.draw_curtain(true, xlimit, &swag, buf, level, wi - 1, wi, ht),
            Edge::Center => {
                let middle = (xlimit + 1) / 2;
                self.draw_curtain(true, middle, &swag, buf, level, wi / 2 - 1, wi, ht);
                self.draw_curtain(false, middle, &swag, buf, level, wi / 2 - 1, wi, ht);
            }
            Edge::Right => self.draw_curtain(false, xlimit, &swag, buf, level, wi - 1, wi, ht),
            Edge::Bottom => self.draw_curtain_vertical(true, ylimit, &swag, buf, level, ht - 1, wi, ht),
            Edge::Middle => {
                let middle = (ylimit + 1) / 2;
                self.draw_curtain_vertical(true, middle, &swag, buf, level, ht / 2 - 1, wi, ht);
                self.draw_curtain_vertical(false, middle, &swag, buf, level, ht / 2 - 1, wi, ht);
            }
            Edge::Top => self.draw_curtain_vertical(false, ylimit, &swag, buf, level, ht - 1, wi, ht),
        }
    }

    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        let (wi, ht) = (info.buffer_wi(), info.buffer_ht());
        let locs = buf.element_locations().to_vec();
        self.by_x.clear();
        self.by_y.clear();
        for l in &locs {
            self.by_x.entry(l.x).or_default().push(*l);
            self.by_y.entry(l.y).or_default().push(*l);
        }
        let swaglen = if ht > 1 { self.swag * wi / 40 } else { 0 };
        let swag_buffer_ht = match self.edge {
            Edge::Middle | Edge::Top | Edge::Bottom => wi,
            _ => ht,
        };
        let mut swag: Vec<i32> = Vec::new();
        for frame in 0..num_frames {
            buf.current_frame = frame;
            let level = self.position_and_level(frame, info);
            if swaglen > 0 {
                swag.clear();
                let a = swag_buffer_ht as f64 / (swaglen * swaglen) as f64;
                for x in 0..swaglen {
                    swag.push((a * x as f64 * x as f64) as i32);
                }
            }
            let (mut xlimit, mut ylimit) = self.limits(swaglen, wi, ht);
            let dir = self.curtain_dir(xlimit, swaglen);
            if dir == 0 {
                xlimit = wi - xlimit;
                ylimit = ht - ylimit;
            }
            match self.edge {
                Edge::Left => self.draw_curtain_location(true, xlimit, &swag, buf, level, wi, info, &locs),
                Edge::Center => {
                    let middle = xlimit / 2;
                    self.draw_curtain_location(true, middle, &swag, buf, level, wi / 2, info, &locs);
                    self.draw_curtain_location(false, middle, &swag, buf, level, wi / 2, info, &locs);
                }
                Edge::Right => self.draw_curtain_location(false, xlimit, &swag, buf, level, wi, info, &locs),
                Edge::Bottom => self.draw_curtain_vertical_location(false, ylimit, &swag, buf, level, ht, info, &locs),
                Edge::Middle => {
                    let middle = ylimit / 2;
                    self.draw_curtain_vertical_location(true, middle, &swag, buf, level, ht / 2, info, &locs);
                    self.draw_curtain_vertical_location(false, middle, &swag, buf, level, ht / 2, info, &locs);
                }
                Edge::Top => self.draw_curtain_vertical_location(true, ylimit, &swag, buf, level, ht, info, &locs),
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
    fn position_mode_hides_direction() {
        let p = DESCRIPTOR.new_params();
        let visible: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(visible.contains(&"PositionCurve"));
        assert!(!visible.contains(&"Direction"));
        assert!(visible.contains(&"Speed"));
    }

    fn curtain(direction: Direction, edge: Edge, position: f64) -> Curtain {
        Curtain {
            direction,
            edge,
            movement_iterations: true,
            speed: 1,
            swag: 0,
            intensity_per_iteration: false,
            position_curve: Curve::flat100(),
            level_curve: Curve::flat100(),
            color: ColorGradient::solid(Color::RED),
            last_curtain_dir: 0,
            last_curtain_limit: 0,
            position,
            by_x: HashMap::new(),
            by_y: HashMap::new(),
        }
    }

    #[test]
    fn open_from_left_covers_leading_columns() {
        // Open, Left edge, halfway across a 10 wide buffer: the right 5 columns are drawn (left edge draws from x = wi-1).
        let mut c = curtain(Direction::Open, Edge::Left, 0.5);
        let mut buf = PixelFrameBuffer::new(10, 4);
        buf.fill(Color::BLACK);
        let (xl, _) = c.limits(0, 10, 4);
        assert_eq!(xl, 5);
        let dir = c.curtain_dir(xl, 0);
        assert_eq!(dir, 0);
        c.draw_curtain(true, 10 - xl - 1, &[], &mut buf, 1.0, 9, 10, 4);
        assert_eq!(buf.get_color_at(9, 0), Color::RED);
        assert_eq!(buf.get_color_at(0, 0), Color::BLACK);
    }
}
