//! Port of VixenModules.Effect.LipSync, including `FastPictureEffect` and the thumbnail.
//!
//! Two mapping types exist. `FaceDefinition` lights the elements whose Face property enables the current
//! phoneme. `Map` draws a picture per phoneme onto a pixel grid; the pictures are the files
//! `<module data root>/LipSync/<PhonemeMapping>/<phoneme>.bmp` that `LipSyncMapData.ImageForPhoneme` reads.
//! The map library itself (`LipSyncMapLibrary`, which supplies the default mapping when `PhonemeMapping`
//! is unknown) is an application service that this crate cannot see, so an unknown mapping renders nothing.

use crate::generated::lip_sync as gen;
use crate::property_data::{FaceComponent, FaceData};
use crate::visual::adjusted_font;
use std::collections::HashMap;
use std::sync::OnceLock;
use vixen_effects::gfx::{Bitmap, Graphics};
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    targets_changed: Some(targets_changed),
    param_changed_ctx: Some(param_changed_ctx),
    force_visual: true,
    visual: Some(visual),
    selection: Some(selection),
    ported: true,
    ..gen::BASE
};

/// `TargetNodesChanged`: `SetOrientation`, on a property LipSync names itself, so `set_targets` can't do it.
fn targets_changed(p: &mut Params, t: &TargetInfo) {
    use vixen_core::element::Orientation;
    if let Some(orientation) = t.targets.first().and_then(|n| t.tree.orientation(*n)) {
        let name = if orientation == Orientation::Horizontal { "Horizontal" } else { "Vertical" };
        p.set("Orientation", ParamValue::Enum(name.to_string()));
    }
}

/// `LipSyncMarkCollectionNameConverter`: phoneme collections, and whichever is selected so an old choice stays visible.
fn selection(name: &str, _row: Option<&Params>, p: &Params, t: &TargetInfo) -> Option<Vec<SelectionValue>> {
    if name != "MarkCollectionId" {
        return None;
    }
    let selected = p.guid("MarkCollectionId");
    Some(
        t.marks
            .iter()
            .filter(|m| m.collection_type == vixen_core::marks::MarkCollectionType::Phoneme || m.id == selected)
            .map(|m| SelectionValue { value: ParamValue::Guid(m.id), label: m.name.clone() })
            .collect(),
    )
}

/// `GetMarkCollectionSelections`: prefers a phoneme collection and, unlike the other effects, never falls back to the first
/// collection of another type.
fn param_changed_ctx(c: &ParamChange, p: &mut Params) {
    if matches!(c.name, MARK_COLLECTIONS | "LipSyncMode") {
        use vixen_core::marks::MarkCollectionType;
        normalize_mark_selection(p, c.info, p.enum_name("LipSyncMode") == "MarkCollection", Some(MarkCollectionType::Phoneme), false);
    }
}

/// `SetMatrixBrowesables`, `SetMappingType` and `SetLipsyncModeBrowsables`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let is_map = p.enum_name("MappingType") == "Map";
    let scale_to_grid = p.boolean("ScaleToGrid");
    v.set("Orientation", is_map);
    v.set("ScaleToGrid", is_map);
    v.set("ScalePercent", is_map && !scale_to_grid);
    v.set("IntensityLevel", is_map);
    v.set("YOffsetCurve", is_map);
    v.set("XOffsetCurve", is_map);
    v.set("PhonemeMapping", is_map);
    v.set("ShowOutline", !is_map);
    v.set("EyeMode", !is_map);
    let marks = p.enum_name("LipSyncMode") == "MarkCollection";
    v.set("StaticPhoneme", !marks);
    v.set("LyricData", !marks);
    v.set("MarkCollectionId", marks);
    v.set("AllowMarkGaps", marks);
}

// ---------------------------------------------------------------------------------------------------------
// Phoneme pictures

/// `LipSync._phonemeBitmaps`: the built-in mouth shapes, keyed by `PhonemeType` name.
fn phoneme_bitmaps() -> &'static HashMap<&'static str, Bitmap> {
    static BITMAPS: OnceLock<HashMap<&'static str, Bitmap>> = OnceLock::new();
    BITMAPS.get_or_init(|| {
        let sources: [(&str, &[u8]); 10] = [
            ("AI", include_bytes!("../assets/AI.jpg")),
            ("E", include_bytes!("../assets/E.jpg")),
            ("ETC", include_bytes!("../assets/etc.jpg")),
            ("FV", include_bytes!("../assets/FV.jpg")),
            ("L", include_bytes!("../assets/L.jpg")),
            ("MBP", include_bytes!("../assets/MBP.jpg")),
            ("O", include_bytes!("../assets/O.jpg")),
            ("REST", include_bytes!("../assets/rest.jpg")),
            ("U", include_bytes!("../assets/U.jpg")),
            ("WQ", include_bytes!("../assets/WQ.jpg")),
        ];
        sources.iter().filter_map(|(name, bytes)| Bitmap::from_bytes(bytes).ok().map(|b| (*name, b))).collect()
    })
}

