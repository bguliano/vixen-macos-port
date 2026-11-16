//! The Rust side of `shim/liquid_shim.cpp`, which is `LiquidFunWrapper` in plain C++.

use std::ffi::c_void;
use std::ptr::NonNull;

/// `EmitterWrapper`.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct LfEmitter {
    pub x: i32,
    pub y: i32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub lifetime: i32,
    pub velocity: i32,
    pub source_size: i32,
    pub flow: i32,
    /// `WrapperParticleType`, in declaration order.
    pub particle_type: i32,
    pub angle: i32,
}

/// One element of `GetParticles`: a position rounded to a pixel, and a colour.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct LfParticle {
    pub x: i32,
    pub y: i32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

extern "C" {
    #[allow(clippy::too_many_arguments)]
    fn lf_world_create(
        width: i32,
        height: i32,
        bottom: bool,
        top: bool,
        left: bool,
        right: bool,
        particle_size: i32,
        lifetime: i32,
        seed: u32,
    ) -> *mut c_void;
    fn lf_world_step(world: *mut c_void, seconds: f32, mix_colors: bool, emitters: *const LfEmitter, emitter_count: usize);
    fn lf_world_particle_count(world: *mut c_void) -> usize;
    fn lf_world_particles(world: *mut c_void, out: *mut LfParticle, capacity: usize) -> usize;
    fn lf_world_destroy(world: *mut c_void);
}

/// Which sides of the display area stop particles.
#[derive(Copy, Clone, Debug, Default)]
pub struct Barriers {
    pub bottom: bool,
    pub top: bool,
    pub left: bool,
    pub right: bool,
}

/// A running simulation. `Initialize` creates it and dropping it is `FinalizeWorld`.
pub struct LiquidWorld {
    handle: NonNull<c_void>,
}

// SAFETY: the world owns all the memory it points to and nothing in it is tied to a thread. `&mut self` on every
// call keeps it to one thread at a time, and the shim sets up LiquidFun's shared tables before any world exists.
unsafe impl Send for LiquidWorld {}

impl LiquidWorld {
    /// `Initialize`. `seed` starts the C runtime `rand()` sequence the wrapper draws from. None if out of memory.
    pub fn new(width: i32, height: i32, barriers: Barriers, particle_size: i32, lifetime: i32, seed: u32) -> Option<LiquidWorld> {
        // A radius of zero would divide by zero throughout the solver. The editor's slider starts at 1.
        let size = particle_size.max(1);
        let Barriers { bottom, top, left, right } = barriers;
        // SAFETY: plain values. The returned handle is checked and owned by the new value.
        let handle = unsafe { lf_world_create(width, height, bottom, top, left, right, size, lifetime, seed) };
        NonNull::new(handle).map(|handle| LiquidWorld { handle })
    }

    /// `StepWorld`: advances the simulation, emits this frame's particles and removes those off the display area.
    pub fn step(&mut self, step_seconds: f32, mix_colors: bool, emitters: &[LfEmitter]) {
        // SAFETY: the handle is live for as long as self, and the slice outlives the call.
        unsafe { lf_world_step(self.handle.as_ptr(), step_seconds, mix_colors, emitters.as_ptr(), emitters.len()) }
    }

    /// `GetParticles`, into a buffer the caller reuses.
    pub fn particles(&mut self, out: &mut Vec<LfParticle>) {
        out.clear();
        // SAFETY: the handle is live, and the shim writes at most `capacity` elements to the start of the
        // spare capacity, then reports how many. Every bit pattern of those fields is valid.
        unsafe {
            let count = lf_world_particle_count(self.handle.as_ptr());
            out.reserve(count);
            let written = lf_world_particles(self.handle.as_ptr(), out.as_mut_ptr(), out.capacity());
            out.set_len(written);
        }
    }
}

