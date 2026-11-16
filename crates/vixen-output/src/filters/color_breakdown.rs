//! Port of Vixen.Modules/OutputFilter/ColorBreakdown: intents in, one command per colour channel out.

use crate::command::OutputCommand;
use crate::dotnet::{byte_from_f64, ushort_from_f64, DotNetColor};
use crate::flow::{StateRef, States};
use crate::ids;
use crate::xmlutil::{self, bool_el, color_el, data_root, text_el, ModuleIds};
use vixen_core::{Color, Hsv};
use vixen_xml::{contract_ns, XElement};

const CLR_NS: &str = "VixenModules.OutputFilter.ColorBreakdown";

/// `ColorBreakdownItem`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorBreakdownItem {
    pub color: DotNetColor,
    pub name: String,
}

impl Default for ColorBreakdownItem {
    /// The C# constructor: white, "Unnamed".
    fn default() -> Self {
        ColorBreakdownItem { color: DotNetColor::WHITE, name: "Unnamed".to_string() }
    }
}

impl ColorBreakdownItem {
    pub fn new(color: DotNetColor, name: &str) -> ColorBreakdownItem {
        ColorBreakdownItem { color, name: name.to_string() }
    }
}

/// `ColorBreakdownData`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorBreakdownConfig {
    pub ids: ModuleIds,
    pub breakdown_items: Vec<ColorBreakdownItem>,
    pub mix_colors: bool,
    pub sixteen_bit: bool,
}

impl Default for ColorBreakdownConfig {
    fn default() -> Self {
        ColorBreakdownConfig {
            ids: ModuleIds::new(ids::COLOR_BREAKDOWN, Default::default()),
            breakdown_items: Vec::new(),
            mix_colors: false,
            sixteen_bit: false,
        }
    }
}

impl ColorBreakdownConfig {
    pub fn from_xml(e: &XElement) -> ColorBreakdownConfig {
        let mut items = Vec::new();
        if let Some(list) = e.child("BreakdownItems") {
            for item in list.children_named("ColorBreakdownItem") {
                items.push(ColorBreakdownItem {
                    color: item.child("Color").map(xmlutil::read_color).unwrap_or_default(),
                    name: xmlutil::get_opt_string(item, "Name").unwrap_or_default(),
                });
            }
        }
        ColorBreakdownConfig {
            ids: ModuleIds::read(e),
            breakdown_items: items,
            mix_colors: xmlutil::get_bool(e, "MixColors"),
            sixteen_bit: xmlutil::get_bool(e, "_16Bit"),
        }
    }

    pub fn to_xml(&self) -> XElement {
        let ns = contract_ns(CLR_NS);
        let mut e = data_root(&ns, "ColorBreakdownData", &self.ids);
        let mut list = XElement::new(&ns, "BreakdownItems");
        for item in &self.breakdown_items {
            let mut x = XElement::new(&ns, "ColorBreakdownItem");
            x.push(color_el(&ns, "Color", &item.color));
            x.push(text_el(&ns, "Name", &item.name));
            list.push(x);
        }
        e.push(list);
        e.push(bool_el(&ns, "MixColors", self.mix_colors));
        e.push(bool_el(&ns, "_16Bit", self.sixteen_bit));
        e
    }

    /// The setup dialog's templates: RGB, RGBW, RGBY, RGxB, GRBW and GRWB.
    pub fn template(name: &str) -> Option<Vec<ColorBreakdownItem>> {
        let red = || ColorBreakdownItem::new(DotNetColor::RED, "Red");
        let green = || ColorBreakdownItem::new(DotNetColor::LIME, "Green");
        let blue = || ColorBreakdownItem::new(DotNetColor::BLUE, "Blue");
        let white = || ColorBreakdownItem::new(DotNetColor::WHITE, "White");
        Some(match name {
            "RGB" => vec![red(), green(), blue()],
            "RGBW" => vec![red(), green(), blue(), white()],
            "RGBY" => vec![red(), green(), blue(), ColorBreakdownItem::new(DotNetColor::YELLOW, "Yellow")],
            "RGxB" => vec![red(), green(), ColorBreakdownItem::new(DotNetColor::BLACK, "[empty]"), blue()],
            "GRBW" => vec![green(), red(), blue(), white()],
            "GRWB" => vec![green(), red(), white(), blue()],
            _ => return None,
        })
    }
}

