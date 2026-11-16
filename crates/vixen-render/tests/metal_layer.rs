//! The real `CAMetalLayer` path: acquiring drawables, drawing into them and presenting them.
//!
//! These layers aren't in a window, so nothing here shows that a frame reached the display or that vsync paces
//! it: `examples/window.rs` is the check for those. What this does cover is everything between `for_layer` and
//! `presentDrawable`, including what ends up in the drawable, which the offscreen tests can't speak to.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_void, CStr};
use std::thread;
use std::time::{Duration, Instant};

use vixen_render::{rgba, FrameSkip, LightGeom, LiveRenderer, Overlay, OverlayLine, RenderError, Renderer};

#[link(name = "objc")]
extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
}

#[link(name = "QuartzCore", kind = "framework")]
extern "C" {}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct CGSize {
    width: f64,
    height: f64,
}

/// Sends a message that takes no arguments.
unsafe fn send<R>(object: *mut c_void, selector: &CStr) -> R {
    let f: extern "C" fn(*mut c_void, *mut c_void) -> R =
        unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
    f(object, unsafe { sel_registerName(selector.as_ptr()) })
}

struct Layer(*mut c_void);

// SAFETY: a CALayer's retain count and properties may be used from any thread.
unsafe impl Send for Layer {}

impl Layer {
    fn new(class: &CStr) -> Layer {
        unsafe {
            let class = objc_getClass(class.as_ptr());
            assert!(!class.is_null());
            let allocated: *mut c_void = send(class, c"alloc");
            Layer(send(allocated, c"init"))
        }
    }

    fn metal() -> Layer {
        Layer::new(c"CAMetalLayer")
    }

    fn drawable_size(&self) -> (f64, f64) {
        let size: CGSize = unsafe { send(self.0, c"drawableSize") };
        (size.width, size.height)
    }

    fn pixel_format(&self) -> usize {
        unsafe { send(self.0, c"pixelFormat") }
    }

    fn colorspace(&self) -> *mut c_void {
        unsafe { send(self.0, c"colorspace") }
    }

    fn allows_timeout(&self) -> bool {
        unsafe { send(self.0, c"allowsNextDrawableTimeout") }
    }

    fn max_drawables(&self) -> usize {
        unsafe { send(self.0, c"maximumDrawableCount") }
    }

    fn framebuffer_only(&self) -> bool {
        unsafe { send(self.0, c"framebufferOnly") }
    }

    fn display_sync(&self) -> bool {
        unsafe { send(self.0, c"displaySyncEnabled") }
    }

    fn is_opaque(&self) -> bool {
        unsafe { send(self.0, c"isOpaque") }
    }

    fn device(&self) -> *mut c_void {
        unsafe { send(self.0, c"device") }
    }
}

impl Drop for Layer {
    fn drop(&mut self) {
        unsafe { send::<()>(self.0, c"release") };
    }
}

const MTL_PIXEL_FORMAT_BGRA8_UNORM: usize = 80;

const RED: u32 = rgba(255, 0, 0, 255);
const RED_PX: [u8; 4] = [255, 0, 0, 255];
const BLACK_PX: [u8; 4] = [0, 0, 0, 255];

fn or_skip<T>(result: Result<T, RenderError>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(RenderError::NoAdapter) if std::env::var_os("VIXEN_RENDER_REQUIRE_GPU").is_none() => {
            eprintln!("skipping: no GPU adapter on this machine");
            None
        }
        Err(error) => panic!("{error}"),
    }
}

macro_rules! gpu {
    ($result:expr) => {
        match or_skip($result) {
            Some(value) => value,
            None => return,
        }
    };
}

/// A layer can be out of drawables for a moment, and a skipped frame stays pending.
fn render_until_drawn(renderer: &mut Renderer) -> vixen_render::FrameStats {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let stats = renderer.render().unwrap();
        if stats.skipped.is_none() {
            return stats;
        }
        assert!(Instant::now() < deadline, "the layer never gave up a drawable: {stats:?}");
    }
}

fn scene(renderer: &mut Renderer) {
    let image: Vec<u8> = [30, 60, 90, 255].iter().copied().cycle().take(64 * 48 * 4).collect();
    renderer.set_background(&image, 64, 48);
    renderer.set_background_alpha(200);
    renderer.set_view(1.25, 3.0, 2.0);
    renderer.set_lights(&[
        LightGeom { x: 10.0, y: 12.0, size: 1.0 },
        LightGeom { x: 20.0, y: 12.0, size: 3.0 },
        LightGeom { x: 32.0, y: 20.0, size: 9.0 },
        LightGeom { x: 40.0, y: 30.0, size: 13.0 },
    ]);
    renderer.set_light_colors(&[
        RED,
        rgba(0, 255, 0, 255),
        rgba(255, 255, 0, 128),
        rgba(255, 0, 255, 77),
    ]);
    renderer.set_overlay(Overlay {
        lines: vec![
            OverlayLine { a: [2.0, 2.0], b: [50.0, 2.0], color: rgba(255, 255, 255, 255), width: 1.0 },
            OverlayLine { a: [2.0, 40.0], b: [50.0, 30.0], color: rgba(0, 255, 255, 200), width: 2.0 },
        ],
        squares: vec![vixen_render::OverlaySquare { center: [8.0, 34.0], size: 6.0, color: rgba(255, 255, 255, 255) }],
        circles: vec![([50.0, 8.0], 3.0, rgba(255, 128, 0, 255))],
    });
}

