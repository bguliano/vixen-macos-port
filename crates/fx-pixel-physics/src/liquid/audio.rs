//! `VixenModules.Media.Audio.AudioUtilities` and `AudioFilters`, as far as the musical flow control needs them. The
//! C# keeps these in a shared module. This crate can't use the port in `fx-audio-state`, so it has its own.

use std::f64::consts::PI;
use vixen_effects::prelude::*;

/// The `AudioUtilities` properties the effect sets.
#[derive(Copy, Clone, Debug)]
pub struct AudioSettings {
    pub attack_time: i32,
    pub decay_time: i32,
    pub normalize: bool,
    pub gain: f64,
    pub low_pass: bool,
    pub low_pass_freq: f64,
    pub high_pass: bool,
    pub high_pass_freq: f64,
}

/// A loaded `AudioUtilities`.
pub struct AudioUtilities {
    volume: Vec<f64>,
    sample_rate: f32,
    total_ms: f64,
}

impl AudioUtilities {
    /// `ReloadAudio`.
    pub fn reload(
        audio: &dyn AudioSource,
        start_time: TimeSpan,
        time_span: TimeSpan,
        settings: &AudioSettings,
    ) -> Option<AudioUtilities> {
        let rate = audio.sample_rate();
        let sample_rate = rate as f32;
        let mut channel = read_effect_audio(audio, start_time, time_span)?;

        if settings.low_pass {
            channel = filter(settings.low_pass_freq, rate as i32, &channel, false);
        }
        if settings.high_pass {
            channel = filter(settings.high_pass_freq, rate as i32, &channel, true);
        }

        let volume = recalculate_volume(&channel, sample_rate, settings);
        Some(AudioUtilities { volume, sample_rate, total_ms: time_span.total_ms() })
    }

    /// `ClosestSample`.
    fn closest_sample(&self, time: i32) -> i32 {
        if time < 0 {
            0
        } else if time as f64 >= self.total_ms {
            self.volume.len() as i32 - 1
        } else {
            // An int times a float is a float in C#, and only then a double.
            ((time as f32 * self.sample_rate) as f64 / 1000.0) as i32
        }
    }

    /// `VolumeAtTime`: in dB, relative to the start of the effect.
    pub fn volume_at_time(&self, time: i32) -> f64 {
        let sample = self.closest_sample(time);
        if sample < 0 || sample as usize >= self.volume.len() {
            return 0.0;
        }
        self.volume[sample as usize]
    }
}

/// `RecalculateVolume`: the signal in dB, with meter ballistics.
fn recalculate_volume(channel: &[f64], rate: f32, s: &AudioSettings) -> Vec<f64> {
    let db = |sample: f64| 20.0 * (if sample == 0.0 { 0.0001 } else { sample }).abs().log10();
    let rate = rate as f64;
    let max_attack = 30.0 / (s.attack_time as f64 / 1000.0 * rate);
    let max_decay = 30.0 / (s.decay_time as f64 / 1000.0 * rate);
    let fast_decay = 30.0 / (10.0 / 1000.0 * rate);

    let mut volume = vec![0.0_f64; channel.len()];
    volume[0] = db(channel[0]);
    let mut max_volume_index = 0;
    for i in 1..volume.len() {
        // Rectify the signal and convert to dB.
        let mut new_sample = db(channel[i]);

        // fastDecay converts the AC signal into a DC representation of the volume.
        if volume[i - 1] > new_sample && (volume[i - 1] - new_sample) > fast_decay {
            new_sample = volume[i - 1] - fast_decay;
        }
        volume[i] = new_sample;

        if volume[i - 1] < volume[i] {
            if (volume[i] - volume[i - 1]) > max_attack {
                volume[i] = volume[i - 1] + max_attack;
            }
        } else if (volume[i - 1] - volume[i]) > max_decay {
            volume[i] = volume[i - 1] - max_decay;
        }
        if volume[i] > volume[max_volume_index] {
            max_volume_index = i;
        }
    }

    let ceiling = volume[max_volume_index];
    let offset = if s.normalize { s.gain + ceiling.abs() } else { s.gain };
    for v in &mut volume {
        *v += offset;
    }
    volume
}

/// `AudioFilters.LowPass` and `HighPass`, which only differ in their coefficients.
fn filter(freq: f64, sample_rate: i32, data: &[f64], high_pass: bool) -> Vec<f64> {
    let fc = freq / sample_rate as f64;
    let num_poles = if fc <= 0.02 {
        4
    } else if fc <= 0.05 {
        6
    } else if fc <= 0.10 {
        10
    } else if fc <= 0.25 {
        18
    } else if fc <= 0.40 {
        10
    } else if fc <= 0.45 {
        6
    } else {
        4
    };
    let (a, b) = generate_coefficients(fc, high_pass, 2.0, num_poles);

    let mut padded = vec![0.0; data.len() + 100];
    let mut new_data = vec![0.0; data.len() + 100];
    padded[50..50 + data.len()].copy_from_slice(data);

    for i in 20..(padded.len() - num_poles) {
        let current = padded[i] * a[0];
        let mut future = 0.0;
        let mut past = 0.0;
        for x in 1..a.len() {
            future += padded[i + x] * a[x];
            past += new_data[i - x] * b[x - 1];
        }
        new_data[i] = current + future + past;
    }

    (0..data.len()).map(|i| new_data[50 + i].clamp(-1.0, 1.0)).collect()
}

