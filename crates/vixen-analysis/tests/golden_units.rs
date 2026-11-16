//! Calls each ported class directly and compares with tools/qm-golden/qm_golden_units.cpp, which does the same with the
//! upstream classes. The check list has to stay the same in both, including the order of the seeds.

mod common;

use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;

use common::{hash_f64, EXACT};
use vixen_analysis::decimator::Decimator;
use vixen_analysis::detection_function::{DetectionFunction, DfConfig};
use vixen_analysis::down_beat::{measure_spec_diff, DownBeat};
use vixen_analysis::fft::FftReal;
use vixen_analysis::math_utilities::{adaptive_threshold, get_max, modulo, next_power_of_two, princarg};
use vixen_analysis::phase_vocoder::PhaseVocoder;
use vixen_analysis::tempo_track_v2::TempoTrackV2;
use vixen_analysis::window::HanningWindow;
use vixen_analysis::RealTime;

type Checks = BTreeMap<String, Vec<f64>>;

fn xorshift32(x: &mut u32) -> u32 {
    *x ^= *x << 13;
    *x ^= *x >> 17;
    *x ^= *x << 5;
    *x
}

fn noise(seed: u32, n: usize, scale: f64) -> Vec<f64> {
    let mut state = seed;
    (0..n).map(|_| xorshift32(&mut state) as i32 as f64 / 2147483648.0 * scale).collect()
}

fn magnitudes(seed: u32, n: usize, scale: f64) -> Vec<f64> {
    noise(seed, n, scale).into_iter().map(f64::abs).collect()
}

fn heavy_tailed(seed: u32, n: usize) -> Vec<f64> {
    noise(seed, n, 12.0).into_iter().map(f64::exp).collect()
}

fn to_float(values: &[f64]) -> Vec<f32> {
    values.iter().map(|&v| v as f32).collect()
}

fn math_checks(checks: &mut Checks) {
    let xs = noise(101, 2000, 100.0);
    let ys = noise(102, 2000, 10.0);
    checks.insert("math_mod".into(), xs.iter().zip(&ys).map(|(&x, &y)| modulo(x, y)).collect());
    checks.insert("math_princarg".into(), xs.iter().map(|&x| princarg(x)).collect());

    for n in [1u32, 2, 3, 8, 15, 16, 17, 64, 128, 512, 2048] {
        let mut data = noise(200 + n, n as usize, 1000.0);
        adaptive_threshold(&mut data);
        checks.insert(format!("adaptive_threshold_{n}"), data);
    }

    let mut out = Vec::new();
    for n in [1u32, 2, 3, 10, 100, 1000] {
        out.push(get_max(&noise(300 + n, n as usize, 1.0)) as f64);
        out.push(get_max(&vec![0.0; n as usize]) as f64);
    }
    checks.insert("math_get_max".into(), out);

    checks.insert("math_next_power_of_two".into(), (-4..5000).map(|x| next_power_of_two(x) as f64).collect());
}

fn window_checks(checks: &mut Checks) {
    for n in [1usize, 2, 3, 512, 1024, 1114] {
        let window = HanningWindow::new(n);
        let input = noise(400 + n as u32, n, 1000.0);
        let mut dst = vec![0.0; n];
        window.cut(&input, &mut dst);
        checks.insert(format!("window_hanning_{n}"), dst);
    }
}

fn fft_checks(checks: &mut Checks) {
    let sizes = [
        2usize, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30, 32, 34, 36, 40, 46, 50, 56, 64, 70, 74, 90, 92,
        96, 128, 184, 256, 348, 370, 512, 556, 742, 1024, 1114, 2048, 4096, 8192,
    ];
    for n in sizes {
        let mut fft = FftReal::new(n).unwrap();
        let input = noise(500 + n as u32, n, 30000.0);
        let (mut re, mut im) = (vec![0.0; n], vec![0.0; n]);
        fft.forward(&input, &mut re, &mut im);
        re.extend_from_slice(&im);
        checks.insert(format!("fft_real_{n}"), re);
    }

    let mut fft = FftReal::new(1024).unwrap();
    let (mut re, mut im) = (vec![0.0; 1024], vec![0.0; 1024]);
    fft.forward(&vec![0.0; 1024], &mut re, &mut im);
    re.extend_from_slice(&im);
    checks.insert("fft_real_silence".into(), re);
}

