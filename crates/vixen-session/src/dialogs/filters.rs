//! Output filter setup: Color Breakdown, Coarse Fine Breakdown, and the tag family (F135 to F150, F230 to F237).

use uuid::Uuid;
use vixen_output::dotnet::DotNetColor;
use vixen_output::filters::{
    ColorBreakdownConfig, ColorBreakdownItem, CurveData, DimmingCurveConfig, FilterConfig, PointPairData,
};
use vixen_output::ids;

/// Which sheet a filter's Configure opens (F2).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FilterSetupKind {
    ColorBreakdown,
    CoarseFine,
    DimmingCurve,
    Tagged,
    Dimming,
    ColorWheel,
    Shutter,
    Prism,
    None,
}

pub fn filter_setup_kind(type_id: &Uuid) -> FilterSetupKind {
    match *type_id {
        ids::COLOR_BREAKDOWN => FilterSetupKind::ColorBreakdown,
        ids::COARSE_FINE_BREAKDOWN => FilterSetupKind::CoarseFine,
        ids::DIMMING_CURVE => FilterSetupKind::DimmingCurve,
        ids::TAGGED_FILTER => FilterSetupKind::Tagged,
        ids::DIMMING_FILTER => FilterSetupKind::Dimming,
        ids::COLOR_WHEEL_FILTER => FilterSetupKind::ColorWheel,
        ids::SHUTTER_FILTER => FilterSetupKind::Shutter,
        ids::PRISM_FILTER => FilterSetupKind::Prism,
        _ => FilterSetupKind::None,
    }
}

// ---- Color Breakdown -----------------------------------------------------------------------------------------------------

/// The templates, in the picker's order (F137).
pub const COLOR_BREAKDOWN_TEMPLATES: [&str; 6] = ["RGB", "RGBW", "RGBY", "RGxB", "GRBW", "GRWB"];

/// F140: a picked colour that is exactly one the mixing filters look for is stored as that known colour. They compare with
/// `Color.Equals`, so a picked pure red stored as a value would land on the blue channel.
pub fn color_for_item(argb: u32) -> DotNetColor {
    match argb {
        0xFFFF_0000 => DotNetColor::RED,
        0xFF00_FF00 => DotNetColor::LIME,
        0xFF00_00FF => DotNetColor::BLUE,
        0xFFFF_FFFF => DotNetColor::WHITE,
        _ => DotNetColor::from_argb((argb >> 24) as u8, (argb >> 16) as u8, (argb >> 8) as u8, argb as u8),
    }
}

/// One row of the colour list. `token` remembers the exact stored colour, so a row whose colour wasn't touched keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorBreakdownRow {
    pub argb: u32,
    pub name: String,
    pub token: String,
}

fn color_token(c: &DotNetColor) -> String {
    format!("{}|{}|{}|{}", c.known_color, c.state, c.value, c.name.as_deref().unwrap_or(""))
}

fn color_from_token(token: &str) -> Option<DotNetColor> {
    let mut parts = token.splitn(4, '|');
    let known_color = parts.next()?.parse().ok()?;
    let state = parts.next()?.parse().ok()?;
    let value = parts.next()?.parse().ok()?;
    let name = parts.next()?;
    Some(DotNetColor { known_color, state, value, name: (!name.is_empty()).then(|| name.to_string()) })
}

fn row_from_item(item: &ColorBreakdownItem) -> ColorBreakdownRow {
    ColorBreakdownRow { argb: item.color.to_argb_u32(), name: item.name.clone(), token: color_token(&item.color) }
}

/// The rows a dialog opens with (F139).
pub fn color_breakdown_rows(config: &ColorBreakdownConfig) -> Vec<ColorBreakdownRow> {
    config.breakdown_items.iter().map(row_from_item).collect()
}

/// F138: what Apply Template replaces the list with. `None` is upstream's "Error applying template: Unknown template."
pub fn color_breakdown_template_rows(template: &str) -> Option<Vec<ColorBreakdownRow>> {
    Some(ColorBreakdownConfig::template(template)?.iter().map(row_from_item).collect())
}

/// F143: "Add Color" appends white, "New Color".
pub fn color_breakdown_new_row() -> ColorBreakdownRow {
    row_from_item(&ColorBreakdownItem::new(DotNetColor::WHITE, "New Color"))
}

