//! The live windows' state and view maths, without a window: `GDIPreviewForm` and `OpenGlPreviewForm` as one. Which view a
//! window has comes from `UseOpenGL`: the flat view zooms about the origin, and the camera view has a perspective camera that maps
//! exactly onto a 2D zoom and pan, because every light is on one plane.

use crate::logging;
use std::sync::Arc;
use uuid::Uuid;
use vixen_io::{ProfileSettings, SettingSection};
use vixen_preview::tools::{window_is_recoverable, Rect};
use vixen_render::LiveRenderer;

/// `Fov`, in degrees.
const FOV_DEGREES: f64 = 45.0;
const FAR_DISTANCE: f32 = 6000.0;
const NEAR_DISTANCE: f32 = 1.0;
/// `SystemInformation.MouseWheelScrollLines`, which has no equivalent on the Mac.
const WHEEL_SCROLL_LINES: i32 = 3;
const FLAT_ZOOM_MIN: f64 = 0.25;
const FLAT_ZOOM_MAX: f64 = 2.0;

/// Reset Size adds this much for the flat view's status bar, in points. It's the height the bar is drawn at.
pub const STATUS_BAR_HEIGHT: f64 = 22.0;

/// Where the camera is. y is up in the world.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CameraView {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// `_focalDepth`: the camera distance at which a scene pixel is one screen pixel.
pub fn focal_depth(client_height: f32) -> f32 {
    (1.0 / (FOV_DEGREES.to_radians() / 2.0).tan() * (client_height as f64 / 2.0)) as f32
}

/// `CreateCamera` with nothing saved.
pub fn camera_default(background: Option<(u32, u32)>, client: (f32, f32)) -> CameraView {
    let (w, h) = background.map_or(client, |(w, h)| (w as f32, h as f32));
    CameraView { x: w / 2.0, y: h / 2.0, z: focal_depth(client.1) }
}

/// The 2D view the camera comes to: `zoom = focal / z`, valid while all lights have z 0. `reference_height` is what a light's y
/// is measured from: the background's height, or the form's whole height without one, so without a background resizing the
/// window shifts the lights. Kept.
pub fn camera_to_view(camera: &CameraView, client: (f32, f32), reference_height: f32) -> (f32, f32, f32) {
    let zoom = focal_depth(client.1) / camera.z.max(f32::EPSILON);
    (zoom, client.0 / 2.0 - camera.x * zoom, client.1 / 2.0 - (reference_height - camera.y) * zoom)
}

/// `Zoom`: moves along z by a whole number of units, with the limits worked out in whole units too, so z can stop just off them.
fn camera_zoom(camera: &mut CameraView, direction: i32) {
    let distance = camera.z + direction as f32;
    let direction = if distance > FAR_DISTANCE {
        (FAR_DISTANCE - camera.z).round() as i32
    } else if distance < NEAR_DISTANCE {
        NEAR_DISTANCE as i32 - camera.z as i32
    } else {
        direction
    };
    camera.z += direction as f32;
}

/// The wheel: `-(delta * lines / factor)` in integer division, with a bigger step on Shift.
pub fn camera_wheel(camera: &mut CameraView, delta: i32, shift: bool) {
    let factor = if shift { 5 } else { 20 };
    camera_zoom(camera, -(delta.wrapping_mul(WHEEL_SCROLL_LINES) / factor));
}

/// A left drag: `dx` and `dy` are the pointer's previous position less its current one. The picture doesn't move one to one with
/// the pointer, and that grows with the window. Kept.
pub fn camera_drag(camera: &mut CameraView, dx: f32, dy: f32) {
    let factor = (camera.z / FAR_DISTANCE) * 7.0;
    camera.x += dx * factor;
    camera.y += -dy * factor;
}