/// `ColorExtensions.RGBToRGBW`, in single precision as written. Each result is `(r, g, b, w)` on 0..255.
pub fn rgb_to_rgbw(color: Color) -> (i32, i32, i32, i32) {
    let ri = color.r as f32;
    let gi = color.g as f32;
    let bi = color.b as f32;
    let t_m = ri.max(gi.max(bi));
    if t_m == 0.0 {
        return (0, 0, 0, 0);
    }
    let multiplier = 255.0f32 / t_m;
    let h_r = ri * multiplier;
    let h_g = gi * multiplier;
    let h_b = bi * multiplier;
    let big_m = h_r.max(h_g.max(h_b));
    let small_m = h_r.min(h_g.min(h_b));
    let luminance = ((big_m + small_m) / 2.0f32 - 127.5f32) * (255.0f32 / 127.5f32) / multiplier;
    // `Convert.ToInt32(float)` widens to double and rounds half to even.
    let round = |v: f32| (v as f64).round_ties_even() as i32;
    let wo = round(luminance).clamp(0, 255);
    let bo = round(bi - luminance).clamp(0, 255);
    let ro = round(ri - luminance).clamp(0, 255);
    let go = round(gi - luminance).clamp(0, 255);
    (ro, go, bo, wo)
}

/// `ToRGBWFast`, then the `(float)` cast each filter applies to its component: `[r, g, b, w]` on 0..1.
fn rgbw_proportions(color: Color) -> [f64; 4] {
    let (r, g, b, w) = rgb_to_rgbw(color);
    let p = |v: i32| (v as f64 / 255.0) as f32 as f64;
    [p(r), p(g), p(b), p(w)]
}

/// The `IBreakdownFilter` behind one output.
#[derive(Copy, Clone, Debug)]
enum Channel {
    /// `ColorBreakdownFilter`: only a state of exactly this colour counts. `.0001` is the C#'s tolerance,
    /// and since the components are integers, that means equal. RGB and lighting compare R, G and B;
    /// discrete values compare the whole ARGB value.
    Exact {
        r: u8,
        g: u8,
        b: u8,
        argb: u32,
    },
    /// `RGBColorBreakdownMixingFilter`.
    MixR,
    MixG,
    MixB,
    /// `RGBWColorBreakdownMixingFilter`, indexing `[r, g, b, w]`.
    Rgbw(usize),
    /// An RGBW filter whose colour isn't one of the four. The C# leaves its delegate null and throws on
    /// the first state; this stays dark instead.
    Dead,
}

/// The `RGBToRGBWConverter` shared by a filter's outputs, so a colour is only converted once.
#[derive(Clone, Debug)]
struct RgbwCache {
    color: Color,
    value: [f64; 4],
}

impl RgbwCache {
    #[inline]
    fn get(&mut self, color: Color) -> [f64; 4] {
        if self.color != color {
            self.color = color;
            self.value = rgbw_proportions(color);
        }
        self.value
    }
}

impl Channel {
    #[inline]
    fn intensity(self, s: StateRef<'_>, cache: &mut RgbwCache) -> f64 {
        // `color.R / 255f` is a float division, widened afterwards. That precision is part of the result.
        let proportion = |v: u8| (v as f32 / 255.0f32) as f64;
        match self {
            Channel::Exact { r, g, b, argb } => match s {
                StateRef::Rgb(v) if v.r == r && v.g == g && v.b == b => Hsv::v_from_color(v.full_color()),
                StateRef::Lighting(l) if l.color.r == r && l.color.g == g && l.color.b == b => l.intensity,
                StateRef::Discrete(d) if d.color.to_argb_u32() == argb => d.intensity,
                _ => 0.0,
            },
            // The mixing filters have no handler for discrete values, so those stay at 0.
            Channel::MixR => match s {
                StateRef::Rgb(v) => proportion(v.r),
                StateRef::Lighting(l) => l.intensity * proportion(l.color.r),
                _ => 0.0,
            },
            Channel::MixG => match s {
                StateRef::Rgb(v) => proportion(v.g),
                StateRef::Lighting(l) => l.intensity * proportion(l.color.g),
                _ => 0.0,
            },
            Channel::MixB => match s {
                StateRef::Rgb(v) => proportion(v.b),
                StateRef::Lighting(l) => l.intensity * proportion(l.color.b),
                _ => 0.0,
            },
            Channel::Rgbw(i) => match s {
                StateRef::Rgb(v) => cache.get(v.full_color())[i],
                StateRef::Lighting(l) => l.intensity * cache.get(l.color)[i],
                _ => 0.0,
            },
            Channel::Dead => 0.0,
        }
    }
}

/// `ColorBreakdownModule` with its outputs.
#[derive(Clone, Debug)]
pub(crate) struct ColorBreakdownRt {
    channels: Vec<Channel>,
    sixteen_bit: bool,
    outputs: Vec<OutputCommand>,
    cache: RgbwCache,
}

