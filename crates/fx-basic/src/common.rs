//! Helpers shared by the chase-like effects (Spin, Alternating, Wipe): the `_PreRender` target selection
//! and the `CheckForInvalidColorData` rule they all copy.

use vixen_effects::prelude::*;

/// The lists of render nodes that `_PreRender` passes to `DoRendering`. `DepthOfEffect` and `TargetNodeHandling` are read
/// as stored: `TargetNodesChanged` and the `ModuleData` setter normalise them, not the render.
pub fn target_sets(ctx: &RenderCtx, individual: bool, d: i32) -> Vec<Vec<NodeId>> {
    if ctx.targets.is_empty() {
        return Vec::new();
    }
    let mut sets = Vec::new();
    if !individual {
        if ctx.targets.len() == 1 {
            sets.push(ctx.tree.nodes_at_depth(ctx.targets[0], d));
        } else {
            sets.push(ctx.targets.to_vec());
        }
    } else if ctx.targets.len() == 1 {
        for n in ctx.tree.nodes_at_depth(ctx.targets[0], d) {
            sets.push(ctx.tree.nodes_at_depth(n, d));
        }
    } else {
        for &n in ctx.targets {
            sets.push(ctx.tree.nodes_at_depth(n, d));
        }
    }
    sets
}

/// The collection a mark-driven mode reads. `BaseEffect.NormalizeMarkCollectionSelections` keeps the stored id while
/// that collection exists and otherwise selects the first collection in the sequence.
pub fn selected_mark_collection<'a>(ctx: &'a RenderCtx, id: Uuid) -> Option<&'a MarkCollection> {
    ctx.mark_collection(id).or_else(|| ctx.marks.first())
}

/// `CheckForInvalidColorData` for effects with a `StaticColor` and a `ColorGradient`.
pub fn validate_static_and_gradient(p: &mut Params, valid: &[Color]) {
    if valid.is_empty() {
        return;
    }
    let in_gradient = p.gradient("ColorGradient").colors_in_gradient();
    if !valid.contains(&p.color("StaticColor")) || !in_gradient.iter().all(|c| valid.contains(c)) {
        p.set("ColorGradient", ParamValue::Gradient(ColorGradient::solid(valid[0])));
        p.set("StaticColor", ParamValue::Color(valid[0]));
    }
}

/// `new Curve(new PointPairList({0, 100}, {level, level}))`.
pub fn level_curve(level: f64) -> Curve {
    Curve::from_points(vec![(0.0, level), (100.0, level)])
}

/// Scales every point's Y of a curve, as the discrete colour paths do.
pub fn scaled_curve(curve: &Curve, f: impl Fn(f64, f64) -> f64) -> Curve {
    let mut c = curve.clone();
    for pt in c.points.iter_mut() {
        pt.1 = f(pt.0, pt.1);
    }
    c
}

/// `Vixen.Common.Graphics.GetAdjustedFont`, with Arial standing in for the bundled DigitalDream font.
pub fn adjusted_font(text: &str, clip: (i32, i32, i32, i32), max_size: i32) -> FontDesc {
    let original = FontDesc::new("Arial", 100.0);
    let (mw, mh) = vixen_effects::gfx::measure_string(text, &original);
    let min_ratio = (clip.3 as f64 / mh as f64).min((clip.2 + 15) as f64 / mw as f64);
    let mut size = (original.size as f64 * min_ratio) as f32 - 2.0;
    if size < 10.0 {
        size = 10.0;
    } else if size > max_size as f32 {
        size = max_size as f32;
    }
    FontDesc::new("Arial", size)
}

/// The shared thumbnail of the device effects: a filled rectangle with a label drawn at the top left.
pub fn draw_device_label(g: &mut vixen_effects::gfx::Graphics, clip: (i32, i32, i32, i32), text: &str, background: Color) {
    if clip.2 <= 0 || clip.3 <= 0 {
        return;
    }
    let font = adjusted_font(text, clip, 48);
    g.fill_rectangle(background, clip.0 as f32, clip.1 as f32, clip.2 as f32, clip.3 as f32);
    // The C# draws at (clip.X + 2, 2): the Y is not offset by the clip's Y. Deliberate in the original.
    g.draw_string(text, &font, Color::WHITE, (clip.0 + 2) as f32, 2.0);
}
