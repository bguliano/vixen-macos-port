//! Export Wire Diagram (F128 to F131): the options dialog's rules (`WireDiagramExportOptions` and its view model), which points a group draws
//! (`ElementModeling`), and the drawing (`WireDiagramSvgBuilder`). None of it changes the session.

use super::tree::{BitSet, Nid, SetupTree};
use crate::SessionError;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::fmt::Write as _;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use vixen_core::element::PropertyData;

// ---- Options --------------------------------------------------------------------------------------------------------

/// `WireDiagramExportOptions`: what one export draws with.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct WireDiagramOptions {
    /// 1 through 1000. 1 labels every node.
    pub label_interval: u32,
    /// 1 through 30.
    pub node_radius: f32,
    /// A whole number from 6 through 24.
    pub label_font_size: f32,
    /// 1 through 4. It scales the whole page, not what's on it.
    pub export_scale: f32,
}

impl Default for WireDiagramOptions {
    fn default() -> WireDiagramOptions {
        WireDiagramOptions { label_interval: 1, node_radius: 6.0, label_font_size: 14.0, export_scale: 1.0 }
    }
}

impl WireDiagramOptions {
    /// `Validate`. A float outside a range, which includes NaN and the infinities, is refused.
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=1000).contains(&self.label_interval) {
            return Err("The label interval must be between 1 and 1000.".to_string());
        }
        if !(1.0..=30.0).contains(&self.node_radius) {
            return Err("The node radius must be finite and between 1 and 30.".to_string());
        }
        if !(6.0..=24.0).contains(&self.label_font_size) || self.label_font_size != self.label_font_size.trunc() {
            return Err("The label font size must be a whole number between 6 and 24.".to_string());
        }
        if !(1.0..=4.0).contains(&self.export_scale) {
            return Err("The export scale must be finite and between 1 and 4.".to_string());
        }
        Ok(())
    }

    /// `CreateOptions`, once `ValidateFields` has passed. The error is the first complaint the dialog has, in that order.
    pub fn from_dialog(dialog: &WireDiagramDialog) -> Result<WireDiagramOptions, String> {
        // The box is disabled while every number shows, so whatever it holds then doesn't count.
        let label_interval = if dialog.all_numbers { 1 } else { parse_interval(&dialog.interval_text).ok_or(BAD_INTERVAL_MESSAGE)? };
        if !(6..=24).contains(&dialog.label_font_size) {
            return Err("Enter a number-label font size from 6 through 24.".to_string());
        }
        if !(1..=30).contains(&dialog.node_radius) {
            return Err("Enter a node radius from 1 through 30.".to_string());
        }
        if !(100..=400).contains(&dialog.scale_percent) {
            // Upstream's wording, slip included. The slider can't get here.
            return Err("Enter an scaling percentage from 100 through 400.".to_string());
        }
        Ok(WireDiagramOptions {
            label_interval,
            node_radius: dialog.node_radius as f32,
            label_font_size: dialog.label_font_size as f32,
            export_scale: dialog.scale_percent as f32 / 100.0,
        })
    }
}

/// What the dialog says about an interval it can't use.
pub const BAD_INTERVAL_MESSAGE: &str = "Enter a whole-number label interval from 1 through 1000.";

/// `int.TryParse(text, NumberStyles.None, …)` and the range: digits only, so no sign, spaces, separators or decimal point.
fn parse_interval(text: &str) -> Option<u32> {
    if !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok().filter(|interval| (1..=1000).contains(interval))
}

/// `WireDiagramExportOptionsViewModel`: what the dialog's controls hold. Each export starts from `default()`, so nothing is remembered between exports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WireDiagramDialog {
    pub all_numbers: bool,
    pub interval_text: String,
    pub node_radius: i32,
    pub label_font_size: i32,
    pub scale_percent: i32,
}

impl Default for WireDiagramDialog {
    fn default() -> WireDiagramDialog {
        WireDiagramDialog { all_numbers: true, interval_text: "50".to_string(), node_radius: 6, label_font_size: 14, scale_percent: 100 }
    }
}

impl WireDiagramDialog {
    /// Why OK is disabled, if it is.
    pub fn error(&self) -> Option<String> {
        WireDiagramOptions::from_dialog(self).err()
    }
}

// ---- Node order and labels ------------------------------------------------------------------------------------------

/// A point of the diagram: where it is, and the number it's labelled with.
pub type WirePoint = ((i32, i32), i32);

/// The group can't be exported because it has a leaf with no Location property.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NoLocation;

fn location_of(tree: &SetupTree, node: Nid) -> Option<(i32, i32)> {
    tree.node(node).properties.iter().find_map(|p| match p.data {
        PropertyData::Location { x, y, .. } => Some((x, y)),
        _ => None,
    })
}

fn order_of(tree: &SetupTree, node: Nid) -> Option<i32> {
    tree.node(node).properties.iter().find_map(|p| match p.data {
        PropertyData::Order { order } => Some(order),
        _ => None,
    })
}

fn all_located(tree: &SetupTree, leaves: &[Nid]) -> bool {
    !leaves.is_empty() && leaves.iter().all(|leaf| location_of(tree, *leaf).is_some())
}

/// The check `ElementsToSvg` makes before it shows the dialog: upstream silently does nothing when this is false.
pub fn wire_diagram_has_locations(tree: &SetupTree, group: Nid) -> bool {
    all_located(tree, &tree.distinct_leaves([group]))
}

/// F129: `ElementsToSvg`'s points, in wiring order.
pub fn wire_diagram_points(tree: &SetupTree, group: Nid) -> Result<Vec<WirePoint>, NoLocation> {
    let leaves = tree.distinct_leaves([group]);
    if !all_located(tree, &leaves) {
        return Err(NoLocation);
    }
    let with_order: Option<Vec<(Nid, i32)>> = leaves.iter().map(|leaf| Some((*leaf, order_of(tree, *leaf)?))).collect();
    let ordered = match with_order {
        Some(with_order) => order_nodes(tree, group, with_order),
        None => leaves,
    };
    Ok(ordered
        .iter()
        .enumerate()
        .map(|(position, node)| (location_of(tree, *node).unwrap_or((0, 0)), order_of(tree, *node).unwrap_or(position as i32 + 1)))
        .collect())
}

/// `OrderNodes`. Upstream's comment on the second half: "This may not work right all the time either". A leaf with no parent inside the group is dropped.
fn order_nodes(tree: &SetupTree, group: Nid, leaves: Vec<(Nid, i32)>) -> Vec<Nid> {
    let mut sorted = leaves.clone();
    sorted.sort_by_key(|(_, order)| *order);
    if sorted.windows(2).all(|pair| pair[0].1 != pair[1].1) {
        return sorted.into_iter().map(|(node, _)| node).collect();
    }

    let inside = non_leaves(tree, group);
    let mut lists: Vec<Vec<(Nid, i32)>> = Vec::new();
    let mut list_of_parent: HashMap<Nid, usize> = HashMap::new();
    for (leaf, order) in leaves {
        let Some(parent) = tree.node(leaf).parents.iter().find(|p| inside.contains(p.0 as usize)) else { continue };
        let list = *list_of_parent.entry(*parent).or_insert_with(|| {
            lists.push(Vec::new());
            lists.len() - 1
        });
        lists[list].push((leaf, order));
    }
    for list in &mut lists {
        list.sort_by_key(|(_, order)| *order);
    }
    lists.into_iter().flatten().map(|(node, _)| node).collect()
}

/// `GetNonLeafEnumerator`, as a set: the group and every group below it.
fn non_leaves(tree: &SetupTree, group: Nid) -> BitSet {
    let mut inside = BitSet::new(tree.slots());
    let mut stack = vec![group];
    while let Some(node) = stack.pop() {
        if !tree.is_leaf(node) && inside.insert(node.0 as usize) {
            stack.extend_from_slice(&tree.node(node).children);
        }
    }
    inside
}

// ---- The drawing ----------------------------------------------------------------------------------------------------

const CANVAS_MARGIN: f32 = 24.0;

// Windows measured labels with GDI+. These are Arial's own numbers (F130), so coordinates can differ slightly from Windows' while the rules stay the same.
const DIGIT_ADVANCE: f32 = 0.556;
const MINUS_ADVANCE: f32 = 0.333;
const ASCENT: f32 = 0.905;
const DESCENT: f32 = 0.212;

/// One thing the SVG draws, in the canvas's coordinates.
#[derive(Clone, Debug, PartialEq)]
enum Mark {
    Star([(f32, f32); 10]),
    Square { x: f32, y: f32, side: f32 },
    Diamond([(f32, f32); 4]),
    Circle { x: f32, y: f32, radius: f32 },
    Wire { from: (f32, f32), to: (f32, f32) },
    /// `x` is where the text starts, and `y` its baseline.
    Label { text: String, x: f32, y: f32 },
}

