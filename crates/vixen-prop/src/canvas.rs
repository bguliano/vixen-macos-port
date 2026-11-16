//! The drawing canvas of a `PropEditor`: what the pointer does in draw and selection mode, the resize and rotate
//! adorner, and the toolbar's align, distribute, flip and light size commands.
//!
//! Ported from `CustomPropEditor/Controls/PropDesigner.cs`, `Adorners/ResizeAdorner.cs` and `DrawingPanelViewModel.cs`.

use std::collections::HashSet;

use uuid::Uuid;

use crate::editor::{CanvasMode, PanelTab, PropEditor};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    /// Control upstream.
    pub command: bool,
    pub shift: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LightDot {
    pub id: Uuid,
    pub owner: Uuid,
    pub x: f64,
    pub y: f64,
    pub size: f64,
    pub selected: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AdornerHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    Center,
    Rotate,
}

impl AdornerHandle {
    pub const ALL: [AdornerHandle; 10] = [
        AdornerHandle::Rotate,
        AdornerHandle::TopLeft,
        AdornerHandle::TopRight,
        AdornerHandle::BottomRight,
        AdornerHandle::BottomLeft,
        AdornerHandle::Top,
        AdornerHandle::Right,
        AdornerHandle::Bottom,
        AdornerHandle::Left,
        AdornerHandle::Center,
    ];
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SizeChange {
    Increase,
    Decrease,
    Match,
}

pub const HANDLE_SIZE: f64 = 10.0;

/// The box around the selected lights, in the frame it had before any rotation. `angle` turns the drawn handles
/// about the rotation centre.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Adorner {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    cx: f64,
    cy: f64,
    angle: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdornerGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub center_x: f64,
    pub center_y: f64,
    pub angle: f64,
    /// Each handle and the top-left of its square, before rotation.
    pub handles: Vec<(AdornerHandle, f64, f64)>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Drag {
    Lights { last: (f64, f64), started: bool },
    Band { start: (f64, f64), end: (f64, f64) },
    Handle { handle: AdornerHandle, last: (f64, f64), started: bool },
}

fn rotate(p: (f64, f64), c: (f64, f64), degrees: f64) -> (f64, f64) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (dx, dy) = (p.0 - c.0, p.1 - c.1);
    (c.0 + dx * cos - dy * sin, c.1 + dx * sin + dy * cos)
}

impl Adorner {
    fn handle_origin(&self, handle: AdornerHandle) -> (f64, f64) {
        let (x, y, w, h, s) = (self.x, self.y, self.width, self.height, HANDLE_SIZE);
        match handle {
            AdornerHandle::TopLeft => (x - s, y - s),
            AdornerHandle::TopRight => (x + w, y - s),
            AdornerHandle::BottomLeft => (x - s, y + h),
            AdornerHandle::BottomRight => (x + w, y + h),
            AdornerHandle::Top => (x + w / 2.0 - s / 2.0, y - s),
            AdornerHandle::Right => (x + w, y + h / 2.0 - s / 2.0),
            AdornerHandle::Bottom => (x + w / 2.0 - s / 2.0, y + h),
            AdornerHandle::Left => (x - s, y + h / 2.0 - s / 2.0),
            AdornerHandle::Center => (x + w / 2.0 - s / 2.0, y + h / 2.0 - s / 2.0),
            AdornerHandle::Rotate => (self.cx - s / 2.0, y - 5.0 * s),
        }
    }

    fn hit(&self, p: (f64, f64)) -> Option<AdornerHandle> {
        let (px, py) = rotate(p, (self.cx, self.cy), -self.angle);
        AdornerHandle::ALL.into_iter().find(|handle| {
            let (hx, hy) = self.handle_origin(*handle);
            px >= hx && px <= hx + HANDLE_SIZE && py >= hy && py <= hy + HANDLE_SIZE
        })
    }

    /// `EnforceSize`. It scales the box's origin along with its size, which is not a real fit test, but it is the
    /// one upstream applies and it decides how far a handle can be pulled.
    fn allows(&self, sx: f64, sy: f64, canvas: (f64, f64)) -> bool {
        let (mut x, mut y, mut w, mut h) = (self.x * sx, self.y * sy, self.width * sx, self.height * sy);
        if sx < 0.0 {
            x += w;
            w = -w;
        }
        if sy < 0.0 {
            y += h;
            h = -h;
        }
        x >= 0.0 && y >= 0.0 && w <= x + canvas.0 && y + h <= canvas.1
    }

    fn scale_bounds(&mut self, sx: f64, sy: f64, origin: (f64, f64)) {
        let (x0, x1) = (origin.0 + (self.x - origin.0) * sx, origin.0 + (self.x + self.width - origin.0) * sx);
        let (y0, y1) = (origin.1 + (self.y - origin.1) * sy, origin.1 + (self.y + self.height - origin.1) * sy);
        self.x = x0.min(x1);
        self.width = (x1 - x0).abs();
        self.y = y0.min(y1);
        self.height = (y1 - y0).abs();
    }
}

/// `GetAngle`, then the 45 degree detents the Control key gives.
fn rotation_angle(center: (f64, f64), mouse: (f64, f64), snap: bool) -> f64 {
    let angle = ((center.1 - mouse.1).atan2(center.0 - mouse.0).to_degrees() + 270.0) % 360.0;
    if !snap {
        return angle;
    }
    if !(22.5..337.5).contains(&angle) {
        0.0
    } else {
        ((angle - 22.5) / 45.0).floor() * 45.0 + 45.0
    }
}