fn phase_vocoder_checks(checks: &mut Checks) {
    for n in [16usize, 512, 1024, 1114] {
        let mut pv = PhaseVocoder::new(n).unwrap();
        let half = n / 2 + 1;
        let (mut mag, mut theta) = (vec![0.0; half], vec![0.0; half]);
        let mut out = Vec::new();
        for frame in 0..3 {
            let input = noise(600 + n as u32 + frame, n, 30000.0);
            pv.process_time_domain(&input, &mut mag, &mut theta);
            out.extend_from_slice(&mag);
            out.extend_from_slice(&theta);
        }
        checks.insert(format!("phase_vocoder_{n}"), out);
    }
}

fn detection_function_checks(checks: &mut Checks) {
    for n in [16usize, 512, 1024, 1114] {
        let mut df = DetectionFunction::new(DfConfig { step_size: n as i32 / 2, frame_length: n as i32 }).unwrap();
        let mut out = Vec::new();
        for frame in 0..8 {
            let input = noise(700 + n as u32 + frame, n, 2147483648.0);
            out.push(df.process_time_domain(&input));
        }
        let silence = vec![0.0; n];
        for _ in 0..3 {
            out.push(df.process_time_domain(&silence));
        }
        checks.insert(format!("detection_function_{n}"), out);
    }
}

fn decimator_checks(checks: &mut Checks) {
    for factor in [1usize, 2, 3, 4, 8, 16] {
        const IN_LENGTH: usize = 512;
        let mut decimator = Decimator::new(IN_LENGTH, factor);
        let mut out = Vec::new();
        for block in 0..4 {
            let input = to_float(&noise(800 + factor as u32 + block, IN_LENGTH, 2147483648.0));
            let mut dst = vec![0.0f32; IN_LENGTH / factor];
            decimator.process(&input, &mut dst);
            out.extend(dst.iter().map(|&v| v as f64));
        }
        checks.insert(format!("decimator_{factor}"), out);
    }
}

fn test_weights() -> [f64; 128] {
    let r = 43.0f64;
    let mut wv = [0.0; 128];
    for (i, weight) in wv.iter_mut().enumerate() {
        let d = i as f64;
        *weight = (d / (r * r)) * ((-(d * d)) / (2.0 * (r * r))).exp();
    }
    wv
}