/// `Build`'s document, before it becomes text.
#[derive(Clone, Debug, PartialEq)]
struct Diagram {
    width_inches: f32,
    height_inches: f32,
    canvas_width: f32,
    canvas_height: f32,
    font_size: f32,
    marks: Vec<Mark>,
}

struct MeasuredLabel {
    index: usize,
    text: String,
    width: f32,
}

struct Layout {
    min_x: i32,
    min_y: i32,
    width: f64,
    scale: f64,
    canvas_width: f32,
    canvas_height: f32,
    origin_x: f64,
    origin_y: f64,
}

fn is_label_selected(index: usize, count: usize, interval: usize) -> bool {
    index == 0 || (index + 1) % interval == 0 || index == count - 1
}

fn label_width(text: &str, font_size: f32) -> f32 {
    let em: f32 = text.bytes().map(|b| if b == b'-' { MINUS_ADVANCE } else { DIGIT_ADVANCE }).sum();
    em * font_size + 2.0
}

fn label_ascent(font_size: f32) -> f32 {
    font_size * ASCENT + 2.0
}

fn label_descent(font_size: f32) -> f32 {
    font_size * DESCENT + 2.0
}

/// `CalculateLayout`: letter paper, whichever way up lets the drawing be larger.
fn calculate_layout(points: &[WirePoint], labels: &[MeasuredLabel], radius: f32, font_size: f32) -> Layout {
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for ((x, y), _) in points {
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
    }
    let label_height = font_size * (ASCENT + DESCENT) + 4.0;
    let mut max_half_diagonal = 0f32;
    for label in labels {
        max_half_diagonal = max_half_diagonal.max((label.width * label.width + label_height * label_height).sqrt() / 2.0);
    }

    let width = f64::from(max_x) - f64::from(min_x);
    let height = f64::from(max_y) - f64::from(min_y);
    let outer_radius = radius * 1.25;
    const LABEL_GAP: f32 = 2.0;
    let side_guard = outer_radius.max(radius + 2.0 * max_half_diagonal + 2.0 * LABEL_GAP);
    let portrait_scale = fit_scale(816.0, 1056.0, width, height, side_guard);
    let landscape_scale = fit_scale(1056.0, 816.0, width, height, side_guard);
    let (canvas_width, canvas_height, scale) =
        if portrait_scale >= landscape_scale { (816.0, 1056.0, portrait_scale) } else { (1056.0, 816.0, landscape_scale) };
    let guard = f64::from(side_guard);
    let margin = f64::from(CANVAS_MARGIN);
    let origin_x = margin + (f64::from(canvas_width - 2.0 * CANVAS_MARGIN) - (width * scale + guard + guard)) / 2.0 + guard;
    let origin_y = margin + (f64::from(canvas_height - 2.0 * CANVAS_MARGIN) - (height * scale + guard + guard)) / 2.0 + guard;
    Layout { min_x, min_y, width, scale, canvas_width, canvas_height, origin_x, origin_y }
}

/// `GetScale`.
fn fit_scale(canvas_width: f32, canvas_height: f32, width: f64, height: f64, guard: f32) -> f64 {
    let usable_width = canvas_width - 2.0 * CANVAS_MARGIN - guard - guard;
    let usable_height = canvas_height - 2.0 * CANVAS_MARGIN - guard - guard;
    let scale_x = if width > 0.0 { f64::from(usable_width) / width } else { f64::INFINITY };
    let scale_y = if height > 0.0 { f64::from(usable_height) / height } else { f64::INFINITY };
    if width == 0.0 && height == 0.0 {
        1.0
    } else {
        scale_x.min(scale_y)
    }
}

fn start_marker(point: (f32, f32), radius: f32) -> Mark {
    let outer_radius = radius * 1.5;
    let inner_radius = outer_radius / 2.0;
    Mark::Star(std::array::from_fn(|index| {
        let angle = -PI / 2.0 + index as f64 * PI / 5.0;
        let vertex_radius = f64::from(if index % 2 == 0 { outer_radius } else { inner_radius });
        (point.0 + (angle.cos() * vertex_radius) as f32, point.1 + (angle.sin() * vertex_radius) as f32)
    }))
}

fn diamond_marker(point: (f32, f32), radius: f32) -> Mark {
    Mark::Diamond([(point.0, point.1 - radius), (point.0 + radius, point.1), (point.0, point.1 + radius), (point.0 - radius, point.1)])
}

/// `Build`. `None` when there are no points, or when the options are ones `Validate` would throw on.
fn build_diagram(points: &[WirePoint], flip: bool, options: &WireDiagramOptions) -> Option<Diagram> {
    if options.validate().is_err() || points.is_empty() {
        return None;
    }
    let count = points.len();
    let interval = options.label_interval as usize;
    let radius = options.node_radius;
    let font_size = options.label_font_size;

    let labels: Vec<MeasuredLabel> = (0..count)
        .filter(|index| is_label_selected(*index, count, interval))
        .map(|index| {
            let text = points[index].1.to_string();
            let width = label_width(&text, font_size);
            MeasuredLabel { index, text, width }
        })
        .collect();
    let layout = calculate_layout(points, &labels, radius, font_size);

    let canvas: Vec<(f32, f32)> = points
        .iter()
        .map(|((source_x, source_y), _)| {
            let mut x = layout.origin_x + (f64::from(*source_x) - f64::from(layout.min_x)) * layout.scale;
            let y = layout.origin_y + (f64::from(*source_y) - f64::from(layout.min_y)) * layout.scale;
            if flip {
                x = layout.origin_x + layout.width * layout.scale - (x - layout.origin_x);
            }
            (x as f32, y as f32)
        })
        .collect();

    let mut marks = Vec::with_capacity(count * 2 + labels.len());
    for (index, point) in canvas.iter().enumerate() {
        marks.push(match index {
            0 => start_marker(*point, radius),
            _ if index == count - 1 => Mark::Square { x: point.0 - radius, y: point.1 - radius, side: radius * 2.0 },
            _ if interval > 1 && is_label_selected(index, count, interval) => diamond_marker(*point, radius),
            _ => Mark::Circle { x: point.0, y: point.1, radius },
        });
    }
    for pair in canvas.windows(2) {
        marks.push(Mark::Wire { from: pair[0], to: pair[1] });
    }

    // Labels go last, so neither markers nor wires cover them.
    let ascent = label_ascent(font_size);
    let descent = label_descent(font_size);
    let mut placer = LabelPlacer::new(&canvas, labels.len(), radius, layout.canvas_width, layout.canvas_height, (ascent + descent) / 2.0);
    for label in labels {
        let center = placer.place(canvas[label.index], label.width);
        marks.push(Mark::Label { text: label.text, x: center.0 - label.width / 2.0, y: center.1 + (ascent - descent) / 2.0 });
    }

    Some(Diagram {
        width_inches: layout.canvas_width / 96.0 * options.export_scale,
        height_inches: layout.canvas_height / 96.0 * options.export_scale,
        canvas_width: layout.canvas_width,
        canvas_height: layout.canvas_height,
        font_size,
        marks,
    })
}

// ---- Placing labels -------------------------------------------------------------------------------------------------

/// A placed label's extent for overlap tests (`GetLabelBounds`), which is three quarters of its real size.
#[derive(Copy, Clone)]
struct LabelBounds {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl LabelBounds {
    fn new(center: (f32, f32), half_width: f32, half_height: f32) -> LabelBounds {
        let width = 1.5 * half_width;
        let height = 1.5 * half_height;
        let left = center.0 - width / 2.0;
        let top = center.1 - height / 2.0;
        LabelBounds { left, top, right: left + width, bottom: top + height }
    }

    fn overlap(&self, other: &LabelBounds) -> f32 {
        let width = self.right.min(other.right) - self.left.max(other.left);
        let height = self.bottom.min(other.bottom) - self.top.max(other.top);
        if width >= 0.0 && height >= 0.0 {
            width * height
        } else {
            0.0
        }
    }
}

const CELL_SIZE: f32 = 32.0;

/// Which items might be near a spot. `PlaceLabel` tests each candidate against every wire and every placed label, which is quadratic in a group's size when every node is
/// numbered, the dialog's default. Only what's near a candidate can touch it, so looking only there gives the same answers.
struct NearbyItems {
    columns: usize,
    rows: usize,
    cells: Vec<Vec<u32>>,
    /// The visit that last saw each item, because an item can be in several cells.
    seen_in: Vec<u32>,
    visit: u32,
}

impl NearbyItems {
    fn new(canvas_width: f32, canvas_height: f32, items: usize) -> NearbyItems {
        let columns = (canvas_width / CELL_SIZE) as usize + 1;
        let rows = (canvas_height / CELL_SIZE) as usize + 1;
        NearbyItems { columns, rows, cells: vec![Vec::new(); columns * rows], seen_in: vec![0; items], visit: 0 }
    }

