//! The picture on an effect's block: `EffectRasterizer.Rasterize`, which either rasterizes the intents or asks the effect to draw a custom
//! thumbnail. Everything here runs on a background thread, away from the session's lock.

use super::ElementId;
use crate::sequence::SequenceSession;
use crate::thumbnail::{rasterize, Pixels, ThumbnailRequest};
use std::sync::Arc;
use uuid::Uuid;
use vixen_core::{EffectIntents, ElementTree, MarkCollection, NodeId, TimeSpan};
use vixen_effects::framework::{EffectDescriptor, Params, TargetInfo};
use vixen_effects::gfx::Graphics;

/// A copy of what's needed to draw one effect's picture.
pub struct PictureSource {
    pub intents: Option<Arc<EffectIntents>>,
    /// `effect.TargetNodes.GetElements()`.
    pub elements: Arc<Vec<Uuid>>,
    pub span: TimeSpan,
    pub start: TimeSpan,
    /// `ForceGenerateVisualRepresentation`: the effect draws its own picture.
    pub visual: Option<VisualSource>,
    pub picture: u64,
}

/// For an effect with a custom thumbnail, what its draw function reads.
pub struct VisualSource {
    pub descriptor: &'static EffectDescriptor,
    pub params: Params,
    pub targets: Vec<NodeId>,
    pub tree: Arc<ElementTree>,
    pub marks: Vec<MarkCollection>,
    pub audio_loaded: bool,
}

/// Takes what a picture needs while holding the session, briefly. `elements_of` lets the caller cache what can be slow for a big group.
pub fn picture_source(session: &SequenceSession, id: ElementId, elements_of: &mut dyn FnMut(NodeId) -> Arc<Vec<Uuid>>) -> Option<PictureSource> {
    let entry = session.effect(id)?;
    let elements = entry.targets.first().map_or_else(|| Arc::new(Vec::new()), |t| elements_of(*t));
    let visual = entry.descriptor.filter(|d| d.visual.is_some() && d.forces_visual(&entry.node.params)).map(|descriptor| VisualSource {
        descriptor,
        params: entry.node.params.clone(),
        targets: entry.targets.clone(),
        tree: Arc::clone(session.tree()),
        marks: session.marks().to_vec(),
        audio_loaded: session.audio().is_some(),
    });
    Some(PictureSource {
        intents: entry.intents.clone(),
        elements,
        span: entry.node.time_span,
        start: entry.node.start,
        visual,
        picture: entry.rendered_generation,
    })
}

/// One picture, or a slice of it. `overall_width` is the whole effect's width in pixels at this zoom; `slice_x` and `width` say which part.
pub fn render_picture(source: &PictureSource, overall_width: f64, slice_x: i32, width: u32, height: u32) -> Pixels {
    if let Some(visual) = &source.visual {
        return render_visual(visual, source, overall_width, slice_x, width, height);
    }
    let Some(intents) = source.intents.as_deref() else { return Pixels::transparent(width, height) };
    if overall_width <= 0.0 || source.span <= TimeSpan::ZERO {
        return Pixels::transparent(width, height);
    }
    let offset_of = |x: f64| TimeSpan(((x / overall_width) * source.span.ticks() as f64) as i64);
    let visible_start = offset_of(slice_x as f64);
    let end_x = slice_x as f64 + width as f64;
    // F42: when the effect ends inside the view the end offset only means "no clipping".
    let visible_end = if end_x >= overall_width { source.span } else { offset_of(end_x) };
    rasterize(&ThumbnailRequest {
        intents,
        elements: &source.elements,
        effect_span: source.span,
        visible_start,
        visible_end,
        overall_width,
        width,
        height,
    })
}

fn render_visual(visual: &VisualSource, source: &PictureSource, overall_width: f64, slice_x: i32, width: u32, height: u32) -> Pixels {
    let Some(draw) = visual.descriptor.visual else { return Pixels::transparent(width, height) };
    let info = TargetInfo {
        tree: &visual.tree,
        targets: &visual.targets,
        marks: &visual.marks,
        start_time: source.start,
        time_span: source.span,
        audio_loaded: visual.audio_loaded,
    };
    let mut g = Graphics::new(width as i32, height as i32);
    draw(&visual.params, &info, &mut g, (-slice_x, 0, overall_width as i32, height as i32));
    // The bitmap hands out straight alpha, and pictures are premultiplied.
    let mut rgba = g.bitmap().to_rgba();
    for px in rgba.chunks_exact_mut(4) {
        let a = px[3] as u32;
        for c in &mut px[..3] {
            *c = ((*c as u32 * a + 127) / 255) as u8;
        }
    }
    Pixels { width, height, rgba }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::{Color, Intent};

    fn ms(v: i64) -> TimeSpan {
        TimeSpan::from_ms_i(v)
    }

    fn source() -> PictureSource {
        let id = Uuid::new_v4();
        let mut intents = EffectIntents::new();
        intents.add_intent_for_element(id, Intent::static_lighting(Color::RED, 1.0, ms(100)), TimeSpan::ZERO);
        intents.add_intent_for_element(id, Intent::static_lighting(Color::BLUE, 1.0, ms(100)), ms(100));
        PictureSource { intents: Some(Arc::new(intents)), elements: Arc::new(vec![id]), span: ms(200), start: ms(1000), visual: None, picture: 1 }
    }

    fn at(p: &Pixels, x: usize) -> [u8; 3] {
        let i = (2 * p.width as usize + x) * 4;
        [p.rgba[i], p.rgba[i + 1], p.rgba[i + 2]]
    }

    #[test]
    fn the_whole_effect_or_a_slice_draws_the_same_colours() {
        let s = source();
        let whole = render_picture(&s, 200.0, 0, 200, 4);
        assert_eq!(at(&whole, 50), [255, 0, 0]);
        assert_eq!(at(&whole, 150), [0, 0, 255]);
        let slice = render_picture(&s, 200.0, 100, 100, 4);
        assert_eq!(at(&slice, 50), [0, 0, 255]);
    }

    #[test]
    fn an_effect_with_nothing_rendered_is_transparent() {
        let mut s = source();
        s.intents = None;
        let p = render_picture(&s, 100.0, 0, 100, 4);
        assert!(p.rgba.iter().all(|b| *b == 0));
    }
}
