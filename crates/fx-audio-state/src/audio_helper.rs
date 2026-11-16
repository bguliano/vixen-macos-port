//! Port of `AudioHelper` (`AudioPluginBase`, `AudioPluginData`) and `VixenModules.Media.Audio.AudioUtilities`
//! and `AudioFilters`. Waveform, VerticalMeter and VUMeter share it.
//!
//! The parameter tables hold the *data class* values (what `[DataMember]`s store), exactly like the XML does:
//! `Gain` is `Data.Gain`, `Range` is `Data.Range` and `ScrollSpeed` is `Data.ScrollSpeed`. The C# property
//! getters invert some of those for the editor (`Range` shows `20 - Data.Range`, `Gain` shows `Data.Gain * 10`)
//! but every render path reads the data value, and so do these ports.

use vixen_effects::prelude::*;

/// `MeterColorTypes`.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MeterColorTypes {
    Linear,
    Discrete,
    Custom,
}

impl MeterColorTypes {
    pub fn from_name(name: &str) -> MeterColorTypes {
        match name {
            "Discrete" => MeterColorTypes::Discrete,
            "Custom" => MeterColorTypes::Custom,
            _ => MeterColorTypes::Linear,
        }
    }
}

/// `Color[] { Color.Lime, Color.Yellow, Color.Red }` at `{ 0, green, red }` as a `ColorBlend`, the default
/// `MeterColorGradient` of Waveform and VerticalMeter, and the "Linear" working gradient.
pub fn linear_gradient(green_pos: i32, red_pos: i32) -> ColorGradient {
    let green = green_pos as f32 / 100.0;
    let red = red_pos as f32 / 100.0;
    gradient_from_blend(&[Color::LIME, Color::YELLOW, Color::RED], &[0.0, green, red])
}

/// `new ColorGradient(ColorBlend)`: every blend entry becomes a colour point at its position.
pub fn gradient_from_blend(colors: &[Color], positions: &[f32]) -> ColorGradient {
    let points = colors.iter().zip(positions.iter()).map(|(&c, &p)| ColorPoint::new(c, p as f64)).collect();
    ColorGradient::from_parts(points, vec![vixen_core::AlphaPoint::new(255, 0.0), vixen_core::AlphaPoint::new(255, 1.0)])
}

/// `UpdateColorGradient`: the gradient the meter really samples.
pub fn working_gradient(p: &Params) -> ColorGradient {
    let green = p.int("GreenColorPosition");
    let red = p.int("RedColorPosition");
    match MeterColorTypes::from_name(p.enum_name("MeterColorStyle")) {
        MeterColorTypes::Discrete => {
            let g = green as f32 / 100.0;
            let y = red as f32 / 100.0; // deliberately named "yellowPos" in the C#, it is the red position
            gradient_from_blend(
                &[Color::LIME, Color::LIME, Color::YELLOW, Color::YELLOW, Color::RED, Color::RED],
                &[0.0, g, g, y, y, 1.0],
            )
        }
        MeterColorTypes::Linear => linear_gradient(green, red),
        MeterColorTypes::Custom => p.gradient("MeterColorGradient"),
    }
}

/// The `// TODO(gen)` default for Waveform's and VerticalMeter's `MeterColorGradient`.
pub fn default_linear_gradient() -> ColorGradient {
    linear_gradient(25, 95)
}

/// The data class' `IntensityCurve`: flat at 100.
pub fn default_intensity_curve() -> Curve {
    Curve::from_points(vec![(0.0, 100.0), (100.0, 100.0)])
}

/// `AudioPluginBase.UpdateColorTypesAttributes`, `UpdateLowHighPassAttributes`, and the `EnableColorTypesSelection` of
/// `CheckForInvalidColorData`: discrete targets are limited to Custom, so the choice goes away.
pub fn visibility(p: &Params, t: &TargetInfo, v: &mut Visibility) {
    let custom = MeterColorTypes::from_name(p.enum_name("MeterColorStyle")) == MeterColorTypes::Custom;
    v.set("MeterColorStyle", !t.has_discrete_colors());
    v.set("MeterColorGradient", custom);
    v.set("RedColorPosition", !custom);
    v.set("GreenColorPosition", !custom);
    v.set("LowPassFreq", p.boolean("LowPass"));
    v.set("HighPassFreq", p.boolean("HighPass"));
}