/// F146.
pub fn color_breakdown_apply(config: &mut ColorBreakdownConfig, rows: &[ColorBreakdownRow], mix: bool, sixteen_bit: bool) {
    config.breakdown_items = rows
        .iter()
        .map(|row| {
            let color = color_from_token(&row.token)
                .filter(|c| c.to_argb_u32() == row.argb)
                .unwrap_or_else(|| color_for_item(row.argb));
            ColorBreakdownItem { color, name: row.name.clone() }
        })
        .collect();
    config.mix_colors = mix;
    config.sixteen_bit = sixteen_bit;
}

/// F144: upstream's filter throws at run time when mixing four colours that aren't red, green, blue and white. Here the odd one
/// stays dark, and the dialog says so.
pub fn color_breakdown_mix_note(rows: &[ColorBreakdownRow], mix: bool) -> Option<String> {
    let supported = [0xFFFF_0000, 0xFF00_FF00, 0xFF00_00FF, 0xFFFF_FFFF];
    (mix && rows.len() == 4 && rows.iter().any(|r| !supported.contains(&r.argb)))
        .then(|| "Four mixed colors must be red, green, blue and white. Any other color here will stay off.".to_string())
}

// ---- Coarse Fine ------------------------------------------------------------------------------------------------------------

/// F149: the messages that block OK. Sliders can't reach them, but imported values can.
pub fn coarse_fine_issues(coarse: i32, fine: i32) -> Vec<String> {
    let mut issues = Vec::new();
    if !(0..=255).contains(&coarse) {
        issues.push("Resting coarse value must be between 0 and 255.".to_string());
    }
    if !(0..=255).contains(&fine) {
        issues.push("Resting fine value must be between 0 and 255.".to_string());
    }
    issues
}

// ---- Tag family ---------------------------------------------------------------------------------------------------------------

/// What a tag filter's sheet edits. Fields a filter doesn't have stay empty.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TagFilterDraft {
    pub tag: String,
    pub automatic: bool,
    pub open_text: String,
    pub close_text: String,
    pub function_name: String,
}

/// The title and the labels of each filter's sheet (F230, F232 to F236).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TagFilterLabels {
    pub title: &'static str,
    /// The checkbox's label. Empty for the plain Tag filter.
    pub automatic: &'static str,
    pub open: &'static str,
    pub close: &'static str,
    pub function: &'static str,
}

pub fn tag_filter_labels(kind: FilterSetupKind) -> TagFilterLabels {
    match kind {
        FilterSetupKind::Dimming => TagFilterLabels {
            title: "Dimming Filter Setup",
            automatic: "Automatically Control Dimmer",
            ..Default::default()
        },
        FilterSetupKind::ColorWheel => TagFilterLabels {
            title: "Color Wheel Filter Setup",
            automatic: "Automatically Control Color Wheel",
            ..Default::default()
        },
        FilterSetupKind::Shutter => TagFilterLabels {
            title: "Shutter Filter Setup",
            automatic: "Automatically Open and Close Shutter",
            open: "Open Shutter Value:",
            close: "Close Shutter Value:",
            function: "",
        },
        FilterSetupKind::Prism => TagFilterLabels {
            title: "Prism Filter Setup",
            automatic: "Automatically Open and Close Prism",
            open: "Open Prism Value:",
            close: "Close Prism Value:",
            function: "Associated Prism Function Name:",
        },
        _ => TagFilterLabels { title: "Tagged Filter Setup", ..Default::default() },
    }
}

pub fn tag_filter_draft(config: &FilterConfig) -> TagFilterDraft {
    let tag = |t: &Option<String>| t.clone().unwrap_or_default();
    match config {
        FilterConfig::Tagged(c) => TagFilterDraft { tag: tag(&c.tag), ..Default::default() },
        FilterConfig::Dimming(c) => {
            TagFilterDraft { tag: tag(&c.tag), automatic: c.convert_rgb_into_dimming_intents, ..Default::default() }
        }
        FilterConfig::ColorWheel(c) => {
            TagFilterDraft { tag: tag(&c.tag), automatic: c.convert_color_intents_into_index_commands, ..Default::default() }
        }
        FilterConfig::Shutter(c) => TagFilterDraft {
            tag: tag(&c.tag),
            automatic: c.convert_color_into_shutter_intents,
            open_text: c.open_shutter_index_value.to_string(),
            close_text: c.close_shutter_index_value.to_string(),
            function_name: String::new(),
        },
        FilterConfig::Prism(c) => TagFilterDraft {
            tag: tag(&c.tag),
            automatic: c.convert_prism_intents_into_open_prism_intents,
            open_text: c.open_prism_index_value.to_string(),
            close_text: c.close_prism_index_value.to_string(),
            function_name: c.associated_function_name.clone().unwrap_or_default(),
        },
        _ => TagFilterDraft::default(),
    }
}

