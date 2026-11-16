/*
    QM DSP Library

    Centre for Digital Music, Queen Mary, University of London.
    This file 2005-2006 Christian Landone, copyright 2013 QMUL.

    This program is free software; you can redistribute it and/or
    modify it under the terms of the GNU General Public License as
    published by the Free Software Foundation; either version 2 of the
    License, or (at your option) any later version.  See the file
    COPYING included with this distribution for more information.
*/

//! Port of `dsp/phasevocoder/PhaseVocoder.cpp`. The unwrapped phases are left out: complex spectral difference
//! never reads them and `unwrapPhases` touches no other state.

use crate::fft::FftReal;

pub struct PhaseVocoder {
    n: usize,
    fft: FftReal,
    time: Vec<f64>,
    real: Vec<f64>,
    imag: Vec<f64>,
}

impl PhaseVocoder {
    /// `None` for an odd size, as for [`FftReal::new`].
    pub fn new(n: usize) -> Option<PhaseVocoder> {
        Some(PhaseVocoder { n, fft: FftReal::new(n)?, time: vec![0.0; n], real: vec![0.0; n], imag: vec![0.0; n] })
    }

    fn fft_shift(src: &mut [f64], n: usize) {
        let hs = n / 2;
        for i in 0..hs {
            src.swap(i, i + hs);
        }
    }

    /// `mag` and `theta` receive n/2 + 1 values.
    pub fn process_time_domain(&mut self, src: &[f64], mag: &mut [f64], theta: &mut [f64]) {
        self.time[..self.n].copy_from_slice(&src[..self.n]);
        Self::fft_shift(&mut self.time, self.n);
        self.fft.forward(&self.time, &mut self.real, &mut self.imag);

        for i in 0..self.n / 2 + 1 {
            mag[i] = (self.real[i] * self.real[i] + self.imag[i] * self.imag[i]).sqrt();
        }
        for i in 0..self.n / 2 + 1 {
            theta[i] = self.imag[i].atan2(self.real[i]);
        }
    }
}
