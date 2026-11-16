//! The part of `VixenModules.Media.Audio.AudioUtilities` (and `AudioFilters`) that Fireworks uses: the
//! ballistic volume in dB of the effect's slice of the audio. The numeric code is the same as the
//! `AudioHelper` port in fx-audio-state, repeated here because effect crates do not depend on each other.

use vixen_effects::prelude::*;

/// The audio settings the C# pushes into its `AudioUtilities` through the property setters.
pub struct AudioSettings {
    pub low_pass: bool,
    pub low_pass_freq: f64,
    pub high_pass: bool,
    pub high_pass_freq: f64,
    pub normalize: bool,
    pub attack_time: i32,
    pub decay_time: i32,
    pub gain: f64,
}

/// `AudioUtilities` after `ReloadAudio`.
pub struct AudioVolume {
    volume: Vec<f64>,
    sample_rate: f32,
    total_ms: f64,
}

impl AudioVolume {
    /// `ReloadAudio`. `None` is every case where the C# leaves `AudioLoaded` false: no audio, no samples,
    /// or an effect shorter than one sample.
    pub fn load(ctx: &RenderCtx, s: &AudioSettings) -> Option<AudioVolume> {
        let audio = ctx.audio?;
        let rate = audio.sample_rate();
        let sample_rate = rate as f32;
        let mut channel = read_effect_audio(audio, ctx.start_time, ctx.time_span)?;
        if s.low_pass {
            channel = filter(s.low_pass_freq, rate as i32, &channel, false);
        }
        if s.high_pass {
            channel = filter(s.high_pass_freq, rate as i32, &channel, true);
        }
        let volume = recalculate_volume(&channel, sample_rate, s.attack_time, s.decay_time, s.normalize, s.gain);
        Some(AudioVolume { volume, sample_rate, total_ms: ctx.time_span.total_ms() })
    }

    /// `ClosestSample`.
    pub fn closest_sample(&self, time: i32) -> i64 {
        if time < 0 {
            0
        } else if time as f64 >= self.total_ms {
            self.volume.len() as i64 - 1
        } else {
            // int * float is a float in C#
            ((time as f32 * self.sample_rate) as f64 / 1000.0) as i64
        }
    }

    /// `VolumeAtTime`: dB relative to the start of the effect.
    pub fn volume_at_time(&self, time: i32) -> f64 {
        let sample = self.closest_sample(time);
        if sample < 0 || sample as usize >= self.volume.len() {
            return 0.0;
        }
        self.volume[sample as usize]
    }
}

