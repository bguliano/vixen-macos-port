//! Output filter setup.

use super::{no_such, parse_id, DialogContext};
use crate::core::FfiError;
use uuid::Uuid;
use vixen_output::filters::{CoarseFineConfig, CurveData, FilterConfig};
use vixen_output::ids;
use vixen_session::dialogs::filters as logic;
use vixen_session::dialogs::filters::{ColorBreakdownRow, CurveDraft, FilterSetupKind, TagFilterDraft};
use vixen_session::dialogs::world::DialogWorld;

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq, Debug)]
pub enum DialogFilterKind {
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

impl From<FilterSetupKind> for DialogFilterKind {
    fn from(k: FilterSetupKind) -> Self {
        match k {
            FilterSetupKind::ColorBreakdown => DialogFilterKind::ColorBreakdown,
            FilterSetupKind::CoarseFine => DialogFilterKind::CoarseFine,
            FilterSetupKind::DimmingCurve => DialogFilterKind::DimmingCurve,
            FilterSetupKind::Tagged => DialogFilterKind::Tagged,
            FilterSetupKind::Dimming => DialogFilterKind::Dimming,
            FilterSetupKind::ColorWheel => DialogFilterKind::ColorWheel,
            FilterSetupKind::Shutter => DialogFilterKind::Shutter,
            FilterSetupKind::Prism => DialogFilterKind::Prism,
            FilterSetupKind::None => DialogFilterKind::None,
        }
    }
}

#[derive(uniffi::Record, Clone, PartialEq, Eq)]
pub struct DialogBreakdownRow {
    pub argb: u32,
    pub name: String,
    /// Remembers the exact stored colour, so a row whose colour wasn't touched keeps it. Pass it back as it came.
    pub token: String,
}

impl From<&ColorBreakdownRow> for DialogBreakdownRow {
    fn from(r: &ColorBreakdownRow) -> Self {
        DialogBreakdownRow { argb: r.argb, name: r.name.clone(), token: r.token.clone() }
    }
}

impl From<&DialogBreakdownRow> for ColorBreakdownRow {
    fn from(r: &DialogBreakdownRow) -> Self {
        ColorBreakdownRow { argb: r.argb, name: r.name.clone(), token: r.token.clone() }
    }
}

#[derive(uniffi::Record, Clone)]
pub struct DialogColorBreakdown {
    pub rows: Vec<DialogBreakdownRow>,
    pub mix: bool,
    pub sixteen_bit: bool,
}

#[derive(uniffi::Record, Clone, PartialEq, Eq)]
pub struct DialogCoarseFine {
    pub enable_default_value_mapping: bool,
    pub resting_coarse: i32,
    pub resting_fine: i32,
}

/// The sheet's title and labels. A label that's empty means the filter doesn't have that field.
#[derive(uniffi::Record, Clone)]
pub struct DialogTagFilterLabels {
    pub title: String,
    pub automatic: String,
    pub open: String,
    pub close: String,
    pub function: String,
}

#[derive(uniffi::Record, Clone, PartialEq, Eq)]
pub struct DialogTagFilter {
    pub tag: String,
    pub automatic: bool,
    pub open_text: String,
    pub close_text: String,
    pub function_name: String,
}

impl From<&DialogTagFilter> for TagFilterDraft {
    fn from(d: &DialogTagFilter) -> Self {
        TagFilterDraft {
            tag: d.tag.clone(),
            automatic: d.automatic,
            open_text: d.open_text.clone(),
            close_text: d.close_text.clone(),
            function_name: d.function_name.clone(),
        }
    }
}

#[derive(uniffi::Record, Clone, Copy, PartialEq)]
pub struct DialogCurvePoint {
    pub x: f64,
    pub y: f64,
}

#[derive(uniffi::Record, Clone, PartialEq)]
pub struct DialogCurve {
    pub points: Vec<DialogCurvePoint>,
    pub library_reference_name: String,
    pub is_current_library_curve: bool,
}

impl From<CurveDraft> for DialogCurve {
    fn from(c: CurveDraft) -> Self {
        DialogCurve {
            points: c.points.into_iter().map(|(x, y)| DialogCurvePoint { x, y }).collect(),
            library_reference_name: c.library_reference_name,
            is_current_library_curve: c.is_current_library_curve,
        }
    }
}

impl From<&DialogCurve> for CurveDraft {
    fn from(c: &DialogCurve) -> Self {
        CurveDraft {
            points: c.points.iter().map(|p| (p.x, p.y)).collect(),
            library_reference_name: c.library_reference_name.clone(),
            is_current_library_curve: c.is_current_library_curve,
        }
    }
}

/// A filter as the dialogs see it.
#[derive(uniffi::Record, Clone)]
pub struct DialogFilter {
    pub id: String,
    pub kind: DialogFilterKind,
    /// `IDataFlowComponent.Name`.
    pub name: String,
}

fn config_of(world: &dyn DialogWorld, id: &Uuid) -> Result<FilterConfig, FfiError> {
    world.filter_config(id).ok_or_else(|| no_such("filter"))
}

fn coarse_fine_config(old: &CoarseFineConfig, draft: &DialogCoarseFine) -> CoarseFineConfig {
    CoarseFineConfig {
        enable_default_value_mapping: draft.enable_default_value_mapping,
        resting_coarse_value: draft.resting_coarse.clamp(0, 255) as u8,
        resting_fine_value: draft.resting_fine.clamp(0, 255) as u8,
        ..old.clone()
    }
}

#[uniffi::export]
impl DialogContext {
    /// F2: which sheet a filter's Configure opens.
    pub fn filter(&self, filter: String) -> Result<DialogFilter, FfiError> {
        let id = parse_id(&filter)?;
        self.world(|world| {
            let type_id = world.filter_type(&id).ok_or_else(|| no_such("filter"))?;
            let config = config_of(world, &id)?;
            Ok(DialogFilter { id: filter, kind: logic::filter_setup_kind(&type_id).into(), name: config.display_name() })
        })
    }

