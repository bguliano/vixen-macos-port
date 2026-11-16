//! Port of VixenModules.Effect.Wipe.WipeModule.

use crate::common::scaled_curve;
use crate::generated::wipe as gen;
use vixen_core::element::{property_ids, PropertyData};
use vixen_effects::prelude::*;
use vixen_effects::pulse_renderer;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    validate: Some(validate),
    targets_changed: Some(normalize_targeting),
    param_changed_ctx: Some(param_changed_ctx),
    selection: Some(selection),
    ported: true,
    ..gen::BASE
};

/// The data half of `UpdateTargetingAttributes`, which has a guard for an effect with no targets yet.
fn normalize_targeting(p: &mut Params, t: &TargetInfo) {
    if t.targets.is_empty() {
        return;
    }
    let depth = t.determine_depth();
    let handling_visible = t.targets.len() > 1 || depth > 2;
    if !handling_visible && p.enum_name("TargetNodeHandling") == "Individual" {
        p.set("TargetNodeHandling", ParamValue::Enum("Group".to_string()));
    }
    let individual = p.enum_name("TargetNodeHandling") == "Individual";
    if t.targets.len() > 1 || !individual {
        p.set("DepthOfEffect", ParamValue::Int(0));
    } else {
        // IsUsefulIntermediateDepth, GetFirstUsefulIntermediateDepth
        let selected = p.int("DepthOfEffect");
        if !(selected > 0 && selected < depth - 1) {
            p.set("DepthOfEffect", ParamValue::Int(if depth > 2 { 1 } else { 0 }));
        }
    }
}

/// `WipeTargetElementDepthConverter`: only the depths between the target and its leaves.
fn selection(name: &str, _row: Option<&Params>, _p: &Params, t: &TargetInfo) -> Option<Vec<SelectionValue>> {
    (name == "DepthOfEffect").then(|| depth_choices(1, t.targets.first().map_or(0, |n| t.tree.max_child_depth(*n)) - 1))
}

/// `UpdateAttributes` runs `UpdateTargetingAttributes` from the `ModuleData` setter and from these property setters.
fn param_changed_ctx(c: &ParamChange, p: &mut Params) {
    if matches!(
        c.name,
        MODULE_DATA | "ColorHandling" | "Direction" | "ReverseDirection" | "MovementCurve" | "WipeMovement" | "TargetNodeHandling" | "DepthOfEffect"
    ) {
        normalize_targeting(p, c.info);
    }
}

/// A C# NullReferenceException (`result.FirstOrDefault().Value` on an empty result) aborts the render. The intents
/// added before it stay.
struct Abort;

#[derive(Copy, Clone, PartialEq)]
enum Dir {
    Vertical,
    Horizontal,
    DiagonalUp,
    DiagonalDown,
    Burst,
    Circle,
    Diamond,
}

#[derive(Copy, Clone, PartialEq)]
enum Movement {
    Count,
    PulseLength,
    Movement,
}

/// `Tuple<IElementNode, int, int, int>`.
#[derive(Copy, Clone)]
struct Located {
    node: NodeId,
    x: i32,
    y: i32,
}

struct Wipe<'a> {
    ctx: &'a RenderCtx<'a>,
    has_discrete: bool,
    gradient: ColorGradient,
    curve: Curve,
    movement_curve: Curve,
    dir: Dir,
    movement: Movement,
    color_across: bool,
    color_per_count: bool,
    reverse_color: bool,
    reverse_dir: bool,
    pulse_time: i32,
    pass_count: i32,
    pulse_percent: f64,
    wipe_on: bool,
    wipe_off: bool,
    x_offset: f64,
    y_offset: f64,
    out: EffectIntents,
    max_x: i32,
    min_x: i32,
    max_y: i32,
    min_y: i32,
    mid_x: i32,
    mid_y: i32,
    width: i32,
    height: i32,
    pulse_pct: i32,
    steps: i32,
}

