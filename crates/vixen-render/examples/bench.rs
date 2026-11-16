//! Measures the renderer against the original's way of drawing a frame.
//!
//!     cargo run --release -p vixen-render --example bench [-- full partial live baseline paced edit hidpi]
//!
//! Every number printed is measured here, on this machine, by this run. Name sections to run only those.
//! `baseline` compares against whatever `full` measured in the same run, and runs `full` if it needs to.
//!
//! All threads run at `QOS_CLASS_USER_INTERACTIVE`, the class the live render thread uses, so that neither side
//! gets parked on an efficiency core.
//!
//! The renderer's frames are offscreen, so nothing paces them. "CPU" is the calling thread's CPU clock, across
//! `set_light_colors` and `render`. Process CPU adds every other thread, which is Metal's and wgpu's work
//! happening off to the side.
//!
//! Flat out, everything stays in cache and at full clock. Real frames arrive with gaps between them, and start
//! cold, so the `paced` section measures both sides again at 60 frames a second.
//!
//! The baseline is `FastPixel.cs` transcribed to Rust: copy the background (`CloneToBuffer`), run the per-pixel
//! circle loop for each light (`DrawCircle`, `SetPixelAlpha`), copy to the output (`UnlockFromBuffer`). That
//! flatters the original: it has no .NET or GDI+ in it, and it stops before `OnPaint` and the double buffer
//! copy the frame twice more.

use std::hint::black_box;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Barrier;
use std::thread;
use std::time::{Duration, Instant};

use vixen_render::{rgba, LightGeom, LiveRenderer, Overlay, OverlayLine, OverlaySquare, Renderer};

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
/// `PreviewPixel`'s default.
const LIGHT_SIZE: i32 = 3;
const COUNTS: [usize; 4] = [1_000, 10_000, 100_000, 500_000];
const MEASURE_FOR: Duration = Duration::from_secs(3);
const WARMUP_FRAMES: usize = 30;

fn run_at_user_interactive() {
    // SAFETY: only affects the calling thread.
    unsafe { libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE, 0) };
}

fn thread_cpu_time() -> Duration {
    let mut time = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // SAFETY: `time` is a valid timespec.
    unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut time) };
    Duration::new(time.tv_sec as u64, time.tv_nsec as u32)
}

fn process_cpu_time() -> Duration {
    // SAFETY: an all-zero rusage is valid, and getrusage only writes to it.
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    let time = |t: libc::timeval| Duration::new(t.tv_sec as u64, t.tv_usec as u32 * 1000);
    time(usage.ru_utime) + time(usage.ru_stime)
}

fn percentile(sorted: &[Duration], fraction: f64) -> Duration {
    sorted[((sorted.len() - 1) as f64 * fraction).round() as usize]
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

/// Evenly over the frame, so lights overlap only as much as the count forces them to.
fn grid_lights(count: usize) -> Vec<LightGeom> {
    let columns = ((count as f64 * WIDTH as f64 / HEIGHT as f64).sqrt().ceil() as usize).max(1);
    let rows = count.div_ceil(columns);
    (0..count)
        .map(|i| LightGeom {
            x: (i % columns) as f32 * WIDTH as f32 / columns as f32,
            y: (i / columns) as f32 * HEIGHT as f32 / rows as f32,
            size: LIGHT_SIZE as f32,
        })
        .collect()
}

fn scramble(mut value: u32) -> u32 {
    value ^= value << 13;
    value ^= value >> 17;
    value ^= value << 5;
    value
}

/// Colours that differ from every other set in every entry: each set has its own alpha.
fn color_sets(count: usize, sets: u8) -> Vec<Vec<u32>> {
    (0..sets)
        .map(|set| {
            (0..count)
                .map(|i| {
                    let [r, g, b, _] = scramble(i as u32 + 1 + set as u32 * 7919).to_le_bytes();
                    rgba(r, g, b, 200 + set)
                })
                .collect()
        })
        .collect()
}

fn background() -> Vec<u8> {
    let mut image = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            image.extend_from_slice(&[(x / 8) as u8, (y / 5) as u8, ((x + y) / 12) as u8, 255]);
        }
    }
    image
}

struct Measured {
    frames: usize,
    cpu_median: Duration,
    cpu_p99: Duration,
    colors_median: Duration,
    render_median: Duration,
    process_cpu_per_frame: Duration,
    frames_per_second: f64,
    uploaded_per_frame: u64,
    draw_calls: u32,
}

