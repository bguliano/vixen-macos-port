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

//! Port of `dsp/tempotracking/TempoTrackV2.cpp`.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::math_utilities::adaptive_threshold;
use crate::Cancelled;

// TempoTrackV2.cpp defines its own EPS; DownBeat.cpp gets a different one from MathAliases.h.
const EPS: f64 = 0.0000008;

const WV_LEN: usize = 128;

pub struct TempoTrackV2 {
    rate: f32,
    increment: i32,
}

fn get_max_val(df: &[f64]) -> f64 {
    let mut maxval = 0.0;
    for &value in df {
        if maxval < value {
            maxval = value;
        }
    }
    maxval
}

fn get_max_ind(df: &[f64]) -> usize {
    let mut maxval = 0.0;
    let mut ind = 0;
    for (i, &value) in df.iter().enumerate() {
        if maxval < value {
            maxval = value;
            ind = i;
        }
    }
    ind
}

#[inline]
fn check(cancel: &AtomicBool) -> Result<(), Cancelled> {
    if cancel.load(Ordering::Relaxed) {
        Err(Cancelled)
    } else {
        Ok(())
    }
}

impl TempoTrackV2 {
    pub fn new(sample_rate: f32, df_increment: i32) -> TempoTrackV2 {
        TempoTrackV2 { rate: sample_rate, increment: df_increment }
    }

    /// `beat_period` must already hold one zero per `df` value; values are in df increment units.
    pub fn calculate_beat_period(
        &self,
        df: &[f64],
        beat_period: &mut [f64],
        tempi: &mut Vec<f64>,
        inputtempo: f64,
        constraintempo: bool,
        cancel: &AtomicBool,
    ) -> Result<(), Cancelled> {
        // The integer division is upstream's: "60*44100/512 is a magic number", and it truncates before the divide.
        let rayparam = (60 * 44100 / 512) as f64 / inputtempo;

        let mut wv = [0.0; WV_LEN];

        if constraintempo {
            for (i, weight) in wv.iter_mut().enumerate() {
                let d = i as f64 - rayparam;
                let spread = rayparam / 4.0;
                *weight = ((-(d * d)) / (2.0 * (spread * spread))).exp();
            }
        } else {
            for (i, weight) in wv.iter_mut().enumerate() {
                let d = i as f64;
                *weight = (d / (rayparam * rayparam)) * ((-(d * d)) / (2.0 * (rayparam * rayparam))).exp();
            }
        }

        // beat tracking frame size (roughly 6 seconds) and hop (1.5 seconds)
        const WINLEN: usize = 512;
        const STEP: usize = 128;

        let mut rcfmat: Vec<[f64; WV_LEN]> = Vec::new();

        let mut i = 0;
        while i + WINLEN < df.len() {
            check(cancel)?;
            let mut rcf = [0.0; WV_LEN];
            Self::get_rcf(&df[i..i + WINLEN], &wv, &mut rcf);
            rcfmat.push(rcf);
            i += STEP;
        }

        self.viterbi_decode(&rcfmat, &wv, beat_period, tempi, cancel)
    }

    pub fn get_rcf(dfframe_in: &[f64], wv: &[f64; WV_LEN], rcf: &mut [f64; WV_LEN]) {
        let mut dfframe = dfframe_in.to_vec();
        adaptive_threshold(&mut dfframe);

        let dfframe_len = dfframe.len();
        let mut acf = vec![0.0; dfframe_len];

        for (lag, value) in acf.iter_mut().enumerate() {
            let mut sum = 0.0;
            for n in 0..dfframe_len - lag {
                let tmp = dfframe[n] * dfframe[n + lag];
                sum += tmp;
            }
            *value = sum / (dfframe_len - lag) as f64;
        }

        const NUMELEM: i32 = 4;

        for i in 2..WV_LEN {
            for a in 1..=NUMELEM {
                for b in 1 - a..=a - 1 {
                    rcf[i - 1] += (acf[(a * i as i32 + b - 1) as usize] * wv[i - 1]) / (2.0 * a as f64 - 1.0);
                }
            }
        }

        adaptive_threshold(rcf);

        let mut rcfsum = 0.0;
        for value in rcf.iter_mut() {
            *value += EPS;
            rcfsum += *value;
        }

        for value in rcf.iter_mut() {
            *value /= rcfsum + EPS;
        }
    }

