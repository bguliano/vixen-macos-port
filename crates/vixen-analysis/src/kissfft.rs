/*
Copyright (c) 2003-2010, Mark Borgerding

All rights reserved.

Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:

    * Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.
    * Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.
    * Neither the author nor the names of any contributors may be used to endorse or promote products derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
*/

//! Port of `ext/kissfft/kiss_fft.c` and `tools/kiss_fftr.c` with `kiss_fft_scalar=double`: the forward transforms
//! QM DSP reaches. The order of every floating-point operation is kept, because it decides the result bits.

const MAXFACTORS: usize = 32;

#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct KissFftCpx {
    pub r: f64,
    pub i: f64,
}

#[inline(always)]
fn c_mul(a: KissFftCpx, b: KissFftCpx) -> KissFftCpx {
    KissFftCpx { r: a.r * b.r - a.i * b.i, i: a.r * b.i + a.i * b.r }
}

#[inline(always)]
fn c_add(a: KissFftCpx, b: KissFftCpx) -> KissFftCpx {
    KissFftCpx { r: a.r + b.r, i: a.i + b.i }
}

#[inline(always)]
fn c_sub(a: KissFftCpx, b: KissFftCpx) -> KissFftCpx {
    KissFftCpx { r: a.r - b.r, i: a.i - b.i }
}

#[inline(always)]
fn kf_cexp(phase: f64) -> KissFftCpx {
    KissFftCpx { r: phase.cos(), i: phase.sin() }
}

fn kf_bfly2(fout: &mut [KissFftCpx], fstride: usize, twiddles: &[KissFftCpx], m: usize) {
    let mut tw1 = 0;
    for k in 0..m {
        let t = c_mul(fout[k + m], twiddles[tw1]);
        tw1 += fstride;
        fout[k + m] = c_sub(fout[k], t);
        fout[k] = c_add(fout[k], t);
    }
}

fn kf_bfly4(fout: &mut [KissFftCpx], fstride: usize, twiddles: &[KissFftCpx], m: usize) {
    let m2 = 2 * m;
    let m3 = 3 * m;
    let (mut tw1, mut tw2, mut tw3) = (0, 0, 0);

    for k in 0..m {
        let scratch0 = c_mul(fout[k + m], twiddles[tw1]);
        let scratch1 = c_mul(fout[k + m2], twiddles[tw2]);
        let scratch2 = c_mul(fout[k + m3], twiddles[tw3]);

        let scratch5 = c_sub(fout[k], scratch1);
        fout[k] = c_add(fout[k], scratch1);
        let scratch3 = c_add(scratch0, scratch2);
        let scratch4 = c_sub(scratch0, scratch2);
        fout[k + m2] = c_sub(fout[k], scratch3);
        tw1 += fstride;
        tw2 += fstride * 2;
        tw3 += fstride * 3;
        fout[k] = c_add(fout[k], scratch3);

        fout[k + m] = KissFftCpx { r: scratch5.r + scratch4.i, i: scratch5.i - scratch4.r };
        fout[k + m3] = KissFftCpx { r: scratch5.r - scratch4.i, i: scratch5.i + scratch4.r };
    }
}

fn kf_bfly3(fout: &mut [KissFftCpx], fstride: usize, twiddles: &[KissFftCpx], m: usize) {
    let m2 = 2 * m;
    let epi3 = twiddles[fstride * m];
    let (mut tw1, mut tw2) = (0, 0);

    for k in 0..m {
        let scratch1 = c_mul(fout[k + m], twiddles[tw1]);
        let scratch2 = c_mul(fout[k + m2], twiddles[tw2]);

        let scratch3 = c_add(scratch1, scratch2);
        let mut scratch0 = c_sub(scratch1, scratch2);
        tw1 += fstride;
        tw2 += fstride * 2;

        fout[k + m].r = fout[k].r - scratch3.r * 0.5;
        fout[k + m].i = fout[k].i - scratch3.i * 0.5;

        scratch0.r *= epi3.i;
        scratch0.i *= epi3.i;

        fout[k] = c_add(fout[k], scratch3);

        fout[k + m2].r = fout[k + m].r + scratch0.i;
        fout[k + m2].i = fout[k + m].i - scratch0.r;

        fout[k + m].r -= scratch0.i;
        fout[k + m].i += scratch0.r;
    }
}

