//! The wipe: `SetupRenderPattern`'s arithmetic and `DrawWipe`'s drawing. A wipe runs along the two sides of a
//! quadrilateral, drawing a line across between them for each step of the longer side.

use super::polygon::{Ellipse, PolygonPoint, Shape};
use super::{gdi, Thrown};
use gdi::EllipseDraw;
use vixen_effects::prelude::*;

/// `MorphWipePolygonRenderData`.
#[derive(Debug, Default)]
pub(super) struct WipeRenderData {
    pub(super) x1_points: Vec<i32>,
    pub(super) y1_points: Vec<i32>,
    pub(super) x2_points: Vec<i32>,
    pub(super) y2_points: Vec<i32>,
    pub(super) direction: bool,
    pub(super) head_is_done: Vec<bool>,
    pub(super) head_acceleration: f64,
    pub(super) tail_acceleration: f64,
    pub(super) head_velocity_zero: f64,
    pub(super) tail_velocity_zero: f64,
    pub(super) stagger: f64,
    /// The C# redraws an ellipse's mask for every repeat on every frame. It never changes, so it is drawn once.
    pub(super) masks: Vec<Option<Vec<bool>>>,
}

impl WipeRenderData {
    /// `GetLengthOfWipePolygon`: the longer of the two sides.
    pub(super) fn length(&self) -> i32 {
        self.x1_points.len().max(self.x2_points.len()) as i32
    }
}

/// The four corners a wipe runs between, from `SetupRenderPattern`: a triangle repeats its last point, an ellipse
/// uses the rectangle around it, and a line becomes a skinny polygon.
pub(super) fn wipe_polygon_model(shape: &Shape) -> Result<[PolygonPoint; 4], Thrown> {
    match shape {
        Shape::Polygon(p) if p.len() == 3 => Ok([p[0], p[1], p[2], p[2]]),
        Shape::Polygon(p) if p.len() >= 4 => Ok([p[0], p[1], p[2], p[3]]),
        Shape::Ellipse(e) if e.points.len() >= 4 => Ok([e.points[0], e.points[1], e.points[2], e.points[3]]),
        Shape::Line(p) if p.len() >= 2 => Ok([p[0], p[0], p[1], p[1]]),
        _ => Err(Thrown),
    }
}

/// `CalculateDirection`.
pub(super) fn calculate_direction(model: &[PolygonPoint; 4], buffer_ht: i32) -> bool {
    let x1a = model[1].x as i32;
    let y1a = buffer_ht - model[1].y as i32;
    let x1b = model[0].x as i32;
    let y1b = buffer_ht - model[0].y as i32;
    let x2a = model[2].x as i32;
    let y2a = buffer_ht - model[2].y as i32;
    let x2b = model[3].x as i32;
    let y2b = buffer_ht - model[3].y as i32;
    (x2a - x1a) + (x2b - x1b) + (y2a - y1a) + (y2b - y1b) >= 0
}

/// `StoreLine`.
pub(super) fn store_line(x0_: i32, y0_: i32, x1_: i32, y1_: i32, vx: &mut Vec<i32>, vy: &mut Vec<i32>) {
    let (mut x0, mut y0) = (x0_, y0_);
    let (x1, y1) = (x1_, y1_);
    let (dx, sx) = ((x1 - x0).abs(), if x0 < x1 { 1 } else { -1 });
    let (dy, sy) = ((y1 - y0).abs(), if y0 < y1 { 1 } else { -1 });
    let mut err = (if dx > dy { dx } else { -dy }) / 2;
    loop {
        vx.push(x0);
        vy.push(y0);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = err;
        if e2 > -dx {
            err -= dy;
            x0 += sx;
        }
        if e2 < dy {
            err += dx;
            y0 += sy;
        }
    }
}

/// `GetIncreasingVelocityZero`.
#[inline]
pub(super) fn increasing_velocity_zero(acceleration: f64, length: f64, time: f64) -> f64 {
    let (t, i) = (time, acceleration);
    (length - 1.0 / 6.0 * i * t * t * t) / t
}

/// `GetIncreasingHeadPosition`.
#[inline]
pub(super) fn increasing_head_position(acceleration: f64, time: f64, velocity_zero: f64) -> f64 {
    let (t, i, v) = (time, acceleration, velocity_zero);
    1.0 / 6.0 * (i * t * t * t) + (v * t)
}

/// `GetPositiveAcceleration`: the largest acceleration that makes it to the end of the shape from a standstill.
#[inline]
pub(super) fn positive_acceleration(length: i32, time: f64) -> f64 {
    length as f64 / (time * time * time) * 6.0
}

