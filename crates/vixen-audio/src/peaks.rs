//! Multi-resolution min/max waveform peaks.
//!
//! Upstream's `Audio.GetSamples(numPixels)` splits the (interleaved) sample array into `numPixels`
//! windows and keeps `(min, max)` of each, `Low`/`High` in `Sample`; empty windows are `(0, 0)`.
//! `Peaks::range` does the same per pixel column, but with exact integer window boundaries and mip
//! levels so any zoom/scroll is O(buckets * log n). Min/max is over all channels of each frame, like
//! upstream, which scans interleaved samples.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use vixen_core::TimeSpan;

use crate::clip::{time_to_frames, AudioClip};

/// Frames per level-0 block.
const BLOCK: usize = 64;

pub struct Peaks {
    clip: Arc<AudioClip>,
    /// levels[l][i] covers frames `[i * BLOCK << l, (i + 1) * BLOCK << l)`. Only full blocks are stored.
    levels: Vec<Vec<[f32; 2]>>,
}

impl Peaks {
    /// Build all mip levels. Cheap (one pass over the samples); run it on a worker thread.
    pub fn compute(clip: Arc<AudioClip>) -> Peaks {
        Self::compute_cancellable(clip, &AtomicBool::new(false)).expect("not cancelled")
    }

    /// Like [`Peaks::compute`], but returns `None` if `cancel` becomes true.
    pub fn compute_cancellable(clip: Arc<AudioClip>, cancel: &AtomicBool) -> Option<Peaks> {
        let ch = clip.channels() as usize;
        let nblocks = clip.frames() / BLOCK;
        let mut l0: Vec<[f32; 2]> = Vec::with_capacity(nblocks);
        let samples = clip.samples();
        for (i, block) in samples.chunks_exact(BLOCK * ch).enumerate() {
            if i % 4096 == 0 && cancel.load(Ordering::Relaxed) {
                return None;
            }
            l0.push(min_max(block));
        }
        let mut levels = vec![l0];
        loop {
            let prev = levels.last().unwrap();
            if prev.len() < 2 {
                break;
            }
            let next: Vec<[f32; 2]> = prev
                .chunks_exact(2)
                .map(|p| [p[0][0].min(p[1][0]), p[0][1].max(p[1][1])])
                .collect();
            levels.push(next);
        }
        Some(Peaks { clip, levels })
    }

    pub fn clip(&self) -> &Arc<AudioClip> {
        &self.clip
    }

    /// Frame window `[start_frame, end_frame)` for a time range, clamped to the clip.
    pub fn frame_range(&self, start: TimeSpan, end: TimeSpan) -> (usize, usize) {
        let sr = self.clip.sample_rate();
        let total = self.clip.frames();
        let s = (time_to_frames(start, sr).floor() as usize).min(total);
        let e = (time_to_frames(end, sr).ceil() as usize).min(total).max(s);
        (s, e)
    }

    /// Frame window of bucket `i` of `buckets` over `[start_frame, end_frame)`: `start + i*span/buckets`
    /// to `start + (i+1)*span/buckets` (integer division), widened to at least one frame.
    pub fn bucket_frames(start_frame: usize, end_frame: usize, buckets: usize, i: usize) -> (usize, usize) {
        let span = end_frame.saturating_sub(start_frame) as u128;
        let b = buckets.max(1) as u128;
        let f0 = start_frame + (i as u128 * span / b) as usize;
        let f1 = start_frame + ((i as u128 + 1) * span / b) as usize;
        (f0, f1.max(f0 + 1))
    }

    /// `(min, max)` per bucket across `[start, end)`. Buckets that fall outside the clip are `(0, 0)`.
    /// The window is `frame_range(start, end)`; see [`Peaks::bucket_frames`] for the boundaries.
    pub fn range(&self, start: TimeSpan, end: TimeSpan, buckets: usize) -> Vec<(f32, f32)> {
        let (s, e) = self.frame_range(start, end);
        self.range_frames(s, e, buckets)
    }

    /// Same as [`Peaks::range`] with explicit frame bounds.
    pub fn range_frames(&self, s: usize, e: usize, buckets: usize) -> Vec<(f32, f32)> {
        let total = self.clip.frames();
        let mut out = Vec::with_capacity(buckets);
        for i in 0..buckets {
            let (f0, f1) = Self::bucket_frames(s, e, buckets, i);
            if f0 >= total {
                out.push((0.0, 0.0));
                continue;
            }
            let f1 = f1.min(total);
            let r = self.min_max_frames(f0, f1);
            out.push((r[0], r[1]));
        }
        out
    }

    fn raw(&self, f0: usize, f1: usize) -> [f32; 2] {
        let ch = self.clip.channels() as usize;
        min_max(&self.clip.samples()[f0 * ch..f1 * ch])
    }

    fn min_max_frames(&self, f0: usize, f1: usize) -> [f32; 2] {
        debug_assert!(f0 < f1);
        if f1 - f0 <= 2 * BLOCK {
            return self.raw(f0, f1);
        }
        let first = f0.div_ceil(BLOCK);
        let last = f1 / BLOCK;
        let mut acc = [f32::INFINITY, f32::NEG_INFINITY];
        let merge = |acc: &mut [f32; 2], v: [f32; 2]| {
            acc[0] = acc[0].min(v[0]);
            acc[1] = acc[1].max(v[1]);
        };
        if first * BLOCK > f0 {
            merge(&mut acc, self.raw(f0, first * BLOCK));
        }
        if last * BLOCK < f1 {
            merge(&mut acc, self.raw(last * BLOCK, f1));
        }
        let (mut lo, mut hi, mut lvl) = (first, last, 0usize);
        while lo < hi {
            if lo & 1 == 1 {
                merge(&mut acc, self.levels[lvl][lo]);
                lo += 1;
            }
            if hi & 1 == 1 {
                hi -= 1;
                merge(&mut acc, self.levels[lvl][hi]);
            }
            lo >>= 1;
            hi >>= 1;
            lvl += 1;
        }
        acc
    }
}

fn min_max(s: &[f32]) -> [f32; 2] {
    let mut mn = f32::INFINITY;
    let mut mx = f32::NEG_INFINITY;
    for &v in s {
        mn = mn.min(v);
        mx = mx.max(v);
    }
    if mn > mx {
        [0.0, 0.0]
    } else {
        [mn, mx]
    }
}
