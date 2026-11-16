//! Port of Vixen.Core/Intent and Sys/EffectIntents. An effect renders ahead of time into intents;
//! at playback each element evaluates the intents active at the current time.

use crate::time::TimeSpan;
use crate::value::{CommandValue, DiscreteValue, LightingValue, RangeValue, RgbValue};
use crate::Color;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// A shared run of per-frame values. Pixel effects render one contiguous block for all elements,
/// and each element's intent points at its slice: that is far cheaper than Vixen's array-per-element.
#[derive(Clone, Debug)]
pub struct FrameArray<T> {
    pub block: Arc<[T]>,
    pub offset: u32,
    pub len: u32,
    pub frame: TimeSpan,
}

impl<T: Copy + Default> FrameArray<T> {
    pub fn new(block: Arc<[T]>, offset: usize, len: usize, frame: TimeSpan) -> FrameArray<T> {
        FrameArray { block, offset: offset as u32, len: len as u32, frame }
    }

    /// Wraps a standalone value array.
    pub fn from_vec(values: Vec<T>, frame: TimeSpan) -> FrameArray<T> {
        let len = values.len();
        FrameArray { block: Arc::from(values), offset: 0, len: len as u32, frame }
    }

    #[inline]
    pub fn values(&self) -> &[T] {
        let start = self.offset as usize;
        &self.block[start..start + self.len as usize]
    }

    /// `StaticArrayIntent.GetStateAt`: truncating index, clamped to the array.
    #[inline]
    pub fn at(&self, rel: TimeSpan) -> T {
        if self.len == 0 {
            return T::default();
        }
        let idx = (rel.total_ms() / self.frame.total_ms()) as i64;
        let last = self.len as i64 - 1;
        let idx = if idx < 0 {
            0
        } else if idx >= self.len as i64 {
            last
        } else {
            idx
        };
        self.block[self.offset as usize + idx as usize]
    }
}

#[derive(Clone, Debug)]
pub enum IntentData {
    /// `LightingIntent`.
    Lighting { start: LightingValue, end: LightingValue },
    /// `StaticLightingIntent`.
    StaticLighting(LightingValue),
    /// `RGBIntent`.
    Rgb { start: RgbValue, end: RgbValue },
    /// `StaticIntent<RGBValue>`.
    StaticRgb(RgbValue),
    /// `DiscreteLightingIntent`.
    Discrete { start: DiscreteValue, end: DiscreteValue },
    /// `StaticDiscreteLightingIntent`.
    StaticDiscrete(DiscreteValue),
    /// `StaticArrayIntent<RGBValue>`.
    ArrayRgb(FrameArray<RgbValue>),
    /// `StaticArrayIntent<LightingValue>`.
    ArrayLighting(FrameArray<LightingValue>),
    /// `StaticArrayIntent<DiscreteValue>`.
    ArrayDiscrete(FrameArray<DiscreteValue>),
    /// `RangeIntent`, for fixture functions.
    Range { start: RangeValue, end: RangeValue },
    /// `StaticIntent<RangeValue<FunctionIdentity>>`.
    StaticRange(RangeValue),
    /// `CommandIntent`: a raw command, such as a fixture index selection or a string for a device.
    Command(CommandValue),
}

/// The value of an intent at one instant (`IIntentState`).
#[derive(Clone, PartialEq, Debug)]
pub enum StateValue {
    Lighting(LightingValue),
    Rgb(RgbValue),
    Discrete(DiscreteValue),
    Range(RangeValue),
    Command(CommandValue),
}

#[derive(Clone, Debug)]
pub struct Intent {
    pub data: IntentData,
    pub span: TimeSpan,
}

/// `Interpolator.Interpolate`: percent outside 0..=1 yields None, and callers use the default value.
#[inline]
fn percent_of(rel: TimeSpan, span: TimeSpan) -> Option<f64> {
    let p = rel.ticks() as f64 / span.ticks() as f64;
    if (0.0..=1.0).contains(&p) {
        Some(p)
    } else {
        None
    }
}

/// `(byte)(start + (end - start) * percent)`.
#[inline]
fn lerp_byte(a: u8, b: u8, p: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * p) as u8
}

impl Intent {
    #[inline]
    pub fn new(data: IntentData, span: TimeSpan) -> Intent {
        Intent { data, span }
    }

    /// `IntentBuilder.CreateIntent(color, intensity, duration)`.
    pub fn static_lighting(color: Color, intensity: f64, span: TimeSpan) -> Intent {
        Intent::new(IntentData::StaticLighting(LightingValue::new(color, intensity)), span)
    }

    /// `IntentBuilder.CreateIntent(startColor, endColor, startIntensity, endIntensity, duration)`.
    pub fn lighting(start_color: Color, end_color: Color, start_i: f64, end_i: f64, span: TimeSpan) -> Intent {
        Intent::new(
            IntentData::Lighting {
                start: LightingValue::new(start_color, start_i),
                end: LightingValue::new(end_color, end_i),
            },
            span,
        )
    }