/// Renders as fast as the GPU lets it. `frames[i]` is the colour array of frame `i`, cycled.
fn measure(renderer: &mut Renderer, frames: &[Vec<u32>]) -> Measured {
    for frame in 0..WARMUP_FRAMES {
        renderer.set_light_colors(&frames[frame % frames.len()]);
        renderer.render().unwrap();
    }
    renderer.wait_for_gpu();

    let mut cpu = Vec::new();
    let mut colors = Vec::new();
    let mut render = Vec::new();
    let mut uploaded = 0u64;
    let mut draw_calls = 0;
    let process_start = process_cpu_time();
    let start = Instant::now();
    while start.elapsed() < MEASURE_FOR {
        let frame = &frames[(WARMUP_FRAMES + cpu.len()) % frames.len()];
        let cpu_start = thread_cpu_time();
        renderer.set_light_colors(frame);
        let cpu_colors = thread_cpu_time();
        let stats = renderer.render().unwrap();
        let cpu_end = thread_cpu_time();

        cpu.push(cpu_end - cpu_start);
        colors.push(cpu_colors - cpu_start);
        render.push(cpu_end - cpu_colors);
        uploaded += stats.lights_uploaded as u64;
        draw_calls = stats.draw_calls;
    }
    renderer.wait_for_gpu();
    let elapsed = start.elapsed();
    let process = process_cpu_time() - process_start;

    let frame_count = cpu.len();
    cpu.sort_unstable();
    colors.sort_unstable();
    render.sort_unstable();
    Measured {
        frames: frame_count,
        cpu_median: percentile(&cpu, 0.5),
        cpu_p99: percentile(&cpu, 0.99),
        colors_median: percentile(&colors, 0.5),
        render_median: percentile(&render, 0.5),
        process_cpu_per_frame: process / frame_count as u32,
        frames_per_second: frame_count as f64 / elapsed.as_secs_f64(),
        uploaded_per_frame: uploaded / frame_count as u64,
        draw_calls,
    }
}

/// From a finished submission to the GPU finishing the next frame, with nothing else queued.
fn gpu_frame_time(renderer: &mut Renderer, frames: &[Vec<u32>]) -> Duration {
    let mut times = Vec::new();
    for i in 0..60 {
        renderer.set_light_colors(&frames[i % frames.len()]);
        renderer.render().unwrap();
        let start = Instant::now();
        renderer.wait_for_gpu();
        times.push(start.elapsed());
    }
    times.sort_unstable();
    percentile(&times, 0.5)
}

/// `FastPixel`, transcribed: BGRA, bounds-checked per pixel, float blending truncated to a byte.
struct FastPixel {
    rgb_values: Vec<u8>,
    bitmap: Vec<u8>,
}

#[derive(Copy, Clone)]
struct Color {
    a: u8,
    r: u8,
    g: u8,
    b: u8,
}

impl FastPixel {
    fn new() -> FastPixel {
        let bytes = (WIDTH * HEIGHT * 4) as usize;
        FastPixel { rgb_values: vec![0; bytes], bitmap: vec![0; bytes] }
    }

    /// `SetPixelAlpha`, for 32 bit bitmaps.
    #[inline(always)]
    fn set_pixel(bytes: &mut [u8], x: i32, y: i32, color: Color) {
        if x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 {
            let index = ((y * WIDTH as i32 + x) * 4) as usize;
            let b = bytes[index] as f32;
            let g = bytes[index + 1] as f32;
            let r = bytes[index + 2] as f32;
            let alpha_offset = 255.0 - color.a as f32;
            let alpha_percent = color.a as f32 / 255.0;
            bytes[index + 3] = 255;
            bytes[index + 2] = (color.r as f32 * alpha_percent + r * alpha_offset / 255.0) as u8;
            bytes[index + 1] = (color.g as f32 * alpha_percent + g * alpha_offset / 255.0) as u8;
            bytes[index] = (color.b as f32 * alpha_percent + b * alpha_offset / 255.0) as u8;
        }
    }

    /// `DrawCircle(Rectangle, Color)`.
    #[inline(always)]
    fn draw_circle(bytes: &mut [u8], left: i32, top: i32, width: i32, color: Color) {
        if width > 0 {
            if width == 1 {
                Self::set_pixel(bytes, left, top, color);
            } else {
                let radius = width / 2;
                for y in -radius..=radius {
                    for x in -radius..=radius {
                        if x * x + y * y <= radius * radius {
                            Self::set_pixel(bytes, left + x, top + y, color);
                        }
                    }
                }
            }
        }
    }
}

