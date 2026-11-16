//! Port of Vixen.Core/Sys/LayerMixing, the LayerMixingFilter modules, and
//! Data/StateCombinator/LayeredStateCombinator.

use crate::color::Hsv;
use crate::intent::StateValue;
use crate::value::{DiscreteValue, LightingValue, RgbValue};
use crate::Color;
use smallvec::SmallVec;
use std::collections::HashMap;
use uuid::{uuid, Uuid};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum LayerType {
    Default,
    Standard,
}

/// The layer mixing filter modules, with their stored settings.
#[derive(Clone, PartialEq, Debug)]
pub enum MixingFilter {
    ProportionalMix,
    HighestValue,
    Mask,
    MaskFill { exclude_zero_values: bool, requires_mixing_partner: bool },
    MultiplyColor,
    IntensityOverlay,
    ColorChange,
    LumaKey { lower_limit: f64, upper_limit: f64 },
    ChromaKey(ChromaKeySettings),
}

#[derive(Clone, PartialEq, Debug)]
pub struct ChromaKeySettings {
    pub lower_limit: f64,
    pub upper_limit: f64,
    pub key_color: Color,
    pub key_saturation: f64,
    pub key_hue: f64,
    pub hue_tolerance: f64,
    pub saturation_tolerance: f64,
    pub transparent_on_zero_brightness: bool,
}

impl Default for ChromaKeySettings {
    fn default() -> Self {
        ChromaKeySettings {
            lower_limit: 0.0,
            upper_limit: 1.0,
            key_color: Color::LIME,
            key_saturation: 1.0,
            key_hue: 120.0,
            hue_tolerance: 5.0,
            saturation_tolerance: 0.05,
            transparent_on_zero_brightness: false,
        }
    }
}

impl MixingFilter {
    /// `IntensityOverlay`, `ColorChange` and `ChromaKey` only produce light where a lower layer does.
    pub fn requires_mixing_partner(&self) -> bool {
        match self {
            MixingFilter::IntensityOverlay | MixingFilter::ColorChange | MixingFilter::ChromaKey(_) => true,
            MixingFilter::MaskFill { requires_mixing_partner, .. } => *requires_mixing_partner,
            _ => false,
        }
    }

    /// The module TypeId from each filter's descriptor.
    pub fn type_id(&self) -> Uuid {
        match self {
            MixingFilter::ProportionalMix => filter_ids::PROPORTIONAL_MIX,
            MixingFilter::HighestValue => filter_ids::HIGHEST_VALUE,
            MixingFilter::Mask => filter_ids::MASK,
            MixingFilter::MaskFill { .. } => filter_ids::MASK_FILL,
            MixingFilter::MultiplyColor => filter_ids::MULTIPLY_COLOR,
            MixingFilter::IntensityOverlay => filter_ids::INTENSITY_OVERLAY,
            MixingFilter::ColorChange => filter_ids::COLOR_CHANGE,
            MixingFilter::LumaKey { .. } => filter_ids::LUMA_KEY,
            MixingFilter::ChromaKey(_) => filter_ids::CHROMA_KEY,
        }
    }

