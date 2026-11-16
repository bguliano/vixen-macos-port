//! Real presentation: a `CAMetalLayer` in a window on the display, driven by `LiveRenderer` the way the app
//! will drive it. This is what `cargo test` can't cover, so it reports what it saw and fails loudly.
//!
//!     cargo run --release -p vixen-render --example window [-- --capture shot.png --screen 1 --sublayer]
//!
//! `--screen` picks a display, as listed when it starts. `--sublayer` hosts the layer under a layer-backed view,
//! which gives it a delegate to find, so wgpu checks the window's occlusion on the render thread. Without it the
//! layer is the view's own hosted layer and has no delegate.
//!
//! It opens a small window for about ten seconds, without taking focus, and checks that:
//!
//! - flooding it with frames renders at the display's refresh rate, not faster, which shows vsync pacing works
//! - a steady 40 Hz producer renders 40 frames a second, so nothing renders for its own sake
//! - with no changes it renders nothing and the process uses next to no CPU
//! - a hidden window renders nothing and backs off, and comes back when shown
//! - resizing the window keeps working
//!
//! `--capture` saves the window to a PNG with `screencapture`, which takes Screen Recording permission.

use std::ffi::{c_char, c_void, CStr, CString};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use vixen_render::{rgba, LightGeom, LiveRenderer, Overlay, OverlayLine, OverlaySquare};

type Id = *mut c_void;

#[link(name = "objc")]
extern "C" {
    fn objc_getClass(name: *const c_char) -> Id;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
}

#[link(name = "AppKit", kind = "framework")]
extern "C" {}

#[link(name = "QuartzCore", kind = "framework")]
extern "C" {}

#[link(name = "Foundation", kind = "framework")]
extern "C" {}

#[repr(C)]
#[derive(Copy, Clone)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct Size {
    width: f64,
    height: f64,
}

fn class(name: &CStr) -> Id {
    let class = unsafe { objc_getClass(name.as_ptr()) };
    assert!(!class.is_null(), "no class {name:?}");
    class
}

fn sel(name: &CStr) -> *mut c_void {
    unsafe { sel_registerName(name.as_ptr()) }
}

/// `[object selector:argument]`, with the types spelled out by the caller.
unsafe fn send<A, R>(object: Id, selector: &CStr, argument: A) -> R {
    let f: extern "C" fn(Id, *mut c_void, A) -> R =
        unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
    f(object, sel(selector), argument)
}

unsafe fn get<R>(object: Id, selector: &CStr) -> R {
    let f: extern "C" fn(Id, *mut c_void) -> R =
        unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
    f(object, sel(selector))
}

unsafe fn ns_string(text: &str) -> Id {
    let text = CString::new(text).unwrap();
    unsafe { send(class(c"NSString"), c"stringWithUTF8String:", text.as_ptr()) }
}

/// Handles window events for a while, which is also what lets the window appear and redraw.
unsafe fn pump_events(app: Id, duration: Duration) {
    let f: extern "C" fn(Id, *mut c_void, u64, Id, Id, bool) -> Id =
        unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
    let deadline = Instant::now() + duration;
    let mode = unsafe { ns_string("kCFRunLoopDefaultMode") };
    while Instant::now() < deadline {
        let until: Id = unsafe { send(class(c"NSDate"), c"dateWithTimeIntervalSinceNow:", 0.005f64) };
        let event = f(app, sel(c"nextEventMatchingMask:untilDate:inMode:dequeue:"), u64::MAX, until, mode, true);
        if !event.is_null() {
            unsafe { send::<Id, ()>(app, c"sendEvent:", event) };
        }
    }
}

fn process_cpu_time() -> Duration {
    // SAFETY: an all-zero rusage is valid, and getrusage only writes to it.
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    let time = |t: libc::timeval| Duration::new(t.tv_sec as u64, t.tv_usec as u32 * 1000);
    time(usage.ru_utime) + time(usage.ru_stime)
}

/// Watches the frame counter from another thread and reports how evenly frames came. This is the cadence the
/// display gets, since each frame starts when a drawable comes free.
struct Cadence {
    gaps: Vec<Duration>,
}

impl Cadence {
    fn watch(live: &LiveRenderer, until: Instant) -> Cadence {
        let mut gaps = Vec::new();
        let mut frames = live.stats().frames_rendered;
        let mut last_frame = Instant::now();
        while Instant::now() < until {
            thread::sleep(Duration::from_micros(250));
            let now_frames = live.stats().frames_rendered;
            if now_frames != frames {
                let now = Instant::now();
                gaps.push(now - last_frame);
                last_frame = now;
                frames = now_frames;
            }
        }
        Cadence { gaps }
    }