fn tempo_checks(checks: &mut Checks) {
    let cancel = AtomicBool::new(false);
    let tt = TempoTrackV2::new(44100.0, 512);
    let wv = test_weights();

    for seed in 1u32..=5 {
        let frame = magnitudes(900 + seed, 512, (seed * seed) as f64 * 100.0);
        let mut rcf = [0.0; 128];
        TempoTrackV2::get_rcf(&frame, &wv, &mut rcf);
        checks.insert(format!("rcf_{seed}"), rcf.to_vec());
    }
    let mut rcf = [0.0; 128];
    TempoTrackV2::get_rcf(&[0.0; 512], &wv, &mut rcf);
    checks.insert("rcf_silence".into(), rcf.to_vec());

    for t_len in [0u32, 1, 2, 3, 8, 32] {
        let mut rcfmat = Vec::new();
        for t in 0..t_len {
            let mut column = magnitudes(1000 + t_len * 100 + t, 128, 1.0);
            let mut sum = 0.0;
            for value in &column {
                sum += *value;
            }
            for value in &mut column {
                *value /= sum;
            }
            let mut fixed = [0.0; 128];
            fixed.copy_from_slice(&column);
            rcfmat.push(fixed);
        }
        let mut period = vec![0.0; t_len as usize * 128 + 37];
        let mut tempi = Vec::new();
        tt.viterbi_decode(&rcfmat, &wv, &mut period, &mut tempi, &cancel).unwrap();
        period.extend_from_slice(&tempi);
        checks.insert(format!("viterbi_{t_len}"), period);
    }

    let silent_df = vec![0.0; 512 + 128 * 3 + 1];
    for constrain in [false, true] {
        let mut out = Vec::new();
        for center in [30, 43, 60] {
            for k in -60..=60 {
                let tempo = (5167.0 / (center as f64 + 0.5)) * (1.0 + k as f64 * 0.0001);
                let mut period = vec![0.0; silent_df.len()];
                let mut tempi = Vec::new();
                tt.calculate_beat_period(&silent_df, &mut period, &mut tempi, tempo, constrain, &cancel).unwrap();
                out.push(period[0]);
                out.push(period[period.len() - 1]);
            }
        }
        let name = if constrain { "beat_period_sweep_gaussian" } else { "beat_period_sweep_rayleigh" };
        checks.insert(name.into(), out);
    }

    {
        let other = TempoTrackV2::new(48000.0, 557);
        let mut rcfmat = Vec::new();
        for t in 0..3 {
            let mut fixed = [0.0; 128];
            fixed.copy_from_slice(&magnitudes(1200 + t, 128, 1.0));
            rcfmat.push(fixed);
        }
        let mut period = vec![0.0; 3 * 128];
        let mut tempi = Vec::new();
        other.viterbi_decode(&rcfmat, &wv, &mut period, &mut tempi, &cancel).unwrap();
        checks.insert("tempi_48000".into(), tempi);
    }

    let alphas = [0.1, 0.5, 0.9, 0.99];
    let tightnesses = [1.0, 4.0, 10.0];
    let periods = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 43.0, 44.0, 45.0, 86.0, 127.0];
    for seed in 1u32..=12 {
        const DF_LEN: usize = 2000;
        let df = heavy_tailed(1300 + seed, DF_LEN);
        let mut state = 1400 + seed;
        let mut current = 0.0;
        let mut period = vec![0.0; DF_LEN];
        for (i, value) in period.iter_mut().enumerate() {
            if i % 128 == 0 {
                current = periods[(xorshift32(&mut state) % 11) as usize];
            }
            *value = current;
        }
        let mut beats = Vec::new();
        let (alpha, tightness) = (alphas[(seed % 4) as usize], tightnesses[(seed % 3) as usize]);
        tt.calculate_beats(&df, &period, &mut beats, alpha, tightness, &cancel).unwrap();
        checks.insert(format!("calculate_beats_{seed}"), beats);
    }
    for p in [1u32, 2, 3, 5, 7, 43, 44, 45, 127] {
        let df = heavy_tailed(1500 + p, 3000);
        let period = vec![p as f64; df.len()];
        let mut beats = Vec::new();
        tt.calculate_beats(&df, &period, &mut beats, 0.9, 4.0, &cancel).unwrap();
        checks.insert(format!("calculate_beats_constant_{p}"), beats);
    }

    for p in [1usize, 2, 3, 5, 9, 45] {
        let mut out = Vec::new();
        for distance in 1..=2 * p + 1 {
            const DF_LEN: usize = 300;
            let mut df = vec![0.0; DF_LEN];
            df[DF_LEN - 1] = 1.0;
            df[DF_LEN - 1 - distance] = 1.0;
            let period = vec![p as f64; DF_LEN];
            let mut beats = Vec::new();
            tt.calculate_beats(&df, &period, &mut beats, 0.9, 4.0, &cancel).unwrap();
            out.push(-1.0);
            out.extend_from_slice(&beats);
        }
        checks.insert(format!("calculate_beats_spikes_{p}"), out);
    }
}