    /// `CombineFullColor`.
    pub fn combine_full_color(&self, high: Color, low: Color) -> Color {
        match self {
            MixingFilter::HighestValue => Color::rgb(high.r.max(low.r), high.g.max(low.g), high.b.max(low.b)),
            MixingFilter::ProportionalMix => {
                let mut hsv = Hsv::from_color(low);
                hsv.set_v(hsv.v * (1.0 - Hsv::v_from_color(high)));
                high.combine(hsv.to_color())
            }
            MixingFilter::Mask => {
                if Hsv::v_from_color(high) > 0.0 {
                    Color::EMPTY
                } else {
                    low
                }
            }
            MixingFilter::MaskFill { exclude_zero_values, .. } => {
                if Hsv::v_from_color(high) > 0.0 || !exclude_zero_values {
                    high
                } else {
                    low
                }
            }
            MixingFilter::MultiplyColor => {
                let conv = |h: u8, l: u8| -> i32 {
                    let (h, l) = (h as i32, l as i32);
                    let v = if h > 0 { l as f64 * (1.0 - (h as f64 / 255.0)) } else { l as f64 };
                    v as i32
                };
                Color::from_rgb_i32(conv(high.r, low.r), conv(high.g, low.g), conv(high.b, low.b))
            }
            MixingFilter::IntensityOverlay => {
                let new_v = Hsv::v_from_color(high);
                let mut c = Hsv::from_color(low);
                c.set_v(c.v * new_v);
                c.to_color()
            }
            MixingFilter::ColorChange => {
                let i = Hsv::v_from_color(low);
                let mut c = Hsv::from_color(high);
                c.set_v(i * c.v);
                c.to_color()
            }
            MixingFilter::LumaKey { lower_limit, upper_limit } => {
                let v = round2(Hsv::v_from_color(low));
                if v >= *lower_limit && v <= *upper_limit {
                    high
                } else {
                    low
                }
            }
            MixingFilter::ChromaKey(d) => chroma_key(d, high, low),
        }
    }

    /// `CombineDiscreteIntensity`.
    pub fn combine_discrete(&self, mut high: DiscreteValue, mut low: DiscreteValue) -> DiscreteValue {
        match self {
            MixingFilter::HighestValue => {
                if high.intensity > low.intensity {
                    high
                } else {
                    low
                }
            }
            MixingFilter::ProportionalMix => {
                let i = low.intensity * (1.0 - high.intensity);
                high.intensity = i.max(high.intensity);
                high
            }
            MixingFilter::Mask => {
                if high.intensity > 0.0 {
                    low.intensity = 0.0;
                }
                low
            }
            MixingFilter::MaskFill { exclude_zero_values, .. } => {
                if high.intensity > 0.0 || !exclude_zero_values {
                    high
                } else {
                    low
                }
            }
            MixingFilter::MultiplyColor | MixingFilter::IntensityOverlay => {
                low.intensity *= high.intensity;
                low
            }
            MixingFilter::ColorChange => {
                high.intensity = low.intensity;
                high
            }
            MixingFilter::LumaKey { lower_limit, upper_limit } => {
                if low.intensity >= *lower_limit && low.intensity <= *upper_limit {
                    high
                } else {
                    low
                }
            }
            MixingFilter::ChromaKey(_) => low,
        }
    }
}

pub mod filter_ids {
    use super::*;
    pub const PROPORTIONAL_MIX: Uuid = uuid!("1f16d67a-1fff-485c-8e76-27ff7e7d072b");
    pub const HIGHEST_VALUE: Uuid = uuid!("01d6d526-9586-44bb-82f4-10abacf58b61");
    pub const MASK: Uuid = uuid!("ace0f097-7982-4edc-b5dd-8989dce60e1d");
    pub const MASK_FILL: Uuid = uuid!("24878b18-14f8-458e-abaa-3a9c9a34931c");
    pub const MULTIPLY_COLOR: Uuid = uuid!("0c9dfc0b-8e39-4167-8036-79408c6b0b35");
    pub const INTENSITY_OVERLAY: Uuid = uuid!("43f48bf5-4088-403f-ad7a-d0ff2114ca31");
    pub const COLOR_CHANGE: Uuid = uuid!("93d4169e-5138-403a-b71c-a5a1ab089801");
    pub const LUMA_KEY: Uuid = uuid!("7583f7d4-df99-4492-8d10-257c2677ff7b");
    pub const CHROMA_KEY: Uuid = uuid!("2d467733-1a64-45ad-bcdc-84bf074747e6");
}

/// `Math.Round(v, 2)` with banker's rounding.
#[inline]
fn round2(v: f64) -> f64 {
    (v * 100.0).round_ties_even() / 100.0
}