/// `Enum.TryParse<PhonemeType>(text.ToUpper())`: exact, case-sensitive member names after upper-casing,
/// so `Unknown` can never match.
fn parse_phoneme(text: &str) -> Option<&'static str> {
    const NAMES: [&str; 10] = ["AI", "E", "O", "U", "FV", "L", "MBP", "WQ", "ETC", "REST"];
    let upper = text.to_uppercase();
    NAMES.iter().find(|n| **n == upper).copied()
}

// ---------------------------------------------------------------------------------------------------------
// Rendering

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut out = EffectIntents::new();

    // SetupMarks: without a phoneme-capable collection the effect falls back to Phoneme mode.
    let mut mark_mode = p.enum_name("LipSyncMode") == "MarkCollection";
    let mut marks: Vec<Mark> = Vec::new();
    if mark_mode {
        if ctx.marks.iter().all(|c| c.collection_type == vixen_core::marks::MarkCollectionType::Generic) {
            mark_mode = false;
        } else if let Some(mc) = ctx.mark_collection(p.guid("MarkCollectionId")) {
            marks.extend(mc.marks_inclusive_of_time(ctx.start_time, ctx.start_time + ctx.time_span).into_iter().cloned());
        }
    }
    let phoneme = p.enum_name("StaticPhoneme");
    let allow_gaps = p.boolean("AllowMarkGaps");

    if p.enum_name("MappingType") == "Map" {
        let _ = render_map(ctx, p, mark_mode, &marks, phoneme, allow_gaps, &mut out);
    } else {
        let has_discrete = ctx.has_discrete_colors();
        for &target in ctx.targets {
            for node in ctx.tree.descendants(target) {
                render_property_map_element(ctx, p, node, has_discrete, mark_mode, &marks, phoneme, allow_gaps, &mut out);
            }
        }
    }
    out
}

/// `RenderPropertyMapElements` for one node. The `else` after the eyes-closed test is the C#'s: the
/// mouth/phoneme logic runs unless this is an eyes-closed element while the eyes are closed, even after an
/// outline or open eye matched.
#[allow(clippy::too_many_arguments)]
fn render_property_map_element(
    ctx: &RenderCtx,
    p: &Params,
    node: NodeId,
    has_discrete: bool,
    mark_mode: bool,
    marks: &[Mark],
    phoneme: &str,
    allow_gaps: bool,
    out: &mut EffectIntents,
) {
    let Some(fm) = FaceData::for_node(ctx.tree, node) else { return };
    let eye_mode = p.enum_name("EyeMode");
    let (intensity, color) = fm.configured_color_and_intensity();
    let add = |out: &mut EffectIntents, duration: TimeSpan, offset: Option<TimeSpan>| {
        let mut result = ctx.create_intents_for_element(has_discrete, node, intensity, color, duration);
        if let Some(o) = offset {
            result.offset_all(o);
        }
        out.add(result);
    };

    if fm.is_face_component(FaceComponent::Outlines) && p.boolean("ShowOutline") {
        add(out, ctx.time_span, None);
    } else if fm.is_face_component(FaceComponent::EyesOpen) && eye_mode == "Open" {
        add(out, ctx.time_span, None);
    }
    if fm.is_face_component(FaceComponent::EyesClosed) && eye_mode == "Closed" {
        add(out, ctx.time_span, None);
    } else if mark_mode {
        let start_time = ctx.start_time;
        let mut last_mark_time = start_time;
        for mark in marks {
            if !allow_gaps {
                let gap = mark.start_time - last_mark_time;
                if gap.total_ms() > 10.0 && fm.phoneme_state("REST") {
                    // Fill the gap with a rest
                    add(out, gap, Some(last_mark_time - start_time));
                }
                last_mark_time = mark.end_time();
            }
            if fm.phoneme_state(&mark.text) {
                add(out, mark.duration, Some(mark.start_time - start_time));
            }
        }
        if !allow_gaps {
            let gap = start_time + ctx.time_span - last_mark_time;
            if gap.total_ms() > 10.0 && fm.phoneme_state("REST") {
                add(out, gap, Some(last_mark_time - start_time));
            }
        }
    } else if fm.phoneme_state(phoneme) {
        add(out, ctx.time_span, None);
    }
}

/// `FastPictureEffect`: draws one picture centred in the grid, moved by the X and Y offset curves.
struct FastPicture {
    image: Bitmap,
    scale_to_grid: bool,
    scale_percent: i32,
    level: i32,
    x_offset: Curve,
    y_offset: Curve,
    /// `EffectEndTime`: the whole LipSync effect, which is what the offset curves run over.
    effect_end: TimeSpan,
    /// `StartTime`: where this picture's slice starts within the effect.
    start: TimeSpan,
    scaled: Option<Bitmap>,
}

