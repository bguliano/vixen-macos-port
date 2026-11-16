/*
    QM DSP Library

    Centre for Digital Music, Queen Mary, University of London.

    This program is free software; you can redistribute it and/or
    modify it under the terms of the GNU General Public License as
    published by the Free Software Foundation; either version 2 of the
    License, or (at your option) any later version.  See the file
    COPYING included with this distribution for more information.
*/

//! Port of `dsp/transforms/FFT.cpp`, the forward `FFTReal` path.

use crate::kissfft::{KissFftCpx, KissFftr};

pub struct FftReal {
    n: usize,
    planf: KissFftr,
    c: Vec<KissFftCpx>,
}

impl FftReal {
    /// `None` for an odd size, where the C++ constructor throws `std::invalid_argument`, and for zero.
    pub fn new(n: usize) -> Option<FftReal> {
        if n == 0 || !n.is_multiple_of(2) {
            return None;
        }
        Some(FftReal { n, planf: KissFftr::new(n), c: vec![KissFftCpx::default(); n / 2 + 1] })
    }

    /// `ri`, `ro` and `io` each hold n values; the conjugate half of the output is filled in too.
    pub fn forward(&mut self, ri: &[f64], ro: &mut [f64], io: &mut [f64]) {
        self.planf.process(ri, &mut self.c);

        for i in 0..=self.n / 2 {
            ro[i] = self.c[i].r;
            io[i] = self.c[i].i;
        }

        let mut i = 0;
        while i + 1 < self.n / 2 {
            ro[self.n - i - 1] = ro[i + 1];
            io[self.n - i - 1] = -io[i + 1];
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odd_sizes_are_refused() {
        assert!(FftReal::new(7).is_none());
        assert!(FftReal::new(0).is_none());
        assert!(FftReal::new(8).is_some());
    }

    #[test]
    fn conjugate_half_mirrors_the_first_half() {
        let n = 16;
        let input: Vec<f64> = (0..n).map(|i| (i as f64 * 0.7).sin() + 0.25).collect();
        let (mut re, mut im) = (vec![0.0; n], vec![0.0; n]);
        FftReal::new(n).unwrap().forward(&input, &mut re, &mut im);
        assert_eq!(im[0], 0.0);
        assert_eq!(im[n / 2], 0.0);
        for k in 1..n / 2 {
            assert_eq!(re[n - k], re[k]);
            assert_eq!(im[n - k], -im[k]);
        }
    }
}
