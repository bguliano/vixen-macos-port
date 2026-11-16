//! The layer mixing filter dialogs. Their data lives in the sequence, so they're on the sequencer's session, as the Layer Editor needs
//! them. The context's versions work on values, for anything without a session.

use super::{parse_id, DialogContext};
use crate::core::FfiError;
use crate::sequence::SequencerSession;
use vixen_core::layer::{ChromaKeySettings, MixingFilter};
use vixen_core::Color;
use vixen_session::dialogs::layers as logic;

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq, Debug)]
pub enum DialogLayerKind {
    ChromaKey,
    LumaKey,
    MaskFill,
    None,
}

impl From<logic::LayerSetupKind> for DialogLayerKind {
    fn from(k: logic::LayerSetupKind) -> Self {
        match k {
            logic::LayerSetupKind::ChromaKey => DialogLayerKind::ChromaKey,
            logic::LayerSetupKind::LumaKey => DialogLayerKind::LumaKey,
            logic::LayerSetupKind::MaskFill => DialogLayerKind::MaskFill,
            logic::LayerSetupKind::None => DialogLayerKind::None,
        }
    }
}

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq, Debug)]
pub enum DialogLimitEnd {
    Lower,
    Upper,
}

#[derive(uniffi::Record, Clone, Copy, PartialEq, Eq)]
pub struct DialogLimits {
    pub lower: i32,
    pub upper: i32,
}

/// What Chroma Key's sheet edits.
#[derive(uniffi::Record, Clone, PartialEq)]
pub struct DialogChromaKey {
    pub limits: DialogLimits,
    pub key: u32,
    pub hue_tolerance: i32,
    pub saturation_percent: i32,
    pub hue_tolerance_edited: bool,
    pub saturation_tolerance_edited: bool,
    pub transparent: bool,
}

impl From<logic::ChromaKeyDraft> for DialogChromaKey {
    fn from(d: logic::ChromaKeyDraft) -> Self {
        DialogChromaKey {
            limits: DialogLimits { lower: d.lower, upper: d.upper },
            key: d.key.to_argb_u32(),
            hue_tolerance: d.hue_tolerance,
            saturation_percent: d.saturation_percent,
            hue_tolerance_edited: d.hue_tolerance_edited,
            saturation_tolerance_edited: d.saturation_tolerance_edited,
            transparent: d.transparent,
        }
    }
}

impl From<&DialogChromaKey> for logic::ChromaKeyDraft {
    fn from(d: &DialogChromaKey) -> Self {
        logic::ChromaKeyDraft {
            lower: d.limits.lower,
            upper: d.limits.upper,
            key: Color::from_argb_u32(d.key),
            hue_tolerance: d.hue_tolerance,
            saturation_percent: d.saturation_percent,
            hue_tolerance_edited: d.hue_tolerance_edited,
            saturation_tolerance_edited: d.saturation_tolerance_edited,
            transparent: d.transparent,
        }
    }
}

#[derive(uniffi::Record, Clone, Copy, PartialEq, Eq)]
pub struct DialogMaskFill {
    pub exclude_zero_values: bool,
    pub requires_mixing_partner: bool,
}

#[uniffi::export]
impl DialogContext {
    /// F216: a limit moved. Clamps to the sliders' ranges and keeps Lower below Upper.
    pub fn clamp_limits(&self, limits: DialogLimits, changed: DialogLimitEnd) -> DialogLimits {
        let changed = match changed {
            DialogLimitEnd::Lower => logic::LimitEnd::Lower,
            DialogLimitEnd::Upper => logic::LimitEnd::Upper,
        };
        let (lower, upper) = logic::clamp_limits(limits.lower, limits.upper, changed);
        DialogLimits { lower, upper }
    }

    /// F212: a new Chroma Key filter's values.
    pub fn default_chroma_key(&self) -> DialogChromaKey {
        logic::chroma_key_draft(&ChromaKeySettings::default()).into()
    }

