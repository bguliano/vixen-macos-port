//! Port of VixenModules.Effect.Balls.

use crate::generated::balls as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    supports_location: true,
    ported: true,
    ..gen::BASE
};

/// `UpdateFillAttribute`, `UpdateCollideAttributes`, `UpdateRandomMoveAttributes` and
/// `UpdateStringOrientationAttributes`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let fill = p.enum_name("BallFill");
    v.set("Colors", fill != "Inverse");
    v.set("BackgroundColor", fill == "Inverse");
    v.set("BallEdgeWidthCurve", fill == "Empty");
    v.set("ChangeCollideColor", p.boolean("Collide"));
    v.set("RandomMaxCurve", p.boolean("RandomMovement"));
    v.set("StringOrientation", p.enum_name("TargetPositioning") == "Strings");
    v.set("TargetPositioning", true);
}

#[derive(Copy, Clone, PartialEq)]
enum BallFill {
    Fade,
    Empty,
    Solid,
    Gradient,
    Inverse,
}

/// `BallClass`. The Guid is a counter.
#[derive(Clone, Copy)]
struct Ball {
    location_x: f64,
    location_y: f64,
    velocity_x: f64,
    velocity_y: f64,
    color_index: usize,
    radius: f64,
    id: u64,
    move_count: i32,
}

struct Balls {
    wrap: bool,
    fill: BallFill,
    random_movement: bool,
    random_radius: bool,
    collide: bool,
    change_collide_color: bool,
    locations: bool,
    colors: Vec<ColorGradient>,
    background_color: ColorGradient,
    level_curve: Curve,
    center_speed_curve: Curve,
    speed_variation_curve: Curve,
    size_curve: Curve,
    ball_count_curve: Curve,
    edge_width_curve: Curve,
    random_max_curve: Curve,

    balls: Vec<Ball>,
    remove_balls: Vec<u64>,
    next_id: u64,
    rand: Rand,
    /// Each gradient sampled at the current frame position, computed once per frame.
    frame_colors: Vec<Color>,
    /// `Colors[ball.ColorIndex]` throws on an empty list, and a pixel effect that throws has no intents at all.
    threw: std::cell::Cell<bool>,

    ball_count: i32,
    min_buffer: i32,
    max_buffer: i32,
    interval_pos: f32,
    interval_pos_factor: f64,
    radius: i32,
    center_speed: f64,
    speed_variation: f64,
    level: f64,
    buffer_ht: i32,
    buffer_wi: i32,
    frame_time: i32,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut e = Balls {
        wrap: p.enum_name("BallType") == "Wrap",
        fill: match p.enum_name("BallFill") {
            "Empty" => BallFill::Empty,
            "Solid" => BallFill::Solid,
            "Gradient" => BallFill::Gradient,
            "Inverse" => BallFill::Inverse,
            _ => BallFill::Fade,
        },
        random_movement: p.boolean("RandomMovement"),
        random_radius: p.boolean("RandomRadius"),
        collide: p.boolean("Collide"),
        change_collide_color: p.boolean("ChangeCollideColor"),
        locations: TargetPositioning::from_name(p.enum_name("TargetPositioning")) == TargetPositioning::Locations,
        colors: p.gradients("Colors"),
        background_color: p.gradient("BackgroundColor"),
        level_curve: p.curve("LevelCurve"),
        center_speed_curve: p.curve("CenterSpeedCurve"),
        speed_variation_curve: p.curve("SpeedVariationCurve"),
        size_curve: p.curve("SizeCurve"),
        ball_count_curve: p.curve("BallCountCurve"),
        edge_width_curve: p.curve("BallEdgeWidthCurve"),
        random_max_curve: p.curve("RandomMaxCurve"),
        balls: Vec::with_capacity(7),
        remove_balls: Vec::new(),
        next_id: 0,
        rand: Rand::default(),
        frame_colors: Vec::new(),
        threw: std::cell::Cell::new(false),
        ball_count: 0,
        min_buffer: 0,
        max_buffer: 0,
        interval_pos: 0.0,
        interval_pos_factor: 0.0,
        radius: 0,
        center_speed: 0.0,
        speed_variation: 0.0,
        level: 0.0,
        buffer_ht: 0,
        buffer_wi: 0,
        frame_time: 50,
    };
    let out = render_pixel_effect(ctx, p, &mut e);
    if e.threw.get() {
        return EffectIntents::new();
    }
    out
}