impl PropEditor {
    pub fn mode(&self) -> CanvasMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: CanvasMode) {
        self.mode = mode;
        self.drag = None;
        self.rebuild_adorner();
    }

    /// Lights in drawing order: element by element as the tree lists them, each element once.
    pub fn light_dots(&self) -> Vec<LightDot> {
        let selected: HashSet<Uuid> = self.light_selection.iter().copied().collect();
        let mut dots = Vec::new();
        for leaf in self.prop.distinct_leaves(self.prop.root_id()) {
            let Some(e) = self.prop.element(leaf) else { continue };
            for light in e.lights() {
                dots.push(LightDot { id: light.id, owner: leaf, x: light.x, y: light.y, size: light.size, selected: selected.contains(&light.id) });
            }
        }
        dots
    }

    fn hit_light(&self, x: f64, y: f64) -> Option<Uuid> {
        self.light_dots().iter().rev().find(|d| (d.x - x).hypot(d.y - y) <= d.size).map(|d| d.id)
    }

    pub fn pointer_moved(&mut self, x: f64, y: f64) {
        self.pointer = Some((x, y));
    }

    pub fn pointer_exited(&mut self) {
        self.pointer = None;
    }

    /// The status bar's "{X:0}, {Y:0}", empty while the pointer is off the canvas.
    pub fn pointer_text(&self) -> String {
        match self.pointer {
            Some((x, y)) => format!("{:.0}, {:.0}", x.round(), y.round()),
            None => String::new(),
        }
    }

    pub fn handle_at(&self, x: f64, y: f64) -> Option<AdornerHandle> {
        self.adorner.as_ref().and_then(|a| a.hit((x, y)))
    }

    pub fn pointer_down(&mut self, x: f64, y: f64, modifiers: Modifiers) {
        self.pointer = Some((x, y));
        self.drag = None;
        self.state_drag = None;
        if self.tab == PanelTab::State {
            if self.state_selected_items().len() > 1 { return; }
            if self.mode == CanvasMode::Draw {
                if self.hit_light(x, y).is_none() {
                    let before: HashSet<_> = self.light_dots().iter().map(|d| d.id).collect();
                    self.add_light_at(x, y);
                    let owners: Vec<_> = self.light_dots().iter().filter(|d| !before.contains(&d.id)).map(|d| d.owner).collect();
                    self.assign_state_elements_impl(&owners, false, false, false);
                }
            } else if let Some(id) = self.hit_light(x, y) {
                let owner = self.prop.light_owners().get(&id).copied();
                if let Some(owner) = owner { self.assign_state_elements(&[owner], false, true); }
            } else {
                self.state_drag = Some(((x, y), (x, y), modifiers.command));
            }
            return;
        }
        if self.mode == CanvasMode::Draw {
            // A click on a light in draw mode does nothing.
            if self.hit_light(x, y).is_none() {
                self.add_light_at(x, y);
            }
            return;
        }
        if let Some(handle) = self.handle_at(x, y) {
            self.drag = Some(Drag::Handle { handle, last: (x, y), started: false });
            return;
        }
        match self.hit_light(x, y) {
            Some(light) => {
                let selected = self.light_selection.contains(&light);
                if modifiers.command {
                    if selected {
                        self.canvas_deselect(&[light]);
                    } else {
                        self.canvas_select(&[light]);
                    }
                } else if !selected {
                    self.canvas_clear();
                    self.canvas_select(&[light]);
                }
                self.drag = Some(Drag::Lights { last: (x, y), started: false });
            }
            None => {
                if !modifiers.command {
                    self.canvas_clear();
                }
                self.drag = Some(Drag::Band { start: (x, y), end: (x, y) });
            }
        }
        self.rebuild_adorner();
    }

    pub fn pointer_dragged(&mut self, x: f64, y: f64, modifiers: Modifiers) {
        self.pointer = Some((x, y));
        if self.tab == PanelTab::State {
            if let Some((start, _, _)) = self.state_drag { self.state_drag = Some((start, (x, y), modifiers.command)); }
            return;
        }
        match self.drag.clone() {
            None => {}
            Some(Drag::Lights { last, started }) => {
                let (dx, dy) = (x - last.0, y - last.1);
                if dx == 0.0 && dy == 0.0 || self.light_selection.is_empty() {
                    return;
                }
                if !started {
                    self.begin("Move Lights");
                }
                self.transform_selected(|p| (p.0 + dx, p.1 + dy));
                self.drag = Some(Drag::Lights { last: (x, y), started: true });
                self.rebuild_adorner();
            }
            Some(Drag::Band { start, .. }) => {
                self.drag = Some(Drag::Band { start, end: (x, y) });
                self.update_band_selection(start, (x, y), modifiers.command);
            }
            Some(Drag::Handle { handle, last, started }) => {
                let changed = self.drag_handle(handle, last, (x, y), modifiers, started);
                self.drag = Some(Drag::Handle { handle, last: (x, y), started: started || changed });
            }
        }
    }

    pub fn pointer_up(&mut self) {
        if let Some((start, end, remove)) = self.state_drag.take() {
            if start != end {
                let (left, right, top, bottom) = (start.0.min(end.0), start.0.max(end.0), start.1.min(end.1), start.1.max(end.1));
                let owners: Vec<_> = self.light_dots().iter().filter(|d| d.x - d.size >= left && d.x + d.size <= right && d.y - d.size >= top && d.y + d.size <= bottom).map(|d| d.owner).collect();
                self.assign_state_elements(&owners, remove, false);
            }
        }
        let was_handle = matches!(self.drag, Some(Drag::Handle { .. }));
        self.drag = None;
        // A handle drag leaves the adorner as it is, turned; anything else builds it afresh.
        if !was_handle {
            self.rebuild_adorner();
        }
    }

    /// The rubber band while one is being dragged: x, y, width, height.
    pub fn band(&self) -> Option<(f64, f64, f64, f64)> {
        if let Some((start, end, _)) = self.state_drag {
            return Some((start.0.min(end.0), start.1.min(end.1), (end.0 - start.0).abs(), (end.1 - start.1).abs()));
        }
        match self.drag {
            Some(Drag::Band { start, end }) if start != end => {
                Some((start.0.min(end.0), start.1.min(end.1), (end.0 - start.0).abs(), (end.1 - start.1).abs()))
            }
            _ => None,
        }
    }

    pub fn adorner(&self) -> Option<AdornerGeometry> {
        let a = self.adorner.as_ref()?;
        Some(AdornerGeometry {
            x: a.x,
            y: a.y,
            width: a.width,
            height: a.height,
            center_x: a.cx,
            center_y: a.cy,
            angle: a.angle,
            handles: AdornerHandle::ALL.into_iter().map(|h| (h, a.handle_origin(h).0, a.handle_origin(h).1)).collect(),
        })
    }

    /// `UpdateResizeAdorner`: selection mode only, not on the State tab, more than one light, and not mid rubber band.
    pub(crate) fn rebuild_adorner(&mut self) {
        self.adorner = None;
        if self.mode != CanvasMode::Select || self.tab == PanelTab::State || matches!(self.drag, Some(Drag::Band { .. })) {
            return;
        }
        if self.light_selection.len() < 2 {
            return;
        }
        let selected: HashSet<Uuid> = self.light_selection.iter().copied().collect();
        let mut bounds: Option<(f64, f64, f64, f64)> = None;
        for (_, light) in self.prop.lights().filter(|(_, l)| selected.contains(&l.id)) {
            let (l, t, r, b) = (light.x - light.size, light.y - light.size, light.x + light.size, light.y + light.size);
            bounds = Some(match bounds {
                None => (l, t, r, b),
                Some((bl, bt, br, bb)) => (bl.min(l), bt.min(t), br.max(r), bb.max(b)),
            });
        }
        if let Some((l, t, r, b)) = bounds {
            self.adorner = Some(Adorner { x: l, y: t, width: r - l, height: b - t, cx: (l + r) / 2.0, cy: (t + b) / 2.0, angle: 0.0 });
        }
    }

    fn transform_selected(&mut self, f: impl Fn((f64, f64)) -> (f64, f64)) {
        let selected: HashSet<Uuid> = self.light_selection.iter().copied().collect();
        for light in self.prop.lights_mut().filter(|l| selected.contains(&l.id)) {
            let (x, y) = f((light.x, light.y));
            light.x = x;
            light.y = y;
        }
    }

    /// `UpdateSelection`: a light is in when its whole circle is inside the band; the rest drop out unless Command is held.
    fn update_band_selection(&mut self, start: (f64, f64), end: (f64, f64), keep_others: bool) {
        let (left, right) = (start.0.min(end.0), start.0.max(end.0));
        let (top, bottom) = (start.1.min(end.1), start.1.max(end.1));
        let (mut inside, mut outside) = (Vec::new(), Vec::new());
        for dot in self.light_dots() {
            let within = dot.x - dot.size >= left && dot.x + dot.size <= right && dot.y - dot.size >= top && dot.y + dot.size <= bottom;
            if within {
                inside.push(dot.id);
            } else {
                outside.push(dot.id);
            }
        }
        self.canvas_select(&inside);
        if !keep_others {
            self.canvas_deselect(&outside);
        }
    }

    fn drag_handle(&mut self, handle: AdornerHandle, last: (f64, f64), now: (f64, f64), modifiers: Modifiers, started: bool) -> bool {
        let Some(mut a) = self.adorner.clone() else { return false };
        let canvas = (self.prop.width, self.prop.height);
        let center = (a.cx, a.cy);
        let name = match handle {
            AdornerHandle::Center => "Move Lights",
            AdornerHandle::Rotate => "Rotate Lights",
            _ => "Resize Lights",
        };
        match handle {
            AdornerHandle::Center => {
                let (dx, dy) = (now.0 - last.0, now.1 - last.1);
                // A step that would leave the canvas is dropped, not caught up later.
                let fits = a.x + dx > 0.0 && a.x + a.width + dx < canvas.0 && a.y + dy > 0.0 && a.y + a.height + dy < canvas.1;
                if !fits || (dx == 0.0 && dy == 0.0) {
                    return false;
                }
                if !started {
                    self.begin(name);
                }
                self.transform_selected(|p| (p.0 + dx, p.1 + dy));
                a.x += dx;
                a.y += dy;
                a.cx += dx;
                a.cy += dy;
            }
            AdornerHandle::Rotate => {
                let angle = rotation_angle(center, now, modifiers.command);
                let difference = angle - a.angle;
                if difference == 0.0 || !difference.is_finite() {
                    return false;
                }
                if !started {
                    self.begin(name);
                }
                self.transform_selected(|p| rotate(p, center, difference));
                a.angle = angle;
            }
            _ => {
                // The handles turn with the adorner, so the pull is measured in its own frame.
                let (dx, dy) = rotate((now.0 - last.0, now.1 - last.1), (0.0, 0.0), -a.angle);
                let (grow_x, grow_y) = (1.0 + dx / a.width, 1.0 + dy / a.height);
                let (shrink_x, shrink_y) = (1.0 - dx / a.width, 1.0 - dy / a.height);
                let (right, bottom) = (a.x + a.width, a.y + a.height);
                let (mut sx, mut sy, origin, corner) = match handle {
                    AdornerHandle::BottomRight => (grow_x, grow_y, (a.x, a.y), true),
                    AdornerHandle::TopRight => (grow_x, shrink_y, (a.x, bottom), true),
                    AdornerHandle::TopLeft => (shrink_x, shrink_y, (right, bottom), true),
                    AdornerHandle::BottomLeft => (shrink_x, grow_y, (right, a.y), true),
                    AdornerHandle::Right => (grow_x, 1.0, (a.x, a.y), false),
                    AdornerHandle::Left => (shrink_x, 1.0, (right, a.y), false),
                    AdornerHandle::Bottom => (1.0, grow_y, (a.x, a.y), false),
                    _ => (1.0, shrink_y, (a.x, bottom), false),
                };
                if corner && modifiers.shift {
                    sx = sx.max(sy);
                    sy = sx;
                }
                if !(sx.is_finite() && sy.is_finite()) || (sx == 1.0 && sy == 1.0) || !a.allows(sx, sy, canvas) {
                    return false;
                }
                if !started {
                    self.begin(name);
                }
                let angle = a.angle;
                self.transform_selected(|p| {
                    let (x, y) = rotate(p, center, -angle);
                    rotate((origin.0 + (x - origin.0) * sx, origin.1 + (y - origin.1) * sy), center, angle)
                });
                a.scale_bounds(sx, sy, origin);
            }
        }
        self.adorner = Some(a);
        true
    }

    /// `AddLight`: onto the single selected tree row's element when there is one, else into the first group that
    /// takes lights. The row the light ends up under is selected and opened.
    fn add_light_at(&mut self, x: f64, y: f64) {
        let target_row = self.single_selected().cloned();
        let target = target_row.as_ref().and_then(|row| row.last().copied());
        let size = self.settings.light_size();
        self.begin("Add Light");
        let Ok(owner) = self.prop.add_light(target, x, y, size) else {
            self.abandon();
            return;
        };
        let new_light = self.prop.element(owner).and_then(|e| e.lights().last()).map(|l| l.id);
        let row = if Some(owner) == target {
            target_row
        } else {
            let parent = self.prop.element(owner).and_then(|e| e.parents().first().copied());
            match (&target_row, parent) {
                (Some(row), Some(parent)) if row.last() == Some(&parent) => target_row,
                (_, Some(parent)) => self.paths_to(parent).into_iter().next(),
                _ => None,
            }
        };
        if let Some(row) = row {
            self.request_expand(row.clone());
            self.add_tree_rows(vec![row]);
        }
        // Upstream leaves a light added under an already selected row unselected, though the row says otherwise.
        if let Some(light) = new_light {
            if self.tree_selection.iter().any(|row| row.last().is_some_and(|id| self.prop.is_same_or_descendant(*id, owner))) {
                self.add_lights_quietly(vec![light]);
            }
        }
        self.rebuild_adorner();
    }

    // ----- Toolbar -----

    pub fn has_light_selection(&self) -> bool {
        !self.light_selection.is_empty()
    }

    fn selected_positions(&self) -> Vec<(Uuid, f64, f64)> {
        let by_id: std::collections::HashMap<Uuid, (f64, f64)> = self.prop.lights().map(|(_, l)| (l.id, (l.x, l.y))).collect();
        self.light_selection.iter().filter_map(|id| by_id.get(id).map(|(x, y)| (*id, *x, *y))).collect()
    }

    fn place(&mut self, moves: &[(Uuid, f64, f64)]) {
        let by_id: std::collections::HashMap<Uuid, (f64, f64)> = moves.iter().map(|(id, x, y)| (*id, (*x, *y))).collect();
        for light in self.prop.lights_mut() {
            if let Some((x, y)) = by_id.get(&light.id) {
                light.x = *x;
                light.y = *y;
            }
        }
        self.rebuild_adorner();
    }

    /// Align Left and Align Right both move every selected light to the first selected light's X, and Top and
    /// Bottom to its Y, as upstream.
    pub fn align(&mut self, edge: Edge) -> bool {
        let mut lights = self.selected_positions();
        let Some(&(_, x, y)) = lights.first() else { return false };
        for light in &mut lights {
            match edge {
                Edge::Left | Edge::Right => light.1 = x,
                Edge::Top | Edge::Bottom => light.2 = y,
            }
        }
        self.begin(match edge {
            Edge::Left => "Align Left",
            Edge::Right => "Align Right",
            Edge::Top => "Align Top",
            Edge::Bottom => "Align Bottom",
        });
        self.place(&lights);
        true
    }

    /// Evenly spaced between the lowest and highest coordinate; needs more than two lights.
    pub fn distribute(&mut self, axis: Axis) -> bool {
        let mut lights = self.selected_positions();
        if lights.len() <= 2 {
            return false;
        }
        let coordinate = |l: &(Uuid, f64, f64)| if axis == Axis::Horizontal { l.1 } else { l.2 };
        lights.sort_by(|a, b| coordinate(a).total_cmp(&coordinate(b)));
        let (min, max) = (coordinate(&lights[0]), coordinate(&lights[lights.len() - 1]));
        let step = (max - min) / (lights.len() - 1) as f64;
        for (i, light) in lights.iter_mut().enumerate().skip(1) {
            let value = min + i as f64 * step;
            if axis == Axis::Horizontal {
                light.1 = value;
            } else {
                light.2 = value;
            }
        }
        self.begin(if axis == Axis::Horizontal { "Distribute Horizontal" } else { "Distribute Vertical" });
        self.place(&lights);
        true
    }

    /// `X = max - (X - min)`, with max and min taken over absolute values as upstream does.
    pub fn flip(&mut self, axis: Axis) -> bool {
        let mut lights = self.selected_positions();
        if lights.is_empty() {
            return false;
        }
        let coordinate = |l: &(Uuid, f64, f64)| if axis == Axis::Horizontal { l.1 } else { l.2 };
        let max = lights.iter().map(|l| coordinate(l).abs()).fold(f64::MIN, f64::max);
        let min = lights.iter().map(|l| coordinate(l).abs()).fold(f64::MAX, f64::min);
        for light in &mut lights {
            if axis == Axis::Horizontal {
                light.1 = max - (light.1 - min);
            } else {
                light.2 = max - (light.2 - min);
            }
        }
        self.begin(if axis == Axis::Horizontal { "Flip Horizontal" } else { "Flip Vertical" });
        self.place(&lights);
        true
    }

    pub fn can_change_light_size(&self, change: SizeChange) -> bool {
        match change {
            SizeChange::Match => self.light_selection.len() > 1,
            _ => !self.light_selection.is_empty(),
        }
    }

    /// Increase and Decrease step each selected light's element once per selected light, so an element with five
    /// selected lights changes by five. Kept as upstream.
    pub fn change_light_size(&mut self, change: SizeChange) -> bool {
        if !self.can_change_light_size(change) {
            return false;
        }
        let owners = self.prop.light_owners();
        let elements: Vec<Uuid> = self.light_selection.iter().filter_map(|l| owners.get(l).copied()).collect();
        let Some(first) = elements.first().copied() else { return false };
        self.begin(match change {
            SizeChange::Increase => "Increase Light Size",
            SizeChange::Decrease => "Decrease Light Size",
            SizeChange::Match => "Match Light Size",
        });
        match change {
            SizeChange::Increase | SizeChange::Decrease => {
                let step = if change == SizeChange::Increase { 1 } else { -1 };
                for element in elements {
                    let size = self.prop.light_size(element);
                    self.prop.set_light_size(element, size + step);
                }
            }
            SizeChange::Match => {
                let size = self.prop.light_size(first);
                for element in elements.into_iter().skip(1) {
                    self.prop.set_light_size(element, size);
                }
            }
        }
        self.rebuild_adorner();
        true
    }

    /// `DeleteSelectedLights`, after the window's confirmation. An element left with no lights goes too.
    pub fn delete_selected_lights(&mut self) -> bool {
        if self.light_selection.is_empty() {
            return false;
        }
        let lights: HashSet<Uuid> = self.light_selection.iter().copied().collect();
        self.begin("Delete Lights");
        self.canvas_clear();
        self.prop.remove_lights(&lights);
        self.after_change();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{PropEditorSettings, RowPath};

    const NOW: &str = "2024-01-01T00:00:00";
    const NONE: Modifiers = Modifiers { command: false, shift: false };
    const COMMAND: Modifiers = Modifiers { command: true, shift: false };
    const SHIFT: Modifiers = Modifiers { command: false, shift: true };

    fn empty() -> PropEditor {
        PropEditor::new_document("Star {1}", PropEditorSettings::default(), NOW)
    }

    /// Lights at the given points, drawn by clicking in draw mode, then selection mode with nothing selected.
    fn drawn(points: &[(f64, f64)]) -> PropEditor {
        let mut editor = empty();
        for (x, y) in points {
            editor.pointer_down(*x, *y, NONE);
            editor.pointer_up();
        }
        editor.set_mode(CanvasMode::Select);
        editor.set_tree_selection(Vec::new());
        editor
    }

    fn positions(editor: &PropEditor) -> Vec<(f64, f64)> {
        editor.light_dots().iter().map(|d| (d.x, d.y)).collect()
    }

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }

    #[test]
    fn drawing_adds_numbered_nodes_under_one_group_and_selects_it() {
        let mut editor = empty();
        editor.pointer_down(100.0, 100.0, NONE);
        editor.pointer_up();
        editor.pointer_down(200.0, 100.0, NONE);
        let rows = editor.tree_rows();
        assert_eq!(rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), vec!["Star {1}", "Star {1} Group", "Star {1} Group - 1", "Star {1} Group - 2"]);
        let group_row: RowPath = rows[1].path.clone();
        assert_eq!(editor.tree_selection(), &[group_row.clone()]);
        assert_eq!(editor.light_selection().len(), 2, "the group row is selected, so both its lights are");
        assert!(editor.take_expand_requests().contains(&group_row));
        assert!(editor.adorner().is_none(), "no adorner in draw mode");
        assert!(editor.is_dirty());
        assert_eq!(editor.undo_name(), Some("Add Light"));
        editor.undo();
        assert_eq!(editor.prop().light_count(), 1);
    }

    #[test]
    fn drawing_onto_a_selected_light_node_makes_a_string_at_its_size() {
        let mut editor = drawn(&[(100.0, 100.0)]);
        let node = editor.tree_rows()[2].clone();
        editor.prop.set_light_size(node.id, 7);
        editor.set_mode(CanvasMode::Draw);
        editor.set_tree_selection(vec![node.path.clone()]);
        editor.pointer_down(150.0, 100.0, NONE);
        let e = editor.prop().element(node.id).unwrap();
        assert_eq!(e.lights().len(), 2);
        assert_eq!(e.lights()[1].size, 7.0);
        assert_eq!(editor.light_selection().len(), 2);
        assert_eq!(editor.tree_selection(), &[node.path]);
    }

    #[test]
    fn a_click_on_a_light_in_draw_mode_adds_nothing() {
        let mut editor = empty();
        editor.pointer_down(100.0, 100.0, NONE);
        editor.pointer_down(101.0, 101.0, NONE);
        assert_eq!(editor.prop().light_count(), 1);
    }

    #[test]
    fn selection_clicks_pick_toggle_and_clear() {
        let mut editor = drawn(&[(100.0, 100.0), (200.0, 100.0), (300.0, 100.0)]);
        let dots = editor.light_dots();
        editor.pointer_down(100.0, 102.0, NONE);
        editor.pointer_up();
        assert_eq!(editor.light_selection(), &[dots[0].id]);
        assert_eq!(editor.tree_selection().len(), 1, "its element's row is selected too");
        editor.pointer_down(200.0, 100.0, COMMAND);
        editor.pointer_up();
        assert_eq!(editor.light_selection(), &[dots[0].id, dots[1].id]);
        assert!(editor.adorner().is_some());
        editor.pointer_down(100.0, 100.0, NONE);
        editor.pointer_up();
        assert_eq!(editor.light_selection().len(), 2, "a selected light stays with its group for dragging");
        editor.pointer_down(100.0, 100.0, COMMAND);
        editor.pointer_up();
        assert_eq!(editor.light_selection(), &[dots[1].id]);
        assert!(editor.adorner().is_none());
        editor.pointer_down(500.0, 500.0, NONE);
        editor.pointer_up();
        assert!(editor.light_selection().is_empty());
        assert!(editor.tree_selection().is_empty());
        assert!(!editor.is_dirty() || editor.undo_name() == Some("Add Light"));
    }

    #[test]
    fn dragging_a_light_moves_the_selection_as_one_undo_step() {
        let mut editor = drawn(&[(100.0, 100.0), (200.0, 100.0)]);
        let undo_depth_name = editor.undo_name().map(str::to_string);
        editor.pointer_down(100.0, 100.0, NONE);
        editor.pointer_down(200.0, 100.0, COMMAND);
        editor.pointer_dragged(210.0, 105.0, NONE);
        editor.pointer_dragged(230.0, 120.0, NONE);
        editor.pointer_up();
        assert_eq!(positions(&editor), vec![(130.0, 120.0), (230.0, 120.0)]);
        assert_eq!(editor.undo_name(), Some("Move Lights"));
        editor.undo();
        assert_eq!(positions(&editor), vec![(100.0, 100.0), (200.0, 100.0)]);
        assert_eq!(editor.undo_name().map(str::to_string), undo_depth_name);
    }

    #[test]
    fn a_rubber_band_takes_lights_wholly_inside_it() {
        let mut editor = drawn(&[(100.0, 100.0), (200.0, 100.0), (300.0, 100.0)]);
        let dots = editor.light_dots();
        editor.pointer_down(90.0, 90.0, NONE);
        editor.pointer_dragged(202.0, 110.0, NONE);
        assert_eq!(editor.light_selection(), &[dots[0].id], "the second light's circle pokes out");
        assert!(editor.adorner().is_none(), "no adorner while the band is out");
        assert_eq!(editor.band(), Some((90.0, 90.0, 112.0, 20.0)));
        editor.pointer_dragged(204.0, 110.0, NONE);
        assert_eq!(editor.light_selection().len(), 2);
        editor.pointer_dragged(150.0, 110.0, NONE);
        assert_eq!(editor.light_selection(), &[dots[0].id], "shrinking the band lets lights go");
        editor.pointer_dragged(204.0, 110.0, NONE);
        editor.pointer_up();
        assert!(editor.band().is_none());
        assert!(editor.adorner().is_some());
        editor.pointer_down(290.0, 90.0, COMMAND);
        editor.pointer_dragged(310.0, 110.0, COMMAND);
        editor.pointer_up();
        assert_eq!(editor.light_selection().len(), 3, "Command keeps what was selected");
        assert!(!editor.is_dirty() || editor.undo_name() == Some("Add Light"));
    }

    fn select_all(editor: &mut PropEditor) {
        let (w, h) = (editor.prop().width, editor.prop().height);
        editor.pointer_down(0.0, 0.0, NONE);
        editor.pointer_dragged(w, h, NONE);
        editor.pointer_up();
    }

    #[test]
    fn the_adorner_wraps_the_selected_circles_and_places_its_handles() {
        let mut editor = drawn(&[(100.0, 100.0), (200.0, 150.0)]);
        select_all(&mut editor);
        let a = editor.adorner().unwrap();
        assert_eq!((a.x, a.y, a.width, a.height), (97.0, 97.0, 106.0, 56.0));
        assert_eq!((a.center_x, a.center_y, a.angle), (150.0, 125.0, 0.0));
        let at = |handle| a.handles.iter().find(|h| h.0 == handle).map(|h| (h.1, h.2)).unwrap();
        assert_eq!(at(AdornerHandle::TopLeft), (87.0, 87.0));
        assert_eq!(at(AdornerHandle::BottomRight), (203.0, 153.0));
        assert_eq!(at(AdornerHandle::Top), (145.0, 87.0));
        assert_eq!(at(AdornerHandle::Center), (145.0, 120.0));
        assert_eq!(at(AdornerHandle::Rotate), (145.0, 47.0));
        assert_eq!(editor.handle_at(205.0, 155.0), Some(AdornerHandle::BottomRight));
        assert_eq!(editor.handle_at(150.0, 125.0), Some(AdornerHandle::Center));
        assert_eq!(editor.handle_at(400.0, 400.0), None);
    }

    #[test]
    fn a_corner_scales_about_the_opposite_corner() {
        let mut editor = drawn(&[(100.0, 100.0), (200.0, 150.0)]);
        select_all(&mut editor);
        editor.pointer_down(205.0, 155.0, NONE);
        editor.pointer_dragged(258.0, 183.0, NONE);
        editor.pointer_up();
        // The box was 106 by 56 from (97, 97): pulled 53 right and 28 down it is half as big again.
        let after = positions(&editor);
        assert!(close(after[0], (101.5, 101.5)), "{after:?}");
        assert!(close(after[1], (251.5, 176.5)), "{after:?}");
        let a = editor.adorner().unwrap();
        assert!(close((a.x, a.y), (97.0, 97.0)) && close((a.width, a.height), (159.0, 84.0)));
        assert_eq!(editor.undo_name(), Some("Resize Lights"));
    }

    #[test]
    fn shift_on_a_corner_keeps_the_shape_and_an_edge_scales_one_axis() {
        let mut editor = drawn(&[(100.0, 100.0), (200.0, 150.0)]);
        select_all(&mut editor);
        editor.pointer_down(205.0, 155.0, NONE);
        editor.pointer_dragged(258.0, 155.0, SHIFT);
        editor.pointer_up();
        let after = positions(&editor);
        assert!(close(after[1], (251.5, 176.5)), "{after:?}");
        editor.undo();
        // The adorner is rebuilt by undo, so the right edge handle is back at x 203..213, y 120..130.
        editor.pointer_down(208.0, 125.0, NONE);
        editor.pointer_dragged(261.0, 140.0, NONE);
        editor.pointer_up();
        let after = positions(&editor);
        assert!(close(after[0], (101.5, 100.0)) && close(after[1], (251.5, 150.0)), "{after:?}");
    }

    #[test]
    fn a_scale_that_fails_the_size_test_is_refused() {
        let mut editor = drawn(&[(700.0, 500.0), (780.0, 580.0)]);
        select_all(&mut editor);
        let before = positions(&editor);
        // Bottom right handle sits at 783..793; doubling the box would push its scaled bottom past the canvas.
        editor.pointer_down(788.0, 588.0, NONE);
        editor.pointer_dragged(788.0 + 86.0, 588.0 + 86.0, NONE);
        editor.pointer_up();
        assert_eq!(positions(&editor), before);
        assert_ne!(editor.undo_name(), Some("Resize Lights"));
    }

    #[test]
    fn the_centre_handle_moves_everything_but_not_off_the_canvas() {
        let mut editor = drawn(&[(100.0, 100.0), (200.0, 150.0)]);
        select_all(&mut editor);
        editor.pointer_down(150.0, 125.0, NONE);
        editor.pointer_dragged(160.0, 135.0, NONE);
        assert_eq!(positions(&editor), vec![(110.0, 110.0), (210.0, 160.0)]);
        editor.pointer_dragged(0.0, 135.0, NONE);
        assert_eq!(positions(&editor), vec![(110.0, 110.0), (210.0, 160.0)], "that step would cross the left edge");
        editor.pointer_dragged(5.0, 135.0, NONE);
        assert_eq!(positions(&editor), vec![(115.0, 110.0), (215.0, 160.0)], "the refused step is not caught up");
        editor.pointer_up();
        let a = editor.adorner().unwrap();
        assert_eq!((a.center_x, a.center_y), (165.0, 135.0));
        assert_eq!(editor.undo_name(), Some("Move Lights"));
    }

    #[test]
    fn the_rotate_handle_turns_lights_about_the_centre() {
        let mut editor = drawn(&[(100.0, 100.0), (200.0, 100.0)]);
        select_all(&mut editor);
        let a = editor.adorner().unwrap();
        assert_eq!((a.center_x, a.center_y), (150.0, 100.0));
        // Rotate handle: x 145..155, y 47..57. Pull it round to the right of the centre: a quarter turn clockwise.
        editor.pointer_down(150.0, 52.0, NONE);
        editor.pointer_dragged(250.0, 100.0, NONE);
        let after = positions(&editor);
        assert!(close(after[0], (150.0, 50.0)) && close(after[1], (150.0, 150.0)), "{after:?}");
        assert_eq!(editor.adorner().unwrap().angle, 90.0);
        editor.pointer_dragged(250.0, 130.0, COMMAND);
        assert_eq!(editor.adorner().unwrap().angle, 90.0, "Command snaps to the nearest 45 degrees");
        editor.pointer_dragged(250.0, 200.0, COMMAND);
        assert_eq!(editor.adorner().unwrap().angle, 135.0);
        editor.pointer_up();
        assert_eq!(editor.adorner().unwrap().angle, 135.0, "the turned adorner stays after the drag");
        assert_eq!(editor.undo_name(), Some("Rotate Lights"));
        editor.undo();
        assert_eq!(positions(&editor), vec![(100.0, 100.0), (200.0, 100.0)]);
    }

    #[test]
    fn handles_are_found_where_they_are_drawn_after_a_turn() {
        let mut editor = drawn(&[(100.0, 100.0), (200.0, 100.0)]);
        select_all(&mut editor);
        editor.pointer_down(150.0, 52.0, NONE);
        editor.pointer_dragged(250.0, 100.0, NONE);
        editor.pointer_up();
        // Unturned, the right edge handle is at x 203..213, y 95..105; a quarter turn puts it below the centre.
        assert_eq!(editor.handle_at(150.0, 158.0), Some(AdornerHandle::Right));
        editor.pointer_down(150.0, 158.0, NONE);
        editor.pointer_dragged(150.0, 211.0, NONE);
        editor.pointer_up();
        let after = positions(&editor);
        assert!(close(after[0], (150.0, 51.5)) && close(after[1], (150.0, 201.5)), "{after:?}");
    }

    #[test]
    fn snapping_covers_every_detent() {
        let c = (0.0, 0.0);
        assert_eq!(rotation_angle(c, (0.0, -10.0), false), 0.0);
        assert_eq!(rotation_angle(c, (10.0, 0.0), false), 90.0);
        assert_eq!(rotation_angle(c, (0.0, 10.0), false), 180.0);
        assert_eq!(rotation_angle(c, (-10.0, 0.0), false), 270.0);
        assert_eq!(rotation_angle(c, (1.0, -10.0), true), 0.0);
        assert_eq!(rotation_angle(c, (-1.0, -10.0), true), 0.0);
        assert_eq!(rotation_angle(c, (10.0, -9.0), true), 45.0);
        assert_eq!(rotation_angle(c, (-10.0, 9.0), true), 225.0);
        assert_eq!(rotation_angle(c, (-10.0, -9.0), true), 315.0);
    }

    #[test]
    fn align_uses_the_first_selected_light_for_both_directions() {
        let mut editor = drawn(&[(100.0, 100.0), (200.0, 150.0), (300.0, 120.0)]);
        editor.pointer_down(200.0, 150.0, NONE);
        editor.pointer_down(100.0, 100.0, COMMAND);
        editor.pointer_down(300.0, 120.0, COMMAND);
        editor.pointer_up();
        assert!(editor.align(Edge::Right));
        assert_eq!(positions(&editor), vec![(200.0, 100.0), (200.0, 150.0), (200.0, 120.0)]);
        assert!(editor.align(Edge::Bottom));
        assert_eq!(positions(&editor), vec![(200.0, 150.0), (200.0, 150.0), (200.0, 150.0)]);
        assert_eq!(editor.undo_name(), Some("Align Bottom"));
    }

    #[test]
    fn distribute_needs_three_and_flip_mirrors() {
        let mut editor = drawn(&[(100.0, 100.0), (120.0, 200.0), (300.0, 130.0)]);
        editor.pointer_down(100.0, 100.0, NONE);
        editor.pointer_down(120.0, 200.0, COMMAND);
        editor.pointer_up();
        assert!(!editor.distribute(Axis::Horizontal));
        editor.pointer_down(300.0, 130.0, COMMAND);
        editor.pointer_up();
        assert!(editor.distribute(Axis::Horizontal));
        assert_eq!(positions(&editor), vec![(100.0, 100.0), (200.0, 200.0), (300.0, 130.0)]);
        assert!(editor.distribute(Axis::Vertical));
        assert_eq!(positions(&editor), vec![(100.0, 100.0), (200.0, 200.0), (300.0, 150.0)]);
        assert!(editor.flip(Axis::Horizontal));
        assert_eq!(positions(&editor), vec![(300.0, 100.0), (200.0, 200.0), (100.0, 150.0)]);
        assert!(editor.flip(Axis::Vertical));
        assert_eq!(positions(&editor), vec![(300.0, 200.0), (200.0, 100.0), (100.0, 150.0)]);
    }

    #[test]
    fn light_size_steps_once_per_selected_light() {
        let mut editor = drawn(&[(100.0, 100.0)]);
        let node = editor.tree_rows()[2].clone();
        editor.set_mode(CanvasMode::Draw);
        editor.set_tree_selection(vec![node.path.clone()]);
        editor.pointer_down(150.0, 100.0, NONE);
        editor.pointer_down(200.0, 100.0, NONE);
        editor.set_mode(CanvasMode::Select);
        assert_eq!(editor.light_selection().len(), 3);
        assert!(editor.change_light_size(SizeChange::Increase));
        assert_eq!(editor.prop().light_size(node.id), 6, "three selected lights of one element: three steps");
        assert!(editor.light_dots().iter().all(|d| d.size == 6.0));
        for _ in 0..4 {
            editor.change_light_size(SizeChange::Decrease);
        }
        assert_eq!(editor.prop().light_size(node.id), 1, "never below one");
    }

    #[test]
    fn match_light_size_copies_the_first_selected_lights_size() {
        let mut editor = drawn(&[(100.0, 100.0), (200.0, 100.0)]);
        let rows = editor.tree_rows();
        editor.prop.set_light_size(rows[3].id, 9);
        editor.pointer_down(200.0, 100.0, NONE);
        assert!(!editor.can_change_light_size(SizeChange::Match));
        editor.pointer_down(100.0, 100.0, COMMAND);
        editor.pointer_up();
        assert!(editor.change_light_size(SizeChange::Match));
        assert_eq!(editor.prop().light_size(rows[2].id), 9);
    }

    #[test]
    fn deleting_lights_removes_emptied_elements_and_clears_selection() {
        let mut editor = drawn(&[(100.0, 100.0), (200.0, 100.0)]);
        assert!(!editor.delete_selected_lights());
        editor.pointer_down(100.0, 100.0, NONE);
        editor.pointer_up();
        assert!(editor.delete_selected_lights());
        assert_eq!(editor.prop().light_count(), 1);
        assert_eq!(editor.tree_rows().len(), 3);
        assert!(editor.light_selection().is_empty() && editor.tree_selection().is_empty());
        assert_eq!(editor.order_rows().len(), 1);
        editor.undo();
        assert_eq!(editor.prop().light_count(), 2);
    }

    #[test]
    fn the_status_text_follows_the_pointer() {
        let mut editor = empty();
        assert_eq!(editor.pointer_text(), "");
        editor.pointer_moved(12.4, 99.6);
        assert_eq!(editor.pointer_text(), "12, 100");
        editor.pointer_exited();
        assert_eq!(editor.pointer_text(), "");
    }

    #[test]
    fn the_state_tab_and_draw_mode_show_no_adorner() {
        let mut editor = drawn(&[(100.0, 100.0), (200.0, 100.0)]);
        select_all(&mut editor);
        assert!(editor.adorner().is_some());
        editor.set_mode(CanvasMode::Draw);
        assert!(editor.adorner().is_none());
        editor.set_mode(CanvasMode::Select);
        assert!(editor.adorner().is_some());
        editor.set_tab(PanelTab::State);
        assert!(editor.adorner().is_none());
    }
}