impl Drop for LiquidWorld {
    fn drop(&mut self) {
        // SAFETY: the handle came from lf_world_create and this is the only place it is destroyed.
        unsafe { lf_world_destroy(self.handle.as_ptr()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 0.05;

    fn emitter(x: i32, y: i32, flow: i32) -> LfEmitter {
        LfEmitter { x, y, r: 255, g: 255, b: 255, lifetime: 1000, velocity: 0, source_size: 0, flow, particle_type: 6, angle: 0 }
    }

    fn world(width: i32, height: i32, barriers: Barriers) -> LiquidWorld {
        LiquidWorld::new(width, height, barriers, 500, 1000, 1).unwrap()
    }

    fn particles(w: &mut LiquidWorld) -> Vec<LfParticle> {
        let mut out = Vec::new();
        w.particles(&mut out);
        out
    }

    /// Steps with no emitters.
    fn run(w: &mut LiquidWorld, frames: i32) {
        for _ in 0..frames {
            w.step(DT, false, &[]);
        }
    }

    #[test]
    fn flow_is_particles_per_step_around_the_emitter() {
        let mut w = world(30, 30, Barriers::default());
        w.step(DT, false, &[emitter(15, 20, 7)]);
        let p = particles(&mut w);
        assert_eq!(p.len(), 7);
        // A source size of zero scatters them half a unit around the emitter.
        assert!(p.iter().all(|p| (14..=16).contains(&p.x) && (19..=21).contains(&p.y)), "{p:?}");
    }

    #[test]
    fn particles_fall_and_leave_the_bottom() {
        let mut w = world(30, 30, Barriers::default());
        w.step(DT, false, &[emitter(15, 5, 1)]);
        assert_eq!(particles(&mut w).len(), 1);
        let start = particles(&mut w)[0].y;
        run(&mut w, 8);
        assert!(particles(&mut w)[0].y < start, "gravity pulls towards y = 0");
        run(&mut w, 200);
        assert!(particles(&mut w).is_empty(), "off the bottom they are destroyed");
    }

    #[test]
    fn a_bottom_barrier_holds_them() {
        // One particle, because several released on the same spot push each other apart hard enough to leave sideways.
        let mut w = world(30, 30, Barriers { bottom: true, ..Barriers::default() });
        w.step(DT, false, &[emitter(15, 5, 1)]);
        run(&mut w, 100);
        let p = particles(&mut w);
        assert_eq!(p.len(), 1, "the barrier keeps it from leaving");
        assert!(p.iter().all(|p| p.y >= -1 && p.y <= 2), "{p:?}");
    }

    #[test]
    fn lifetime_is_in_hundredths_of_a_second() {
        let mut w = world(30, 30, Barriers { bottom: true, ..Barriers::default() });
        let e = LfEmitter { lifetime: 50, ..emitter(15, 5, 1) };
        w.step(DT, false, &[e]);
        // 0.5 s, less 1% and plus up to 20%.
        run(&mut w, 8);
        assert_eq!(particles(&mut w).len(), 1, "still alive at 0.4 s");
        run(&mut w, 6);
        assert!(particles(&mut w).is_empty(), "gone by 0.7 s");
    }

    #[test]
    fn a_lifetime_of_zero_is_very_short_not_forever() {
        let mut w = world(30, 30, Barriers { bottom: true, ..Barriers::default() });
        w.step(DT, false, &[LfEmitter { lifetime: 0, ..emitter(15, 5, 1) }]);
        assert_eq!(particles(&mut w).len(), 1);
        run(&mut w, 2);
        assert!(particles(&mut w).is_empty());
    }

    #[test]
    fn the_nozzle_angle_sets_the_direction() {
        for (angle, dx, dy) in [(0, 1, 0), (180, -1, 0), (90, 0, 1)] {
            let mut w = world(60, 60, Barriers::default());
            let e = LfEmitter { velocity: 50, angle, ..emitter(30, 30, 1) };
            w.step(DT, false, &[e]);
            run(&mut w, 5);
            let p = particles(&mut w)[0];
            if dx != 0 {
                assert_eq!((p.x - 30).signum(), dx, "angle {angle}: {p:?}");
            }
            if dy != 0 {
                assert_eq!((p.y - 30).signum(), dy, "angle {angle}: {p:?}");
            }
        }
    }

    #[test]
    fn a_source_size_spreads_across_the_nozzle() {
        // At 0 degrees the nozzle is vertical, so the particles spread in y only, in one direction, by at most half
        // the size, scaled by height / 200.
        let mut w = world(100, 100, Barriers::default());
        let e = LfEmitter { source_size: 100, velocity: 0, ..emitter(50, 40, 30) };
        w.step(DT, false, &[e]);
        let p = particles(&mut w);
        assert!(p.iter().all(|p| p.x == 50), "{p:?}");
        assert!(p.iter().all(|p| p.y >= 40 && p.y <= 65), "{p:?}");
        assert!(p.iter().any(|p| p.y > 45), "{p:?}");
    }

    /// The colour arithmetic of the wrapper: down to a float, into a byte, back through a float and truncated.
    #[test]
    fn colours_survive_the_float_trip_as_the_cpp_computes_them() {
        let through = |c: u8| {
            let inverse = 1.0f32 / 255.0;
            let stored = (255.0f32 * (c as f32 / 255.0)) as u8;
            ((inverse * stored as f32) * 255.0) as i32 as u8
        };
        let mut w = world(30, 30, Barriers::default());
        for c in [0u8, 1, 37, 128, 200, 254, 255] {
            let e = LfEmitter { r: c, g: c, b: c, ..emitter(15, 15, 1) };
            w.step(DT, false, &[e]);
        }
        let got: Vec<u8> = particles(&mut w).iter().map(|p| p.r).collect();
        let want: Vec<u8> = [0u8, 1, 37, 128, 200, 254, 255].iter().map(|c| through(*c)).collect();
        assert_eq!(got, want);
    }

    #[test]
    fn mixing_colours_needs_the_flag() {
        let red = LfEmitter { r: 255, g: 0, b: 0, ..emitter(15, 15, 40) };
        let blue = LfEmitter { r: 0, g: 0, b: 255, ..emitter(15, 15, 40) };
        let mixed = |mix: bool| {
            let mut w = world(30, 30, Barriers { bottom: true, ..Barriers::default() });
            for _ in 0..20 {
                w.step(DT, mix, &[red, blue]);
            }
            particles(&mut w).iter().filter(|p| p.r > 0 && p.b > 0).count()
        };
        assert_eq!(mixed(false), 0);
        assert!(mixed(true) > 0);
    }

    #[test]
    fn the_same_seed_gives_the_same_particles() {
        let run_it = |seed: u32| {
            let mut w = LiquidWorld::new(30, 30, Barriers::default(), 500, 1000, seed).unwrap();
            for _ in 0..30 {
                w.step(DT, true, &[emitter(15, 25, 10)]);
            }
            particles(&mut w)
        };
        assert_eq!(run_it(7), run_it(7));
        assert_ne!(run_it(7), run_it(8));
    }

    #[test]
    fn worlds_run_on_many_threads_at_once() {
        let handles: Vec<_> = (0..8)
            .map(|seed| {
                std::thread::spawn(move || {
                    let mut w = LiquidWorld::new(20, 20, Barriers::default(), 500, 1000, seed).unwrap();
                    for _ in 0..50 {
                        w.step(DT, true, &[emitter(10, 15, 5)]);
                    }
                    particles(&mut w).len()
                })
            })
            .collect();
        for h in handles {
            assert!(h.join().unwrap() > 0);
        }
    }

    #[test]
    fn every_particle_type_runs() {
        // Some of these form pairs and triads, which takes the solver down paths plain water doesn't.
        for particle_type in 0..9 {
            let mut w = world(30, 30, Barriers { bottom: true, top: true, left: true, right: true });
            let e = LfEmitter { particle_type, velocity: 10, source_size: 20, angle: 45, ..emitter(15, 20, 20) };
            for _ in 0..40 {
                w.step(DT, true, &[e]);
            }
            assert!(!particles(&mut w).is_empty(), "type {particle_type}");
        }
    }

    #[test]
    fn a_particle_size_of_zero_is_survivable() {
        let mut w = LiquidWorld::new(20, 20, Barriers::default(), 0, 1000, 1).unwrap();
        for _ in 0..10 {
            w.step(DT, false, &[emitter(10, 15, 5)]);
        }
        assert!(!particles(&mut w).is_empty());
    }
}