fn kf_bfly5(fout: &mut [KissFftCpx], fstride: usize, twiddles: &[KissFftCpx], m: usize) {
    let ya = twiddles[fstride * m];
    let yb = twiddles[fstride * 2 * m];

    for u in 0..m {
        let (i0, i1, i2, i3, i4) = (u, u + m, u + 2 * m, u + 3 * m, u + 4 * m);
        let scratch0 = fout[i0];

        let scratch1 = c_mul(fout[i1], twiddles[u * fstride]);
        let scratch2 = c_mul(fout[i2], twiddles[2 * u * fstride]);
        let scratch3 = c_mul(fout[i3], twiddles[3 * u * fstride]);
        let scratch4 = c_mul(fout[i4], twiddles[4 * u * fstride]);

        let scratch7 = c_add(scratch1, scratch4);
        let scratch10 = c_sub(scratch1, scratch4);
        let scratch8 = c_add(scratch2, scratch3);
        let scratch9 = c_sub(scratch2, scratch3);

        fout[i0].r += scratch7.r + scratch8.r;
        fout[i0].i += scratch7.i + scratch8.i;

        let scratch5 = KissFftCpx {
            r: scratch0.r + scratch7.r * ya.r + scratch8.r * yb.r,
            i: scratch0.i + scratch7.i * ya.r + scratch8.i * yb.r,
        };
        let scratch6 =
            KissFftCpx { r: scratch10.i * ya.i + scratch9.i * yb.i, i: -(scratch10.r * ya.i) - scratch9.r * yb.i };

        fout[i1] = c_sub(scratch5, scratch6);
        fout[i4] = c_add(scratch5, scratch6);

        let scratch11 = KissFftCpx {
            r: scratch0.r + scratch7.r * yb.r + scratch8.r * ya.r,
            i: scratch0.i + scratch7.i * yb.r + scratch8.i * ya.r,
        };
        let scratch12 =
            KissFftCpx { r: -(scratch10.i * yb.i) + scratch9.i * ya.i, i: scratch10.r * yb.i - scratch9.r * ya.i };

        fout[i2] = c_add(scratch11, scratch12);
        fout[i3] = c_sub(scratch11, scratch12);
    }
}

fn kf_bfly_generic(
    fout: &mut [KissFftCpx],
    fstride: usize,
    twiddles: &[KissFftCpx],
    scratch: &mut [KissFftCpx],
    m: usize,
    p: usize,
) {
    let norig = twiddles.len();

    for u in 0..m {
        let mut k = u;
        for slot in scratch[..p].iter_mut() {
            *slot = fout[k];
            k += m;
        }

        k = u;
        for _ in 0..p {
            let mut twidx = 0;
            let mut sum = scratch[0];
            for &value in &scratch[1..p] {
                twidx += fstride * k;
                if twidx >= norig {
                    twidx -= norig;
                }
                let t = c_mul(value, twiddles[twidx]);
                sum = c_add(sum, t);
            }
            fout[k] = sum;
            k += m;
        }
    }
}

/// `facbuf` holds p1,m1,p2,m2,... where p[i] * m[i] = m[i-1] and m0 = n.
fn kf_factor(mut n: i32) -> Vec<(usize, usize)> {
    let mut factors = Vec::with_capacity(MAXFACTORS);
    let mut p = 4;
    let floor_sqrt = (n as f64).sqrt().floor();

    loop {
        while n % p != 0 {
            match p {
                4 => p = 2,
                2 => p = 3,
                _ => p += 2,
            }
            if p as f64 > floor_sqrt {
                p = n;
            }
        }
        n /= p;
        factors.push((p as usize, n as usize));
        if n <= 1 {
            break;
        }
    }
    factors
}