    // ---- Color Breakdown -----------------------------------------------------------------------------------------------------

    pub fn color_breakdown(&self, filter: String) -> Result<DialogColorBreakdown, FfiError> {
        let id = parse_id(&filter)?;
        self.world(|world| match config_of(world, &id)? {
            FilterConfig::ColorBreakdown(c) => Ok(DialogColorBreakdown {
                rows: logic::color_breakdown_rows(&c).iter().map(Into::into).collect(),
                mix: c.mix_colors,
                sixteen_bit: c.sixteen_bit,
            }),
            _ => Err(no_such("Color Breakdown filter")),
        })
    }

    /// F137.
    pub fn color_breakdown_templates(&self) -> Vec<String> {
        logic::COLOR_BREAKDOWN_TEMPLATES.iter().map(|t| t.to_string()).collect()
    }

    /// F138.
    pub fn color_breakdown_template(&self, template: String) -> Result<Vec<DialogBreakdownRow>, FfiError> {
        logic::color_breakdown_template_rows(&template)
            .map(|rows| rows.iter().map(Into::into).collect())
            .ok_or_else(|| FfiError::new("Error applying template: Unknown template."))
    }

    /// F143.
    pub fn color_breakdown_new_row(&self) -> DialogBreakdownRow {
        (&logic::color_breakdown_new_row()).into()
    }

    /// F144.
    pub fn color_breakdown_mix_note(&self, rows: Vec<DialogBreakdownRow>, mix: bool) -> Option<String> {
        let rows: Vec<ColorBreakdownRow> = rows.iter().map(Into::into).collect();
        logic::color_breakdown_mix_note(&rows, mix)
    }

    /// F146.
    pub fn apply_color_breakdown(&self, filter: String, draft: DialogColorBreakdown) -> Result<(), FfiError> {
        let id = parse_id(&filter)?;
        let rows: Vec<ColorBreakdownRow> = draft.rows.iter().map(Into::into).collect();
        self.world(|world| match config_of(world, &id)? {
            FilterConfig::ColorBreakdown(mut c) => {
                logic::color_breakdown_apply(&mut c, &rows, draft.mix, draft.sixteen_bit);
                world.set_filter_config(FilterConfig::ColorBreakdown(c));
                Ok(())
            }
            _ => Err(no_such("Color Breakdown filter")),
        })
    }

    // ---- Coarse Fine ---------------------------------------------------------------------------------------------------------

    pub fn coarse_fine(&self, filter: String) -> Result<DialogCoarseFine, FfiError> {
        let id = parse_id(&filter)?;
        self.world(|world| match config_of(world, &id)? {
            FilterConfig::CoarseFine(c) => Ok(DialogCoarseFine {
                enable_default_value_mapping: c.enable_default_value_mapping,
                resting_coarse: c.resting_coarse_value as i32,
                resting_fine: c.resting_fine_value as i32,
            }),
            _ => Err(no_such("Coarse Fine Breakdown filter")),
        })
    }