/// Same scene, same pipelines: what reaches a real drawable has to be what the offscreen tests verify.
#[test]
fn a_real_drawable_gets_exactly_the_offscreen_pixels() {
    for scale in [1.0, 2.0] {
        let (width, height) = ((96.0 * scale) as u32, (72.0 * scale) as u32);

        let mut offscreen = gpu!(Renderer::headless(width, height, scale));
        scene(&mut offscreen);
        offscreen.render().unwrap();
        let expected = offscreen.read_pixels();

        let layer = Layer::metal();
        let mut on_layer = gpu!(unsafe { Renderer::for_layer_readable(layer.0, width, height, scale) });
        assert!(on_layer.read_pixels().is_empty(), "nothing has been drawn yet");
        scene(&mut on_layer);
        render_until_drawn(&mut on_layer);
        let actual = on_layer.read_pixels();

        assert_eq!(actual.len(), expected.len());
        let differing = actual.chunks(4).zip(expected.chunks(4)).filter(|(a, b)| a != b).count();
        assert_eq!(differing, 0, "scale {scale}: {differing} pixels differ between the drawable and offscreen");
        assert!(actual.chunks(4).any(|pixel| pixel == RED_PX), "the scene really drew");
    }
}

#[test]
fn the_layer_is_set_up_for_display() {
    let layer = Layer::metal();
    let mut renderer = gpu!(unsafe { Renderer::for_layer(layer.0, 320, 200, 2.0) });
    render_until_drawn(&mut renderer);

    assert!(!layer.device().is_null());
    assert_eq!(layer.pixel_format(), MTL_PIXEL_FORMAT_BGRA8_UNORM, "not sRGB, so blending matches the original");
    assert_eq!(layer.drawable_size(), (320.0, 200.0));
    assert!(layer.framebuffer_only(), "the app's layer pays nothing for readback");
    assert!(layer.display_sync(), "vsync on");
    assert!(layer.is_opaque());
    assert_eq!(layer.max_drawables(), 3);
    assert!(layer.allows_timeout(), "wgpu turns this off, and nextDrawable could then block forever");
    assert!(!layer.colorspace().is_null(), "wgpu clears this, and nothing would be colour matched");
}

#[test]
fn resizing_reconfigures_the_layer_and_keeps_our_settings() {
    let layer = Layer::metal();
    let mut renderer = gpu!(unsafe { Renderer::for_layer_readable(layer.0, 64, 64, 1.0) });
    renderer.set_lights(&[LightGeom { x: 10.0, y: 10.0, size: 1.0 }]);
    renderer.set_light_colors(&[RED]);
    render_until_drawn(&mut renderer);
    assert_eq!(layer.drawable_size(), (64.0, 64.0));

    for (width, height, scale) in [(128, 96, 2.0), (33, 77, 1.0), (640, 480, 2.0), (640, 480, 1.0)] {
        renderer.resize(width, height, scale);
        render_until_drawn(&mut renderer);
        assert_eq!(layer.drawable_size(), (width as f64, height as f64));
        assert!(layer.allows_timeout());
        assert!(!layer.colorspace().is_null());

        let pixels = renderer.read_pixels();
        assert_eq!(pixels.len(), (width * height * 4) as usize);
        let at = |x: u32, y: u32| -> [u8; 4] {
            let i = ((y * width + x) * 4) as usize;
            pixels[i..i + 4].try_into().unwrap()
        };
        let x = (10.5 * scale) as u32;
        let y = (10.5 * scale) as u32;
        assert_eq!(at(x, y), RED_PX, "{width} x {height} at scale {scale}");
        assert_eq!(at(x + 8, y), BLACK_PX);
    }
}

#[test]
fn a_zero_size_layer_waits_and_recovers() {
    let layer = Layer::metal();
    let mut renderer = gpu!(unsafe { Renderer::for_layer(layer.0, 0, 0, 2.0) });
    assert_eq!(renderer.render().unwrap().skipped, Some(FrameSkip::ZeroSize));

    renderer.resize(100, 100, 2.0);
    render_until_drawn(&mut renderer);
    assert_eq!(layer.drawable_size(), (100.0, 100.0));

    renderer.resize(100, 0, 2.0);
    assert_eq!(renderer.render().unwrap().skipped, Some(FrameSkip::ZeroSize));
    renderer.resize(50, 50, 2.0);
    render_until_drawn(&mut renderer);
    assert_eq!(layer.drawable_size(), (50.0, 50.0));
}