    pub fn viterbi_decode(
        &self,
        rcfmat: &[[f64; WV_LEN]],
        wv: &[f64; WV_LEN],
        beat_period: &mut [f64],
        tempi: &mut Vec<f64>,
        cancel: &AtomicBool,
    ) -> Result<(), Cancelled> {
        let wv_len = WV_LEN;

        let mut tmat = vec![[0.0; WV_LEN]; WV_LEN];

        // variance of Gaussians in transition matrix
        // formed of Gaussians on diagonal - implies slow tempo change
        let sigma = 8.0f64;
        // don't want really short beat periods, or really long ones
        for (i, row) in tmat.iter_mut().enumerate().take(wv_len - 20).skip(20) {
            for (j, value) in row.iter_mut().enumerate().take(wv_len - 20).skip(20) {
                let mu = i as f64;
                let d = j as f64 - mu;
                *value = ((-(d * d)) / (2.0 * (sigma * sigma))).exp();
            }
        }

        let t_len = rcfmat.len();

        if t_len < 2 {
            return Ok(()); // can't do anything at all meaningful
        }

        // calculate_beat_period's caller always has room. The C++ would write past the end otherwise.
        if beat_period.len() < t_len * 128 {
            return Ok(());
        }

        let q = WV_LEN;

        let mut delta = vec![[0.0; WV_LEN]; t_len];
        let mut psi = vec![[0usize; WV_LEN]; t_len];

        for j in 0..q {
            delta[0][j] = wv[j] * rcfmat[0][j];
            psi[0][j] = 0;
        }

        let mut deltasum = 0.0;
        for value in &delta[0] {
            deltasum += *value;
        }
        for value in delta[0].iter_mut() {
            *value /= deltasum + EPS;
        }

        let mut tmp_vec = [0.0; WV_LEN];

        for t in 1..t_len {
            check(cancel)?;

            for j in 0..q {
                for i in 0..q {
                    tmp_vec[i] = delta[t - 1][i] * tmat[j][i];
                }

                delta[t][j] = get_max_val(&tmp_vec);

                psi[t][j] = get_max_ind(&tmp_vec);

                delta[t][j] *= rcfmat[t][j];
            }

            let mut deltasum = 0.0;
            for value in &delta[t] {
                deltasum += *value;
            }
            for value in delta[t].iter_mut() {
                *value /= deltasum + EPS;
            }
        }

        let mut bestpath = vec![0usize; t_len];

        // find starting point - best beat period for "last" frame
        bestpath[t_len - 1] = get_max_ind(&delta[t_len - 1]);

        // backtrace through index of maximum values in psi
        let mut t = t_len as i32 - 2;
        while t > 0 {
            let u = t as usize;
            bestpath[u] = psi[u + 1][bestpath[u + 1]];
            t -= 1;
        }

        // weird but necessary hack -- couldn't get above loop to terminate at t >= 0
        bestpath[0] = psi[1][bestpath[1]];

        let mut lastind = 0;
        for (i, &best) in bestpath.iter().enumerate() {
            const STEP: usize = 128;
            for j in 0..STEP {
                lastind = i * STEP + j;
                beat_period[lastind] = best as f64;
            }
        }

        // fill in the last values...
        let last = beat_period[lastind];
        for value in beat_period[lastind..].iter_mut() {
            *value = last;
        }

        for value in beat_period.iter() {
            tempi.push((60.0 * self.rate as f64 / self.increment as f64) / *value);
        }

        Ok(())
    }

