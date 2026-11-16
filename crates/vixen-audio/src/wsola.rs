//! Pitch-preserving time stretching: WSOLA (waveform-similarity overlap-add).
//!
//! Upstream uses SoundTouch for speed changes; this is the same family of algorithm. The whole clip is
//! in memory, so the stretcher reads the source at random: each output hop of `hs` frames is built from a
//! Hann-windowed grain of `2 * hs` frames taken near the nominal analysis position (advancing
//! `hs * speed` per hop), shifted within +-`delta` to the offset whose waveform best continues the
//! previous grain. No allocation after construction.

use crate::clip::AudioClip;

/// Borrowed source for the stretcher.
#[derive(Clone, Copy)]
pub struct Src<'a> {
    pub interleaved: &'a [f32],
    pub mono: &'a [f32],
    pub channels: usize,
}

impl<'a> Src<'a> {
    pub fn of(clip: &'a AudioClip) -> Src<'a> {
        Src { interleaved: clip.samples(), mono: clip.mono(), channels: clip.channels() as usize }
    }
    #[inline]
    fn frames(&self) -> i64 {
        self.mono.len() as i64
    }
    #[inline]
    fn frame_sample(&self, frame: i64, c: usize) -> f32 {
        if frame < 0 || frame >= self.frames() {
            0.0
        } else {
            self.interleaved[frame as usize * self.channels + c]
        }
    }
}

pub struct Wsola {
    ch: usize,
    hs: usize,
    delta: i64,
    window: Vec<f32>,
    /// Windowed second half of the previous grain (`hs` frames, interleaved).
    tail: Vec<f32>,
    /// Finished output hop (`hs` frames, interleaved) and read cursor / length in frames.
    hop: Vec<f32>,
    hop_pos: usize,
    hop_len: usize,
    /// Similarity weight (Hann over `hs`) and the weighted reference for the correlation search.
    weight: Vec<f32>,
    refbuf: Vec<f32>,
    /// Nominal analysis position of the next grain.
    nom: f64,
    prev_start: i64,
    started: bool,
    finished: bool,
    /// Nominal source position of the next frame handed out.
    out_pos: f64,
}

impl Wsola {
    pub fn new(sample_rate: u32, channels: usize) -> Wsola {
        let hs = ((sample_rate as f64 * 0.020).round() as usize).max(16);
        let delta = ((sample_rate as f64 * 0.010).round() as i64).max(4);
        let n = 2 * hs;
        let window = (0..n)
            .map(|i| (0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos())) as f32)
            .collect();
        Wsola {
            ch: channels,
            hs,
            delta,
            window,
            tail: vec![0.0; hs * channels],
            hop: vec![0.0; hs * channels],
            hop_pos: 0,
            hop_len: 0,
            weight: (0..hs)
                .map(|i| (0.5 * (1.0 - (2.0 * std::f64::consts::PI * (i as f64 + 0.5) / hs as f64).cos())) as f32)
                .collect(),
            refbuf: vec![0.0; hs],
            nom: 0.0,
            prev_start: 0,
            started: false,
            finished: false,
            out_pos: 0.0,
        }
    }

    /// Start (or restart) producing output from source frame `pos`.
    pub fn reset(&mut self, pos: f64) {
        self.nom = pos;
        self.out_pos = pos;
        self.started = false;
        self.finished = false;
        self.hop_pos = 0;
        self.hop_len = 0;
    }

    /// Nominal source position (frames) of the next output frame.
    pub fn position(&self) -> f64 {
        self.out_pos
    }

    pub fn is_finished(&self) -> bool {
        self.finished && self.hop_pos >= self.hop_len
    }

    /// Fill `out` (interleaved, `out.len() / channels` frames) at the given tempo ratio. Returns the
    /// number of frames written; fewer than requested means the source ran out.
    pub fn render(&mut self, src: Src<'_>, speed: f32, out: &mut [f32]) -> usize {
        let ch = self.ch;
        let want = out.len() / ch;
        let mut done = 0;
        while done < want {
            if self.hop_pos >= self.hop_len {
                if self.finished {
                    break;
                }
                self.next_hop(src, speed as f64);
                if self.hop_len == 0 {
                    break;
                }
            }
            let n = (self.hop_len - self.hop_pos).min(want - done);
            out[done * ch..(done + n) * ch]
                .copy_from_slice(&self.hop[self.hop_pos * ch..(self.hop_pos + n) * ch]);
            self.hop_pos += n;
            done += n;
        }
        self.out_pos += done as f64 * speed as f64;
        done
    }

