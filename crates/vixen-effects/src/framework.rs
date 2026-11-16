//! Effect descriptors, parameters and the render context: the Rust form of `EffectModuleDescriptorBase`,
//! `EffectModuleInstanceBase`, `BaseEffect`, and the attribute metadata the effect editor reads.

pub use vixen_video::cache::{CacheContext as VideoCacheContext, CacheSession as VideoCacheSession};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use vixen_core::color::Rgb;
use vixen_core::element::ElementColorType;
use vixen_core::{
    Color, ColorGradient, Curve, EffectIntents, ElementTree, GradientLevelPair, Intent, MarkCollection, NodeId, TimeSpan,
    Uuid,
};

/// `EffectGroups`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum EffectGroup {
    Basic,
    Pixel,
    Device,
    IntelligentFixture,
}

/// One member of an enum-typed parameter.
#[derive(Copy, Clone, Debug)]
pub struct EnumMember {
    /// The C# member name, which is also what DataContractSerializer writes.
    pub name: &'static str,
    /// `[Description]` text shown in the editor's drop-down, or the member name if it has none.
    pub label: &'static str,
}

#[derive(Debug)]
pub struct EnumDesc {
    pub name: &'static str,
    pub members: &'static [EnumMember],
}

/// What a parameter holds and how its XML is written.
#[derive(Copy, Clone, Debug)]
pub enum ParamKind {
    Bool,
    Int,
    Long,
    Double,
    Str,
    Enum(&'static EnumDesc),
    Curve,
    Gradient,
    GradientList,
    GradientLevels,
    /// System.Drawing.Color
    Color,
    /// List&lt;Color&gt;
    ColorList,
    /// ColorModels.RGB, as SetLevel stores it.
    Rgb,
    /// A 0..1 value shown as a percentage.
    Percentage,
    /// SerializableFont
    Font,
    Guid,
    Time,
    /// A nested `[DataContract]` object with its own table, such as a Whirlpool's settings.
    Record(&'static RecordDesc),
    /// A list of nested objects, such as a Wave effect's wave forms.
    RecordList(&'static RecordDesc),
    /// A type this table does not model. The value stays as raw XML so a load and save keeps it.
    Other(&'static str),
}

/// The contract and members of a nested data object.
#[derive(Debug)]
pub struct RecordDesc {
    /// `[DataContract]` type name, also the list item element name.
    pub xml_type: &'static str,
    /// CLR namespace, after `http://schemas.datacontract.org/2004/07/`.
    pub xml_ns: &'static str,
    pub params: &'static [ParamDesc],
    /// `UpdateAttributes` for one row, where visibility differs from row to row.
    pub visibility: Option<RecordVisibilityFn>,
}

/// `[PropertyEditor]` plus `[NumberRange]`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Editor {
    /// Chosen by parameter type, as the WPF property grid does.
    Default,
    /// `[NumberRange]` with no slider: a numeric field.
    Number { min: f64, max: f64, step: f64 },
    Slider { min: f64, max: f64, step: f64 },
    SliderDouble { min: f64, max: f64, step: f64 },
    /// `LevelEditor`: a 0..1 value on a 0..100 slider that snaps to whole steps.
    Level,
    /// The slider the grid gives a `Percentage` property: a 0..1 value on a 0..100 slider that doesn't snap.
    Percent,
    Selection,
    Label,
    /// A file picker for the given kind ("Image", "Video", "Svg", "Gled", "File").
    Path(&'static str),
}

/// The conversion a C# property applies between its stored value and what the editor shows. Integer arithmetic,
/// as in the setters: `Times(10)` shows `stored * 10` and stores `shown / 10`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Display {
    Stored,
    Times(i32),
    /// Shows and stores `k - value`.
    Minus(i32),
}

impl Display {
    pub fn shown(self, stored: i32) -> i32 {
        match self {
            Display::Stored => stored,
            Display::Times(k) => stored * k,
            Display::Minus(k) => k - stored,
        }
    }

    pub fn stored(self, shown: i32) -> i32 {
        match self {
            Display::Stored => shown,
            Display::Times(k) => shown / k,
            Display::Minus(k) => k - shown,
        }
    }
}

/// One effect property: C# attributes plus its `[DataMember]`.
#[derive(Debug)]
pub struct ParamDesc {
    /// C# property name, and the key render code looks values up by.
    pub name: &'static str,
    /// `[DataMember]` name in the data class. Empty for computed properties that are not stored.
    pub xml: &'static str,
    /// CLR namespace of the class that declares the member, used when writing XML.
    pub xml_ns: &'static str,
    /// Class hierarchy depth of the declaring class. DataContractSerializer writes base members first.
    pub xml_level: u8,
    pub kind: ParamKind,
    pub category: &'static str,
    pub category_order: i32,
    pub label: &'static str,
    pub description: &'static str,
    pub order: i32,
    /// Initial `[Browsable]` state. `EffectDescriptor::visibility` adjusts it at run time.
    pub browsable: bool,
    /// `[Value]`: part of the effect's user-settable value set, and copied by "paste parameters".
    pub is_value: bool,
    /// `[ExcludeFromEffectDefault]`.
    pub exclude_from_default: bool,
    pub editor: Editor,
    /// How the editor's value relates to the stored one, for properties whose getter and setter convert.
    pub display: Display,
    /// `[BoolDescription]`: the words shown for true and false, in place of a check box.
    pub bool_labels: Option<(&'static str, &'static str)>,
    pub default: fn() -> ParamValue,
}

/// The value of one effect property.
#[derive(Clone, Debug, PartialEq)]
pub enum ParamValue {
    None,
    Bool(bool),
    Int(i32),
    Long(i64),
    Double(f64),
    Str(String),
    /// An enum member name.
    Enum(String),
    Curve(Curve),
    Gradient(ColorGradient),
    GradientList(Vec<ColorGradient>),
    GradientLevels(Vec<GradientLevelPair>),
    Color(Color),
    ColorList(Vec<Color>),
    Rgb(Rgb),
    Percentage(f64),
    Font(FontDesc),
    Guid(Uuid),
    Time(TimeSpan),
    Record(Box<Params>),
    RecordList(Vec<Params>),
    /// Unmodelled data, kept verbatim as the XML of the member's children.
    Raw(String),
}

/// `SerializableFont`: stored as a name, a point size and a style.
#[derive(Clone, Debug, PartialEq)]
pub struct FontDesc {
    pub name: String,
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
}

impl FontDesc {
    pub fn new(name: &str, size: f32) -> FontDesc {
        FontDesc { name: name.to_string(), size, bold: false, italic: false }
    }
}

/// An effect's parameter values, parallel to its descriptor's table.
#[derive(Clone, Debug)]
pub struct Params {
    pub desc: &'static [ParamDesc],
    pub values: Vec<ParamValue>,
    /// XML of `[DataMember]`s this port does not model, so they survive a load and save.
    pub unknown_members: Vec<(String, String)>,
}

impl PartialEq for Params {
    fn eq(&self, other: &Params) -> bool {
        self.values == other.values && self.unknown_members == other.unknown_members
    }
}

impl Params {
    /// `new XxxData()`: every value at its default.
    pub fn defaults(desc: &'static [ParamDesc]) -> Params {
        Params { desc, values: desc.iter().map(|d| (d.default)()).collect(), unknown_members: Vec::new() }
    }

    #[inline]
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.desc.iter().position(|d| d.name == name)
    }

    /// Looks up a value. An unknown name is a bug in the effect's port, so it panics in tests and
    /// degrades to `None` in release builds rather than killing a show.
    pub fn get(&self, name: &str) -> &ParamValue {
        match self.index_of(name) {
            Some(i) => &self.values[i],
            None => {
                debug_assert!(false, "unknown effect parameter {name}");
                &ParamValue::None
            }
        }
    }

    pub fn set(&mut self, name: &str, value: ParamValue) -> bool {
        match self.index_of(name) {
            Some(i) => {
                self.values[i] = value;
                true
            }
            None => false,
        }
    }

    pub fn int(&self, name: &str) -> i32 {
        match self.get(name) {
            ParamValue::Int(v) => *v,
            ParamValue::Long(v) => *v as i32,
            ParamValue::Double(v) => *v as i32,
            _ => 0,
        }
    }

    pub fn double(&self, name: &str) -> f64 {
        match self.get(name) {
            ParamValue::Double(v) | ParamValue::Percentage(v) => *v,
            ParamValue::Int(v) => *v as f64,
            ParamValue::Long(v) => *v as f64,
            _ => 0.0,
        }
    }

    pub fn boolean(&self, name: &str) -> bool {
        matches!(self.get(name), ParamValue::Bool(true))
    }

    pub fn string(&self, name: &str) -> &str {
        match self.get(name) {
            ParamValue::Str(s) | ParamValue::Enum(s) => s,
            _ => "",
        }
    }

    /// The enum member name, for matching: `match p.enum_name("Direction") { "Up" => .. }`.
    #[inline]
    pub fn enum_name(&self, name: &str) -> &str {
        self.string(name)
    }

    pub fn curve(&self, name: &str) -> Curve {
        match self.get(name) {
            ParamValue::Curve(c) => c.clone(),
            _ => Curve::default(),
        }
    }

    pub fn gradient(&self, name: &str) -> ColorGradient {
        match self.get(name) {
            ParamValue::Gradient(g) => g.clone(),
            _ => ColorGradient::default(),
        }
    }

    pub fn gradients(&self, name: &str) -> Vec<ColorGradient> {
        match self.get(name) {
            ParamValue::GradientList(g) => g.clone(),
            _ => Vec::new(),
        }
    }

    pub fn gradient_levels(&self, name: &str) -> Vec<GradientLevelPair> {
        match self.get(name) {
            ParamValue::GradientLevels(g) => g.clone(),
            _ => Vec::new(),
        }
    }

    pub fn color(&self, name: &str) -> Color {
        match self.get(name) {
            ParamValue::Color(c) => *c,
            ParamValue::Rgb(c) => c.to_color(),
            _ => Color::EMPTY,
        }
    }

    pub fn colors(&self, name: &str) -> Vec<Color> {
        match self.get(name) {
            ParamValue::ColorList(c) => c.clone(),
            _ => Vec::new(),
        }
    }

    pub fn font(&self, name: &str) -> FontDesc {
        match self.get(name) {
            ParamValue::Font(f) => f.clone(),
            _ => FontDesc::new("Arial", 8.0),
        }
    }

    pub fn guid(&self, name: &str) -> Uuid {
        match self.get(name) {
            ParamValue::Guid(g) => *g,
            _ => Uuid::nil(),
        }
    }

    /// A nested object's values.
    pub fn record(&self, name: &str) -> Option<&Params> {
        match self.get(name) {
            ParamValue::Record(r) => Some(r),
            _ => None,
        }
    }

    /// A list of nested objects' values.
    pub fn records(&self, name: &str) -> &[Params] {
        match self.get(name) {
            ParamValue::RecordList(r) => r,
            _ => &[],
        }
    }

    pub fn time(&self, name: &str) -> TimeSpan {
        match self.get(name) {
            ParamValue::Time(t) => *t,
            _ => TimeSpan::ZERO,
        }
    }
}

/// Run-time `[Browsable]` overrides, the result of the C# `SetBrowsable` / `UpdateAttributes` calls.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Visibility {
    pub overrides: Vec<(&'static str, bool)>,
}

impl Visibility {
    #[inline]
    pub fn set(&mut self, name: &'static str, visible: bool) {
        match self.overrides.iter_mut().find(|o| o.0 == name) {
            Some(o) => o.1 = visible,
            None => self.overrides.push((name, visible)),
        }
    }

    pub fn is_visible(&self, d: &ParamDesc) -> bool {
        self.overrides.iter().find(|o| o.0 == d.name).map(|o| o.1).unwrap_or(d.browsable)
    }
}

/// Audio analysis for effects that react to the music (`AudioHelper`).
pub trait AudioSource: Send + Sync {
    /// Mono samples, -1..1.
    fn samples(&self) -> &[f32];
    fn sample_rate(&self) -> u32;
    fn duration(&self) -> TimeSpan;
    /// The samples as decoded, `channels` per frame.
    fn interleaved(&self) -> &[f32] {
        self.samples()
    }
    fn channels(&self) -> u32 {
        1
    }
}

/// What `AudioUtilities.LoadAudioIntoMemory` reads for an effect, through `Audio.GetMonoSamples`.
///
/// Both counts are multiplied by the channel count, but the read is then one mono sample per count. On a stereo file the
/// result holds twice the effect's length, the audio after the effect included, so that audio counts towards the
/// filters and the normalising peak. An odd start position pairs each right sample with the next frame's left, and the
/// mix is NAudio's `StereoToMonoSampleProvider` in `float`. Reading on past the end of the audio leaves zeros.
///
/// Returns `None` when there's nothing to read: no sample rate, an effect too short or too long for a buffer, or one that
/// starts past the end of the audio.
pub fn read_effect_audio(audio: &dyn AudioSource, start_time: TimeSpan, time_span: TimeSpan) -> Option<Vec<f64>> {
    let rate = audio.sample_rate();
    if rate == 0 {
        return None;
    }
    let channels = audio.channels().max(1) as usize;
    let per_second = rate as f32 as f64;
    let start = (per_second * start_time.total_secs() * channels as f64) as i64;
    let total = (per_second * time_span.total_secs() * channels as f64) as i64;
    if total <= 0 || total > 1 << 31 {
        return None;
    }
    let mut out = vec![0.0_f64; total as usize];
    if start < 0 {
        return Some(out);
    }
    let source = audio.interleaved();
    let start = start as usize;
    // `CachedSoundSampleProvider.Read` copies a negative count from past the end, and `Array.Copy` throws.
    if start > source.len() {
        return None;
    }
    if channels == 1 {
        if let Some(rest) = source.get(start..) {
            for (to, from) in out.iter_mut().zip(rest) {
                *to = *from as f64;
            }
        }
        return Some(out);
    }
    for (i, slot) in out.iter_mut().enumerate() {
        let position = start + i * channels;
        let Some(frame) = source.get(position..) else { break };
        if frame.is_empty() {
            break;
        }
        // NAudio only accepts two channels here. Anything else is mixed evenly rather than failing the effect.
        let weight = 1.0_f32 / channels as f32;
        let mixed: f32 = (0..channels).map(|c| frame.get(c).copied().unwrap_or(0.0) * weight).sum();
        *slot = mixed as f64;
    }
    Some(out)
}

/// Everything a render function may read. Immutable, so effects render in parallel.
pub struct RenderCtx<'a> {
    pub video_cache: Option<vixen_video::cache::CacheContext<'a>>,
    pub tree: &'a ElementTree,
    pub targets: &'a [NodeId],
    pub start_time: TimeSpan,
    pub time_span: TimeSpan,
    /// `VixenSystem.DefaultUpdateInterval`, in ms.
    pub frame_ms: i32,
    /// The sequence's mark collections.
    pub marks: &'a [MarkCollection],
    pub audio: Option<&'a dyn AudioSource>,
    /// Vixen's "Module Data Files" folder. An effect's own files are under `<root>/<EffectName>`, the
    /// C# `ModulePath` and `[ModuleDataPath]` convention.
    pub module_data_root: Option<&'a std::path::Path>,
    pub cancel: &'a AtomicBool,
}

impl RenderCtx<'_> {
    /// This effect's place, for code shared with the editor.
    pub fn target_info(&self) -> TargetInfo<'_> {
        TargetInfo {
            tree: self.tree,
            targets: self.targets,
            marks: self.marks,
            start_time: self.start_time,
            time_span: self.time_span,
            audio_loaded: self.audio.is_some(),
        }
    }

    #[inline]
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// `BaseEffect.FrameTimespan`.
    #[inline]
    pub fn frame_timespan(&self) -> TimeSpan {
        TimeSpan::from_ms_i(self.frame_ms as i64)
    }

    /// `GetValidColors()`; its non-emptiness is `HasDiscreteColors`.
    pub fn valid_colors(&self) -> Vec<Color> {
        self.tree.valid_colors_for(self.targets)
    }

    /// `BaseEffect.HasDiscreteColors`.
    pub fn has_discrete_colors(&self) -> bool {
        !self.valid_colors().is_empty()
    }

    /// `BaseEffect.IsElementDiscrete`.
    #[inline]
    pub fn is_element_discrete(&self, node: NodeId) -> bool {
        self.tree.is_discrete(node)
    }

    /// `BaseEffect.GetEffectTimeIntervalPosition(TimeSpan)`.
    #[inline]
    pub fn time_interval_position(&self, t: TimeSpan) -> f64 {
        t.total_ms() / self.time_span.total_ms()
    }

    /// `BaseEffect.DetermineDepth`.
    pub fn determine_depth(&self) -> i32 {
        self.targets.iter().map(|t| self.tree.max_child_depth(*t)).min().unwrap_or(i32::MAX)
    }

    /// `BaseEffect.CreateIntent(node, color, intensity, duration)`.
    pub fn create_intent(&self, has_discrete: bool, node: NodeId, color: Color, intensity: f64, duration: TimeSpan) -> Intent {
        if has_discrete && self.is_element_discrete(node) {
            Intent::static_discrete(color, intensity, duration)
        } else {
            Intent::static_lighting(color, intensity, duration)
        }
    }

    /// `BaseEffect.CreateIntent(node, startColor, endColor, startIntensity, endIntensity, duration)`.
    #[allow(clippy::too_many_arguments)]
    pub fn create_lerp_intent(
        &self,
        has_discrete: bool,
        node: NodeId,
        start_color: Color,
        end_color: Color,
        start_i: f64,
        end_i: f64,
        duration: TimeSpan,
    ) -> Intent {
        if has_discrete && self.is_element_discrete(node) {
            Intent::discrete(start_color, start_i, end_i, duration)
        } else {
            Intent::lighting(start_color, end_color, start_i, end_i, duration)
        }
    }

    /// `BaseEffect.CreateIntentsForElement`.
    pub fn create_intents_for_element(
        &self,
        has_discrete: bool,
        node: NodeId,
        intensity: f64,
        color: Color,
        duration: TimeSpan,
    ) -> EffectIntents {
        let mut out = EffectIntents::new();
        for leaf in self.tree.leaves(node) {
            if has_discrete && self.is_element_discrete(leaf) && !self.tree.valid_colors(leaf, false).contains(&color) {
                continue;
            }
            if let Some(element) = self.tree.node(leaf).element_id {
                let intent = self.create_intent(has_discrete, leaf, color, intensity, duration);
                out.add_intent_for_element(element, intent, TimeSpan::ZERO);
            }
        }
        out
    }

    /// The first mark collection with the given id, for effects with a mark collection selector.
    pub fn mark_collection(&self, id: Uuid) -> Option<&MarkCollection> {
        self.marks.iter().find(|m| m.id == id)
    }

    /// The target's colour type, for discrete-aware rendering.
    pub fn color_type(&self, node: NodeId) -> ElementColorType {
        self.tree.color_type(node)
    }
}