/// `CalculatePointScaleFactor`. `focal` is the stored `_focalDepth`, which only opening and Reset Size update.
pub fn point_scale(camera: &CameraView, focal: f32, client: (f32, f32), background: Option<(u32, u32)>, enabled: bool) -> f32 {
    if !enabled {
        return 1.0;
    }
    let size_scale = background.map_or(1.0, |(w, h)| (client.0 / w.max(1) as f32 + client.1 / h.max(1) as f32) / 2.0);
    (focal / camera.z.max(f32::EPSILON) * size_scale).max(0.1)
}

/// `ZoomLevel`'s setter in the flat view, whose maximum is 2 where the design canvas allows 4.
pub fn flat_zoom_step(zoom: f64, delta: f64) -> f64 {
    (zoom + delta).clamp(FLAT_ZOOM_MIN, FLAT_ZOOM_MAX)
}

/// The window's saved state. Positions are Windows-style: top left, y down, in points. The flat view keeps its frame size and the
/// camera view its client size, each under its own key group, so flipping `UseOpenGL` shows the other group's state.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveWindowState {
    pub location: Option<(i32, i32)>,
    /// Frame size in the flat view, client size in the camera view. 0 means not saved.
    pub width: i32,
    pub height: i32,
    /// "Normal", "Maximized" or "Minimized". The flat view only.
    pub window_state: String,
    pub show_status: bool,
    pub always_on_top: bool,
    pub on_top_when_active: bool,
    pub transparent: bool,
    pub show_borders: bool,
    pub lock_position: bool,
    pub zoom_level: f64,
    pub camera: Option<CameraView>,
    pub enable_light_scaling: bool,
}

impl Default for LiveWindowState {
    fn default() -> LiveWindowState {
        LiveWindowState {
            location: None,
            width: 0,
            height: 0,
            window_state: "Normal".to_string(),
            show_status: true,
            always_on_top: false,
            on_top_when_active: false,
            transparent: false,
            show_borders: true,
            lock_position: false,
            zoom_level: 1.0,
            camera: None,
            enable_light_scaling: true,
        }
    }
}

fn group(id: &Uuid, camera: bool) -> String {
    if camera {
        format!("OpenGLPreview_{id}")
    } else {
        format!("Preview_{id}")
    }
}

/// `RestoreWindowState`'s reads.
pub fn load_live_window_state(settings: &ProfileSettings, id: &Uuid, camera: bool) -> LiveWindowState {
    let name = group(id, camera);
    let section = SettingSection::AppSettings;
    let key = |k: &str| format!("{name}/{k}");
    const UNSET: i32 = i32::MIN;
    let x = settings.get_i32(section, &key("WindowLocationX"), UNSET);
    let y = settings.get_i32(section, &key("WindowLocationY"), UNSET);
    let d = LiveWindowState::default();
    let mut state = LiveWindowState {
        location: (x != UNSET && y != UNSET).then_some((x, y)),
        show_status: settings.get_bool(section, &key("ShowStatus"), d.show_status),
        always_on_top: settings.get_bool(section, &key("AlwaysOnTop"), d.always_on_top),
        on_top_when_active: settings.get_bool(section, &key("OnTopWhenActive"), d.on_top_when_active),
        transparent: settings.get_bool(section, &key("TransparentBackground"), d.transparent),
        ..d
    };
    if camera {
        state.width = settings.get_i32(section, &key("ClientWidth"), 0);
        state.height = settings.get_i32(section, &key("ClientHeight"), 0);
        state.enable_light_scaling = settings.get_bool(section, &key("EnableLightScaling"), true);
        const NOT_SAVED: f32 = f32::MIN;
        let cx = settings.get_f32(section, &key("CameraPositionX"), NOT_SAVED);
        let cy = settings.get_f32(section, &key("CameraPositionY"), NOT_SAVED);
        let cz = settings.get_f32(section, &key("CameraPositionZ"), NOT_SAVED);
        state.camera = (cx != NOT_SAVED && cy != NOT_SAVED && cz != NOT_SAVED).then_some(CameraView { x: cx, y: cy, z: cz });
    } else {
        state.width = settings.get_i32(section, &key("WindowWidth"), 0);
        state.height = settings.get_i32(section, &key("WindowHeight"), 0);
        state.window_state = settings.get_string(section, &key("WindowState"), "Normal");
        state.show_borders = settings.get_bool(section, &key("ShowBorders"), true);
        state.lock_position = settings.get_bool(section, &key("LockPosition"), false);
        state.zoom_level = settings.get_f64(section, &key("ZoomLevel"), 1.0);
    }
    state
}