fn chroma_key(d: &ChromaKeySettings, high: Color, low: Color) -> Color {
    if d.transparent_on_zero_brightness && high.brightness() == 0.0 {
        return low;
    }
    let low_v = round2(Hsv::v_from_color(low));
    if !(low_v >= d.lower_limit && low_v <= d.upper_limit) {
        return low;
    }
    let low_s = round2(Hsv::from_color(low).s);
    if !(low_s <= d.key_saturation + d.saturation_tolerance && low_s >= d.key_saturation - d.saturation_tolerance) {
        return low;
    }
    let low_h = low.hue() as f64;
    if low_h - d.hue_tolerance > 0.0
        && low_h + d.hue_tolerance < 360.0
        && low_h >= d.key_hue - d.hue_tolerance
        && low_h <= d.key_hue + d.hue_tolerance
    {
        return high;
    }
    if d.key_hue - d.hue_tolerance <= 0.0
        && (low_h >= d.key_hue - d.hue_tolerance + 360.0 || low_h <= d.key_hue + d.hue_tolerance)
    {
        return high;
    }
    if d.key_hue + d.hue_tolerance >= 360.0
        && low_h >= d.key_hue - d.hue_tolerance
        && low_h <= d.key_hue + d.hue_tolerance - 360.0
    {
        return high;
    }
    low
}

#[derive(Clone, Debug)]
pub struct Layer {
    pub id: Uuid,
    pub layer_type: LayerType,
    pub level: i32,
    pub name: String,
    /// `None` only on the default layer.
    pub filter: Option<MixingFilter>,
    /// Instance id of the mixing filter module, kept so files round-trip.
    pub filter_instance_id: Uuid,
}

impl Layer {
    pub fn default_layer() -> Layer {
        Layer {
            id: Uuid::new_v4(),
            layer_type: LayerType::Default,
            level: 0,
            name: "Default".to_string(),
            filter: None,
            filter_instance_id: Uuid::nil(),
        }
    }

    pub fn standard(name: &str, filter: MixingFilter) -> Layer {
        Layer {
            id: Uuid::new_v4(),
            layer_type: LayerType::Standard,
            level: 0,
            name: name.to_string(),
            filter: Some(filter),
            filter_instance_id: Uuid::new_v4(),
        }
    }

    /// `Layer.RequiresMixingPartner`.
    #[inline]
    pub fn requires_mixing_partner(&self) -> bool {
        self.layer_type == LayerType::Standard && self.filter.as_ref().is_some_and(|f| f.requires_mixing_partner())
    }
}

/// `SequenceLayers`. `layers[0]` is the top of the stack and the highest level.
#[derive(Clone, Debug)]
pub struct SequenceLayers {
    pub layers: Vec<Layer>,
    pub effect_layer_map: HashMap<Uuid, Uuid>,
}

impl Default for SequenceLayers {
    fn default() -> Self {
        SequenceLayers { layers: vec![Layer::default_layer()], effect_layer_map: HashMap::new() }
    }
}

impl SequenceLayers {
    /// `UpdateLevels`: the bottom of the list is level 0.
    pub fn update_levels(&mut self) {
        let n = self.layers.len() as i32;
        for (i, l) in self.layers.iter_mut().enumerate() {
            l.level = n - 1 - i as i32;
        }
    }

    pub fn add_layer(&mut self, layer: Layer) {
        self.layers.insert(0, layer);
        self.update_levels();
    }

    pub fn default_layer_id(&self) -> Uuid {
        self.layers
            .iter()
            .find(|l| l.layer_type == LayerType::Default)
            .map(|l| l.id)
            .unwrap_or_else(Uuid::nil)
    }

    pub fn layer(&self, id: &Uuid) -> Option<&Layer> {
        self.layers.iter().find(|l| &l.id == id)
    }

