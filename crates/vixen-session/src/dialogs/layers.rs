//! The layer mixing filter dialogs (F212 to F229). Their data lives in the sequence, not the module store.

use vixen_core::layer::{ChromaKeySettings, MixingFilter};
use vixen_core::{Color, Hsv};

/// F216: `(int)(limit * 100)`, which truncates, so 0.29 reads as 28.
pub fn limit_to_percent(limit: f64) -> i32 {
    (limit * 100.0) as i32
}

/// Which end of a brightness range moved.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LimitEnd {
    Lower,
    Upper,
}

/// F216: a value past the other limit snaps to it, and neither leaves its slider's range. Upstream throws for 100 in Lower or 0 in
/// Upper, since those sliders stop at 99 and start at 1.
pub fn clamp_limits(lower: i32, upper: i32, changed: LimitEnd) -> (i32, i32) {
    match changed {
        LimitEnd::Lower => {
            let lower = lower.clamp(0, 99);
            (lower.min(upper), upper)
        }
        LimitEnd::Upper => {
            let upper = upper.clamp(1, 100);
            (lower, upper.max(lower))
        }
    }
}

/// F213 to F217. Saturation is the slider's value over 100, where upstream reads the wrong slider with integer division (F215).
pub fn chroma_key_from_dialog(
    lower: i32,
    upper: i32,
    key: Color,
    hue_tolerance: f64,
    saturation_percent: i32,
    transparent: bool,
) -> ChromaKeySettings {
    ChromaKeySettings {
        lower_limit: lower as f64 / 100.0,
        upper_limit: upper as f64 / 100.0,
        key_color: key,
        key_saturation: (Hsv::from_color(key).s * 100.0).round_ties_even() / 100.0,
        key_hue: key.hue() as f64,
        hue_tolerance,
        saturation_tolerance: saturation_percent as f64 / 100.0,
        transparent_on_zero_brightness: transparent,
    }
}

/// What a Chroma Key sheet shows.
#[derive(Clone, Debug, PartialEq)]
pub struct ChromaKeyDraft {
    pub lower: i32,
    pub upper: i32,
    pub key: Color,
    pub hue_tolerance: i32,
    pub saturation_percent: i32,
    pub hue_tolerance_edited: bool,
    pub saturation_tolerance_edited: bool,
    pub transparent: bool,
}

/// F213 to F216.
pub fn chroma_key_draft(settings: &ChromaKeySettings) -> ChromaKeyDraft {
    ChromaKeyDraft {
        lower: limit_to_percent(settings.lower_limit),
        upper: limit_to_percent(settings.upper_limit),
        key: settings.key_color,
        hue_tolerance: settings.hue_tolerance.round_ties_even() as i32,
        saturation_percent: (settings.saturation_tolerance * 100.0).round_ties_even() as i32,
        hue_tolerance_edited: false,
        saturation_tolerance_edited: false,
        transparent: settings.transparent_on_zero_brightness,
    }
}

/// F219's OK: both limits are written as the shown whole percent over 100, moved or not, so a stored 0.29 that opened as 28
/// goes back as 0.28 (`LumaKeySetup.cs:20-21`, `LumaKeyModule.cs:43-44`).
pub fn luma_key_apply(lower: i32, upper: i32) -> (f64, f64) {
    (lower as f64 / 100.0, upper as f64 / 100.0)
}

/// OK. A tolerance slider nobody moved keeps the stored value exactly, since only its Scroll handler writes the property. The
/// limits are always written from the shown percents (`ChromaKeySetup.cs:29-30`).
pub fn chroma_key_apply(old: &ChromaKeySettings, draft: &ChromaKeyDraft) -> ChromaKeySettings {
    let opened = chroma_key_draft(old);
    let mut new = chroma_key_from_dialog(
        draft.lower,
        draft.upper,
        draft.key,
        draft.hue_tolerance as f64,
        draft.saturation_percent,
        draft.transparent,
    );
    if !draft.hue_tolerance_edited && draft.hue_tolerance == opened.hue_tolerance {
        new.hue_tolerance = old.hue_tolerance;
    }
    if !draft.saturation_tolerance_edited && draft.saturation_percent == opened.saturation_percent {
        new.saturation_tolerance = old.saturation_tolerance;
    }
    new
}

/// The sheet a layer's Configuration button opens, or `None` for a filter with no setup, where the button doesn't show (F5, F224 to F229).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LayerSetupKind {
    ChromaKey,
    LumaKey,
    MaskFill,
    None,
}

