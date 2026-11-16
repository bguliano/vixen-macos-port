//! The output filter modules: typed configuration for each, and the runtime the graph evaluates.

pub mod coarse_fine;
pub mod color_breakdown;
pub mod dimming_curve;
pub mod tagged;

pub use coarse_fine::CoarseFineConfig;
pub use color_breakdown::{rgb_to_rgbw, ColorBreakdownConfig, ColorBreakdownItem};
pub use dimming_curve::{CurveData, DimmingCurveConfig, PointPairData};
pub use tagged::{
    ColorWheelFilterConfig, DimmingFilterConfig, FixtureColorWheelEntry, PrismFilterConfig, ShutterFilterConfig,
    TaggedFilterConfig,
};

use crate::flow::{DataFlowType, Flow};
use crate::ids;
use crate::xmlutil::{unwrap_module, ModuleIds};
use coarse_fine::CoarseFineRt;
use color_breakdown::ColorBreakdownRt;
use dimming_curve::DimmingCurveRt;
use tagged::TaggedRt;
use uuid::Uuid;
use vixen_xml::XElement;

/// The typed data of any output filter module.
#[derive(Clone, Debug, PartialEq)]
pub enum FilterConfig {
    ColorBreakdown(ColorBreakdownConfig),
    DimmingCurve(DimmingCurveConfig),
    Dimming(DimmingFilterConfig),
    CoarseFine(CoarseFineConfig),
    Tagged(TaggedFilterConfig),
    Prism(PrismFilterConfig),
    Shutter(ShutterFilterConfig),
    ColorWheel(ColorWheelFilterConfig),
}

impl FilterConfig {
    /// The data a new module instance gets. `None` for a type this port doesn't have.
    pub fn new_default(type_id: &Uuid, instance_id: Uuid) -> Option<FilterConfig> {
        let m = ModuleIds::new(*type_id, instance_id);
        Some(match *type_id {
            ids::COLOR_BREAKDOWN => FilterConfig::ColorBreakdown(ColorBreakdownConfig { ids: m, ..Default::default() }),
            ids::DIMMING_CURVE => FilterConfig::DimmingCurve(DimmingCurveConfig { ids: m, ..Default::default() }),
            ids::DIMMING_FILTER => FilterConfig::Dimming(DimmingFilterConfig { ids: m, ..Default::default() }),
            ids::COARSE_FINE_BREAKDOWN => FilterConfig::CoarseFine(CoarseFineConfig { ids: m, ..Default::default() }),
            ids::TAGGED_FILTER => FilterConfig::Tagged(TaggedFilterConfig { ids: m, ..Default::default() }),
            ids::PRISM_FILTER => FilterConfig::Prism(PrismFilterConfig { ids: m, ..Default::default() }),
            ids::SHUTTER_FILTER => FilterConfig::Shutter(ShutterFilterConfig { ids: m, ..Default::default() }),
            ids::COLOR_WHEEL_FILTER => FilterConfig::ColorWheel(ColorWheelFilterConfig { ids: m, ..Default::default() }),
            _ => return None,
        })
    }

    /// Reads a filter's module data, given as the `<Module>` element or the data inside it. The ids on a
    /// `<Module>` element win, as they do in `XmlModuleDataModelSerializer.ReadObject`.
    pub fn from_xml(type_id: &Uuid, e: &XElement) -> Option<FilterConfig> {
        let (data, wrapper_ids) = unwrap_module(e);
        let mut config = match *type_id {
            ids::COLOR_BREAKDOWN => FilterConfig::ColorBreakdown(ColorBreakdownConfig::from_xml(data)),
            ids::DIMMING_CURVE => FilterConfig::DimmingCurve(DimmingCurveConfig::from_xml(data)),
            ids::DIMMING_FILTER => FilterConfig::Dimming(DimmingFilterConfig::from_xml(data)),
            ids::COARSE_FINE_BREAKDOWN => FilterConfig::CoarseFine(CoarseFineConfig::from_xml(data)),
            ids::TAGGED_FILTER => FilterConfig::Tagged(TaggedFilterConfig::from_xml(data)),
            ids::PRISM_FILTER => FilterConfig::Prism(PrismFilterConfig::from_xml(data)),
            ids::SHUTTER_FILTER => FilterConfig::Shutter(ShutterFilterConfig::from_xml(data)),
            ids::COLOR_WHEEL_FILTER => FilterConfig::ColorWheel(ColorWheelFilterConfig::from_xml(data)),
            _ => return None,
        };
        if let Some(w) = wrapper_ids {
            *config.ids_mut() = w;
        }
        Some(config)
    }