/// Keeps the optimiser from removing or merging a copy. In the original, each one goes through `Marshal.Copy` or
/// GDI+, where it can't be seen through.
fn opaque(bytes: &mut [u8]) {
    black_box(bytes);
}

/// One frame the way `GDIPreviewForm.UpdatePreview` draws it, on one thread.
fn baseline_frame(fp: &mut FastPixel, background: &[u8], lights: &[(i32, i32)], colors: &[Color]) {
    fp.rgb_values.copy_from_slice(background); // CloneToBuffer
    opaque(&mut fp.rgb_values);
    for (&(x, y), &color) in lights.iter().zip(colors) {
        FastPixel::draw_circle(&mut fp.rgb_values, x, y, LIGHT_SIZE, color);
    }
    fp.bitmap.copy_from_slice(&fp.rgb_values); // UnlockFromBuffer
    opaque(&mut fp.bitmap);
}

/// `VixenPreviewControl.RenderInForeground`, which runs on every mouse move while editing. Two new full-size
/// buffers and five full-frame copies, around the same circle loop.
fn baseline_edit_frame(background: &[u8], lights: &[(i32, i32)], colors: &[Color], back: &mut [u8], window: &mut [u8]) {
    let mut bitmap = background.to_vec(); // new Bitmap(_alphaBackground)
    opaque(&mut bitmap);
    let mut rgb_values = bitmap.clone(); // new FastPixel(...), then fp.Lock()
    opaque(&mut rgb_values);
    for (&(x, y), &color) in lights.iter().zip(colors) {
        FastPixel::draw_circle(&mut rgb_values, x, y, LIGHT_SIZE, color);
    }
    bitmap.copy_from_slice(&rgb_values); // fp.Unlock(true)
    opaque(&mut bitmap);
    back.copy_from_slice(&bitmap); // bufferedGraphics.Graphics.DrawImage
    opaque(back);
    window.copy_from_slice(back); // bufferedGraphics.Render
    opaque(window);
}

/// `SetPixelAlpha` again, on bytes other threads write to at the same time. `Parallel.ForEach` in
/// `GDIPreviewForm.UpdatePreview` has the same race, with no lock inside it: overlapping lights lose updates here
/// as they do there. Relaxed atomics are plain loads and stores on this hardware, so this costs what it would.
#[inline(always)]
fn set_pixel_shared(bytes: &[AtomicU8], x: i32, y: i32, color: Color) {
    if x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 {
        let index = ((y * WIDTH as i32 + x) * 4) as usize;
        let b = bytes[index].load(Ordering::Relaxed) as f32;
        let g = bytes[index + 1].load(Ordering::Relaxed) as f32;
        let r = bytes[index + 2].load(Ordering::Relaxed) as f32;
        let alpha_offset = 255.0 - color.a as f32;
        let alpha_percent = color.a as f32 / 255.0;
        bytes[index + 3].store(255, Ordering::Relaxed);
        bytes[index + 2].store((color.r as f32 * alpha_percent + r * alpha_offset / 255.0) as u8, Ordering::Relaxed);
        bytes[index + 1].store((color.g as f32 * alpha_percent + g * alpha_offset / 255.0) as u8, Ordering::Relaxed);
        bytes[index].store((color.b as f32 * alpha_percent + b * alpha_offset / 255.0) as u8, Ordering::Relaxed);
    }
}

struct ParallelResult {
    wall: Duration,
    /// Every thread's CPU, per frame.
    cpu: Duration,
}