impl Balls {
    fn calculate_center_speed(&self, factor: f64) -> f64 {
        scale_curve_to_value(self.center_speed_curve.get_value(factor), self.max_buffer as f64 / 10.0, 0.0)
            * self.frame_time as f64
            / 50.0
    }

    fn calculate_speed_variation(&self, factor: f64) -> f64 {
        scale_curve_to_value(self.speed_variation_curve.get_value(factor), self.max_buffer as f64 / 10.0, 0.0)
            * self.frame_time as f64
            / 50.0
    }

    fn calculate_size(&self, factor: f64) -> i32 {
        let value = scale_curve_to_value(self.size_curve.get_value(factor), self.min_buffer as f64 / 2.0, 1.0) as i32;
        value.max(1)
    }

    fn calculate_edge_width(&self, factor: f64) -> i32 {
        let value = scale_curve_to_value(self.edge_width_curve.get_value(factor), 40.0, 1.0) as i32;
        value.max(1)
    }

    fn calculate_ball_count(&self, factor: f64) -> i32 {
        let value = scale_curve_to_value(self.ball_count_curve.get_value(factor), 100.0, 1.0) as i32;
        value.max(1)
    }

    fn calculate_random_max(&self, factor: f64) -> i32 {
        scale_curve_to_value(self.random_max_curve.get_value(factor), 40.0, 5.0) as i32
    }

    /// The per-frame head shared by `RenderEffect` and `RenderEffectByLocation`. Returns the inverse
    /// background colour.
    fn begin_frame(&mut self, frame: i32, info: &PixelInfo) -> Color {
        self.interval_pos = info.interval_position(frame) as f32;
        // float * int stays float in C#.
        self.interval_pos_factor = (self.interval_pos * 100.0f32) as f64;

        self.radius = self.calculate_size(self.interval_pos_factor);
        self.center_speed = self.calculate_center_speed(self.interval_pos_factor);
        self.speed_variation = self.calculate_speed_variation(self.interval_pos_factor);
        self.level = self.level_curve.get_value(self.interval_pos_factor) / 100.0;
        let max_random_time = self.calculate_random_max(self.interval_pos_factor);
        let inverse_back_color = self.background_color.get_color_at_f32(self.interval_pos);

        let min_speed = self.center_speed - (self.speed_variation / 2.0);
        let max_speed = self.center_speed + (self.speed_variation / 2.0);

        self.ball_count = self.calculate_ball_count(self.interval_pos_factor);

        self.frame_colors.clear();
        for g in &self.colors {
            self.frame_colors.push(g.get_color_at_f32(self.interval_pos));
        }

        self.create_balls(min_speed, max_speed, max_random_time);
        self.update_balls(min_speed, max_speed, max_random_time);
        self.remove_excess_balls();
        inverse_back_color
    }