    fn describe(&mut self) -> String {
        if self.gaps.is_empty() {
            return "no frames".to_string();
        }
        self.gaps.sort_unstable();
        let at = |fraction: f64| self.gaps[((self.gaps.len() - 1) as f64 * fraction) as usize];
        format!(
            "frame gaps: p10 {:.1} ms, median {:.1} ms, p99 {:.1} ms, worst {:.1} ms",
            at(0.1).as_secs_f64() * 1000.0,
            at(0.5).as_secs_f64() * 1000.0,
            at(0.99).as_secs_f64() * 1000.0,
            self.gaps.last().unwrap().as_secs_f64() * 1000.0,
        )
    }
}

const WIDTH: f64 = 720.0;
const HEIGHT: f64 = 480.0;

/// Every size that matters, in strings like the ones a display has.
fn lights() -> Vec<LightGeom> {
    let mut lights = Vec::new();
    for (row, size) in [1.0f32, 2.0, 3.0, 5.0, 9.0, 15.0].into_iter().enumerate() {
        let y = 60.0 + row as f32 * 50.0;
        let spacing = size.max(4.0) * 2.0;
        let mut x = 40.0;
        while x < 680.0 {
            lights.push(LightGeom { x, y, size });
            x += spacing;
        }
    }
    lights
}

fn background() -> (Vec<u8>, u32, u32) {
    let (width, height) = (WIDTH as u32, HEIGHT as u32);
    let mut image = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            image.extend_from_slice(&[(20 + x / 16) as u8, (20 + y / 12) as u8, 70, 255]);
        }
    }
    (image, width, height)
}

fn overlay() -> Overlay {
    let band = |a: [f32; 2], b: [f32; 2]| OverlayLine { a, b, color: rgba(255, 255, 255, 255), width: 1.0 };
    Overlay {
        lines: vec![
            band([20.0, 20.0], [700.0, 20.0]),
            band([700.0, 20.0], [700.0, 400.0]),
            band([700.0, 400.0], [20.0, 400.0]),
            band([20.0, 400.0], [20.0, 20.0]),
            OverlayLine { a: [40.0, 430.0], b: [300.0, 460.0], color: rgba(0, 255, 255, 200), width: 2.0 },
        ],
        squares: [[20.0, 20.0], [700.0, 20.0], [700.0, 400.0], [20.0, 400.0]]
            .map(|center| OverlaySquare { center, size: 6.0, color: rgba(255, 255, 255, 255) })
            .to_vec(),
        circles: vec![([360.0, 10.0], 3.0, rgba(255, 255, 255, 255))],
    }
}

/// A chase with a little of every channel, so each frame differs from the last.
fn fill_colors(colors: &mut [u32], frame: u32) {
    for (i, color) in colors.iter_mut().enumerate() {
        let phase = (i as u32).wrapping_add(frame) % 96;
        *color = match phase / 32 {
            0 => rgba(255, (phase * 8) as u8, 0, 255),
            1 => rgba(0, 255, ((phase - 32) * 8) as u8, 255),
            _ => rgba(((phase - 64) * 8) as u8, 40, 255, 220),
        };
    }
}

