//! `LiveRenderer`: the thread, the mailbox, and that nothing waits on anything.

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use vixen_render::{rgba, LightGeom, LiveRenderer, Overlay, OverlayLine, RenderError};

const RED: u32 = rgba(255, 0, 0, 255);
const GREEN: u32 = rgba(0, 255, 0, 255);
const RED_PX: [u8; 4] = [255, 0, 0, 255];
const GREEN_PX: [u8; 4] = [0, 255, 0, 255];
const BLUE_PX: [u8; 4] = [0, 0, 255, 255];
const BLACK_PX: [u8; 4] = [0, 0, 0, 255];
const WHITE_PX: [u8; 4] = [255, 255, 255, 255];

fn gpu(width: u32, height: u32) -> Option<LiveRenderer> {
    match LiveRenderer::headless(width, height, 1.0) {
        Ok(live) => Some(live),
        Err(RenderError::NoAdapter) if std::env::var_os("VIXEN_RENDER_REQUIRE_GPU").is_none() => {
            eprintln!("skipping: no GPU adapter on this machine");
            None
        }
        Err(error) => panic!("{error}"),
    }
}

macro_rules! live {
    ($width:expr, $height:expr) => {
        match gpu($width, $height) {
            Some(live) => live,
            None => return,
        }
    };
}

fn at(pixels: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let i = (y * width + x) * 4;
    pixels[i..i + 4].try_into().unwrap()
}

fn light(x: f32, y: f32, size: f32) -> LightGeom {
    LightGeom { x, y, size }
}

/// Waits until the thread has handled everything so far.
fn settle(live: &LiveRenderer) {
    live.read_pixels();
}

#[test]
fn is_shareable_across_threads() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<LiveRenderer>();
}

#[test]
fn draws_a_first_frame_without_being_asked() {
    let live = live!(8, 8);
    let pixels = live.read_pixels();
    assert_eq!(pixels.len(), 8 * 8 * 4);
    assert_eq!(at(&pixels, 8, 4, 4), BLACK_PX);
    assert_eq!(live.stats().frames_rendered, 1);
}

#[test]
fn published_colours_reach_the_screen() {
    let live = live!(32, 32);
    live.set_lights(&[light(10.0, 10.0, 5.0)]);
    live.publish_colors(&[RED]);
    assert_eq!(at(&live.read_pixels(), 32, 10, 10), RED_PX);

    live.publish_colors(&[GREEN]);
    assert_eq!(at(&live.read_pixels(), 32, 10, 10), GREEN_PX);
}

#[test]
fn colours_published_before_their_lights_are_not_lost() {
    let live = live!(32, 32);
    settle(&live);
    live.publish_colors(&[RED, GREEN]);
    settle(&live);
    live.set_lights(&[light(5.0, 5.0, 1.0), light(10.0, 5.0, 1.0)]);
    let pixels = live.read_pixels();
    assert_eq!(at(&pixels, 32, 5, 5), RED_PX);
    assert_eq!(at(&pixels, 32, 10, 5), GREEN_PX);
}

#[test]
fn every_kind_of_change_applies_in_order() {
    let live = live!(32, 32);
    let image: Vec<u8> = BLUE_PX.iter().copied().cycle().take(32 * 32 * 4).collect();
    live.set_background(&image, 32, 32);
    live.set_lights(&[light(5.0, 5.0, 1.0), light(10.0, 5.0, 1.0)]);
    live.publish_colors(&[RED, RED]);
    live.update_light_range(1, &[light(20.0, 5.0, 1.0)]);
    live.set_view(1.0, 2.0, 3.0);
    live.set_overlay(Overlay {
        lines: vec![OverlayLine { a: [0.0, 20.0], b: [20.0, 20.0], color: rgba(255, 255, 255, 255), width: 1.0 }],
        ..Default::default()
    });
    let pixels = live.read_pixels();
    assert_eq!(at(&pixels, 32, 7, 8), RED_PX, "light 0, panned");
    assert_eq!(at(&pixels, 32, 22, 8), RED_PX, "light 1, moved and panned");
    assert_eq!(at(&pixels, 32, 12, 8), BLUE_PX, "light 1 left here");
    assert_eq!(at(&pixels, 32, 10, 23), WHITE_PX, "overlay, panned");

    live.set_background_alpha(0);
    assert_eq!(at(&live.read_pixels(), 32, 1, 1), BLACK_PX);
    live.set_background_alpha(255);
    live.clear_background();
    assert_eq!(at(&live.read_pixels(), 32, 12, 8), BLACK_PX);

    live.resize(16, 16, 1.0);
    assert_eq!(live.read_pixels().len(), 16 * 16 * 4);
}