/// A forward `kiss_fft_cfg`.
pub struct KissFft {
    factors: Vec<(usize, usize)>,
    twiddles: Vec<KissFftCpx>,
    // kf_bfly_generic mallocs this on every call; the contents never outlive it, so one buffer does.
    scratch: Vec<KissFftCpx>,
}

impl KissFft {
    pub fn new(nfft: usize) -> KissFft {
        let pi = std::f64::consts::PI;
        let twiddles = (0..nfft).map(|i| kf_cexp(-2.0 * pi * i as f64 / nfft as f64)).collect();
        let factors = kf_factor(nfft as i32);
        let largest = factors.iter().map(|&(p, _)| p).max().unwrap_or(1);
        KissFft { factors, twiddles, scratch: vec![KissFftCpx::default(); largest] }
    }

    pub fn nfft(&self) -> usize {
        self.twiddles.len()
    }

    /// `kiss_fft` where `fin` is `kiss_fft_cpx` data viewed as r,i,r,i doubles, as kiss_fftr casts its input.
    pub fn process_interleaved(&mut self, fin: &[f64], fout: &mut [KissFftCpx]) {
        kf_work(fout, fin, 0, 1, &self.factors, &self.twiddles, &mut self.scratch);
    }

    /// `kiss_fft` on separate real and imaginary inputs.
    pub fn process(&mut self, fin: &[KissFftCpx], fout: &mut [KissFftCpx]) {
        let interleaved: Vec<f64> = fin.iter().flat_map(|c| [c.r, c.i]).collect();
        self.process_interleaved(&interleaved, fout);
    }
}

fn kf_work(
    fout: &mut [KissFftCpx],
    f: &[f64],
    f_index: usize,
    fstride: usize,
    factors: &[(usize, usize)],
    twiddles: &[KissFftCpx],
    scratch: &mut [KissFftCpx],
) {
    let (p, m) = factors[0];
    let fout = &mut fout[..p * m];

    let mut index = f_index;
    if m == 1 {
        for out in fout.iter_mut() {
            *out = KissFftCpx { r: f[2 * index], i: f[2 * index + 1] };
            index += fstride;
        }
    } else {
        for part in fout.chunks_exact_mut(m) {
            kf_work(part, f, index, fstride * p, &factors[1..], twiddles, scratch);
            index += fstride;
        }
    }

    match p {
        2 => kf_bfly2(fout, fstride, twiddles, m),
        3 => kf_bfly3(fout, fstride, twiddles, m),
        4 => kf_bfly4(fout, fstride, twiddles, m),
        5 => kf_bfly5(fout, fstride, twiddles, m),
        _ => kf_bfly_generic(fout, fstride, twiddles, scratch, m, p),
    }
}

/// A forward `kiss_fftr_cfg`.
pub struct KissFftr {
    substate: KissFft,
    tmpbuf: Vec<KissFftCpx>,
    super_twiddles: Vec<KissFftCpx>,
}

impl KissFftr {
    /// `nfft` must be even, as `kiss_fftr_alloc` requires. The caller checks.
    pub fn new(nfft: usize) -> KissFftr {
        let ncfft = nfft >> 1;
        let super_twiddles =
            (0..ncfft / 2).map(|i| kf_cexp(-std::f64::consts::PI * ((i + 1) as f64 / ncfft as f64 + 0.5))).collect();
        KissFftr { substate: KissFft::new(ncfft), tmpbuf: vec![KissFftCpx::default(); ncfft], super_twiddles }
    }