/// F235: 0 to 255, what `byte.Parse` accepts. Upstream throws on anything else.
pub fn validate_index_value(text: &str) -> Result<u8, String> {
    text.trim().parse::<u8>().map_err(|_| "0-255".to_string())
}

/// F231, F235, F236: whether OK is available. Unlike upstream the value boxes only count when the checkbox is on, since they're
/// disabled otherwise.
pub fn tag_filter_can_apply(config: &FilterConfig, draft: &TagFilterDraft) -> bool {
    if draft.tag.is_empty() {
        return false;
    }
    match config {
        FilterConfig::Shutter(_) | FilterConfig::Prism(_) => {
            !draft.automatic
                || (validate_index_value(&draft.open_text).is_ok() && validate_index_value(&draft.close_text).is_ok())
        }
        _ => true,
    }
}

/// F231 to F236. Upstream never saves Prism's open value (F236); this does.
pub fn tag_filter_apply(config: &mut FilterConfig, draft: &TagFilterDraft) -> Result<(), String> {
    let index = |text: &str, old: u8| -> Result<u8, String> {
        if draft.automatic {
            validate_index_value(text)
        } else {
            Ok(validate_index_value(text).unwrap_or(old))
        }
    };
    match config {
        FilterConfig::Tagged(c) => c.tag = Some(draft.tag.clone()),
        FilterConfig::Dimming(c) => {
            c.tag = Some(draft.tag.clone());
            c.convert_rgb_into_dimming_intents = draft.automatic;
        }
        FilterConfig::ColorWheel(c) => {
            c.tag = Some(draft.tag.clone());
            c.convert_color_intents_into_index_commands = draft.automatic;
        }
        FilterConfig::Shutter(c) => {
            c.open_shutter_index_value = index(&draft.open_text, c.open_shutter_index_value)?;
            c.close_shutter_index_value = index(&draft.close_text, c.close_shutter_index_value)?;
            c.tag = Some(draft.tag.clone());
            c.convert_color_into_shutter_intents = draft.automatic;
        }
        FilterConfig::Prism(c) => {
            c.open_prism_index_value = index(&draft.open_text, c.open_prism_index_value)?;
            c.close_prism_index_value = index(&draft.close_text, c.close_prism_index_value)?;
            c.tag = Some(draft.tag.clone());
            c.convert_prism_intents_into_open_prism_intents = draft.automatic;
            // An empty name matches every prism intent at run time, and stays an empty string in the file.
            c.associated_function_name = Some(draft.function_name.clone());
        }
        _ => {}
    }
    Ok(())
}

// ---- Dimming Curve --------------------------------------------------------------------------------------------------------

/// What the curve editor shows and hands back.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CurveDraft {
    pub points: Vec<(f64, f64)>,
    pub library_reference_name: String,
    pub is_current_library_curve: bool,
}

pub fn curve_draft(data: &CurveData) -> CurveDraft {
    CurveDraft {
        points: data.points.iter().map(|p| (p.x, p.y)).collect(),
        library_reference_name: data.library_reference_name.clone().unwrap_or_default(),
        is_current_library_curve: data.is_current_library_curve,
    }
}

/// A new curve: a ramp with x = y.
pub fn default_curve_draft() -> CurveDraft {
    curve_draft(&CurveData::default())
}

pub fn curve_data_from_draft(draft: &CurveDraft, old: Option<&CurveData>) -> CurveData {
    let points: Vec<PointPairData> = draft
        .points
        .iter()
        .enumerate()
        .map(|(i, (x, y))| match old.and_then(|o| o.points.get(i)) {
            Some(p) if p.x == *x && p.y == *y => p.clone(),
            _ => PointPairData::new(*x, *y),
        })
        .collect();
    CurveData {
        is_current_library_curve: draft.is_current_library_curve,
        points,
        library_reference_name: Some(draft.library_reference_name.clone()),
    }
}

