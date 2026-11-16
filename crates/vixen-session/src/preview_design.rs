//! What connects the design surface to the GPU: a [`DesignerHost`] feeds each [`Change`] from a `DesignSession` to the renderer
//! itself, so a window only forwards events and reads back a small summary. Light data never reaches the UI.
//!
//! Also the file side of `VixenPreviewControl`: background images and writing a preview back into the module store.

use crate::preview::{background_path, PreviewEntry, MODULE_FOLDER};
use crate::SessionError;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;
use vixen_core::Color;
use vixen_io::{DataPaths, ModuleEntry, SystemData};
use vixen_preview::overlay::{InfoLabel, Overlay};
use vixen_preview::session::{Change, Cursor, MenuItem, Tool, ToolbarState};
use vixen_preview::tools::{is_all_black, to_int32};
use vixen_preview::{DesignSession, PreviewData, ShapeId, VIXEN_PREVIEW_MODULE_TYPE};
use vixen_render::{rgba, BeamWidthModel, LightGeom, LiveRenderer, MovingHeadInstance, MovingHeadParams, OverlayLine, OverlaySquare};

const DATA_MODEL_TYPE: &str = "VixenModules.Preview.VixenPreview.VixenPreviewData, VixenPreview";
const ASSEMBLY_NAME: &str = "Module.Preview.VixenPreview";

#[inline]
fn gpu_color(c: Color) -> u32 {
    rgba(c.r, c.g, c.b, c.a)
}

/// The renderer's overlay from the session's: selection outlines and the rubber band. Fixtures use
/// the shared volume pass; labels stay with the window.
pub fn render_overlay(overlay: &Overlay) -> vixen_render::Overlay {
    let line = |l: &vixen_preview::overlay::OverlayLine| OverlayLine { a: l.a, b: l.b, color: gpu_color(l.color), width: l.width };
    let mut lines: Vec<OverlayLine> = Vec::with_capacity(overlay.lines.len() + 4 + overlay.fixtures.len() * 4);
    lines.extend(overlay.lines.iter().map(line));
    if let Some(band) = &overlay.rubber_band {
        lines.extend(band.lines().iter().map(line));
    }
    vixen_render::Overlay {
        lines,
        squares: overlay.squares.iter().map(|s| OverlaySquare { center: s.center, size: s.size, color: gpu_color(s.color) }).collect(),
        circles: overlay.circles.iter().map(|c| (c.center, c.radius, gpu_color(c.color))).collect(),
    }
}

/// What the window has to do after an event. Every field stays empty for a drag that only moved lights, so that costs no
/// UI work at all.
#[derive(Clone, Debug, Default)]
pub struct DesignerOutcome {
    pub cursor: Option<Cursor>,
    pub show_properties: Option<ShapeId>,
    pub clear_properties: bool,
    pub elements_deselected: bool,
    pub element_template: Option<(ShapeId, &'static str)>,
    pub context_menu: Option<Vec<MenuItem>>,
    pub zoom: Option<f64>,
    /// The session's scroll moved: scroll the scroll view there, without telling the session again.
    pub scroll: Option<(i32, i32)>,
    /// The scrolled document's size changed: the background's size at this zoom.
    pub content_size: Option<(i32, i32)>,
    pub tool: Option<Tool>,
    pub clipboard: Option<String>,
    pub message: Option<String>,
    pub lock_hint: bool,
    pub undo_changed: bool,
    /// Only when it differs from the last one given.
    pub toolbar: Option<ToolbarState>,
    /// Only while Prop Information is showing or just turned off.
    pub labels: Option<Vec<InfoLabel>>,
    /// The shape shown in Properties may have new values.
    pub refresh_properties: bool,
}

/// A `DesignSession` and the renderer it drives.
pub struct DesignerHost {
    pub session: DesignSession,
    renderer: Arc<LiveRenderer>,
    colors: Vec<u32>,
    /// Lights drawn larger than the scene says, in order: `LightRun::size_delta`, the first pixel of a pixel string.
    size_deltas: Vec<(u32, f32)>,
    view: (f32, f32, f32),
    scroll: (i32, i32),
    content_size: (i32, i32),
    toolbar: Option<ToolbarState>,
    labels_showing: bool,
    properties_shape: Option<ShapeId>,
    /// Pinch reaches the session as whole wheel units, and the rest carries to the next event.
    magnify_carry: f64,
}

impl DesignerHost {
    pub fn new(session: DesignSession, renderer: Arc<LiveRenderer>) -> DesignerHost {
        DesignerHost {
            session,
            renderer,
            colors: Vec::new(),
            size_deltas: Vec::new(),
            view: (f32::NAN, 0.0, 0.0),
            scroll: (0, 0),
            content_size: (0, 0),
            toolbar: None,
            labels_showing: false,
            properties_shape: None,
            magnify_carry: 0.0,
        }
    }