/// `IsIncreasingAccelerationValid`: the wipe starts moving forwards and never backs up.
pub(super) fn is_increasing_acceleration_valid(acceleration: f64, length: f64, time: f64, frame_time: i32) -> bool {
    let (t, i) = (time, acceleration);
    let v = (length - 1.0 / 6.0 * i * t * t * t) / t;
    let step = frame_time.max(1) as f64;
    let mut valid = true;
    let mut x_previous = 0.0;
    let mut ti = 0.0;
    while ti < t {
        let x = increasing_head_position(i, ti, v);
        if x < x_previous {
            valid = false;
            break;
        }
        x_previous = x;
        ti += step;
    }
    v > 0.0 && valid
}

/// `GetNegativeAcceleration`. Halving ends in a valid value for any shape with a length and a positive time. For
/// anything else, which only a damaged file can produce, the C# loops forever, and this reports it as a failure.
pub(super) fn negative_acceleration(length: i32, time: f64, acceleration: i32, frame_time: i32) -> Result<f64, Thrown> {
    let mut tail_acceleration = acceleration as f64;
    let mut previous_acceleration = 0.0;
    while !is_increasing_acceleration_valid(tail_acceleration, length as f64, time, frame_time) {
        if tail_acceleration == 0.0 {
            return Err(Thrown);
        }
        previous_acceleration = tail_acceleration;
        tail_acceleration /= 2.0;
    }
    Ok(refine_acceleration(previous_acceleration, tail_acceleration, length, time, frame_time))
}

/// `RefineAcceleration`: a binary search between an invalid and a valid acceleration.
pub(super) fn refine_acceleration(mut invalid: f64, mut valid: f64, length: i32, time: f64, frame_time: i32) -> f64 {
    const MAX_ITERATIONS: i32 = 10000;
    let mut iterations = 0;
    while invalid != valid && iterations < MAX_ITERATIONS {
        let test = (invalid + valid) / 2.0;
        let unchanged = if is_increasing_acceleration_valid(test, length as f64, time, frame_time) {
            let unchanged = valid == test;
            valid = test;
            unchanged
        } else {
            let unchanged = invalid == test;
            invalid = test;
            unchanged
        };
        // Neighbouring doubles can have a midpoint equal to one of them. Then every remaining iteration repeats this
        // one and the C# spends them all. The answer is the same, so stop.
        if unchanged {
            break;
        }
        iterations += 1;
    }
    valid
}

/// The mask `RenderEffectWipes` renders for an ellipse: a filled ellipse flipped into frame buffer coordinates,
/// stored as `[x][y]`.
pub(super) fn ellipse_mask(ellipse: &Ellipse, center: PolygonPoint, wi: i32, ht: i32) -> Vec<bool> {
    let mut mask = vec![false; wi.max(0) as usize * ht.max(0) as usize];
    let (width, height, angle) = (ellipse.width, ellipse.height, ellipse.angle);
    let draw = EllipseDraw { center_x: center.x, center_y: center.y, width, height, angle };
    draw.fill(wi, ht, &mut |x, y| {
        let flipped = ht - y - 1;
        if x >= 0 && x < wi && flipped >= 0 && flipped < ht {
            mask[x as usize * ht as usize + flipped as usize] = true;
        }
    });
    mask
}

/// One side of a wipe's quadrilateral.
#[derive(Copy, Clone)]
pub(super) struct Side<'a> {
    x: &'a [i32],
    y: &'a [i32],
}

impl<'a> Side<'a> {
    pub(super) fn new(x: &'a [i32], y: &'a [i32]) -> Side<'a> {
        Side { x, y }
    }

    /// `List<int>` indexing, which throws out of range.
    #[inline]
    pub(super) fn at(&self, index: i32) -> Result<(i32, i32), Thrown> {
        if index < 0 || index as usize >= self.x.len() {
            return Err(Thrown);
        }
        Ok((self.x[index as usize], self.y[index as usize]))
    }
}

/// What the wipe's lines are drawn on.
pub(super) struct WipeTarget<'a> {
    pub(super) buf: &'a mut PixelFrameBuffer,
    pub(super) mask: Option<&'a Vec<bool>>,
    pub(super) wi: i32,
    pub(super) ht: i32,
    /// How far the repeating pattern has moved this pass. The C# copies the point lists to apply it.
    pub(super) offset_x: i32,
    pub(super) offset_y: i32,
    pub(super) direction: bool,
}

impl WipeTarget<'_> {
    /// `maskFrameBuffer.GetColorAt(x, y)` compared with the empty colour, or true when there is no mask. The frame
    /// buffer's jagged array throws out of range, and an ellipse wipe near the edge can get there.
    #[inline]
    pub(super) fn in_mask(&self, x: i32, y: i32) -> Result<bool, Thrown> {
        match self.mask {
            None => Ok(true),
            Some(mask) => {
                if x < 0 || y < 0 || x >= self.wi || y >= self.ht {
                    return Err(Thrown);
                }
                Ok(mask[x as usize * self.ht as usize + y as usize])
            }
        }
    }
}