/// F237.
pub fn dimming_curve_apply(config: &mut DimmingCurveConfig, draft: &CurveDraft) {
    config.curve = curve_data_from_draft(draft, Some(&config.curve));
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_output::filters::{CoarseFineConfig, PrismFilterConfig, ShutterFilterConfig, TaggedFilterConfig};
    use vixen_xml::XElement;

    fn apply_template(name: &str) -> Vec<ColorBreakdownItem> {
        let rows = color_breakdown_template_rows(name).unwrap();
        let mut c = ColorBreakdownConfig::default();
        color_breakdown_apply(&mut c, &rows, false, false);
        c.breakdown_items
    }

    #[test]
    fn templates_match_upstream() {
        assert_eq!(COLOR_BREAKDOWN_TEMPLATES, ["RGB", "RGBW", "RGBY", "RGxB", "GRBW", "GRWB"]);
        let names = |n: &str| apply_template(n).iter().map(|i| i.name.clone()).collect::<Vec<_>>();
        assert_eq!(names("RGB"), ["Red", "Green", "Blue"]);
        assert_eq!(names("RGBW"), ["Red", "Green", "Blue", "White"]);
        assert_eq!(names("RGBY"), ["Red", "Green", "Blue", "Yellow"]);
        assert_eq!(names("RGxB"), ["Red", "Green", "[empty]", "Blue"]);
        assert_eq!(names("GRBW"), ["Green", "Red", "Blue", "White"]);
        assert_eq!(names("GRWB"), ["Green", "Red", "White", "Blue"]);
        assert!(color_breakdown_template_rows("nope").is_none());
    }

    #[test]
    fn templates_store_known_colours_and_green_is_lime() {
        let items = apply_template("RGxB");
        assert_eq!(items[0].color, DotNetColor::RED);
        assert_eq!(items[1].color, DotNetColor::LIME);
        assert_eq!(items[1].color.known_color, vixen_output::dotnet::known::LIME);
        assert_eq!(items[2].color, DotNetColor::BLACK);
        assert_eq!(items[3].color, DotNetColor::BLUE);
        assert_eq!(apply_template("RGBY")[3].color, DotNetColor::YELLOW);
    }

    #[test]
    fn a_picked_pure_colour_is_stored_as_the_known_one() {
        assert_eq!(color_for_item(0xFFFF_0000).known_color, 141);
        assert_eq!(color_for_item(0xFF00_FF00), DotNetColor::LIME);
        assert_eq!(color_for_item(0xFF00_00FF), DotNetColor::BLUE);
        assert_eq!(color_for_item(0xFFFF_FFFF), DotNetColor::WHITE);
        let other = color_for_item(0xFF12_3456);
        assert!(!other.is_known_color());
        assert_eq!(other.to_argb_u32(), 0xFF12_3456);
    }

    #[test]
    fn an_untouched_row_keeps_its_stored_colour_and_a_changed_one_does_not() {
        // A value colour red, as upstream's picker stores it.
        let stored = ColorBreakdownItem { color: DotNetColor::from_rgb(255, 0, 0), name: "Picked".to_string() };
        let mut c = ColorBreakdownConfig { breakdown_items: vec![stored.clone()], ..Default::default() };
        let mut rows = color_breakdown_rows(&c);
        assert_eq!(rows[0].argb, 0xFFFF_0000);
        color_breakdown_apply(&mut c, &rows, false, false);
        assert_eq!(c.breakdown_items[0], stored, "unchanged rows round-trip exactly");
        rows[0].argb = 0xFF00_00FF;
        color_breakdown_apply(&mut c, &rows, false, false);
        assert_eq!(c.breakdown_items[0].color, DotNetColor::BLUE);
    }

    #[test]
    fn a_new_row_is_white_new_color() {
        let r = color_breakdown_new_row();
        assert_eq!((r.argb, r.name.as_str()), (0xFFFF_FFFF, "New Color"));
    }

    #[test]
    fn mix_and_16_bit_are_staged_until_apply() {
        let mut c = ColorBreakdownConfig::default();
        color_breakdown_apply(&mut c, &[], true, true);
        assert!(c.mix_colors && c.sixteen_bit);
        let xml = c.to_xml();
        assert_eq!(xml.child_text("_16Bit").as_deref(), Some("true"));
        assert_eq!(xml.child_text("MixColors").as_deref(), Some("true"));
    }

    #[test]
    fn four_mixed_colours_outside_rgbw_get_a_note() {
        let rgby = color_breakdown_template_rows("RGBY").unwrap();
        assert!(color_breakdown_mix_note(&rgby, true).is_some());
        assert!(color_breakdown_mix_note(&rgby, false).is_none());
        assert!(color_breakdown_mix_note(&color_breakdown_template_rows("RGxB").unwrap(), true).is_some());
        assert!(color_breakdown_mix_note(&color_breakdown_template_rows("RGBW").unwrap(), true).is_none());
        assert!(color_breakdown_mix_note(&color_breakdown_template_rows("RGB").unwrap(), true).is_none());
    }

    #[test]
    fn coarse_fine_ranges() {
        assert!(coarse_fine_issues(0, 255).is_empty());
        assert_eq!(coarse_fine_issues(256, 0), ["Resting coarse value must be between 0 and 255."]);
        assert_eq!(coarse_fine_issues(0, -1), ["Resting fine value must be between 0 and 255."]);
    }

    #[test]
    fn coarse_fine_round_trips() {
        let c = CoarseFineConfig { enable_default_value_mapping: true, resting_coarse_value: 0xA5, resting_fine_value: 0x3C, ..Default::default() };
        let back = CoarseFineConfig::from_xml(&c.to_xml());
        assert_eq!(back, c);
    }

    #[test]
    fn index_values_are_bytes() {
        assert_eq!(validate_index_value("0"), Ok(0));
        assert_eq!(validate_index_value("255"), Ok(255));
        for bad in ["256", "-1", "abc", ""] {
            assert!(validate_index_value(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn an_empty_tag_cannot_be_saved() {
        let c = FilterConfig::Tagged(TaggedFilterConfig::default());
        assert!(!tag_filter_can_apply(&c, &TagFilterDraft::default()));
        assert!(tag_filter_can_apply(&c, &TagFilterDraft { tag: "Pan".to_string(), ..Default::default() }));
    }

    #[test]
    fn value_boxes_only_count_when_the_box_is_checked() {
        let c = FilterConfig::Shutter(ShutterFilterConfig::default());
        let mut d = TagFilterDraft { tag: "S".to_string(), open_text: "x".to_string(), close_text: "1".to_string(), ..Default::default() };
        assert!(tag_filter_can_apply(&c, &d));
        d.automatic = true;
        assert!(!tag_filter_can_apply(&c, &d));
        d.open_text = "2".to_string();
        assert!(tag_filter_can_apply(&c, &d));
    }

    #[test]
    fn prism_ok_stores_open_and_close_separately() {
        let mut c = FilterConfig::Prism(PrismFilterConfig::default());
        let d = TagFilterDraft {
            tag: "P".to_string(),
            automatic: true,
            open_text: "12".to_string(),
            close_text: "34".to_string(),
            function_name: "Prism".to_string(),
        };
        tag_filter_apply(&mut c, &d).unwrap();
        let FilterConfig::Prism(p) = &c else { panic!() };
        assert_eq!((p.open_prism_index_value, p.close_prism_index_value), (12, 34));
        assert_eq!(p.associated_function_name.as_deref(), Some("Prism"));
        assert_eq!(c.display_name(), "Prism Filter: P");
        assert_eq!(tag_filter_draft(&c), d);
    }

    #[test]
    fn a_filter_config_round_trips_through_its_xml() {
        let mut c = FilterConfig::Shutter(ShutterFilterConfig::default());
        tag_filter_apply(
            &mut c,
            &TagFilterDraft { tag: "S".to_string(), automatic: true, open_text: "7".to_string(), close_text: "8".to_string(), ..Default::default() },
        )
        .unwrap();
        let xml = c.to_xml();
        let text = xml.to_fragment();
        let back = FilterConfig::from_xml(&ids::SHUTTER_FILTER, &XElement::parse(&text).unwrap()).unwrap();
        assert_eq!(tag_filter_draft(&back).open_text, "7");
    }

    #[test]
    fn dimming_curve_keeps_untouched_points_and_library_name() {
        let mut c = DimmingCurveConfig::default();
        let mut d = curve_draft(&c.curve);
        assert_eq!(d.points, [(0.0, 0.0), (100.0, 100.0)]);
        d.points[1].1 = 50.0;
        d.library_reference_name = "Lib".to_string();
        dimming_curve_apply(&mut c, &d);
        assert_eq!(c.curve.points[1].y, 50.0);
        assert_eq!(c.curve.library_reference_name.as_deref(), Some("Lib"));
        assert_eq!(c.curve.points[0].schema, 11);
    }
}