/// The playback path, `GDIPreviewForm.UpdatePreview`: copies on one thread, circle loops on all of them. The
/// workers wait at a barrier between frames, like a pool's threads, so spawning them isn't what's measured.
fn baseline_parallel(lights: &[(i32, i32)], colors: &[Color], background: &[u8], frames: usize) -> ParallelResult {
    let workers = thread::available_parallelism().map_or(4, |n| n.get());
    let mut rgb_values = vec![0u8; background.len()];
    let mut bitmap = vec![0u8; background.len()];
    let start_frame = Barrier::new(workers + 1);
    let end_frame = Barrier::new(workers + 1);
    let chunk = lights.len().div_ceil(workers);

    let buffer = rgb_values.as_mut_ptr() as usize;
    let len = rgb_values.len();
    // SAFETY: AtomicU8 has the layout of u8, and the buffer outlives the scope below. It is only touched as
    // atomics by the workers between the barriers, and only as plain bytes by this thread outside them.
    let shared: &[AtomicU8] = unsafe { std::slice::from_raw_parts(buffer as *const AtomicU8, len) };

    let cpu_start = process_cpu_time();
    let mut times = thread::scope(|scope| {
        for worker in 0..workers {
            let (start_frame, end_frame) = (&start_frame, &end_frame);
            let lights = lights.chunks(chunk).nth(worker).unwrap_or(&[]);
            let colors = colors.chunks(chunk).nth(worker).unwrap_or(&[]);
            scope.spawn(move || {
                run_at_user_interactive();
                for _ in 0..frames {
                    start_frame.wait();
                    let radius = LIGHT_SIZE / 2;
                    for (&(left, top), &color) in lights.iter().zip(colors) {
                        for y in -radius..=radius {
                            for x in -radius..=radius {
                                if x * x + y * y <= radius * radius {
                                    set_pixel_shared(shared, left + x, top + y, color);
                                }
                            }
                        }
                    }
                    end_frame.wait();
                }
            });
        }

        let mut times = Vec::with_capacity(frames);
        for _ in 0..frames {
            let start = Instant::now();
            // SAFETY: the workers are all waiting at a barrier, so nothing else is using either buffer.
            unsafe { std::ptr::copy_nonoverlapping(background.as_ptr(), buffer as *mut u8, len) };
            start_frame.wait();
            end_frame.wait();
            // SAFETY: as above, and `bitmap` is this thread's alone.
            unsafe { std::ptr::copy_nonoverlapping(buffer as *const u8, bitmap.as_mut_ptr(), len) };
            opaque(&mut bitmap);
            times.push(start.elapsed());
        }
        times
    });
    let cpu = (process_cpu_time() - cpu_start) / frames as u32;

    times.sort_unstable();
    ParallelResult { wall: percentile(&times, 0.5), cpu }
}

fn section_full(image: &[u8]) -> Vec<Duration> {
    println!("\n## Renderer: every light changes every frame");
    println!("| Lights | CPU median | CPU p99 | Diff + copy | Render | Process CPU | Offscreen FPS | GPU frame | Draw calls | Frames |");
    println!("|---|---|---|---|---|---|---|---|---|---|");
    let mut cpu_medians = Vec::new();
    for count in COUNTS {
        let mut renderer = Renderer::headless(WIDTH, HEIGHT, 1.0).unwrap();
        renderer.set_background(image, WIDTH, HEIGHT);
        renderer.set_lights(&grid_lights(count));
        let colors = color_sets(count, 8);
        let result = measure(&mut renderer, &colors);
        let gpu = gpu_frame_time(&mut renderer, &colors);
        assert_eq!(result.uploaded_per_frame, count as u64, "every light should have been uploaded");
        println!(
            "| {count} | {:.3} ms | {:.3} ms | {:.3} ms | {:.3} ms | {:.3} ms | {:.0} | {:.3} ms | {} | {} |",
            ms(result.cpu_median),
            ms(result.cpu_p99),
            ms(result.colors_median),
            ms(result.render_median),
            ms(result.process_cpu_per_frame),
            result.frames_per_second,
            ms(gpu),
            result.draw_calls,
            result.frames,
        );
        cpu_medians.push(result.cpu_median);
    }
    cpu_medians
}

fn section_partial(image: &[u8]) {
    println!("\n## Renderer: only some lights change");
    println!("Changed lights sit in one run for \"contiguous\", or spread evenly for \"scattered\". CPU includes comparing the array.");
    println!("| Lights | Changed | Pattern | CPU median | CPU p99 | Diff + copy | Render | Lights uploaded per frame | Offscreen FPS |");
    println!("|---|---|---|---|---|---|---|---|---|");
    for count in [100_000usize, 500_000] {
        let mut renderer = Renderer::headless(WIDTH, HEIGHT, 1.0).unwrap();
        renderer.set_background(image, WIDTH, HEIGHT);
        renderer.set_lights(&grid_lights(count));
        let base = color_sets(count, 1).remove(0);

        for (changed, pattern) in [
            (count / 10, "contiguous"),
            (count / 100, "contiguous"),
            (count / 1000, "contiguous"),
            (count / 10, "scattered"),
            (count / 100, "scattered"),
            (count / 1000, "scattered"),
            (0, "-"),
        ] {
            let frames: Vec<Vec<u32>> = (0..8u8)
                .map(|set| {
                    let mut colors = base.clone();
                    if let Some(spacing) = count.checked_div(changed) {
                        for i in 0..changed {
                            let index = if pattern == "contiguous" { i } else { i * spacing };
                            colors[index] = rgba(set * 30, 255 - set * 30, 128, 255);
                        }
                    }
                    colors
                })
                .collect();
            let result = measure(&mut renderer, &frames);
            println!(
                "| {count} | {changed} | {pattern} | {:.3} ms | {:.3} ms | {:.3} ms | {:.3} ms | {} | {:.0} |",
                ms(result.cpu_median),
                ms(result.cpu_p99),
                ms(result.colors_median),
                ms(result.render_median),
                result.uploaded_per_frame,
                result.frames_per_second,
            );
        }
    }
}