pub type RenderFn = fn(&RenderCtx, &Params) -> EffectIntents;
pub type VisibilityFn = fn(&Params, &TargetInfo, &mut Visibility);
/// Visibility inside one row of a `RecordList`: the row's values, then the effect's.
pub type RecordVisibilityFn = fn(&Params, &Params, &mut Visibility);
pub type ParamChangedFn = fn(&str, &mut Params);
/// A property setter whose side effect needs the old value or the targets.
pub type ParamChangedCtxFn = fn(&ParamChange, &mut Params);
/// `TargetNodesChanged`: runs when an effect is given new targets, not on load.
pub type TargetsChangedFn = fn(&mut Params, &TargetInfo);
pub type ValidateFn = fn(&mut Params, &[Color]);
/// `GenerateVisualRepresentation`: draws a timeline thumbnail into `(x, y, w, h)` of the canvas.
pub type VisualFn = fn(&Params, &TargetInfo, &mut crate::gfx::Graphics, (i32, i32, i32, i32));

/// One choice in a drop down: the value to store, and the text to show.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectionValue {
    pub value: ParamValue,
    pub label: String,
}

impl SelectionValue {
    /// A choice whose stored value is the text it shows, as the C# string properties have.
    pub fn text(label: impl Into<String>) -> SelectionValue {
        let label = label.into();
        SelectionValue { value: ParamValue::Str(label.clone()), label }
    }
}

