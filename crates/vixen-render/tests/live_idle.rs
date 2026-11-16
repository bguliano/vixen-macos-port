//! An idle preview must cost no CPU. This is a test binary of its own, with one test, so that the process's CPU
//! time is only this renderer's.

use std::thread;
use std::time::Duration;

use vixen_render::{rgba, LightGeom, LiveRenderer, RenderError};

fn process_cpu_time() -> Duration {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY: `usage` is a valid rusage, and RUSAGE_SELF is a valid target.
    assert_eq!(unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) }, 0);
    let time = |t: libc::timeval| Duration::new(t.tv_sec as u64, t.tv_usec as u32 * 1000);
    time(usage.ru_utime) + time(usage.ru_stime)
}

#[test]
fn an_idle_preview_costs_no_cpu() {
    let live = match LiveRenderer::headless(1920, 1080, 1.0) {
        Ok(live) => live,
        Err(RenderError::NoAdapter) if std::env::var_os("VIXEN_RENDER_REQUIRE_GPU").is_none() => {
            eprintln!("skipping: no GPU adapter on this machine");
            return;
        }
        Err(error) => panic!("{error}"),
    };

    let lights: Vec<_> =
        (0..100_000).map(|i| LightGeom { x: (i % 1920) as f32, y: (i / 1920) as f32, size: 3.0 }).collect();
    live.set_lights(&lights);
    live.publish_colors(&vec![rgba(255, 0, 0, 255); lights.len()]);
    live.read_pixels();

    // Let GPU completion handlers and the driver's own bookkeeping finish.
    thread::sleep(Duration::from_millis(200));
    let frames_before = live.stats().frames_rendered;
    let cpu_before = process_cpu_time();

    let idle = Duration::from_secs(2);
    thread::sleep(idle);

    let cpu = process_cpu_time() - cpu_before;
    assert_eq!(live.stats().frames_rendered, frames_before);
    eprintln!("{cpu:?} of CPU across {idle:?} idle");
    // 0.1%. Busy-polling, or a timer at anything like display rate, would be far beyond this.
    assert!(cpu < idle / 1000, "an idle preview used {cpu:?} of CPU in {idle:?}");
}