impl ColorBreakdownRt {
    pub(crate) fn new(cfg: &ColorBreakdownConfig) -> ColorBreakdownRt {
        let rgbw = cfg.mix_colors && cfg.breakdown_items.len() == 4;
        let channels = cfg
            .breakdown_items
            .iter()
            .map(|item| {
                let color = item.color.color();
                if rgbw {
                    // Compares `ToArgb()`, so a value colour matches.
                    match item.color.to_argb_u32() {
                        0xFFFF_0000 => Channel::Rgbw(0),
                        0xFF00_FF00 => Channel::Rgbw(1),
                        0xFF00_00FF => Channel::Rgbw(2),
                        0xFFFF_FFFF => Channel::Rgbw(3),
                        _ => Channel::Dead,
                    }
                } else if cfg.mix_colors {
                    // `Color.Equals` compares state too: a red saved as a value colour is not `Color.Red`,
                    // and it lands on the blue channel. Deliberately kept.
                    if item.color == DotNetColor::RED {
                        Channel::MixR
                    } else if item.color == DotNetColor::LIME {
                        Channel::MixG
                    } else {
                        Channel::MixB
                    }
                } else {
                    Channel::Exact { r: color.r, g: color.g, b: color.b, argb: color.to_argb_u32() }
                }
            })
            .collect::<Vec<_>>();
        let zero = if cfg.sixteen_bit { OutputCommand::U16(0) } else { OutputCommand::U8(0) };
        ColorBreakdownRt {
            outputs: vec![zero; channels.len()],
            channels,
            sixteen_bit: cfg.sixteen_bit,
            cache: RgbwCache { color: Color::EMPTY, value: [0.0; 4] },
        }
    }

    /// `ProcessInputData` on every output. `None` is an `IntentsDataFlowData` with a null list.
    #[inline]
    pub(crate) fn update(&mut self, states: Option<States<'_>>) {
        let cache = &mut self.cache;
        for (channel, out) in self.channels.iter().zip(self.outputs.iter_mut()) {
            let mut intensity = 0.0f64;
            if let Some(states) = states {
                states.for_each(|s| intensity = channel.intensity(s, cache).max(intensity));
            }
            *out = if self.sixteen_bit {
                OutputCommand::U16(ushort_from_f64(intensity * 65535.0))
            } else {
                OutputCommand::U8(byte_from_f64(intensity * 255.0))
            };
        }
    }

    pub(crate) fn output_count(&self) -> usize {
        self.outputs.len()
    }