/// An effect's own `TypeConverter.GetStandardValues`, for choices `EffectDescriptor::selection_values` can't work out from the
/// tables: the property, the row it is in if it is in a list, the effect's values and its place. `None` means no opinion.
pub type SelectionFn = fn(&str, Option<&Params>, &Params, &TargetInfo) -> Option<Vec<SelectionValue>>;

/// What an effect's editor logic reads from the instance besides its own data: `TargetNodes`, the sequence's
/// mark collections, its place on the timeline and whether the sequence has audio.
#[derive(Copy, Clone)]
pub struct TargetInfo<'a> {
    pub tree: &'a ElementTree,
    pub targets: &'a [NodeId],
    pub marks: &'a [MarkCollection],
    pub start_time: TimeSpan,
    pub time_span: TimeSpan,
    pub audio_loaded: bool,
}

impl TargetInfo<'static> {
    /// An effect that hasn't been placed: no targets, no marks and no audio.
    pub fn detached() -> TargetInfo<'static> {
        static EMPTY: std::sync::LazyLock<ElementTree> = std::sync::LazyLock::new(ElementTree::new);
        TargetInfo {
            tree: &EMPTY,
            targets: &[],
            marks: &[],
            start_time: TimeSpan::ZERO,
            time_span: TimeSpan::ZERO,
            audio_loaded: false,
        }
    }
}

