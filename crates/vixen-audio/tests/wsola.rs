mod common;
use common::*;
use vixen_audio::{time_stretch, AudioClip};

/// Frequency from positive-going zero crossings over the middle of the signal.
fn freq(samples: &[f32], rate: u32) -> f64 {
    let a = samples.len() / 4;
    let b = samples.len() * 3 / 4;
    let mut first = None;
    let mut n = 0usize;
    for i in a + 1..b {
        if samples[i - 1] < 0.0 && samples[i] >= 0.0 {
            let frac = -samples[i - 1] / (samples[i] - samples[i - 1]);
            let pos = i as f64 - 1.0 + frac as f64;
            if first.is_none() {
                first = Some(pos);
            }
            n += 1;
            // track precise last crossing
            LAST.with(|l| l.set(pos));
        }
    }
    let first = first.unwrap();
    let last = LAST.with(|l| l.get());
    (n - 1) as f64 * rate as f64 / (last - first)
}
thread_local!(static LAST: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) });

#[test]
fn stretch_keeps_pitch_and_scales_length() {
    for &(f, rate) in &[(440.0f64, 44100u32), (110.0, 48000), (1000.0, 22050)] {
        let clip = sine_clip(rate, 1, 6.0, f, 0.7);
        for &speed in &[0.5f32, 0.75, 1.25, 1.5, 2.0, 2.1, 4.0] {
            let out = time_stretch(&clip, speed);
            let expect = clip.frames() as f64 / speed as f64;
            let err = (out.len() as f64 - expect).abs() / expect;
            assert!(err < 0.02, "f={f} speed={speed}: len {} vs {expect}", out.len());
            let got = freq(&out, rate);
            assert!(((got - f) / f).abs() < 0.02, "f={f} speed={speed}: measured {got}");
            let peak = out.iter().cloned().fold(0.0f32, |a, b| a.max(b.abs()));
            assert!(peak > 0.6 && peak < 0.8, "amplitude {peak}");
        }
    }
}

#[test]
fn stereo_stretch_keeps_channels() {
    let rate = 44100;
    let mut s = Vec::new();
    for i in 0..rate as usize * 4 {
        let t = i as f64 / rate as f64;
        s.push((2.0 * std::f64::consts::PI * 300.0 * t).sin() as f32 * 0.5);
        s.push((2.0 * std::f64::consts::PI * 600.0 * t).sin() as f32 * 0.5);
    }
    let clip = AudioClip::from_interleaved(s, 2, rate);
    let out = time_stretch(&clip, 1.5);
    assert_eq!(out.len() % 2, 0);
    let l: Vec<f32> = out.chunks(2).map(|f| f[0]).collect();
    let r: Vec<f32> = out.chunks(2).map(|f| f[1]).collect();
    assert!(((freq(&l, rate) - 300.0) / 300.0).abs() < 0.02);
    assert!(((freq(&r, rate) - 600.0) / 600.0).abs() < 0.02);
}

#[test]
fn speed_one_is_an_exact_copy() {
    let clip = noise_clip(44100, 2, 5000, 8);
    assert_eq!(time_stretch(&clip, 1.0), clip.samples());
}