fn visibility(p: &Params, t: &TargetInfo, v: &mut Visibility) {
    let m = p.enum_name("WipeMovement");
    let across = p.enum_name("ColorHandling") == "ColorAcrossItems";
    v.set("PassCount", m == "Count");
    v.set("PulsePercent", m != "PulseLength");
    v.set("WipeOn", m == "Count");
    v.set("WipeOff", m == "Count");
    v.set("PulseTime", m == "PulseLength");
    v.set("ColorHandling", m != "Movement");
    v.set("MovementCurve", m == "Movement");
    v.set("ReverseDirection", m != "Movement");
    v.set("ColorAcrossItemPerCount", across && m != "Movement");
    v.set("ReverseColorDirection", m == "Movement");
    let burst_like = matches!(p.enum_name("Direction"), "Burst" | "Circle" | "Dimaond");
    v.set("XOffset", burst_like);
    v.set("YOffset", burst_like);
    // UpdateTargetingAttributes
    let depth = t.determine_depth();
    let individual = p.enum_name("TargetNodeHandling") == "Individual";
    v.set("TargetNodeHandling", !t.targets.is_empty() && (t.targets.len() > 1 || depth > 2));
    v.set("DepthOfEffect", !t.targets.is_empty() && individual && t.targets.len() == 1 && depth > 2);
}

/// `CheckForInvalidColorData`.
fn validate(p: &mut Params, valid: &[Color]) {
    if valid.is_empty() {
        return;
    }
    if !p.gradient("ColorGradient").colors_in_gradient().iter().all(|c| valid.contains(c)) {
        p.set("ColorGradient", ParamValue::Gradient(ColorGradient::solid(valid[0])));
    }
}

fn location_of(ctx: &RenderCtx, leaf: NodeId) -> Option<(i32, i32, i32)> {
    ctx.tree.property(leaf, &property_ids::LOCATION).and_then(|p| match p.data {
        PropertyData::Location { x, y, z } => Some((x, y, z)),
        _ => None,
    })
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    if ctx.targets.is_empty() {
        return EffectIntents::new();
    }
    let dir = match p.enum_name("Direction") {
        "Vertical" => Dir::Vertical,
        "DiagonalUp" => Dir::DiagonalUp,
        "DiagonalDown" => Dir::DiagonalDown,
        "Burst" => Dir::Burst,
        "Circle" => Dir::Circle,
        "Dimaond" => Dir::Diamond,
        _ => Dir::Horizontal,
    };
    let movement = match p.enum_name("WipeMovement") {
        "PulseLength" => Movement::PulseLength,
        "Movement" => Movement::Movement,
        _ => Movement::Count,
    };
    let mut w = Wipe {
        ctx,
        has_discrete: ctx.has_discrete_colors(),
        gradient: p.gradient("ColorGradient"),
        curve: p.curve("Curve"),
        movement_curve: p.curve("MovementCurve"),
        dir,
        movement,
        color_across: p.enum_name("ColorHandling") == "ColorAcrossItems",
        color_per_count: p.boolean("ColorAcrossItemPerCount"),
        reverse_color: p.boolean("ReverseColorDirection"),
        reverse_dir: p.boolean("ReverseDirection"),
        pulse_time: p.int("PulseTime"),
        pass_count: p.int("PassCount"),
        pulse_percent: p.double("PulsePercent"),
        wipe_on: p.boolean("WipeOn"),
        wipe_off: p.boolean("WipeOff"),
        x_offset: p.double("XOffset"),
        y_offset: p.double("YOffset"),
        out: EffectIntents::new(),
        max_x: 0,
        min_x: 0,
        max_y: 0,
        min_y: 0,
        mid_x: 0,
        mid_y: 0,
        width: 0,
        height: 0,
        pulse_pct: 0,
        steps: 0,
    };

    // GetTargetRenderGroups. The handling and depth were normalised when the targets or the data were assigned.
    let individual = p.enum_name("TargetNodeHandling") == "Individual";
    let depth_of_effect = p.int("DepthOfEffect");

    let groups: Vec<Vec<NodeId>> = if !individual {
        vec![ctx.targets.to_vec()]
    } else if ctx.targets.len() == 1 {
        ctx.tree.nodes_at_depth(ctx.targets[0], depth_of_effect).into_iter().map(|n| vec![n]).collect()
    } else {
        ctx.targets.iter().map(|&n| vec![n]).collect()
    };
    for g in groups {
        if ctx.cancelled() {
            return EffectIntents::new();
        }
        if w.render_for_targets(&g).is_err() {
            // The exception ends the render, but `_elementData` keeps what was added so far and that is what plays.
            break;
        }
    }
    w.out
}