/// `Picture.ScalePictureImage`.
fn scale_picture_image(image: &Bitmap, max_w: i32, max_h: i32) -> Bitmap {
    let ratio_x = max_w as f64 / image.width() as f64;
    let ratio_y = max_h as f64 / image.height() as f64;
    let ratio = ratio_x.min(ratio_y);
    let mut new_w = (image.width() as f64 * ratio) as i32;
    let mut new_h = (image.height() as f64 * ratio) as i32;
    if new_h <= 0 {
        new_h = 1;
    }
    if new_w <= 0 {
        new_w = 1;
    }
    image.resized(new_w, new_h)
}

/// `Picture.ScaleImage`.
fn scale_image(image: &Bitmap, scale: f64) -> Bitmap {
    let max_w = (image.width() as f64 * scale).round_ties_even() as i32;
    let max_h = (image.height() as f64 * scale).round_ties_even() as i32;
    let ratio_x = max_w as f64 / image.width() as f64;
    let ratio_y = max_h as f64 / image.height() as f64;
    let ratio = ratio_x.min(ratio_y);
    let new_w = ((image.width() as f64 * ratio) as i32).max(1);
    let new_h = ((image.height() as f64 * ratio) as i32).max(1);
    image.resized(new_w, new_h)
}

impl PixelEffect for FastPicture {
    fn setup_render(&mut self, info: &PixelInfo) {
        self.scaled = Some(if self.scale_to_grid {
            scale_picture_image(&self.image, info.buffer_wi(), info.buffer_ht())
        } else {
            scale_image(&self.image, self.scale_percent as f64 / 100.0)
        });
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let Some(fp) = &self.scaled else { return };
        let interval_pos_factor = 100.0 / self.effect_end.total_ms() * (self.start.total_ms() + (frame * info.frame_time) as f64);
        let (ht, wi) = (info.buffer_ht() as f64, info.buffer_wi() as f64);
        let (fw, fh) = (fp.width(), fp.height());
        let y_adjust =
            scale_curve_to_value(self.y_offset.get_value(interval_pos_factor), ht / 2.0 + fh as f64 / 2.0, -(ht / 2.0 + fh as f64 / 2.0)) as i32;
        let x_adjust =
            scale_curve_to_value(self.x_offset.get_value(interval_pos_factor), wi / 2.0 + fw as f64 / 2.0, -(wi / 2.0 + fw as f64 / 2.0)) as i32;
        let y_offset = (((info.buffer_ht() + fh) as f64 / 2.0) + y_adjust as f64 - 1.0) as i32;
        let x_offset = (x_adjust as f64 + (info.buffer_wi() as f64 - fw as f64) / 2.0) as i32;
        for x in 0..fw {
            for y in 0..fh {
                let mut c = fp.get_pixel(x, y);
                if self.level < 100 {
                    let mut hsv = Hsv::from_color(c);
                    hsv.set_v(hsv.v * (self.level as f64 / 100.0));
                    c = hsv.to_color();
                }
                buf.set_pixel(x + x_offset, y_offset - y, c);
            }
        }
    }

    fn cleanup_render(&mut self) {
        self.scaled = None;
    }
}

/// `LipSyncMapData.ImageForPhoneme`: `<root>/LipSync/<mapping>/<phoneme>.bmp`. A missing file throws in
/// the C#, which leaves the effect dark; here that is an `Err` that aborts the render.
struct MapImages {
    dir: std::path::PathBuf,
    cache: HashMap<String, Bitmap>,
}

impl MapImages {
    fn image(&mut self, phoneme: &str) -> Result<Bitmap, ()> {
        let key = phoneme.to_lowercase();
        if let Some(b) = self.cache.get(&key) {
            return Ok(b.clone());
        }
        // Phoneme text comes from marks; never let it climb out of the mapping folder.
        if phoneme.contains(['/', '\\']) || phoneme.contains("..") {
            return Err(());
        }
        let bitmap = Bitmap::from_file(&self.dir.join(format!("{phoneme}.bmp"))).map_err(|_| ())?;
        self.cache.insert(key, bitmap.clone());
        Ok(bitmap)
    }
}