fn down_beat_checks(checks: &mut Checks) {
    let cancel = AtomicBool::new(false);

    for size in [0usize, 2, 4, 8, 16, 512, 2048, 4096, 8192] {
        let mut out = Vec::new();
        for seed in 1u32..=4 {
            let a = magnitudes(1600 + size as u32 + seed, size, 100.0);
            let b = magnitudes(1700 + size as u32 + seed, size, 100.0);
            out.push(measure_spec_diff(&a, &b));
        }
        out.push(measure_spec_diff(&vec![0.0; size], &vec![0.0; size]));
        checks.insert(format!("spec_diff_{size}"), out);
    }

    let arrangements = [
        (44100.0f32, 16usize, 512usize),
        (48000.0, 16, 557),
        (22050.0, 8, 256),
        (8000.0, 2, 92),
        (5000.0, 1, 58),
        (1.0, 1, 1),
    ];
    let mut index = 0u32;
    for (rate, factor, increment) in arrangements {
        for bpb in [0, 1, 3, 4, 5, 16] {
            let mut downbeat = DownBeat::new(rate, factor, increment);
            downbeat.set_beats_per_bar(bpb);

            let audio = to_float(&noise(1800 + index, 20000, 2147483648.0));

            let mut beats = Vec::new();
            let mut state = 1900 + index;
            let mut position = 0.0;
            let count = 40 + xorshift32(&mut state) % 40;
            for _ in 0..count {
                position += (1 + xorshift32(&mut state) % 300) as f64;
                beats.push(position);
            }
            beats.push(position + 100000.0);
            beats.push(position + 100001.0);
            beats.push(position);

            let mut downbeats = Vec::new();
            downbeat.find_down_beats(&audio, &beats, &mut downbeats, &cancel).unwrap();

            let mut out: Vec<f64> = downbeats.iter().map(|&d| d as f64).collect();
            out.extend_from_slice(downbeat.beat_sd());
            checks.insert(format!("find_down_beats_{index}"), out);
            index += 1;
        }
    }

    for beat_count in [0, 1, 2, 3] {
        let mut downbeat = DownBeat::new(44100.0, 16, 512);
        downbeat.set_beats_per_bar(4);
        let audio = to_float(&noise(2000, 4000, 1.0));
        let beats: Vec<f64> = (0..beat_count).map(|i| i as f64 * 40.0).collect();
        let mut downbeats = Vec::new();
        downbeat.find_down_beats(&audio, &beats, &mut downbeats, &cancel).unwrap();
        let mut out: Vec<f64> = downbeats.iter().map(|&d| d as f64).collect();
        out.extend_from_slice(downbeat.beat_sd());
        checks.insert(format!("find_down_beats_few_{beat_count}"), out);
    }

    let sizes = [(44100.0f32, 16usize), (48000.0, 16), (22050.0, 8), (8000.0, 2), (96000.0, 32), (5000.0, 1), (1.0, 1)];
    checks.insert(
        "beat_frame_sizes".into(),
        sizes.iter().map(|&(rate, factor)| DownBeat::new(rate, factor, 64).beat_frame_size() as f64).collect(),
    );
}

fn real_time_checks(checks: &mut Checks) {
    let mut out = Vec::new();
    for rate in [1u32, 8000, 22050, 44100, 48000, 96000, 192000] {
        for frame in [0i64, 1, 2, 511, 512, 1024, 22050, 44099, 44100, 44101, 1000000, 123456789, -1, -44100, -66150] {
            let t = RealTime::frame2real_time(frame, rate);
            out.push(t.sec as f64);
            out.push(t.nsec as f64);
        }
    }
    checks.insert("real_time_frames".into(), out);

    let mut out = Vec::new();
    for s in [-3, -1, 0, 1, 2] {
        for n in [-1500000000, -1000000000, -999999999, -1, 0, 1, 999999999, 1000000000, 1500000000] {
            let t = RealTime::new(s, n);
            out.push(t.sec as f64);
            out.push(t.nsec as f64);
            let sum = t + RealTime::new(1, 600000000);
            out.push(sum.sec as f64);
            out.push(sum.nsec as f64);
        }
    }
    checks.insert("real_time_normalise_and_add".into(), out);
}

#[test]
fn every_class_matches_the_cpp() {
    let mut checks = Checks::new();
    math_checks(&mut checks);
    window_checks(&mut checks);
    fft_checks(&mut checks);
    phase_vocoder_checks(&mut checks);
    detection_function_checks(&mut checks);
    decimator_checks(&mut checks);
    tempo_checks(&mut checks);
    down_beat_checks(&mut checks);
    real_time_checks(&mut checks);

    let path = common::golden_dir().join("units/primitives.json");
    let text = std::fs::read_to_string(&path).expect("units/primitives.json exists");
    let golden: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
    let golden = golden["checks"].as_object().expect("checks object");

    let mut failures = Vec::new();
    for (name, want) in golden {
        let Some(got) = checks.get(name) else {
            failures.push(format!("{name}: not run here"));
            continue;
        };
        let want_len = want["len"].as_u64().expect("len") as usize;
        if got.len() != want_len {
            failures.push(format!("{name}: {} values, want {want_len}", got.len()));
        } else if EXACT && hash_f64(got) != common::parse_hash(&want["fnv1a64"]) {
            failures.push(format!("{name}: values differ"));
        }
    }
    for name in checks.keys() {
        if !golden.contains_key(name) {
            failures.push(format!("{name}: not in the golden file"));
        }
    }

    println!("{} class-level checks against the C++", golden.len());
    assert!(failures.is_empty(), "{} of {} checks differ:\n{}", failures.len(), golden.len(), failures.join("\n"));
}