impl Wipe<'_> {
    fn render_for_targets(&mut self, targets: &[NodeId]) -> Result<(), Abort> {
        let ctx = self.ctx;
        let mut located: Vec<Located> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for &t in targets {
            for leaf in ctx.tree.leaves(t) {
                if let Some((x, y, _z)) = location_of(ctx, leaf) {
                    if x > 0 && seen.insert(leaf) {
                        located.push(Located { node: leaf, x, y });
                    }
                }
            }
        }
        if located.is_empty() {
            return Ok(());
        }
        self.max_x = located.iter().map(|l| l.x).max().unwrap_or(0);
        self.max_y = located.iter().map(|l| l.y).max().unwrap_or(0);
        self.min_x = located.iter().map(|l| l.x).min().unwrap_or(0);
        self.min_y = located.iter().map(|l| l.y).min().unwrap_or(0);
        self.width = self.max_x - self.min_x;
        self.height = self.max_y - self.min_y;
        self.mid_x = self.width / 2;
        self.mid_y = self.height / 2;

        let render_nodes = match self.dir {
            Dir::DiagonalUp | Dir::DiagonalDown => self.diagonal(&located),
            Dir::Vertical | Dir::Horizontal => self.lrud(&located),
            Dir::Circle => self.circle(&located),
            Dir::Diamond => self.diamond(&located),
            Dir::Burst => self.rectangle(&located),
        };
        match self.movement {
            Movement::Count => self.render_count(&render_nodes),
            Movement::PulseLength => self.render_pulse_length(&render_nodes),
            Movement::Movement => {
                self.render_movement(&render_nodes);
                Ok(())
            }
        }
    }

    fn pulse_percent_int(&self, extent: i32) -> i32 {
        (extent as f64 * (self.pulse_percent / 100.0)) as i32
    }

    /// `OrderBy` ascending when not reversed, otherwise `OrderByDescending`; keys are unique and already ascending.
    fn order(&self, mut groups: Vec<Vec<NodeId>>) -> Vec<Vec<NodeId>> {
        if self.reverse_dir && self.movement != Movement::Movement {
            groups.reverse();
        }
        groups
    }

    fn dist(a: (i32, i32), b: (i32, i32)) -> f64 {
        distance_from_point(a.0, a.1, b.0, b.1)
    }

    fn max_steps(&self, c: (i32, i32)) -> i32 {
        let mut steps = Self::dist((self.max_x, self.max_y), c) as i32;
        steps = (Self::dist((self.max_x, self.min_y), c) as i32).max(steps);
        steps = (Self::dist((self.min_x, self.min_y), c) as i32).max(steps);
        (Self::dist((self.min_x, self.max_y), c) as i32).max(steps)
    }

    fn circle(&mut self, nodes: &[Located]) -> Vec<Vec<NodeId>> {
        self.steps = (Self::dist((self.max_x, self.max_y), (self.min_x, self.min_y)) / 2.0) as i32;
        let center = (
            ((self.x_offset + 100.0) / 2.0) as i32 * self.width / 100 + self.min_x,
            (100 - ((self.y_offset + 100.0) / 2.0) as i32) * self.height / 100 + self.min_y,
        );
        let mut steps = self.max_steps(center);
        self.pulse_pct = self.pulse_percent_int(self.width);
        if self.movement == Movement::Movement {
            steps += self.pulse_pct;
        }
        let mut groups = Vec::new();
        for i in 0..=steps {
            let g: Vec<NodeId> = nodes.iter().filter(|n| Self::dist(center, (n.x, n.y)) as i32 == i).map(|n| n.node).collect();
            groups.push(g);
        }
        self.order(groups)
    }

    fn diamond(&mut self, nodes: &[Located]) -> Vec<Vec<NodeId>> {
        let (w, h) = (self.width, self.height);
        self.steps = (((w as f64).powi(2) + (h as f64).powi(2)).sqrt() / 1.5) as i32;
        let xo = ((self.x_offset + 100.0) / 2.0) as i32;
        let yo = ((self.y_offset + 100.0) / 2.0) as i32;
        let center = (xo * w / 100 + self.min_x, (100 - yo) * h / 100 + self.min_y);
        let mut steps = (self.max_steps(center) as f64 * 1.41) as i32;
        // Double division before the truncating cast, unlike Burst.
        let xo = (scale_curve_to_value(xo as f64, -w as f64, w as f64).round_ties_even() / 2.0) as i32;
        let yo = (scale_curve_to_value(yo as f64, h as f64, -h as f64).round_ties_even() / 2.0) as i32;
        self.pulse_pct = self.pulse_percent_int(w);
        if self.movement == Movement::Movement {
            steps += self.pulse_pct;
        }
        let (min_x, min_y, max_x, max_y, mid_x, mid_y) = (self.min_x, self.min_y, self.max_x, self.max_y, self.mid_x, self.mid_y);
        let mut groups = Vec::new();
        for i in 0..=steps {
            let mut g = Vec::new();
            for n in nodes {
                let mut loc = (n.y - min_y + yo) - (n.x - min_x + xo) + (w - h) / 2;
                if loc < 0 {
                    loc = -loc;
                }
                if loc == i
                    && ((max_y - mid_y - yo - n.y) <= i && (max_x - mid_x - xo - n.x) <= i)
                    && (n.y - min_y - mid_y + yo) <= i
                    && (n.x - min_x - mid_x + xo) <= i
                {
                    g.push(n.node);
                }
                let mut loc = (n.x + xo - min_x + n.y + yo - min_y) - (w + h) / 2;
                if loc < 0 {
                    loc = -loc;
                }
                if loc == i
                    && ((max_y - mid_y - n.y - yo) <= i && (max_x - mid_x - n.x - xo) <= i)
                    && (n.y + yo - min_y - mid_y) <= i
                    && (n.x + xo - min_x - mid_x) <= i
                {
                    g.push(n.node);
                }
            }
            groups.push(g);
        }
        self.order(groups)
    }

    fn rectangle(&mut self, nodes: &[Located]) -> Vec<Vec<NodeId>> {
        let (w, h) = (self.width, self.height);
        self.steps = w.max(h) / 2;
        let xo = ((self.x_offset + 100.0) / 2.0) as i32;
        let yo = ((self.y_offset + 100.0) / 2.0) as i32;
        let center = (xo * w / 100 + self.min_x, yo * h / 100 + self.min_y);
        let mut steps = self.max_steps(center);
        // Integer division after the cast here.
        let xo = (scale_curve_to_value(xo as f64, -w as f64, w as f64).round_ties_even() as i32) / 2;
        let yo = (scale_curve_to_value(yo as f64, h as f64, -h as f64).round_ties_even() as i32) / 2;
        self.pulse_pct = self.pulse_percent_int(w);
        if self.movement == Movement::Movement {
            steps += self.pulse_pct;
        }
        let (max_x, max_y, mid_x, mid_y) = (self.max_x, self.max_y, self.mid_x, self.mid_y);
        let mut groups = Vec::new();
        for i in 0..=steps {
            let mut g = Vec::new();
            for n in nodes {
                let dy = max_y - mid_y - n.y - yo;
                let dx = max_x - mid_x - n.x - xo;
                if dy <= i && dy >= -i && (dx == i || dx == -i) {
                    g.push(n.node);
                }
                if dx <= i && dx >= -i && (dy == i || dy == -i) {
                    g.push(n.node);
                }
            }
            groups.push(g);
        }
        self.order(groups)
    }

    fn diagonal(&mut self, nodes: &[Located]) -> Vec<Vec<NodeId>> {
        let (w, h) = (self.width, self.height);
        self.steps = (((w as f64).powi(2) + (h as f64).powi(2)).sqrt() * 1.41) as i32;
        self.pulse_pct = self.pulse_percent_int(w);
        if self.movement == Movement::Movement {
            self.steps += self.pulse_pct;
        }
        let rev = self.reverse_dir || self.movement == Movement::Movement;
        let mut groups = Vec::new();
        for i in 0..=self.steps {
            let mut g = Vec::new();
            for n in nodes {
                let (dx, dy) = (n.x - self.min_x, n.y - self.min_y);
                let hit = if rev {
                    if self.dir == Dir::DiagonalUp {
                        dx + dy == self.steps - i
                    } else {
                        dy - dx + w == i
                    }
                } else if self.dir == Dir::DiagonalUp {
                    dx - dy + h == i
                } else {
                    dx + dy == i
                };
                if hit {
                    g.push(n.node);
                }
            }
            groups.push(g);
        }
        groups
    }

    fn lrud(&mut self, nodes: &[Located]) -> Vec<Vec<NodeId>> {
        let vertical = self.dir == Dir::Vertical;
        self.pulse_pct = self.pulse_percent_int(if vertical { self.height } else { self.width });
        self.steps = if vertical { self.height } else { self.width };
        if self.movement == Movement::Movement {
            self.steps += self.pulse_pct;
        }
        let mut groups = Vec::new();
        for i in 0..=self.steps {
            let g: Vec<NodeId> = nodes
                .iter()
                .filter(|n| if vertical { self.height - (n.y - self.min_y) == i } else { self.width - (n.x - self.min_x) == i })
                .map(|n| n.node)
                .collect();
            groups.push(g);
        }
        // Unlike the other shapes, ascending here is the reversed / movement case.
        if !(self.reverse_dir || self.movement == Movement::Movement) {
            groups.reverse();
        }
        groups
    }

    fn position_in_group(&self, effect_time: TimeSpan, segment: TimeSpan) -> f64 {
        let mut pos = 1.0 / (self.ctx.time_span.ticks() - segment.ticks()) as f64 * effect_time.ticks() as f64;
        if self.color_per_count {
            pos = pos * self.pass_count as f64 % 1.0;
        }
        if pos.is_nan() {
            0.0
        } else {
            pos
        }
    }

    fn intents_of(&self, node: NodeId, result: &EffectIntents) -> Option<Vec<IntentNode>> {
        let e = self.ctx.tree.node(node).element_id?;
        result.nodes_for(&e).cloned()
    }

    fn add_start(&mut self, node: NodeId, n: &IntentNode, g: Option<&ColorGradient>) {
        let r = pulse_renderer::starting_static_pulse(self.ctx.tree, node, Some(n), self.has_discrete, g);
        self.out.add(r);
    }

    fn add_end(&mut self, node: NodeId, n: &IntentNode, g: Option<&ColorGradient>) {
        let r = pulse_renderer::extended_static_pulse(self.ctx.tree, node, Some(n), self.ctx.time_span, self.has_discrete, g);
        self.out.add(r);
    }

    fn render_pulse_length(&mut self, render_nodes: &[Vec<NodeId>]) -> Result<(), Abort> {
        let ctx = self.ctx;
        let tree = ctx.tree;
        let hd = self.has_discrete;
        if render_nodes.is_empty() {
            return Ok(());
        }
        let ts = ctx.time_span;
        let mut effect_time = TimeSpan::ZERO;
        let intervals = self.pulse_time as f64 / render_nodes.len() as f64;
        let mut interval_time = TimeSpan::from_ms(intervals);
        // The C# widens a tiny interval to 5 ms to bound render time.
        interval_time = TimeSpan::from_ms(interval_time.total_ms().max(5.0));
        let segment = TimeSpan::from_ms(self.pulse_time as f64);
        while effect_time < ts {
            for item in render_nodes {
                if ctx.cancelled() {
                    return Err(Abort);
                }
                for &element in item {
                    if !self.color_across {
                        let mut r = pulse_renderer::render_node(tree, element, &self.curve, &self.gradient, segment, hd, false);
                        r.offset_all(effect_time);
                        self.out.add(r);
                    } else {
                        let pos = self.position_in_group(effect_time, segment);
                        if hd {
                            for (color, prop) in self.gradient.get_discrete_colors_and_proportions_at(pos) {
                                let curve = scaled_curve(&self.curve, |_, y| y * prop as f64);
                                let mut r = pulse_renderer::render_node(tree, element, &curve, &ColorGradient::solid(color), segment, hd, false);
                                r.offset_all(effect_time);
                                // The C# never adds this result to the output; ported as written.
                            }
                        } else {
                            let g = ColorGradient::solid(self.gradient.get_color_at(pos));
                            let mut r = pulse_renderer::render_node(tree, element, &self.curve, &g, segment, hd, false);
                            r.offset_all(effect_time);
                            self.out.add(r);
                        }
                    }
                }
                effect_time += interval_time;
                if effect_time >= ts {
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn render_count(&mut self, render_nodes: &[Vec<NodeId>]) -> Result<(), Abort> {
        let ctx = self.ctx;
        let tree = ctx.tree;
        let hd = self.has_discrete;
        let n = render_nodes.len() as i32;
        // PassCount 0 divides by zero in C#.
        if n == 0 || self.pass_count <= 0 {
            return Ok(());
        }
        let ts = ctx.time_span;
        let pass = self.pass_count;
        let mut effect_time = TimeSpan::ZERO;
        let pulse_segment = (ts.ticks() as f64 * ((self.pulse_percent * (self.steps as f64 / n as f64)) / 100.0)) / pass as f64;
        let interval_time = TimeSpan::from_ticks(((ts.ticks() as f64 - pulse_segment) / (n * pass) as f64) as i64);
        let segment = TimeSpan::from_ticks(pulse_segment as i64);

        let mut count = 0;
        while count < pass {
            for item in render_nodes {
                if ctx.cancelled() {
                    return Err(Abort);
                }
                for &element in item {
                    if !self.color_across {
                        let mut r = pulse_renderer::render_node(tree, element, &self.curve, &self.gradient, segment, hd, false);
                        r.offset_all(effect_time);
                        let first = self.intents_of(element, &r);
                        if self.wipe_off && count == 0 && !r.is_empty() {
                            for i in first.iter().flatten() {
                                self.add_start(element, i, None);
                            }
                        }
                        if self.wipe_on && !r.is_empty() && count == pass - 1 {
                            for i in first.iter().flatten() {
                                self.add_end(element, i, None);
                            }
                        }
                        self.out.add(r);
                    } else {
                        let pos = self.position_in_group(effect_time, segment);
                        if hd {
                            for (color, prop) in self.gradient.get_discrete_colors_and_proportions_at(pos) {
                                let curve = scaled_curve(&self.curve, |_, y| y * prop as f64);
                                let g = ColorGradient::solid(color);
                                let mut r = pulse_renderer::render_node(tree, element, &curve, &g, segment, hd, false);
                                r.offset_all(effect_time);
                                let first = self.intents_of(element, &r);
                                if self.wipe_off && count == 0 {
                                    let f = first.as_ref().ok_or(Abort)?;
                                    for i in f {
                                        self.add_start(element, i, Some(&g));
                                    }
                                }
                                if !r.is_empty() {
                                    self.out.add(r);
                                }
                                if self.wipe_on && count == pass - 1 {
                                    let f = first.as_ref().ok_or(Abort)?;
                                    for i in f {
                                        self.add_end(element, i, Some(&g));
                                    }
                                }
                            }
                        } else {
                            let g = ColorGradient::solid(self.gradient.get_color_at(pos));
                            let mut r = pulse_renderer::render_node(tree, element, &self.curve, &g, segment, hd, false);
                            r.offset_all(effect_time);
                            let first = self.intents_of(element, &r);
                            if self.wipe_off && count == 0 {
                                for i in first.as_ref().ok_or(Abort)? {
                                    self.add_start(element, i, None);
                                }
                            }
                            if self.wipe_on && count == pass - 1 {
                                for i in first.as_ref().ok_or(Abort)? {
                                    self.add_end(element, i, None);
                                }
                            }
                            self.out.add(r);
                        }
                    }
                }
                effect_time += interval_time;
            }
            count += 1;
        }
        Ok(())
    }

    fn render_movement(&mut self, render_nodes: &[Vec<NodeId>]) {
        let ctx = self.ctx;
        let ts = ctx.time_span;
        const TIME_INTERVAL: f64 = 50.0;
        struct Wc {
            index: i32,
            start: TimeSpan,
            duration: TimeSpan,
            reverse_color: i32,
        }
        let mut previous = 2.0;
        let mut start_time = TimeSpan::ZERO;
        let interval = TimeSpan::from_ms(TIME_INTERVAL);
        let intervals = (ts.total_ms() / TIME_INTERVAL).ceil().round_ties_even() as i32;
        let burst = if self.dir != Dir::DiagonalUp { 0 } else { self.pulse_pct - 1 };
        let mut elements: Vec<Wc> = Vec::new();
        for i in 0..intervals {
            let position = 100.0 / intervals as f64 * i as f64;
            let movement = self.movement_curve.get_value(position) / 100.0;
            if previous != movement {
                if let Some(last) = elements.last_mut() {
                    last.duration = start_time - last.start;
                }
                let mut wc = Wc {
                    index: ((render_nodes.len() as i32 - 1) as f64 * movement) as i32,
                    start: start_time,
                    duration: ts - start_time,
                    reverse_color: 0,
                };
                if self.reverse_color {
                    wc.reverse_color = if previous < movement { 0 } else { 1 };
                }
                elements.push(wc);
            }
            previous = movement;
            start_time += interval;
        }
        let pos = (100.0 / self.pulse_pct as f64) / 100.0;
        for wc in &elements {
            for i in 0..self.pulse_pct {
                let mut position = wc.reverse_color as f64 - pos * i as f64;
                if position < 0.0 {
                    position = -position;
                }
                let color = self.gradient.get_color_at(position);
                let curve_value = self.curve.get_value(position * 100.0) / 100.0;
                let idx = wc.index - i;
                if idx >= 0 && idx + burst < render_nodes.len() as i32 && idx + burst >= 0 {
                    if ctx.cancelled() {
                        return;
                    }
                    for &item in &render_nodes[(idx + burst) as usize] {
                        let mut r = pulse_renderer::render_node_static(ctx.tree, item, curve_value, color, wc.duration);
                        r.offset_all(wc.start);
                        self.out.add(r);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::element::PropertyInstance;
    use vixen_core::Uuid;
    use vixen_effects::testing;

    #[test]
    fn targeting_is_normalised_on_load_and_on_assignment() {
        let mut tree = ElementTree::new();
        let string = tree.add_string("S", 4);
        let second = tree.add_string("S2", 4);
        let deep = testing::deep_grid(&mut tree, "D", 2, 2, 3);
        let individual = |depth: i32| {
            let mut p = DESCRIPTOR.new_params();
            p.set("TargetNodeHandling", ParamValue::Enum("Individual".into()));
            p.set("DepthOfEffect", ParamValue::Int(depth));
            p
        };
        // Shallow: nothing to choose between, on load as well as on assignment.
        let t = testing::target_info(&tree, std::slice::from_ref(&string));
        let mut p = individual(3);
        DESCRIPTOR.load_data(&mut p, &t);
        assert_eq!((p.enum_name("TargetNodeHandling"), p.int("DepthOfEffect")), ("Group", 0));
        // Deep with a useless depth: the first useful one. A useful depth stays.
        let deep_targets = [deep];
        let t = testing::target_info(&tree, &deep_targets);
        let mut p = individual(3);
        DESCRIPTOR.set_targets(&mut p, &t);
        assert_eq!((p.enum_name("TargetNodeHandling"), p.int("DepthOfEffect")), ("Individual", 1));
        let mut p = individual(2);
        DESCRIPTOR.load_data(&mut p, &t);
        assert_eq!(p.int("DepthOfEffect"), 2);
        // Several targets: handling stays, depth doesn't.
        let two = [deep, second];
        let t = testing::target_info(&tree, &two);
        let mut p = individual(2);
        DESCRIPTOR.set_targets(&mut p, &t);
        assert_eq!((p.enum_name("TargetNodeHandling"), p.int("DepthOfEffect")), ("Individual", 0));
        // A property setter runs the same rule, and with no targets it does nothing.
        let mut p = individual(3);
        DESCRIPTOR.set_param(&mut p, "ColorHandling", ParamValue::Enum("ColorAcrossItems".into()), &TargetInfo::detached());
        assert_eq!((p.enum_name("TargetNodeHandling"), p.int("DepthOfEffect")), ("Individual", 3));
        let t = testing::target_info(&tree, &deep_targets);
        DESCRIPTOR.set_param(&mut p, "ColorHandling", ParamValue::Enum("ColorAcrossItems".into()), &t);
        assert_eq!(p.int("DepthOfEffect"), 1);
    }

    #[test]
    fn the_depth_drop_down_offers_the_levels_between_target_and_leaves() {
        let mut tree = ElementTree::new();
        let deep = testing::deep_grid(&mut tree, "D", 2, 2, 3);
        let targets = [deep];
        let t = testing::target_info(&tree, &targets);
        let p = DESCRIPTOR.new_params();
        let labels: Vec<String> = DESCRIPTOR.selection_values("DepthOfEffect", None, &p, &t).into_iter().map(|c| c.label).collect();
        assert_eq!(labels, ["1", "2"]);
        assert!(DESCRIPTOR.selection_values("DepthOfEffect", None, &p, &TargetInfo::detached()).is_empty());
    }

    #[test]
    fn targets_decide_what_the_editor_offers() {
        let mut tree = ElementTree::new();
        let string = tree.add_string("S", 4);
        let second = tree.add_string("S2", 4);
        let deep = testing::deep_grid(&mut tree, "D", 2, 2, 3);
        let mut p = DESCRIPTOR.new_params();
        let names = testing::visible_names(&DESCRIPTOR, &p, &TargetInfo::detached());
        assert!(!names.contains(&"TargetNodeHandling") && !names.contains(&"DepthOfEffect"));
        let names = testing::visible_names(&DESCRIPTOR, &p, &testing::target_info(&tree, &[string]));
        assert!(!names.contains(&"TargetNodeHandling") && !names.contains(&"DepthOfEffect"));
        let names = testing::visible_names(&DESCRIPTOR, &p, &testing::target_info(&tree, &[string, second]));
        assert!(names.contains(&"TargetNodeHandling") && !names.contains(&"DepthOfEffect"));
        let names = testing::visible_names(&DESCRIPTOR, &p, &testing::target_info(&tree, &[deep]));
        assert!(names.contains(&"TargetNodeHandling") && !names.contains(&"DepthOfEffect"));
        p.set("TargetNodeHandling", ParamValue::Enum("Individual".into()));
        let names = testing::visible_names(&DESCRIPTOR, &p, &testing::target_info(&tree, &[deep]));
        assert!(names.contains(&"TargetNodeHandling") && names.contains(&"DepthOfEffect"));
    }

    /// A row of `n` single-pixel elements at x = 10, 20, ... on one line, under one group.
    fn row(tree: &mut ElementTree, n: usize) -> (NodeId, Vec<NodeId>) {
        let g = tree.add_node(Uuid::new_v4(), "G", None);
        let mut leaves = Vec::new();
        for i in 0..n {
            let e = tree.add_element(&format!("E{i}"));
            tree.node_mut(e).properties.push(PropertyInstance {
                type_id: property_ids::LOCATION,
                instance_id: Uuid::new_v4(),
                data: PropertyData::Location { x: (i as i32 + 1) * 10, y: 5 + i as i32 * 7, z: 0 },
            });
            tree.add_child(g, e);
            leaves.push(e);
        }
        (g, leaves)
    }

    fn first_start(out: &EffectIntents, tree: &ElementTree, n: NodeId) -> Option<f64> {
        out.nodes_for(&tree.node(n).element_id.unwrap()).map(|v| v[0].start.total_ms())
    }

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
    }

    #[test]
    fn located_elements_produce_light_and_unlocated_do_not() {
        let mut tree = ElementTree::new();
        let (g, _) = row(&mut tree, 5);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[g], 2000, |_| {});
        assert_eq!(out.map.len(), 5);
        // The C# only wipes elements with a Location property whose X is above zero.
        let plain = tree.add_string("S", 4);
        assert!(testing::render_with(&DESCRIPTOR, &tree, &[plain], 2000, |_| {}).is_empty());
    }

    #[test]
    fn horizontal_wipe_moves_left_to_right_by_default_and_reverses() {
        let mut tree = ElementTree::new();
        let (g, leaves) = row(&mut tree, 4);
        // Horizontal, not reversed: groups are keyed by (width - x) and ordered descending, so the smallest x is first.
        let out = testing::render_with(&DESCRIPTOR, &tree, &[g], 2000, |_| {});
        let starts: Vec<f64> = leaves.iter().map(|l| first_start(&out, &tree, *l).unwrap()).collect();
        assert!(starts[0] < starts[3], "{starts:?}");
        let out = testing::render_with(&DESCRIPTOR, &tree, &[g], 2000, |p| {
            p.set("ReverseDirection", ParamValue::Bool(true));
        });
        let starts: Vec<f64> = leaves.iter().map(|l| first_start(&out, &tree, *l).unwrap()).collect();
        assert!(starts[0] > starts[3], "{starts:?}");
    }

    #[test]
    fn every_direction_and_movement_renders() {
        let mut tree = ElementTree::new();
        let (g, _) = row(&mut tree, 6);
        for dir in ["Vertical", "Horizontal", "DiagonalUp", "DiagonalDown", "Burst", "Circle", "Dimaond"] {
            for mv in ["Count", "PulseLength", "Movement"] {
                let out = testing::render_with(&DESCRIPTOR, &tree, &[g], 1500, |p| {
                    p.set("Direction", ParamValue::Enum(dir.into()));
                    p.set("WipeMovement", ParamValue::Enum(mv.into()));
                });
                assert!(!out.is_empty(), "{dir} {mv}");
            }
        }
    }

    #[test]
    fn visibility_burst_shows_offsets() {
        let mut p = DESCRIPTOR.new_params();
        let mut v = Visibility::default();
        visibility(&p, &TargetInfo::detached(), &mut v);
        assert!(v.overrides.contains(&("XOffset", false)));
        p.set("Direction", ParamValue::Enum("Circle".into()));
        let mut v = Visibility::default();
        visibility(&p, &TargetInfo::detached(), &mut v);
        assert!(v.overrides.contains(&("XOffset", true)));
    }
}