impl TargetInfo<'_> {
    /// `GetValidColors()`; its non-emptiness is `HasDiscreteColors`.
    pub fn valid_colors(&self) -> Vec<Color> {
        self.tree.valid_colors_for(self.targets)
    }

    /// `BaseEffect.HasDiscreteColors`.
    pub fn has_discrete_colors(&self) -> bool {
        !self.valid_colors().is_empty()
    }

    /// `BaseEffect.DetermineDepth`.
    pub fn determine_depth(&self) -> i32 {
        self.targets.iter().map(|t| self.tree.max_child_depth(*t)).min().unwrap_or(i32::MAX)
    }

    /// The first mark collection with the given id, for effects with a mark collection selector.
    pub fn mark_collection(&self, id: Uuid) -> Option<&MarkCollection> {
        self.marks.iter().find(|m| m.id == id)
    }
}

/// A property setter's view of a change.
pub struct ParamChange<'a> {
    pub name: &'a str,
    pub old: &'a ParamValue,
    pub info: &'a TargetInfo<'a>,
}

/// `EffectModuleDescriptorBase` plus the data contract identity.
pub struct EffectDescriptor {
    pub type_id: Uuid,
    pub name: &'static str,
    pub group: EffectGroup,
    pub author: &'static str,
    pub description: &'static str,
    pub version: &'static str,
    /// `[DataContract]` type name of the data class, as written in `i:type`.
    pub data_type: &'static str,
    /// CLR namespace of the data class, after `http://schemas.datacontract.org/2004/07/`.
    pub data_ns: &'static str,
    pub info_link: &'static str,
    /// `Information`: the text the effect editor shows above its More Info link.
    pub information: &'static str,
    /// `EffectImage.png` in the C# project, relative to `upstream/Vixen/src`.
    pub icon: &'static str,
    pub params: &'static [ParamDesc],
    pub render: RenderFn,
    /// The `UpdateAttributes` logic.
    pub visibility: Option<VisibilityFn>,
    /// Side effects in C# property setters.
    pub param_changed: Option<ParamChangedFn>,
    pub param_changed_ctx: Option<ParamChangedCtxFn>,
    pub targets_changed: Option<TargetsChangedFn>,
    /// `CheckForInvalidColorData`: corrects colours the targets cannot show. Receives the valid colours.
    pub validate: Option<ValidateFn>,
    /// `ForceGenerateVisualRepresentation`: draws a custom thumbnail instead of rasterising intents.
    pub force_visual: bool,
    /// For an effect where that depends on its values, such as State's Show Effect Visual.
    pub force_visual_when: Option<fn(&Params) -> bool>,
    pub visual: Option<VisualFn>,
    /// The drop downs that read the targets or the effect's values.
    pub selection: Option<SelectionFn>,
    /// Whether `TargetPositioning.Locations` is implemented.
    pub supports_location: bool,
    /// False until the effect's render logic has been ported.
    pub ported: bool,
}