/// `RenderMapMatrix`. `Err` means the C# threw part-way (a missing picture): everything already produced
/// is discarded.
fn render_map(
    ctx: &RenderCtx,
    p: &Params,
    mark_mode: bool,
    marks: &[Mark],
    phoneme: &str,
    allow_gaps: bool,
    out: &mut EffectIntents,
) -> Result<(), ()> {
    let (Some(root), mapping) = (ctx.module_data_root, p.string("PhonemeMapping")) else { return Err(()) };
    if mapping.is_empty() || mapping.contains(['/', '\\']) || mapping.contains("..") {
        return Err(());
    }
    let dir = root.join("LipSync").join(mapping);
    if !dir.is_dir() {
        return Err(());
    }
    let mut images = MapImages { dir, cache: HashMap::new() };
    let orientation = StringOrientation::from_name(p.enum_name("Orientation"));
    let mut result = EffectIntents::new();

    let mut pic = FastPicture {
        image: Bitmap::new(1, 1),
        scale_to_grid: p.boolean("ScaleToGrid"),
        scale_percent: p.int("ScalePercent"),
        level: p.int("IntensityLevel"),
        x_offset: p.curve("XOffsetCurve"),
        y_offset: p.curve("YOffsetCurve"),
        effect_end: ctx.time_span,
        start: TimeSpan::ZERO,
        scaled: None,
    };
    let start_time = ctx.start_time;

    // `_thePic.Render()`: a pixel render of this picture over `span`, shifted by `offset`.
    let draw = |pic: &mut FastPicture, image: Bitmap, span: TimeSpan, offset: Option<TimeSpan>, result: &mut EffectIntents| {
        pic.image = image;
        let sub = RenderCtx { video_cache: ctx.video_cache,
            tree: ctx.tree,
            targets: ctx.targets,
            start_time: ctx.start_time,
            time_span: span,
            frame_ms: ctx.frame_ms,
            marks: ctx.marks,
            audio: ctx.audio,
            module_data_root: ctx.module_data_root,
            cancel: ctx.cancel,
        };
        let mut r = render_pixel(&sub, orientation, TargetPositioning::Strings, pic);
        if let Some(o) = offset {
            r.offset_all(o);
        }
        result.add(r);
    };

    if mark_mode {
        let mut last_mark_time = start_time;
        for mark in marks {
            if !allow_gaps {
                let gap = mark.start_time - last_mark_time;
                if gap.total_ms() > 10.0 {
                    let rest = images.image("REST")?;
                    pic.start = mark.start_time - start_time - gap;
                    draw(&mut pic, rest, gap, Some(last_mark_time - start_time), &mut result);
                }
                last_mark_time = mark.end_time();
            }
            let image = images.image(&mark.text)?;
            pic.start = mark.start_time - start_time;
            draw(&mut pic, image, mark.duration, Some(mark.start_time - start_time), &mut result);
        }
        if !allow_gaps {
            pic.start = last_mark_time - start_time;
            let gap = start_time + ctx.time_span - last_mark_time;
            if gap.total_ms() > 10.0 {
                let rest = images.image("REST")?;
                draw(&mut pic, rest, gap, Some(last_mark_time - start_time), &mut result);
            }
        }
    } else {
        let image = images.image(phoneme)?;
        draw(&mut pic, image, ctx.time_span, None, &mut result);
    }
    out.add(result);
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// Thumbnail

/// What `GenerateVisualRepresentation` reads from the effect instance besides its parameters: the marks of
/// the last render and the effect's start and length.
pub struct VisualContext<'a> {
    pub marks: &'a [Mark],
    pub start_time: TimeSpan,
    pub time_span: TimeSpan,
}

/// The `VisualFn` for the descriptor.
fn visual(p: &Params, t: &TargetInfo, g: &mut Graphics, rect: (i32, i32, i32, i32)) {
    // `_marks`, from SetupMarks.
    let marks: Vec<Mark> = t
        .mark_collection(p.guid("MarkCollectionId"))
        .map(|mc| mc.marks_inclusive_of_time(t.start_time, t.start_time + t.time_span).into_iter().cloned().collect())
        .unwrap_or_default();
    draw_visual(p, g, rect, &VisualContext { marks: &marks, start_time: t.start_time, time_span: t.time_span });
}

/// `new Bitmap(image, width, height)`, which throws for a non-positive size.
fn scaled_to(image: &Bitmap, w: i32, h: i32) -> Option<Bitmap> {
    if w <= 0 || h <= 0 {
        None
    } else {
        Some(image.resized(w, h))
    }
}

/// `GenerateVisualRepresentation`. Any case where the C# throws (a bitmap of non-positive size) ends the
/// drawing there, because the C# catches the exception around the whole method.
pub fn draw_visual(p: &Params, g: &mut Graphics, clip: (i32, i32, i32, i32), vc: &VisualContext) {
    let _ = draw_visual_inner(p, g, clip, vc);
}

