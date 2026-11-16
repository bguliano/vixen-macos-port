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

//! Port of `dsp/rateconversion/Decimator.cpp`, the `float` path.

// The filter coefficients are copied digit for digit from the C++. They parse to the same doubles.
#![allow(clippy::excessive_precision)]

pub struct Decimator {
    input_length: usize,
    output_length: usize,
    dec_factor: usize,
    o: [f64; 7],
    a: [f64; 8],
    b: [f64; 8],
    dec_buffer: Vec<f64>,
}

impl Decimator {
    pub const HIGHEST_SUPPORTED_FACTOR: usize = 8;

    pub fn new(in_length: usize, dec_factor: usize) -> Decimator {
        let (b, a) = match dec_factor {
            8 => (
                [
                    0.060111378492136,
                    -0.257323420830598,
                    0.420583503165928,
                    -0.222750785197418,
                    -0.222750785197418,
                    0.420583503165928,
                    -0.257323420830598,
                    0.060111378492136,
                ],
                [
                    1.0,
                    -5.667654878577432,
                    14.062452278088417,
                    -19.737303840697738,
                    16.889698874608641,
                    -8.796600612325928,
                    2.577553446979888,
                    -0.326903916815751,
                ],
            ),
            4 => (
                [
                    0.10133306904918619,
                    -0.2447523353702363,
                    0.33622528590120965,
                    -0.13936581560633518,
                    -0.13936581560633382,
                    0.3362252859012087,
                    -0.2447523353702358,
                    0.10133306904918594,
                ],
                [
                    1.0,
                    -3.9035590278139427,
                    7.5299379980621133,
                    -8.6890803793177511,
                    6.4578667096099176,
                    -3.0242979431223631,
                    0.83043385136748382,
                    -0.094420800837809335,
                ],
            ),
            2 => (
                [
                    0.20898944260075727,
                    0.40011234879814367,
                    0.819741973072733,
                    1.0087419911682323,
                    1.0087419911682325,
                    0.81974197307273156,
                    0.40011234879814295,
                    0.20898944260075661,
                ],
                [
                    1.0,
                    0.0077331184208358217,
                    1.9853971155964376,
                    0.19296739275341004,
                    1.2330748872852182,
                    0.18705341389316466,
                    0.23659265908013868,
                    0.032352924250533946,
                ],
            ),
            // The C++ warns on stderr for anything but 1 here and uses no antialiasing filter.
            _ => ([1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        };

        Decimator {
            input_length: in_length,
            output_length: in_length / dec_factor,
            dec_factor,
            o: [0.0; 7],
            a,
            b,
            dec_buffer: vec![0.0; in_length],
        }
    }

    pub fn output_length(&self) -> usize {
        self.output_length
    }

    fn do_anti_alias(&mut self, src: &[f32]) {
        let (a, b) = (&self.a, &self.b);
        let o = &mut self.o;
        for (dst, value) in self.dec_buffer.iter_mut().zip(src) {
            let input = *value as f64;

            let output = input * b[0] + o[0];

            o[0] = input * b[1] - output * a[1] + o[1];
            o[1] = input * b[2] - output * a[2] + o[2];
            o[2] = input * b[3] - output * a[3] + o[3];
            o[3] = input * b[4] - output * a[4] + o[4];
            o[4] = input * b[5] - output * a[5] + o[5];
            o[5] = input * b[6] - output * a[6] + o[6];
            o[6] = input * b[7] - output * a[7];

            *dst = output;
        }
    }

    pub fn process(&mut self, src: &[f32], dst: &mut [f32]) {
        if self.dec_factor == 1 {
            dst[..self.output_length].copy_from_slice(&src[..self.output_length]);
            return;
        }

        self.do_anti_alias(&src[..self.input_length]);

        for (i, out) in dst[..self.output_length].iter_mut().enumerate() {
            *out = self.dec_buffer[self.dec_factor * i] as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_factor_picks_every_nth_sample_without_filtering() {
        let src: Vec<f32> = (0..32).map(|i| i as f32).collect();
        let mut dst = vec![0.0; 2];
        Decimator::new(32, 16).process(&src, &mut dst);
        assert_eq!(dst, vec![0.0, 16.0]);
    }

    #[test]
    fn factor_one_copies() {
        let mut dst = vec![0.0; 3];
        Decimator::new(3, 1).process(&[1.0, 2.0, 3.0], &mut dst);
        assert_eq!(dst, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn filter_state_carries_between_blocks() {
        let src = vec![1.0f32; 16];
        let mut decimator = Decimator::new(16, 8);
        let (mut first, mut second) = (vec![0.0; 2], vec![0.0; 2]);
        decimator.process(&src, &mut first);
        decimator.process(&src, &mut second);
        assert_ne!(first, second);
    }
}
