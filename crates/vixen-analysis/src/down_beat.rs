/*
    QM DSP Library

    Centre for Digital Music, Queen Mary, University of London.
    This file copyright 2008-2009 Matthew Davies and QMUL.

    This program is free software; you can redistribute it and/or
    modify it under the terms of the GNU General Public License as
    published by the Free Software Foundation; either version 2 of the
    License, or (at your option) any later version.  See the file
    COPYING included with this distribution for more information.
*/

//! Port of `dsp/tempotracking/DownBeat.cpp`.

use std::f64::consts::PI;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::decimator::Decimator;
use crate::fft::FftReal;
use crate::math_utilities::{adaptive_threshold, get_max, next_power_of_two};
use crate::Cancelled;

// From maths/MathAliases.h, not the larger EPS in TempoTrackV2.cpp.
const EPS: f64 = 2.2204e-016;

pub struct DownBeat {
    bpb: i32,
    factor: usize,
    increment: usize,
    decimator1: Option<Decimator>,
    decimator2: Option<Decimator>,
    buffer: Vec<f32>,
    decbuf: Vec<f32>,
    beat_frame_size: usize,
    beat_frame: Vec<f64>,
    fft: FftReal,
    fft_real_out: Vec<f64>,
    fft_imag_out: Vec<f64>,
    beat_sd: Vec<f64>,
}

impl DownBeat {
    /// `decimation_factor` is a power of two no greater than 64, and `df_increment` a multiple of it.
    pub fn new(original_sample_rate: f32, decimation_factor: usize, df_increment: usize) -> DownBeat {
        // beat frame size is next power of two up from 1.3 seconds at the
        // downsampled rate (happens to produce 4096 for 44100 or 48000 at
        // 16x decimation, which is our expected normal situation)
        let downsampled = original_sample_rate / decimation_factor as f32;
        let mut beat_frame_size = next_power_of_two((downsampled as f64 * 1.3) as i32) as usize;
        if beat_frame_size < 2 {
            beat_frame_size = 2;
        }
        DownBeat {
            bpb: 0,
            factor: decimation_factor,
            increment: df_increment,
            decimator1: None,
            decimator2: None,
            buffer: Vec::new(),
            decbuf: Vec::new(),
            beat_frame_size,
            beat_frame: vec![0.0; beat_frame_size],
            // A power of two of at least 2, so never odd.
            fft: FftReal::new(beat_frame_size).expect("power of two sizes are even"),
            fft_real_out: vec![0.0; beat_frame_size],
            fft_imag_out: vec![0.0; beat_frame_size],
            beat_sd: Vec::new(),
        }
    }

    pub fn beat_frame_size(&self) -> usize {
        self.beat_frame_size
    }

    pub fn set_beats_per_bar(&mut self, bpb: i32) {
        self.bpb = bpb;
    }

    fn make_decimators(&mut self) {
        if self.factor < 2 {
            return;
        }
        let highest = Decimator::HIGHEST_SUPPORTED_FACTOR;
        if self.factor <= highest {
            self.decimator1 = Some(Decimator::new(self.increment, self.factor));
            return;
        }
        self.decimator1 = Some(Decimator::new(self.increment, highest));
        self.decimator2 = Some(Decimator::new(self.increment / highest, self.factor / highest));
        self.decbuf = vec![0.0; self.increment / highest];
    }

    /// Call with blocks holding at least `df_increment` samples at the original rate.
    pub fn push_audio_block(&mut self, audio: &[f32]) {
        if self.decimator1.is_none() && self.factor > 1 {
            self.make_decimators();
        }
        let fill = self.buffer.len();
        self.buffer.resize(fill + self.increment / self.factor, 0.0);
        let target = &mut self.buffer[fill..];
        match (&mut self.decimator1, &mut self.decimator2) {
            (Some(first), Some(second)) => {
                first.process(audio, &mut self.decbuf);
                second.process(&self.decbuf, target);
            }
            (Some(first), None) => first.process(audio, target),
            // just copy across (factor is presumably 1)
            _ => target.copy_from_slice(&audio[..self.increment]),
        }
    }

    pub fn buffered_audio(&self) -> &[f32] {
        &self.buffer
    }

    /// Only the buffer: the decimators keep their filter state across a reset, as in the C++.
    pub fn reset_audio_buffer(&mut self) {
        self.buffer = Vec::new();
    }