/// `ForceGenerateVisualRepresentation` for these values.
impl EffectDescriptor {
    pub fn forces_visual(&self, p: &Params) -> bool {
        self.force_visual_when.map_or(self.force_visual, |f| f(p))
    }

    /// Sets a parameter as its C# property setter does, side effects included.
    pub fn set_param(&self, p: &mut Params, name: &str, value: ParamValue, t: &TargetInfo) {
        let old = p.get(name).clone();
        p.set(name, value);
        if name == "TargetPositioning" && p.index_of("StringOrientation").is_some() {
            // `PixelEffectBase.TargetPositioning`.
            if p.enum_name("TargetPositioning") == "Locations" {
                p.set("StringOrientation", ParamValue::Enum("Vertical".to_string()));
            } else {
                apply_target_orientation(p, t);
            }
        }
        if let Some(f) = self.param_changed {
            f(name, p);
        }
        if let Some(f) = self.param_changed_ctx {
            f(&ParamChange { name, old: &old, info: t }, p);
        }
    }

    /// `effect.TargetNodes = ...`, for an effect the user has moved, pasted or newly placed. The `PixelEffectBase`
    /// and `CheckForInvalidColorData` parts are generic; anything else is the effect's own `targets_changed`.
    pub fn set_targets(&self, p: &mut Params, t: &TargetInfo) {
        if p.index_of("StringOrientation").is_some() && p.index_of("TargetPositioning").is_some() && p.enum_name("TargetPositioning") != "Locations" {
            apply_target_orientation(p, t);
        }
        if let Some(f) = self.validate {
            if !t.targets.is_empty() {
                f(p, &t.valid_colors());
            }
        }
        if let Some(f) = self.targets_changed {
            f(p, t);
        }
    }

