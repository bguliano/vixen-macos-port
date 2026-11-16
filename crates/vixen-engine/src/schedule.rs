//! Per-element schedules and frame evaluation.

use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use vixen_core::layer::LayeredState;
use vixen_core::{CombinedState, EffectIntents, ElementTree, Intent, Layer, StateCombinator, TimeSpan, Uuid};

/// Above this many elements a frame is evaluated on several threads. Below it, the coordination costs more
/// than it saves.
const PARALLEL_THRESHOLD: usize = 30_000;

/// Maps element ids to dense indices, so per-frame data can live in flat arrays.
#[derive(Clone, Debug, Default)]
pub struct ElementMap {
    pub ids: Vec<Uuid>,
    index: HashMap<Uuid, u32>,
}

impl ElementMap {
    pub fn from_tree(tree: &ElementTree) -> ElementMap {
        let mut m = ElementMap::default();
        for id in tree.element_ids() {
            m.push(id);
        }
        m
    }

    pub fn from_ids(ids: impl IntoIterator<Item = Uuid>) -> ElementMap {
        let mut m = ElementMap::default();
        for id in ids {
            m.push(id);
        }
        m
    }

    fn push(&mut self, id: Uuid) {
        let next = self.ids.len() as u32;
        if let std::collections::hash_map::Entry::Vacant(v) = self.index.entry(id) {
            v.insert(next);
            self.ids.push(id);
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    #[inline]
    pub fn index_of(&self, id: &Uuid) -> Option<u32> {
        self.index.get(id).copied()
    }
}

/// One intent placed in absolute time on one element.
#[derive(Clone, Debug)]
struct Entry {
    /// First tick the entry is active: the later of the intent's start and the effect's start.
    lo: i64,
    /// Last tick it is active (inclusive, as `TimeNode.IntersectsInclusively` is).
    hi: i64,
    /// What the intent's relative time is measured from.
    base: i64,
    layer: u16,
    effect: Uuid,
    intent: Intent,
}

/// An element's entries sorted by `lo`, with the running maximum of `hi` so a lookup can stop early.
#[derive(Clone, Debug, Default)]
struct ElementSchedule {
    entries: Vec<Entry>,
    max_hi: Vec<i64>,
}

impl ElementSchedule {
    fn build(mut entries: Vec<Entry>) -> ElementSchedule {
        entries.sort_by_key(|e| e.lo);
        let mut max_hi = Vec::with_capacity(entries.len());
        let mut m = i64::MIN;
        for e in &entries {
            m = m.max(e.hi);
            max_hi.push(m);
        }
        ElementSchedule { entries, max_hi }
    }

    /// Appends the states active at `t`, and returns whether any were.
    #[inline]
    fn collect(&self, t: i64, layers: &[LayerInfo], out: &mut Vec<LayeredState>) {
        // Everything at or after this index starts after `t`.
        let end = self.entries.partition_point(|e| e.lo <= t);
        let mut i = end;
        while i > 0 {
            i -= 1;
            // Nothing at or before `i` reaches `t`.
            if self.max_hi[i] < t {
                break;
            }
            let e = &self.entries[i];
            if e.hi >= t {
                let info = &layers[e.layer as usize];
                out.push(LayeredState {
                    value: e.intent.state_at(TimeSpan(t - e.base)),
                    level: info.level,
                    requires_partner: info.requires_partner,
                    layer_index: e.layer,
                });
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct LayerInfo {
    level: i32,
    requires_partner: bool,
}

/// A rendered effect placed on the time line.
#[derive(Clone, Debug)]
pub struct ScheduledEffect {
    pub id: Uuid,
    pub start: TimeSpan,
    pub span: TimeSpan,
    /// Index into the schedule's layer list.
    pub layer: u16,
    pub intents: Arc<EffectIntents>,
}

/// Everything needed to evaluate a frame. Immutable once built, so the frame thread reads it without locks.
/// Elements share their schedules between versions through `Arc`, so an edit that touches a few elements
/// rebuilds only those.
#[derive(Clone, Debug)]
pub struct Schedule {
    elements: Arc<ElementMap>,
    per_element: Vec<Arc<ElementSchedule>>,
    layers: Arc<Vec<Layer>>,
    layer_info: Arc<Vec<LayerInfo>>,
    /// The time span some effect occupies, used to skip whole frames in empty regions.
    range: Option<(i64, i64)>,
}

fn layer_info(layers: &[Layer]) -> Vec<LayerInfo> {
    layers.iter().map(|l| LayerInfo { level: l.level, requires_partner: l.requires_mixing_partner() }).collect()
}

fn entries_for(effect: &ScheduledEffect, element_nodes: &[vixen_core::IntentNode]) -> Vec<Entry> {
    let effect_lo = effect.start.ticks();
    let effect_hi = (effect.start + effect.span).ticks();
    element_nodes
        .iter()
        .filter_map(|n| {
            let base = effect_lo + n.start.ticks();
            let lo = base.max(effect_lo);
            let hi = (effect_lo + n.end.ticks()).min(effect_hi);
            (lo <= hi).then(|| Entry { lo, hi, base, layer: effect.layer, effect: effect.id, intent: n.intent.clone() })
        })
        .collect()
}

impl Schedule {
    /// Whether any mapped intent can contribute to a frame.
    pub fn is_empty(&self) -> bool { self.range.is_none() }

    pub fn empty(elements: Arc<ElementMap>, layers: Vec<Layer>) -> Schedule {
        let empty = Arc::new(ElementSchedule::default());
        Schedule {
            per_element: vec![empty; elements.len()],
            elements,
            layer_info: Arc::new(layer_info(&layers)),
            layers: Arc::new(layers),
            range: None,
        }
    }

    /// Builds a schedule from scratch. Intents for elements not in the map are ignored, as they are for
    /// elements that no longer exist in a C# profile.
    pub fn build(elements: Arc<ElementMap>, layers: Vec<Layer>, effects: &[ScheduledEffect]) -> Schedule {
        let mut buckets: Vec<Vec<Entry>> = vec![Vec::new(); elements.len()];
        for effect in effects {
            for (element, nodes) in &effect.intents.map {
                if let Some(i) = elements.index_of(element) {
                    buckets[i as usize].extend(entries_for(effect, nodes));
                }
            }
        }
        let per_element: Vec<Arc<ElementSchedule>> = buckets.into_iter().map(|b| Arc::new(ElementSchedule::build(b))).collect();
        let mut s = Schedule {
            per_element,
            elements,
            layer_info: Arc::new(layer_info(&layers)),
            layers: Arc::new(layers),
            range: None,
        };
        s.recompute_range();
        s
    }

    fn recompute_range(&mut self) {
        let mut range: Option<(i64, i64)> = None;
        for p in &self.per_element {
            if let (Some(first), Some(last)) = (p.entries.first(), p.max_hi.last()) {
                range = Some(match range {
                    None => (first.lo, *last),
                    Some((a, b)) => (a.min(first.lo), b.max(*last)),
                });
            }
        }
        self.range = range;
    }

    /// A new schedule with the given effects removed and others added or replaced. Only the elements those
    /// effects touch are rebuilt, so this is cheap even on a very large sequence.
    pub fn with_changes(&self, removed: &[Uuid], added: &[ScheduledEffect]) -> Schedule {
        let mut next = self.clone();
        let mut touched: HashMap<u32, Vec<Entry>> = HashMap::new();
        let removed: HashSet<Uuid> = removed.iter().copied().collect();

        if !removed.is_empty() {
            for (i, p) in self.per_element.iter().enumerate() {
                if p.entries.iter().any(|e| removed.contains(&e.effect)) {
                    let kept = p.entries.iter().filter(|e| !removed.contains(&e.effect)).cloned().collect();
                    touched.insert(i as u32, kept);
                }
            }
        }
        for effect in added {
            for (element, nodes) in &effect.intents.map {
                if let Some(i) = self.elements.index_of(element) {
                    let list = touched.entry(i).or_insert_with(|| {
                        self.per_element[i as usize]
                            .entries
                            .iter()
                            .filter(|e| !removed.contains(&e.effect))
                            .cloned()
                            .collect()
                    });
                    list.extend(entries_for(effect, nodes));
                }
            }
        }
        for (i, entries) in touched {
            next.per_element[i as usize] = Arc::new(ElementSchedule::build(entries));
        }
        next.recompute_range();
        next
    }

    /// The same schedule with different layer settings.
    pub fn with_layers(&self, layers: Vec<Layer>) -> Schedule {
        let mut next = self.clone();
        next.layer_info = Arc::new(layer_info(&layers));
        next.layers = Arc::new(layers);
        next
    }

    #[inline]
    pub fn elements(&self) -> &Arc<ElementMap> {
        &self.elements
    }

    pub fn entry_count(&self) -> usize {
        self.per_element.iter().map(|p| p.entries.len()).sum()
    }

    /// Whether anything is scheduled at `t`, so an empty region costs nothing.
    #[inline]
    pub fn any_possible_at(&self, t: TimeSpan) -> bool {
        matches!(self.range, Some((a, b)) if t.ticks() >= a && t.ticks() <= b)
    }

    /// Evaluates every element at `t` into `out`, one `CombinedState` per element. Returns whether any
    /// element has state, the C# `ElementsHaveState`.
    pub fn evaluate(&self, t: TimeSpan, scratch: &mut EvalScratch, out: &mut Vec<CombinedState>) -> bool {
        out.resize_with(self.elements.len(), CombinedState::default);
        if !self.any_possible_at(t) {
            for s in out.iter_mut() {
                s.clear();
            }
            return false;
        }
        let ticks = t.ticks();
        if out.len() >= PARALLEL_THRESHOLD {
            out.par_iter_mut()
                .with_min_len(2048)
                .zip(self.per_element.par_iter())
                .map_init(EvalScratch::default, |sc, (state, schedule)| {
                    self.evaluate_one(ticks, schedule, sc, state);
                    !state.is_empty()
                })
                .reduce(|| false, |a, b| a || b)
        } else {
            let mut any = false;
            for (state, schedule) in out.iter_mut().zip(self.per_element.iter()) {
                self.evaluate_one(ticks, schedule, scratch, state);
                any |= !state.is_empty();
            }
            any
        }
    }

    #[inline]
    fn evaluate_one(&self, t: i64, schedule: &ElementSchedule, scratch: &mut EvalScratch, state: &mut CombinedState) {
        scratch.states.clear();
        schedule.collect(t, &self.layer_info, &mut scratch.states);
        if scratch.states.is_empty() {
            state.clear();
        } else {
            scratch.combinator.combine(&mut scratch.states, &self.layers, state);
        }
    }
}

/// Reusable per-thread working storage, so evaluating a frame allocates nothing after the first few frames.
#[derive(Default)]
pub struct EvalScratch {
    states: Vec<LayeredState>,
    combinator: StateCombinator,
}

/// Several contexts feed each element's raw states into one combinator, as
/// `Element._AggregateStateFromContexts` does. Mixing already-combined RGB frames would
/// discard layer masks, discrete colors and fixture commands from the other contexts.
#[derive(Default)]
pub struct FrameMixer {
    elements: Option<Arc<ElementMap>>,
    schedules: Vec<Arc<Schedule>>,
    indices: Vec<Vec<Option<usize>>>,
    offsets: Vec<u16>,
    layers: Vec<Layer>,
    valid: bool,
}

impl FrameMixer {
    pub fn evaluate(
        &mut self,
        elements: &Arc<ElementMap>,
        frames: &[(Arc<Schedule>, TimeSpan)],
        scratch: &mut EvalScratch,
        out: &mut Vec<CombinedState>,
    ) -> bool {
        if frames.len() == 1 && frames[0].0.elements.ids == elements.ids {
            // Do not retain retired show schedules when the editor becomes the only
            // context. The fast path otherwise bypasses the cache invalidation below.
            if !self.schedules.is_empty() {
                self.schedules.clear();
                self.indices.clear();
                self.offsets.clear();
                self.layers.clear();
                self.elements = None;
            }
            return frames[0].0.evaluate(frames[0].1, scratch, out);
        }
        out.resize_with(elements.len(), CombinedState::default);
        if self.elements.as_ref().is_none_or(|old| !Arc::ptr_eq(old, elements))
            || self.schedules.len() != frames.len()
            || self.schedules.iter().zip(frames).any(|(old, (new, _))| !Arc::ptr_eq(old, new))
        {
            self.elements = Some(elements.clone());
            self.schedules.clear();
            self.indices.clear();
            self.offsets.clear();
            self.layers.clear();
            self.valid = frames.iter().map(|(s, _)| s.layers.len()).sum::<usize>() <= u16::MAX as usize + 1;
            for (schedule, _) in frames {
                self.schedules.push(schedule.clone());
                if self.valid {
                    self.offsets.push(self.layers.len() as u16);
                    self.layers.extend(schedule.layers.iter().cloned());
                    self.indices.push(elements.ids.iter().map(|id| schedule.elements.index_of(id).map(|i| i as usize)).collect());
                }
            }
        }
        if !self.valid || frames.is_empty() {
            for state in out { state.clear(); }
            return false;
        }
        let evaluate_one = |index: usize, state: &mut CombinedState, sc: &mut EvalScratch| {
            sc.states.clear();
            for (context, (schedule, time)) in frames.iter().enumerate() {
                if !schedule.any_possible_at(*time) { continue; }
                let Some(element) = self.indices[context][index] else { continue };
                let first = sc.states.len();
                schedule.per_element[element].collect(time.ticks(), &schedule.layer_info, &mut sc.states);
                for raw in &mut sc.states[first..] { raw.layer_index += self.offsets[context]; }
            }
            sc.combinator.combine(&mut sc.states, &self.layers, state);
            !state.is_empty()
        };
        if out.len() >= PARALLEL_THRESHOLD {
            out.par_iter_mut().enumerate().with_min_len(2048)
                .map_init(EvalScratch::default, |sc, (index, state)| evaluate_one(index, state, sc))
                .reduce(|| false, |a, b| a || b)
        } else {
            let mut any = false;
            for (index, state) in out.iter_mut().enumerate() { any |= evaluate_one(index, state, scratch); }
            any
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::{Color, Intent, LightingValue, MixingFilter, RgbValue, SequenceLayers};

    fn ms(v: i64) -> TimeSpan {
        TimeSpan::from_ms_i(v)
    }

    fn intents(element: Uuid, intent: Intent, start: TimeSpan) -> Arc<EffectIntents> {
        let mut e = EffectIntents::new();
        e.add_intent_for_element(element, intent, start);
        Arc::new(e)
    }

    fn effect(start: i64, span: i64, layer: u16, intents: Arc<EffectIntents>) -> ScheduledEffect {
        ScheduledEffect { id: Uuid::new_v4(), start: ms(start), span: ms(span), layer, intents }
    }

    fn map(n: usize) -> (Arc<ElementMap>, Vec<Uuid>) {
        let ids: Vec<Uuid> = (0..n).map(|_| Uuid::new_v4()).collect();
        (Arc::new(ElementMap::from_ids(ids.clone())), ids)
    }

    fn default_layers() -> Vec<Layer> {
        SequenceLayers::default().layers
    }

    fn eval(s: &Schedule, t: i64) -> Vec<CombinedState> {
        let mut out = Vec::new();
        s.evaluate(ms(t), &mut EvalScratch::default(), &mut out);
        out
    }

    #[test]
    fn active_only_inside_the_effect_inclusive_at_both_ends() {
        let (m, ids) = map(1);
        let e = effect(100, 50, 0, intents(ids[0], Intent::static_lighting(Color::RED, 1.0, ms(50)), TimeSpan::ZERO));
        let s = Schedule::build(m, default_layers(), &[e]);
        assert!(eval(&s, 99)[0].is_empty());
        assert_eq!(eval(&s, 100)[0].lighting, Some(LightingValue::new(Color::RED, 1.0)));
        assert_eq!(eval(&s, 150)[0].lighting, Some(LightingValue::new(Color::RED, 1.0)));
        assert!(eval(&s, 151)[0].is_empty());
    }

    #[test]
    fn intent_times_are_relative_to_the_effect() {
        let (m, ids) = map(1);
        let intent = Intent::lighting(Color::BLACK, Color::WHITE, 0.0, 1.0, ms(100));
        let e = effect(1000, 200, 0, intents(ids[0], intent, ms(50)));
        let s = Schedule::build(m, default_layers(), &[e]);
        assert!(eval(&s, 1049)[0].is_empty());
        let mid = eval(&s, 1100)[0].lighting.unwrap();
        assert!((mid.intensity - 0.5).abs() < 1e-9);
    }

    #[test]
    fn an_intent_past_the_end_of_its_effect_is_cut_off() {
        let (m, ids) = map(1);
        let e = effect(0, 100, 0, intents(ids[0], Intent::static_lighting(Color::RED, 1.0, ms(500)), TimeSpan::ZERO));
        let s = Schedule::build(m, default_layers(), &[e]);
        assert!(!eval(&s, 100)[0].is_empty());
        assert!(eval(&s, 101)[0].is_empty());
    }

    #[test]
    fn overlapping_effects_on_one_layer_mix_highest_wins() {
        let (m, ids) = map(1);
        let a = effect(0, 100, 0, intents(ids[0], Intent::static_lighting(Color::RED, 1.0, ms(100)), TimeSpan::ZERO));
        let b = effect(50, 100, 0, intents(ids[0], Intent::static_lighting(Color::BLUE, 1.0, ms(100)), TimeSpan::ZERO));
        let s = Schedule::build(m, default_layers(), &[a, b]);
        assert_eq!(eval(&s, 75)[0].rgb, Some(RgbValue { r: 255, g: 0, b: 255 }));
        assert_eq!(eval(&s, 25)[0].lighting.unwrap().color, Color::RED);
    }

    #[test]
    fn a_long_earlier_effect_is_still_found_behind_short_later_ones() {
        let (m, ids) = map(1);
        let long = effect(0, 1000, 0, intents(ids[0], Intent::static_lighting(Color::RED, 1.0, ms(1000)), TimeSpan::ZERO));
        let mut all = vec![long];
        for i in 0..50 {
            all.push(effect(100 + i * 2, 1, 0, intents(ids[0], Intent::static_lighting(Color::BLUE, 1.0, ms(1)), TimeSpan::ZERO)));
        }
        let s = Schedule::build(m, default_layers(), &all);
        assert_eq!(eval(&s, 900)[0].lighting.unwrap().color, Color::RED);
    }

    #[test]
    fn layers_apply_their_mixing_filter() {
        let (m, ids) = map(1);
        let mut layers = SequenceLayers::default();
        layers.add_layer(Layer::standard("Top", MixingFilter::Mask));
        let top = effect(0, 100, 0, intents(ids[0], Intent::static_lighting(Color::WHITE, 1.0, ms(100)), TimeSpan::ZERO));
        let bottom = effect(0, 100, 1, intents(ids[0], Intent::static_lighting(Color::RED, 1.0, ms(100)), TimeSpan::ZERO));
        let s = Schedule::build(m, layers.layers, &[top, bottom]);
        assert_eq!(eval(&s, 50)[0].rgb, Some(RgbValue::BLACK));
    }

    #[test]
    fn contexts_mix_raw_layers_and_resolve_each_elements_own_clock() {
        let (elements, ids) = map(2);
        let mut upper = SequenceLayers::default();
        upper.add_layer(Layer::standard("Mask", MixingFilter::Mask));
        let mask = Arc::new(Schedule::build(elements.clone(), upper.layers, &[
            effect(100, 100, 0, intents(ids[0], Intent::static_lighting(Color::WHITE, 1.0, ms(100)), TimeSpan::ZERO))
        ]));
        let reversed = Arc::new(ElementMap::from_ids([ids[1], ids[0]]));
        let base = Arc::new(Schedule::build(reversed, default_layers(), &[
            effect(0, 100, 0, intents(ids[0], Intent::static_lighting(Color::RED, 1.0, ms(100)), TimeSpan::ZERO)),
            effect(0, 100, 0, intents(ids[1], Intent::static_lighting(Color::BLUE, 1.0, ms(100)), TimeSpan::ZERO))
        ]));
        let mut mixer = FrameMixer::default();
        let mut scratch = EvalScratch::default();
        let mut out = Vec::new();
        assert!(mixer.evaluate(&elements, &[(mask.clone(), ms(150)), (base.clone(), ms(50))], &mut scratch, &mut out));
        assert_eq!(out[0].rgb, Some(RgbValue::BLACK), "the other context supplies the mask's mixing partner");
        assert_eq!(out[1].lighting.unwrap().color, Color::BLUE, "element UUIDs, not dense indices, align contexts");
        mixer.evaluate(&elements, &[(mask, ms(250)), (base.clone(), ms(50))], &mut scratch, &mut out);
        assert_eq!(out[0].lighting.unwrap().color, Color::RED, "each context advances on its own clock");
        mixer.evaluate(&elements, &[(base, ms(150))], &mut scratch, &mut out);
        assert!(out.iter().all(CombinedState::is_empty));
    }

    #[test]
    fn same_level_contexts_use_highest_color_and_clear_on_removal() {
        let (elements, ids) = map(1);
        let build = |color| Arc::new(Schedule::build(elements.clone(), default_layers(), &[
            effect(0, 100, 0, intents(ids[0], Intent::static_lighting(color, 1.0, ms(100)), TimeSpan::ZERO))
        ]));
        let mut mixer = FrameMixer::default();
        let mut scratch = EvalScratch::default();
        let mut out = Vec::new();
        mixer.evaluate(&elements, &[(build(Color::RED), ms(20)), (build(Color::BLUE), ms(60))], &mut scratch, &mut out);
        assert_eq!(out[0].rgb, Some(RgbValue::new(Color::rgb(255, 0, 255))));
        assert!(!mixer.evaluate(&elements, &[], &mut scratch, &mut out));
        assert!(out[0].is_empty());
    }

    #[test]
    fn returning_to_one_context_releases_retired_schedules() {
        let (elements, _) = map(1);
        let kept = Arc::new(Schedule::empty(elements.clone(), default_layers()));
        let retired = Arc::new(Schedule::empty(elements.clone(), default_layers()));
        let weak = Arc::downgrade(&retired);
        let mut mixer = FrameMixer::default();
        let mut scratch = EvalScratch::default();
        let mut out = Vec::new();
        mixer.evaluate(&elements, &[(kept.clone(), ms(0)), (retired, ms(0))], &mut scratch, &mut out);
        mixer.evaluate(&elements, &[(kept, ms(0))], &mut scratch, &mut out);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn replacing_an_effect_only_changes_what_it_touched() {
        let (m, ids) = map(3);
        let a = effect(0, 100, 0, intents(ids[0], Intent::static_lighting(Color::RED, 1.0, ms(100)), TimeSpan::ZERO));
        let b = effect(0, 100, 0, intents(ids[1], Intent::static_lighting(Color::BLUE, 1.0, ms(100)), TimeSpan::ZERO));
        let s = Schedule::build(m, default_layers(), &[a.clone(), b.clone()]);
        let untouched = s.per_element[1].clone();

        let c = effect(0, 100, 0, intents(ids[0], Intent::static_lighting(Color::LIME, 1.0, ms(100)), TimeSpan::ZERO));
        let next = s.with_changes(&[a.id], std::slice::from_ref(&c));

        assert_eq!(eval(&next, 50)[0].lighting.unwrap().color, Color::LIME);
        assert_eq!(eval(&next, 50)[1].lighting.unwrap().color, Color::BLUE);
        assert!(Arc::ptr_eq(&next.per_element[1], &untouched), "an untouched element must be shared, not rebuilt");
        // The original stays usable, as the frame thread may still be reading it.
        assert_eq!(eval(&s, 50)[0].lighting.unwrap().color, Color::RED);
    }

    #[test]
    fn removing_an_effect_clears_its_elements() {
        let (m, ids) = map(1);
        let a = effect(0, 100, 0, intents(ids[0], Intent::static_lighting(Color::RED, 1.0, ms(100)), TimeSpan::ZERO));
        let s = Schedule::build(m, default_layers(), std::slice::from_ref(&a));
        assert!(!s.with_changes(&[a.id], &[]).any_possible_at(ms(50)));
    }

    #[test]
    fn unknown_elements_are_ignored() {
        let (m, _) = map(1);
        let e = effect(0, 100, 0, intents(Uuid::new_v4(), Intent::static_lighting(Color::RED, 1.0, ms(100)), TimeSpan::ZERO));
        let s = Schedule::build(m, default_layers(), &[e]);
        assert_eq!(s.entry_count(), 0);
    }

    #[test]
    fn evaluate_reports_whether_anything_has_state() {
        let (m, ids) = map(2);
        let e = effect(10, 10, 0, intents(ids[0], Intent::static_lighting(Color::RED, 1.0, ms(10)), TimeSpan::ZERO));
        let s = Schedule::build(m, default_layers(), &[e]);
        let mut out = Vec::new();
        let mut sc = EvalScratch::default();
        assert!(!s.evaluate(ms(0), &mut sc, &mut out));
        assert!(s.evaluate(ms(15), &mut sc, &mut out));
        assert!(!s.evaluate(ms(25), &mut sc, &mut out));
        assert!(out.iter().all(|x| x.is_empty()), "leaving an active region must clear earlier state");
    }

    #[test]
    fn parallel_and_serial_paths_agree() {
        let n = PARALLEL_THRESHOLD + 100;
        let (m, ids) = map(n);
        let effects: Vec<_> = (0..n)
            .step_by(7)
            .map(|i| effect(0, 100, 0, intents(ids[i], Intent::static_lighting(Color::RED, 1.0, ms(100)), TimeSpan::ZERO)))
            .collect();
        let s = Schedule::build(m, default_layers(), &effects);
        let mut out = Vec::new();
        assert!(s.evaluate(ms(50), &mut EvalScratch::default(), &mut out));
        for (i, state) in out.iter().enumerate() {
            assert_eq!(!state.is_empty(), i % 7 == 0, "{i}");
        }
    }
}
