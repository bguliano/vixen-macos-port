//! Module type ids and descriptors, copied from each module's C# descriptor class.

use crate::xmlutil::ModuleIds;
use uuid::{uuid, Uuid};
use vixen_xml::XElement;

pub const GENERIC_SERIAL: Uuid = uuid!("d22c9233-16e9-4008-a215-e577f78a7a2e");
pub const DUMMY_LIGHTING: Uuid = uuid!("b6ee6308-189c-4268-8996-32a4bab8ab5f");
pub const E131: Uuid = uuid!("771d7eba-662a-48d5-aedb-445c8708e878");
pub const DDP: Uuid = uuid!("110b5fe9-cd4d-4e73-bb5b-67552a1ad6d1");
pub const RENARD: Uuid = uuid!("f66130a6-ff8a-48a7-bf1e-fac6cec2fc2c");
pub const DMX_USB_PRO: Uuid = uuid!("8abf27a7-3156-4f4c-9d43-ada45cdd6d0c");
pub const OPEN_DMX: Uuid = uuid!("7911568a-8eda-4d1d-9e72-be41d7a843e4");
pub const ELEXOL_ETHER_IO: Uuid = uuid!("2565e617-89db-4761-be8d-4d5574cafba9");
pub const RDS: Uuid = uuid!("692b66b9-e831-4f39-bb73-7653fa62b053");
pub const LAUNCHER: Uuid = uuid!("1c4bfd64-3172-43a7-a7fc-0a77c88b0489");
pub const DEBUG_CONTROLLER: Uuid = uuid!("f32bda77-28f0-46c4-a6bd-ba0ba73a1c13");

pub const COLOR_BREAKDOWN: Uuid = uuid!("ab38a16f-0de1-4f6e-a8c0-ae5b20d347e0");
pub const DIMMING_CURVE: Uuid = uuid!("2e40d6b1-43d2-4668-b63a-c600fadd7dd5");
pub const DIMMING_FILTER: Uuid = uuid!("61913688-ef44-473c-91d4-d256cca6cb91");
pub const COARSE_FINE_BREAKDOWN: Uuid = uuid!("7c1465df-054c-4875-ab61-d7e7f5236897");
pub const TAGGED_FILTER: Uuid = uuid!("fe552dcf-1bc0-4588-b23c-2600151d9fc5");
pub const PRISM_FILTER: Uuid = uuid!("2522bd94-0434-4728-9400-0e18b212dc21");
pub const SHUTTER_FILTER: Uuid = uuid!("72b86092-0a59-4e40-b0a8-9e5fad0b8eb3");
pub const COLOR_WHEEL_FILTER: Uuid = uuid!("18470c0c-1e75-464f-bd9f-a3e241cd1eb8");

/// What the app's lists and `ModuleStore.xml` need to know about a module type.
#[derive(Copy, Clone, Debug)]
pub struct ModuleDescriptor {
    pub type_id: Uuid,
    pub type_name: &'static str,
    pub description: &'static str,
    /// `dataModelType` on the `<Module>` element. `None` for a module with no data.
    pub data_model_type: Option<&'static str>,
    /// `assemblyName` on the `<Module>` element.
    pub assembly_name: &'static str,
}

/// Every controller module, in the order the hardware list shows them.
pub static CONTROLLER_DESCRIPTORS: [ModuleDescriptor; 11] = [
    ModuleDescriptor {
        type_id: GENERIC_SERIAL,
        type_name: "Generic Serial",
        description: "Generic Serial hardware module",
        data_model_type: Some("VixenModules.Output.GenericSerial.Data, GenericSerial"),
        assembly_name: "Module.Controller.GenericSerial",
    },
    ModuleDescriptor {
        type_id: RENARD,
        type_name: "Renard",
        description: "Renard hardware module",
        data_model_type: Some("VixenModules.Output.Renard.Data, Renard"),
        assembly_name: "Module.Controller.Renard",
    },
    ModuleDescriptor {
        type_id: DMX_USB_PRO,
        type_name: "DMX Pro",
        description: "DMX USB Pro hardware controller module",
        data_model_type: Some("VixenModules.Output.DmxUsbPro.Data, DMXUsbPro"),
        assembly_name: "Module.Controller.DMXUsbPro",
    },
    ModuleDescriptor {
        type_id: OPEN_DMX,
        type_name: "DMX Open",
        description: "Enttec Open DMX Output Module",
        data_model_type: Some("VixenModules.Controller.OpenDMX.OpenDMXData, OpenDMX"),
        assembly_name: "Module.Controller.OpenDMX",
    },
    ModuleDescriptor {
        type_id: E131,
        type_name: "Streaming ACN (E1.31)",
        description: "An output plugin that emits E1.31 data.",
        data_model_type: Some("VixenModules.Output.E131.E131ModuleDataModel, E131"),
        assembly_name: "Module.Controller.E131",
    },
    ModuleDescriptor {
        type_id: DDP,
        type_name: "DDP",
        description: "DDP Output Controller Module",
        data_model_type: Some("VixenModules.Output.DDP.DDPData, DDP"),
        assembly_name: "Module.Controller.DDP",
    },
    ModuleDescriptor {
        type_id: ELEXOL_ETHER_IO,
        type_name: "Elexol Ethernet I/O 24",
        description: "Elexol Ether I/O 24",
        data_model_type: Some("VixenModules.Output.ElexolEtherIO.ElexolEtherIOData, ElexolEtherIO"),
        assembly_name: "Module.Controller.ElexolEtherIO",
    },
    ModuleDescriptor {
        type_id: RDS,
        type_name: "RDS",
        description: "Generic Windows Command and RDS hardware module",
        data_model_type: Some("VixenModules.Output.CommandController.Data, RDSController"),
        assembly_name: "Module.Controller.RDSController",
    },
    ModuleDescriptor {
        type_id: LAUNCHER,
        type_name: "Launcher",
        description: "Generic Windows Command / App Launcher",
        data_model_type: Some("VixenModules.Output.LauncherController.Data, LauncherController"),
        assembly_name: "Module.Controller.LauncherController",
    },
    ModuleDescriptor {
        type_id: DUMMY_LIGHTING,
        type_name: "Dummy Lighting (for testing)",
        description: "A dummy/test output module that will display lighting commands in a new window.",
        data_model_type: Some("VixenModules.Output.DummyLighting.DummyLightingData, DummyLighting"),
        assembly_name: "Module.Controller.DummyLighting",
    },
    ModuleDescriptor {
        type_id: DEBUG_CONTROLLER,
        type_name: "Debugging Values",
        description: "A debugging controller that will display raw values in an output window.",
        data_model_type: None,
        assembly_name: "Module.Controller.DebugController",
    },
];