    /// The pixel of slack keeps rounding in the exact tests from mattering at a cell's edge. Anything off the page counts as being in the nearest cells.
    fn span(low: f32, high: f32, count: usize) -> RangeInclusive<usize> {
        let cell = |value: f32| ((value / CELL_SIZE) as usize).min(count - 1);
        cell(low - 1.0)..=cell(high + 1.0)
    }

    fn add(&mut self, item: u32, left: f32, top: f32, right: f32, bottom: f32) {
        for row in Self::span(top, bottom, self.rows) {
            for column in Self::span(left, right, self.columns) {
                let cell = &mut self.cells[row * self.columns + column];
                if cell.last() != Some(&item) {
                    cell.push(item);
                }
            }
        }
    }

    /// In pieces no longer than a cell, so a long diagonal isn't in every cell of its bounding box.
    fn add_wire(&mut self, item: u32, from: (f32, f32), to: (f32, f32)) {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let pieces = (dx.abs().max(dy.abs()) / CELL_SIZE).ceil().max(1.0) as u32;
        let mut start = from;
        for piece in 1..=pieces {
            let t = piece as f32 / pieces as f32;
            let end = (from.0 + dx * t, from.1 + dy * t);
            self.add(item, start.0.min(end.0), start.1.min(end.1), start.0.max(end.0), start.1.max(end.1));
            start = end;
        }
    }

    /// Every item whose cells touch the rectangle, once each, in no particular order.
    fn visit(&mut self, left: f32, top: f32, right: f32, bottom: f32, mut found: impl FnMut(usize)) {
        self.visit += 1;
        for row in Self::span(top, bottom, self.rows) {
            for column in Self::span(left, right, self.columns) {
                for item in &self.cells[row * self.columns + column] {
                    let item = *item as usize;
                    if self.seen_in[item] != self.visit {
                        self.seen_in[item] = self.visit;
                        found(item);
                    }
                }
            }
        }
    }
}

/// `PlaceLabel`, keeping what it has placed so far.
struct LabelPlacer<'a> {
    points: &'a [(f32, f32)],
    wires: NearbyItems,
    placed: Vec<LabelBounds>,
    placed_nearby: NearbyItems,
    overlapping: Vec<usize>,
    radius: f32,
    canvas_width: f32,
    canvas_height: f32,
    half_height: f32,
}