#[test]
fn an_idle_preview_renders_nothing() {
    let live = live!(32, 32);
    live.set_lights(&[light(10.0, 10.0, 5.0)]);
    live.publish_colors(&[RED]);
    settle(&live);
    let settled = live.stats().frames_rendered;

    thread::sleep(Duration::from_millis(150));
    assert_eq!(live.stats().frames_rendered, settled, "nothing changed, nothing drawn");

    // The same colours again aren't a change either, however often they arrive.
    for _ in 0..100 {
        live.publish_colors(&[RED]);
    }
    settle(&live);
    thread::sleep(Duration::from_millis(50));
    assert_eq!(live.stats().frames_rendered, settled, "identical colours aren't a change");

    live.publish_colors(&[GREEN]);
    settle(&live);
    assert_eq!(live.stats().frames_rendered, settled + 1, "and a real change draws once");

    // Views and overlays that don't change anything don't draw either.
    live.set_view(1.0, 0.0, 0.0);
    settle(&live);
    assert_eq!(live.stats().frames_rendered, settled + 1);
    live.set_view(2.0, 0.0, 0.0);
    settle(&live);
    assert_eq!(live.stats().frames_rendered, settled + 2);

    live.request_redraw();
    settle(&live);
    assert_eq!(live.stats().frames_rendered, settled + 3, "unless asked");
}

/// Far more frames arrive than a heavy scene can render, and the publisher never notices.
#[test]
fn a_flood_of_frames_keeps_only_the_newest_and_never_holds_up_the_publisher() {
    let (width, height) = (1920, 1080);
    let live = live!(width as u32, height as u32);
    let lights: Vec<_> = (0..500_000)
        .map(|i| light((i % width) as f32, (i / width % height) as f32, 7.0))
        .collect();
    live.set_lights(&lights);
    settle(&live);

    let mut colors = vec![RED; lights.len()];
    let mut publishes = 0u64;
    let mut slowest = Duration::ZERO;
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(500) {
        let value = (publishes % 200 + 1) as u8;
        colors.fill(rgba(value, 0, 0, 255));
        let before = Instant::now();
        live.publish_colors(&colors);
        slowest = slowest.max(before.elapsed());
        publishes += 1;
    }

    colors.fill(GREEN);
    live.publish_colors(&colors);
    publishes += 1;
    let pixels = live.read_pixels();

    let stats = live.stats();
    assert_eq!(at(&pixels, width, 100, 50), GREEN_PX, "the very last frame is what's on screen");
    assert!(stats.last_error.is_none(), "{:?}", stats.last_error);
    assert!(stats.frames_dropped > 0, "{publishes} publishes, {stats:?}");
    assert!(
        stats.frames_rendered + stats.frames_dropped >= publishes,
        "every frame is drawn or counted as dropped: {publishes} publishes, {stats:?}"
    );
    // Copying two megabytes. A publisher that waited for the GPU would take whole frames.
    assert!(slowest < Duration::from_millis(50), "slowest publish took {slowest:?}");
    eprintln!("{publishes} publishes, slowest {slowest:?}, {stats:?}");
}