    /// `IntentBuilder.CreateDiscreteIntent(color, intensity, duration)`.
    pub fn static_discrete(color: Color, intensity: f64, span: TimeSpan) -> Intent {
        Intent::new(IntentData::StaticDiscrete(DiscreteValue::new(color, intensity)), span)
    }

    /// `IntentBuilder.CreateDiscreteIntent(color, startIntensity, endIntensity, duration)`.
    pub fn discrete(color: Color, start_i: f64, end_i: f64, span: TimeSpan) -> Intent {
        Intent::new(
            IntentData::Discrete { start: DiscreteValue::new(color, start_i), end: DiscreteValue::new(color, end_i) },
            span,
        )
    }

    /// `GetStateAt(intentRelativeTime)`.
    pub fn state_at(&self, rel: TimeSpan) -> StateValue {
        match &self.data {
            IntentData::Lighting { start, end } => StateValue::Lighting(match percent_of(rel, self.span) {
                Some(p) => {
                    let color = Color::rgb(
                        lerp_byte(start.color.r, end.color.r, p),
                        lerp_byte(start.color.g, end.color.g, p),
                        lerp_byte(start.color.b, end.color.b, p),
                    );
                    LightingValue::new(color, start.intensity + (end.intensity - start.intensity) * p)
                }
                None => LightingValue::default(),
            }),
            IntentData::StaticLighting(v) => StateValue::Lighting(match percent_of(rel, self.span) {
                Some(_) => *v,
                None => LightingValue::default(),
            }),
            IntentData::Rgb { start, end } => StateValue::Rgb(match percent_of(rel, self.span) {
                Some(p) => RgbValue {
                    r: lerp_byte(start.r, end.r, p),
                    g: lerp_byte(start.g, end.g, p),
                    b: lerp_byte(start.b, end.b, p),
                },
                None => RgbValue::default(),
            }),
            IntentData::StaticRgb(v) => StateValue::Rgb(*v),
            IntentData::Discrete { start, end } => StateValue::Discrete(match percent_of(rel, self.span) {
                Some(p) => DiscreteValue::new(start.color, start.intensity + (end.intensity - start.intensity) * p),
                None => DiscreteValue::default(),
            }),
            IntentData::StaticDiscrete(v) => StateValue::Discrete(match percent_of(rel, self.span) {
                Some(_) => *v,
                None => DiscreteValue::default(),
            }),
            IntentData::ArrayRgb(a) => StateValue::Rgb(a.at(rel)),
            IntentData::ArrayLighting(a) => StateValue::Lighting(a.at(rel)),
            IntentData::ArrayDiscrete(a) => StateValue::Discrete(a.at(rel)),
            IntentData::Range { start, end } => match percent_of(rel, self.span) {
                Some(p) => StateValue::Range(RangeValue {
                    tag_type: start.tag_type,
                    tag: start.tag.clone(),
                    value: start.value + (end.value - start.value) * p,
                    label: start.label.clone(),
                }),
                // NonSegmentedLinearIntent returns default(T), including default metadata.
                None => StateValue::Range(RangeValue::new(crate::value::FunctionIdentity::Pan, "", 0.0, "")),
            },
            IntentData::StaticRange(v) => StateValue::Range(v.clone()),
            IntentData::Command(c) => StateValue::Command(c.clone()),
        }
    }
}

/// `IntentNode`: an intent placed on a time line.
#[derive(Clone, Debug)]
pub struct IntentNode {
    pub start: TimeSpan,
    pub end: TimeSpan,
    pub intent: Intent,
}

impl IntentNode {
    #[inline]
    pub fn new(intent: Intent, start: TimeSpan) -> IntentNode {
        IntentNode { start, end: start + intent.span, intent }
    }

    /// `TimeNode.IntersectsInclusively`.
    #[inline]
    pub fn contains(&self, t: TimeSpan) -> bool {
        t >= self.start && t <= self.end
    }

    #[inline]
    pub fn offset(&mut self, by: TimeSpan) {
        self.start += by;
        self.end = self.start + self.intent.span;
    }
}

/// `EffectIntents`: intent nodes keyed by Element.Id.
#[derive(Clone, Debug, Default)]
pub struct EffectIntents {
    pub map: HashMap<Uuid, Vec<IntentNode>>,
}

impl EffectIntents {
    pub fn new() -> EffectIntents {
        EffectIntents::default()
    }