pub fn layer_setup_kind(filter: &MixingFilter) -> LayerSetupKind {
    match filter {
        MixingFilter::ChromaKey(_) => LayerSetupKind::ChromaKey,
        MixingFilter::LumaKey { .. } => LayerSetupKind::LumaKey,
        MixingFilter::MaskFill { .. } => LayerSetupKind::MaskFill,
        _ => LayerSetupKind::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_truncate_as_upstream_loads_them() {
        assert_eq!(limit_to_percent(0.29), 28);
        assert_eq!(limit_to_percent(0.57), 56);
        assert_eq!(limit_to_percent(0.58), 57);
        assert_eq!(limit_to_percent(1.0), 100);
    }

    #[test]
    fn a_limit_past_the_other_snaps_to_it() {
        assert_eq!(clamp_limits(80, 50, LimitEnd::Lower), (50, 50));
        assert_eq!(clamp_limits(30, 20, LimitEnd::Upper), (30, 30));
        assert_eq!(clamp_limits(10, 90, LimitEnd::Lower), (10, 90));
    }

    #[test]
    fn typing_past_a_slider_clamps_instead_of_throwing() {
        assert_eq!(clamp_limits(100, 100, LimitEnd::Lower), (99, 100));
        assert_eq!(clamp_limits(0, 0, LimitEnd::Upper), (0, 1));
    }

    #[test]
    fn the_dialog_derives_saturation_and_hue_from_the_key() {
        let s = chroma_key_from_dialog(0, 100, Color::rgb(0, 255, 0), 5.0, 25, false);
        assert_eq!(s.saturation_tolerance, 0.25);
        assert_eq!(s.key_hue, 120.0);
        assert_eq!(s.key_saturation, 1.0);
        assert_eq!(s.upper_limit, 1.0);
    }

    #[test]
    fn a_new_filter_opens_with_its_defaults() {
        let d = chroma_key_draft(&ChromaKeySettings::default());
        assert_eq!((d.lower, d.upper, d.hue_tolerance, d.saturation_percent, d.transparent), (0, 100, 5, 5, false));
        assert_eq!(d.key, Color::LIME);
    }

    #[test]
    fn untouched_tolerances_stay_and_limits_go_back_as_shown() {
        let old = ChromaKeySettings {
            lower_limit: 0.29,
            upper_limit: 0.57,
            hue_tolerance: 5.4,
            saturation_tolerance: 0.123,
            ..ChromaKeySettings::default()
        };
        let new = chroma_key_apply(&old, &chroma_key_draft(&old));
        assert_eq!((new.hue_tolerance, new.saturation_tolerance), (5.4, 0.123));
        // `_lowerLimit / 100d` of the truncated 28 and 56.
        assert_eq!((new.lower_limit, new.upper_limit), (0.28, 0.56));
        assert_eq!(luma_key_apply(limit_to_percent(0.29), limit_to_percent(1.0)), (0.28, 1.0));
        // Whole percents that survive the truncation are unchanged.
        let whole = ChromaKeySettings { lower_limit: 0.25, upper_limit: 0.75, ..ChromaKeySettings::default() };
        assert_eq!(chroma_key_apply(&whole, &chroma_key_draft(&whole)), whole);
    }

    #[test]
    fn the_flag_comes_through() {
        let old = ChromaKeySettings::default();
        let mut d = chroma_key_draft(&old);
        d.transparent = true;
        d.saturation_percent = 40;
        let new = chroma_key_apply(&old, &d);
        assert!(new.transparent_on_zero_brightness);
        assert_eq!(new.saturation_tolerance, 0.4);
    }

    #[test]
    fn tolerances_moved_back_to_the_opening_value_stay_edited() {
        let old = ChromaKeySettings {
            hue_tolerance: 5.4,
            saturation_tolerance: 0.123,
            ..ChromaKeySettings::default()
        };
        let mut draft = chroma_key_draft(&old);
        // The hue slider moved from 5 to 6 and back to 5. Its Scroll handler has still run.
        draft.hue_tolerance_edited = true;
        let hue_only = chroma_key_apply(&old, &draft);
        assert_eq!(hue_only.hue_tolerance, 5.0);
        assert_eq!(hue_only.saturation_tolerance, 0.123, "the other slider was untouched");
        // Likewise 12 -> 13 -> 12 writes the shown 0.12, following F215's intentional Mac correction.
        draft.saturation_tolerance_edited = true;
        let both = chroma_key_apply(&old, &draft);
        assert_eq!(both.hue_tolerance, 5.0);
        assert_eq!(both.saturation_tolerance, 0.12);
    }

    #[test]
    fn only_three_filters_have_setup() {
        assert_eq!(layer_setup_kind(&MixingFilter::ChromaKey(ChromaKeySettings::default())), LayerSetupKind::ChromaKey);
        assert_eq!(layer_setup_kind(&MixingFilter::LumaKey { lower_limit: 0.0, upper_limit: 1.0 }), LayerSetupKind::LumaKey);
        assert_eq!(
            layer_setup_kind(&MixingFilter::MaskFill { exclude_zero_values: true, requires_mixing_partner: false }),
            LayerSetupKind::MaskFill
        );
        for f in [
            MixingFilter::ColorChange,
            MixingFilter::HighestValue,
            MixingFilter::IntensityOverlay,
            MixingFilter::Mask,
            MixingFilter::MultiplyColor,
            MixingFilter::ProportionalMix,
        ] {
            assert_eq!(layer_setup_kind(&f), LayerSetupKind::None);
        }
    }
}