    pub fn renderer(&self) -> &Arc<LiveRenderer> {
        &self.renderer
    }

    /// Sends everything to the renderer, as for a new scene, and tells the window everything. For after opening.
    pub fn apply_all(&mut self) -> DesignerOutcome {
        self.content_size = (-1, -1);
        self.toolbar = None;
        let mut change = Change::default();
        change.scene.push(vixen_preview::SceneDelta { shape: None, range: 0..self.session.scene().light_count(), links_changed: true, resized: true });
        change.overlay_changed = true;
        change.light_colors_changed = true;
        self.apply(change)
    }

    /// `Hosting` in `vixen_preview`: geometry, overlay, colours and view to the renderer, and the rest for the window.
    pub fn apply(&mut self, change: Change) -> DesignerOutcome {
        let overlay = (change.overlay_changed || change.light_colors_changed).then(|| self.session.overlay());

        let old_deltas = if change.light_colors_changed {
            let new_deltas = overlay.as_ref().map_or_else(Vec::new, |o| {
                o.light_runs
                    .iter()
                    .filter(|r| r.size_delta != 0.0)
                    .flat_map(|r| {
                        let delta = r.size_delta;
                        (r.start..r.start + r.len).map(move |i| (i, delta))
                    })
                    .collect()
            });
            Some(std::mem::replace(&mut self.size_deltas, new_deltas))
        } else {
            None
        };

        self.send_geometry(&change, old_deltas.as_deref());
        if let Some(overlay) = &overlay {
            let heads = overlay.fixtures.iter().filter_map(|f| {
                let item = self.session.data().display_items.iter().find(|item| item.id == f.shape)?;
                let data = item.shape.moving_head()?;
                let top = data.mounting_position == vixen_preview::shape::MountingPosition::Top;
                let length = f.width.min(f.height);
                Some(MovingHeadInstance {
                    position: [f.x + f.width/2.0, f.y + f.height/2.0], length,
                    params: MovingHeadParams { mounted_top: top, tilt_offset_deg: if top { 0.0 } else { 180.0 },
                        color_left: gpu_color(f.beam_color), color_right: gpu_color(f.beam_color),
                        beam_width: BeamWidthModel::EditorTopPlusBottom, max_beam_length: length,
                        beam_length_pct: 20.0, thumbnail: true, ..Default::default() },
                })
            }).collect();
            self.renderer.set_moving_heads(heads);
            if change.overlay_changed {
                self.renderer.set_overlay(render_overlay(overlay));
            }
            if change.light_colors_changed {
                self.colors.resize(self.session.scene().light_count(), 0);
                overlay.fill_light_colors(&mut self.colors, 0, gpu_color);
                self.renderer.publish_colors(&self.colors);
            }
        }
        self.sync_view();

        let labels = overlay.as_ref().and_then(|o| {
            if o.labels.is_empty() && !self.labels_showing {
                return None;
            }
            self.labels_showing = !o.labels.is_empty();
            Some(o.labels.clone())
        });

        let scroll = self.session.scroll();
        let scroll_out = (scroll != self.scroll).then_some(scroll);
        self.scroll = scroll;
        let content = self.document_size();
        let content_out = (content != self.content_size).then_some(content);
        self.content_size = content;

        // Scans for locks. A drag sets none of these, so it doesn't pay for that.
        let state_may_differ = change.selection_changed || change.undo_changed || change.tool_changed || !change.shapes_added.is_empty() || !change.shapes_removed.is_empty() || change.light_colors_changed || self.toolbar.is_none();
        let toolbar = state_may_differ.then(|| self.session.toolbar_state()).filter(|t| self.toolbar != Some(*t));
        if let Some(t) = toolbar {
            self.toolbar = Some(t);
        }

        // A click on another shape deselects one and selects the other in one event, so what counts is where they end up. Selecting the same
        // shape again after a deselect within an event still shows it.
        let shown = self.session.properties_shape();
        let reshow = change.show_properties.is_some() && shown == self.properties_shape;
        let show_properties = shown.filter(|_| reshow || shown != self.properties_shape);
        let clear_properties = shown.is_none() && (self.properties_shape.is_some() || change.clear_properties);
        self.properties_shape = shown;

        DesignerOutcome {
            cursor: change.cursor,
            show_properties,
            clear_properties,
            elements_deselected: change.elements_deselected,
            element_template: change.element_template,
            context_menu: change.context_menu,
            zoom: change.zoom_changed,
            scroll: scroll_out,
            content_size: content_out,
            tool: change.tool_changed.then(|| self.session.tool()),
            clipboard: change.clipboard,
            message: change.message,
            lock_hint: change.lock_hint,
            undo_changed: change.undo_changed,
            toolbar,
            labels,
            refresh_properties: change.undo_changed || !change.scene.is_empty(),
        }
    }