    /// The data element, for inside the `<Module>` element.
    pub fn to_xml(&self) -> XElement {
        match self {
            FilterConfig::ColorBreakdown(c) => c.to_xml(),
            FilterConfig::DimmingCurve(c) => c.to_xml(),
            FilterConfig::Dimming(c) => c.to_xml(),
            FilterConfig::CoarseFine(c) => c.to_xml(),
            FilterConfig::Tagged(c) => c.to_xml(),
            FilterConfig::Prism(c) => c.to_xml(),
            FilterConfig::Shutter(c) => c.to_xml(),
            FilterConfig::ColorWheel(c) => c.to_xml(),
        }
    }

    pub fn ids(&self) -> &ModuleIds {
        match self {
            FilterConfig::ColorBreakdown(c) => &c.ids,
            FilterConfig::DimmingCurve(c) => &c.ids,
            FilterConfig::Dimming(c) => &c.ids,
            FilterConfig::CoarseFine(c) => &c.ids,
            FilterConfig::Tagged(c) => &c.ids,
            FilterConfig::Prism(c) => &c.ids,
            FilterConfig::Shutter(c) => &c.ids,
            FilterConfig::ColorWheel(c) => &c.ids,
        }
    }

    pub fn ids_mut(&mut self) -> &mut ModuleIds {
        match self {
            FilterConfig::ColorBreakdown(c) => &mut c.ids,
            FilterConfig::DimmingCurve(c) => &mut c.ids,
            FilterConfig::Dimming(c) => &mut c.ids,
            FilterConfig::CoarseFine(c) => &mut c.ids,
            FilterConfig::Tagged(c) => &mut c.ids,
            FilterConfig::Prism(c) => &mut c.ids,
            FilterConfig::Shutter(c) => &mut c.ids,
            FilterConfig::ColorWheel(c) => &mut c.ids,
        }
    }

    /// `IDataFlowComponent.Name`: the type's name, with the tag for the tagged filters.
    pub fn display_name(&self) -> String {
        let type_name = ids::filter_descriptor(&self.ids().type_id).map_or("", |d| d.type_name);
        let tag = match self {
            FilterConfig::Dimming(c) => &c.tag,
            FilterConfig::Tagged(c) => &c.tag,
            FilterConfig::Prism(c) => &c.tag,
            FilterConfig::Shutter(c) => &c.tag,
            FilterConfig::ColorWheel(c) => &c.tag,
            _ => return type_name.to_string(),
        };
        format!("{type_name}: {}", tag.as_deref().unwrap_or(""))
    }

    /// `InputDataType`.
    pub fn input_data_type(&self) -> DataFlowType {
        DataFlowType::MultipleIntents
    }

    /// `OutputDataType`.
    pub fn output_data_type(&self) -> DataFlowType {
        match self {
            FilterConfig::ColorBreakdown(_) => DataFlowType::SingleCommand,
            FilterConfig::CoarseFine(_) => DataFlowType::MultipleCommands,
            _ => DataFlowType::MultipleIntents,
        }
    }

    /// `IDataFlowOutput.Name` of each output.
    pub fn output_names(&self) -> Vec<String> {
        let tag = |t: &Option<String>| t.clone().unwrap_or_default();
        match self {
            FilterConfig::ColorBreakdown(c) => c.breakdown_items.iter().map(|i| i.name.clone()).collect(),
            FilterConfig::DimmingCurve(_) => vec!["Dimming Curve Output".to_string()],
            FilterConfig::CoarseFine(_) => vec!["Coarse-Fine Breakdown".to_string(); 2],
            FilterConfig::Dimming(c) => vec![tag(&c.tag)],
            FilterConfig::Tagged(c) => vec![tag(&c.tag)],
            FilterConfig::Prism(c) => vec![tag(&c.tag)],
            FilterConfig::Shutter(c) => vec![tag(&c.tag)],
            FilterConfig::ColorWheel(c) => vec![tag(&c.tag)],
        }
    }
}

/// A compiled filter.
#[derive(Clone, Debug)]
pub(crate) enum FilterRt {
    ColorBreakdown(ColorBreakdownRt),
    DimmingCurve(Box<DimmingCurveRt>),
    Tagged(TaggedRt),
    CoarseFine(CoarseFineRt),
}