    /// F219: a new Luma Key filter's values.
    pub fn default_luma_key(&self) -> DialogLimits {
        DialogLimits { lower: 0, upper: 100 }
    }

    /// F220: a new Mask And Fill filter's values.
    pub fn default_mask_fill(&self) -> DialogMaskFill {
        DialogMaskFill { exclude_zero_values: true, requires_mixing_partner: false }
    }
}

impl SequencerSession {
    fn with_layer_filter<R>(&self, layer: &str, f: impl FnOnce(&mut MixingFilter) -> R) -> Result<R, FfiError> {
        let id = parse_id(layer)?;
        let mut session = self.session.lock();
        let result = {
            let target = session.document_mut().layers.layers.iter_mut().find(|l| l.id == id).and_then(|l| l.filter.as_mut());
            target.map(f)
        };
        result.ok_or_else(|| FfiError::new("That layer has no filter."))
    }
}

#[uniffi::export]
impl SequencerSession {
    /// F5: which sheet a layer's Configuration opens. `None` hides the button.
    pub fn layer_filter_kind(&self, layer: String) -> Result<DialogLayerKind, FfiError> {
        self.with_layer_filter(&layer, |filter| logic::layer_setup_kind(filter).into())
    }

    /// F212.
    pub fn chroma_key(&self, layer: String) -> Result<Option<DialogChromaKey>, FfiError> {
        self.with_layer_filter(&layer, |filter| match filter {
            MixingFilter::ChromaKey(s) => Some(logic::chroma_key_draft(s).into()),
            _ => None,
        })
    }

    /// F212 to F218.
    pub fn apply_chroma_key(&self, layer: String, draft: DialogChromaKey) -> Result<(), FfiError> {
        self.with_layer_filter(&layer, |filter| {
            if let MixingFilter::ChromaKey(s) = filter {
                *s = logic::chroma_key_apply(s, &(&draft).into());
            }
        })?;
        self.session.lock().layers_changed();
        Ok(())
    }

    /// F219.
    pub fn luma_key(&self, layer: String) -> Result<Option<DialogLimits>, FfiError> {
        self.with_layer_filter(&layer, |filter| match filter {
            MixingFilter::LumaKey { lower_limit, upper_limit } => {
                Some(DialogLimits { lower: logic::limit_to_percent(*lower_limit), upper: logic::limit_to_percent(*upper_limit) })
            }
            _ => None,
        })
    }

    /// F219.
    pub fn apply_luma_key(&self, layer: String, limits: DialogLimits) -> Result<(), FfiError> {
        self.with_layer_filter(&layer, |filter| {
            if let MixingFilter::LumaKey { lower_limit, upper_limit } = filter {
                (*lower_limit, *upper_limit) = logic::luma_key_apply(limits.lower, limits.upper);
            }
        })?;
        self.session.lock().layers_changed();
        Ok(())
    }

    /// F220 to F222.
    pub fn mask_fill(&self, layer: String) -> Result<Option<DialogMaskFill>, FfiError> {
        self.with_layer_filter(&layer, |filter| match filter {
            MixingFilter::MaskFill { exclude_zero_values, requires_mixing_partner } => Some(DialogMaskFill {
                exclude_zero_values: *exclude_zero_values,
                requires_mixing_partner: *requires_mixing_partner,
            }),
            _ => None,
        })
    }

    /// F220 to F223.
    pub fn apply_mask_fill(&self, layer: String, draft: DialogMaskFill) -> Result<(), FfiError> {
        self.with_layer_filter(&layer, |filter| {
            if let MixingFilter::MaskFill { exclude_zero_values, requires_mixing_partner } = filter {
                *exclude_zero_values = draft.exclude_zero_values;
                *requires_mixing_partner = draft.requires_mixing_partner;
            }
        })?;
        self.session.lock().layers_changed();
        Ok(())
    }
}