/// What playback and the UI pay, with `LiveRenderer` on its own thread: at most one call per frame.
fn section_live(image: &[u8]) {
    println!("\n## LiveRenderer: what the caller pays, at 60 frames a second with every light changing");
    println!("| Lights | publish_colors median | publish_colors worst | Render thread CPU per frame | Frames rendered | Dropped |");
    println!("|---|---|---|---|---|---|");
    for count in COUNTS {
        let live = LiveRenderer::headless(WIDTH, HEIGHT, 1.0).unwrap();
        live.set_background(image, WIDTH, HEIGHT);
        live.set_lights(&grid_lights(count));
        let colors = color_sets(count, 8);
        live.read_pixels();

        let start_frames = live.stats().frames_rendered;
        let mut publish = Vec::new();
        let mut render_cpu = Vec::new();
        for frame in 0..180 {
            let before = Instant::now();
            live.publish_colors(&colors[frame % colors.len()]);
            publish.push(before.elapsed());
            thread::sleep(Duration::from_micros(16_666));
            render_cpu.push(live.stats().last_frame_cpu);
        }
        let stats = live.stats();
        assert!(stats.last_error.is_none(), "{:?}", stats.last_error);
        publish.sort_unstable();
        render_cpu.sort_unstable();
        println!(
            "| {count} | {:.3} ms | {:.3} ms | {:.3} ms | {} | {} |",
            ms(percentile(&publish, 0.5)),
            ms(*publish.last().unwrap()),
            ms(percentile(&render_cpu, 0.5)),
            stats.frames_rendered - start_frames,
            stats.frames_dropped,
        );
    }
}

/// Both sides again, with a frame's gap between frames, as they'd really run.
fn section_paced(image: &[u8]) {
    const FRAME_GAP: Duration = Duration::from_micros(16_666);
    const FRAMES: usize = 180;

    println!("\n## Both methods at 60 frames a second, every light changing: CPU per frame, median");
    println!("| Lights | Original's method, one thread | Renderer, calling thread | Original / renderer |");
    println!("|---|---|---|---|");
    for count in COUNTS {
        let mut renderer = Renderer::headless(WIDTH, HEIGHT, 1.0).unwrap();
        renderer.set_background(image, WIDTH, HEIGHT);
        renderer.set_lights(&grid_lights(count));
        let colors = color_sets(count, 8);
        let mut after = Vec::with_capacity(FRAMES);
        for frame in 0..FRAMES {
            let start = thread_cpu_time();
            renderer.set_light_colors(&colors[frame % colors.len()]);
            renderer.render().unwrap();
            after.push(thread_cpu_time() - start);
            thread::sleep(FRAME_GAP);
        }
        after.sort_unstable();
        let after = percentile(&after, 0.5);
        drop(renderer);

        let lights: Vec<(i32, i32)> = grid_lights(count).iter().map(|l| (l.x as i32, l.y as i32)).collect();
        let baseline_colors: Vec<Color> = colors[0]
            .iter()
            .map(|&c| {
                let [r, g, b, a] = c.to_le_bytes();
                Color { a, r, g, b }
            })
            .collect();
        let mut fp = FastPixel::new();
        let mut before = Vec::with_capacity(FRAMES);
        for _ in 0..FRAMES {
            let start = thread_cpu_time();
            baseline_frame(&mut fp, image, &lights, &baseline_colors);
            before.push(thread_cpu_time() - start);
            thread::sleep(FRAME_GAP);
        }
        before.sort_unstable();
        let before = percentile(&before, 0.5);

        println!(
            "| {count} | {:.3} ms | {:.3} ms | {:.1}x |",
            ms(before),
            ms(after),
            before.as_secs_f64() / after.as_secs_f64(),
        );
    }
}