/// `SaveWindowState`, as one write. Upstream writes the whole file once per key, per move event. Size and location stay out while
/// minimized.
pub fn save_live_window_state(settings: &ProfileSettings, id: &Uuid, camera: bool, state: &LiveWindowState) -> Result<(), vixen_io::IoError> {
    let name = group(id, camera);
    let flag = |b: bool| if b { "True".to_string() } else { "False".to_string() };
    let mut values: Vec<(String, String)> = Vec::with_capacity(14);
    let mut put = |key: &str, value: String| values.push((format!("{name}/{key}"), value));
    let minimized = state.window_state == "Minimized";
    if !minimized {
        put(if camera { "ClientHeight" } else { "WindowHeight" }, state.height.to_string());
        put(if camera { "ClientWidth" } else { "WindowWidth" }, state.width.to_string());
        if let Some((x, y)) = state.location {
            put("WindowLocationX", x.to_string());
            put("WindowLocationY", y.to_string());
        }
    }
    put("ShowStatus", flag(state.show_status));
    put("AlwaysOnTop", flag(state.always_on_top));
    put("OnTopWhenActive", flag(state.on_top_when_active));
    put("TransparentBackground", flag(state.transparent));
    if camera {
        if let Some(c) = state.camera {
            put("CameraPositionX", c.x.to_string());
            put("CameraPositionY", c.y.to_string());
            put("CameraPositionZ", c.z.to_string());
        }
        put("EnableLightScaling", flag(state.enable_light_scaling));
    } else {
        put("WindowState", state.window_state.clone());
        put("ShowBorders", flag(state.show_borders));
        put("LockPosition", flag(state.lock_position));
        put("ZoomLevel", state.zoom_level.to_string());
    }
    let refs: Vec<(&str, String)> = values.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
    settings.put_many(SettingSection::AppSettings, &refs)
}

/// Where a live window opens.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RestoredBounds {
    /// None for the system's default placement.
    pub location: Option<(i32, i32)>,
    /// Frame size in the flat view, client size in the camera view.
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
}

/// `RestoreWindowState`'s decisions, in points. `frame_extra` is the frame's size beyond its content. `blank_client` is the blank
/// background's size on a flat window's first open with no image: the designer's canvas size, which is why upstream's 400 and 300
/// are effectively unreachable.
pub fn restore_bounds(state: &LiveWindowState, camera: bool, background: Option<(u32, u32)>, shapes_extent: (i32, i32), screens: &[Rect], frame_extra: (i32, i32)) -> RestoredBounds {
    if camera {
        let (mut w, mut h) = (state.width, state.height);
        if let Some((bw, bh)) = background {
            if w == 0 {
                w = bw as i32;
            }
            if h == 0 {
                h = bh as i32;
            }
        }
        if h <= 0 || w <= 0 {
            // FindMaxPreviewSize when both are over 50, otherwise the form's first size.
            let (sw, sh) = if shapes_extent.0 > 50 && shapes_extent.1 > 50 { shapes_extent } else { (800, 600) };
            h = if h > 0 { h } else { sh };
            w = if w > 0 { w } else { sw };
        }
        let location = state.location.filter(|(x, y)| window_is_recoverable(Rect::new(*x, *y, w, h), screens));
        return RestoredBounds { location, width: w, height: h, maximized: false };
    }
    let (mut w, mut h) = (state.width, state.height);
    // The status bar's height goes on only when it shows, as `statusStrip.Visible` decides.
    let status = if state.show_status { STATUS_BAR_HEIGHT as i32 } else { 0 };
    let (bw, bh) = background.map_or((896, 428), |(bw, bh)| (bw as i32, bh as i32));
    if w < 20 {
        w = if bw > 20 { bw + frame_extra.0 } else { 400 };
    }
    if h < 10 {
        h = if bh > 10 { bh + frame_extra.1 + status } else { 300 };
    }
    let location = state.location.unwrap_or((i32::MIN / 2, i32::MIN / 2));
    let bounds = Rect::new(location.0, location.1, w, h);
    if state.window_state == "Maximized" && screens.iter().any(|s| intersects(*s, bounds)) {
        return RestoredBounds { location: Some(location), width: w, height: h, maximized: true };
    }
    if state.location.is_some() && window_is_recoverable(bounds, screens) {
        return RestoredBounds { location: Some(location), width: w, height: h, maximized: false };
    }
    RestoredBounds { location: None, width: w, height: h, maximized: false }
}