#[test]
fn publishers_and_commands_from_many_threads_at_once() {
    let live = Arc::new(live!(64, 64));
    live.set_lights(&(0..64).map(|i| light(i as f32, 5.0, 1.0)).collect::<Vec<_>>());

    let workers: Vec<_> = (0..4u8)
        .map(|id| {
            let live = live.clone();
            thread::spawn(move || {
                for i in 0..500u32 {
                    live.publish_colors(&[rgba(id * 10 + 1, (i % 250) as u8, 0, 255); 64]);
                    if i % 50 == 0 {
                        live.set_view(1.0 + (i % 3) as f32, 0.0, 0.0);
                        let _ = live.stats();
                    }
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }

    live.set_view(1.0, 0.0, 0.0);
    live.publish_colors(&[RED; 64]);
    let pixels = live.read_pixels();
    assert_eq!(at(&pixels, 64, 30, 5), RED_PX);
    assert!(live.stats().last_error.is_none());
}

#[test]
fn zero_size_waits_for_a_real_size_without_spinning() {
    let live = live!(0, 0);
    live.set_lights(&[light(5.0, 5.0, 1.0)]);
    live.publish_colors(&[RED]);
    assert!(live.read_pixels().is_empty());
    assert_eq!(live.stats().frames_rendered, 0);

    live.resize(16, 16, 1.0);
    assert_eq!(at(&live.read_pixels(), 16, 5, 5), RED_PX);
    assert_eq!(live.stats().frames_rendered, 1);

    live.resize(0, 16, 1.0);
    assert!(live.read_pixels().is_empty());
    live.publish_colors(&[GREEN]);
    assert!(live.read_pixels().is_empty());
    assert_eq!(live.stats().frames_rendered, 1);

    live.resize(16, 16, 1.0);
    assert_eq!(at(&live.read_pixels(), 16, 5, 5), GREEN_PX);
}

/// A read that follows a resize answers at the new size, whatever moment the resize reached the thread.
#[test]
fn reads_always_see_the_resize_before_them() {
    let live = live!(32, 32);
    let lights: Vec<_> = (0..200_000).map(|i| light((i % 64) as f32, 5.0, 3.0)).collect();
    live.set_lights(&lights);
    let mut colors = vec![RED; lights.len()];
    for size in 0..300u32 {
        let (width, height) = (16 + size % 40, 16 + size % 23);
        // Colours keep the thread drawing, so that some resizes land while it holds a target.
        colors[0] = rgba((size % 250) as u8, 1, 2, 255);
        live.publish_colors(&colors);
        live.resize(width, height, 1.0);
        assert_eq!(live.read_pixels().len(), (width * height * 4) as usize, "resize number {size}");
    }
    assert!(live.stats().last_error.is_none());
}

#[test]
fn stats_report_real_cpu_time() {
    let live = live!(256, 256);
    let lights: Vec<_> = (0..100_000).map(|i| light((i % 256) as f32, (i / 256 % 256) as f32, 3.0)).collect();
    live.set_lights(&lights);
    live.publish_colors(&vec![RED; lights.len()]);
    settle(&live);
    let stats = live.stats();
    assert!(stats.frames_rendered >= 1);
    assert!(stats.last_frame_cpu > Duration::ZERO);
    assert!(stats.last_frame_cpu < Duration::from_millis(100), "{stats:?}");
}

#[test]
fn a_bad_call_is_reported_and_rendering_goes_on() {
    let live = live!(16, 16);
    live.set_background(&[0; 3], 8, 8);
    live.set_lights(&[light(5.0, 5.0, 1.0)]);
    live.publish_colors(&[RED]);
    let pixels = live.read_pixels();
    assert_eq!(at(&pixels, 16, 5, 5), RED_PX);
    let error = live.stats().last_error.expect("the bad buffer is reported");
    assert!(error.contains("set_background"), "{error}");
}

#[test]
fn dropping_stops_the_thread_whatever_it_is_doing() {
    for _ in 0..5 {
        let live = live!(64, 64);
        live.set_lights(&(0..10_000).map(|i| light((i % 64) as f32, 5.0, 3.0)).collect::<Vec<_>>());
        live.publish_colors(&vec![RED; 10_000]);
        drop(live);
    }
    let live = live!(8, 8);
    drop(live);
}

#[test]
fn transparent_clears_to_no_alpha_and_a_dim_light_stays_opaque() {
    let Some(live) = gpu(16, 16) else { return };
    let rgba_image: Vec<u8> = [200u8, 200, 200, 255].repeat(16 * 16);
    live.set_background(&rgba_image, 16, 16);
    live.set_lights(&[LightGeom { x: 4.0, y: 4.0, size: 1.0 }]);
    live.publish_colors(&[rgba(255, 0, 0, 128)]);
    let shown = live.read_pixels();
    assert_eq!(shown[3], 255, "opaque without transparency");
    assert_eq!(shown[0], 200);

    live.set_transparent(true);
    let pixels = live.read_pixels();
    assert_eq!(&pixels[..4], &[0, 0, 0, 0], "unlit is clear, with no background");
    let light = &pixels[(4 * 16 + 4) * 4..][..4];
    assert_eq!(light[3], 255, "a dim light is opaque");
    assert!((127..=129).contains(&light[0]), "its colour is over black: {light:?}");

    live.set_transparent(false);
    assert_eq!(live.read_pixels()[3], 255);
}

#[test]
fn light_scale_makes_lights_opengl_points() {
    let Some(live) = gpu(32, 32) else { return };
    live.set_lights(&[LightGeom { x: 16.0, y: 16.0, size: 4.0 }]);
    live.publish_colors(&[rgba(255, 255, 255, 255)]);
    let lit = |pixels: &[u8]| pixels.chunks_exact(4).filter(|p| p[0] > 128).count();
    let flat = lit(&live.read_pixels());
    live.set_light_scale(3.0);
    let big = lit(&live.read_pixels());
    // Twelve across is about 113 pixels. The flat rule's radius of 2 is about 21.
    assert!(flat < 40, "{flat}");
    assert!((90..150).contains(&big), "{big}");
    live.set_light_scale(0.1);
    let small = lit(&live.read_pixels());
    assert!(small < 8, "{small}");
    // A size under 1 stays one pixel however the light scale goes.
    live.set_lights(&[LightGeom { x: 16.0, y: 16.0, size: 0.5 }]);
    live.set_light_scale(10.0);
    assert!(lit(&live.read_pixels()) <= 4);
}

#[test]
fn frame_gaps_are_kept_oldest_first() {
    let Some(live) = gpu(8, 8) else { return };
    live.set_lights(&[LightGeom { x: 1.0, y: 1.0, size: 1.0 }]);
    for i in 0..5u8 {
        live.publish_colors(&[rgba(i, 0, 0, 255)]);
        live.read_pixels();
        thread::sleep(Duration::from_millis(5));
    }
    let gaps = live.recent_frame_gaps();
    assert!(!gaps.is_empty());
    assert_eq!(gaps.len() as u64, live.stats().frames_rendered - 1);
}