/// `DrawThickLine(int, int, int, int, ...)`: a line with the gaps at its diagonal steps filled in.
pub(super) fn draw_thick_line(t: &mut WipeTarget, x0_: i32, y0_: i32, x1_: i32, y1_: i32, color: Color) -> Result<(), Thrown> {
    let (mut x0, mut y0) = (x0_, y0_);
    let (x1, y1) = (x1_, y1_);
    let (mut lastx, mut lasty) = (x0, y0);
    let (dx, sx) = ((x1 - x0).abs(), if x0 < x1 { 1 } else { -1 });
    let (dy, sy) = ((y1 - y0).abs(), if y0 < y1 { 1 } else { -1 });
    let mut err = (if dx > dy { dx } else { -dy }) / 2;
    let masked = t.mask.is_some();

    loop {
        // With a mask, off the bottom or the left of the buffer is simply not drawn.
        if !masked || (y0 >= 0 && x0 >= 0 && t.in_mask(x0, y0)?) {
            t.buf.set_pixel(x0, y0, color);
        }

        if x0 != lastx && y0 != lasty && x0_ != x1_ && y0_ != y1_ {
            let mut fix = 0;
            if x0 > lastx {
                fix += 1;
            }
            if y0 > lasty {
                fix += 2;
            }
            if t.direction {
                fix += 4;
            }
            let (near_x, near_y, room) = match fix {
                2 | 4 => (x0 + 1, y0, x0 < t.wi - 2),
                3 | 5 => (x0 - 1, y0, x0 > 0),
                0 | 1 => (x0, y0 + 1, y0 < t.ht - 2),
                _ => (x0, y0 - 1, y0 > 0),
            };
            if room && t.in_mask(near_x, near_y)? {
                t.buf.set_pixel(near_x, near_y, color);
            }
        }
        lastx = x0;
        lasty = y0;
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = err;
        if e2 > -dx {
            err -= dy;
            x0 += sx;
        }
        if e2 < dy {
            err += dx;
            y0 += sy;
        }
    }
    Ok(())
}

/// Walks `DrawThickLine(double longIndex, ...)` from `from` towards `to` in the C#'s steps of 0.1. At 0.1 per step
/// most steps land on the same two points as the step before, and redrawing them changes nothing, so only the ones
/// that moved are drawn. `color` gets the integer long index.
pub(super) fn sweep(
    t: &mut WipeTarget,
    from: f64,
    to: i32,
    short: Side,
    long: Side,
    length: i32,
    mut color: impl FnMut(i32) -> Color,
) -> Result<(), Thrown> {
    let mut x = from;
    let mut last = None;
    while x < to as f64 {
        let pct = x / length as f64;
        let short_index = (short.x.len() as f64 * pct) as i32;
        let long_index = x as i32;
        if last != Some((short_index, long_index)) {
            let a = short.at(short_index)?;
            let b = long.at(long_index)?;
            let c = color(long_index);
            let (ox, oy) = (t.offset_x, t.offset_y);
            draw_thick_line(t, a.0 + ox, a.1 + oy, b.0 + ox, b.1 + oy, c)?;
            last = Some((short_index, long_index));
        }
        x += 0.1;
    }
    Ok(())
}

/// A wipe's two colours for this frame, with their brightness applied.
#[derive(Copy, Clone)]
pub(super) struct WipeColors {
    pub(super) head: Color,
    pub(super) tail: Color,
}

/// `DrawWipe`.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_wipe(
    t: &mut WipeTarget,
    interval_head_pos: f64,
    tail_position: i32,
    short: Side,
    long: Side,
    length: i32,
    colors: WipeColors,
    head_length: i32,
) -> Result<(), Thrown> {
    let mut end_of_head = (length as f64 * interval_head_pos).round_ties_even() as i32;
    let mut start_of_head = end_of_head.wrapping_sub(head_length);

    // Don't allow the head to start off the display element, so that the tail is calculated properly.
    if start_of_head < 0 {
        start_of_head = 0;
    }
    if start_of_head > length - 1 {
        start_of_head = length;
    }

    // The tail.
    let tail_hsv = Hsv::from_color(colors.tail);
    sweep(t, tail_position as f64, start_of_head, short, long, length, |index| {
        let mut hsv = tail_hsv;
        hsv.set_v(hsv.v * tail_intensity(index, tail_position, start_of_head));
        hsv.to_color()
    })?;

    // The head goes last, so that it wins where the two overlap.
    if start_of_head < length {
        if end_of_head > length - 1 {
            end_of_head = length - 1;
        }
        sweep(t, start_of_head as f64, end_of_head, short, long, length, |_| colors.head)?;
    }
    Ok(())
}

/// `GetTailIntensity`: the half of the tail nearest the head is at full intensity and the rest fades to nothing.
#[inline]
pub(super) fn tail_intensity(x: i32, tail_start: i32, tail_end: i32) -> f64 {
    let intensity = (x - tail_start) as f64 + 1.0;
    let intensity = intensity / tail_end.wrapping_sub(tail_start) as f64;
    if intensity > 0.5 {
        1.0
    } else {
        intensity / 0.5
    }
}
