//! The hardware output layer: turns each element's per-frame state into controller output, and drives real hardware.
//!
//! - [`OutputGraph`] compiles the data flow patches (element -> filters -> controller outputs) and fills each controller's
//!   commands for a frame.
//! - [`OutputModule`] is one controller's driver, with a typed [`ControllerConfig`] that reads and writes its `ModuleStore.xml`.
//! - [`ControllerRunner`] runs each enabled controller on its own thread with a latest-frame mailbox, so slow hardware can't hold
//!   the engine up.
//!
//! Numbers here decide what reaches the lights, so the conversions follow the C#, including its oddities. See
//! docs/PORTING_GUIDE.md.

pub mod command;
pub mod dotnet;
pub mod filters;
pub mod flow;
pub mod graph;
pub mod ids;
pub mod modules;
pub mod net;
pub mod policy;
pub mod runner;
pub mod serial;
pub mod xmlutil;

pub use command::OutputCommand;
pub use dotnet::DotNetColor;
pub use filters::FilterConfig;
pub use flow::DataFlowType;
pub use graph::{
    ControllerDef, FilterDef, GraphDefinition, GraphWarning, ModuleDataLookup, NoModuleData, OutputAddress, OutputDef,
    OutputGraph, PatchDef,
};
pub use modules::{create_module, ControllerConfig, ModuleShared, ModuleStatus, OutputModule};
pub use policy::{Combinator, DataPolicy, Evaluator};
pub use runner::{ControllerRunner, ControllerStats, ControllerTimings, Mailbox, TimingSummary};
pub use xmlutil::ModuleIds;
