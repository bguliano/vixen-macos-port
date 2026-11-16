use super::*;
use serde_json::{json, Value};
impl Editor {
    pub fn pointer_down(&mut self, p: Point) {
        if self.mode == Mode::Select {
            if let Some((_, _, center, handles)) = self.adorner() {
                if let Some(i) = handles.iter().position(|h| near(p, *h, 5.)) {
                    self.drag = Some(Drag::Handle {
                        handle: i as u32,
                        previous: p,
                        center,
                    });
                    return;
                }
            }
        }
        if p.x >= self.canvas_width || p.y >= self.canvas_height || p.x < 0. || p.y < 0. {
            return;
        }
        match self.mode {
            Mode::Polygon | Mode::Line => {
                if !self.can_add() {
                    return;
                }
                if self.mode == Mode::Polygon
                    && self.partial.len() >= 3
                    && near(p, self.partial[0], 10.)
                {
                    let points = std::mem::take(&mut self.partial);
                    self.complete_partial(Kind::Polygon, points);
                    self.ghost = None;
                } else {
                    self.partial.push(p);
                    self.ghost = Some(p);
                    if self.mode == Mode::Line && self.partial.len() == 2 {
                        let points = std::mem::take(&mut self.partial);
                        self.complete_partial(Kind::Line, points);
                        self.ghost = None;
                    }
                }
            }
            Mode::Ellipse => {
                let points = self.ellipse_points(p);
                self.add_shape(Shape::new(Kind::Ellipse, points, 0., self.time_mode()));
            }
            Mode::AddPoint => {
                let hit = self
                    .shapes
                    .iter()
                    .filter(|s| self.visible(s) && s.kind == Kind::Polygon)
                    .find_map(|s| {
                        (0..s.points.len())
                            .find(|i| {
                                distance(p, s.points[*i], s.points[(*i + 1) % s.points.len()]) <= 2.
                            })
                            .map(|i| (s.id, i))
                    });
                if let Some((id, i)) = hit {
                    let s = self.shapes.iter_mut().find(|s| s.id == id).unwrap();
                    s.points.insert(i + 1, p);
                    s.recalculate();
                    self.select_shape(Some(id));
                }
            }
            Mode::Select => {
                self.select_shape(None);
                // Upstream tests each kind's points before its centre, and ellipses have no corner hit target.
                let mut hit = None;
                for kind in [Kind::Polygon, Kind::Line, Kind::Ellipse] {
                    for s in self
                        .shapes
                        .iter()
                        .filter(|s| self.visible(s) && s.kind == kind)
                    {
                        if kind != Kind::Ellipse {
                            if let Some(i) = s.points.iter().position(|q| near(*q, p, 5.)) {
                                hit = Some((s.id, Some(i)));
                                break;
                            }
                        }
                    }
                    if hit.is_none() {
                        hit = self
                            .shapes
                            .iter()
                            .filter(|s| self.visible(s) && s.kind == kind)
                            .find(|s| near(s.center, p, 5.))
                            .map(|s| (s.id, None));
                    }
                    if hit.is_some() {
                        break;
                    }
                }
                if let Some((id, point)) = hit {
                    self.selected = Some(id);
                    if let Some(i) = point {
                        self.points = vec![(id, i)];
                        self.drag = Some(Drag::Point(id, i));
                        self.ghost = Some(p);
                    } else {
                        self.select_shape(Some(id));
                        self.drag = Some(Drag::Move(p));
                    }
                } else {
                    self.drag = Some(Drag::Band(p));
                    self.band = Some((p, p));
                }
            }
        }
    }
    pub fn pointer_move(&mut self, p: Point, dragging: bool, shift: bool) {
        let p = if matches!(self.drag, Some(Drag::Handle { .. } | Drag::Move(_))) {
            p
        } else {
            self.clamp(p)
        };
        if !dragging {
            if !self.partial.is_empty() {
                self.ghost = Some(p);
            }
            return;
        }
        match self.drag.clone() {
            Some(Drag::Point(..)) => self.ghost = Some(p),
            Some(Drag::Band(a)) => self.band = Some((a, p)),
            Some(Drag::Move(previous)) => {
                if self.translate(Point::new(p.x - previous.x, p.y - previous.y)) {
                    self.drag = Some(Drag::Move(p));
                }
            }
            Some(Drag::Handle {
                handle,
                previous,
                center,
            }) => {
                if handle == 8 {
                    if self.translate(Point::new(p.x - previous.x, p.y - previous.y)) {
                        self.drag = Some(Drag::Handle {
                            handle,
                            previous: p,
                            center,
                        });
                    }
                } else if handle == 9 {
                    let degrees = ((p.y - center.y).atan2(p.x - center.x)
                        - (previous.y - center.y).atan2(previous.x - center.x))
                    .to_degrees();
                    if self.rotate_selected(degrees, center) {
                        self.drag = Some(Drag::Handle {
                            handle,
                            previous: p,
                            center,
                        });
                    } else {
                        self.drag = None;
                        self.select_shape(None);
                    }
                } else {
                    self.resize(
                        handle,
                        Point::new(p.x - previous.x, p.y - previous.y),
                        shift,
                    );
                    self.drag = Some(Drag::Handle {
                        handle,
                        previous: p,
                        center,
                    });
                }
            }
            None => {}
        }
    }
    pub fn pointer_up(&mut self, p: Point, shift: bool) {
        self.pointer_move(p, true, shift);
        let p = self.clamp(p);
        match self.drag.take() {
            Some(Drag::Point(id, i)) => {
                if let Some(s) = self.shapes.iter_mut().find(|s| s.id == id) {
                    if let Some(point) = s.points.get_mut(i) {
                        *point = p;
                    }
                    s.recalculate();
                }
            }
            Some(Drag::Band(a)) => {
                let (lo, hi) = bounds(&[a, p]);
                self.points = self
                    .shapes
                    .iter()
                    .filter(|s| self.visible(s) && s.kind != Kind::Ellipse)
                    .flat_map(|s| {
                        s.points
                            .iter()
                            .enumerate()
                            .filter(move |(_, p)| {
                                p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y
                            })
                            .map(move |(i, _)| (s.id, i))
                    })
                    .collect();
                let ids: Vec<_> = self.points.iter().map(|(id, _)| *id).collect();
                self.selected = ids.first().copied().filter(|id| {
                    ids.iter().all(|other| other == id)
                        && (ids.len() == 1
                            || self
                                .shapes
                                .iter()
                                .find(|s| s.id == *id)
                                .is_some_and(|s| s.points.len() == ids.len()))
                });
            }
            _ => {}
        }
        self.ghost = None;
        self.band = None;
    }
    fn complete_partial(&mut self, kind: Kind, points: Vec<Point>) {
        let mut shape = Shape::new(kind, points, 0., self.time_mode());
        shape.label = self.partial_label.take().unwrap_or_default();
        self.add_shape(shape);
        // Free-form drawing immediately creates and selects the next draft.
        if !self.time_mode() {
            self.partial_label = Some(String::new());
        }
    }
    pub fn pointer_leave(&mut self) {
        if matches!(self.mode, Mode::Polygon | Mode::Line) {
            self.partial.clear();
            self.partial_label = Some(String::new());
            self.ghost = None;
        }
    }
    pub fn cursor(&self, p: Point) -> &'static str {
        if self.mode == Mode::Polygon && self.partial.len() >= 2 && near(p, self.partial[0], 10.) {
            return "cross";
        }
        if self.mode == Mode::Select {
            for s in self.shapes.iter().filter(|s| self.visible(s)) {
                if near(p, s.center, 5.) {
                    return "cross";
                }
                if s.kind != Kind::Ellipse && s.points.iter().any(|q| near(p, *q, 5.)) {
                    return "move";
                }
            }
        }
        if self.mode == Mode::AddPoint
            && self
                .shapes
                .iter()
                .filter(|s| self.visible(s) && s.kind == Kind::Polygon)
                .any(|s| {
                    (0..s.points.len())
                        .any(|i| distance(p, s.points[i], s.points[(i + 1) % s.points.len()]) <= 2.)
                })
        {
            return "add";
        }
        "arrow"
    }
    fn selection_points(&self) -> Vec<Point> {
        self.points
            .iter()
            .filter_map(|(id, i)| {
                self.shapes
                    .iter()
                    .find(|s| s.id == *id)
                    .and_then(|s| s.points.get(*i))
                    .copied()
            })
            .collect()
    }
    pub fn adorner(&self) -> Option<(Point, Point, Point, Vec<Point>)> {
        let mut points = self.selection_points();
        if points.len() < 2 || self.mode != Mode::Select {
            return None;
        }
        let ellipse = self.selected_shape().filter(|s| s.kind == Kind::Ellipse);
        let angle = ellipse.map_or(0., |s| s.angle);
        let center = ellipse.map_or_else(
            || {
                let (a, b) = bounds(&points);
                Point::new((a.x + b.x) / 2., (a.y + b.y) / 2.)
            },
            |s| s.center,
        );
        for p in &mut points {
            *p = rotate(*p, center, -angle);
        }
        let (a, b) = bounds(&points);
        let mid = Point::new((a.x + b.x) / 2., (a.y + b.y) / 2.);
        let handles = vec![
            a,
            Point::new(mid.x, a.y),
            Point::new(b.x, a.y),
            Point::new(b.x, mid.y),
            b,
            Point::new(mid.x, b.y),
            Point::new(a.x, b.y),
            Point::new(a.x, mid.y),
            mid,
            Point::new(mid.x, a.y - 30.),
        ]
        .into_iter()
        .map(|p| rotate(p, center, angle))
        .collect();
        Some((a, b, center, handles))
    }
    fn transform(&mut self, mut f: impl FnMut(Point) -> Point) -> bool {
        let transformed: Vec<_> = self.selection_points().into_iter().map(&mut f).collect();
        if transformed.iter().any(|p| {
            !p.x.is_finite()
                || !p.y.is_finite()
                || round10(p.x) < 0.
                || round10(p.y) < 0.
                || round10(p.x) > round10(self.canvas_width - 1.)
                || round10(p.y) > round10(self.canvas_height - 1.)
        }) {
            return false;
        }
        for ((id, i), p) in self.points.iter().zip(transformed) {
            if let Some(s) = self.shapes.iter_mut().find(|s| s.id == *id) {
                if let Some(point) = s.points.get_mut(*i) {
                    *point = Point::new(
                        p.x.clamp(0., self.canvas_width - 1.),
                        p.y.clamp(0., self.canvas_height - 1.),
                    );
                }
            }
        }
        for s in &mut self.shapes {
            s.recalculate();
        }
        true
    }
    pub fn translate(&mut self, d: Point) -> bool {
        self.transform(|p| Point::new(p.x + d.x, p.y + d.y))
    }
    pub fn rotate_selected(&mut self, angle: f64, center: Point) -> bool {
        if self.transform(|p| rotate(p, center, angle)) {
            if let Some(s) = self.selected_shape_mut() {
                if s.kind == Kind::Ellipse {
                    s.angle += angle;
                }
            }
            true
        } else {
            false
        }
    }
    fn resize(&mut self, handle: u32, delta: Point, shift: bool) -> bool {
        let Some((a, b, center, _)) = self.adorner() else {
            return false;
        };
        let angle = self
            .selected_shape()
            .filter(|s| s.kind == Kind::Ellipse)
            .map_or(0., |s| s.angle);
        let delta = rotate(delta, Point::default(), -angle);
        let w = if b.x == a.x { 1. } else { b.x - a.x };
        let h = if b.y == a.y { 1. } else { b.y - a.y };
        let left = matches!(handle, 0 | 6 | 7);
        let top = matches!(handle, 0 | 1 | 2);
        let horizontal = !matches!(handle, 1 | 5);
        let vertical = !matches!(handle, 3 | 7);
        let mut sx = if horizontal {
            1. + delta.x * if left { -1. } else { 1. } / w
        } else {
            1.
        };
        let mut sy = if vertical {
            1. + delta.y * if top { -1. } else { 1. } / h
        } else {
            1.
        };
        if shift && horizontal && vertical {
            sx = sx.max(sy);
            sy = sx;
        }
        let anchor = Point::new(if left { b.x } else { a.x }, if top { b.y } else { a.y });
        self.transform(|p| {
            let p = rotate(p, center, -angle);
            rotate(
                Point::new(
                    anchor.x + (p.x - anchor.x) * sx,
                    anchor.y + (p.y - anchor.y) * sy,
                ),
                center,
                angle,
            )
        })
    }
    pub fn copy(&self) -> Option<String> {
        if self.morph_mode == MorphMode::Pattern {
            return None;
        }
        let s = self.selected_shape()?;
        if s.kind == Kind::Polygon
            && (self.mode != Mode::Select || self.points.len() != s.points.len())
        {
            return None;
        }
        Some(json!({"format":"vixen-polygon-1","kind":s.kind.name(),"label":s.label,"fill":s.fill,"points":s.points.iter().map(|p|[p.x,p.y]).collect::<Vec<_>>(),"angle":s.angle,"start":s.start_side}).to_string())
    }
    pub fn paste(&mut self, json: &str) -> bool {
        if !self.can_add() {
            return false;
        }
        let Ok(v) = serde_json::from_str::<Value>(json) else {
            return false;
        };
        if v["format"] != "vixen-polygon-1" {
            return false;
        }
        let kind = match v["kind"].as_str() {
            Some("Polygon") => Kind::Polygon,
            Some("Line") => Kind::Line,
            Some("Ellipse") => Kind::Ellipse,
            _ => return false,
        };
        let Some(raw) = v["points"].as_array() else {
            return false;
        };
        if raw.len() > 1_000_000 {
            return false;
        }
        let points: Option<Vec<_>> = raw
            .iter()
            .map(|p| Some(Point::new(p[0].as_f64()?, p[1].as_f64()?)))
            .collect();
        let Some(points) = points else {
            return false;
        };
        if match kind {
            Kind::Polygon => points.len() < 3,
            Kind::Line => points.len() != 2,
            Kind::Ellipse => points.len() != 4,
        } {
            return false;
        }
        let mut s = Shape::new(kind, points, 0., false);
        s.start_lime = false;
        s.label = v["label"].as_str().unwrap_or("").into();
        s.fill = match v["fill"].as_str() {
            Some("Solid") => "Solid",
            Some("Outline") => "Outline",
            _ => "Wipe",
        }
        .into();
        s.angle = v["angle"].as_f64().unwrap_or(0.);
        s.start_side = v["start"].as_i64().unwrap_or(0) as i32;
        let d = Point::new(
            self.canvas_width / 2. - s.center.x,
            self.canvas_height / 2. - s.center.y,
        );
        for p in &mut s.points {
            *p = self.clamp(Point::new(p.x + d.x, p.y + d.y));
        }
        s.recalculate();
        let id = s.id;
        self.add_shape(s);
        self.set_mode(Mode::Select);
        self.select_shape(Some(id));
        true
    }
}
fn near(a: Point, b: Point, t: f64) -> bool {
    (a.x - b.x).abs() <= t && (a.y - b.y).abs() <= t
}
fn round10(n: f64) -> f64 {
    (n * 1e10).round_ties_even() / 1e10
}
fn distance(p: Point, a: Point, b: Point) -> f64 {
    let (x, y) = (b.x - a.x, b.y - a.y);
    let den = x * x + y * y;
    if den == 0. {
        return (p.x - a.x).hypot(p.y - a.y);
    }
    let t = (((p.x - a.x) * x + (p.y - a.y) * y) / den).clamp(0., 1.);
    (p.x - a.x - t * x).hypot(p.y - a.y - t * y)
}
