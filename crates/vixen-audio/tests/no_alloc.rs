//! The audio callback must not allocate. Counts allocations on this thread while pumping.
mod common;
use common::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::time::Duration;
use vixen_audio::*;
use vixen_core::TimeSpan;

struct Counting;
thread_local!(static COUNT: Cell<usize> = const { Cell::new(0) });
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let _ = COUNT.try_with(|c| c.set(c.get() + 1));
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        let _ = COUNT.try_with(|c| c.set(c.get() + 1));
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let _ = COUNT.try_with(|c| c.set(c.get() + 1));
        System.realloc(p, l, n)
    }
}
#[global_allocator]
static A: Counting = Counting;

#[test]
fn callback_does_not_allocate() {
    let clip = sine_clip(44100, 2, 20.0, 440.0, 0.5);
    let fake = FakeBackend::new(48000, 2);
    let p = AudioPlayer::with_backend(Box::new(fake.clone()), None);
    p.set_clip(clip);
    let mut buf = vec![0.0f32; 2 * 3000];
    p.play();
    fake.pump_into(&mut buf); // warm up
    for (preserve_pitch, speed) in [(true, 1.0f32), (true, 0.7), (true, 4.0), (false, 0.1), (false, 1.6), (false, 4.0), (true, 1.0)] {
        p.set_rate_mode(preserve_pitch);
        p.set_speed(speed);
        p.set_volume(0.8);
        let before = COUNT.with(|c| c.get());
        for i in 0..50 {
            fake.advance_clock(Duration::from_millis(10));
            if i == 25 {
                p.seek(TimeSpan::from_secs(3.0));
            }
            let _ = p.position();
            fake.pump_into(&mut buf[..2 * (200 + i * 50)]);
        }
        let after = COUNT.with(|c| c.get());
        assert_eq!(before, after, "allocations in the audio path at speed {speed}");
    }
}