    fn update_balls(&mut self, min_speed: f64, max_speed: f64, max_random_time: i32) {
        let factor = self.interval_pos_factor;
        // These only depend on the frame, not on the ball.
        let prev_speed = self.calculate_center_speed(factor - 1.0);
        let prev_variation = self.calculate_speed_variation(factor - 1.0);
        let prev_size = self.calculate_size(factor - 1.0);
        let cur_speed = self.calculate_center_speed(factor);
        let cur_variation = self.calculate_speed_variation(factor);
        let cur_size = self.calculate_size(factor);

        for i in 0..self.balls.len() {
            let ball = self.balls[i];
            if self.ball_count < self.balls.len() as i32 - self.remove_balls.len() as i32 {
                // Removes balls if ball count curve position is below the current number of balls. Will only
                // remove ones that hit the edge of the grid.
                let r = self.radius as f64;
                // The last test uses LocationY + Radius <= 1, as written upstream.
                if ball.location_x + r >= (self.buffer_wi - 1) as f64
                    || ball.location_y + r >= (self.buffer_ht - 1) as f64
                    || ball.location_x - r <= 1.0
                    || ball.location_y + r <= 1.0
                {
                    self.remove_balls.push(ball.id);
                }
            }

            // Move the ball.
            if self.random_movement && self.balls[i].move_count == 0 {
                let speed = self.rand.next_double() * (max_speed - min_speed) + min_speed;
                let mut vx = self.rand.next_double() * speed;
                let mut vy = self.rand.next_double() * speed;
                if self.rand.next_range(0, 2) == 0 {
                    vx = -vx;
                }
                if self.rand.next_range(0, 2) == 0 {
                    vy = -vy;
                }
                let mc = self.rand.next_range(5, max_random_time);
                let b = &mut self.balls[i];
                b.velocity_x = vx;
                b.velocity_y = vy;
                b.move_count = mc;
            } else {
                // Adjust ball speeds when user adjust Speed curve
                let b = &mut self.balls[i];
                if self.center_speed > prev_speed || self.center_speed < prev_speed {
                    let ratio = cur_speed / prev_speed;
                    b.velocity_x *= ratio;
                    b.velocity_y *= ratio;
                }
                if self.speed_variation > prev_variation || self.speed_variation < prev_variation {
                    let ratio = cur_variation / prev_variation;
                    b.velocity_x *= ratio;
                    b.velocity_y *= ratio;
                }
                if self.radius > prev_size || self.radius < prev_size {
                    let ratio = cur_size as f64 / prev_size as f64;
                    b.radius *= ratio;
                }
                b.move_count -= 1;
            }

            {
                let b = &mut self.balls[i];
                b.location_x += b.velocity_x;
                b.location_y += b.velocity_y;
            }

            // Collisions: reverse direction and nudge.
            if self.collide {
                for j in 0..self.balls.len() {
                    let (ball, ball1) = (self.balls[i], self.balls[j]);
                    let max_radius = ball.radius.max(ball1.radius);
                    if (ball1.location_x + max_radius >= ball.location_x
                        && ball1.location_x - max_radius < ball.location_x
                        && ball1.location_y + max_radius >= ball.location_y
                        && ball1.location_y - max_radius < ball.location_y)
                        && ball1.id != ball.id
                    {
                        {
                            let b = &mut self.balls[i];
                            b.velocity_x = -b.velocity_x;
                            b.velocity_y = -b.velocity_y;
                        }
                        {
                            let b1 = &mut self.balls[j];
                            b1.velocity_x = -b1.velocity_x;
                            b1.velocity_y = -b1.velocity_y;
                        }
                        {
                            let b = &mut self.balls[i];
                            b.location_y += b.velocity_y;
                            b.location_x += b.velocity_x;
                        }
                        {
                            let b1 = &mut self.balls[j];
                            b1.location_y += b1.velocity_y;
                            b1.location_x += b1.velocity_x;
                        }

                        // Changes the Ball color when it collides with another ball and will not change to the
                        // same color.
                        let n = self.colors.len();
                        if n > 1 && self.change_collide_color {
                            let mut color_index;
                            loop {
                                color_index = self.rand.next_range(0, n as i32) as usize;
                                if self.balls[i].color_index != color_index {
                                    break;
                                }
                            }
                            self.balls[i].color_index = color_index;
                            loop {
                                color_index = self.rand.next_range(0, n as i32) as usize;
                                if self.balls[j].color_index != color_index {
                                    break;
                                }
                            }
                            self.balls[j].color_index = color_index;
                        }
                        break;
                    }
                }
            }

            let (wi, ht) = (self.buffer_wi as f64, self.buffer_ht as f64);
            let b = &mut self.balls[i];
            if !self.wrap {
                // BallType.Bounce
                if b.location_x - b.radius < 0.0 {
                    b.location_x = b.radius;
                    b.velocity_x = -b.velocity_x;
                } else if b.location_x + b.radius >= wi {
                    b.location_x = wi - b.radius - 1.0;
                    b.velocity_x = -b.velocity_x;
                }
                if b.location_y - b.radius < 0.0 {
                    b.location_y = b.radius;
                    b.velocity_y = -b.velocity_y;
                } else if b.location_y + b.radius >= ht {
                    b.location_y = ht - b.radius - 1.0;
                    b.velocity_y = -b.velocity_y;
                }
            } else {
                // BallType.Wrap
                if b.location_x + b.radius < 0.0 {
                    b.location_x += wi + (b.radius * 2.0);
                }
                if b.location_y + b.radius < 0.0 {
                    b.location_y += ht + (b.radius * 2.0);
                }
                if b.location_x - b.radius > wi {
                    b.location_x -= wi + (b.radius * 2.0);
                }
                if b.location_y - b.radius > ht {
                    b.location_y -= ht + (b.radius * 2.0);
                }
            }
        }
    }