/// `AudioFilters.generateCoefficients` (dspguide.com, chapter 20): the A and B coefficients.
fn generate_coefficients(frequency_cutoff: f64, high_pass: bool, percent_ripple: f64, num_poles: usize) -> (Vec<f64>, Vec<f64>) {
    let mut a = [0.0_f64; 22];
    let mut b = [0.0_f64; 22];
    a[2] = 1.0;
    b[2] = 1.0;
    let poles = num_poles as f64;

    for p in 1..=(num_poles / 2) {
        let pole = p as f64 - 1.0;
        let mut rp = -(PI / (poles * 2.0) + pole * PI / poles).cos();
        let mut ip = (PI / (poles * 2.0) + pole * PI / poles).sin();

        if percent_ripple != 0.0 {
            // Warp from a circle to an ellipse.
            let es = ((100.0 / (100.0 - percent_ripple)).powf(2.0) - 1.0).sqrt();
            let vx = (1.0 / poles) * ((1.0 / es) + ((1.0 / es.powf(2.0)) + 1.0).sqrt()).ln();
            let kx = (1.0 / poles) * ((1.0 / es) + ((1.0 / es.powf(2.0)) - 1.0).sqrt()).ln();
            let kx = (kx.exp() + (-kx).exp()) / 2.0;
            rp = rp * ((vx.exp() - (-vx).exp()) / 2.0) / kx;
            ip = ip * ((vx.exp() + (-vx).exp()) / 2.0) / kx;
        }

        // s-domain to z-domain conversion.
        let t = 2.0 * 0.5_f64.tan();
        let w = 2.0 * PI * frequency_cutoff;
        let m = rp.powf(2.0) + ip.powf(2.0);
        let d = 4.0 - 4.0 * rp * t + m * t.powf(2.0);
        let x0 = t.powf(2.0) / d;
        let x1 = 2.0 * t.powf(2.0) / d;
        let x2 = t.powf(2.0) / d;
        let y1 = (8.0 - 2.0 * m * t.powf(2.0)) / d;
        let y2 = (-4.0 - 4.0 * rp * t - m * t.powf(2.0)) / d;

        // Low pass to low pass, or low pass to high pass.
        let k = if high_pass {
            -(w / 2.0 + 0.5).cos() / (w / 2.0 - 0.5).cos()
        } else {
            (0.5 - w / 2.0).sin() / (0.5 + w / 2.0).sin()
        };
        let d = 1.0 + y1 * k - y2 * k.powf(2.0);
        let a0 = (x0 - x1 * k + x2 * k.powf(2.0)) / d;
        let mut a1 = (-2.0 * x0 * k + x1 + x1 * k.powf(2.0) - 2.0 * x2 * k) / d;
        let a2 = (x0 * k.powf(2.0) - x1 * k + x2) / d;
        let mut b1 = (2.0 * k + y1 + y1 * k.powf(2.0) - 2.0 * y2 * k) / d;
        let b2 = (-(k.powf(2.0)) - y1 * k + y2) / d;
        if high_pass {
            a1 = -a1;
            b1 = -b1;
        }

        let (ta, tb) = (a, b);
        for i in 2..22 {
            a[i] = a0 * ta[i] + a1 * ta[i - 1] + a2 * ta[i - 2];
            b[i] = tb[i] - b1 * tb[i - 1] - b2 * tb[i - 2];
        }
    }

    b[2] = 0.0;
    for i in 0..20 {
        a[i] = a[i + 2];
        b[i] = -b[i + 2];
    }

    let (mut sa, mut sb) = (0.0, 0.0);
    for i in 0..20 {
        let sign = if high_pass { (-1.0_f64).powi(i as i32) } else { 1.0 };
        sa += a[i] * sign;
        sb += b[i] * sign;
    }
    let gain = sa / (1.0 - sb);
    for v in a.iter_mut().take(20) {
        *v /= gain;
    }

    (a[..=num_poles].to_vec(), b[1..=num_poles].to_vec())
}

#[cfg(test)]
pub mod test_support {
    use super::*;

    /// A mono signal for tests.
    pub struct FakeAudio {
        pub samples: Vec<f32>,
        pub rate: u32,
    }

    impl AudioSource for FakeAudio {
        fn samples(&self) -> &[f32] {
            &self.samples
        }
        fn sample_rate(&self) -> u32 {
            self.rate
        }
        fn duration(&self) -> TimeSpan {
            TimeSpan::from_secs(self.samples.len() as f64 / self.rate as f64)
        }
    }