fn intersects(a: Rect, b: Rect) -> bool {
    a.x < b.x.saturating_add(b.width) && b.x < a.x.saturating_add(a.width) && a.y < b.y.saturating_add(b.height) && b.y < a.y.saturating_add(a.height)
}

/// One window's view, driving its renderer.
pub struct LiveView {
    pub camera_mode: bool,
    pub state: LiveWindowState,
    pub camera: CameraView,
    focal: f32,
    pub background: Option<(u32, u32)>,
    /// The area the canvas shows, in points: the whole content area in the camera view, above the status bar in the flat one.
    client: (f32, f32),
    frame_height: f32,
    renderer: Arc<LiveRenderer>,
}

impl LiveView {
    pub fn new(camera_mode: bool, state: LiveWindowState, background: Option<(u32, u32)>, client: (f32, f32), frame_height: f32, renderer: Arc<LiveRenderer>) -> LiveView {
        let view = LiveView {
            camera_mode,
            camera: state.camera.unwrap_or_else(|| camera_default(background, client)),
            focal: focal_depth(client.1),
            state,
            background,
            client,
            frame_height,
            renderer,
        };
        view.apply();
        view.renderer.set_transparent(view.state.transparent);
        view
    }

    /// The canvas changed size, or the window did, in points.
    pub fn set_client(&mut self, client: (f32, f32), frame_height: f32) {
        // Height 0 is forced to 100 upstream, which guards the camera's division.
        self.client = (client.0, if client.1 <= 0.0 { 100.0 } else { client.1 });
        self.frame_height = frame_height;
        self.apply();
    }

    pub fn client(&self) -> (f32, f32) {
        self.client
    }

    /// Projects a fixture legend with the same perspective and plane origin as its GPU mesh.
    pub fn project_fixture(&self, point: [f32; 3]) -> Option<(f32, f32, f32)> {
        if !self.camera_mode { return None; }
        let distance = self.camera.z - point[2];
        if !(1.0..6000.0).contains(&distance) { return None; }
        let reference = self.background.map_or(self.frame_height, |(_, h)| h as f32);
        let scale = focal_depth(self.client.1) / distance;
        Some((self.client.0/2.0+(point[0]-self.camera.x)*scale,
            self.client.1/2.0+(point[1]-(reference-self.camera.y))*scale,scale))
    }

    /// The view values to the renderer.
    fn apply(&self) {
        if self.camera_mode {
            let reference = self.background.map_or(self.frame_height, |(_, h)| h as f32);
            let (zoom, x, y) = camera_to_view(&self.camera, self.client, reference);
            self.renderer.set_view(zoom, x, y);
            self.renderer.set_camera_depth(self.camera.z);
            let reference_size = self.background.map_or((self.client.0, self.frame_height), |(w,h)| (w as f32,h as f32));
            self.renderer.set_fixture_reference(reference_size.0,reference_size.1);
            self.renderer.set_light_scale(point_scale(&self.camera, self.focal, self.client, self.background, self.state.enable_light_scaling));
        } else {
            self.renderer.set_camera_depth(0.0);
            self.renderer.set_light_scale(0.0);
            self.renderer.set_view(self.state.zoom_level as f32, 0.0, 0.0);
        }
    }