    fn create_balls(&mut self, min_speed: f64, max_speed: f64, max_random_time: i32) {
        while self.ball_count > self.balls.len() as i32 {
            // Sets Radius size and Ball location
            let radius = if self.random_radius { self.rand.next_range(1, self.radius + 1) } else { self.radius };
            let location_x = if self.buffer_wi - radius >= radius {
                self.rand.next_range(radius, self.buffer_wi - radius)
            } else {
                self.rand.next_range(1, self.buffer_wi)
            } as f64;
            let location_y = if self.buffer_ht - radius >= radius {
                self.rand.next_range(radius, self.buffer_ht - radius)
            } else {
                self.rand.next_range(1, self.buffer_ht)
            } as f64;

            if self.collide {
                // Will not add a ball if one exists in the same spot.
                for ball in &self.balls {
                    if (location_x >= ball.location_x && location_x - (radius as f64) < ball.location_x)
                        || (location_y >= ball.location_y && location_y - (radius as f64) < ball.location_y)
                    {
                        return;
                    }
                }
            }

            let speed = self.rand.next_double() * (max_speed - min_speed) + min_speed;
            let mut vx = self.rand.next_double() * speed;
            let mut vy = self.rand.next_double() * speed;
            if self.rand.next_range(0, 2) == 0 {
                vx = -vx;
            }
            if self.rand.next_range(0, 2) == 0 {
                vy = -vy;
            }
            let color_index = self.rand.next_range(0, self.colors.len() as i32).max(0) as usize;
            let move_count = self.rand.next_range(5, max_random_time);
            self.next_id += 1;
            self.balls.push(Ball {
                location_x,
                location_y,
                velocity_x: vx,
                velocity_y: vy,
                color_index,
                radius: radius as f64,
                id: self.next_id,
                move_count,
            });
        }
    }

    fn remove_excess_balls(&mut self) {
        if self.ball_count < self.balls.len() as i32 {
            for id in &self.remove_balls {
                if let Some(pos) = self.balls.iter().position(|b| b.id == *id) {
                    self.balls.remove(pos);
                }
            }
            self.remove_balls.clear();
        }
    }

    fn calculate_pixel<B: PixelBuffer>(&self, x_in: i32, y_in: i32, buf: &mut B, inverse_back_color: Color, info: &PixelInfo, edge_width: i32) {
        let (x_coord, y_coord) = (x_in, y_in);
        let (mut x, mut y) = (x_in, y_in);
        if self.locations {
            // Flip me over and offset my coordinates so I can act like the string version
            y = ((info.buffer_ht_offset() - y) + (self.buffer_ht - 1 + info.buffer_ht_offset())).abs();
            y -= info.buffer_ht_offset();
            x -= info.buffer_wi_offset();
        }
        let (xf, yf) = (x as f64, y as f64);

        for ball in &self.balls {
            // This saves going through all X and Y locations, significantly reducing render times.
            if yf <= ball.location_y + ball.radius + 1.0
                && yf >= ball.location_y - ball.radius
                && xf <= ball.location_x + ball.radius + 1.0
                && xf >= ball.location_x - ball.radius
            {
                let distance_from_ball_center = distance_from_point(
                    ball.location_x.round_ties_even() as i32,
                    ball.location_y.round_ties_even() as i32,
                    x,
                    y,
                );
                let distance =
                    if distance_from_ball_center > 1.4 && distance_from_ball_center < 1.51 { 2.0 } else { distance_from_ball_center };

                let base = self.frame_colors.get(ball.color_index).copied();
                match self.fill {
                    BallFill::Solid => {
                        if distance <= ball.radius {
                            match base {
                                Some(c) => self.put(buf, x_coord, y_coord, c, self.level),
                                None => self.threw.set(true),
                            }
                            return;
                        }
                    }
                    BallFill::Empty => {
                        if distance >= ball.radius - edge_width as f64 && distance < ball.radius {
                            match base {
                                Some(c) => self.put(buf, x_coord, y_coord, c, self.level),
                                None => self.threw.set(true),
                            }
                            return;
                        }
                    }
                    BallFill::Inverse => {
                        if distance <= ball.radius {
                            buf.set_pixel(x_coord, y_coord, Color::BLACK);
                            return;
                        }
                    }
                    BallFill::Fade => {
                        if distance <= ball.radius * 0.9 {
                            match base {
                                Some(c) => {
                                    let mut hsv = Hsv::from_color(c);
                                    hsv.set_v(hsv.v * ((1.0 - distance / ball.radius) * self.level));
                                    buf.set_pixel_hsv(x_coord, y_coord, hsv);
                                }
                                None => self.threw.set(true),
                            }
                            return;
                        }
                    }
                    BallFill::Gradient => {
                        if distance <= ball.radius {
                            match self.colors.get(ball.color_index) {
                                Some(g) => {
                                    let c = g.get_color_at(distance / ball.radius);
                                    self.put(buf, x_coord, y_coord, c, self.level);
                                }
                                None => self.threw.set(true),
                            }
                            return;
                        }
                    }
                }
            }
            // Add the background color when no other color has been used for the balls.
            if self.fill == BallFill::Inverse {
                buf.set_pixel(x_coord, y_coord, inverse_back_color);
            }
        }
    }