/// `CheckForInvalidColorData` in `AudioPluginBase`. Waveform overrides it to build a transition when the
/// targets offer several colours (`blend_transitions`).
pub fn check_for_invalid_color_data(p: &mut Params, valid_colors: &[Color], blend_transitions: bool) {
    if valid_colors.is_empty() {
        return;
    }
    let gradient = p.gradient("MeterColorGradient");
    if !gradient.colors_in_gradient().iter().all(|c| valid_colors.contains(c)) {
        let replacement = if blend_transitions && valid_colors.len() > 1 {
            let n = (valid_colors.len() - 1) as f32;
            let positions: Vec<f32> = (0..valid_colors.len()).map(|i| i as f32 / n).collect();
            gradient_from_blend(valid_colors, &positions)
        } else {
            ColorGradient::solid(valid_colors[0])
        };
        p.set("MeterColorGradient", ParamValue::Gradient(replacement));
    }
    // Discrete colour targets can only use Custom.
    p.set("MeterColorStyle", ParamValue::Enum("Custom".to_string()));
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

/// `AudioUtilities`: the audio of one effect, turned into a ballistic volume in dB.
pub struct AudioHelper {
    volume: Vec<f64>,
    sample_rate: f32,
    total_ms: f64,
}

impl AudioHelper {
    /// `ReloadAudio`. `None` is every case where the C# leaves `AudioLoaded` false or throws (no audio,
    /// no channels, an effect shorter than one sample), so the effect renders nothing.
    pub fn load(ctx: &RenderCtx, p: &Params) -> Option<AudioHelper> {
        let audio = ctx.audio?;
        let rate = audio.sample_rate();
        let sample_rate = rate as f32;
        let mut channel = read_effect_audio(audio, ctx.start_time, ctx.time_span)?;
        if p.boolean("LowPass") {
            channel = filter(p.int("LowPassFreq") as f64, rate as i32, &channel, false);
        }
        if p.boolean("HighPass") {
            channel = filter(p.int("HighPassFreq") as f64, rate as i32, &channel, true);
        }
        let volume = recalculate_volume(
            &channel,
            sample_rate,
            p.int("AttackTime"),
            p.int("DecayTime"),
            p.boolean("Normalize"),
            p.int("Gain") as f64,
        );
        Some(AudioHelper { volume, sample_rate, total_ms: ctx.time_span.total_ms() })
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

    pub fn len(&self) -> usize {
        self.volume.len()
    }

    pub fn is_empty(&self) -> bool {
        self.volume.is_empty()
    }
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

/// Everything a meter's `RenderNode` needs.
pub struct MeterEnv<'a, 'b> {
    pub ctx: &'a RenderCtx<'b>,
    pub audio: AudioHelper,
    pub gradient: ColorGradient,
    pub level: Curve,
    /// `Data.Range`.
    pub range: i32,
    gradient_colors: Vec<Color>,
}

impl MeterEnv<'_, '_> {
    /// `(VolumeAtTime(t) + Data.Range) / Data.Range`, clamped to 0..1 the way the C# does it.
    pub fn gradient_position(&self, time: i32) -> f64 {
        let mut pos = (self.audio.volume_at_time(time) + self.range as f64) / self.range as f64;
        // "Some odd corner cases". A zero Range gives NaN here, which both C# checks let through.
        if pos <= 0.0 {
            pos = 0.0;
        }
        if pos >= 1.0 {
            pos = 1.0;
        }
        pos
    }

    /// `GenerateEffectIntents`.
    #[allow(clippy::too_many_arguments)]
    pub fn generate_effect_intents(
        &self,
        out: &mut EffectIntents,
        element: Uuid,
        node_colors: &[Color],
        start_pos: f64,
        end_pos: f64,
        duration: TimeSpan,
        start_time: TimeSpan,
        is_discrete: bool,
    ) {
        if is_discrete {
            for &color in node_colors {
                let proportion = self.gradient.get_proportion_of_color_at(start_pos, color);
                let start_intensity = (self.level.get_value(start_pos * 100.0) / 100.0) * proportion;
                let proportion = self.gradient.get_proportion_of_color_at(end_pos, color);
                let end_intensity = (self.level.get_value(end_pos * 100.0) / 100.0) * proportion;
                if start_intensity > 0.0 && end_intensity > 0.0 {
                    out.add_intent_for_element(element, Intent::discrete(color, start_intensity, end_intensity, duration), start_time);
                }
            }
        } else {
            let start_intensity = self.level.get_value(start_pos * 100.0) / 100.0;
            let end_intensity = self.level.get_value(end_pos * 100.0) / 100.0;
            if start_intensity > 0.0 && end_intensity > 0.0 {
                let intent = Intent::lighting(
                    self.gradient.get_color_at(start_pos),
                    self.gradient.get_color_at(end_pos),
                    start_intensity,
                    end_intensity,
                    duration,
                );
                out.add_intent_for_element(element, intent, start_time);
            }
        }
    }

    /// The colours `GenerateEffectIntents` iterates for a discrete node: its valid colours that the
    /// gradient contains (`Intersect` keeps the first sequence's order).
    pub fn discrete_colors(&self, node: NodeId) -> Vec<Color> {
        let mut out: Vec<Color> = Vec::new();
        for c in self.ctx.tree.valid_colors(node, false) {
            if self.gradient_colors.contains(&c) && !out.contains(&c) {
                out.push(c);
            }
        }
        out
    }
}

/// `AudioPluginBase._PreRender` and `GetNodesToRenderOn`: loads the audio, then calls `render_node` once
/// per node at the effect depth. No audio, no output.
pub fn render_meter(ctx: &RenderCtx, p: &Params, mut render_node: impl FnMut(&MeterEnv, NodeId, &mut EffectIntents)) -> EffectIntents {
    let mut out = EffectIntents::new();
    let Some(audio) = AudioHelper::load(ctx, p) else {
        return out;
    };
    let gradient = working_gradient(p);
    let gradient_colors = gradient.colors_in_gradient();
    let env = MeterEnv { ctx, audio, gradient, level: p.curve("MeterIntensityCurve"), range: p.int("Range"), gradient_colors };
    for node in nodes_to_render_on(ctx, p.int("DepthOfEffect")) {
        if ctx.cancelled() {
            return out;
        }
        render_node(&env, node, &mut out);
    }
    out
}

fn nodes_to_render_on(ctx: &RenderCtx, depth: i32) -> Vec<NodeId> {
    let mut nodes: Vec<NodeId> = ctx.targets.to_vec();
    if depth != 0 {
        for _ in 0..depth {
            nodes = nodes.iter().flat_map(|n| ctx.tree.node(*n).children.clone()).collect();
        }
    }
    // Too deep: fall back to the targets' leaves.
    if nodes.is_empty() {
        nodes = ctx.targets.iter().flat_map(|t| ctx.tree.leaves(*t)).collect();
    }
    nodes
}

#[cfg(test)]
pub mod test_support {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use vixen_effects::framework::AudioSource;

    /// A synthetic mono sine wave, optionally silent for the first `silent_ms`.
    pub struct FakeAudio {
        pub samples: Vec<f32>,
        pub rate: u32,
    }

    impl FakeAudio {
        pub fn sine(rate: u32, seconds: f64, freq: f64, amplitude: f32) -> FakeAudio {
            let n = (rate as f64 * seconds) as usize;
            let samples = (0..n)
                .map(|i| amplitude * (2.0 * std::f64::consts::PI * freq * i as f64 / rate as f64).sin() as f32)
                .collect();
            FakeAudio { samples, rate }
        }

        /// A constant full-scale signal after `silent_ms` of silence.
        pub fn dc_after(rate: u32, seconds: f64, silent_ms: f64) -> FakeAudio {
            let n = (rate as f64 * seconds) as usize;
            let first = (rate as f64 * silent_ms / 1000.0) as usize;
            FakeAudio { samples: (0..n).map(|i| if i < first { 0.0 } else { 1.0 }).collect(), rate }
        }
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

    pub fn render_audio(
        d: &'static EffectDescriptor,
        tree: &ElementTree,
        targets: &[NodeId],
        ms: i64,
        audio: Option<&dyn AudioSource>,
        edit: impl FnOnce(&mut Params),
    ) -> EffectIntents {
        let cancel = AtomicBool::new(false);
        let ctx = RenderCtx { video_cache: None,
            tree,
            targets,
            start_time: TimeSpan::ZERO,
            time_span: TimeSpan::from_ms_i(ms),
            frame_ms: 50,
            marks: &[],
            audio,
            module_data_root: None,
            cancel: &cancel,
        };
        let mut p = d.new_params();
        edit(&mut p);
        if let Some(v) = d.validate {
            v(&mut p, &ctx.valid_colors());
        }
        (d.render)(&ctx, &p)
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;

    #[test]
    fn dc_signal_is_zero_db() {
        let audio = FakeAudio::dc_after(8000, 1.0, 0.0);
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let tree = ElementTree::new();
        let ctx = RenderCtx { video_cache: None,
            tree: &tree,
            targets: &[],
            start_time: TimeSpan::ZERO,
            time_span: TimeSpan::from_ms_i(1000),
            frame_ms: 50,
            marks: &[],
            audio: Some(&audio),
            module_data_root: None,
            cancel: &cancel,
        };
        let p = crate::waveform::DESCRIPTOR.new_params();
        let h = AudioHelper::load(&ctx, &p).unwrap();
        assert_eq!(h.len(), 8000);
        assert!(h.volume_at_time(500).abs() < 1e-9);
        // time beyond the effect clamps to the last sample
        assert_eq!(h.closest_sample(5000), 7999);
        assert_eq!(h.closest_sample(-3), 0);
        assert_eq!(h.closest_sample(500), 4000);
    }

    #[test]
    fn attack_limits_rise_from_silence() {
        // Silence is -80 dB, the 1.0 that follows may only rise 30 dB per AttackTime (50 ms).
        let audio = FakeAudio::dc_after(8000, 1.0, 500.0);
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let tree = ElementTree::new();
        let ctx = RenderCtx { video_cache: None,
            tree: &tree,
            targets: &[],
            start_time: TimeSpan::ZERO,
            time_span: TimeSpan::from_ms_i(1000),
            frame_ms: 50,
            marks: &[],
            audio: Some(&audio),
            module_data_root: None,
            cancel: &cancel,
        };
        let mut p = crate::waveform::DESCRIPTOR.new_params();
        p.set("Normalize", ParamValue::Bool(false));
        let h = AudioHelper::load(&ctx, &p).unwrap();
        assert!((h.volume_at_time(100) + 80.0).abs() < 1e-9);
        // 50 ms after the step it has risen 30 dB; 5 sample-widths of tolerance
        let v = h.volume_at_time(550);
        assert!((v + 50.0).abs() < 0.2, "{v}");
        assert_eq!(h.volume_at_time(900), 0.0);
    }

    #[test]
    fn filters_attenuate_out_of_band() {
        let rate = 8000;
        let low: Vec<f64> = FakeAudio::sine(rate, 0.5, 100.0, 1.0).samples.iter().map(|&s| s as f64).collect();
        let high: Vec<f64> = FakeAudio::sine(rate, 0.5, 3000.0, 1.0).samples.iter().map(|&s| s as f64).collect();
        let peak = |d: &[f64]| d[1000..3000].iter().fold(0.0_f64, |m, v| m.max(v.abs()));
        assert!(peak(&filter(300.0, rate as i32, &low, false)) > 0.7);
        assert!(peak(&filter(300.0, rate as i32, &high, false)) < 0.1);
        assert!(peak(&filter(1000.0, rate as i32, &high, true)) > 0.7);
        assert!(peak(&filter(1000.0, rate as i32, &low, true)) < 0.1);
    }

    #[test]
    fn default_gradient_positions() {
        let g = default_linear_gradient();
        assert_eq!(g.get_color_at(0.0), Color::LIME);
        assert_eq!(g.get_color_at(0.25), Color::YELLOW);
        assert_eq!(g.get_color_at(0.95), Color::RED);
        let d = working_gradient_for_style("Discrete");
        // ColorGradient's SortedList keeps one colour per position, so the doubled "discrete" points do not
        // make hard steps in the C# either. Only the ends are pinned.
        assert_eq!(d.get_color_at(0.0), Color::LIME);
        assert_eq!(d.get_color_at(1.0), Color::RED);
    }

    fn working_gradient_for_style(style: &str) -> ColorGradient {
        let mut p = crate::waveform::DESCRIPTOR.new_params();
        p.set("MeterColorStyle", ParamValue::Enum(style.to_string()));
        working_gradient(&p)
    }

    #[test]
    fn invalid_gradient_is_replaced_for_discrete_targets() {
        let mut p = crate::waveform::DESCRIPTOR.new_params();
        check_for_invalid_color_data(&mut p, &[Color::RED, Color::BLUE], true);
        assert_eq!(p.enum_name("MeterColorStyle"), "Custom");
        let g = p.gradient("MeterColorGradient");
        assert_eq!(g.get_color_at(0.0), Color::RED);
        assert_eq!(g.get_color_at(1.0), Color::BLUE);
        let mut p = crate::vu_meter::DESCRIPTOR.new_params();
        check_for_invalid_color_data(&mut p, &[Color::BLUE, Color::RED], false);
        assert_eq!(p.gradient("MeterColorGradient").colors_in_gradient(), vec![Color::BLUE]);
    }
}