    /// The scroll bars moved, or the session's zoom did. Only the view uniform changes, so nothing re-rasterizes.
    pub fn sync_view(&mut self) {
        let (h, v) = self.session.scroll();
        let view = (self.session.scene().zoom() as f32, -(h as f32), -(v as f32));
        if view != self.view {
            self.view = view;
            self.renderer.set_view(view.0, view.1, view.2);
        }
    }

    /// The scroll view's bounds changed. No outcome: the view it came from is already there.
    pub fn set_scroll(&mut self, h: i32, v: i32) {
        self.session.set_scroll(h, v);
        self.scroll = self.session.scroll();
        self.sync_view();
    }

    /// F52: a pinch. `delta = magnification * zoom * 1000` in the session's wheel units, with the fraction carried.
    pub fn magnify(&mut self, magnification: f64, x: i32, y: i32) -> DesignerOutcome {
        let exact = magnification * self.session.zoom() * 1000.0 + self.magnify_carry;
        let delta = exact.trunc();
        self.magnify_carry = exact - delta;
        if delta == 0.0 {
            return DesignerOutcome::default();
        }
        let change = self.session.mouse_wheel(delta as i32, x, y);
        self.apply(change)
    }

    /// The scrolled document: `background size x zoom`.
    pub fn document_size(&self) -> (i32, i32) {
        let zoom = self.session.zoom();
        self.session.background_size().map_or((0, 0), |(w, h)| (to_int32(w as f64 * zoom), to_int32(h as f64 * zoom)))
    }

    fn lights_for(&self, range: std::ops::Range<usize>) -> Vec<LightGeom> {
        let scene = self.session.scene();
        let Some(source) = scene.lights.get(range.clone()) else { return Vec::new() };
        let mut out: Vec<LightGeom> = source.iter().map(|l| LightGeom { x: l.x, y: l.y, size: l.size }).collect();
        let first = self.size_deltas.partition_point(|(i, _)| (*i as usize) < range.start);
        for (i, delta) in &self.size_deltas[first..] {
            if *i as usize >= range.end {
                break;
            }
            out[*i as usize - range.start].size += delta;
        }
        out
    }