/// `AudioFilters.generateCoefficients`: (A, B).
fn generate_coefficients(fc: f64, high_pass: bool, percent_ripple: f64, num_poles: usize) -> (Vec<f64>, Vec<f64>) {
    use std::f64::consts::PI;
    let mut a = [0.0_f64; 22];
    let mut b = [0.0_f64; 22];
    a[2] = 1.0;
    b[2] = 1.0;
    let mut k;
    let np = num_poles as f64;
    for p in 1..=(num_poles / 2) {
        let pf = p as f64;
        let mut rp = -(PI / (np * 2.0) + (pf - 1.0) * PI / np).cos();
        let mut ip = (PI / (np * 2.0) + (pf - 1.0) * PI / np).sin();
        if percent_ripple != 0.0 {
            let es = ((100.0 / (100.0 - percent_ripple)).powi(2) - 1.0).sqrt();
            let vx = (1.0 / np) * ((1.0 / es) + ((1.0 / es.powi(2)) + 1.0).sqrt()).ln();
            let mut kx = (1.0 / np) * ((1.0 / es) + ((1.0 / es.powi(2)) - 1.0).sqrt()).ln();
            kx = (kx.exp() + (-kx).exp()) / 2.0;
            rp = rp * ((vx.exp() - (-vx).exp()) / 2.0) / kx;
            ip = ip * ((vx.exp() + (-vx).exp()) / 2.0) / kx;
        }
        let t = 2.0 * 0.5_f64.tan();
        let w = 2.0 * PI * fc;
        let m = rp.powi(2) + ip.powi(2);
        let mut d = 4.0 - 4.0 * rp * t + m * t.powi(2);
        let x0 = t.powi(2) / d;
        let x1 = 2.0 * t.powi(2) / d;
        let x2 = t.powi(2) / d;
        let y1 = (8.0 - 2.0 * m * t.powi(2)) / d;
        let y2 = (-4.0 - 4.0 * rp * t - m * t.powi(2)) / d;

        k = if high_pass { -(w / 2.0 + 0.5).cos() / (w / 2.0 - 0.5).cos() } else { (0.5 - w / 2.0).sin() / (0.5 + w / 2.0).sin() };
        d = 1.0 + y1 * k - y2 * k.powi(2);
        let a0 = (x0 - x1 * k + x2 * k.powi(2)) / d;
        let mut a1 = (-2.0 * x0 * k + x1 + x1 * k.powi(2) - 2.0 * x2 * k) / d;
        let a2 = (x0 * k.powi(2) - x1 * k + x2) / d;
        let mut b1 = (2.0 * k + y1 + y1 * k.powi(2) - 2.0 * y2 * k) / d;
        let b2 = (-(k.powi(2)) - y1 * k + y2) / d;
        if high_pass {
            a1 = -a1;
            b1 = -b1;
        }
        let ta = a;
        let tb = b;
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
        let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
        if !high_pass {
            sa += a[i];
            sb += b[i];
        } else {
            sa += a[i] * sign;
            sb += b[i] * sign;
        }
    }
    let gain = sa / (1.0 - sb);
    for v in a.iter_mut().take(20) {
        *v /= gain;
    }
    let ra = a[..=num_poles].to_vec();
    let rb: Vec<f64> = (1..=num_poles).map(|i| b[i]).collect();
    (ra, rb)
}

/// `AudioFilters.LowPass` / `HighPass`: they are the same code apart from the coefficients.
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
    let (ca, cb) = generate_coefficients(fc, high_pass, 2.0, num_poles);
    let mut padded = vec![0.0; data.len() + 100];
    let mut new_data = vec![0.0; data.len() + 100];
    padded[50..50 + data.len()].copy_from_slice(data);
    let end = padded.len().saturating_sub(num_poles);
    for i in 20..end {
        let current = padded[i] * ca[0];
        let mut future = 0.0;
        let mut past = 0.0;
        for x in 1..ca.len() {
            future += padded[i + x] * ca[x];
            past += new_data[i - x] * cb[x - 1];
        }
        new_data[i] = current + future + past;
    }
    (0..data.len()).map(|i| new_data[50 + i].clamp(-1.0, 1.0)).collect()
}

/// `AudioUtilities.RecalculateVolume`.
fn recalculate_volume(channel: &[f64], rate: f32, attack_time: i32, decay_time: i32, normalize: bool, gain: f64) -> Vec<f64> {
    let rate = rate as f64;
    let db = |s: f64| 20.0 * (if s == 0.0 { 0.0001 } else { s }).abs().log10();
    let mut volume = vec![0.0_f64; channel.len()];
    volume[0] = db(channel[0]);
    let max_attack = 30.0 / (attack_time as f64 / 1000.0 * rate);
    let max_decay = 30.0 / (decay_time as f64 / 1000.0 * rate);
    let fast_decay = 30.0 / (10.0 / 1000.0 * rate);
    let mut max_i = 0;
    for i in 1..volume.len() {
        let mut new_sample = db(channel[i]);
        // fastDecay converts the AC signal into a DC representation of the volume
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
        if volume[i] > volume[max_i] {
            max_i = i;
        }
    }
    let ceiling = volume[max_i];
    if normalize {
        for v in volume.iter_mut() {
            *v = *v + gain + ceiling.abs();
        }
    } else {
        for v in volume.iter_mut() {
            *v += gain;
        }
    }
    volume
}