    pub fn with_capacity(n: usize) -> EffectIntents {
        EffectIntents { map: HashMap::with_capacity(n) }
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// `AddIntentForElement`.
    #[inline]
    pub fn add_intent_for_element(&mut self, element: Uuid, intent: Intent, start: TimeSpan) {
        self.map.entry(element).or_default().push(IntentNode::new(intent, start));
    }

    /// `Add(EffectIntents)`.
    pub fn add(&mut self, other: EffectIntents) {
        for (id, nodes) in other.map {
            self.map.entry(id).or_default().extend(nodes);
        }
    }

    /// `OffsetAllCommandsByTime`.
    pub fn offset_all(&mut self, by: TimeSpan) {
        for nodes in self.map.values_mut() {
            for n in nodes {
                n.offset(by);
            }
        }
    }

    /// `GetIntentNodesForElement`.
    pub fn nodes_for(&self, element: &Uuid) -> Option<&Vec<IntentNode>> {
        self.map.get(element)
    }

    /// `Restrict`.
    pub fn restrict(&self, start: TimeSpan, end: TimeSpan) -> EffectIntents {
        EffectIntents {
            map: self
                .map
                .iter()
                .map(|(k, v)| {
                    (*k, v.iter().filter(|y| !(y.start >= end || y.end < start)).cloned().collect::<Vec<_>>())
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(v: i64) -> TimeSpan {
        TimeSpan::from_ms_i(v)
    }

    #[test]
    fn lighting_intent_interpolates_colour_and_intensity() {
        let i = Intent::lighting(Color::BLACK, Color::WHITE, 0.0, 1.0, ms(100));
        match i.state_at(ms(50)) {
            StateValue::Lighting(v) => {
                assert_eq!(v.color, Color::rgb(127, 127, 127));
                assert!((v.intensity - 0.5).abs() < 1e-12);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn past_the_end_yields_the_default_value() {
        let i = Intent::lighting(Color::WHITE, Color::WHITE, 1.0, 1.0, ms(100));
        assert_eq!(i.state_at(ms(150)), StateValue::Lighting(LightingValue::default()));
    }

    #[test]
    fn invalid_range_time_returns_default_identity_and_metadata() {
        use crate::value::FunctionIdentity;
        let start = RangeValue::new(FunctionIdentity::Tilt, "Tilt channel", 0.3, "Tilt label");
        let end = RangeValue { value: 0.7, ..start.clone() };
        let default = StateValue::Range(RangeValue::new(FunctionIdentity::Pan, "", 0.0, ""));
        let data = IntentData::Range { start: start.clone(), end };
        let zero = Intent::new(data.clone(), TimeSpan::ZERO);
        assert_eq!(zero.state_at(TimeSpan::ZERO), default);
        let intent = Intent::new(data, ms(100));
        assert_eq!(intent.state_at(ms(-1)), default);
        assert_eq!(intent.state_at(ms(101)), default);
        assert_eq!(intent.state_at(TimeSpan::ZERO), StateValue::Range(start));
    }

    #[test]
    fn static_array_indexes_by_frame_and_clamps() {
        let values = vec![
            RgbValue { r: 1, g: 0, b: 0 },
            RgbValue { r: 2, g: 0, b: 0 },
            RgbValue { r: 3, g: 0, b: 0 },
        ];
        let a = FrameArray::from_vec(values, ms(50));
        assert_eq!(a.at(ms(0)).r, 1);
        assert_eq!(a.at(ms(49)).r, 1);
        assert_eq!(a.at(ms(50)).r, 2);
        assert_eq!(a.at(ms(10_000)).r, 3);
        assert_eq!(a.at(ms(-10)).r, 1);
    }

    #[test]
    fn shared_block_slices_are_independent() {
        let block: Arc<[RgbValue]> = Arc::from(vec![
            RgbValue { r: 1, g: 0, b: 0 },
            RgbValue { r: 2, g: 0, b: 0 },
            RgbValue { r: 3, g: 0, b: 0 },
            RgbValue { r: 4, g: 0, b: 0 },
        ]);
        let second = FrameArray::new(block, 2, 2, ms(50));
        assert_eq!(second.at(ms(0)).r, 3);
        assert_eq!(second.at(ms(60)).r, 4);
        assert_eq!(second.at(ms(1000)).r, 4);
    }

    #[test]
    fn an_empty_frame_array_reads_as_default() {
        let a: FrameArray<RgbValue> = FrameArray::from_vec(Vec::new(), ms(50));
        assert_eq!(a.at(ms(100)), RgbValue::BLACK);
    }

    #[test]
    fn offset_moves_nodes() {
        let mut e = EffectIntents::new();
        let id = Uuid::new_v4();
        e.add_intent_for_element(id, Intent::static_lighting(Color::RED, 1.0, ms(10)), ms(5));
        e.offset_all(ms(100));
        let n = &e.nodes_for(&id).unwrap()[0];
        assert_eq!(n.start, ms(105));
        assert_eq!(n.end, ms(115));
    }

    #[test]
    fn restrict_keeps_only_overlapping_nodes() {
        let mut e = EffectIntents::new();
        let id = Uuid::new_v4();
        e.add_intent_for_element(id, Intent::static_lighting(Color::RED, 1.0, ms(10)), ms(0));
        e.add_intent_for_element(id, Intent::static_lighting(Color::RED, 1.0, ms(10)), ms(100));
        let r = e.restrict(ms(50), ms(200));
        assert_eq!(r.nodes_for(&id).unwrap().len(), 1);
    }
}