    pub fn settings() -> AudioSettings {
        AudioSettings {
            attack_time: 50,
            decay_time: 1500,
            normalize: false,
            gain: 0.0,
            low_pass: false,
            low_pass_freq: 100.0,
            high_pass: false,
            high_pass_freq: 500.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;

    fn step(rate: u32, seconds: f64, silent_ms: f64) -> FakeAudio {
        let n = (rate as f64 * seconds) as usize;
        let first = (rate as f64 * silent_ms / 1000.0) as usize;
        FakeAudio { samples: (0..n).map(|i| if i < first { 0.0 } else { 1.0 }).collect(), rate }
    }

    fn sine(rate: u32, seconds: f64, freq: f64) -> Vec<f64> {
        let n = (rate as f64 * seconds) as usize;
        (0..n).map(|i| (2.0 * PI * freq * i as f64 / rate as f64).sin()).collect()
    }

    fn load(audio: &FakeAudio, ms: i64, s: &AudioSettings) -> AudioUtilities {
        AudioUtilities::reload(audio, TimeSpan::ZERO, TimeSpan::from_ms_i(ms), s).unwrap()
    }

    #[test]
    fn a_full_scale_signal_is_zero_db_and_silence_minus_eighty() {
        let u = load(&step(8000, 1.0, 500.0), 1000, &settings());
        assert!((u.volume_at_time(100) + 80.0).abs() < 1e-9);
        assert_eq!(u.volume_at_time(900), 0.0);
    }

    #[test]
    fn attack_limits_the_rise() {
        // 30 dB per AttackTime: 50 ms after the step it has climbed from -80 to -50.
        let u = load(&step(8000, 1.0, 500.0), 1000, &settings());
        let v = u.volume_at_time(550);
        assert!((v + 50.0).abs() < 0.2, "{v}");
    }

    #[test]
    fn decay_limits_the_fall() {
        let rate = 8000;
        let samples = (0..rate).map(|i| if i < rate / 2 { 1.0 } else { 0.0 }).collect();
        let u = load(&FakeAudio { samples, rate: rate as u32 }, 1000, &settings());
        // 30 dB per DecayTime (1500 ms): 10 dB after half a second.
        let v = u.volume_at_time(999);
        assert!((v + 10.0).abs() < 0.2, "{v}");
    }

    #[test]
    fn normalize_lifts_by_the_ceiling() {
        // A constant half scale signal sits at about -6 dB, which normalizing moves to 0, and gain adds on top.
        let audio = FakeAudio { samples: vec![0.5; 8000], rate: 8000 };
        let plain = load(&audio, 1000, &settings()).volume_at_time(500);
        assert!((plain + 6.0206).abs() < 1e-3, "{plain}");
        let normalized = load(&audio, 1000, &AudioSettings { normalize: true, gain: 10.0, ..settings() }).volume_at_time(500);
        assert!((normalized - 10.0).abs() < 1e-9, "{normalized}");
    }

    #[test]
    fn short_audio_pads_with_silence_and_times_clamp() {
        let audio = FakeAudio { samples: vec![1.0; 4000], rate: 8000 };
        let u = load(&audio, 1000, &settings());
        assert_eq!(u.volume.len(), 8000);
        assert_eq!(u.closest_sample(-3), 0);
        assert_eq!(u.closest_sample(500), 4000);
        assert_eq!(u.closest_sample(5000), 7999, "at or past the end is the last sample");
    }

    #[test]
    fn start_time_picks_the_samples() {
        let audio = step(8000, 2.0, 1000.0);
        let u = AudioUtilities::reload(&audio, TimeSpan::from_ms_i(1000), TimeSpan::from_ms_i(500), &settings()).unwrap();
        assert_eq!(u.volume_at_time(100), 0.0, "a second in, the signal is on");
    }

    #[test]
    fn no_samples_or_no_rate_is_no_audio() {
        let audio = FakeAudio { samples: vec![1.0; 10], rate: 0 };
        assert!(AudioUtilities::reload(&audio, TimeSpan::ZERO, TimeSpan::from_ms_i(10), &settings()).is_none());
        let audio = FakeAudio { samples: vec![1.0; 10], rate: 8000 };
        assert!(AudioUtilities::reload(&audio, TimeSpan::ZERO, TimeSpan::ZERO, &settings()).is_none());
    }

    #[test]
    fn filters_attenuate_out_of_band() {
        let rate = 8000;
        let low = sine(rate, 0.5, 100.0);
        let high = sine(rate, 0.5, 3000.0);
        let peak = |d: &[f64]| d[1000..3000].iter().fold(0.0_f64, |m, v| m.max(v.abs()));
        assert!(peak(&filter(300.0, rate as i32, &low, false)) > 0.7);
        assert!(peak(&filter(300.0, rate as i32, &high, false)) < 0.1);
        assert!(peak(&filter(1000.0, rate as i32, &high, true)) > 0.7);
        assert!(peak(&filter(1000.0, rate as i32, &low, true)) < 0.1);
    }

    #[test]
    fn coefficient_counts_follow_the_poles() {
        for poles in [4, 6, 10, 18] {
            let (a, b) = generate_coefficients(0.1, false, 2.0, poles);
            assert_eq!(a.len(), poles + 1);
            assert_eq!(b.len(), poles);
        }
    }
}