    /// F149.
    pub fn coarse_fine_issues(&self, draft: DialogCoarseFine) -> Vec<String> {
        logic::coarse_fine_issues(draft.resting_coarse, draft.resting_fine)
    }

    /// F150: used for OK, for each change in Live mode, and for restoring after Cancel. Swaps atomically, without rebuilding the graph.
    pub fn apply_coarse_fine(&self, filter: String, draft: DialogCoarseFine) -> Result<(), FfiError> {
        let id = parse_id(&filter)?;
        self.world(|world| match config_of(world, &id)? {
            FilterConfig::CoarseFine(c) => {
                world.set_filter_config(FilterConfig::CoarseFine(coarse_fine_config(&c, &draft)));
                Ok(())
            }
            _ => Err(no_such("Coarse Fine Breakdown filter")),
        })
    }

    // ---- Tag family ----------------------------------------------------------------------------------------------------------

    pub fn tag_filter_labels(&self, filter: String) -> Result<DialogTagFilterLabels, FfiError> {
        let id = parse_id(&filter)?;
        let type_id = self.world(|world| world.filter_type(&id)).ok_or_else(|| no_such("filter"))?;
        let labels = logic::tag_filter_labels(logic::filter_setup_kind(&type_id));
        Ok(DialogTagFilterLabels {
            title: labels.title.to_string(),
            automatic: labels.automatic.to_string(),
            open: labels.open.to_string(),
            close: labels.close.to_string(),
            function: labels.function.to_string(),
        })
    }

    pub fn tag_filter(&self, filter: String) -> Result<DialogTagFilter, FfiError> {
        let id = parse_id(&filter)?;
        self.world(|world| {
            let d = logic::tag_filter_draft(&config_of(world, &id)?);
            Ok(DialogTagFilter {
                tag: d.tag,
                automatic: d.automatic,
                open_text: d.open_text,
                close_text: d.close_text,
                function_name: d.function_name,
            })
        })
    }

    /// F231, F235: whether OK is available.
    pub fn tag_filter_can_apply(&self, filter: String, draft: DialogTagFilter) -> Result<bool, FfiError> {
        let id = parse_id(&filter)?;
        self.world(|world| Ok(logic::tag_filter_can_apply(&config_of(world, &id)?, &(&draft).into())))
    }

    /// F235.
    pub fn index_value_is_valid(&self, text: String) -> bool {
        logic::validate_index_value(&text).is_ok()
    }

    /// F231 to F236.
    pub fn apply_tag_filter(&self, filter: String, draft: DialogTagFilter) -> Result<(), FfiError> {
        let id = parse_id(&filter)?;
        self.world(|world| {
            let mut config = config_of(world, &id)?;
            logic::tag_filter_apply(&mut config, &(&draft).into()).map_err(FfiError::new)?;
            world.set_filter_config(config);
            Ok(())
        })
    }

    // ---- Curves --------------------------------------------------------------------------------------------------------------

    pub fn dimming_curve(&self, filter: String) -> Result<DialogCurve, FfiError> {
        let id = parse_id(&filter)?;
        self.world(|world| match config_of(world, &id)? {
            FilterConfig::DimmingCurve(c) => Ok(logic::curve_draft(&c.curve).into()),
            _ => Err(no_such("Dimming Curve filter")),
        })
    }

    /// F237.
    pub fn apply_dimming_curve(&self, filter: String, curve: DialogCurve) -> Result<(), FfiError> {
        let id = parse_id(&filter)?;
        self.world(|world| match config_of(world, &id)? {
            FilterConfig::DimmingCurve(mut c) => {
                logic::dimming_curve_apply(&mut c, &(&curve).into());
                world.set_filter_config(FilterConfig::DimmingCurve(c));
                Ok(())
            }
            _ => Err(no_such("Dimming Curve filter")),
        })
    }

    /// `new Curve()`: a ramp with x = y.
    pub fn default_curve(&self) -> DialogCurve {
        logic::curve_draft(&CurveData::default()).into()
    }

    /// Every filter of a module type, for the test window.
    pub fn filters_of_type(&self, type_id: String) -> Result<Vec<String>, FfiError> {
        let wanted = parse_id(&type_id)?;
        let system = self.core.system.read();
        Ok(system.filters.iter().filter(|f| f.type_id == wanted).map(|f| f.instance_id.to_string()).collect())
    }

    /// F2's type ids, for the test window. The order is upstream's list.
    pub fn filter_type_ids(&self) -> Vec<String> {
        ids::FILTER_DESCRIPTORS.iter().map(|d| d.type_id.to_string()).collect()
    }
}