    /// One upload per run of shapes next to each other, so a large selection is one copy and not thousands.
    fn send_geometry(&mut self, change: &Change, old_deltas: Option<&[(u32, f32)]>) {
        if change.scene.iter().any(|d| d.resized) {
            let all = self.lights_for(0..self.session.scene().light_count());
            self.renderer.set_lights(&all);
            return;
        }
        let mut ranges: Vec<std::ops::Range<usize>> = change.scene.iter().map(|d| d.range.clone()).filter(|r| !r.is_empty()).collect();
        // A light that stopped or started being the larger first pixel moves with no scene change.
        if let Some(old) = old_deltas {
            if old != self.size_deltas.as_slice() {
                ranges.extend(old.iter().chain(self.size_deltas.iter()).map(|(i, _)| *i as usize..*i as usize + 1));
            }
        }
        if ranges.is_empty() {
            return;
        }
        ranges.sort_unstable_by_key(|r| r.start);
        let mut current = ranges[0].clone();
        for r in &ranges[1..] {
            if r.start <= current.end {
                current.end = current.end.max(r.end);
            } else {
                let lights = self.lights_for(current.clone());
                self.renderer.update_light_range_vec(current.start, lights);
                current = r.clone();
            }
        }
        let lights = self.lights_for(current.clone());
        self.renderer.update_light_range_vec(current.start, lights);
    }
}

/// What opening a background image found. `LoadBackground`'s outcomes, which decide whether the file name stays.
#[derive(Debug)]
pub enum BackgroundLoad {
    /// No file there. The C# treats this as no background, silently.
    None,
    Image { rgba: Vec<u8>, width: u32, height: u32 },
    /// Every pixel is opaque black, from a past bug that saved black bitmaps. The file reference goes (F55).
    AllBlack,
    /// `There was an error loading the background image: {message}`.
    Failed(String),
}

/// The designer's load of a background (F55, F56). For a large photo this is the slow part of opening, so it belongs off the
/// main thread. The live windows use `load_background`, without the all-black check.
pub fn load_background_checked(path: &Path) -> BackgroundLoad {
    if !path.is_file() {
        return BackgroundLoad::None;
    }
    let image = match image::open(path) {
        Ok(image) => image.to_rgba8(),
        Err(error) => return BackgroundLoad::Failed(error.to_string()),
    };
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    if is_all_black(&rgba) {
        return BackgroundLoad::AllBlack;
    }
    BackgroundLoad::Image { rgba, width, height }
}

/// An image's size from its header, without decoding it, so the window has its numbers before the decode finishes.
pub fn background_dimensions(path: &Path) -> Option<(u32, u32)> {
    image::ImageReader::open(path).ok()?.with_guessed_format().ok()?.into_dimensions().ok()
}

/// `buttonSetBackground_Click`: copies the file into the module's data folder under a new `{guid}{ext}` name and returns that
/// name. The old file stays on disk, because a duplicated preview shares it.
pub fn import_background(paths: &DataPaths, source: &Path) -> Result<String, SessionError> {
    let extension = source.extension().map_or_else(String::new, |e| format!(".{}", e.to_string_lossy()));
    let name = format!("{}{}", Uuid::new_v4(), extension);
    let folder = paths.module_data_dir().join(MODULE_FOLDER);
    std::fs::create_dir_all(&folder).map_err(|e| SessionError::Other(format!("The module folder couldn't be created: {e}")))?;
    std::fs::copy(source, folder.join(&name)).map_err(|e| SessionError::Other(format!("The image couldn't be copied: {e}")))?;
    Ok(name)
}

/// `ResizeImageBackground`'s image half: a high quality resample, always written as a new `{guid}.jpg`. Runs off the main thread.
pub fn resize_background_image(paths: &DataPaths, file_name: &str, width: u32, height: u32) -> Result<String, SessionError> {
    let image = image::open(background_path(paths, file_name)).map_err(|e| SessionError::Other(format!("The image couldn't be read: {e}")))?;
    let resized = image.resize_exact(width, height, image::imageops::FilterType::CatmullRom);
    let name = format!("{}.jpg", Uuid::new_v4());
    let folder = paths.module_data_dir().join(MODULE_FOLDER);
    std::fs::create_dir_all(&folder).map_err(|e| SessionError::Other(format!("The module folder couldn't be created: {e}")))?;
    // JPEG has no alpha. The canvas composites over black, so the flattening does too.
    let mut flat = image::RgbImage::new(width, height);
    let source = resized.to_rgba8();
    for (out, p) in flat.pixels_mut().zip(source.pixels()) {
        let a = p[3] as u32;
        *out = image::Rgb([(p[0] as u32 * a / 255) as u8, (p[1] as u32 * a / 255) as u8, (p[2] as u32 * a / 255) as u8]);
    }
    flat.save_with_format(folder.join(&name), image::ImageFormat::Jpeg).map_err(|e| SessionError::Other(format!("The image couldn't be written: {e}")))?;
    Ok(name)
}

/// Writes a preview back into the module store. Everything it doesn't understand comes back unchanged through `to_xml`.
pub fn save_preview_data(system: &mut SystemData, entry: &PreviewEntry, data: &PreviewData) {
    let xml = data.to_xml();
    match system.module_store.instance_mut(entry.type_id, entry.id) {
        Some(existing) => existing.set_data(xml),
        None => {
            let type_id = if entry.type_id.is_nil() { VIXEN_PREVIEW_MODULE_TYPE } else { entry.type_id };
            system.module_store.set_instance(ModuleEntry::new(DATA_MODEL_TYPE, &type_id, &entry.id, ASSEMBLY_NAME, xml));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_preview::overlay::theme;
    use vixen_preview::point::PreviewPoint;
    use vixen_preview::shape::{Modifiers, Shape, StringType};
    use vixen_preview::Key;

    fn headless(w: u32, h: u32) -> Option<Arc<LiveRenderer>> {
        LiveRenderer::headless(w, h, 1.0).ok().map(Arc::new)
    }

    fn lit(pixels: &[u8], w: usize, x: usize, y: usize) -> [u8; 4] {
        pixels[(y * w + x) * 4..][..4].try_into().unwrap()
    }

    fn line_session() -> DesignSession {
        let mut d = PreviewData::new();
        d.add_shape(Shape::new_line(PreviewPoint::new(10, 10), PreviewPoint::new(60, 10), 6, None, 1.0));
        DesignSession::new(d, None)
    }

    #[test]
    fn a_line_shows_in_its_edit_colour_and_a_drag_moves_it_on_the_gpu() {
        let Some(renderer) = headless(100, 100) else { return };
        let mut host = DesignerHost::new(line_session(), renderer.clone());
        host.session.set_background_size(100, 100);
        host.session.set_view_size(100, 100);
        host.apply_all();
        let pixels = renderer.read_pixels();
        let white = [255, 255, 255, 255];
        assert_eq!(lit(&pixels, 100, 10, 10), white, "unlinked lights are white");

        // Select, then drag down 30.
        let c = host.session.mouse_down(30, 10, Modifiers::NONE);
        let o = host.apply(c);
        assert!(o.show_properties.is_some() && !o.clear_properties);
        let c = host.session.mouse_up(30, 10, Modifiers::NONE);
        host.apply(c);
        let pixels = renderer.read_pixels();
        let green = [theme::SELECTED.r, theme::SELECTED.g, theme::SELECTED.b, 255];
        assert_eq!(lit(&pixels, 100, 10, 10), green, "selected lights are lime green");

        let c = host.session.mouse_down(30, 10, Modifiers::NONE);
        host.apply(c);
        let c = host.session.mouse_move(30, 40, Modifiers::NONE);
        let o = host.apply(c);
        assert!(o.toolbar.is_none() && o.message.is_none() && o.cursor.is_none(), "a drag costs the window nothing");
        let c = host.session.mouse_up(30, 40, Modifiers::NONE);
        let o = host.apply(c);
        assert!(o.undo_changed && o.refresh_properties);
        let pixels = renderer.read_pixels();
        assert_eq!(lit(&pixels, 100, 10, 40), green);
        assert_eq!(lit(&pixels, 100, 10, 10), [0, 0, 0, 255], "gone from where it was");
    }

    #[test]
    fn clicking_from_one_shape_to_another_ends_up_showing_the_second() {
        let Some(renderer) = headless(100, 100) else { return };
        let mut d = PreviewData::new();
        let first = d.add_shape(Shape::new_line(PreviewPoint::new(10, 10), PreviewPoint::new(60, 10), 6, None, 1.0));
        let second = d.add_shape(Shape::new_line(PreviewPoint::new(10, 50), PreviewPoint::new(60, 50), 6, None, 1.0));
        let mut host = DesignerHost::new(DesignSession::new(d, None), renderer);
        let c = host.session.mouse_down(30, 10, Modifiers::NONE);
        assert_eq!(host.apply(c).show_properties, Some(first));
        let c = host.session.mouse_up(30, 10, Modifiers::NONE);
        assert!(host.apply(c).show_properties.is_none());
        // One event deselects the first and selects the second.
        let c = host.session.mouse_down(30, 50, Modifiers::NONE);
        let o = host.apply(c);
        assert_eq!(o.show_properties, Some(second));
        assert!(!o.clear_properties);
        let c = host.session.mouse_up(30, 50, Modifiers::NONE);
        host.apply(c);
        // And a click on nothing leaves none.
        let c = host.session.mouse_down(90, 90, Modifiers::NONE);
        let o = host.apply(c);
        assert!(o.clear_properties && o.show_properties.is_none());
    }

    #[test]
    fn hidden_locked_lights_draw_nothing_and_the_first_pixel_of_a_pixel_string_is_larger() {
        let Some(renderer) = headless(100, 100) else { return };
        let mut d = PreviewData::new();
        let mut s = Shape::new_line(PreviewPoint::new(20, 50), PreviewPoint::new(80, 50), 4, None, 1.0);
        s.set_string_type(StringType::Pixel);
        s.set_pixel_size(4);
        d.add_shape(s);
        let mut host = DesignerHost::new(DesignSession::new(d, None), renderer.clone());
        host.apply_all();
        let pixels = renderer.read_pixels();
        let yellow = [255, 255, 0, 255];
        // Size 4 reaches 2 from the centre, and the first pixel's extra 2 reaches 3.
        assert_eq!(lit(&pixels, 100, 20, 50), yellow);
        assert!(lit(&pixels, 100, 23, 50)[0] > 150, "the disc's edge is anti-aliased");
        assert_eq!(lit(&pixels, 100, 83, 50), [0, 0, 0, 255], "the other lights aren't larger");

        host.session.select_all();
        let c = host.session.lock_selected();
        let o = host.apply(c);
        assert!(o.lock_hint);
        assert!(o.toolbar.unwrap().unlock_all);
        // A locked pixel string has no yellow first pixel, so it goes back to its normal size.
        let pixels = renderer.read_pixels();
        assert_eq!(lit(&pixels, 100, 23, 50), [0, 0, 0, 255]);

        let c = host.session.set_hide_locked(true);
        host.apply(c);
        let pixels = renderer.read_pixels();
        assert_eq!(lit(&pixels, 100, 20, 50), [0, 0, 0, 255]);
    }

    #[test]
    fn the_scroll_and_zoom_reach_the_renderer_and_the_window_only_hears_of_real_changes() {
        let Some(renderer) = headless(100, 100) else { return };
        let mut host = DesignerHost::new(line_session(), renderer.clone());
        host.session.set_background_size(400, 300);
        host.session.set_view_size(100, 100);
        let first = host.session.set_tool(Tool::Select);
        host.apply(first);
        let o = host.apply_all();
        assert_eq!(o.content_size, Some((400, 300)), "even when an earlier change already told it");
        assert!(o.toolbar.is_some());
        host.set_scroll(5, 5);
        let pixels = renderer.read_pixels();
        assert_eq!(lit(&pixels, 100, 5, 5), [255, 255, 255, 255], "the first light at (10, 10) is 5 in");

        let c = host.session.key_down(Key::Plus, Modifiers::CTRL);
        let o = host.apply(c);
        assert_eq!(o.zoom, Some(1.25));
        assert_eq!(o.content_size, Some((500, 375)));
        let again = host.apply(Change::default());
        assert!(again.scroll.is_none() && again.content_size.is_none() && again.toolbar.is_none());
    }

    #[test]
    fn a_pinch_carries_what_is_less_than_a_wheel_unit() {
        let Some(renderer) = headless(16, 16) else { return };
        let mut host = DesignerHost::new(line_session(), renderer);
        let o = host.magnify(0.0004, 0, 0);
        assert!(o.zoom.is_none());
        let o = host.magnify(0.0007, 0, 0);
        assert!((o.zoom.unwrap() - 1.001).abs() < 1e-9);
    }

    #[test]
    fn background_files() {
        let dir = std::env::temp_dir().join(format!("vixen-bg-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let paths = DataPaths::new(&dir);
        assert!(matches!(load_background_checked(&dir.join("missing.png")), BackgroundLoad::None));

        let bad = dir.join("bad.png");
        std::fs::write(&bad, b"not a png").unwrap();
        assert!(matches!(load_background_checked(&bad), BackgroundLoad::Failed(_)));

        let black = dir.join("black.png");
        image::RgbaImage::from_pixel(4, 4, image::Rgba([0, 0, 0, 255])).save(&black).unwrap();
        assert!(matches!(load_background_checked(&black), BackgroundLoad::AllBlack));

        let good = dir.join("good.png");
        image::RgbaImage::from_pixel(40, 20, image::Rgba([10, 20, 30, 255])).save(&good).unwrap();
        assert!(matches!(load_background_checked(&good), BackgroundLoad::Image { width: 40, height: 20, .. }));
        assert_eq!(background_dimensions(&good), Some((40, 20)));

        let name = import_background(&paths, &good).unwrap();
        assert!(name.ends_with(".png"));
        assert!(background_path(&paths, &name).is_file());
        let resized = resize_background_image(&paths, &name, 20, 10).unwrap();
        assert!(resized.ends_with(".jpg"));
        assert_eq!(background_dimensions(&background_path(&paths, &resized)), Some((20, 10)));
        assert!(background_path(&paths, &name).is_file(), "the old file stays");
        std::fs::remove_dir_all(&dir).ok();
    }
}