    /// `GetLayer(effectNode)`: unassigned effects sit on the default layer.
    pub fn layer_index_for_effect(&self, effect: &Uuid) -> usize {
        let id = self.effect_layer_map.get(effect).copied();
        id.and_then(|id| self.layers.iter().position(|l| l.id == id))
            .or_else(|| self.layers.iter().position(|l| l.layer_type == LayerType::Default))
            .unwrap_or(0)
    }

    pub fn assign_effect(&mut self, effect: Uuid, layer: Uuid) {
        self.effect_layer_map.insert(effect, layer);
    }
}

/// One element's state after layers are mixed: the output of `LayeredStateCombinator.Combine`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CombinedState {
    /// A lone lighting state passes through unconverted, as `Combine` returns its input when there
    /// is nothing to mix. Colour breakdown and the preview treat it differently from RGB.
    pub lighting: Option<LightingValue>,
    /// The mixed full-colour value, when any layer produced one.
    pub rgb: Option<RgbValue>,
    /// One entry per discrete colour.
    pub discrete: SmallVec<[DiscreteValue; 4]>,
    /// Fixture range and command values, which pass through unmixed.
    pub other: SmallVec<[StateValue; 2]>,
}

impl CombinedState {
    pub fn clear(&mut self) {
        self.lighting = None;
        self.rgb = None;
        self.discrete.clear();
        self.other.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.lighting.is_none() && self.rgb.is_none() && self.discrete.is_empty() && self.other.is_empty()
    }
}

/// A state tagged with where its layer sits, the combinator's input.
#[derive(Clone, Debug)]
pub struct LayeredState {
    pub value: StateValue,
    pub level: i32,
    pub requires_partner: bool,
    /// Index into the layer list, used to find the mixing filter.
    pub layer_index: u16,
}

/// Reusable working storage for `LayeredStateCombinator`, so per-frame mixing allocates nothing.
#[derive(Default)]
pub struct StateCombinator {
    sorted: Vec<LayeredState>,
    temp_discrete: SmallVec<[(i32, DiscreteValue); 4]>,
    combined_discrete: SmallVec<[(i32, DiscreteValue); 4]>,
}

impl StateCombinator {
    /// `Combine`. `states` is consumed in place and left empty.
    pub fn combine(&mut self, states: &mut Vec<LayeredState>, layers: &[Layer], out: &mut CombinedState) {
        out.clear();
        if states.len() <= 1 {
            if let Some(s) = states.first() {
                if !s.requires_partner {
                    push_single(&s.value, out);
                }
            }
            states.clear();
            return;
        }
        if states.iter().all(|s| s.requires_partner) {
            states.clear();
            return;
        }

        std::mem::swap(states, &mut self.sorted);
        states.clear();
        // List.Sort is unstable in .NET; within one level the mix is commutative for the
        // highest-wins rule, so a stable sort gives the same result.
        self.sorted.sort_by(|a, b| b.level.cmp(&a.level));

        let mut temp_mix: Option<Color> = None;
        let mut combined_mix: Option<Color> = None;
        self.temp_discrete.clear();
        self.combined_discrete.clear();

        let mut current_level = self.sorted[0].level;
        let mut current_index = self.sorted[0].layer_index;
        let mut filter: Option<&MixingFilter> = None;

        for i in 0..self.sorted.len() {
            let level = self.sorted[i].level;
            let index = self.sorted[i].layer_index;
            if level != current_level {
                mix_layers(&mut temp_mix, &mut combined_mix, &mut self.temp_discrete, &mut self.combined_discrete, filter);
                filter = layers.get(current_index as usize).and_then(|l| l.filter.as_ref());
                current_level = level;
                current_index = index;
            }
            handle(&self.sorted[i].value, &mut temp_mix, &mut self.temp_discrete, out);
        }
        mix_layers(&mut temp_mix, &mut combined_mix, &mut self.temp_discrete, &mut self.combined_discrete, filter);

        if let Some(c) = combined_mix {
            out.rgb = Some(RgbValue::new(c));
        }
        for (_, v) in self.combined_discrete.drain(..) {
            out.discrete.push(v);
        }
        self.sorted.clear();
    }
}

