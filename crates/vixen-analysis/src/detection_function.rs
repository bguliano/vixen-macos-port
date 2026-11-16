/*
    QM DSP Library

    Centre for Digital Music, Queen Mary, University of London.
    This file 2005-2006 Christian Landone.

    This program is free software; you can redistribute it and/or
    modify it under the terms of the GNU General Public License as
    published by the Free Software Foundation; either version 2 of the
    License, or (at your option) any later version.  See the file
    COPYING included with this distribution for more information.
*/

//! Port of `dsp/onsets/DetectionFunction.cpp` for the configuration `BarBeatTracker::initialise` always uses:
//! `DF_COMPLEXSD` with adaptive whitening off.

use crate::math_utilities::princarg;
use crate::phase_vocoder::PhaseVocoder;
use crate::window::HanningWindow;

/// The `DFConfig` fields that matter to [`DetectionFunction`].
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DfConfig {
    pub step_size: i32,
    pub frame_length: i32,
}

pub struct DetectionFunction {
    data_length: usize,
    half_length: usize,
    mag_history: Vec<f64>,
    phase_history: Vec<f64>,
    phase_history_old: Vec<f64>,
    windowed: Vec<f64>,
    magnitude: Vec<f64>,
    theta_angle: Vec<f64>,
    window: HanningWindow,
    phase_voc: PhaseVocoder,
}

impl DetectionFunction {
    /// `None` when `frame_length` is odd, where the C++ throws from the FFT constructor.
    pub fn new(config: DfConfig) -> Option<DetectionFunction> {
        let data_length = usize::try_from(config.frame_length).ok()?;
        let half_length = data_length / 2 + 1;
        Some(DetectionFunction {
            data_length,
            half_length,
            mag_history: vec![0.0; half_length],
            phase_history: vec![0.0; half_length],
            phase_history_old: vec![0.0; half_length],
            windowed: vec![0.0; data_length],
            magnitude: vec![0.0; half_length],
            theta_angle: vec![0.0; half_length],
            window: HanningWindow::new(data_length),
            phase_voc: PhaseVocoder::new(data_length)?,
        })
    }

    pub fn process_time_domain(&mut self, samples: &[f64]) -> f64 {
        self.window.cut(&samples[..self.data_length], &mut self.windowed);
        self.phase_voc.process_time_domain(&self.windowed, &mut self.magnitude, &mut self.theta_angle);
        self.complex_sd()
    }

    fn complex_sd(&mut self) -> f64 {
        let mut val = 0.0;

        for i in 0..self.half_length {
            let tmp_phase = self.theta_angle[i] - 2.0 * self.phase_history[i] + self.phase_history_old[i];
            let dev = princarg(tmp_phase);

            // meas = magHistory - (magnitude * exp(j * dev)), written out: exp(j * dev) is (cos, sin) because
            // exp(0) is exactly 1, and the C++ only ever reads the squared parts, so the sign of a zero is lost.
            let (sin, cos) = (dev.sin(), dev.cos());
            let meas_real = self.mag_history[i] - self.magnitude[i] * cos;
            let meas_imag = -(self.magnitude[i] * sin);

            val += (meas_real * meas_real + meas_imag * meas_imag).sqrt();

            self.phase_history_old[i] = self.phase_history[i];
            self.phase_history[i] = self.theta_angle[i];
            self.mag_history[i] = self.magnitude[i];
        }

        val
    }
}