    /// A wheel notch (120) or a share of one. `I` and `O` come here too, with ±120.
    pub fn wheel(&mut self, delta: i32, shift: bool) {
        if self.camera_mode {
            camera_wheel(&mut self.camera, delta, shift);
            self.state.camera = Some(self.camera);
        } else {
            self.state.zoom_level = flat_zoom_step(self.state.zoom_level, delta as f64 / 1500.0);
        }
        self.apply();
    }

    /// `I` and `O` in the flat view: .05, twice that with Shift. The camera view uses the wheel's formula.
    pub fn zoom_key(&mut self, zoom_in: bool, shift: bool) {
        if self.camera_mode {
            self.wheel(if zoom_in { 120 } else { -120 }, shift);
        } else {
            let step = if shift { 0.10 } else { 0.05 };
            self.state.zoom_level = flat_zoom_step(self.state.zoom_level, if zoom_in { step } else { -step });
            self.apply();
        }
    }

    pub fn drag(&mut self, dx: f32, dy: f32) {
        if self.camera_mode {
            camera_drag(&mut self.camera, dx, dy);
            self.state.camera = Some(self.camera);
            self.apply();
        }
    }

    /// Reset Zoom, in the flat view.
    pub fn reset_zoom(&mut self) {
        self.state.zoom_level = 1.0;
        self.apply();
    }

    /// Reset Size. Returns the content size to give the window. The camera view needs a background, and leaves no room for the
    /// status bar, which covers the image's bottom edge. With no image the flat view's "background" is the control's own size, so
    /// at any zoom but 1 each Reset Size scales the window by the zoom again. Kept.
    pub fn reset_size(&mut self) -> Option<(f64, f64)> {
        if self.camera_mode {
            let (w, h) = self.background?;
            let client = (w as f32, h as f32);
            self.client = client;
            self.focal = focal_depth(client.1);
            self.camera = CameraView { x: client.0 / 2.0, y: client.1 / 2.0, z: self.focal };
            self.state.camera = Some(self.camera);
            self.apply();
            Some((client.0 as f64, client.1 as f64))
        } else {
            let zoom = self.state.zoom_level;
            let (w, h) = self.background.map_or((self.client.0 as f64, self.client.1 as f64), |(w, h)| (w as f64, h as f64));
            let bar = if self.state.show_status { STATUS_BAR_HEIGHT } else { 0.0 };
            Some(((w * zoom).trunc(), (h * zoom).trunc() + bar))
        }
    }

    /// Center Preview: the camera over the middle of the window, not the background. z stays.
    pub fn center(&mut self) {
        self.camera.x = self.client.0 / 2.0;
        self.camera.y = self.client.1 / 2.0;
        self.state.camera = Some(self.camera);
        self.apply();
    }

    pub fn set_light_scaling(&mut self, on: bool) {
        self.state.enable_light_scaling = on;
        self.apply();
    }

    pub fn set_transparent(&mut self, on: bool) {
        self.state.transparent = on;
        self.renderer.set_transparent(on);
    }

    pub fn reload_background(&mut self, background: Option<(u32, u32)>) {
        self.background = background;
        self.apply();
    }