    /// Beat positions are in df increment units.
    pub fn calculate_beats(
        &self,
        df: &[f64],
        beat_period: &[f64],
        beats: &mut Vec<f64>,
        alpha: f64,
        tightness: f64,
        cancel: &AtomicBool,
    ) -> Result<(), Cancelled> {
        // The length check is new: the C++ reads past the end when the period is the shorter of the two.
        if df.is_empty() || beat_period.is_empty() || beat_period.len() < df.len() {
            return Ok(());
        }

        let df_len = df.len();

        let mut cumscore = vec![0.0; df_len]; // store cumulative score
        let mut backlink = vec![-1i32; df_len]; // backlink (stores best beat locations at each time instant)

        let mut txwt: Vec<f64> = Vec::new();
        let mut scorecands: Vec<f64> = Vec::new();
        // beat_period holds long runs of one value, and txwt depends on nothing else.
        let mut txwt_for = None;

        for i in 0..df_len {
            if i % 4096 == 0 {
                check(cancel)?;
            }

            let mu = beat_period[i];
            let prange_min = (-2.0 * mu) as i32;
            // C round(), half away from zero, like the other round() below.
            let prange_max = (-0.5 * mu).round() as i32;

            // transition range
            // Zero only for a negative period, which the tempo search can't produce.
            let txwt_len = (prange_max - prange_min + 1).max(0) as usize;

            if txwt_for != Some(mu.to_bits()) {
                txwt.clear();
                for j in 0..txwt_len {
                    let v = tightness * (((2.0 * mu).round() - j as f64) / mu).ln();
                    txwt.push((-0.5 * (v * v)).exp());
                }
                txwt_for = Some(mu.to_bits());
            }

            scorecands.clear();
            scorecands.resize(txwt_len, 0.0);

            for j in 0..txwt_len {
                // IF IN THE ALLOWED RANGE, THEN LOOK AT CUMSCORE[I+PRANGE_MIN+J
                // ELSE LEAVE AT DEFAULT VALUE FROM INITIALISATION:  D_VEC_T SCORECANDS (TXWT.SIZE());
                let cscore_ind = i as i32 + prange_min + j as i32;
                if cscore_ind >= 0 {
                    scorecands[j] = txwt[j] * cumscore[cscore_ind as usize];
                }
            }

            // find max value and index of maximum value
            let vv = get_max_val(&scorecands);
            let xx = get_max_ind(&scorecands);

            cumscore[i] = alpha * vv + (1.0 - alpha) * df[i];
            backlink[i] = i as i32 + prange_min + xx as i32;
        }

        // STARTING POINT, I.E. LAST BEAT.. PICK A STRONG POINT IN cumscore VECTOR
        let last_period = beat_period[beat_period.len() - 1];
        let tail_start = (df_len as f64 - last_period) as i32;
        // Negative is out of bounds in the C++. A period only becomes non-zero with more than 640 df values, so this
        // takes a caller-supplied period longer than the track.
        let tmp_vec = cumscore.get(tail_start.max(0) as usize..).unwrap_or(&[]);

        let mut startpoint = ((get_max_ind(tmp_vec) as i32 + df_len as i32) as f64 - last_period) as i32;

        // can happen if no results obtained earlier (e.g. input too short)
        if startpoint >= backlink.len() as i32 {
            startpoint = backlink.len() as i32 - 1;
        }
        startpoint = startpoint.max(0);

        // USE BACKLINK TO GET EACH NEW BEAT (TOWARDS THE BEGINNING OF THE FILE)
        //  BACKTRACKING FROM THE END TO THE BEGINNING.. MAKING SURE NOT TO GO BEFORE SAMPLE 0
        let mut ibeats = vec![startpoint];
        loop {
            let b = ibeats[ibeats.len() - 1];
            let link = backlink[b as usize];
            // The C++ tests for equality ("shouldn't happen... haha"). A later frame takes a negative period, which the
            // tempo search can't produce, and would run off the end.
            if link <= 0 || link >= b {
                break;
            }
            ibeats.push(link);
        }

        // REVERSE SEQUENCE OF IBEATS AND STORE AS BEATS
        for &beat in ibeats.iter().rev() {
            beats.push(beat as f64);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_max_treats_nan_as_not_greater() {
        assert_eq!(get_max_ind(&[f64::NAN, 1.0, f64::NAN]), 1);
        assert_eq!(get_max_ind(&[f64::NAN]), 0);
        assert_eq!(get_max_val(&[f64::NAN]), 0.0);
        assert_eq!(get_max_ind(&[]), 0);
    }

    #[test]
    fn too_short_for_two_frames_leaves_the_period_at_zero_and_picks_the_last_frame() {
        let cancel = AtomicBool::new(false);
        let tt = TempoTrackV2::new(44100.0, 512);
        let df = vec![1.0; 100];
        let mut period = vec![0.0; 100];
        let mut tempi = Vec::new();
        tt.calculate_beat_period(&df, &mut period, &mut tempi, 120.0, false, &cancel).unwrap();
        assert!(period.iter().all(|&p| p == 0.0));
        assert!(tempi.is_empty());
        let mut beats = Vec::new();
        tt.calculate_beats(&df, &period, &mut beats, 0.9, 4.0, &cancel).unwrap();
        assert_eq!(beats, vec![99.0]);
    }

    #[test]
    fn mismatched_or_odd_periods_do_not_panic() {
        let cancel = AtomicBool::new(false);
        let tt = TempoTrackV2::new(44100.0, 512);
        let df = vec![1.0; 100];
        let mut beats = Vec::new();

        tt.calculate_beats(&df, &[43.0; 10], &mut beats, 0.9, 4.0, &cancel).unwrap();
        assert!(beats.is_empty());

        tt.calculate_beats(&df, &[5000.0; 100], &mut beats, 0.9, 4.0, &cancel).unwrap();
        assert!(!beats.is_empty());

        beats.clear();
        tt.calculate_beats(&df, &[-7.0; 100], &mut beats, 0.9, 4.0, &cancel).unwrap();
        tt.calculate_beats(&df, &[f64::NAN; 100], &mut beats, 0.9, 4.0, &cancel).unwrap();

        let mut tempi = Vec::new();
        let long_df = vec![1.0; 1000];
        let mut short_period = vec![0.0; 10];
        tt.calculate_beat_period(&long_df, &mut short_period, &mut tempi, 120.0, false, &cancel).unwrap();
        assert!(short_period.iter().all(|&p| p == 0.0));
    }

    #[test]
    fn a_set_flag_stops_the_period_search() {
        let cancel = AtomicBool::new(true);
        let tt = TempoTrackV2::new(44100.0, 512);
        let df = vec![1.0; 1000];
        let mut period = vec![0.0; 1000];
        let mut tempi = Vec::new();
        assert_eq!(tt.calculate_beat_period(&df, &mut period, &mut tempi, 120.0, false, &cancel), Err(Cancelled));
    }
}