/// Every output filter module.
pub static FILTER_DESCRIPTORS: [ModuleDescriptor; 8] = [
    ModuleDescriptor {
        type_id: COLOR_BREAKDOWN,
        type_name: "Color Breakdown",
        description: "An output filter that breaks down color intents into discrete color components.",
        data_model_type: Some("VixenModules.OutputFilter.ColorBreakdown.ColorBreakdownData, ColorBreakdown"),
        assembly_name: "Module.OutputFilter.ColorBreakdown",
    },
    ModuleDescriptor {
        type_id: DIMMING_CURVE,
        type_name: "Dimming Curve",
        description: "An output filter that translates lighting intensity values according to a curve (to compensate for non-linear lighting response).",
        data_model_type: Some("VixenModules.OutputFilter.DimmingCurve.DimmingCurveData, DimmingCurve"),
        assembly_name: "Module.OutputFilter.DimmingCurve",
    },
    ModuleDescriptor {
        type_id: DIMMING_FILTER,
        type_name: "Dimming Filter",
        description: "An output filter that converts color intents into dimmer intents.",
        data_model_type: Some("VixenModules.OutputFilter.DimmingFilter.DimmingFilterData, DimmingFilter"),
        assembly_name: "Module.OutputFilter.DimmingFilter",
    },
    ModuleDescriptor {
        type_id: COARSE_FINE_BREAKDOWN,
        type_name: "Coarse Fine Breakdown",
        description: "An output filter that breaks an intent value into a coarse (high byte) and a fine (low byte).",
        data_model_type: Some("VixenModules.OutputFilter.CoarseFineBreakdown.CoarseFineBreakdownData, CoarseFineBreakdown"),
        assembly_name: "Module.OutputFilter.CoarseFineBreakdown",
    },
    ModuleDescriptor {
        type_id: TAGGED_FILTER,
        type_name: "Tag Filter",
        description: "An output filter that filters intents based on a tag.",
        data_model_type: Some("VixenModules.OutputFilter.TaggedFilter.TaggedFilterData, TaggedFilter"),
        assembly_name: "Module.OutputFilter.TaggedFilter",
    },
    ModuleDescriptor {
        type_id: PRISM_FILTER,
        type_name: "Prism Filter",
        description: "An output filter that converts prism intents into open prism commands.",
        data_model_type: Some("VixenModules.OutputFilter.PrismFilter.PrismFilterData, PrismFilter"),
        assembly_name: "Module.OutputFilter.PrismFilter",
    },
    ModuleDescriptor {
        type_id: SHUTTER_FILTER,
        type_name: "Shutter Filter",
        description: "An output filter that converts color intents into shutter commands.",
        data_model_type: Some("VixenModules.OutputFilter.ShutterFilter.ShutterFilterData, ShutterFilter"),
        assembly_name: "Module.OutputFilter.ShutterFilter",
    },
    ModuleDescriptor {
        type_id: COLOR_WHEEL_FILTER,
        type_name: "Color Wheel Filter",
        description: "An output filter that converts color intents into color wheel index commands.",
        data_model_type: Some("VixenModules.OutputFilter.ColorWheelFilter.ColorWheelFilterData, ColorWheelFilter"),
        assembly_name: "Module.OutputFilter.ColorWheelFilter",
    },
];

pub fn controller_descriptor(type_id: &Uuid) -> Option<&'static ModuleDescriptor> {
    CONTROLLER_DESCRIPTORS.iter().find(|d| d.type_id == *type_id)
}

pub fn filter_descriptor(type_id: &Uuid) -> Option<&'static ModuleDescriptor> {
    FILTER_DESCRIPTORS.iter().find(|d| d.type_id == *type_id)
}

/// Wraps module data in the `<Module>` element `ModuleStore.xml` holds it in.
pub fn module_element(descriptor: &ModuleDescriptor, ids: &ModuleIds, data: XElement) -> XElement {
    let mut e = XElement::new("", "Module");
    e.set_attr("dataModelType", descriptor.data_model_type.unwrap_or_default());
    e.set_attr("moduleType", ids.type_id.to_string());
    e.set_attr("moduleInstance", ids.instance_id.to_string());
    e.set_attr("assemblyName", descriptor.assembly_name);
    e.push(data);
    e
}