impl FilterRt {
    pub(crate) fn new(config: &FilterConfig) -> FilterRt {
        match config {
            FilterConfig::ColorBreakdown(c) => FilterRt::ColorBreakdown(ColorBreakdownRt::new(c)),
            FilterConfig::DimmingCurve(c) => FilterRt::DimmingCurve(Box::new(DimmingCurveRt::new(c))),
            FilterConfig::CoarseFine(c) => FilterRt::CoarseFine(CoarseFineRt::new(c)),
            FilterConfig::Tagged(c) => FilterRt::Tagged(TaggedRt::tagged(c)),
            FilterConfig::Dimming(c) => FilterRt::Tagged(TaggedRt::dimming(c)),
            FilterConfig::Prism(c) => FilterRt::Tagged(TaggedRt::prism(c)),
            FilterConfig::Shutter(c) => FilterRt::Tagged(TaggedRt::shutter(c)),
            FilterConfig::ColorWheel(c) => FilterRt::Tagged(TaggedRt::color_wheel(c)),
        }
    }

    pub(crate) fn output_count(&self) -> usize {
        match self {
            FilterRt::ColorBreakdown(f) => f.output_count(),
            FilterRt::CoarseFine(_) => 2,
            FilterRt::DimmingCurve(_) | FilterRt::Tagged(_) => 1,
        }
    }

    /// `OutputFilterModuleInstanceBase.Update`: dispatches on the data's type. Each module only overrides the
    /// handlers it listed in the C#, and the rest are empty, so that data leaves its outputs as they were.
    #[inline]
    pub(crate) fn update(&mut self, flow: Flow<'_>) {
        match self {
            FilterRt::ColorBreakdown(f) => match flow {
                Flow::Intents(s) => f.update(Some(s)),
                Flow::NullIntents => f.update(None),
                _ => {}
            },
            FilterRt::DimmingCurve(f) => match flow {
                Flow::Intents(s) => f.update_intents(Some(s)),
                Flow::NullIntents => f.update_intents(None),
                Flow::Command(c) => f.update_command(c),
                _ => {}
            },
            FilterRt::Tagged(f) => match flow {
                Flow::Intents(s) => f.update(Some(s)),
                Flow::NullIntents => f.update(None),
                _ => {}
            },
            FilterRt::CoarseFine(f) => match flow {
                Flow::Intents(s) => f.update_intents(Some(s)),
                Flow::NullIntents => f.update_intents(None),
                Flow::Command(c) => f.update_command(c),
                _ => {}
            },
        }
    }

    /// Reads an output's `Data`. An index out of range reads as null, but the graph never builds one.
    #[inline]
    pub(crate) fn output(&self, index: usize) -> Flow<'_> {
        match self {
            FilterRt::ColorBreakdown(f) => f.output(index).map_or(Flow::Null, Flow::Command),
            FilterRt::DimmingCurve(f) => f.output(),
            FilterRt::Tagged(f) => f.output(),
            FilterRt::CoarseFine(f) => f.output(index).map_or(Flow::Null, Flow::Commands),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_flow_types_match_the_modules() {
        let config = |t: &Uuid| FilterConfig::new_default(t, Uuid::nil()).unwrap();
        assert_eq!(config(&ids::COLOR_BREAKDOWN).display_name(), "Color Breakdown");
        assert_eq!(config(&ids::COLOR_BREAKDOWN).output_data_type(), DataFlowType::SingleCommand);
        assert_eq!(config(&ids::COARSE_FINE_BREAKDOWN).output_data_type(), DataFlowType::MultipleCommands);
        assert_eq!(config(&ids::COARSE_FINE_BREAKDOWN).output_names().len(), 2);
        assert_eq!(config(&ids::DIMMING_CURVE).output_data_type(), DataFlowType::MultipleIntents);
        assert_eq!(config(&ids::DIMMING_CURVE).output_names(), ["Dimming Curve Output"]);
        let tagged = FilterConfig::Tagged(TaggedFilterConfig { tag: Some("Pan".to_string()), ..Default::default() });
        assert_eq!(tagged.display_name(), "Tag Filter: Pan");
        assert_eq!(tagged.output_names(), ["Pan"]);
        assert_eq!(tagged.input_data_type(), DataFlowType::MultipleIntents);
    }
}