    /// `findDownBeats` on the audio buffered by [`push_audio_block`](Self::push_audio_block).
    pub fn find_down_beats_in_buffer(
        &mut self,
        beats: &[f64],
        downbeats: &mut Vec<i32>,
        cancel: &AtomicBool,
    ) -> Result<(), Cancelled> {
        let audio = std::mem::take(&mut self.buffer);
        let result = self.find_down_beats(&audio, beats, downbeats, cancel);
        self.buffer = audio;
        result
    }

    /// `audio` is downsampled, `beats` are in df increment units, and `downbeats` receives indices into `beats`.
    pub fn find_down_beats(
        &mut self,
        audio: &[f32],
        beats: &[f64],
        downbeats: &mut Vec<i32>,
        cancel: &AtomicBool,
    ) -> Result<(), Cancelled> {
        // FIND DOWNBEATS BY PARTITIONING THE INPUT AUDIO FILE INTO BEAT SEGMENTS
        // WHERE THE AUDIO FRAMES ARE DOWNSAMPLED  BY A FACTOR OF 16 (fs ~= 2700Hz)
        // THEN TAKING THE JENSEN-SHANNON DIVERGENCE BETWEEN BEAT SYNCHRONOUS SPECTRAL FRAMES

        // IMPLEMENTATION (MOSTLY) FOLLOWS:
        //  DAVIES AND PLUMBLEY "A SPECTRAL DIFFERENCE APPROACH TO EXTRACTING DOWNBEATS IN MUSICAL AUDIO"
        //  EUSIPCO 2006, FLORENCE, ITALY

        let half = self.beat_frame_size / 2;
        let mut newspec = vec![0.0; half]; // magnitude spectrum of current beat
        let mut oldspec = vec![0.0; half]; // magnitude spectrum of previous beat

        self.beat_sd.clear();

        let audio_length = audio.len();
        if audio_length == 0 {
            return Ok(());
        }

        let (increment, factor) = (self.increment as f64, self.factor as f64);

        let mut i = 0;
        while i + 1 < beats.len() {
            if cancel.load(Ordering::Relaxed) {
                return Err(Cancelled);
            }

            // Copy the extents of the current beat from downsampled array
            // into beat frame buffer

            let beatstart = ((beats[i] * increment) / factor) as usize;
            let mut beatend = ((beats[i + 1] * increment) / factor) as usize;
            if beatend >= audio_length {
                beatend = audio_length - 1;
            }
            if beatend < beatstart {
                beatend = beatstart;
            }
            let beatlen = beatend - beatstart;

            // Also apply a Hanning window to the beat frame buffer, sized
            // to the beat extents rather than the frame size.  (Because
            // the size varies, it's easier to do this by hand than use
            // our Window abstraction.)

            let filled = beatlen.min(self.beat_frame_size);
            for j in 0..filled {
                let mul = 0.5 * (1.0 - (2.0 * PI * (j as f64 / beatlen as f64)).cos());
                self.beat_frame[j] = audio[beatstart + j] as f64 * mul;
            }

            for value in self.beat_frame[filled..].iter_mut() {
                *value = 0.0;
            }

            // Now FFT beat frame

            self.fft.forward(&self.beat_frame, &mut self.fft_real_out, &mut self.fft_imag_out);

            // Calculate magnitudes

            for j in 0..half {
                newspec[j] =
                    (self.fft_real_out[j] * self.fft_real_out[j] + self.fft_imag_out[j] * self.fft_imag_out[j]).sqrt();
            }

            // Preserve peaks by applying adaptive threshold

            adaptive_threshold(&mut newspec);

            // Calculate JS divergence between new and old spectral frames

            if i > 0 {
                // otherwise we have no previous frame
                self.beat_sd.push(measure_spec_diff(&oldspec, &newspec));
            }

            // Copy newspec across to old

            oldspec.copy_from_slice(&newspec);

            i += 1;
        }

        // We now have all spectral difference measures in specdiff

        let mut timesig = self.bpb;
        if timesig == 0 {
            timesig = 4;
        }
        // The C++ throws from the vector constructor on a negative size. Nothing can come out of this.
        if timesig < 0 {
            return Ok(());
        }

        let mut dbcand = vec![0.0; timesig as usize]; // downbeat candidates

        // look for beat transition which leads to greatest spectral change
        for beat in 0..timesig {
            let mut count = 0;
            let mut example = beat - 1;
            while example < self.beat_sd.len() as i32 {
                if example >= 0 {
                    dbcand[beat as usize] += self.beat_sd[example as usize] / timesig as f64;
                    count += 1;
                }
                example += timesig;
            }
            if count > 0 {
                dbcand[beat as usize] /= count as f64;
            }
        }

        // first downbeat is beat at index of maximum value of dbcand
        let dbind = get_max(&dbcand) as i32;

        // remaining downbeats are at timesig intervals from the first
        let mut i = dbind;
        while i < beats.len() as i32 {
            downbeats.push(i);
            i += timesig;
        }

        Ok(())
    }