    /// What loading a sequence does to an effect's saved values. `EffectNodeSurrogate` gives a new effect its targets
    /// while it still holds default data, so `TargetNodesChanged` never sees the saved values. The `ModuleData` setter
    /// does, with the targets already there, and `SequenceService.Load` then assigns the mark collections. A setter's
    /// target-dependent work reaches the effect through `param_changed_ctx` as `MODULE_DATA` and `MARK_COLLECTIONS`.
    pub fn load_data(&self, p: &mut Params, t: &TargetInfo) {
        if let Some(f) = self.validate {
            f(p, &t.valid_colors());
        }
        if let Some(f) = self.param_changed_ctx {
            f(&ParamChange { name: MODULE_DATA, old: &ParamValue::None, info: t }, p);
        }
        self.set_marks(p, t);
    }

    /// What a `SelectionEditor` offers for a property, or for a field of `row` in a list: the C# `TypeConverter`s, whose
    /// lists come from the targets, the marks and the attributes. An effect's own rule comes first. Empty when the list
    /// comes from outside the effect, such as LipSync's phoneme mappings.
    pub fn selection_values(&self, name: &str, row: Option<&Params>, p: &Params, t: &TargetInfo) -> Vec<SelectionValue> {
        if let Some(values) = self.selection.and_then(|f| f(name, row, p, t)) {
            return values;
        }
        let Some(desc) = row.unwrap_or(p).desc.iter().find(|d| d.name == name) else { return Vec::new() };
        match desc.kind {
            // `BooleanStringTypeConverter`.
            ParamKind::Bool => {
                let (yes, no) = desc.bool_labels.unwrap_or(("True", "False"));
                vec![
                    SelectionValue { value: ParamValue::Bool(true), label: yes.to_string() },
                    SelectionValue { value: ParamValue::Bool(false), label: no.to_string() },
                ]
            }
            ParamKind::Enum(e) => {
                e.members.iter().map(|m| SelectionValue { value: ParamValue::Enum(m.name.to_string()), label: m.label.to_string() }).collect()
            }
            // `IMarkCollectionNameConverter`, which shows names while the data holds the id.
            ParamKind::Guid if name == "MarkCollectionId" => {
                t.marks.iter().map(|m| SelectionValue { value: ParamValue::Guid(m.id), label: m.name.clone() }).collect()
            }
            // `TargetElementDepthConverter`.
            ParamKind::Int if name == "DepthOfEffect" => depth_choices(0, t.targets.first().map_or(0, |n| t.tree.max_child_depth(*n))),
            _ => Vec::new(),
        }
    }