    #[inline]
    pub(crate) fn output(&self, index: usize) -> Option<&OutputCommand> {
        self.outputs.get(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::{CombinedState, DiscreteValue, LightingValue, RgbValue};

    fn config(items: Vec<ColorBreakdownItem>, mix: bool, sixteen: bool) -> ColorBreakdownConfig {
        ColorBreakdownConfig { breakdown_items: items, mix_colors: mix, sixteen_bit: sixteen, ..Default::default() }
    }

    fn rgb_template() -> Vec<ColorBreakdownItem> {
        ColorBreakdownConfig::template("RGB").unwrap()
    }

    fn run(cfg: &ColorBreakdownConfig, state: &CombinedState) -> Vec<OutputCommand> {
        let mut rt = ColorBreakdownRt::new(cfg);
        rt.update(Some(States::Element(state)));
        rt.outputs.clone()
    }

    fn lighting(c: Color, i: f64) -> CombinedState {
        CombinedState { lighting: Some(LightingValue::new(c, i)), ..Default::default() }
    }

    fn rgb(r: u8, g: u8, b: u8) -> CombinedState {
        CombinedState { rgb: Some(RgbValue { r, g, b }), ..Default::default() }
    }

    fn discrete(values: &[(Color, f64)]) -> CombinedState {
        let mut s = CombinedState::default();
        for (c, i) in values {
            s.discrete.push(DiscreteValue::new(*c, *i));
        }
        s
    }

    fn bytes(v: &[OutputCommand]) -> Vec<u8> {
        v.iter().map(|c| c.byte()).collect()
    }

    #[test]
    fn outputs_start_at_zero_commands() {
        let rt = ColorBreakdownRt::new(&config(rgb_template(), true, false));
        assert_eq!(rt.outputs, vec![OutputCommand::U8(0); 3]);
        let rt = ColorBreakdownRt::new(&config(rgb_template(), true, true));
        assert_eq!(rt.outputs, vec![OutputCommand::U16(0); 3]);
    }

    #[test]
    fn rgb_mix_passes_components_through() {
        let cfg = config(rgb_template(), true, false);
        assert_eq!(bytes(&run(&cfg, &rgb(200, 100, 50))), vec![200, 100, 50]);
        for v in 0..=255u8 {
            assert_eq!(bytes(&run(&cfg, &rgb(v, v, v))), vec![v, v, v], "value {v}");
        }
    }

    #[test]
    fn rgb_mix_scales_a_lone_lighting_state_by_its_intensity() {
        let cfg = config(rgb_template(), true, false);
        assert_eq!(bytes(&run(&cfg, &lighting(Color::RED, 0.5))), vec![127, 0, 0]);
        assert_eq!(bytes(&run(&cfg, &lighting(Color::rgb(200, 100, 50), 1.0))), vec![200, 100, 50]);
        // Half of each channel, truncated.
        assert_eq!(bytes(&run(&cfg, &lighting(Color::rgb(200, 101, 3), 0.5))), vec![100, 50, 1]);
    }

    #[test]
    fn rgb_mix_ignores_discrete_values() {
        let cfg = config(rgb_template(), true, false);
        assert_eq!(bytes(&run(&cfg, &discrete(&[(Color::RED, 1.0)]))), vec![0, 0, 0]);
    }

    #[test]
    fn mix_channel_is_chosen_by_full_color_equality() {
        // Color.Red and Color.Lime are known colours. A value colour with the same RGB is neither, so it
        // becomes the blue channel.
        let items = vec![
            ColorBreakdownItem::new(DotNetColor::from_rgb(255, 0, 0), "Red"),
            ColorBreakdownItem::new(DotNetColor::from_rgb(0, 255, 0), "Green"),
            ColorBreakdownItem::new(DotNetColor::from_rgb(0, 0, 255), "Blue"),
        ];
        let cfg = config(items, true, false);
        assert_eq!(bytes(&run(&cfg, &rgb(10, 20, 30))), vec![30, 30, 30]);
    }

    #[test]
    fn exact_match_needs_the_same_colour() {
        let items = vec![ColorBreakdownItem::new(DotNetColor::RED, "Red"), ColorBreakdownItem::new(DotNetColor::WHITE, "White")];
        let cfg = config(items, false, false);
        // Lighting: the base colour must match, intensity carries through.
        assert_eq!(bytes(&run(&cfg, &lighting(Color::RED, 0.5))), vec![127, 0]);
        assert_eq!(bytes(&run(&cfg, &lighting(Color::WHITE, 0.25))), vec![0, 63]);
        assert_eq!(bytes(&run(&cfg, &lighting(Color::rgb(254, 0, 0), 1.0))), vec![0, 0]);
        // Mixed RGB: the brightness-applied colour must match, so only a full-brightness one ever does.
        assert_eq!(bytes(&run(&cfg, &rgb(255, 0, 0))), vec![255, 0]);
        assert_eq!(bytes(&run(&cfg, &rgb(128, 0, 0))), vec![0, 0]);
        assert_eq!(bytes(&run(&cfg, &rgb(255, 255, 255))), vec![0, 255]);
        // Discrete: exact colour, including alpha.
        assert_eq!(bytes(&run(&cfg, &discrete(&[(Color::RED, 0.5), (Color::WHITE, 1.0)]))), vec![127, 255]);
        assert_eq!(bytes(&run(&cfg, &discrete(&[(Color::argb(0, 255, 0, 0), 1.0)]))), vec![0, 0]);
    }

    #[test]
    fn exact_match_of_a_discrete_value_ignores_a_near_colour() {
        let cfg = config(vec![ColorBreakdownItem::new(DotNetColor::from_rgb(255, 165, 0), "Orange")], false, false);
        assert_eq!(bytes(&run(&cfg, &discrete(&[(Color::ORANGE, 0.5)]))), vec![127]);
        assert_eq!(bytes(&run(&cfg, &discrete(&[(Color::rgb(255, 165, 1), 1.0)]))), vec![0]);
    }

    #[test]
    fn highest_state_wins_on_each_output() {
        let cfg = config(vec![ColorBreakdownItem::new(DotNetColor::RED, "Red")], false, false);
        let mut s = rgb(255, 0, 0);
        s.discrete.push(DiscreteValue::new(Color::RED, 0.5));
        assert_eq!(bytes(&run(&cfg, &s)), vec![255]);
    }

    #[test]
    fn sixteen_bit_outputs_use_the_full_range() {
        let cfg = config(rgb_template(), true, true);
        assert_eq!(run(&cfg, &lighting(Color::RED, 0.5))[0], OutputCommand::U16(32767));
        assert_eq!(run(&cfg, &rgb(255, 0, 0))[0], OutputCommand::U16(65535));
        // A 16-bit command reads as 0 on a byte-oriented controller.
        assert_eq!(run(&cfg, &rgb(255, 0, 0))[0].byte(), 0);
    }

    #[test]
    fn no_states_means_zero_not_null() {
        let cfg = config(rgb_template(), true, false);
        let mut rt = ColorBreakdownRt::new(&cfg);
        rt.update(Some(States::Element(&rgb(255, 255, 255))));
        rt.update(None);
        assert_eq!(rt.outputs, vec![OutputCommand::U8(0); 3]);
        rt.update(Some(States::Element(&rgb(255, 255, 255))));
        rt.update(Some(States::Element(&CombinedState::default())));
        assert_eq!(rt.outputs, vec![OutputCommand::U8(0); 3]);
    }

    #[test]
    fn rgb_to_rgbw_matches_the_published_algorithm() {
        // Pure colours have no white in them.
        assert_eq!(rgb_to_rgbw(Color::RED), (255, 0, 0, 0));
        assert_eq!(rgb_to_rgbw(Color::LIME), (0, 255, 0, 0));
        assert_eq!(rgb_to_rgbw(Color::BLUE), (0, 0, 255, 0));
        // White is all white.
        assert_eq!(rgb_to_rgbw(Color::WHITE), (0, 0, 0, 255));
        assert_eq!(rgb_to_rgbw(Color::BLACK), (0, 0, 0, 0));
        // Gray: hue-normalised to white, then luminance divided back down.
        assert_eq!(rgb_to_rgbw(Color::rgb(128, 128, 128)), (0, 0, 0, 128));
        // M=255, m=127.5 -> (191.25 - 127.5) * 2 / 1 = 127.5, which rounds to the even 128.
        assert_eq!(rgb_to_rgbw(Color::rgb(255, 127, 127)).3, 127);
        assert_eq!(rgb_to_rgbw(Color::rgb(255, 128, 128)), (127, 0, 0, 128));
    }

    #[test]
    fn four_mixed_items_use_the_rgbw_filter() {
        let cfg = config(ColorBreakdownConfig::template("RGBW").unwrap(), true, false);
        assert_eq!(bytes(&run(&cfg, &rgb(255, 255, 255))), vec![0, 0, 0, 255]);
        assert_eq!(bytes(&run(&cfg, &rgb(255, 0, 0))), vec![255, 0, 0, 0]);
        assert_eq!(bytes(&run(&cfg, &rgb(255, 128, 128))), vec![127, 0, 0, 128]);
        assert_eq!(bytes(&run(&cfg, &lighting(Color::WHITE, 0.5))), vec![0, 0, 0, 127]);
        // Order comes from the items, by colour value.
        let cfg = config(ColorBreakdownConfig::template("GRWB").unwrap(), true, false);
        assert_eq!(bytes(&run(&cfg, &rgb(255, 128, 128))), vec![0, 127, 128, 0]);
    }

    #[test]
    fn four_mixed_items_with_a_foreign_colour_stay_dark_on_that_channel() {
        let cfg = config(ColorBreakdownConfig::template("RGBY").unwrap(), true, false);
        assert_eq!(bytes(&run(&cfg, &rgb(255, 0, 0))), vec![255, 0, 0, 0]);
        assert_eq!(bytes(&run(&cfg, &rgb(255, 255, 0))), vec![255, 255, 0, 0]);
    }

    #[test]
    fn three_or_five_mixed_items_stay_rgb() {
        let cfg = config(ColorBreakdownConfig::template("RGB").unwrap(), true, false);
        assert_eq!(bytes(&run(&cfg, &rgb(255, 255, 255))), vec![255, 255, 255]);
        let mut items = ColorBreakdownConfig::template("RGBW").unwrap();
        items.push(ColorBreakdownItem::new(DotNetColor::BLACK, "x"));
        let cfg = config(items, true, false);
        // White and Black are neither red nor lime, so both read the blue component.
        assert_eq!(bytes(&run(&cfg, &rgb(1, 2, 3))), vec![1, 2, 3, 3, 3]);
    }

    #[test]
    fn templates_match_the_setup_dialog() {
        let names = |t: &str| ColorBreakdownConfig::template(t).unwrap().into_iter().map(|i| i.name).collect::<Vec<_>>();
        assert_eq!(names("RGxB"), ["Red", "Green", "[empty]", "Blue"]);
        assert_eq!(names("GRWB"), ["Green", "Red", "White", "Blue"]);
        assert!(ColorBreakdownConfig::template("nope").is_none());
    }
}
