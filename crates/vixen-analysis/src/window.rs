/*
    QM DSP library
    Centre for Digital Music, Queen Mary, University of London.
    This file Copyright 2006 Chris Cannam.

    This program is free software; you can redistribute it and/or
    modify it under the terms of the GNU General Public License as
    published by the Free Software Foundation; either version 2 of the
    License, or (at your option) any later version.  See the file
    COPYING included with this distribution for more information.
*/

//! Port of `base/Window.h`: the Hanning window, the one type the tracker builds.

use std::f64::consts::PI;

/// `Window<double>(HanningWindow, n)`. Periodic rather than symmetrical, by design upstream.
pub struct HanningWindow {
    cache: Vec<f64>,
}

impl HanningWindow {
    pub fn new(n: usize) -> HanningWindow {
        let cache =
            (0..n).map(|i| if n > 1 { 0.50 - 0.50 * (2.0 * PI * i as f64 / n as f64).cos() } else { 1.0 }).collect();
        HanningWindow { cache }
    }

    pub fn cut(&self, src: &[f64], dst: &mut [f64]) {
        for ((out, input), mult) in dst.iter_mut().zip(src).zip(&self.cache) {
            *out = *input * *mult;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hanning_is_periodic() {
        let window = HanningWindow::new(8);
        let src = [1.0; 8];
        let mut out = [0.0; 8];
        window.cut(&src, &mut out);
        assert_eq!(out[0], 0.0);
        assert!((out[4] - 1.0).abs() < 1e-15);
        assert!((out[1] - out[7]).abs() < 1e-15);
    }

    #[test]
    fn a_single_point_window_stays_at_one() {
        let window = HanningWindow::new(1);
        let mut out = [0.0];
        window.cut(&[5.0], &mut out);
        assert_eq!(out[0], 5.0);
    }
}