    /// `effect.MarkCollections = ...`, also what a collection being added to the sequence or removed from it amounts to.
    pub fn set_marks(&self, p: &mut Params, t: &TargetInfo) {
        if let Some(f) = self.param_changed_ctx {
            f(&ParamChange { name: MARK_COLLECTIONS, old: &ParamValue::None, info: t }, p);
        }
    }
}

/// Depth drop down entries for `from..to`.
pub fn depth_choices(from: i32, to: i32) -> Vec<SelectionValue> {
    (from..to).map(|i| SelectionValue { value: ParamValue::Int(i), label: i.to_string() }).collect()
}

/// The name `param_changed_ctx` sees for the `ModuleData` setter, the one that runs when a sequence loads.
pub const MODULE_DATA: &str = "ModuleData";
/// The name `param_changed_ctx` sees for the `MarkCollections` setter.
pub const MARK_COLLECTIONS: &str = "MarkCollections";

/// The common `TargetNodesChanged` rule: `if (firstNode != null && DepthOfEffect > firstNode.GetMaxChildDepth() - 1)
/// DepthOfEffect = 0`.
pub fn reset_depth_beyond_first_target(p: &mut Params, t: &TargetInfo) {
    if let Some(first) = t.targets.first() {
        if p.int("DepthOfEffect") > t.tree.max_child_depth(*first) - 1 {
            p.set("DepthOfEffect", ParamValue::Int(0));
        }
    }
}

/// `MarkCollectionSelectionService.Normalize` for a selection that is active: the stored id while that collection
/// exists, otherwise the first collection of the preferred type, otherwise the first of any type if allowed.
pub fn normalized_mark_selection(
    id: Uuid,
    marks: &[MarkCollection],
    preferred: Option<vixen_core::marks::MarkCollectionType>,
    first_fallback: bool,
) -> Uuid {
    if !id.is_nil() && marks.iter().any(|m| m.id == id) {
        return id;
    }
    if let Some(m) = marks.iter().find(|m| Some(m.collection_type) == preferred) {
        return m.id;
    }
    if first_fallback {
        marks.first().map_or(Uuid::nil(), |m| m.id)
    } else {
        Uuid::nil()
    }
}

/// `BaseEffect.NormalizeMarkCollectionSelections` for an effect's own `MarkCollectionId`, when `active` says it is using
/// it. An effect that has not been placed has no `MarkCollections` at all, so it is left alone, as the C# does.
pub fn normalize_mark_selection(
    p: &mut Params,
    t: &TargetInfo,
    active: bool,
    preferred: Option<vixen_core::marks::MarkCollectionType>,
    first_fallback: bool,
) {
    if !active || (t.targets.is_empty() && t.marks.is_empty()) {
        return;
    }
    let id = normalized_mark_selection(p.guid("MarkCollectionId"), t.marks, preferred, first_fallback);
    p.set("MarkCollectionId", ParamValue::Guid(id));
}