/// Dragging one shape on the display-design canvas, at 60 mouse moves a second. The renderer uploads the shape
/// and the handles. The original redraws everything, whatever moved.
fn section_edit(image: &[u8]) {
    const FRAME_GAP: Duration = Duration::from_micros(16_666);
    const FRAMES: usize = 180;
    const SHAPE_LIGHTS: usize = 100;

    println!("\n## Dragging a shape of {SHAPE_LIGHTS} lights while editing, at 60 mouse moves a second: CPU per move, median");
    println!("| Lights | Original's RenderInForeground | Renderer: update_light_range + set_overlay + render | Original / renderer |");
    println!("|---|---|---|---|");
    for count in COUNTS {
        let geometry = grid_lights(count);
        let colors = color_sets(count, 1).remove(0);

        let mut renderer = Renderer::headless(WIDTH, HEIGHT, 1.0).unwrap();
        renderer.set_background(image, WIDTH, HEIGHT);
        renderer.set_lights(&geometry);
        renderer.set_light_colors(&colors);
        renderer.render().unwrap();

        let mut after = Vec::with_capacity(FRAMES);
        let mut moved = geometry[..SHAPE_LIGHTS].to_vec();
        for frame in 0..FRAMES {
            let offset = (frame % 200) as f32;
            for (light, original) in moved.iter_mut().zip(&geometry) {
                light.x = original.x + offset;
            }
            let handle = |x: f32, y: f32| OverlaySquare { center: [x + offset, y], size: 6.0, color: rgba(255, 255, 255, 255) };
            let overlay = Overlay {
                lines: vec![
                    OverlayLine { a: [offset, 0.0], b: [offset + 400.0, 0.0], color: rgba(255, 255, 255, 255), width: 1.0 },
                    OverlayLine { a: [offset, 300.0], b: [offset + 400.0, 300.0], color: rgba(255, 255, 255, 255), width: 1.0 },
                ],
                squares: vec![handle(0.0, 0.0), handle(400.0, 0.0), handle(0.0, 300.0), handle(400.0, 300.0)],
                circles: vec![([offset + 200.0, -10.0], 3.0, rgba(255, 255, 255, 255))],
            };

            let start = thread_cpu_time();
            renderer.update_light_range(0, &moved);
            renderer.set_overlay(overlay);
            let stats = renderer.render().unwrap();
            after.push(thread_cpu_time() - start);
            assert_eq!(stats.lights_uploaded, 0, "dragging shouldn't re-send colours");
            thread::sleep(FRAME_GAP);
        }
        after.sort_unstable();
        let after = percentile(&after, 0.5);
        drop(renderer);

        let lights: Vec<(i32, i32)> = geometry.iter().map(|l| (l.x as i32, l.y as i32)).collect();
        let baseline_colors: Vec<Color> = colors
            .iter()
            .map(|&c| {
                let [r, g, b, a] = c.to_le_bytes();
                Color { a, r, g, b }
            })
            .collect();
        let mut back = vec![0u8; image.len()];
        let mut window = vec![0u8; image.len()];
        let mut before = Vec::with_capacity(FRAMES);
        for _ in 0..FRAMES {
            let start = thread_cpu_time();
            baseline_edit_frame(image, &lights, &baseline_colors, &mut back, &mut window);
            before.push(thread_cpu_time() - start);
            thread::sleep(FRAME_GAP);
        }
        before.sort_unstable();
        let before = percentile(&before, 0.5);

        println!(
            "| {count} | {:.3} ms | {:.3} ms | {:.0}x |",
            ms(before),
            ms(after),
            before.as_secs_f64() / after.as_secs_f64(),
        );
    }
}