    /// One value for each beat passed to `find_down_beats`, less one.
    pub fn beat_sd(&self) -> &[f64] {
        &self.beat_sd
    }
}

pub fn measure_spec_diff(oldspec: &[f64], newspec: &[f64]) -> f64 {
    // JENSEN-SHANNON DIVERGENCE BETWEEN SPECTRAL FRAMES

    let mut specsize = 512; // ONLY LOOK AT FIRST 512 SAMPLES OF SPECTRUM.
    if specsize > oldspec.len() / 4 {
        specsize = oldspec.len() / 4;
    }

    let mut sd = 0.0;

    let mut sumnew = 0.0;
    let mut sumold = 0.0;

    // The C++ takes both spectra by value and changes its copies.
    let mut new_copy = [0.0; 512];
    let mut old_copy = [0.0; 512];

    for i in 0..specsize {
        new_copy[i] = newspec[i] + EPS;
        old_copy[i] = oldspec[i] + EPS;

        sumnew += new_copy[i];
        sumold += old_copy[i];
    }

    for i in 0..specsize {
        let mut newval = new_copy[i] / sumnew;
        let mut oldval = old_copy[i] / sumold;

        // IF ANY SPECTRAL VALUES ARE 0 (SHOULDN'T BE ANY!) SET THEM TO 1
        if newval == 0.0 {
            newval = 1.0;
        }

        if oldval == 0.0 {
            oldval = 1.0;
        }

        // JENSEN-SHANNON CALCULATION
        let sd1 = 0.5 * oldval + 0.5 * newval;
        sd = sd + (-sd1 * sd1.ln()) + (0.5 * (oldval * oldval.ln())) + (0.5 * (newval * newval.ln()));
    }

    sd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beat_frame_size_rules() {
        assert_eq!(DownBeat::new(44100.0, 16, 512).beat_frame_size(), 4096);
        assert_eq!(DownBeat::new(48000.0, 16, 557).beat_frame_size(), 4096);
        assert_eq!(DownBeat::new(22050.0, 8, 256).beat_frame_size(), 4096);
        assert_eq!(DownBeat::new(1.0, 1, 1).beat_frame_size(), 2);
    }

    #[test]
    fn identical_spectra_have_no_divergence() {
        let spec: Vec<f64> = (0..2048).map(|i| (i % 7) as f64).collect();
        assert!(measure_spec_diff(&spec, &spec).abs() < 1e-12);
    }

    #[test]
    fn silent_spectra_do_not_produce_nan() {
        let zero = vec![0.0; 2048];
        assert!(measure_spec_diff(&zero, &zero).is_finite());
    }

    #[test]
    fn negative_beats_per_bar_finds_nothing_instead_of_panicking() {
        let cancel = AtomicBool::new(false);
        let mut down_beat = DownBeat::new(44100.0, 16, 512);
        down_beat.set_beats_per_bar(-1);
        let mut downbeats = Vec::new();
        down_beat.find_down_beats(&[0.0; 1024], &[0.0, 10.0, 20.0], &mut downbeats, &cancel).unwrap();
        assert!(downbeats.is_empty());
    }

    #[test]
    fn empty_audio_returns_no_downbeats() {
        let cancel = AtomicBool::new(false);
        let mut down_beat = DownBeat::new(44100.0, 16, 512);
        let mut downbeats = Vec::new();
        down_beat.find_down_beats(&[], &[0.0, 10.0], &mut downbeats, &cancel).unwrap();
        assert!(downbeats.is_empty());
    }

    #[test]
    fn push_audio_block_appends_increment_over_factor_samples() {
        for (rate, factor, step, expected) in [
            (44100.0, 16, 512, 32),
            (48000.0, 16, 557, 34),
            (22050.0, 8, 256, 32),
            (96000.0, 32, 1114, 34),
            (5000.0, 1, 58, 58),
        ] {
            let mut down_beat = DownBeat::new(rate, factor, step);
            down_beat.push_audio_block(&vec![0.5; step * 2]);
            down_beat.push_audio_block(&vec![0.5; step * 2]);
            assert_eq!(down_beat.buffered_audio().len(), expected * 2, "rate {rate}");
        }
    }
}