fn draw_visual_inner(p: &Params, g: &mut Graphics, clip: (i32, i32, i32, i32), vc: &VisualContext) -> Option<()> {
    let (cx, cy, cw, ch) = clip;
    let bitmaps = phoneme_bitmaps();
    if p.enum_name("LipSyncMode") == "MarkCollection" {
        let allow_gaps = p.boolean("AllowMarkGaps");
        let ticks = vc.time_span.ticks() as f64;
        let start = vc.start_time;
        let black_line = |g: &mut Graphics, x: i32| g.draw_line(Color::BLACK, 1.0, x as f32, 0.0, x as f32, ch as f32);
        if !vc.marks.is_empty() {
            let mut first = true;
            let mut end_x = 0;
            let mut last_mark_time = start;
            for mark in vc.marks {
                if !allow_gaps {
                    let gap = mark.start_time - last_mark_time;
                    if gap.total_ms() > 10.0 {
                        if let Some(rest) = bitmaps.get("REST") {
                            end_x = ((mark.start_time.ticks() - start.ticks()) as f64 / ticks * cw as f64) as i32;
                            let start_x = ((last_mark_time.ticks() - start.ticks()) as f64 / ticks * cw as f64) as i32;
                            let scaled = scaled_to(rest, cw.min(end_x - start_x), ch)?;
                            g.draw_image(&scaled, (cx + start_x) as f32, cy as f32, scaled.width() as f32, scaled.height() as f32);
                            if first {
                                first = false;
                            } else {
                                black_line(g, cx + start_x);
                            }
                        }
                    }
                    last_mark_time = mark.end_time();
                }
                if let Some(ph) = parse_phoneme(&mark.text) {
                    if let Some(image) = bitmaps.get(ph) {
                        end_x = ((mark.end_time().ticks() - start.ticks()) as f64 / ticks * cw as f64) as i32;
                        if end_x <= 0 {
                            continue;
                        }
                        let start_x = ((mark.start_time.ticks() - start.ticks()).max(0) as f64 / ticks * cw as f64) as i32;
                        let scaled = scaled_to(image, cw.min(end_x - start_x), ch)?;
                        g.draw_image(&scaled, (cx + start_x) as f32, cy as f32, scaled.width() as f32, scaled.height() as f32);
                        if first && mark.start_time <= start {
                            first = false;
                            continue;
                        }
                        black_line(g, cx + start_x);
                    }
                }
                first = false;
            }
            if !allow_gaps {
                let gap = start + vc.time_span - last_mark_time;
                if gap.total_ms() > 10.0 {
                    if let Some(rest) = bitmaps.get("REST") {
                        let end_x = cw;
                        let start_x = ((last_mark_time.ticks() - start.ticks()) as f64 / ticks * cw as f64) as i32;
                        let scaled = scaled_to(rest, cw.min(end_x - start_x), ch)?;
                        g.draw_image(&scaled, (cx + start_x) as f32, cy as f32, scaled.width() as f32, scaled.height() as f32);
                        black_line(g, cx + start_x);
                    }
                }
            } else if let Some(last) = vc.marks.last() {
                if last.end_time() < start + vc.time_span {
                    // draw a closing line on the image
                    black_line(g, cx + end_x);
                }
            }
        } else if let Some(rest) = bitmaps.get("REST") {
            // in mark mode with no marks, draw a rest.
            let end_x = ((vc.time_span.ticks() + start.ticks()) as f64 / ticks * cw as f64) as i32;
            let start_x = (start.ticks() as f64 / ticks * cw as f64) as i32;
            let scaled = scaled_to(rest, cw.min(end_x - start_x), ch)?;
            g.draw_image(&scaled, cx as f32, cy as f32, scaled.width() as f32, scaled.height() as f32);
        }
    } else {
        let lyric = p.string("LyricData");
        let display = if lyric.trim().is_empty() { "-" } else { lyric };
        g.fill_rectangle(Color::GREEN, cx as f32, cy as f32, cw as f32, ch as f32);
        let mut scaled: Option<Bitmap> = None;
        if let Some(image) = bitmaps.get(p.enum_name("StaticPhoneme")) {
            let s = scaled_to(image, ch.min(cw), ch)?;
            g.draw_image(&s, cx as f32, cy as f32, s.width() as f32, s.height() as f32);
            scaled = Some(s);
        }
        if let Some(s) = scaled {
            if s.width() < cw {
                let text_start = cx + s.width();
                let text_width = cw - s.width();
                let font = adjusted_font(display, text_width, ch, 48);
                // the C# draws at y = 2, not at the rectangle's top
                g.draw_string(display, &font, Color::YELLOW, (2 + text_start) as f32, 2.0);
            }
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::element::{PropertyData, PropertyInstance};
    use vixen_core::marks::MarkCollectionType;
    use vixen_effects::testing;

    fn face_xml(phonemes: &[(&str, bool)], components: &[(&str, bool)], color: u32) -> String {
        let entries = |kv: &[(&str, bool)], tag: &str| {
            kv.iter().map(|(k, v)| format!("<{tag}><Key>{k}</Key><Value>{v}</Value></{tag}>")).collect::<String>()
        };
        format!(
            "<FaceData><DefaultColor><state>2</state><value>{color}</value></DefaultColor><FaceComponents>{}</FaceComponents><PhonemeList>{}</PhonemeList></FaceData>",
            entries(components, "KeyValueOfFaceComponentboolean"),
            entries(phonemes, "KeyValueOfstringboolean")
        )
    }

    fn add_face(tree: &mut ElementTree, node: NodeId, xml: String) {
        tree.node_mut(node).properties.push(PropertyInstance {
            type_id: crate::property_data::FACE_ID,
            instance_id: Uuid::new_v4(),
            data: PropertyData::Unknown { raw_xml: xml },
        });
    }

    /// A face with a mouth that shows AI and REST, an outline and eyes.
    fn face() -> (ElementTree, NodeId, NodeId, NodeId, NodeId, NodeId) {
        let mut tree = ElementTree::new();
        let root = tree.add_node(Uuid::new_v4(), "Face", None);
        let mouth = tree.add_element("Mouth");
        let outline = tree.add_element("Outline");
        let eyes_open = tree.add_element("EyesOpen");
        let eyes_closed = tree.add_element("EyesClosed");
        for n in [mouth, outline, eyes_open, eyes_closed] {
            tree.add_child(root, n);
        }
        add_face(&mut tree, mouth, face_xml(&[("AI", true), ("REST", true), ("E", false)], &[("Mouth", true)], 0xFF00FF00));
        add_face(&mut tree, outline, face_xml(&[], &[("Outlines", true)], 0xFFFF0000));
        add_face(&mut tree, eyes_open, face_xml(&[], &[("EyesOpen", true)], 0xFF0000FF));
        add_face(&mut tree, eyes_closed, face_xml(&[], &[("EyesClosed", true)], 0xFFFFFF00));
        (tree, root, mouth, outline, eyes_open, eyes_closed)
    }

    fn render_with_marks(
        tree: &ElementTree,
        target: NodeId,
        ms: i64,
        marks: &[MarkCollection],
        edit: impl FnOnce(&mut Params),
    ) -> EffectIntents {
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let targets = [target];
        let ctx = RenderCtx { video_cache: None,
            tree,
            targets: &targets,
            start_time: TimeSpan::from_ms_i(1000),
            time_span: TimeSpan::from_ms_i(ms),
            frame_ms: 50,
            marks,
            audio: None,
            module_data_root: None,
            cancel: &cancel,
        };
        let mut p = DESCRIPTOR.new_params();
        edit(&mut p);
        render(&ctx, &p)
    }

    fn color_of(out: &EffectIntents, tree: &ElementTree, n: NodeId) -> Vec<(f64, f64, Color)> {
        out.nodes_for(&tree.node(n).element_id.unwrap())
            .map(|v| {
                v.iter()
                    .map(|i| match i.intent.state_at(TimeSpan::ZERO) {
                        vixen_core::StateValue::Lighting(l) => (i.start.total_ms(), i.intent.span.total_ms(), l.color),
                        other => panic!("{other:?}"),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        // Defaults target elements without a Face property: dark, as in the C#.
        let mut tree = ElementTree::new();
        let n = tree.add_element("x");
        assert!(render_with_marks(&tree, n, 1000, &[], |_| {}).is_empty());
    }

    #[test]
    fn static_phoneme_lights_mouth_outline_and_open_eyes() {
        let (tree, root, mouth, outline, eyes_open, eyes_closed) = face();
        let out = render_with_marks(&tree, root, 2000, &[], |p| {
            p.set("LipSyncMode", ParamValue::Enum("Phoneme".into()));
            p.set("StaticPhoneme", ParamValue::Enum("AI".into()));
        });
        assert_eq!(color_of(&out, &tree, mouth), vec![(0.0, 2000.0, Color::LIME)]);
        assert_eq!(color_of(&out, &tree, outline), vec![(0.0, 2000.0, Color::RED)]);
        assert_eq!(color_of(&out, &tree, eyes_open), vec![(0.0, 2000.0, Color::BLUE)]);
        assert!(color_of(&out, &tree, eyes_closed).is_empty());
        // E is switched off on the mouth
        let out = render_with_marks(&tree, root, 2000, &[], |p| {
            p.set("LipSyncMode", ParamValue::Enum("Phoneme".into()));
            p.set("StaticPhoneme", ParamValue::Enum("E".into()));
        });
        assert!(color_of(&out, &tree, mouth).is_empty());
    }

    #[test]
    fn eye_and_outline_options() {
        let (tree, root, _mouth, outline, eyes_open, eyes_closed) = face();
        let out = render_with_marks(&tree, root, 1000, &[], |p| {
            p.set("LipSyncMode", ParamValue::Enum("Phoneme".into()));
            p.set("EyeMode", ParamValue::Enum("Closed".into()));
            p.set("ShowOutline", ParamValue::Bool(false));
        });
        assert!(color_of(&out, &tree, outline).is_empty());
        assert!(color_of(&out, &tree, eyes_open).is_empty());
        assert_eq!(color_of(&out, &tree, eyes_closed).len(), 1);
        let off = render_with_marks(&tree, root, 1000, &[], |p| {
            p.set("LipSyncMode", ParamValue::Enum("Phoneme".into()));
            p.set("EyeMode", ParamValue::Enum("Off".into()));
        });
        assert!(color_of(&off, &tree, eyes_open).is_empty() && color_of(&off, &tree, eyes_closed).is_empty());
    }

    fn phoneme_collection() -> MarkCollection {
        let mut mc = MarkCollection::default();
        mc.collection_type = MarkCollectionType::Phoneme;
        // The effect is 1000..3000 ms.
        let mut m = Mark::new(TimeSpan::from_ms_i(1500));
        m.duration = TimeSpan::from_ms_i(500);
        m.text = "ai".into();
        mc.add_mark(m);
        mc
    }

    #[test]
    fn marks_drive_the_mouth_and_gaps_become_rest() {
        let (tree, root, mouth, outline, ..) = face();
        let mc = phoneme_collection();
        let id = mc.id;
        let marks = [mc];
        let out = render_with_marks(&tree, root, 2000, &marks, |p| {
            p.set("MarkCollectionId", ParamValue::Guid(id));
        });
        // gap 1000..1500 rest, AI 1500..2000, trailing gap 2000..3000 rest (offsets are relative to 1000)
        assert_eq!(
            color_of(&out, &tree, mouth),
            vec![(0.0, 500.0, Color::LIME), (500.0, 500.0, Color::LIME), (1000.0, 1000.0, Color::LIME)]
        );
        // the outline still lights for the whole effect
        assert_eq!(color_of(&out, &tree, outline), vec![(0.0, 2000.0, Color::RED), ]);
        // with gaps allowed only the mark remains
        let out = render_with_marks(&tree, root, 2000, &marks, |p| {
            p.set("MarkCollectionId", ParamValue::Guid(id));
            p.set("AllowMarkGaps", ParamValue::Bool(true));
        });
        assert_eq!(color_of(&out, &tree, mouth), vec![(500.0, 500.0, Color::LIME)]);
    }

    #[test]
    fn generic_collections_fall_back_to_phoneme_mode() {
        let (tree, root, mouth, ..) = face();
        let mut mc = phoneme_collection();
        mc.collection_type = MarkCollectionType::Generic;
        let id = mc.id;
        let out = render_with_marks(&tree, root, 2000, &[mc], |p| {
            p.set("MarkCollectionId", ParamValue::Guid(id));
            p.set("StaticPhoneme", ParamValue::Enum("AI".into()));
        });
        assert_eq!(color_of(&out, &tree, mouth), vec![(0.0, 2000.0, Color::LIME)]);
    }

    #[test]
    fn gaps_under_ten_ms_are_not_filled() {
        let (tree, root, mouth, ..) = face();
        let mut mc = MarkCollection::default();
        mc.collection_type = MarkCollectionType::Phoneme;
        let mut m = Mark::new(TimeSpan::from_ms_i(1005));
        m.duration = TimeSpan::from_ms_i(1990);
        m.text = "AI".into();
        mc.add_mark(m);
        let id = mc.id;
        let out = render_with_marks(&tree, root, 2000, &[mc], |p| p.set("MarkCollectionId", ParamValue::Guid(id)).then_some(()).unwrap());
        assert_eq!(color_of(&out, &tree, mouth), vec![(5.0, 1990.0, Color::LIME)]);
    }

    #[test]
    fn map_mode_draws_pictures_from_the_module_data_folder() {
        // 2x2 red picture saved as a BMP in <root>/LipSync/Faces/REST.bmp, rendered onto a 2x2 grid.
        let root = std::env::temp_dir().join(format!("lipsync-test-{}", Uuid::new_v4()));
        let dir = root.join("LipSync").join("Faces");
        std::fs::create_dir_all(&dir).unwrap();
        let mut bmp = Vec::new();
        {
            // 24-bit uncompressed BMP, 2x2, all red (BGR), rows padded to 4 bytes
            let (w, h) = (2u32, 2u32);
            let row = vec![0u8, 0, 255, 0, 0, 255, 0, 0];
            let size = 54 + row.len() as u32 * h;
            bmp.extend_from_slice(b"BM");
            bmp.extend_from_slice(&size.to_le_bytes());
            bmp.extend_from_slice(&[0, 0, 0, 0]);
            bmp.extend_from_slice(&54u32.to_le_bytes());
            bmp.extend_from_slice(&40u32.to_le_bytes());
            bmp.extend_from_slice(&w.to_le_bytes());
            bmp.extend_from_slice(&h.to_le_bytes());
            bmp.extend_from_slice(&1u16.to_le_bytes());
            bmp.extend_from_slice(&24u16.to_le_bytes());
            bmp.extend_from_slice(&[0; 24]);
            for _ in 0..h {
                bmp.extend_from_slice(&row);
            }
        }
        std::fs::write(dir.join("REST.bmp"), &bmp).unwrap();

        let mut tree = ElementTree::new();
        let grid = testing::grid(&mut tree, "G", 2, 2);
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let targets = [grid];
        let ctx = RenderCtx { video_cache: None,
            tree: &tree,
            targets: &targets,
            start_time: TimeSpan::ZERO,
            time_span: TimeSpan::from_ms_i(500),
            frame_ms: 50,
            marks: &[],
            audio: None,
            module_data_root: Some(&root),
            cancel: &cancel,
        };
        let mut p = DESCRIPTOR.new_params();
        p.set("MappingType", ParamValue::Enum("Map".into()));
        p.set("LipSyncMode", ParamValue::Enum("Phoneme".into()));
        p.set("PhonemeMapping", ParamValue::Str("Faces".into()));
        p.set("IntensityLevel", ParamValue::Int(50));
        let out = render(&ctx, &p);
        assert_eq!(out.map.len(), 4);
        let any = out.map.values().next().unwrap();
        match &any[0].intent.data {
            IntentData::ArrayRgb(a) => {
                let v = a.at(TimeSpan::from_ms_i(100));
                // red at 50% brightness
                assert!(v.r >= 126 && v.r <= 129 && v.g == 0 && v.b == 0, "{v:?}");
            }
            other => panic!("{other:?}"),
        }
        // an unknown phoneme picture (no file) leaves the effect dark
        p.set("StaticPhoneme", ParamValue::Enum("AI".into()));
        assert!(render(&ctx, &p).is_empty());
        // and so does an unknown mapping
        p.set("PhonemeMapping", ParamValue::Str("Nope".into()));
        assert!(render(&ctx, &p).is_empty());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn visibility_rules() {
        let names = |p: &Params| -> Vec<&'static str> {
            DESCRIPTOR.visible_params(p).iter().map(|i| DESCRIPTOR.params[*i].name).collect()
        };
        let mut p = DESCRIPTOR.new_params();
        let n = names(&p);
        assert!(n.contains(&"EyeMode") && n.contains(&"ShowOutline") && n.contains(&"MarkCollectionId"));
        assert!(!n.contains(&"PhonemeMapping") && !n.contains(&"StaticPhoneme") && !n.contains(&"ScalePercent"));
        p.set("MappingType", ParamValue::Enum("Map".into()));
        p.set("LipSyncMode", ParamValue::Enum("Phoneme".into()));
        p.set("ScaleToGrid", ParamValue::Bool(false));
        let n = names(&p);
        assert!(n.contains(&"PhonemeMapping") && n.contains(&"ScalePercent") && n.contains(&"StaticPhoneme"));
        assert!(!n.contains(&"EyeMode") && !n.contains(&"AllowMarkGaps"));
    }

    #[test]
    fn embedded_phoneme_images_decode() {
        let b = phoneme_bitmaps();
        assert_eq!(b.len(), 10);
        assert!(b["REST"].width() > 0);
        assert_eq!(parse_phoneme("etc"), Some("ETC"));
        assert_eq!(parse_phoneme("unknown"), None);
    }

    #[test]
    fn thumbnail_phoneme_mode_draws_green_picture_and_text() {
        let mut p = DESCRIPTOR.new_params();
        p.set("LipSyncMode", ParamValue::Enum("Phoneme".into()));
        p.set("StaticPhoneme", ParamValue::Enum("O".into()));
        p.set("LyricData", ParamValue::Str("hello".into()));
        let mut g = Graphics::new(200, 40);
        visual(&p, &TargetInfo::detached(), &mut g, (0, 0, 200, 40));
        // right-hand side is background green (the text sits at the left of it), left is the picture
        assert_eq!(g.bitmap().get_pixel(199, 39), Color::GREEN);
        let left = g.bitmap().get_pixel(5, 20);
        assert!(left != Color::GREEN && left.a == 255);
        // lyric text is yellow
        let mut yellow = 0;
        for y in 0..40 {
            for x in 40..200 {
                let c = g.bitmap().get_pixel(x, y);
                if c.r > 200 && c.g > 200 && c.b < 100 {
                    yellow += 1;
                }
            }
        }
        assert!(yellow > 20, "{yellow}");
    }

    #[test]
    fn thumbnail_marks_mode_scales_pictures_over_the_effect() {
        let mut p = DESCRIPTOR.new_params();
        p.set("AllowMarkGaps", ParamValue::Bool(true));
        let mut m = Mark::new(TimeSpan::from_ms_i(500));
        m.duration = TimeSpan::from_ms_i(500);
        m.text = "MBP".into();
        let marks = [m];
        let mut g = Graphics::new(200, 40);
        draw_visual(&p, &mut g, (0, 0, 200, 40), &VisualContext { marks: &marks, start_time: TimeSpan::ZERO, time_span: TimeSpan::from_ms_i(1000) });
        // nothing before the mark, a picture over its half, and a closing line is drawn only if it ends early
        assert_eq!(g.bitmap().get_pixel(20, 20), Color::EMPTY);
        assert!(g.bitmap().get_pixel(150, 20).a > 0);
    }
}