    pub fn save(&self, id: &Uuid) {
        if let Err(error) = save_live_window_state(&ProfileSettings::shared(), id, self.camera_mode, &self.state) {
            logging::error("VixenPreview", "Couldn't save a preview's window state.", Some(&error.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Rect = Rect { x: 0, y: 0, width: 1920, height: 1080 };

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn focal_depth_and_the_default_camera_put_a_scene_pixel_on_a_screen_pixel() {
        assert!(close(focal_depth(800.0), 965.685));
        let camera = camera_default(Some((1000, 800)), (1000.0, 800.0));
        assert_eq!((camera.x, camera.y), (500.0, 400.0));
        let (zoom, x, y) = camera_to_view(&camera, (1000.0, 800.0), 800.0);
        assert!(close(zoom, 1.0) && close(x, 0.0) && close(y, 0.0));
    }

    #[test]
    fn the_wheel_moves_z_by_whole_units_and_stops_by_the_same_rule() {
        let mut c = CameraView { x: 0.0, y: 0.0, z: 1000.0 };
        camera_wheel(&mut c, 120, false);
        assert_eq!(c.z, 982.0);
        camera_wheel(&mut c, 120, true);
        assert_eq!(c.z, 910.0);
        camera_wheel(&mut c, -120, false);
        assert_eq!(c.z, 928.0);
        // Going in from 4.5: the limit is worked out from the truncated z.
        let mut c = CameraView { x: 0.0, y: 0.0, z: 4.5 };
        camera_wheel(&mut c, 120, false);
        assert_eq!(c.z, 1.5);
        // Going out: rounded from the fraction, so it can land half a unit past.
        let mut c = CameraView { x: 0.0, y: 0.0, z: 5999.5 };
        camera_wheel(&mut c, -120, false);
        assert_eq!(c.z, 6000.5);
    }

    #[test]
    fn a_drag_moves_the_camera_against_the_pointer_with_its_distance() {
        let mut c = CameraView { x: 0.0, y: 0.0, z: 6000.0 };
        camera_drag(&mut c, 1.0, 1.0);
        assert!(close(c.x, 7.0) && close(c.y, -7.0));
        // On screen it's 7 * focal / 6000 pixels per pointer pixel, whatever the distance.
        let focal = focal_depth(800.0);
        let mut near = CameraView { x: 0.0, y: 0.0, z: 500.0 };
        let before = camera_to_view(&near, (800.0, 800.0), 800.0).1;
        camera_drag(&mut near, -1.0, 0.0);
        let after = camera_to_view(&near, (800.0, 800.0), 800.0).1;
        assert!(close(after - before, 7.0 * focal / 6000.0));
    }

    #[test]
    fn point_scale_rules() {
        let focal = focal_depth(600.0);
        let c = CameraView { x: 0.0, y: 0.0, z: focal };
        assert_eq!(point_scale(&c, focal, (800.0, 600.0), Some((800, 600)), false), 1.0);
        assert!(close(point_scale(&c, focal, (800.0, 600.0), Some((800, 600)), true), 1.0));
        assert!(close(point_scale(&c, focal, (1600.0, 1200.0), Some((800, 600)), true), 2.0));
        let far = CameraView { x: 0.0, y: 0.0, z: 1e7 };
        assert_eq!(point_scale(&far, focal, (800.0, 600.0), None, true), 0.1);
    }

    #[test]
    fn flat_zoom_clamps_at_two() {
        assert_eq!(flat_zoom_step(1.95, 0.1), 2.0);
        assert_eq!(flat_zoom_step(0.3, -0.1), 0.25);
        assert!((flat_zoom_step(1.0, 120.0 / 1500.0) - 1.08).abs() < 1e-9);
    }

    #[test]
    fn flat_restore_rules() {
        let frame = (0, 28);
        let saved = |w, h| LiveWindowState { width: w, height: h, location: Some((100, 100)), ..LiveWindowState::default() };
        // Width 10 with a background 800 wide is the background plus the frame.
        let b = restore_bounds(&saved(10, 400), false, Some((800, 600)), (0, 0), &[SCREEN], frame);
        assert_eq!((b.width, b.height, b.location), (800, 400, Some((100, 100))));
        // No image and no saved size: the blank bitmap's size.
        let b = restore_bounds(&LiveWindowState::default(), false, None, (0, 0), &[SCREEN], frame);
        assert_eq!((b.width, b.height, b.location), (896, 428 + 28 + 22, None));
        // Maximized and on a screen.
        let mut s = saved(800, 600);
        s.window_state = "Maximized".to_string();
        assert!(restore_bounds(&s, false, None, (0, 0), &[SCREEN], frame).maximized);
        // Off screen: default place, keeping the size.
        let mut s = saved(800, 600);
        s.location = Some((5000, 5000));
        let b = restore_bounds(&s, false, None, (0, 0), &[SCREEN], frame);
        assert_eq!((b.location, b.width, b.height), (None, 800, 600));
    }

    #[test]
    fn camera_restore_rules() {
        let frame = (0, 28);
        let none = LiveWindowState::default();
        let b = restore_bounds(&none, true, Some((800, 600)), (0, 0), &[SCREEN], frame);
        assert_eq!((b.width, b.height), (800, 600));
        let b = restore_bounds(&none, true, None, (1000, 700), &[SCREEN], frame);
        assert_eq!((b.width, b.height), (1000, 700));
        let b = restore_bounds(&none, true, None, (40, 700), &[SCREEN], frame);
        assert_eq!((b.width, b.height), (800, 600));
    }

    #[test]
    fn settings_round_trip_under_upstreams_keys() {
        let dir = std::env::temp_dir().join(format!("vixen-live-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let settings = ProfileSettings::open(dir.join("Settings.xml"));
        let id = Uuid::new_v4();
        assert_eq!(load_live_window_state(&settings, &id, false), LiveWindowState::default());

        let flat = LiveWindowState { location: Some((3, 4)), width: 500, height: 400, show_borders: false, zoom_level: 1.5, always_on_top: true, ..LiveWindowState::default() };
        save_live_window_state(&settings, &id, false, &flat).unwrap();
        assert_eq!(load_live_window_state(&settings, &id, false), flat);
        assert_eq!(settings.get_i32(SettingSection::AppSettings, &format!("Preview_{id}/WindowWidth"), 0), 500);
        assert_eq!(settings.get_string(SettingSection::AppSettings, &format!("Preview_{id}/ShowBorders"), ""), "False");

        // The other view has its own state.
        assert_eq!(load_live_window_state(&settings, &id, true).width, 0);
        let camera = LiveWindowState { width: 640, height: 480, camera: Some(CameraView { x: 1.5, y: 2.5, z: 900.25 }), enable_light_scaling: false, ..LiveWindowState::default() };
        save_live_window_state(&settings, &id, true, &camera).unwrap();
        assert_eq!(load_live_window_state(&settings, &id, true), camera);
        assert_eq!(settings.get_i32(SettingSection::AppSettings, &format!("OpenGLPreview_{id}/ClientHeight"), 0), 480);

        // Minimized: the sizes stay as they were.
        let minimized = LiveWindowState { window_state: "Minimized".to_string(), width: 1, height: 1, ..flat.clone() };
        save_live_window_state(&settings, &id, false, &minimized).unwrap();
        assert_eq!(settings.get_i32(SettingSection::AppSettings, &format!("Preview_{id}/WindowWidth"), 0), 500);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reset_size_in_each_view() {
        let Ok(renderer) = LiveRenderer::headless(8, 8, 1.0) else { return };
        let renderer = Arc::new(renderer);
        let mut flat = LiveView::new(false, LiveWindowState { zoom_level: 1.5, ..LiveWindowState::default() }, Some((800, 600)), (400.0, 300.0), 350.0, renderer.clone());
        assert_eq!(flat.reset_size(), Some((1200.0, 922.0)));
        flat.reset_zoom();
        assert_eq!(flat.reset_size(), Some((800.0, 622.0)));

        let mut camera = LiveView::new(true, LiveWindowState::default(), Some((800, 600)), (400.0, 300.0), 328.0, renderer.clone());
        assert_eq!(camera.reset_size(), Some((800.0, 600.0)));
        assert!(close(camera.camera.z, focal_depth(600.0)));
        let mut no_background = LiveView::new(true, LiveWindowState::default(), None, (400.0, 300.0), 328.0, renderer);
        assert_eq!(no_background.reset_size(), None);
    }
}