impl<'a> LabelPlacer<'a> {
    fn new(points: &'a [(f32, f32)], labels: usize, radius: f32, canvas_width: f32, canvas_height: f32, half_height: f32) -> LabelPlacer<'a> {
        let mut wires = NearbyItems::new(canvas_width, canvas_height, points.len());
        for (wire, pair) in points.windows(2).enumerate() {
            wires.add_wire(wire as u32, pair[0], pair[1]);
        }
        LabelPlacer {
            points,
            wires,
            placed: Vec::with_capacity(labels),
            placed_nearby: NearbyItems::new(canvas_width, canvas_height, labels),
            overlapping: Vec::new(),
            radius,
            canvas_width,
            canvas_height,
            half_height,
        }
    }

    /// The centre for a label `width` wide at `point`. Of the candidates on the page, the fewest wire crossings, then the least overlap with labels already placed, then the
    /// nearest start of text, then the first. Upstream treats "no crossings" and "the fewest" as two cases, which picks the same one. If none is on the page, the nearest.
    fn place(&mut self, point: (f32, f32), width: f32) -> (f32, f32) {
        const GAP: f32 = 2.0;
        const CLEARANCE: f32 = 2.5;
        const DIRECTIONS: [(f32, f32); 8] =
            [(1.0, -1.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (-1.0, 1.0), (-1.0, 0.0), (-1.0, -1.0), (0.0, -1.0)];
        let half_width = width / 2.0;
        let half_height = self.half_height;

        let mut best: Option<((f32, f32), (u32, f32, f32))> = None;
        let mut nearest = (point, f32::INFINITY);
        for (direction_x, direction_y) in DIRECTIONS {
            let length = (direction_x * direction_x + direction_y * direction_y).sqrt();
            let (unit_x, unit_y) = (direction_x / length, direction_y / length);
            let projected_half_size = unit_x.abs() * half_width + unit_y.abs() * half_height;
            for gap_multiplier in [1.0, 2.0] {
                let distance = self.radius + projected_half_size + GAP * gap_multiplier;
                let candidate = (point.0 + unit_x * distance, point.1 + unit_y * distance);

                let (text_x, text_y) = (candidate.0 - half_width - point.0, candidate.1 - point.1);
                let start_distance = (text_x * text_x + text_y * text_y).sqrt();
                if start_distance < nearest.1 {
                    nearest = (candidate, start_distance);
                }

                let on_page = candidate.0 - half_width >= CLEARANCE
                    && candidate.0 + half_width <= self.canvas_width - CLEARANCE
                    && candidate.1 - half_height >= CLEARANCE
                    && candidate.1 + half_height <= self.canvas_height - CLEARANCE;
                if !on_page {
                    continue;
                }
                let crossings = self.count_crossings(candidate, half_width + CLEARANCE, half_height + CLEARANCE);
                if best.is_some_and(|(_, rank)| crossings > rank.0) {
                    continue;
                }
                let rank = (crossings, self.overlap_area(LabelBounds::new(candidate, half_width, half_height)), start_distance);
                if best.is_none_or(|(_, other)| rank < other) {
                    best = Some((candidate, rank));
                }
            }
        }

        let chosen = best.map_or(nearest.0, |(candidate, _)| candidate);
        let bounds = LabelBounds::new(chosen, half_width, half_height);
        self.placed_nearby.add(self.placed.len() as u32, bounds.left, bounds.top, bounds.right, bounds.bottom);
        self.placed.push(bounds);
        chosen
    }

    /// `CountIntersections`, with the wires in the label's coordinates and the label's rectangle grown by the clearance.
    fn count_crossings(&mut self, center: (f32, f32), half_width: f32, half_height: f32) -> u32 {
        let points = self.points;
        let mut crossings = 0;
        self.wires.visit(center.0 - half_width, center.1 - half_height, center.0 + half_width, center.1 + half_height, |wire| {
            let (from, to) = (points[wire], points[wire + 1]);
            let start = (from.0 - center.0, from.1 - center.1);
            let end = (to.0 - center.0, to.1 - center.1);
            if segment_intersects_rectangle(start, end, half_width, half_height) {
                crossings += 1;
            }
        });
        crossings
    }

    /// `CalculateLabelOverlapArea`. The sum goes in the order the labels were placed, since rounding makes it depend on that.
    fn overlap_area(&mut self, bounds: LabelBounds) -> f32 {
        let overlapping = &mut self.overlapping;
        overlapping.clear();
        self.placed_nearby.visit(bounds.left, bounds.top, bounds.right, bounds.bottom, |label| overlapping.push(label));
        overlapping.sort_unstable();
        let mut area = 0.0;
        for label in overlapping.iter() {
            area += bounds.overlap(&self.placed[*label]);
        }
        area
    }
}

fn segment_intersects_rectangle(start: (f32, f32), end: (f32, f32), half_width: f32, half_height: f32) -> bool {
    let (mut min, mut max) = (0.0, 1.0);
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    clip(-dx, start.0 + half_width, &mut min, &mut max)
        && clip(dx, half_width - start.0, &mut min, &mut max)
        && clip(-dy, start.1 + half_height, &mut min, &mut max)
        && clip(dy, half_height - start.1, &mut min, &mut max)
}

fn clip(direction: f32, distance: f32, min: &mut f32, max: &mut f32) -> bool {
    if direction == 0.0 {
        return distance >= 0.0;
    }
    let amount = distance / direction;
    if direction < 0.0 {
        if amount > *max {
            return false;
        }
        *min = min.max(amount);
    } else {
        if amount < *min {
            return false;
        }
        *max = max.min(amount);
    }
    true
}

// ---- The file -------------------------------------------------------------------------------------------------------

fn write_polygon(out: &mut String, vertices: &[(f32, f32)], fill: &str) {
    out.push_str("    <polygon points=\"");
    for (index, (x, y)) in vertices.iter().enumerate() {
        let separator = if index == 0 { "" } else { " " };
        let _ = write!(out, "{separator}{x},{y}");
    }
    let _ = writeln!(out, "\" fill=\"{fill}\"/>");
}

fn render(diagram: &Diagram) -> String {
    let mut out = String::with_capacity(256 + diagram.marks.len() * 80);
    let _ = write!(
        out,
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\" width=\"{}in\" height=\"{}in\" viewBox=\"0 0 {} {}\">\n  <g>\n",
        diagram.width_inches, diagram.height_inches, diagram.canvas_width, diagram.canvas_height
    );
    for mark in &diagram.marks {
        match mark {
            Mark::Star(vertices) => write_polygon(&mut out, vertices, "green"),
            Mark::Diamond(vertices) => write_polygon(&mut out, vertices, "red"),
            Mark::Square { x, y, side } => {
                let _ = writeln!(out, "    <rect x=\"{x}\" y=\"{y}\" width=\"{side}\" height=\"{side}\" fill=\"red\"/>");
            }
            Mark::Circle { x, y, radius } => {
                let _ = writeln!(out, "    <circle cx=\"{x}\" cy=\"{y}\" r=\"{radius}\" fill=\"red\"/>");
            }
            Mark::Wire { from, to } => {
                let _ = writeln!(out, "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"1\"/>", from.0, from.1, to.0, to.1);
            }
            Mark::Label { text, x, y } => {
                let _ = writeln!(out, "    <text x=\"{x}\" y=\"{y}\" fill=\"black\" font-family=\"Arial\" font-size=\"{}\">{text}</text>", diagram.font_size);
            }
        }
    }
    out.push_str("  </g>\n</svg>\n");
    out
}

/// F130. `None` when there are no points, or when the options don't pass `validate`.
pub fn wire_diagram_svg(points: &[WirePoint], flip: bool, options: &WireDiagramOptions) -> Option<String> {
    build_diagram(points, flip, options).map(|diagram| render(&diagram))
}

/// F131's file: a new `.svg` in `folder`, for the caller to open. `None` when `wire_diagram_svg` has nothing to write.
pub fn write_wire_diagram(points: &[WirePoint], flip: bool, options: &WireDiagramOptions, folder: &Path) -> Result<Option<PathBuf>, SessionError> {
    let Some(svg) = wire_diagram_svg(points, flip, options) else { return Ok(None) };
    let path = folder.join(format!("Wire Diagram {}.svg", Uuid::new_v4()));
    vixen_io::file::write_atomic(&path, svg.as_bytes()).map_err(SessionError::Io)?;
    Ok(Some(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::element::{property_ids, PropertyInstance};
    use vixen_xml::XElement;

    // ---- Reading a diagram ------------------------------------------------------------------------------------------

    impl Diagram {
        fn stars(&self) -> Vec<&[(f32, f32); 10]> {
            self.marks.iter().filter_map(|m| if let Mark::Star(v) = m { Some(v) } else { None }).collect()
        }

        fn diamonds(&self) -> Vec<&[(f32, f32); 4]> {
            self.marks.iter().filter_map(|m| if let Mark::Diamond(v) = m { Some(v) } else { None }).collect()
        }

        fn circles(&self) -> Vec<(f32, f32, f32)> {
            self.marks.iter().filter_map(|m| if let Mark::Circle { x, y, radius } = m { Some((*x, *y, *radius)) } else { None }).collect()
        }

        fn squares(&self) -> Vec<(f32, f32, f32)> {
            self.marks.iter().filter_map(|m| if let Mark::Square { x, y, side } = m { Some((*x, *y, *side)) } else { None }).collect()
        }

        fn wires(&self) -> Vec<((f32, f32), (f32, f32))> {
            self.marks.iter().filter_map(|m| if let Mark::Wire { from, to } = m { Some((*from, *to)) } else { None }).collect()
        }

        fn labels(&self) -> Vec<(&str, f32, f32)> {
            self.marks.iter().filter_map(|m| if let Mark::Label { text, x, y } = m { Some((text.as_str(), *x, *y)) } else { None }).collect()
        }

        fn texts(&self) -> Vec<&str> {
            self.labels().into_iter().map(|l| l.0).collect()
        }

        fn kinds(&self) -> Vec<&'static str> {
            self.marks
                .iter()
                .map(|m| match m {
                    Mark::Star(_) => "star",
                    Mark::Square { .. } => "square",
                    Mark::Diamond(_) => "diamond",
                    Mark::Circle { .. } => "circle",
                    Mark::Wire { .. } => "wire",
                    Mark::Label { .. } => "label",
                })
                .collect()
        }

        /// The star's centre, then the circles', then the square's.
        fn marker_centers(&self) -> Vec<(f32, f32)> {
            let mut out: Vec<(f32, f32)> = self.stars().iter().map(|v| polygon_center(&v[..])).collect();
            out.extend(self.circles().iter().map(|c| (c.0, c.1)));
            out.extend(self.squares().iter().map(|s| (s.0 + s.2 / 2.0, s.1 + s.2 / 2.0)));
            out
        }
    }

    fn polygon_center(vertices: &[(f32, f32)]) -> (f32, f32) {
        let count = vertices.len() as f64;
        (
            (vertices.iter().map(|v| f64::from(v.0)).sum::<f64>() / count) as f32,
            (vertices.iter().map(|v| f64::from(v.1)).sum::<f64>() / count) as f32,
        )
    }

    fn close(a: f32, b: f32, tolerance: f32) -> bool {
        (a - b).abs() <= tolerance
    }

    fn sequence(count: i32) -> Vec<WirePoint> {
        (1..=count).map(|i| ((i, i), i)).collect()
    }

    fn with_interval(label_interval: u32) -> WireDiagramOptions {
        WireDiagramOptions { label_interval, ..Default::default() }
    }

    fn draw(points: &[WirePoint], flip: bool, options: &WireDiagramOptions) -> Diagram {
        build_diagram(points, flip, options).expect("a diagram")
    }

    // ---- WireDiagramExportOptionsTests ------------------------------------------------------------------------------

    #[test]
    fn defaults_are_all_labels_current_radius_and_normal_scale() {
        let options = WireDiagramOptions::default();
        assert_eq!(options.label_interval, 1);
        assert_eq!(options.node_radius, 6.0);
        assert_eq!(options.label_font_size, 14.0);
        assert_eq!(options.export_scale, 1.0);
        assert_eq!(options.validate(), Ok(()));
    }

    #[test]
    fn validate_label_interval_bounds() {
        for label_interval in [1, 1000] {
            assert_eq!(with_interval(label_interval).validate(), Ok(()));
        }
        for label_interval in [0, 1001] {
            assert_eq!(with_interval(label_interval).validate(), Err("The label interval must be between 1 and 1000.".to_string()));
        }
    }

    #[test]
    fn validate_node_radius_bounds() {
        for node_radius in [1.0, 30.0] {
            assert_eq!(WireDiagramOptions { node_radius, ..Default::default() }.validate(), Ok(()));
        }
        for node_radius in [0.0, 30.01, f32::NAN, f32::INFINITY] {
            let refused = WireDiagramOptions { node_radius, ..Default::default() }.validate();
            assert_eq!(refused, Err("The node radius must be finite and between 1 and 30.".to_string()));
        }
    }

    #[test]
    fn validate_label_font_size_bounds() {
        for label_font_size in [6.0, 14.0, 24.0] {
            assert_eq!(WireDiagramOptions { label_font_size, ..Default::default() }.validate(), Ok(()));
        }
        for label_font_size in [0.0, 5.0, 25.0, 14.5, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let refused = WireDiagramOptions { label_font_size, ..Default::default() }.validate();
            assert_eq!(refused, Err("The label font size must be a whole number between 6 and 24.".to_string()));
        }
    }

    #[test]
    fn validate_export_scale_bounds() {
        for export_scale in [1.0, 4.0] {
            assert_eq!(WireDiagramOptions { export_scale, ..Default::default() }.validate(), Ok(()));
        }
        for export_scale in [0.99, 4.01, f32::NAN, f32::INFINITY] {
            let refused = WireDiagramOptions { export_scale, ..Default::default() }.validate();
            assert_eq!(refused, Err("The export scale must be finite and between 1 and 4.".to_string()));
        }
    }

    // ---- WireDiagramExportOptionsViewModelTests -----------------------------------------------------------------------

    fn interval_dialog(text: &str) -> WireDiagramDialog {
        WireDiagramDialog { all_numbers: false, interval_text: text.to_string(), ..Default::default() }
    }

    #[test]
    fn the_dialog_starts_at_the_design_defaults() {
        let dialog = WireDiagramDialog::default();
        assert!(dialog.all_numbers);
        assert_eq!(dialog.interval_text, "50");
        assert_eq!(dialog.label_font_size, 14);
        assert_eq!(dialog.node_radius, 6);
        assert_eq!(dialog.scale_percent, 100);
        assert_eq!(dialog.error(), None);
    }

    #[test]
    fn all_numbers_ignores_the_disabled_interval() {
        for text in ["50", "0", "abc", ""] {
            let dialog = WireDiagramDialog { all_numbers: true, interval_text: text.to_string(), ..Default::default() };
            assert_eq!(dialog.error(), None);
            assert_eq!(WireDiagramOptions::from_dialog(&dialog).unwrap().label_interval, 1);
        }
    }

    #[test]
    fn interval_mode_uses_the_interval_typed() {
        for (text, expected) in [("50", 50), ("100", 100), ("1", 1), ("1000", 1000), ("050", 50)] {
            assert_eq!(WireDiagramOptions::from_dialog(&interval_dialog(text)).unwrap().label_interval, expected, "{text}");
        }
    }

    #[test]
    fn interval_mode_refuses_anything_but_a_plain_whole_number_in_range() {
        for text in ["", "abc", "0", "1001", "50.5", "+50", "-1", " 50", "50 ", "1,000", "5e1", "٥٠", "99999999999"] {
            let dialog = interval_dialog(text);
            assert_eq!(dialog.error().as_deref(), Some(BAD_INTERVAL_MESSAGE), "{text:?}");
            assert_eq!(BAD_INTERVAL_MESSAGE, "Enter a whole-number label interval from 1 through 1000.");
            assert!(WireDiagramOptions::from_dialog(&dialog).is_err());
        }
    }

    #[test]
    fn accepting_maps_the_sliders_independently() {
        for label_font_size in [6, 14, 24] {
            let options = WireDiagramOptions::from_dialog(&WireDiagramDialog { label_font_size, ..Default::default() }).unwrap();
            assert_eq!(options.label_font_size, label_font_size as f32);
        }
        let options = WireDiagramOptions::from_dialog(&WireDiagramDialog { label_font_size: 24, node_radius: 1, ..Default::default() }).unwrap();
        assert_eq!(options.label_font_size, 24.0);
        assert_eq!(options.node_radius, 1.0);
        let options = WireDiagramOptions::from_dialog(&WireDiagramDialog { scale_percent: 200, ..Default::default() }).unwrap();
        assert_eq!(options.export_scale, 2.0);
        assert_eq!(options.validate(), Ok(()));
    }

    #[test]
    fn sliders_out_of_range_prevent_acceptance() {
        for label_font_size in [5, 25] {
            assert!(WireDiagramDialog { label_font_size, ..Default::default() }.error().is_some());
        }
        for node_radius in [0, 31] {
            assert!(WireDiagramDialog { node_radius, ..Default::default() }.error().is_some());
        }
        for scale_percent in [99, 401] {
            assert!(WireDiagramDialog { scale_percent, ..Default::default() }.error().is_some());
        }
    }

    // ---- WireDiagramSvgBuilderTests -----------------------------------------------------------------------------------

    #[test]
    fn empty_input_draws_nothing() {
        assert_eq!(build_diagram(&[], false, &WireDiagramOptions::default()), None);
        assert_eq!(wire_diagram_svg(&[], false, &WireDiagramOptions::default()), None);
    }

    #[test]
    fn options_validate_would_throw_on_draw_nothing() {
        assert_eq!(wire_diagram_svg(&sequence(3), false, &with_interval(0)), None);
    }

    #[test]
    fn preserves_labels_markers_links_and_input_order() {
        let nodes = [((10, 20), 17), ((30, 30), 4), ((60, 50), 29)];
        let d = draw(&nodes, false, &WireDiagramOptions::default());

        assert_eq!(d.kinds(), ["star", "circle", "square", "wire", "wire", "label", "label", "label"]);
        assert_eq!(d.texts(), ["17", "4", "29"]);
        let wires = d.wires();
        let star = d.stars()[0];
        let middle = d.circles()[0];
        let end = d.squares()[0];
        assert!(close(star[0].0, wires[0].0 .0, 0.001));
        assert_eq!(star[0].1 + 9.0, wires[0].0 .1);
        assert_eq!((middle.0, middle.1), wires[0].1);
        assert_eq!((middle.0, middle.1), wires[1].0);
        assert_eq!(end.0 + end.2 / 2.0, wires[1].1 .0);
        assert_eq!(end.1 + end.2 / 2.0, wires[1].1 .1);
    }

    #[test]
    fn keeps_coincident_node_label_placement_finite_and_deterministic() {
        let nodes: Vec<WirePoint> = (1..=5).map(|i| ((10, 10), i)).collect();
        let first = draw(&nodes, false, &with_interval(1));
        let second = draw(&nodes, false, &with_interval(1));
        assert_eq!(first.texts(), ["1", "2", "3", "4", "5"]);
        assert_eq!(first.labels(), second.labels());
        assert!(first.labels().iter().all(|l| l.1.is_finite() && l.2.is_finite()));
        assert!(first.kinds()[9..].iter().all(|k| *k == "label"));
    }

    #[test]
    fn flip_reverses_horizontal_node_positions_without_reordering_labels() {
        let nodes = [((10, 20), 1), ((30, 30), 2), ((60, 50), 3)];
        let options = WireDiagramOptions::default();
        let (front, back) = (draw(&nodes, false, &options), draw(&nodes, true, &options));

        assert_eq!(front.canvas_width, back.canvas_width);
        let mirrored_axis = front.stars()[0][0].0 + back.stars()[0][0].0;
        assert!(close(mirrored_axis, front.circles()[0].0 + back.circles()[0].0, 0.001));
        assert!(close(mirrored_axis, front.squares()[0].0 + back.squares()[0].0 + front.squares()[0].2, 0.001));
        assert_eq!(back.texts(), ["1", "2", "3"]);
    }

    #[test]
    fn a_single_node_is_a_star_with_no_wire_or_end_marker() {
        let d = draw(&[((10, 20), 42)], false, &WireDiagramOptions::default());
        assert_eq!(d.kinds(), ["star", "label"]);
        assert_eq!(d.texts(), ["42"]);
    }

    #[test]
    fn scales_marker_dimensions_with_radius_and_not_labels() {
        let nodes = [((10, 20), 1), ((30, 30), 2), ((60, 50), 3)];
        let standard = draw(&nodes, false, &WireDiagramOptions::default());
        let doubled = draw(&nodes, false, &WireDiagramOptions { node_radius: 12.0, ..Default::default() });

        assert_eq!(standard.circles()[0].2, 6.0);
        assert_eq!(doubled.circles()[0].2, 12.0);
        assert_eq!(standard.squares()[0].2, 12.0);
        assert_eq!(doubled.squares()[0].2, 24.0);
        assert_eq!(standard.font_size, 14.0);
        assert_eq!(doubled.font_size, 14.0);
        let top = |d: &Diagram| {
            let star = d.stars()[0];
            (star[0].1 - polygon_center(&star[..]).1).abs()
        };
        assert!(close(top(&standard), 9.0, 0.001));
        assert!(close(top(&doubled), 18.0, 0.001));
    }

    #[test]
    fn filters_labels_by_sequence_position() {
        let cases: [(i32, u32, &[&str]); 5] = [
            (123, 50, &["1", "50", "100", "123"]),
            (100, 50, &["1", "50", "100"]),
            (5, 1, &["1", "2", "3", "4", "5"]),
            (5, 10, &["1", "5"]),
            (1, 50, &["1"]),
        ];
        for (count, interval, expected) in cases {
            let d = draw(&sequence(count), false, &with_interval(interval));
            assert_eq!(d.texts(), expected, "{count}, {interval}");
            let inner = (count - 2).max(0) as usize;
            let diamonds = if interval > 1 { (1..=inner).filter(|i| (i + 1) % interval as usize == 0).count() } else { 0 };
            assert_eq!(d.stars().len(), 1);
            assert_eq!(d.diamonds().len(), diamonds);
            assert_eq!(d.circles().len(), inner - diamonds);
            assert_eq!(d.squares().len(), usize::from(count > 1));
            assert_eq!(d.wires().len(), count as usize - 1);
        }
    }

    #[test]
    fn filters_repeated_nonsequential_labels_by_position() {
        let nodes = [((1, 1), 8), ((2, 2), 8), ((3, 3), 4), ((4, 4), 4), ((5, 5), 8)];
        let d = draw(&nodes, false, &with_interval(3));
        assert_eq!(d.texts(), ["8", "4", "8"]);
        assert_eq!(d.stars().len(), 1);
        assert_eq!(d.diamonds().len(), 1);
        assert_eq!(d.circles().len(), 2);
        assert_eq!(d.squares().len(), 1);
        assert_eq!(d.wires().len(), 4);
    }

    #[test]
    fn uses_diamonds_at_selected_interior_positions_in_interval_mode() {
        let d = draw(&sequence(123), false, &with_interval(50));
        assert_eq!(d.stars().len(), 1);
        assert_eq!(d.diamonds().len(), 2);
        assert_eq!(d.circles().len(), 119);
        assert_eq!(d.squares().len(), 1);
        assert_eq!(d.wires().len(), 122);
        assert_eq!(d.texts(), ["1", "50", "100", "123"]);
        let kinds = d.kinds();
        let marker_count = kinds.iter().take_while(|k| **k != "wire").count();
        assert_eq!(marker_count, 123);
        assert!(kinds[marker_count + 122..].iter().all(|k| *k == "label"));
    }

    #[test]
    fn interval_one_keeps_all_interior_nodes_as_circles() {
        let d = draw(&sequence(123), false, &with_interval(1));
        assert_eq!(d.stars().len(), 1);
        assert_eq!(d.diamonds().len(), 0);
        assert_eq!(d.circles().len(), 121);
        assert_eq!(d.squares().len(), 1);
        assert_eq!(d.wires().len(), 122);
        assert_eq!(d.texts().len(), 123);
    }

    fn five_diagonal(label: impl Fn(i32) -> i32) -> Vec<WirePoint> {
        (1..=5).map(|i| ((i * 10, i * 20), label(i))).collect()
    }

    #[test]
    fn diamond_vertices_use_the_selected_radius() {
        let d = draw(&five_diagonal(|i| i), false, &WireDiagramOptions { label_interval: 2, node_radius: 9.0, ..Default::default() });
        let diamonds = d.diamonds();
        assert_eq!(diamonds.len(), 2);
        for diamond in diamonds {
            let (x, y) = polygon_center(&diamond[..]);
            let expected = [(x, y - 9.0), (x + 9.0, y), (x, y + 9.0), (x - 9.0, y)];
            for (vertex, wanted) in diamond.iter().zip(expected) {
                assert!(close(vertex.0, wanted.0, 0.001) && close(vertex.1, wanted.1, 0.001));
            }
        }
    }

    #[test]
    fn flip_mirrors_diamond_centers_and_preserves_label_text() {
        let nodes = five_diagonal(|i| i * 3);
        let (front, back) = (draw(&nodes, false, &with_interval(2)), draw(&nodes, true, &with_interval(2)));
        let axis = polygon_center(&front.stars()[0][..]).0 + polygon_center(&back.stars()[0][..]).0;
        assert_eq!(front.diamonds().len(), 2);
        assert_eq!(back.diamonds().len(), 2);
        for (f, b) in front.diamonds().iter().zip(back.diamonds()) {
            let (f, b) = (polygon_center(&f[..]), polygon_center(&b[..]));
            assert!(close(axis, f.0 + b.0, 0.001));
            assert!(close(f.1, b.1, 0.001));
        }
        assert_eq!(front.texts(), back.texts());
    }

    #[test]
    fn selects_orientation_and_view_box_from_model_extents() {
        for (width, height, expected_width, expected_height) in [(100, 1000, 816.0, 1056.0), (1000, 100, 1056.0, 816.0), (100, 100, 816.0, 1056.0)] {
            let d = draw(&[((0, 0), 1), ((width, height), 2)], false, &WireDiagramOptions::default());
            assert_eq!((d.canvas_width, d.canvas_height), (expected_width, expected_height));
            assert_eq!(d.width_inches, expected_width / 96.0);
            assert_eq!(d.height_inches, expected_height / 96.0);
        }
    }

    #[test]
    fn applies_export_scale_only_to_the_physical_size() {
        let nodes = [((10, 20), 1), ((30, 1020), 2)];
        let standard = draw(&nodes, false, &WireDiagramOptions::default());
        let enlarged = draw(&nodes, false, &WireDiagramOptions { export_scale: 2.0, ..Default::default() });

        assert_eq!((standard.canvas_width, standard.canvas_height), (816.0, 1056.0));
        assert_eq!((enlarged.canvas_width, enlarged.canvas_height), (816.0, 1056.0));
        assert_eq!(enlarged.width_inches, standard.width_inches * 2.0);
        assert_eq!(enlarged.height_inches, standard.height_inches * 2.0);
        assert_eq!(standard.marks, enlarged.marks);

        let standard = render(&standard);
        let enlarged = render(&enlarged);
        assert!(standard.contains("width=\"8.5in\""));
        assert!(standard.contains("height=\"11in\""));
        assert!(enlarged.contains("width=\"17in\""));
        assert!(enlarged.contains("height=\"22in\""));
    }

    #[test]
    fn uses_portrait_when_orientation_scales_tie() {
        let d = draw(&[((10, 20), 42)], false, &WireDiagramOptions::default());
        assert_eq!((d.canvas_width, d.canvas_height), (816.0, 1056.0));
    }

    #[test]
    fn uses_one_uniform_scale_for_coordinate_differences() {
        let d = draw(&[((-10, -20), 1), ((10, 20), 2), ((30, 60), 3)], false, &WireDiagramOptions::default());
        let centers = d.marker_centers();
        let per_unit_x = (centers[1].0 - centers[0].0) / 20.0;
        let per_unit_y = (centers[1].1 - centers[0].1) / 40.0;
        assert!(close(per_unit_x, (centers[2].0 - centers[1].0) / 20.0, 0.001));
        assert!(close(per_unit_y, (centers[2].1 - centers[1].1) / 40.0, 0.001));
        assert!(close(per_unit_x, per_unit_y, 0.001));
    }

    #[test]
    fn handles_degenerate_and_extreme_coordinates_without_overflow() {
        let cases: [&[(i32, i32)]; 5] = [
            &[(-100, -50), (100, -50)],
            &[(-50, -100), (-50, 100)],
            &[(-10, 20)],
            &[(5, 5), (5, 5), (5, 5)],
            &[(i32::MIN, i32::MIN), (i32::MAX, i32::MAX)],
        ];
        for case in cases {
            let nodes: Vec<WirePoint> = case.iter().enumerate().map(|(i, p)| (*p, i as i32 + 1)).collect();
            let d = draw(&nodes, false, &WireDiagramOptions::default());
            let finite = |p: &(f32, f32)| p.0.is_finite() && p.1.is_finite();
            for mark in &d.marks {
                assert!(match mark {
                    Mark::Star(v) => v.iter().all(finite),
                    Mark::Diamond(v) => v.iter().all(finite),
                    Mark::Square { x, y, .. } | Mark::Circle { x, y, .. } | Mark::Label { x, y, .. } => finite(&(*x, *y)),
                    Mark::Wire { from, to } => finite(from) && finite(to),
                });
            }
        }
    }

    #[test]
    fn keeps_long_endpoint_labels_and_markers_inside_the_canvas_margins() {
        for font_size in [6.0, 14.0, 24.0] {
            let nodes = [((-1000, -1000), i32::MIN), ((1000, 1000), i32::MAX)];
            let d = draw(&nodes, false, &WireDiagramOptions { label_font_size: font_size, ..Default::default() });
            let star = d.stars()[0];
            let end = d.squares()[0];
            assert!(star.iter().all(|p| p.0 >= 23.9 && p.1 >= 23.9));
            assert!(end.0 + end.2 <= d.canvas_width - 23.9);
            assert!(end.1 + end.2 <= d.canvas_height - 23.9);
            assert_eq!(d.font_size, font_size);
            for (text, x, y) in d.labels() {
                assert!(x >= 24.0);
                assert!(x + label_width(text, font_size) <= d.canvas_width - 24.0);
                assert!(y - label_ascent(font_size) >= 24.0);
                assert!(y + label_descent(font_size) <= d.canvas_height - 24.0);
            }
            assert_eq!(d.texts(), [i32::MIN.to_string(), i32::MAX.to_string()]);
        }
    }

    #[test]
    fn flip_mirrors_marker_centers_and_preserves_label_text() {
        let nodes = [((-20, 10), 100), ((30, 20), 200), ((80, 40), 300)];
        let (front, back) = (draw(&nodes, false, &WireDiagramOptions::default()), draw(&nodes, true, &WireDiagramOptions::default()));
        assert_eq!(front.canvas_width, back.canvas_width);
        let (f, b) = (front.marker_centers(), back.marker_centers());
        for index in 0..f.len() {
            assert!(close(f[0].0 + b[0].0, f[index].0 + b[index].0, 0.01));
            assert!(close(f[index].1, b[index].1, 0.01));
        }
        assert_eq!(front.texts(), back.texts());
    }

    #[test]
    fn keeps_labels_horizontal_and_starts_text_near_the_node_when_clear() {
        let options = WireDiagramOptions { label_interval: 1, node_radius: 5.0, label_font_size: 8.0, ..Default::default() };
        let d = draw(&[((0, 0), 1)], false, &options);
        let center = polygon_center(&d.stars()[0][..]);
        let offset = d.labels()[0].1 - center.0;
        assert!((5.0..=10.0).contains(&offset), "{offset}");
        assert!(!render(&d).contains("rotate"));
    }

    #[test]
    fn separates_labels_when_nearby_nodes_have_clear_alternatives() {
        let nodes = [((0, 0), 1), ((500, 0), 2), ((501, 0), 3), ((502, 0), 4), ((1000, 0), 5)];
        let options = WireDiagramOptions { label_interval: 1, node_radius: 5.0, label_font_size: 8.0, ..Default::default() };
        let d = draw(&nodes, false, &options);
        let nearby: Vec<_> = d.labels()[1..4].to_vec();
        let separated = (0..3).any(|a| (a + 1..3).any(|b| ((nearby[a].1 - nearby[b].1).powi(2) + (nearby[a].2 - nearby[b].2).powi(2)).sqrt() > 5.0));
        assert!(separated);
    }

    // ---- The port's own: placement and text ---------------------------------------------------------------------------

    /// `PlaceLabel` with its LINQ written out, against every wire and every placed label, so `LabelPlacer` can be held to it.
    fn upstream_place_label(
        point: (f32, f32),
        width: f32,
        half_height: f32,
        points: &[(f32, f32)],
        placed: &[LabelBounds],
        radius: f32,
        canvas: (f32, f32),
    ) -> (f32, f32) {
        let half_width = width / 2.0;
        let directions = [(1f32, -1f32), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (-1.0, 1.0), (-1.0, 0.0), (-1.0, -1.0), (0.0, -1.0)];
        let mut candidates = Vec::new();
        for (x, y) in directions {
            let length = (x * x + y * y).sqrt();
            let (unit_x, unit_y) = (x / length, y / length);
            let projected = unit_x.abs() * half_width + unit_y.abs() * half_height;
            for multiplier in [1.0f32, 2.0] {
                let distance = radius + projected + 2.0 * multiplier;
                candidates.push((point.0 + unit_x * distance, point.1 + unit_y * distance));
            }
        }
        let clearance = 2.5;
        let crossings = |c: (f32, f32)| {
            points
                .windows(2)
                .filter(|w| {
                    let start = (w[0].0 - c.0, w[0].1 - c.1);
                    let end = (w[1].0 - c.0, w[1].1 - c.1);
                    segment_intersects_rectangle(start, end, half_width + clearance, half_height + clearance)
                })
                .count()
        };
        let overlap = |c: (f32, f32)| {
            let bounds = LabelBounds::new(c, half_width, half_height);
            placed.iter().fold(0f32, |sum, other| sum + bounds.overlap(other))
        };
        let distance = |c: (f32, f32)| {
            let (x, y) = (c.0 - half_width - point.0, c.1 - point.1);
            (x * x + y * y).sqrt()
        };
        let in_bounds: Vec<(f32, f32)> = candidates
            .iter()
            .copied()
            .filter(|c| c.0 - half_width >= clearance && c.0 + half_width <= canvas.0 - clearance && c.1 - half_height >= clearance && c.1 + half_height <= canvas.1 - clearance)
            .collect();
        // Stable sorts, so the candidates' own order is the last tie break.
        let by = |a: f32, b: f32| a.partial_cmp(&b).unwrap();
        if in_bounds.is_empty() {
            let mut list = candidates;
            list.sort_by(|a, b| by(distance(*a), distance(*b)));
            return list[0];
        }
        let clear: Vec<(f32, f32)> = in_bounds.iter().copied().filter(|c| crossings(*c) == 0).collect();
        let mut list = if clear.is_empty() { in_bounds } else { clear };
        list.sort_by(|a, b| crossings(*a).cmp(&crossings(*b)).then(by(overlap(*a), overlap(*b))).then(by(distance(*a), distance(*b))));
        list[0]
    }

    struct Numbers(u64);

    impl Numbers {
        /// 0 up to but not including `limit`.
        fn next(&mut self, limit: f32) -> f32 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((self.0 >> 40) as f32 / (1u64 << 24) as f32) * limit
        }
    }

    fn placements_match_upstream(points: &[(f32, f32)], width: f32, half_height: f32, radius: f32, canvas: (f32, f32)) {
        let mut placer = LabelPlacer::new(points, points.len(), radius, canvas.0, canvas.1, half_height);
        let mut placed = Vec::new();
        for (index, point) in points.iter().enumerate() {
            let expected = upstream_place_label(*point, width, half_height, points, &placed, radius, canvas);
            let actual = placer.place(*point, width);
            assert_eq!(actual, expected, "label {index} of {}", points.len());
            placed.push(LabelBounds::new(expected, width / 2.0, half_height));
        }
    }

    #[test]
    fn placement_matches_upstream_with_long_wires_everywhere() {
        let mut n = Numbers(1);
        let points: Vec<(f32, f32)> = (0..300).map(|_| (n.next(816.0), n.next(1056.0))).collect();
        placements_match_upstream(&points, 20.0, 9.8, 6.0, (816.0, 1056.0));
    }

    #[test]
    fn placement_matches_upstream_in_a_tight_cluster() {
        let mut n = Numbers(2);
        let points: Vec<(f32, f32)> = (0..300).map(|_| (400.0 + n.next(70.0), 500.0 + n.next(70.0))).collect();
        placements_match_upstream(&points, 28.0, 12.0, 3.0, (1056.0, 816.0));
    }

    #[test]
    fn placement_matches_upstream_with_short_wires_and_coincident_points() {
        let mut n = Numbers(3);
        let mut at = (408.0, 528.0);
        let mut points = Vec::new();
        for i in 0..300 {
            if i % 7 != 0 {
                at = (at.0 + n.next(40.0) - 20.0, at.1 + n.next(40.0) - 20.0);
            }
            points.push(at);
        }
        placements_match_upstream(&points, 15.0, 8.0, 6.0, (816.0, 1056.0));
    }

    #[test]
    fn placement_matches_upstream_around_cell_edges() {
        // Wires along and across the index's grid lines, and exactly through the corners of cells.
        let mut points = Vec::new();
        for i in 0..24 {
            let line = i as f32 * CELL_SIZE;
            points.extend([(line, 32.0), (line, 800.0), (800.0, line), (32.0, line)]);
        }
        placements_match_upstream(&points, 10.0, 7.0, 2.0, (1056.0, 816.0));
    }

    #[test]
    fn with_no_candidate_on_the_page_the_nearest_start_wins() {
        let mut n = Numbers(4);
        let points: Vec<(f32, f32)> = (0..30).map(|_| (n.next(30.0), n.next(30.0))).collect();
        placements_match_upstream(&points, 40.0, 20.0, 6.0, (30.0, 30.0));
    }

    #[test]
    fn the_first_label_avoids_its_own_wire() {
        let radius = 6.0;
        let font_size = 14.0;
        let d = draw(&[((0, 0), 1), ((100, 0), 2)], false, &WireDiagramOptions::default());
        let start = polygon_center(&d.stars()[0][..]);
        let width = label_width("1", font_size);
        let half_height = (label_ascent(font_size) + label_descent(font_size)) / 2.0;
        // East is on the wire, so north-east at the nearer distance is the first one clear.
        let unit = std::f32::consts::FRAC_1_SQRT_2;
        let distance = radius + unit * (width / 2.0) + unit * half_height + 2.0;
        let (text, x, _) = d.labels()[0];
        assert_eq!(text, "1");
        assert!(close(x, start.0 + unit * distance - width / 2.0, 0.01), "{x}");
        // The last one has its east free.
        let end = d.squares()[0];
        let (text, x, _) = d.labels()[1];
        assert_eq!(text, "2");
        assert!(close(x, end.0 + end.2 / 2.0 + radius + 2.0, 0.01), "{x}");
    }

    #[test]
    fn the_text_is_well_formed_and_draws_what_the_table_says() {
        let nodes = [((0, 0), 1), ((0, 10), 2), ((0, 20), 3), ((0, 30), 4)];
        let svg = wire_diagram_svg(&nodes, false, &with_interval(2)).unwrap();
        let root = XElement::parse(&svg).expect("well formed");
        assert!(root.is("svg"));
        assert_eq!(root.attr("width"), Some("8.5in"));
        assert_eq!(root.attr("height"), Some("11in"));
        assert_eq!(root.attr("viewBox"), Some("0 0 816 1056"));
        let group: Vec<&XElement> = root.elements().collect();
        assert_eq!(group.len(), 1);
        assert!(group[0].is("g"));
        let children: Vec<&XElement> = group[0].elements().collect();
        let names: Vec<String> = children.iter().map(|c| c.local.clone()).collect();
        assert_eq!(names, ["polygon", "polygon", "circle", "rect", "line", "line", "line", "text", "text", "text"]);

        // Green star of 10 vertices, then a red diamond of 4, at positions 1 and 2.
        assert_eq!(children[0].attr("fill"), Some("green"));
        assert_eq!(children[0].attr("points").unwrap().split(' ').count(), 10);
        assert_eq!(children[1].attr("fill"), Some("red"));
        assert_eq!(children[1].attr("points").unwrap().split(' ').count(), 4);
        assert_eq!(children[2].attr("fill"), Some("red"));
        assert_eq!(children[2].attr("r"), Some("6"));
        assert_eq!(children[3].attr("fill"), Some("red"));
        assert_eq!(children[3].attr("width"), Some("12"));
        assert_eq!(children[3].attr("height"), Some("12"));
        assert_eq!(children[4].attr("stroke"), Some("black"));
        assert_eq!(children[4].attr("stroke-width"), Some("1"));
        for text in &children[7..] {
            assert_eq!(text.attr("fill"), Some("black"));
            assert_eq!(text.attr("font-family"), Some("Arial"));
            assert_eq!(text.attr("font-size"), Some("14"));
            assert_eq!(text.attr("stroke"), None);
            assert_eq!(text.attr("transform"), None);
        }
        let texts: Vec<String> = children[7..].iter().map(|c| c.text()).collect();
        assert_eq!(texts, ["1", "2", "4"]);
    }

    // ---- Node order and labels: no upstream tests, so these come from reading ElementsToSvg ---------------------------------

    fn prop(data: PropertyData) -> PropertyInstance {
        let type_id = match data {
            PropertyData::Location { .. } => property_ids::LOCATION,
            _ => property_ids::ORDER,
        };
        PropertyInstance { type_id, instance_id: Uuid::new_v4(), data }
    }

    fn group(t: &mut SetupTree, parent: Option<Nid>, name: &str) -> Nid {
        t.create_node(parent, Some(name), false, true).0
    }

    /// A leaf at `(x, x)` with the Order given.
    fn leaf(t: &mut SetupTree, parent: Nid, x: i32, order: Option<i32>) -> Nid {
        let n = t.create_node(Some(parent), Some("Leaf"), true, true).0;
        t.node_mut(n).properties.push(prop(PropertyData::Location { x, y: x, z: 0 }));
        if let Some(order) = order {
            t.node_mut(n).properties.push(prop(PropertyData::Order { order }));
        }
        n
    }

    #[test]
    fn with_no_orders_points_keep_tree_order_and_count_up() {
        let mut t = SetupTree::new();
        let g = group(&mut t, None, "G");
        let inner = group(&mut t, Some(g), "Inner");
        leaf(&mut t, inner, 30, None);
        leaf(&mut t, g, 10, None);
        leaf(&mut t, inner, 20, None);
        assert_eq!(wire_diagram_points(&t, g), Ok(vec![((30, 30), 1), ((20, 20), 2), ((10, 10), 3)]));
    }

    #[test]
    fn a_leaf_with_no_location_stops_the_export() {
        let mut t = SetupTree::new();
        let g = group(&mut t, None, "G");
        leaf(&mut t, g, 10, None);
        assert!(wire_diagram_has_locations(&t, g));
        t.create_node(Some(g), Some("Bare"), true, true);
        assert!(!wire_diagram_has_locations(&t, g));
        assert_eq!(wire_diagram_points(&t, g), Err(NoLocation));
    }

    #[test]
    fn distinct_orders_sort_the_points_and_label_them() {
        let mut t = SetupTree::new();
        let g = group(&mut t, None, "G");
        leaf(&mut t, g, 10, Some(7));
        leaf(&mut t, g, 20, Some(-2));
        leaf(&mut t, g, 30, Some(5));
        assert_eq!(wire_diagram_points(&t, g), Ok(vec![((20, 20), -2), ((30, 30), 5), ((10, 10), 7)]));
    }

    #[test]
    fn repeated_orders_sort_within_each_parent_in_order_of_first_appearance() {
        let mut t = SetupTree::new();
        let g = group(&mut t, None, "G");
        let second = group(&mut t, Some(g), "Second");
        let first = group(&mut t, Some(g), "First");
        leaf(&mut t, second, 10, Some(2));
        leaf(&mut t, first, 20, Some(2));
        leaf(&mut t, second, 30, Some(1));
        leaf(&mut t, first, 40, Some(1));
        leaf(&mut t, g, 50, Some(9));
        assert_eq!(wire_diagram_points(&t, second), Ok(vec![((30, 30), 1), ((10, 10), 2)]), "distinct within one group");
        assert_eq!(
            wire_diagram_points(&t, g),
            Ok(vec![((30, 30), 1), ((10, 10), 2), ((40, 40), 1), ((20, 20), 2), ((50, 50), 9)]),
            "Second first, because its leaves come first",
        );
    }

    #[test]
    fn the_parent_that_counts_is_the_first_one_inside_the_group() {
        let mut t = SetupTree::new();
        let elsewhere = group(&mut t, None, "Elsewhere");
        let g = group(&mut t, None, "G");
        let inner = group(&mut t, Some(g), "Inner");
        let shared = leaf(&mut t, elsewhere, 10, Some(5));
        t.add_child_to_parent(shared, Some(inner), None);
        leaf(&mut t, inner, 30, Some(1));
        leaf(&mut t, g, 20, Some(1));
        // `shared` sorts with Inner's other leaf, though Elsewhere is its first parent.
        assert_eq!(wire_diagram_points(&t, g), Ok(vec![((30, 30), 1), ((10, 10), 5), ((20, 20), 1)]));
    }

    #[test]
    fn a_shared_leaf_is_one_point() {
        let mut t = SetupTree::new();
        let g = group(&mut t, None, "G");
        let a = group(&mut t, Some(g), "A");
        let b = group(&mut t, Some(g), "B");
        let shared = leaf(&mut t, a, 10, None);
        t.add_child_to_parent(shared, Some(b), None);
        leaf(&mut t, b, 20, None);
        assert_eq!(wire_diagram_points(&t, g), Ok(vec![((10, 10), 1), ((20, 20), 2)]));
    }

    #[test]
    fn when_only_some_have_an_order_it_is_tree_order_with_mixed_labels() {
        let mut t = SetupTree::new();
        let g = group(&mut t, None, "G");
        leaf(&mut t, g, 10, None);
        leaf(&mut t, g, 20, Some(40));
        leaf(&mut t, g, 30, None);
        assert_eq!(wire_diagram_points(&t, g), Ok(vec![((10, 10), 1), ((20, 20), 40), ((30, 30), 3)]));
    }

    // ---- The file ---------------------------------------------------------------------------------------------------

    #[test]
    fn writes_a_new_svg_each_time_and_nothing_for_no_points() {
        let folder = std::env::temp_dir().join(format!("vixen-wire-test-{}", Uuid::new_v4()));
        let nodes = [((0, 0), 1), ((10, 10), 2)];
        let options = WireDiagramOptions::default();
        let first = write_wire_diagram(&nodes, false, &options, &folder).unwrap().unwrap();
        let second = write_wire_diagram(&nodes, false, &options, &folder).unwrap().unwrap();
        assert_ne!(first, second);
        assert_eq!(first.extension().and_then(|e| e.to_str()), Some("svg"));
        assert_eq!(first.parent(), Some(folder.as_path()));
        assert_eq!(std::fs::read_to_string(&first).unwrap(), wire_diagram_svg(&nodes, false, &options).unwrap());
        assert_eq!(write_wire_diagram(&[], false, &options, &folder).unwrap(), None);
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// A guard against going back to testing everything against everything. Run it with `-- --ignored`.
    #[test]
    #[ignore]
    fn a_whole_display_numbered_by_default_takes_seconds() {
        let mut n = Numbers(5);
        let mut at = (5000.0f32, 5000.0f32);
        let nodes: Vec<WirePoint> = (0..50_000)
            .map(|i| {
                at = (at.0 + n.next(40.0) - 20.0, at.1 + n.next(40.0) - 20.0);
                ((at.0 as i32, at.1 as i32), i)
            })
            .collect();
        let started = std::time::Instant::now();
        let svg = wire_diagram_svg(&nodes, false, &WireDiagramOptions::default()).unwrap();
        let spent = started.elapsed();
        println!("{} points in {spent:?}, {} bytes", nodes.len(), svg.len());
        assert!(spent.as_secs_f32() < 30.0, "{spent:?}");
    }
}