fn main() {
    let capture_path = std::env::args().skip_while(|arg| arg != "--capture").nth(1);
    let screen_index: usize =
        std::env::args().skip_while(|arg| arg != "--screen").nth(1).map_or(0, |n| n.parse().expect("--screen N"));
    let sublayer = std::env::args().any(|arg| arg == "--sublayer");

    // SAFETY: these are AppKit's documented selectors, with their documented signatures, on the main thread.
    unsafe {
        let app: Id = get(class(c"NSApplication"), c"sharedApplication");
        // Accessory: no Dock icon, and it doesn't take focus from whatever the user is doing.
        send::<i64, bool>(app, c"setActivationPolicy:", 1);

        let screens: Id = get(class(c"NSScreen"), c"screens");
        let screen_count: usize = get(screens, c"count");
        let mut origin = (200.0, 200.0);
        for index in 0..screen_count {
            let screen: Id = send(screens, c"objectAtIndex:", index);
            let frame: Rect = get(screen, c"frame");
            let scale: f64 = get(screen, c"backingScaleFactor");
            let refresh: i64 = get(screen, c"maximumFramesPerSecond");
            println!(
                "Screen {index}: {} x {} at ({}, {}), scale {scale}, {refresh} Hz",
                frame.width, frame.height, frame.x, frame.y
            );
            if index == screen_index {
                origin = (frame.x + 100.0, frame.y + 100.0);
            }
        }

        let window: Id = get(class(c"NSWindow"), c"alloc");
        let init: extern "C" fn(Id, *mut c_void, Rect, u64, u64, bool) -> Id =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        let content = Rect { x: origin.0, y: origin.1, width: WIDTH, height: HEIGHT };
        let titled_and_closable = 1 | 2;
        let buffered = 2;
        let window = init(
            window,
            sel(c"initWithContentRect:styleMask:backing:defer:"),
            content,
            titled_and_closable,
            buffered,
            false,
        );
        // AppKit keeps a new window on the main screen, whatever rectangle it was given.
        send::<Size, ()>(window, c"setFrameOrigin:", Size { width: origin.0, height: origin.1 });
        send::<bool, ()>(window, c"setReleasedWhenClosed:", false);
        send::<Id, ()>(window, c"setTitle:", ns_string("vixen-render presentation check"));

        let layer: Id = get(class(c"CAMetalLayer"), c"new");
        let view: Id = get(window, c"contentView");
        if sublayer {
            send::<bool, ()>(view, c"setWantsLayer:", true);
            let host: Id = get(view, c"layer");
            send::<Id, ()>(host, c"addSublayer:", layer);
            send::<Rect, ()>(layer, c"setFrame:", Rect { x: 0.0, y: 0.0, width: WIDTH, height: HEIGHT });
        } else {
            send::<Id, ()>(view, c"setLayer:", layer);
            send::<bool, ()>(view, c"setWantsLayer:", true);
        }
        let scale: f64 = get(window, c"backingScaleFactor");
        send::<f64, ()>(layer, c"setContentsScale:", scale);
        get::<()>(window, c"orderFrontRegardless");
        pump_events(app, Duration::from_millis(200));

        let mut host = layer;
        let mut delegate: Id = get(host, c"delegate");
        while delegate.is_null() {
            host = get(host, c"superlayer");
            if host.is_null() {
                break;
            }
            delegate = get(host, c"delegate");
        }
        let screen: Id = get(window, c"screen");
        let refresh: i64 = get(screen, c"maximumFramesPerSecond");
        let number: i64 = get(window, c"windowNumber");
        println!(
            "Window {number} at scale {scale}, on a {refresh} Hz display. wgpu {} check occlusion: the layer {} a delegate",
            if delegate.is_null() { "can't" } else { "will" },
            if delegate.is_null() { "has no route to" } else { "has a route to" },
        );

        let (width_px, height_px) = ((WIDTH * scale) as u32, (HEIGHT * scale) as u32);
        let live = LiveRenderer::for_layer(layer, width_px, height_px, scale as f32).expect("for_layer failed");

        let lights = lights();
        let (image, image_width, image_height) = background();
        live.set_background(&image, image_width, image_height);
        live.set_background_alpha(200);
        live.set_lights(&lights);
        live.set_overlay(overlay());
        let mut colors = vec![0u32; lights.len()];

        // Flood: far more frames than the display can show, published from another thread as playback would.
        let frames_before = live.stats().frames_rendered;
        let flood_seconds = 3.0;
        let start = Instant::now();
        let (publishes, mut cadence) = thread::scope(|scope| {
            let publisher = scope.spawn(|| {
                let mut publishes = 0u32;
                let mut colors = colors.clone();
                while start.elapsed().as_secs_f64() < flood_seconds {
                    fill_colors(&mut colors, publishes);
                    live.publish_colors(&colors);
                    publishes += 1;
                }
                publishes
            });
            let watcher =
                scope.spawn(|| Cadence::watch(&live, start + Duration::from_secs_f64(flood_seconds)));
            pump_events(app, Duration::from_secs_f64(flood_seconds));
            (publisher.join().unwrap(), watcher.join().unwrap())
        });
        let elapsed = start.elapsed().as_secs_f64();
        let flood = live.stats();
        let flood_rate = (flood.frames_rendered - frames_before) as f64 / elapsed;
        println!(
            "Flood: {publishes} frames published in {elapsed:.2}s, {} rendered ({flood_rate:.1} per second), {} dropped, last frame {:?} of CPU, {}",
            flood.frames_rendered - frames_before,
            flood.frames_dropped,
            flood.last_frame_cpu,
            cadence.describe(),
        );
        assert!(flood.last_error.is_none(), "{:?}", flood.last_error);
        // Thousands a second is what this scene manages unpaced, so this only has to show the display holding it
        // near its refresh rate. Runs do go a little over: one reached 82 a second on a 72 Hz display.
        assert!(
            flood_rate < refresh as f64 * 1.5,
            "{flood_rate:.1} frames a second is far faster than the {refresh} Hz display: nothing is pacing it"
        );
        if flood_rate < refresh as f64 * 0.5 {
            println!("WARNING: {flood_rate:.1} frames a second on a {refresh} Hz display");
        }

        // A steady producer: what playback looks like. Every frame shows, nothing extra.
        let frames_before = live.stats().frames_rendered;
        let dropped_before = live.stats().frames_dropped;
        let start = Instant::now();
        let mut produced = 0;
        let mut cadence = thread::scope(|scope| {
            let watcher = scope.spawn(|| Cadence::watch(&live, start + Duration::from_secs(2)));
            while start.elapsed() < Duration::from_secs(2) {
                produced += 1;
                fill_colors(&mut colors, produced);
                live.publish_colors(&colors);
                pump_events(app, Duration::from_millis(25));
            }
            watcher.join().unwrap()
        });
        let steady = live.stats();
        println!(
            "Steady: {produced} frames published at 25 ms intervals, {} rendered, {} dropped, {}",
            steady.frames_rendered - frames_before,
            steady.frames_dropped - dropped_before,
            cadence.describe(),
        );
        assert!(steady.last_error.is_none(), "{:?}", steady.last_error);
        let rendered = (steady.frames_rendered - frames_before) as i64;
        assert!((rendered - produced as i64).abs() <= 2, "{produced} produced, {rendered} rendered");

        // Idle: nothing changes, and nothing happens.
        pump_events(app, Duration::from_millis(200));
        let frames_before = live.stats().frames_rendered;
        let cpu_before = process_cpu_time();
        thread::sleep(Duration::from_secs(2));
        let cpu = process_cpu_time() - cpu_before;
        let idle_frames = live.stats().frames_rendered - frames_before;
        println!("Idle for 2s: {idle_frames} frames rendered, {cpu:?} of process CPU");
        assert_eq!(idle_frames, 0, "an idle preview rendered");

        if let Some(path) = &capture_path {
            pump_events(app, Duration::from_millis(100));
            let status = Command::new("screencapture")
                .args(["-x", "-o", &format!("-l{number}"), path])
                .status()
                .expect("could not run screencapture");
            println!("Captured to {path}: {status}");
        }

        // Hidden: changes wait, without rendering and without spinning.
        send::<Id, ()>(window, c"orderOut:", std::ptr::null_mut());
        pump_events(app, Duration::from_millis(500));
        let frames_before = live.stats().frames_rendered;
        let cpu_before = process_cpu_time();
        for frame in 0..40 {
            fill_colors(&mut colors, 10_000 + frame);
            live.publish_colors(&colors);
            pump_events(app, Duration::from_millis(25));
        }
        let cpu = process_cpu_time() - cpu_before;
        let hidden = live.stats();
        let hidden_frames = hidden.frames_rendered - frames_before;
        println!("Hidden for 1s with changes arriving: {hidden_frames} frames rendered, {cpu:?} of process CPU");
        assert!(hidden.last_error.is_none(), "{:?}", hidden.last_error);

        // Shown again: the changes that waited appear without anything new being published.
        get::<()>(window, c"orderFrontRegardless");
        let frames_before = live.stats().frames_rendered;
        pump_events(app, Duration::from_millis(500));
        let shown_frames = live.stats().frames_rendered - frames_before;
        println!("Shown again: {shown_frames} frames rendered within 500 ms");
        assert!(shown_frames >= 1, "the window came back but nothing was drawn");

        // Resizing, as a user would drag it: a stream of sizes while frames keep coming.
        let frames_before = live.stats().frames_rendered;
        for step in 0..60 {
            let t = step as f64 / 59.0;
            let size = Size { width: WIDTH - 200.0 * t, height: HEIGHT - 120.0 * t };
            send::<Size, ()>(window, c"setContentSize:", size);
            if sublayer {
                send::<Rect, ()>(layer, c"setFrame:", Rect { x: 0.0, y: 0.0, width: size.width, height: size.height });
            }
            live.resize((size.width * scale) as u32, (size.height * scale) as u32, scale as f32);
            fill_colors(&mut colors, 20_000 + step);
            live.publish_colors(&colors);
            pump_events(app, Duration::from_millis(16));
        }
        pump_events(app, Duration::from_millis(100));
        let drawable: Size = get(layer, c"drawableSize");
        let resized = live.stats();
        println!(
            "Resized over 60 steps: {} frames rendered, drawable now {} x {}",
            resized.frames_rendered - frames_before,
            drawable.width,
            drawable.height,
        );
        assert!(resized.last_error.is_none(), "{:?}", resized.last_error);
        assert_eq!(drawable.width as u32, ((WIDTH - 200.0) * scale) as u32);

        let start = Instant::now();
        drop(live);
        println!("Stopped in {:?}", start.elapsed());
        get::<()>(window, c"close");
        println!("PASS");
    }
}