/// `PixelEffectBase.SetOrientation`: follows the first target's Orientation property when it has one.
fn apply_target_orientation(p: &mut Params, t: &TargetInfo) {
    use vixen_core::element::Orientation;
    let Some(orientation) = t.targets.first().and_then(|n| t.tree.orientation(*n)) else { return };
    let name = if orientation == Orientation::Horizontal { "Horizontal" } else { "Vertical" };
    p.set("StringOrientation", ParamValue::Enum(name.to_string()));
}

/// The `render` of an effect that has not been ported yet: lights stay dark, nothing crashes.
pub fn not_ported(_ctx: &RenderCtx, _params: &Params) -> EffectIntents {
    EffectIntents::new()
}

impl std::fmt::Debug for EffectDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EffectDescriptor({})", self.name)
    }
}

impl EffectDescriptor {
    pub fn new_params(&self) -> Params {
        Params::defaults(self.params)
    }

    /// The editor's visible parameters right now, in display order: categories by category order then
    /// name, properties by `[PropertyOrder]` then display name, ignoring case in both (`CategoryItemComparer`,
    /// `PropertyItemComparer`).
    pub fn visible_params(&self, p: &Params) -> Vec<usize> {
        self.visible_params_for(p, &TargetInfo::detached())
    }

    /// `visible_params` for an effect in place on the timeline.
    pub fn visible_params_for(&self, p: &Params, t: &TargetInfo) -> Vec<usize> {
        let mut vis = Visibility::default();
        if let Some(f) = self.visibility {
            f(p, t, &mut vis);
        }
        let mut idx: Vec<usize> = (0..self.params.len()).filter(|i| vis.is_visible(&self.params[*i])).collect();
        idx.sort_by_cached_key(|i| {
            let d = &self.params[*i];
            (d.category_order, d.category.to_lowercase(), d.order, d.label.to_lowercase(), *i)
        });
        idx
    }
}

/// `ThreadSafeRandom`. .NET's sequence is not reproduced because Vixen never seeds it.
#[derive(Clone, Debug)]
pub struct Rand(fastrand::Rng);

impl Default for Rand {
    fn default() -> Self {
        Rand(fastrand::Rng::new())
    }
}

impl Rand {
    pub fn seeded(seed: u64) -> Rand {
        Rand(fastrand::Rng::with_seed(seed))
    }

    /// `Random.Next()`.
    #[inline]
    pub fn next(&mut self) -> i32 {
        self.0.i32(0..i32::MAX)
    }

    /// `Random.Next(max)`: 0..max. Returns 0 for max <= 0.
    #[inline]
    pub fn next_max(&mut self, max: i32) -> i32 {
        if max <= 0 {
            0
        } else {
            self.0.i32(0..max)
        }
    }

    /// `Random.Next(min, max)`: min..max, returning min when the range is empty.
    #[inline]
    pub fn next_range(&mut self, min: i32, max: i32) -> i32 {
        if max <= min {
            min
        } else {
            self.0.i32(min..max)
        }
    }

    /// `Random.NextDouble()`.
    #[inline]
    pub fn next_double(&mut self) -> f64 {
        self.0.f64()
    }
}

/// `BaseEffect.ConvertRange`.
#[inline]
pub fn convert_range(original_start: f64, original_end: f64, new_start: f64, new_end: f64, value: f64) -> f64 {
    let scale = (new_end - new_start) / (original_end - original_start);
    new_start + (value - original_start) * scale
}

/// `BaseEffect.ScaleCurveToValue`: a 0..100 curve value into `minimum..maximum`.
#[inline]
pub fn scale_curve_to_value(value: f64, maximum: f64, minimum: f64) -> f64 {
    convert_range(0.0, 100.0, minimum, maximum, value)
}

/// `PixelEffectBase.ScaleValueToCurve`: a `minimum..maximum` value into a 0..100 curve value.
#[inline]
pub fn scale_value_to_curve(value: f64, maximum: f64, minimum: f64) -> f64 {
    convert_range(minimum, maximum, 0.0, 100.0, value)
}

/// `BaseEffect.DistanceFromPoint`.
#[inline]
pub fn distance_from_point(ox: i32, oy: i32, x: i32, y: i32) -> f64 {
    let (dx, dy) = ((x - ox) as f64, (y - oy) as f64);
    (dx * dx + dy * dy).sqrt()
}

/// Shared-ownership block for intents that use `FrameArray`.
pub type SharedBlock<T> = Arc<[T]>;