/// The renderer at the resolution of a 5K display, at scale 2. The original has no equivalent.
fn section_hidpi() {
    const WIDTH_5K: u32 = 5120;
    const HEIGHT_5K: u32 = 2880;

    println!("\n## Renderer at {WIDTH_5K} x {HEIGHT_5K}, scale 2: every light changes every frame");
    println!("| Lights | CPU median | CPU p99 | Process CPU | Offscreen FPS | GPU frame |");
    println!("|---|---|---|---|---|---|");

    let (logical_width, logical_height) = (WIDTH_5K / 2, HEIGHT_5K / 2);
    let mut image = Vec::with_capacity((logical_width * logical_height * 4) as usize);
    for y in 0..logical_height {
        for x in 0..logical_width {
            image.extend_from_slice(&[(x / 12) as u8, (y / 7) as u8, ((x + y) / 16) as u8, 255]);
        }
    }

    for count in COUNTS {
        let columns = ((count as f64 * logical_width as f64 / logical_height as f64).sqrt().ceil() as usize).max(1);
        let rows = count.div_ceil(columns);
        let lights: Vec<_> = (0..count)
            .map(|i| LightGeom {
                x: (i % columns) as f32 * logical_width as f32 / columns as f32,
                y: (i / columns) as f32 * logical_height as f32 / rows as f32,
                size: LIGHT_SIZE as f32,
            })
            .collect();

        let mut renderer = Renderer::headless(WIDTH_5K, HEIGHT_5K, 2.0).unwrap();
        renderer.set_background(&image, logical_width, logical_height);
        renderer.set_lights(&lights);
        let colors = color_sets(count, 8);
        let result = measure(&mut renderer, &colors);
        let gpu = gpu_frame_time(&mut renderer, &colors);
        println!(
            "| {count} | {:.3} ms | {:.3} ms | {:.3} ms | {:.0} | {:.3} ms |",
            ms(result.cpu_median),
            ms(result.cpu_p99),
            ms(result.process_cpu_per_frame),
            result.frames_per_second,
            ms(gpu),
        );
    }
}

fn section_baseline(image: &[u8], renderer_cpu: &[Duration]) {
    println!("\n## Baseline: the original's CPU method, one 1920 x 1080 frame, plain Rust, flat out");
    println!("Copy the background, circle loop, copy to the output. All threads is Parallel.ForEach in UpdatePreview.");
    println!("| Lights | One thread | All threads, elapsed | All threads, total CPU | Renderer CPU median | One thread / renderer |");
    println!("|---|---|---|---|---|---|");
    for (index, count) in COUNTS.into_iter().enumerate() {
        let lights: Vec<(i32, i32)> = grid_lights(count).iter().map(|l| (l.x as i32, l.y as i32)).collect();
        let colors: Vec<Color> = color_sets(count, 1)
            .remove(0)
            .iter()
            .map(|&c| {
                let [r, g, b, a] = c.to_le_bytes();
                Color { a, r, g, b }
            })
            .collect();

        let mut fp = FastPixel::new();
        let mut single = Vec::new();
        let start = Instant::now();
        while start.elapsed() < MEASURE_FOR {
            let frame_start = thread_cpu_time();
            baseline_frame(&mut fp, image, &lights, &colors);
            single.push(thread_cpu_time() - frame_start);
        }
        single.sort_unstable();
        let single_median = percentile(&single, 0.5);
        let parallel = baseline_parallel(&lights, &colors, image, 100);

        println!(
            "| {count} | {:.2} ms | {:.2} ms | {:.2} ms | {:.3} ms | {:.0}x |",
            ms(single_median),
            ms(parallel.wall),
            ms(parallel.cpu),
            ms(renderer_cpu[index]),
            single_median.as_secs_f64() / renderer_cpu[index].as_secs_f64(),
        );
    }
}

fn main() {
    run_at_user_interactive();

    let requested: Vec<String> = std::env::args().skip(1).collect();
    let wants = |section: &str| requested.is_empty() || requested.iter().any(|name| name == section);

    let image = background();
    let probe = Renderer::headless(WIDTH, HEIGHT, 1.0).expect("the benchmark needs a GPU");
    println!("GPU: {}", probe.adapter_name());
    println!("CPU: {} threads", thread::available_parallelism().map_or(0, |n| n.get()));
    println!("Frame: {WIDTH} x {HEIGHT}, size {LIGHT_SIZE} lights, with a background image");
    drop(probe);

    let renderer_cpu = (wants("full") || wants("baseline")).then(|| section_full(&image));
    if wants("partial") {
        section_partial(&image);
    }
    if wants("live") {
        section_live(&image);
    }
    if wants("baseline") {
        section_baseline(&image, renderer_cpu.as_deref().unwrap());
    }
    if wants("paced") {
        section_paced(&image);
    }
    if wants("edit") {
        section_edit(&image);
    }
    if wants("hidpi") {
        section_hidpi();
    }
}