#[test]
fn bad_layer_pointers_are_errors() {
    let result = unsafe { Renderer::for_layer(std::ptr::null_mut(), 64, 64, 1.0) };
    assert_eq!(result.err(), Some(RenderError::NullLayer));

    // The easy mistake: a view's default layer.
    let plain = Layer::new(c"CALayer");
    let result = unsafe { Renderer::for_layer(plain.0, 64, 64, 1.0) };
    assert_eq!(result.err(), Some(RenderError::NotAMetalLayer));
}

/// Metal autoreleases drawables, and a thread started from Rust has no pool. If they leaked into a pool that
/// never drains, a layer would soon run out of its three drawables, and every frame after that would wait out
/// the one second timeout.
#[test]
fn frames_on_a_plain_thread_do_not_run_out_of_drawables() {
    let layer = Layer::metal();
    let worker = thread::spawn(move || {
        let layer = layer; // The whole Layer, not just its pointer field.
        let Some(mut renderer) = or_skip(unsafe { Renderer::for_layer(layer.0, 256, 256, 1.0) }) else {
            return;
        };
        renderer.set_lights(&[LightGeom { x: 10.0, y: 10.0, size: 3.0 }]);
        let start = Instant::now();
        let mut drawn = 0;
        while start.elapsed() < Duration::from_secs(2) && drawn < 120 {
            renderer.set_light_colors(&[rgba(drawn as u8, 0, 0, 255)]);
            let stats = renderer.render().unwrap();
            if stats.skipped.is_none() {
                drawn += 1;
            }
        }
        assert_eq!(drawn, 120, "only {drawn} frames in {:?}", start.elapsed());
        eprintln!("120 frames on a plain thread in {:?}", start.elapsed());
    });
    worker.join().unwrap();
}

#[test]
fn live_renderer_on_a_real_layer_renders_changes_and_then_stops() {
    let layer = Layer::metal();
    let live = gpu!(unsafe { LiveRenderer::for_layer(layer.0, 256, 256, 1.0) });
    live.set_lights(&[LightGeom { x: 10.0, y: 10.0, size: 3.0 }]);

    let wait_for = |target: u64| {
        let deadline = Instant::now() + Duration::from_secs(5);
        while live.stats().frames_rendered < target {
            assert!(Instant::now() < deadline, "stuck at {:?}, wanted {target}", live.stats());
            thread::sleep(Duration::from_millis(1));
        }
    };

    live.publish_colors(&[RED]);
    wait_for(1);
    for i in 1..30u8 {
        live.publish_colors(&[rgba(i, 0, 0, 255)]);
        thread::sleep(Duration::from_millis(5));
    }
    wait_for(2);

    // Give it a moment to finish, then nothing further should happen.
    thread::sleep(Duration::from_millis(100));
    let settled = live.stats();
    assert!(settled.last_error.is_none(), "{settled:?}");
    assert!(settled.last_frame_cpu > Duration::ZERO);
    thread::sleep(Duration::from_millis(200));
    assert_eq!(live.stats().frames_rendered, settled.frames_rendered, "idle");

    live.resize(128, 128, 2.0);
    wait_for(settled.frames_rendered + 1);
    assert_eq!(layer.drawable_size(), (128.0, 128.0));
    drop(live);
    assert_eq!(layer.drawable_size(), (128.0, 128.0), "the layer outlives the renderer");
}

/// A resize can arrive while the thread holds a drawable of the old size, and `Surface::configure` panics if one
/// is alive. Sizes keep changing here while frames flow, and it must end up at the last one.
#[test]
fn resizing_while_frames_are_being_drawn_settles_on_the_last_size() {
    let layer = Layer::metal();
    let live = gpu!(unsafe { LiveRenderer::for_layer(layer.0, 64, 64, 1.0) });
    live.set_lights(&[LightGeom { x: 10.0, y: 10.0, size: 3.0 }]);

    let start = Instant::now();
    let mut step = 0u32;
    while start.elapsed() < Duration::from_millis(500) {
        step += 1;
        live.resize(64 + step % 200, 64 + step % 100, 1.0);
        live.publish_colors(&[rgba(step as u8, 0, 0, 255)]);
        thread::sleep(Duration::from_micros(200));
    }
    live.resize(300, 200, 2.0);
    live.publish_colors(&[RED]);

    let deadline = Instant::now() + Duration::from_secs(5);
    while layer.drawable_size() != (300.0, 200.0) {
        assert!(Instant::now() < deadline, "stuck at {:?}", layer.drawable_size());
        thread::sleep(Duration::from_millis(1));
    }
    let stats = live.stats();
    assert!(stats.last_error.is_none(), "{stats:?}");
    assert!(stats.frames_rendered > 0, "{stats:?}");
}