    /// The `if (_level < 1) { hsv.V *= _level ... } else { color }` pattern.
    #[inline]
    fn put<B: PixelBuffer>(&self, buf: &mut B, x: i32, y: i32, c: Color, level: f64) {
        if level < 1.0 {
            let mut hsv = Hsv::from_color(c);
            hsv.set_v(hsv.v * level);
            buf.set_pixel_hsv(x, y, hsv);
        } else {
            buf.set_pixel(x, y, c);
        }
    }
}

impl PixelEffect for Balls {
    fn setup_render(&mut self, info: &PixelInfo) {
        self.buffer_wi = info.buffer_wi();
        self.buffer_ht = info.buffer_ht();
        self.frame_time = info.frame_time;
        self.min_buffer = self.buffer_ht.min(self.buffer_wi);
        self.max_buffer = self.buffer_ht.max(self.buffer_wi);
        self.balls.clear();
        self.remove_balls.clear();
        self.next_id = 0;
        self.rand = Rand::default();
    }

    fn cleanup_render(&mut self) {
        self.balls.clear();
        self.remove_balls.clear();
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let inverse_back_color = self.begin_frame(frame, info);
        let edge_width = self.calculate_edge_width(self.interval_pos_factor);
        // Iterate through all grid locations.
        for y in 0..self.buffer_ht {
            for x in 0..self.buffer_wi {
                self.calculate_pixel(x, y, buf, inverse_back_color, info, edge_width);
            }
        }
    }

    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        let mut nodes = buf.element_locations().to_vec();
        nodes.sort_by_key(|l| (l.y, l.x));
        for frame in 0..num_frames {
            buf.current_frame = frame;
            let inverse_back_color = self.begin_frame(frame, info);
            let edge_width = self.calculate_edge_width(self.interval_pos_factor);
            for l in &nodes {
                self.calculate_pixel(l.x, l.y, buf, inverse_back_color, info, edge_width);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{frame, lit};
    use vixen_effects::testing;

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        testing::assert_produces_light(&DESCRIPTOR);
    }

    #[test]
    fn visibility_follows_fill() {
        let mut p = DESCRIPTOR.new_params();
        p.set("BallFill", ParamValue::Enum("Inverse".to_string()));
        let visible: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(visible.contains(&"BackgroundColor"));
        assert!(!visible.contains(&"Colors"));
        assert!(!visible.contains(&"BallEdgeWidthCurve"));
        p.set("BallFill", ParamValue::Enum("Empty".to_string()));
        let visible: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(visible.contains(&"BallEdgeWidthCurve"));
    }

    #[test]
    fn ball_count_and_solid_fill() {
        // One ball (count curve at 0 maps to 1), solid red, stationary: lit pixels are within the radius.
        let f = frame(&DESCRIPTOR, 20, 20, 1000, 3, |p| {
            p.set("BallFill", ParamValue::Enum("Solid".to_string()));
            p.set("BallCountCurve", ParamValue::Curve(Curve::flat(0.0)));
            p.set("CenterSpeedCurve", ParamValue::Curve(Curve::flat(0.0)));
            p.set("SpeedVariationCurve", ParamValue::Curve(Curve::flat(0.0)));
            p.set("SizeCurve", ParamValue::Curve(Curve::flat(0.0)));
            p.set("Colors", ParamValue::GradientList(vec![ColorGradient::solid(Color::RED)]));
        });
        // Size curve 0 gives radius 1: a handful of pixels, all pure red.
        let n = lit(&f);
        assert!((1..=9).contains(&n), "lit {n}");
        for c in f.iter().flatten().filter(|c| c.r > 0) {
            assert_eq!((c.g, c.b), (0, 0));
        }
    }
}