    fn next_hop(&mut self, src: Src<'_>, speed: f64) {
        let (ch, hs) = (self.ch, self.hs);
        let total = src.frames();
        self.hop_pos = 0;
        if !self.started {
            let a = self.nom.round() as i64;
            if a >= total {
                self.finished = true;
                self.hop_len = 0;
                return;
            }
            // First hop is the raw source (no fade-in); its windowed second half is the tail.
            for i in 0..hs {
                for c in 0..ch {
                    self.hop[i * ch + c] = src.frame_sample(a + i as i64, c);
                    self.tail[i * ch + c] = self.window[hs + i] * src.frame_sample(a + (hs + i) as i64, c);
                }
            }
            self.prev_start = a;
            self.started = true;
            self.nom += hs as f64 * speed;
            self.hop_len = hs;
            return;
        }
        let target = self.nom.round() as i64;
        if target >= total {
            // Nothing left to splice in: flush the tail and stop.
            self.hop.copy_from_slice(&self.tail);
            self.hop_len = hs;
            self.finished = true;
            return;
        }
        let cont = self.prev_start + hs as i64;
        let g = self.search(src, cont, target);
        for i in 0..hs {
            let w = self.window[i];
            let w2 = self.window[hs + i];
            for c in 0..ch {
                let idx = i * ch + c;
                self.hop[idx] = self.tail[idx] + w * src.frame_sample(g + i as i64, c);
                self.tail[idx] = w2 * src.frame_sample(g + (hs + i) as i64, c);
            }
        }
        self.prev_start = g;
        self.nom += hs as f64 * speed;
        self.hop_len = hs;
    }

    /// Best grain start near `target`: the candidate whose first `hs` frames correlate best with what
    /// followed the previous grain in the source (`cont`).
    fn search(&mut self, src: Src<'_>, cont: i64, target: i64) -> i64 {
        let hs = self.hs;
        for k in 0..hs {
            let i = cont + k as i64;
            let v = if i >= 0 && i < src.frames() { src.mono[i as usize] } else { 0.0 };
            self.refbuf[k] = v * self.weight[k];
        }
        let lo = (target - self.delta).max(0);
        let hi = target + self.delta;
        if lo > hi {
            return target;
        }
        let mut best = target;
        // Scores are only comparable at the same stride, so coarse and fine passes each stay consistent.
        let mut best_score = self.score(src, target, 4);
        let mut c = lo;
        while c <= hi {
            let s = self.score(src, c, 4);
            if s > best_score {
                best_score = s;
                best = c;
            }
            c += 4;
        }
        let (rlo, rhi) = ((best - 3).max(lo), (best + 3).min(hi));
        let mut c = rlo;
        let mut fine_best = best;
        let mut fine_score = self.score(src, best, 1);
        while c <= rhi {
            let s = self.score(src, c, 1);
            if s > fine_score {
                fine_score = s;
                fine_best = c;
            }
            c += 1;
        }
        fine_best
    }

    /// Hann-weighted normalised correlation (the taper removes the phase bias a rectangular window has at
    /// low frequencies) of `refbuf` against `mono[c..]`, sampling every `stride`-th frame.
    fn score(&self, src: Src<'_>, c: i64, stride: usize) -> f32 {
        let len = src.mono.len() as i64;
        let k0 = (-c).max(0) as usize;
        let k1 = ((len - c).max(0) as usize).min(self.hs);
        if k0 >= k1 {
            return f32::NEG_INFINITY;
        }
        let mut dot = 0.0f32;
        let mut e = 1e-9f32;
        let mut k = k0.div_ceil(stride) * stride;
        while k < k1 {
            let v = src.mono[(c + k as i64) as usize];
            dot += v * self.refbuf[k];
            e += v * v * self.weight[k];
            k += stride;
        }
        dot / e.sqrt()
    }
}

/// Offline time stretch of a whole clip (interleaved result). `speed` 1.0 returns an exact copy.
pub fn time_stretch(clip: &AudioClip, speed: f32) -> Vec<f32> {
    if (speed - 1.0).abs() < 1e-6 {
        return clip.samples().to_vec();
    }
    let ch = clip.channels() as usize;
    let mut w = Wsola::new(clip.sample_rate(), ch);
    w.reset(0.0);
    let src = Src::of(clip);
    let mut out = Vec::with_capacity((clip.samples().len() as f64 / speed as f64) as usize + 8 * w.hs * ch);
    let mut buf = vec![0.0f32; 1024 * ch];
    loop {
        let n = w.render(src, speed, &mut buf);
        out.extend_from_slice(&buf[..n * ch]);
        if n < 1024 {
            break;
        }
    }
    out
}