fn push_single(v: &StateValue, out: &mut CombinedState) {
    match v {
        StateValue::Lighting(l) => out.lighting = Some(*l),
        StateValue::Rgb(r) => out.rgb = Some(*r),
        StateValue::Discrete(d) => out.discrete.push(*d),
        other => out.other.push(other.clone()),
    }
}

/// The per-type `Handle` methods.
fn handle(
    v: &StateValue,
    temp_mix: &mut Option<Color>,
    temp_discrete: &mut SmallVec<[(i32, DiscreteValue); 4]>,
    out: &mut CombinedState,
) {
    match v {
        StateValue::Lighting(l) => {
            let c = l.full_color();
            *temp_mix = Some(match temp_mix {
                None => c,
                Some(t) => t.combine(c),
            });
        }
        StateValue::Rgb(r) => {
            let c = r.full_color();
            *temp_mix = Some(match temp_mix {
                None => c,
                Some(t) => t.combine(c),
            });
        }
        StateValue::Discrete(d) => {
            let key = d.color.to_argb();
            if let Some(e) = temp_discrete.iter_mut().find(|x| x.0 == key) {
                if e.1.intensity < d.intensity {
                    e.1 = *d;
                }
            } else {
                temp_discrete.push((key, *d));
            }
        }
        other => out.other.push(other.clone()),
    }
}

fn mix_layers(
    temp_mix: &mut Option<Color>,
    combined_mix: &mut Option<Color>,
    temp_discrete: &mut SmallVec<[(i32, DiscreteValue); 4]>,
    combined_discrete: &mut SmallVec<[(i32, DiscreteValue); 4]>,
    filter: Option<&MixingFilter>,
) {
    if let Some(temp) = *temp_mix {
        *combined_mix = Some(match (*combined_mix, filter) {
            (None, _) => temp,
            (Some(c), Some(f)) => f.combine_full_color(c, temp),
            // The default layer has no filter and is always at the bottom. Only a corrupted
            // layer stack can reach this, so fall back to the default mixing behaviour.
            (Some(c), None) => MixingFilter::ProportionalMix.combine_full_color(c, temp),
        });
    }
    if !temp_discrete.is_empty() {
        if combined_discrete.is_empty() {
            combined_discrete.extend(temp_discrete.iter().copied());
        } else {
            mix_discrete(combined_discrete, temp_discrete, filter);
        }
        temp_discrete.clear();
    }
    // The C# resets this to Color.Empty, not null. A later layer holding only discrete values
    // therefore still mixes an "empty" colour here, which is reproduced deliberately.
    *temp_mix = Some(Color::EMPTY);
}

