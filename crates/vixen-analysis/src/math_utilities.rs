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

//! Port of `maths/MathUtilities.cpp`: the functions the bar and beat tracker calls.

use std::f64::consts::PI;

/// `MathUtilities::mod`: floored, not a truncating `%`.
#[inline]
pub fn modulo(x: f64, y: f64) -> f64 {
    let a = (x / y).floor();
    x - (y * a)
}

#[inline]
pub fn princarg(ang: f64) -> f64 {
    modulo(ang + PI, -2.0 * PI) + PI
}

/// `mean(vector, start, count)`. The running sum is why `adaptive_threshold` can't use a sliding window:
/// a different summation order changes the rounding.
#[inline]
pub fn mean(src: &[f64], start: usize, count: usize) -> f64 {
    if count == 0 {
        return 0.0;
    }
    let mut sum = 0.0;
    for value in &src[start..start + count] {
        sum += *value;
    }
    sum / count as f64
}

pub fn adaptive_threshold(data: &mut [f64]) {
    let sz = data.len();
    if sz == 0 {
        return;
    }

    let mut smoothed = vec![0.0; sz];

    const P_PRE: usize = 8;
    const P_POST: usize = 7;

    for (i, smooth) in smoothed.iter_mut().enumerate() {
        let first = i.saturating_sub(P_PRE);
        let last = (sz - 1).min(i + P_POST);
        *smooth = mean(data, first, last - first + 1);
    }

    for (value, smooth) in data.iter_mut().zip(&smoothed) {
        *value -= *smooth;
        if *value < 0.0 {
            *value = 0.0;
        }
    }
}

/// `getMax(vector)`: the index of the first largest value. `data` must not be empty.
pub fn get_max(data: &[f64]) -> usize {
    let mut index = 0;
    let mut max = data[0];
    for (i, &value) in data.iter().enumerate() {
        if value > max {
            max = value;
            index = i;
        }
    }
    index
}

pub fn is_power_of_two(x: i32) -> bool {
    x >= 1 && x & (x - 1) == 0
}

pub fn next_power_of_two(mut x: i32) -> i32 {
    if is_power_of_two(x) {
        return x;
    }
    if x < 1 {
        return 1;
    }
    let mut n = 1;
    while x != 0 {
        x >>= 1;
        n <<= 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn princarg_maps_into_range() {
        assert_eq!(princarg(0.0), 0.0);
        assert!((princarg(3.0 * PI) - -PI).abs() < 1e-12 || (princarg(3.0 * PI) - PI).abs() < 1e-12);
        assert!((princarg(2.5 * PI) - 0.5 * PI).abs() < 1e-12);
        assert!((princarg(-2.5 * PI) - -0.5 * PI).abs() < 1e-12);
    }

    #[test]
    fn power_of_two_rules() {
        assert_eq!(next_power_of_two(1300), 2048);
        assert_eq!(next_power_of_two(2048), 2048);
        assert_eq!(next_power_of_two(14), 16);
        assert_eq!(next_power_of_two(7), 8);
        assert_eq!(next_power_of_two(1), 1);
        assert_eq!(next_power_of_two(0), 1);
        assert_eq!(next_power_of_two(-3), 1);
        assert_eq!(next_power_of_two(3583), 4096);
    }

    #[test]
    fn adaptive_threshold_clips_below_the_moving_mean() {
        let mut data = vec![1.0, 1.0, 1.0, 9.0, 1.0, 1.0, 1.0, 1.0];
        adaptive_threshold(&mut data);
        assert_eq!(data[0], 0.0);
        assert!((data[3] - 7.0).abs() < 1e-12);
        assert_eq!(data[7], 0.0);
    }

    #[test]
    fn get_max_returns_the_first_largest() {
        assert_eq!(get_max(&[0.0, 0.0, 0.0]), 0);
        assert_eq!(get_max(&[1.0, 3.0, 3.0, 2.0]), 1);
    }
}