    /// `kiss_fftr`: `timedata` has nfft points, `freqdata` receives nfft/2+1.
    pub fn process(&mut self, timedata: &[f64], freqdata: &mut [KissFftCpx]) {
        let ncfft = self.substate.nfft();
        self.substate.process_interleaved(timedata, &mut self.tmpbuf);

        let tdc = self.tmpbuf[0];
        freqdata[0].r = tdc.r + tdc.i;
        freqdata[ncfft].r = tdc.r - tdc.i;
        freqdata[ncfft].i = 0.0;
        freqdata[0].i = 0.0;

        for k in 1..=ncfft / 2 {
            let fpk = self.tmpbuf[k];
            let fpnk = KissFftCpx { r: self.tmpbuf[ncfft - k].r, i: -self.tmpbuf[ncfft - k].i };

            let f1k = c_add(fpk, fpnk);
            let f2k = c_sub(fpk, fpnk);
            let tw = c_mul(f2k, self.super_twiddles[k - 1]);

            freqdata[k].r = (f1k.r + tw.r) * 0.5;
            freqdata[k].i = (f1k.i + tw.i) * 0.5;
            freqdata[ncfft - k].r = (f1k.r - tw.r) * 0.5;
            freqdata[ncfft - k].i = (tw.i - f1k.i) * 0.5;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factors_match_kiss_fft_ordering() {
        assert_eq!(kf_factor(512), vec![(4, 128), (4, 32), (4, 8), (4, 2), (2, 1)]);
        assert_eq!(kf_factor(2048), vec![(4, 512), (4, 128), (4, 32), (4, 8), (4, 2), (2, 1)]);
        assert_eq!(kf_factor(557), vec![(557, 1)]);
        assert_eq!(kf_factor(348), vec![(4, 87), (3, 29), (29, 1)]);
        assert_eq!(kf_factor(185), vec![(5, 37), (37, 1)]);
        assert_eq!(kf_factor(1), vec![(1, 1)]);
    }

    fn naive_dft(input: &[KissFftCpx]) -> Vec<KissFftCpx> {
        let n = input.len();
        (0..n)
            .map(|k| {
                let mut sum = KissFftCpx::default();
                for (t, x) in input.iter().enumerate() {
                    let phase = -2.0 * std::f64::consts::PI * (k * t % n) as f64 / n as f64;
                    sum = c_add(sum, c_mul(*x, kf_cexp(phase)));
                }
                sum
            })
            .collect()
    }

    #[test]
    fn every_butterfly_agrees_with_a_naive_dft() {
        // 2, 4 and 3, 5, generic radices all occur here.
        for n in [1usize, 2, 3, 4, 5, 6, 7, 8, 9, 10, 15, 16, 21, 25, 29, 60, 87, 92, 128, 348, 557] {
            let input: Vec<KissFftCpx> = (0..n)
                .map(|i| KissFftCpx { r: ((i * 7 + 3) % 11) as f64 - 5.0, i: ((i * 5 + 1) % 13) as f64 - 6.0 })
                .collect();
            let mut out = vec![KissFftCpx::default(); n];
            KissFft::new(n).process(&input, &mut out);
            let expected = naive_dft(&input);
            for (a, b) in out.iter().zip(&expected) {
                assert!((a.r - b.r).abs() < 1e-8 && (a.i - b.i).abs() < 1e-8, "n={n}: {a:?} vs {b:?}");
            }
        }
    }

    #[test]
    fn real_transform_agrees_with_complex_transform() {
        for n in [2usize, 4, 6, 8, 16, 184, 348, 512, 742, 1024] {
            let input: Vec<f64> = (0..n).map(|i| ((i * 13 + 5) % 17) as f64 - 8.0).collect();
            let complex: Vec<KissFftCpx> = input.iter().map(|&r| KissFftCpx { r, i: 0.0 }).collect();
            let mut expected = vec![KissFftCpx::default(); n];
            KissFft::new(n).process(&complex, &mut expected);
            let mut out = vec![KissFftCpx::default(); n / 2 + 1];
            KissFftr::new(n).process(&input, &mut out);
            for (a, b) in out.iter().zip(&expected) {
                assert!((a.r - b.r).abs() < 1e-8 && (a.i - b.i).abs() < 1e-8, "n={n}: {a:?} vs {b:?}");
            }
        }
    }
}