fn mix_discrete(
    high: &mut SmallVec<[(i32, DiscreteValue); 4]>,
    low: &SmallVec<[(i32, DiscreteValue); 4]>,
    filter: Option<&MixingFilter>,
) {
    let fallback = MixingFilter::ProportionalMix;
    let filter = filter.unwrap_or(&fallback);
    for (key, value) in low.iter() {
        if filter.requires_mixing_partner() {
            high.retain(|h| low.iter().any(|l| l.0 == h.0));
        }
        if let Some(pos) = high.iter().position(|h| h.0 == *key) {
            let v = filter.combine_discrete(high[pos].1, *value);
            if v.intensity > 0.0 {
                high[pos].1 = v;
            } else {
                high.remove(pos);
            }
        } else {
            high.push((*key, *value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(top: MixingFilter) -> Vec<Layer> {
        let mut layers = SequenceLayers::default();
        layers.add_layer(Layer::standard("Top", top));
        layers.layers
    }

    fn state(v: StateValue, layers: &[Layer], index: usize) -> LayeredState {
        LayeredState {
            value: v,
            level: layers[index].level,
            requires_partner: layers[index].requires_mixing_partner(),
            layer_index: index as u16,
        }
    }

    fn lit(c: Color) -> StateValue {
        StateValue::Lighting(LightingValue::new(c, 1.0))
    }

    #[test]
    fn single_state_passes_through() {
        let layers = stack(MixingFilter::ProportionalMix);
        let mut comb = StateCombinator::default();
        let mut out = CombinedState::default();
        let mut s = vec![state(lit(Color::RED), &layers, 1)];
        comb.combine(&mut s, &layers, &mut out);
        assert_eq!(out.lighting, Some(LightingValue::new(Color::RED, 1.0)));
        assert_eq!(out.rgb, None);
    }

    #[test]
    fn partner_only_layer_with_no_partner_is_dark() {
        let layers = stack(MixingFilter::IntensityOverlay);
        let mut comb = StateCombinator::default();
        let mut out = CombinedState::default();
        let mut s = vec![state(lit(Color::RED), &layers, 0)];
        comb.combine(&mut s, &layers, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn same_layer_values_combine_highest_wins() {
        let layers = stack(MixingFilter::ProportionalMix);
        let mut comb = StateCombinator::default();
        let mut out = CombinedState::default();
        let mut s = vec![state(lit(Color::RED), &layers, 1), state(lit(Color::BLUE), &layers, 1)];
        comb.combine(&mut s, &layers, &mut out);
        assert_eq!(out.rgb, Some(RgbValue { r: 255, g: 0, b: 255 }));
    }

    #[test]
    fn mask_layer_blocks_the_layer_below() {
        let layers = stack(MixingFilter::Mask);
        let mut comb = StateCombinator::default();
        let mut out = CombinedState::default();
        let mut s = vec![state(lit(Color::WHITE), &layers, 0), state(lit(Color::RED), &layers, 1)];
        comb.combine(&mut s, &layers, &mut out);
        // Mask returns Color.Empty, and new RGBValue(Empty) is black.
        assert_eq!(out.rgb, Some(RgbValue::BLACK));
    }

    #[test]
    fn highest_value_mixes_two_layers() {
        let layers = stack(MixingFilter::HighestValue);
        let mut comb = StateCombinator::default();
        let mut out = CombinedState::default();
        let mut s = vec![state(lit(Color::RED), &layers, 0), state(lit(Color::BLUE), &layers, 1)];
        comb.combine(&mut s, &layers, &mut out);
        assert_eq!(out.rgb, Some(RgbValue { r: 255, g: 0, b: 255 }));
    }

    #[test]
    fn discrete_keeps_each_colour_separately() {
        let layers = stack(MixingFilter::ProportionalMix);
        let mut comb = StateCombinator::default();
        let mut out = CombinedState::default();
        let d = |c, i| StateValue::Discrete(DiscreteValue::new(c, i));
        let mut s = vec![
            state(d(Color::RED, 0.5), &layers, 1),
            state(d(Color::RED, 0.9), &layers, 1),
            state(d(Color::BLUE, 0.2), &layers, 1),
        ];
        comb.combine(&mut s, &layers, &mut out);
        assert_eq!(out.discrete.len(), 2);
        let red = out.discrete.iter().find(|v| v.color == Color::RED).unwrap();
        assert_eq!(red.intensity, 0.9);
    }

    #[test]
    fn update_levels_counts_up_from_the_bottom() {
        let mut l = SequenceLayers::default();
        l.add_layer(Layer::standard("A", MixingFilter::Mask));
        l.add_layer(Layer::standard("B", MixingFilter::Mask));
        assert_eq!(l.layers[0].level, 2);
        assert_eq!(l.layers[2].level, 0);
        assert_eq!(l.layers[2].layer_type, LayerType::Default);
    }

    #[test]
    fn unassigned_effects_land_on_the_default_layer() {
        let mut l = SequenceLayers::default();
        l.add_layer(Layer::standard("A", MixingFilter::Mask));
        assert_eq!(l.layer_index_for_effect(&Uuid::new_v4()), 1);
    }
}
