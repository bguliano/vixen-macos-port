//! The data flow graph compiled into a flat program: the port of `DataFlowManager`, `FilterManager` and the parts of
//! `OutputController` that turn data into commands.
//!
//! In the C#, each frame walks `IDataFlowComponent` objects, and reading a component's output returns a data object that
//! the next component dispatches on. Here `build` resolves the patches once into filter nodes in dependency order and one
//! source per controller output, and `evaluate` runs that list without allocating.

use crate::command::OutputCommand;
use crate::filters::{FilterConfig, FilterRt};
use crate::flow::{Flow, States};
use crate::modules::ControllerConfig;
use crate::policy::{DataPolicy, PolicyState};
use std::collections::HashMap;
use uuid::Uuid;
use vixen_core::{CombinedState, DiscreteValue};
use vixen_xml::XElement;

/// One controller output, as in `SystemConfig.xml`. Its index is its position in the controller's list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputDef {
    pub id: Uuid,
    pub name: String,
}

/// A `<Controller>` from `SystemConfig.xml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControllerDef {
    pub id: Uuid,
    pub name: String,
    pub hardware_type_id: Uuid,
    pub hardware_instance_id: Uuid,
    pub outputs: Vec<OutputDef>,
}

/// A `<FilterNode>` from `SystemConfig.xml`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FilterDef {
    pub type_id: Uuid,
    pub instance_id: Uuid,
}

/// A `<Patch>` from `SystemConfig.xml`: `component_id` reads its data from output `output_index` of `source_id`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatchDef {
    pub component_id: Uuid,
    pub source_id: Option<Uuid>,
    pub output_index: i32,
}

/// Everything the graph is built from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphDefinition {
    /// The dense element index is the position in this list. `evaluate` takes one state per entry.
    pub elements: Vec<Uuid>,
    pub controllers: Vec<ControllerDef>,
    pub filters: Vec<FilterDef>,
    pub patches: Vec<PatchDef>,
}

/// Finds a module's data, as `ModuleStore.xml` has it. The result may be the `<Module>` element or the data inside it.
pub trait ModuleDataLookup {
    fn module_data(&self, type_id: &Uuid, instance_id: &Uuid) -> Option<&XElement>;
}

impl ModuleDataLookup for HashMap<(Uuid, Uuid), XElement> {
    fn module_data(&self, type_id: &Uuid, instance_id: &Uuid) -> Option<&XElement> {
        self.get(&(*type_id, *instance_id))
    }
}

/// A lookup with nothing in it: every module gets the data a new instance would.
#[derive(Copy, Clone, Debug, Default)]
pub struct NoModuleData;

impl ModuleDataLookup for NoModuleData {
    fn module_data(&self, _type_id: &Uuid, _instance_id: &Uuid) -> Option<&XElement> {
        None
    }
}

/// A controller output's address in the graph.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OutputAddress {
    pub controller: usize,
    pub output: usize,
}

/// Something the graph couldn't use. None of these stop it from being built, and every output still gets a command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphWarning {
    /// `XmlOutputFilterSerializer.ReadObject` gets null for a filter module that doesn't exist, so the filter is dropped.
    UnknownFilterType { type_id: Uuid, instance_id: Uuid },
    /// The controller has outputs but no module in this port. Its outputs still evaluate, with the usual 8-bit policy.
    UnknownControllerType { id: Uuid, type_id: Uuid },
    /// The patch's component is neither an output nor a filter that exists.
    UnknownPatchComponent { component_id: Uuid },
    /// The patch has no source, or its source isn't something that exists. The C# throws while loading.
    UnknownPatchSource { component_id: Uuid, source_id: Option<Uuid> },
    /// The source has no output with this index, so `DataFlowManager.Initialize` skips the patch.
    PatchOutputOutOfRange { component_id: Uuid, source_id: Uuid, output_index: i32 },
    /// `SetComponentSource` refuses a patch that would make a loop.
    CircularPatch { component_id: Uuid, source_id: Uuid },
}

/// What a component reads from.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Source {
    /// Nothing is patched. An output with no source has a null command, and a filter with no source never runs.
    None,
    /// `overlay` is an index into `overlays`, or `NO_OVERLAY`.
    Element {
        index: u32,
        overlay: u32,
    },
    Filter {
        node: u32,
        output: u32,
    },
}

const NO_OVERLAY: u32 = u32::MAX;

/// A filter and where it reads from.
#[derive(Clone, Debug)]
struct Node {
    filter: FilterRt,
    source: Source,
    instance_id: Uuid,
}

/// A controller and its output program.
#[derive(Clone, Debug)]
pub struct CompiledController {
    pub id: Uuid,
    pub name: String,
    pub hardware_type_id: Uuid,
    pub hardware_instance_id: Uuid,
    pub output_ids: Vec<Uuid>,
    pub output_names: Vec<String>,
    /// `None` when the module type isn't one this port has.
    pub config: Option<ControllerConfig>,
    policy: PolicyState,
    bindings: Vec<Source>,
    commands: Vec<OutputCommand>,
}

impl CompiledController {
    /// This frame's command for each output.
    #[inline]
    pub fn commands(&self) -> &[OutputCommand] {
        &self.commands
    }

    pub fn data_policy(&self) -> DataPolicy {
        self.policy.policy()
    }

    pub fn output_count(&self) -> usize {
        self.commands.len()
    }
}

/// An element's discrete values, while a dimming curve is changing them in place. See `DimmingCurveRt::update_element_in_place`.
#[derive(Clone, Debug)]
struct Overlay {
    element: u32,
    values: Vec<DiscreteValue>,
}

/// The compiled graph. Build it once when the patching changes, then call `evaluate` once per frame.
#[derive(Clone, Debug)]
pub struct OutputGraph {
    elements: Vec<Uuid>,
    element_lookup: HashMap<Uuid, u32>,
    controllers: Vec<CompiledController>,
    /// Filters that can receive data, parents before children, followed by those that can't.
    nodes: Vec<Node>,
    /// How many of `nodes` run. Anything after that has no element above it, so `FilterManager` never updates it and its
    /// outputs keep their initial values.
    active_nodes: usize,
    filter_configs: HashMap<Uuid, FilterConfig>,
    overlays: Vec<Overlay>,
    empty_state: CombinedState,
    warnings: Vec<GraphWarning>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Component {
    Element(u32),
    Output { controller: u32, output: u32 },
    Filter(u32),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum SourceRef {
    Element(u32),
    Filter { filter: u32, output: u32 },
}

impl OutputGraph {
    /// Compiles a graph. Anything wrong with the data is recorded in `warnings`, and the rest still works.
    pub fn build(def: &GraphDefinition, modules: &dyn ModuleDataLookup) -> OutputGraph {
        let mut warnings = Vec::new();

        // Filters. A filter with no stored data gets what `_GetOrCreateAsInstanceData` creates.
        let mut filters: Vec<Option<(FilterDef, FilterConfig, FilterRt)>> = Vec::with_capacity(def.filters.len());
        for f in &def.filters {
            let config = match modules.module_data(&f.type_id, &f.instance_id) {
                Some(data) => FilterConfig::from_xml(&f.type_id, data),
                None => FilterConfig::new_default(&f.type_id, f.instance_id),
            };
            match config {
                Some(mut config) => {
                    // The node's ids are what the module instance has, whatever the data says.
                    *config.ids_mut() = crate::xmlutil::ModuleIds::new(f.type_id, f.instance_id);
                    let rt = FilterRt::new(&config);
                    filters.push(Some((*f, config, rt)));
                }
                None => {
                    warnings.push(GraphWarning::UnknownFilterType { type_id: f.type_id, instance_id: f.instance_id });
                    filters.push(None);
                }
            }
        }

        // `DataFlowManager._componentLookup`, filled in the order `VixenSystem.LoadSystemConfig` adds components.
        let mut lookup: HashMap<Uuid, Component> = HashMap::with_capacity(def.elements.len() + def.filters.len());
        let mut element_lookup = HashMap::with_capacity(def.elements.len());
        for (i, id) in def.elements.iter().enumerate() {
            lookup.insert(*id, Component::Element(i as u32));
            element_lookup.insert(*id, i as u32);
        }
        for (c, controller) in def.controllers.iter().enumerate() {
            for (o, output) in controller.outputs.iter().enumerate() {
                lookup.insert(output.id, Component::Output { controller: c as u32, output: o as u32 });
            }
        }
        for (i, f) in filters.iter().enumerate() {
            if let Some((d, _, _)) = f {
                lookup.insert(d.instance_id, Component::Filter(i as u32));
            }
        }

        // `DataFlowManager.Initialize`: the patches in order, so a later patch on a component replaces an earlier one.
        let mut filter_source: Vec<Option<SourceRef>> = vec![None; filters.len()];
        let mut output_source: Vec<Vec<Option<SourceRef>>> =
            def.controllers.iter().map(|c| vec![None; c.outputs.len()]).collect();
        let mut filter_patch_order: Vec<u32> = Vec::with_capacity(filters.len());
        for patch in &def.patches {
            let Some(component) = lookup.get(&patch.component_id).copied() else {
                warnings.push(GraphWarning::UnknownPatchComponent { component_id: patch.component_id });
                continue;
            };
            let source_component = patch.source_id.and_then(|id| lookup.get(&id).copied());
            let (Some(source_component), Some(source_id)) = (source_component, patch.source_id) else {
                warnings.push(GraphWarning::UnknownPatchSource { component_id: patch.component_id, source_id: patch.source_id });
                continue;
            };
            let source = match source_component {
                Component::Element(e) => (patch.output_index == 0).then_some(SourceRef::Element(e)),
                Component::Filter(f) => {
                    let count = filters[f as usize].as_ref().map_or(0, |(_, _, rt)| rt.output_count());
                    (patch.output_index >= 0 && (patch.output_index as usize) < count)
                        .then_some(SourceRef::Filter { filter: f, output: patch.output_index as u32 })
                }
                // An output adapter has no `Outputs`, so nothing can read from one.
                Component::Output { .. } => None,
            };
            let Some(source) = source else {
                warnings.push(GraphWarning::PatchOutputOutOfRange {
                    component_id: patch.component_id,
                    source_id,
                    output_index: patch.output_index,
                });
                continue;
            };
            match component {
                // An element's `Source` setter does nothing.
                Component::Element(_) => {}
                Component::Output { controller, output } => output_source[controller as usize][output as usize] = Some(source),
                Component::Filter(f) => {
                    if creates_loop(f, source, &filter_source) {
                        warnings.push(GraphWarning::CircularPatch { component_id: patch.component_id, source_id });
                        continue;
                    }
                    filter_source[f as usize] = Some(source);
                    filter_patch_order.push(f);
                }
            }
        }

        // `FilterManager._rootFilters` and `_filterChildren`. A filter runs if it hangs off an element, directly or not.
        let mut children: Vec<Vec<u32>> = vec![Vec::new(); filters.len()];
        let mut roots: Vec<u32> = Vec::new();
        let mut seen = vec![false; filters.len()];
        for &f in &filter_patch_order {
            if std::mem::replace(&mut seen[f as usize], true) {
                continue;
            }
            match filter_source[f as usize] {
                Some(SourceRef::Element(_)) => roots.push(f),
                Some(SourceRef::Filter { filter, .. }) => children[filter as usize].push(f),
                None => {}
            }
        }
        // Parents before children, so that one pass in this order always sees this frame's data.
        let mut order: Vec<u32> = Vec::with_capacity(filters.len());
        let mut stack: Vec<u32> = Vec::new();
        for &root in &roots {
            stack.push(root);
            while let Some(f) = stack.pop() {
                order.push(f);
                stack.extend(children[f as usize].iter().rev());
            }
        }
        let active_nodes = order.len();
        let mut placed = vec![false; filters.len()];
        for &f in &order {
            placed[f as usize] = true;
        }
        for (i, f) in filters.iter().enumerate() {
            if !placed[i] && f.is_some() {
                order.push(i as u32);
            }
        }
        let mut node_of: Vec<u32> = vec![u32::MAX; filters.len()];
        for (n, &f) in order.iter().enumerate() {
            node_of[f as usize] = n as u32;
        }

        // A dimming curve writes discrete intensities back into the element's state, so anything else that reads that
        // element later in the frame sees them. Only elements with another consumer need the extra copy.
        let mut consumers = vec![0u32; def.elements.len()];
        let mut has_curve = vec![false; def.elements.len()];
        for (f, source) in filter_source.iter().enumerate() {
            if let Some(SourceRef::Element(e)) = source {
                consumers[*e as usize] += 1;
                if matches!(filters[f].as_ref().map(|t| &t.2), Some(FilterRt::DimmingCurve(_))) {
                    has_curve[*e as usize] = true;
                }
            }
        }
        for sources in &output_source {
            for source in sources {
                if let Some(SourceRef::Element(e)) = source {
                    consumers[*e as usize] += 1;
                }
            }
        }
        let mut overlay_of = vec![NO_OVERLAY; def.elements.len()];
        let mut overlays = Vec::new();
        for e in 0..def.elements.len() {
            if has_curve[e] && consumers[e] > 1 {
                overlay_of[e] = overlays.len() as u32;
                overlays.push(Overlay { element: e as u32, values: Vec::with_capacity(4) });
            }
        }

        let resolve = |source: Option<SourceRef>| match source {
            None => Source::None,
            Some(SourceRef::Element(e)) => Source::Element { index: e, overlay: overlay_of[e as usize] },
            Some(SourceRef::Filter { filter, output }) => Source::Filter { node: node_of[filter as usize], output },
        };

        let mut filter_configs = HashMap::with_capacity(filters.len());
        let mut slots: Vec<Option<Node>> = (0..order.len()).map(|_| None).collect();
        for (i, f) in filters.into_iter().enumerate() {
            if let Some((d, config, rt)) = f {
                filter_configs.insert(d.instance_id, config);
                slots[node_of[i] as usize] =
                    Some(Node { filter: rt, source: resolve(filter_source[i]), instance_id: d.instance_id });
            }
        }
        let nodes: Vec<Node> = slots.into_iter().flatten().collect();

        let mut controllers = Vec::with_capacity(def.controllers.len());
        for (c, d) in def.controllers.iter().enumerate() {
            let config = match modules.module_data(&d.hardware_type_id, &d.hardware_instance_id) {
                Some(data) => ControllerConfig::from_xml(&d.hardware_type_id, data),
                None => ControllerConfig::new_default(&d.hardware_type_id, d.hardware_instance_id),
            };
            let config = config.map(|mut config| {
                *config.ids_mut() = crate::xmlutil::ModuleIds::new(d.hardware_type_id, d.hardware_instance_id);
                config
            });
            if config.is_none() {
                warnings.push(GraphWarning::UnknownControllerType { id: d.id, type_id: d.hardware_type_id });
            }
            let policy = config.as_ref().map_or(DataPolicy::HIGHEST_WINS_8BIT, ControllerConfig::data_policy);
            controllers.push(CompiledController {
                id: d.id,
                name: d.name.clone(),
                hardware_type_id: d.hardware_type_id,
                hardware_instance_id: d.hardware_instance_id,
                output_ids: d.outputs.iter().map(|o| o.id).collect(),
                output_names: d.outputs.iter().map(|o| o.name.clone()).collect(),
                config,
                policy: PolicyState::new(policy),
                bindings: output_source[c].iter().map(|s| resolve(*s)).collect(),
                commands: vec![OutputCommand::None; d.outputs.len()],
            });
        }

        OutputGraph {
            elements: def.elements.clone(),
            element_lookup,
            controllers,
            nodes,
            active_nodes,
            filter_configs,
            overlays,
            empty_state: CombinedState::default(),
            warnings,
        }
    }

    /// One frame: `Filters.Update()`, then `UpdateCommands()` on every controller. `states` has one entry per element, and
    /// an element past the end of it counts as having no state. Allocates nothing once the lists have grown to fit.
    pub fn evaluate(&mut self, states: &[CombinedState]) {
        let OutputGraph { controllers, nodes, active_nodes, overlays, empty_state, .. } = self;
        for overlay in overlays.iter_mut() {
            overlay.values.clear();
            if let Some(state) = states.get(overlay.element as usize) {
                overlay.values.extend_from_slice(&state.discrete);
            }
        }
        for i in 0..*active_nodes {
            let (done, rest) = nodes.split_at_mut(i);
            let node = &mut rest[0];
            if let Source::Element { index, overlay } = node.source {
                if overlay != NO_OVERLAY {
                    if let FilterRt::DimmingCurve(curve) = &mut node.filter {
                        let state = states.get(index as usize).unwrap_or(empty_state);
                        curve.update_element_in_place(state, &mut overlays[overlay as usize].values);
                        continue;
                    }
                }
            }
            let flow = read_source(node.source, done, states, empty_state, overlays);
            // `_UpdateFromSource`: a null data object doesn't reach the filter.
            if !matches!(flow, Flow::Null) {
                node.filter.update(flow);
            }
        }
        for controller in controllers.iter_mut() {
            let CompiledController { policy, bindings, commands, .. } = controller;
            for (binding, command) in bindings.iter().zip(commands.iter_mut()) {
                let flow = read_source(*binding, nodes, states, empty_state, overlays);
                policy.generate(flow, command);
            }
        }
    }

    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    pub fn element_ids(&self) -> &[Uuid] {
        &self.elements
    }

    /// The dense index of an element id.
    pub fn element_index(&self, id: &Uuid) -> Option<usize> {
        self.element_lookup.get(id).map(|i| *i as usize)
    }

    pub fn controllers(&self) -> &[CompiledController] {
        &self.controllers
    }

    pub fn controller_index(&self, id: &Uuid) -> Option<usize> {
        self.controllers.iter().position(|c| c.id == *id)
    }

    /// This frame's command for each output of a controller. Empty for an index out of range.
    #[inline]
    pub fn commands(&self, controller: usize) -> &[OutputCommand] {
        self.controllers.get(controller).map_or(&[], CompiledController::commands)
    }

    /// One controller's outputs as bytes, the way the byte-oriented modules and `ExportCommandHandler` read them: an 8-bit
    /// command's value, and 0 for anything else. Writes as many as both sides have.
    pub fn copy_bytes(&self, controller: usize, out: &mut [u8]) {
        for (b, c) in out.iter_mut().zip(self.commands(controller)) {
            *b = c.byte();
        }
    }

    /// A row of channel bytes across several controllers, in the order given: one export interval, as an FSEQ frame holds it.
    /// Returns how many bytes were written.
    pub fn fill_row(&self, controllers: &[usize], row: &mut [u8]) -> usize {
        let mut position = 0;
        for &c in controllers {
            for command in self.commands(c) {
                let Some(slot) = row.get_mut(position) else {
                    return position;
                };
                *slot = command.byte();
                position += 1;
            }
        }
        position
    }

    /// The total number of outputs across all controllers.
    pub fn output_count(&self) -> usize {
        self.controllers.iter().map(CompiledController::output_count).sum()
    }

    /// The typed data of a filter.
    pub fn filter_config(&self, instance_id: &Uuid) -> Option<&FilterConfig> {
        self.filter_configs.get(instance_id)
    }

    /// A filter's setup changed: its setters recreate the outputs in the C#, and this does the same. Returns `false`, leaving
    /// everything as it was, if the filter isn't in the graph, is another type, or would end up with a different number of
    /// outputs. A change like that can move patches, so build the graph again from the new data.
    pub fn set_filter_config(&mut self, instance_id: &Uuid, mut config: FilterConfig) -> bool {
        let Some(existing) = self.filter_configs.get_mut(instance_id) else {
            return false;
        };
        let ids = *existing.ids();
        if config.ids().type_id != ids.type_id {
            return false;
        }
        *config.ids_mut() = ids;
        let rt = FilterRt::new(&config);
        let Some(node) = self.nodes.iter_mut().find(|n| n.instance_id == *instance_id) else {
            return false;
        };
        if rt.output_count() != node.filter.output_count() {
            return false;
        }
        node.filter = rt;
        *existing = config;
        true
    }

    /// A controller's setup changed. Its data policy follows, as it does through `DataPolicyFactoryChanged`, for the render
    /// styles of the dummy controller. Returns `false` if the index is out of range.
    pub fn set_controller_config(&mut self, controller: usize, config: ControllerConfig) -> bool {
        let Some(c) = self.controllers.get_mut(controller) else {
            return false;
        };
        c.policy = PolicyState::new(config.data_policy());
        c.config = Some(config);
        true
    }

    /// How many filters run each frame. The rest have no element above them.
    pub fn active_filter_count(&self) -> usize {
        self.active_nodes
    }

    pub fn filter_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn warnings(&self) -> &[GraphWarning] {
        &self.warnings
    }

    /// `Export.BuildChannelNames` for one output: the name of the element it comes from, with each Color Breakdown on the way
    /// adding the name of its colour, or `{controller}_{n}` when nothing is patched to it.
    pub fn channel_name(&self, address: OutputAddress, element_name: &dyn Fn(&Uuid) -> String) -> Option<String> {
        let controller = self.controllers.get(address.controller)?;
        let source = *controller.bindings.get(address.output)?;
        if source == Source::None {
            return Some(format!("{}_{}", controller.name, address.output + 1));
        }
        Some(self.reverse_channel_name(source, element_name))
    }

    /// `ReverseBuildChannelName`.
    fn reverse_channel_name(&self, source: Source, element_name: &dyn Fn(&Uuid) -> String) -> String {
        match source {
            Source::None => String::new(),
            Source::Element { index, .. } => self.elements.get(index as usize).map(element_name).unwrap_or_default(),
            Source::Filter { node, output } => {
                let Some(node) = self.nodes.get(node as usize) else { return String::new() };
                let config = self.filter_configs.get(&node.instance_id);
                let own_name = || {
                    config.and_then(|c| c.output_names().get(output as usize).cloned()).unwrap_or_default()
                };
                if node.source == Source::None {
                    return own_name();
                }
                let above = self.reverse_channel_name(node.source, element_name);
                if matches!(config, Some(FilterConfig::ColorBreakdown(_))) {
                    format!("{above} {}", own_name())
                } else {
                    above
                }
            }
        }
    }

    /// The element whose data reaches an output, following the patches back through the filters.
    pub fn element_for_output(&self, address: OutputAddress) -> Option<usize> {
        let mut source = *self.controllers.get(address.controller)?.bindings.get(address.output)?;
        loop {
            match source {
                Source::None => return None,
                Source::Element { index, .. } => return Some(index as usize),
                Source::Filter { node, .. } => source = self.nodes.get(node as usize)?.source,
            }
        }
    }

    /// Every output an element's data reaches.
    pub fn outputs_for_element(&self, element: usize) -> Vec<OutputAddress> {
        let mut result = Vec::new();
        for (c, controller) in self.controllers.iter().enumerate() {
            for o in 0..controller.bindings.len() {
                let address = OutputAddress { controller: c, output: o };
                if self.element_for_output(address) == Some(element) {
                    result.push(address);
                }
            }
        }
        result
    }

    /// The filter instances between an output and its element, nearest the output first.
    pub fn filters_for_output(&self, address: OutputAddress) -> Vec<Uuid> {
        let mut result = Vec::new();
        let Some(mut source) = self.controllers.get(address.controller).and_then(|c| c.bindings.get(address.output)).copied()
        else {
            return result;
        };
        while let Source::Filter { node, .. } = source {
            let Some(node) = self.nodes.get(node as usize) else {
                break;
            };
            result.push(node.instance_id);
            source = node.source;
        }
        result
    }
}

/// `IDataFlowComponentReference.GetOutputState`.
#[inline]
fn read_source<'a>(
    source: Source,
    nodes: &'a [Node],
    states: &'a [CombinedState],
    empty: &'a CombinedState,
    overlays: &'a [Overlay],
) -> Flow<'a> {
    match source {
        // `Output.State` is null, so `OutputController.Update` leaves a null command.
        Source::None => Flow::Null,
        Source::Element { index, overlay } => {
            let state = states.get(index as usize).unwrap_or(empty);
            if overlay == NO_OVERLAY {
                Flow::Intents(States::Element(state))
            } else {
                Flow::Intents(States::Overlay(state, &overlays[overlay as usize].values))
            }
        }
        Source::Filter { node, output } => match nodes.get(node as usize) {
            Some(n) => n.filter.output(output as usize),
            None => Flow::Null,
        },
    }
}

/// `_CheckComponentSourceForCircularDependency`.
fn creates_loop(filter: u32, source: SourceRef, filter_source: &[Option<SourceRef>]) -> bool {
    let mut current = source;
    loop {
        match current {
            SourceRef::Element(_) => return false,
            SourceRef::Filter { filter: f, .. } => {
                if f == filter {
                    return true;
                }
                match filter_source[f as usize] {
                    Some(next) => current = next,
                    None => return false,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dotnet::DotNetColor;
    use crate::filters::{
        CoarseFineConfig, ColorBreakdownConfig, ColorBreakdownItem, CurveData, DimmingCurveConfig, ShutterFilterConfig,
        TaggedFilterConfig,
    };
    use crate::ids;
    use crate::xmlutil::ModuleIds;
    use uuid::uuid;
    use vixen_core::{Color, CommandValue, DiscreteValue, FunctionIdentity, LightingValue, RangeValue, RgbValue, StateValue};

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    const CONTROLLER_INSTANCE: Uuid = uuid!("c0000000-0000-0000-0000-000000000001");

    /// A one-controller graph builder with `outputs` outputs, numbered from `OUTPUT_BASE`.
    struct Builder {
        def: GraphDefinition,
        data: HashMap<(Uuid, Uuid), XElement>,
    }

    const OUTPUT_BASE: u128 = 1000;

    impl Builder {
        fn new(elements: usize, outputs: usize, hardware: Uuid) -> Builder {
            let def = GraphDefinition {
                elements: (0..elements as u128).map(id).collect(),
                controllers: vec![ControllerDef {
                    id: id(9000),
                    name: "Test".to_string(),
                    hardware_type_id: hardware,
                    hardware_instance_id: CONTROLLER_INSTANCE,
                    outputs: (0..outputs as u128)
                        .map(|o| OutputDef { id: id(OUTPUT_BASE + o), name: format!("Output {}", o + 1) })
                        .collect(),
                }],
                ..Default::default()
            };
            Builder { def, data: HashMap::new() }
        }

        fn filter(&mut self, config: FilterConfig, instance: u128) -> u128 {
            let mut config = config;
            let ids = ModuleIds::new(config.ids().type_id, id(instance));
            *config.ids_mut() = ids;
            self.def.filters.push(FilterDef { type_id: ids.type_id, instance_id: ids.instance_id });
            self.data.insert((ids.type_id, ids.instance_id), config.to_xml());
            instance
        }

        fn patch(&mut self, component: u128, source: u128, output: i32) {
            self.def.patches.push(PatchDef { component_id: id(component), source_id: Some(id(source)), output_index: output });
        }

        fn output(&mut self, output: u128, source: u128, source_output: i32) {
            self.patch(OUTPUT_BASE + output, source, source_output);
        }

        fn build(&self) -> OutputGraph {
            OutputGraph::build(&self.def, &self.data)
        }
    }

    fn rgb_filter(mix: bool) -> FilterConfig {
        FilterConfig::ColorBreakdown(ColorBreakdownConfig {
            breakdown_items: ColorBreakdownConfig::template("RGB").unwrap(),
            mix_colors: mix,
            ..Default::default()
        })
    }

    fn state_rgb(r: u8, g: u8, b: u8) -> CombinedState {
        CombinedState { rgb: Some(RgbValue { r, g, b }), ..Default::default() }
    }

    fn state_discrete(c: Color, i: f64) -> CombinedState {
        let mut s = CombinedState::default();
        s.discrete.push(DiscreteValue::new(c, i));
        s
    }

    fn state_other(v: StateValue) -> CombinedState {
        let mut s = CombinedState::default();
        s.other.push(v);
        s
    }

    fn bytes(g: &OutputGraph) -> Vec<u8> {
        g.commands(0).iter().map(OutputCommand::byte).collect()
    }

    #[test]
    fn an_unpatched_output_is_null_and_a_patched_empty_element_is_null_too() {
        let mut b = Builder::new(1, 2, ids::GENERIC_SERIAL);
        b.output(0, 0, 0);
        let mut g = b.build();
        g.evaluate(&[CombinedState::default()]);
        assert_eq!(g.commands(0), &[OutputCommand::None, OutputCommand::None]);
        assert!(g.warnings().is_empty());
    }

    #[test]
    fn an_element_patched_straight_to_an_output_uses_the_controllers_policy() {
        let mut b = Builder::new(1, 1, ids::GENERIC_SERIAL);
        b.output(0, 0, 0);
        let mut g = b.build();
        g.evaluate(&[state_discrete(Color::WHITE, 0.5)]);
        assert_eq!(g.commands(0), &[OutputCommand::U8(127)]);
        g.evaluate(&[state_rgb(10, 200, 30)]);
        assert_eq!(g.commands(0), &[OutputCommand::U8(200)]);
        let lit = CombinedState { lighting: Some(LightingValue::new(Color::RED, 1.0)), ..Default::default() };
        g.evaluate(&[lit]);
        assert_eq!(g.commands(0), &[OutputCommand::U8(255)]);
        g.evaluate(&[CombinedState::default()]);
        assert_eq!(g.commands(0), &[OutputCommand::None]);
    }

    #[test]
    fn a_missing_state_counts_as_empty() {
        let mut b = Builder::new(2, 2, ids::GENERIC_SERIAL);
        b.output(0, 0, 0);
        b.output(1, 1, 0);
        let mut g = b.build();
        g.evaluate(&[state_rgb(1, 2, 3)]);
        assert_eq!(g.commands(0), &[OutputCommand::U8(3), OutputCommand::None]);
        g.evaluate(&[]);
        assert_eq!(g.commands(0), &[OutputCommand::None, OutputCommand::None]);
    }

    #[test]
    fn color_breakdown_feeds_each_output_its_channel() {
        let mut b = Builder::new(1, 3, ids::GENERIC_SERIAL);
        let f = b.filter(rgb_filter(true), 100);
        b.patch(f, 0, 0);
        for o in 0..3 {
            b.output(o, f, o as i32);
        }
        let mut g = b.build();
        assert_eq!(g.active_filter_count(), 1);
        g.evaluate(&[state_rgb(200, 100, 50)]);
        assert_eq!(bytes(&g), vec![200, 100, 50]);
        // With no state the filter still reports 0 as a command, not null.
        g.evaluate(&[CombinedState::default()]);
        assert_eq!(g.commands(0), &[OutputCommand::U8(0), OutputCommand::U8(0), OutputCommand::U8(0)]);
    }

    #[test]
    fn a_16_bit_color_breakdown_reaches_byte_controllers_as_zero() {
        let mut b = Builder::new(1, 1, ids::E131);
        let f = b.filter(
            FilterConfig::ColorBreakdown(ColorBreakdownConfig {
                breakdown_items: vec![ColorBreakdownItem::new(DotNetColor::RED, "Red")],
                mix_colors: true,
                sixteen_bit: true,
                ..Default::default()
            }),
            100,
        );
        b.patch(f, 0, 0);
        b.output(0, f, 0);
        let mut g = b.build();
        g.evaluate(&[state_rgb(255, 0, 0)]);
        assert_eq!(g.commands(0), &[OutputCommand::U16(65535)]);
        assert_eq!(bytes(&g), vec![0]);
    }

    #[test]
    fn filters_chain_through_a_dimming_curve_on_intents() {
        let mut b = Builder::new(1, 3, ids::GENERIC_SERIAL);
        let curve = b.filter(
            FilterConfig::DimmingCurve(DimmingCurveConfig {
                curve: CurveData::from_points(&[(0.0, 0.0), (100.0, 50.0)]),
                ..Default::default()
            }),
            100,
        );
        let f = b.filter(rgb_filter(true), 101);
        b.patch(curve, 0, 0);
        b.patch(f, curve, 0);
        for o in 0..3 {
            b.output(o, f, o as i32);
        }
        let mut g = b.build();
        g.evaluate(&[state_rgb(255, 0, 0)]);
        assert_eq!(bytes(&g), vec![127, 0, 0]);
    }

    #[test]
    fn patches_can_list_a_child_before_its_parent() {
        let mut b = Builder::new(1, 1, ids::GENERIC_SERIAL);
        let f = b.filter(rgb_filter(true), 100);
        let curve = b.filter(
            FilterConfig::DimmingCurve(DimmingCurveConfig {
                curve: CurveData::from_points(&[(0.0, 0.0), (100.0, 50.0)]),
                ..Default::default()
            }),
            101,
        );
        b.output(0, f, 0);
        b.patch(f, curve, 0);
        b.patch(curve, 0, 0);
        let mut g = b.build();
        g.evaluate(&[state_rgb(255, 0, 0)]);
        assert_eq!(bytes(&g), vec![127]);
    }

    #[test]
    fn a_dimming_curve_on_commands_after_a_breakdown() {
        let mut b = Builder::new(1, 1, ids::GENERIC_SERIAL);
        let f = b.filter(rgb_filter(true), 100);
        let curve = b.filter(
            FilterConfig::DimmingCurve(DimmingCurveConfig {
                curve: CurveData::from_points(&[(0.0, 0.0), (100.0, 50.0)]),
                ..Default::default()
            }),
            101,
        );
        b.patch(f, 0, 0);
        b.patch(curve, f, 0);
        b.output(0, curve, 0);
        let mut g = b.build();
        g.evaluate(&[state_rgb(255, 0, 0)]);
        assert_eq!(g.commands(0), &[OutputCommand::U8(127)]);
    }

    #[test]
    fn a_curve_with_no_states_gives_a_null_command_downstream() {
        let mut b = Builder::new(1, 1, ids::GENERIC_SERIAL);
        let curve = b.filter(FilterConfig::DimmingCurve(DimmingCurveConfig::default()), 100);
        b.patch(curve, 0, 0);
        b.output(0, curve, 0);
        let mut g = b.build();
        g.evaluate(&[CombinedState::default()]);
        assert_eq!(g.commands(0), &[OutputCommand::None]);
    }

    #[test]
    fn a_tagged_filter_after_a_curve_with_no_states_stays_empty() {
        let mut b = Builder::new(1, 1, ids::GENERIC_SERIAL);
        let curve = b.filter(FilterConfig::DimmingCurve(DimmingCurveConfig::default()), 100);
        let tagged =
            b.filter(FilterConfig::Tagged(TaggedFilterConfig { tag: Some("Dim".to_string()), ..Default::default() }), 101);
        b.patch(curve, 0, 0);
        b.patch(tagged, curve, 0);
        b.output(0, tagged, 0);
        let mut g = b.build();
        g.evaluate(&[CombinedState::default()]);
        assert_eq!(g.commands(0), &[OutputCommand::None]);
    }

    #[test]
    fn a_fixture_chain_splits_a_range_into_coarse_and_fine() {
        // Element -> tag filter -> coarse/fine -> two outputs, as the intelligent fixture template builds it.
        let mut b = Builder::new(1, 2, ids::E131);
        let tagged =
            b.filter(FilterConfig::Tagged(TaggedFilterConfig { tag: Some("Pan".to_string()), ..Default::default() }), 100);
        let split = b.filter(FilterConfig::CoarseFine(CoarseFineConfig::default()), 101);
        b.patch(tagged, 0, 0);
        b.patch(split, tagged, 0);
        b.output(0, split, 0);
        b.output(1, split, 1);
        let mut g = b.build();
        g.evaluate(&[state_other(StateValue::Range(RangeValue::new(FunctionIdentity::Pan, "Pan", 0.5, "")))]);
        assert_eq!(bytes(&g), vec![0x7F, 0xFF]);
        // A range with another tag doesn't get through, and without default mapping the outputs are null.
        g.evaluate(&[state_other(StateValue::Range(RangeValue::new(FunctionIdentity::Tilt, "Tilt", 0.5, "")))]);
        assert_eq!(g.commands(0), &[OutputCommand::None, OutputCommand::None]);
    }

    #[test]
    fn a_breakdown_into_coarse_fine_splits_16_bit_commands() {
        let mut b = Builder::new(1, 2, ids::E131);
        let f = b.filter(
            FilterConfig::ColorBreakdown(ColorBreakdownConfig {
                breakdown_items: vec![ColorBreakdownItem::new(DotNetColor::RED, "Red")],
                mix_colors: true,
                sixteen_bit: true,
                ..Default::default()
            }),
            100,
        );
        let split = b.filter(FilterConfig::CoarseFine(CoarseFineConfig::default()), 101);
        b.patch(f, 0, 0);
        b.patch(split, f, 0);
        b.output(0, split, 0);
        b.output(1, split, 1);
        let mut g = b.build();
        g.evaluate(&[CombinedState { lighting: Some(LightingValue::new(Color::RED, 0.5)), ..Default::default() }]);
        assert_eq!(bytes(&g), vec![0x7F, 0xFF]);
    }

    #[test]
    fn the_shutter_close_command_reaches_the_output_even_with_no_element_above_it() {
        // No patch into the filter, so `FilterManager` never updates it. Reading its output still runs the getter.
        let mut b = Builder::new(1, 1, ids::E131);
        let shutter = b.filter(
            FilterConfig::Shutter(ShutterFilterConfig {
                tag: Some("Shutter".to_string()),
                convert_color_into_shutter_intents: true,
                open_shutter_index_value: 255,
                close_shutter_index_value: 9,
                ..Default::default()
            }),
            100,
        );
        b.output(0, shutter, 0);
        let mut g = b.build();
        assert_eq!(g.active_filter_count(), 0);
        assert_eq!(g.filter_count(), 1);
        g.evaluate(&[state_rgb(255, 255, 255)]);
        assert_eq!(g.commands(0), &[OutputCommand::U8(9)]);
        assert_eq!(g.element_for_output(OutputAddress { controller: 0, output: 0 }), None);
    }

    #[test]
    fn highest_wins_means_a_close_value_above_open_stays_closed() {
        let mut b = Builder::new(1, 1, ids::E131);
        let shutter = b.filter(
            FilterConfig::Shutter(ShutterFilterConfig {
                tag: Some("Shutter".to_string()),
                convert_color_into_shutter_intents: true,
                open_shutter_index_value: 20,
                close_shutter_index_value: 200,
                ..Default::default()
            }),
            100,
        );
        b.patch(shutter, 0, 0);
        b.output(0, shutter, 0);
        let mut g = b.build();
        g.evaluate(&[state_rgb(255, 255, 255)]);
        assert_eq!(g.commands(0), &[OutputCommand::U8(200)]);
    }

    #[test]
    fn an_orphan_breakdown_reads_as_zero_commands() {
        let mut b = Builder::new(1, 1, ids::E131);
        let f = b.filter(rgb_filter(true), 100);
        b.output(0, f, 0);
        let mut g = b.build();
        g.evaluate(&[state_rgb(255, 255, 255)]);
        assert_eq!(g.commands(0), &[OutputCommand::U8(0)]);
    }

    #[test]
    fn a_later_patch_replaces_an_earlier_one() {
        let mut b = Builder::new(2, 1, ids::GENERIC_SERIAL);
        b.output(0, 0, 0);
        b.output(0, 1, 0);
        let mut g = b.build();
        g.evaluate(&[state_rgb(1, 1, 1), state_rgb(2, 2, 2)]);
        assert_eq!(g.commands(0), &[OutputCommand::U8(2)]);
    }

    #[test]
    fn a_circular_patch_is_refused() {
        let mut b = Builder::new(1, 1, ids::GENERIC_SERIAL);
        let a = b.filter(FilterConfig::DimmingCurve(DimmingCurveConfig::default()), 100);
        let c = b.filter(FilterConfig::DimmingCurve(DimmingCurveConfig::default()), 101);
        b.patch(a, c, 0);
        b.patch(c, a, 0);
        let g = b.build();
        assert_eq!(g.warnings(), &[GraphWarning::CircularPatch { component_id: id(101), source_id: id(100) }]);
        assert_eq!(g.active_filter_count(), 0);
    }

    #[test]
    fn a_filter_patched_to_itself_is_refused() {
        let mut b = Builder::new(1, 1, ids::GENERIC_SERIAL);
        let a = b.filter(FilterConfig::DimmingCurve(DimmingCurveConfig::default()), 100);
        b.patch(a, a, 0);
        assert!(matches!(b.build().warnings()[0], GraphWarning::CircularPatch { .. }));
    }

    #[test]
    fn bad_patches_are_recorded_and_skipped() {
        let mut b = Builder::new(1, 2, ids::GENERIC_SERIAL);
        let f = b.filter(rgb_filter(true), 100);
        b.patch(f, 0, 5); // No such element output.
        b.output(0, f, 7); // No such filter output.
        b.output(1, 12345, 0); // No such source.
        b.patch(12346, 0, 0); // No such component.
        b.patch(OUTPUT_BASE, OUTPUT_BASE + 1, 0); // An output can't be a source.
        b.def.patches.push(PatchDef { component_id: id(OUTPUT_BASE), source_id: None, output_index: 0 });
        let mut g = b.build();
        assert_eq!(g.warnings().len(), 6);
        g.evaluate(&[state_rgb(255, 255, 255)]);
        assert_eq!(g.commands(0), &[OutputCommand::None, OutputCommand::None]);
    }

    #[test]
    fn an_unknown_filter_type_is_dropped_and_its_patches_with_it() {
        let mut b = Builder::new(1, 1, ids::GENERIC_SERIAL);
        b.def.filters.push(FilterDef { type_id: id(777), instance_id: id(100) });
        b.patch(100, 0, 0);
        b.output(0, 100, 0);
        let mut g = b.build();
        assert_eq!(g.warnings()[0], GraphWarning::UnknownFilterType { type_id: id(777), instance_id: id(100) });
        g.evaluate(&[state_rgb(255, 255, 255)]);
        assert_eq!(g.commands(0), &[OutputCommand::None]);
    }

    #[test]
    fn an_unknown_controller_type_still_evaluates() {
        let mut b = Builder::new(1, 1, id(778));
        b.output(0, 0, 0);
        let mut g = b.build();
        assert_eq!(g.warnings().len(), 1);
        g.evaluate(&[state_rgb(9, 9, 9)]);
        assert_eq!(g.commands(0), &[OutputCommand::U8(9)]);
        assert!(g.controllers()[0].config.is_none());
    }

    #[test]
    fn dummy_lighting_policy_comes_from_its_data() {
        let mut b = Builder::new(1, 1, ids::DUMMY_LIGHTING);
        b.output(0, 0, 0);
        let config = crate::modules::DummyLightingConfig {
            ids: ModuleIds::new(ids::DUMMY_LIGHTING, CONTROLLER_INSTANCE),
            render_style: crate::modules::RenderStyle::RGBSingleChannel,
            ..Default::default()
        };
        b.data.insert((ids::DUMMY_LIGHTING, CONTROLLER_INSTANCE), config.to_xml());
        let mut g = b.build();
        g.evaluate(&[state_rgb(10, 20, 30)]);
        assert_eq!(g.commands(0), &[OutputCommand::Color(Color::rgb(10, 20, 30))]);
    }

    #[test]
    fn rds_policy_keeps_the_last_command_so_strings_get_through() {
        let mut b = Builder::new(1, 1, ids::RDS);
        b.output(0, 0, 0);
        let mut g = b.build();
        let text: std::sync::Arc<str> = std::sync::Arc::from("RDS|Hello");
        g.evaluate(&[state_other(StateValue::Command(CommandValue::Str(text.clone())))]);
        assert_eq!(g.commands(0), &[OutputCommand::Str(text)]);
    }

    #[test]
    fn a_curve_changes_a_discrete_value_for_a_direct_output_on_the_same_element() {
        // The C# writes the new intensity into the state object both consumers share.
        let mut b = Builder::new(1, 2, ids::GENERIC_SERIAL);
        let curve = b.filter(
            FilterConfig::DimmingCurve(DimmingCurveConfig {
                curve: CurveData::from_points(&[(0.0, 0.0), (100.0, 50.0)]),
                ..Default::default()
            }),
            100,
        );
        b.patch(curve, 0, 0);
        b.output(0, curve, 0);
        b.output(1, 0, 0);
        let mut g = b.build();
        g.evaluate(&[state_discrete(Color::WHITE, 1.0)]);
        assert_eq!(g.commands(0), &[OutputCommand::U8(127), OutputCommand::U8(127)]);
        // The element's own state isn't touched, so the next frame starts from the real value again.
        g.evaluate(&[state_discrete(Color::WHITE, 1.0)]);
        assert_eq!(g.commands(0), &[OutputCommand::U8(127), OutputCommand::U8(127)]);
    }

    #[test]
    fn a_curve_doesnt_touch_rgb_for_a_direct_output() {
        let mut b = Builder::new(1, 2, ids::GENERIC_SERIAL);
        let curve = b.filter(
            FilterConfig::DimmingCurve(DimmingCurveConfig {
                curve: CurveData::from_points(&[(0.0, 0.0), (100.0, 50.0)]),
                ..Default::default()
            }),
            100,
        );
        b.patch(curve, 0, 0);
        b.output(0, curve, 0);
        b.output(1, 0, 0);
        let mut g = b.build();
        g.evaluate(&[state_rgb(255, 0, 0)]);
        assert_eq!(g.commands(0), &[OutputCommand::U8(127), OutputCommand::U8(255)]);
    }

    #[test]
    fn reach_queries_follow_the_patches() {
        let mut b = Builder::new(2, 4, ids::GENERIC_SERIAL);
        let f = b.filter(rgb_filter(true), 100);
        b.patch(f, 1, 0);
        b.output(0, f, 0);
        b.output(1, f, 1);
        b.output(2, f, 2);
        b.output(3, 0, 0);
        let g = b.build();
        let at = |output| OutputAddress { controller: 0, output };
        assert_eq!(g.outputs_for_element(1), vec![at(0), at(1), at(2)]);
        assert_eq!(g.outputs_for_element(0), vec![at(3)]);
        assert_eq!(g.element_for_output(at(1)), Some(1));
        assert_eq!(g.filters_for_output(at(1)), vec![id(100)]);
        assert!(g.filters_for_output(at(3)).is_empty());
        assert_eq!(g.element_index(&id(1)), Some(1));
        assert_eq!(g.element_index(&id(99)), None);
    }

    #[test]
    fn fill_row_concatenates_controllers_in_the_order_given() {
        let def = GraphDefinition {
            elements: vec![id(0), id(1)],
            controllers: vec![
                ControllerDef {
                    id: id(9000),
                    name: "A".to_string(),
                    hardware_type_id: ids::E131,
                    hardware_instance_id: id(9001),
                    outputs: vec![OutputDef { id: id(1000), name: String::new() }],
                },
                ControllerDef {
                    id: id(9002),
                    name: "B".to_string(),
                    hardware_type_id: ids::E131,
                    hardware_instance_id: id(9003),
                    outputs: vec![
                        OutputDef { id: id(1001), name: String::new() },
                        OutputDef { id: id(1002), name: String::new() },
                    ],
                },
            ],
            filters: vec![],
            patches: vec![
                PatchDef { component_id: id(1000), source_id: Some(id(0)), output_index: 0 },
                PatchDef { component_id: id(1002), source_id: Some(id(1)), output_index: 0 },
            ],
        };
        let mut g = OutputGraph::build(&def, &NoModuleData);
        g.evaluate(&[state_rgb(11, 0, 0), state_rgb(22, 0, 0)]);
        let mut row = [9u8; 3];
        assert_eq!(g.fill_row(&[1, 0], &mut row), 3);
        assert_eq!(row, [0, 22, 11]);
        let mut small = [9u8; 2];
        assert_eq!(g.fill_row(&[0, 1], &mut small), 2);
        assert_eq!(small, [11, 0]);
        assert_eq!(g.output_count(), 3);
    }

    #[test]
    fn a_filters_setup_can_change_while_the_graph_is_in_use() {
        let mut b = Builder::new(1, 1, ids::GENERIC_SERIAL);
        let f = b.filter(rgb_filter(true), 100);
        b.patch(f, 0, 0);
        b.output(0, f, 0);
        let mut g = b.build();
        g.evaluate(&[state_rgb(200, 0, 0)]);
        assert_eq!(g.commands(0), &[OutputCommand::U8(200)]);

        // Mixing off: a dimmed red no longer matches the red channel.
        assert!(g.set_filter_config(&id(100), rgb_filter(false)));
        g.evaluate(&[state_rgb(200, 0, 0)]);
        assert_eq!(g.commands(0), &[OutputCommand::U8(0)]);
        let FilterConfig::ColorBreakdown(c) = g.filter_config(&id(100)).unwrap() else { panic!() };
        assert!(!c.mix_colors);
        assert_eq!(c.ids.instance_id, id(100));

        // A different shape needs a rebuild, and another type isn't this filter.
        let two = FilterConfig::ColorBreakdown(ColorBreakdownConfig {
            breakdown_items: vec![ColorBreakdownItem::new(DotNetColor::RED, "Red")],
            ..Default::default()
        });
        assert!(!g.set_filter_config(&id(100), two));
        assert!(!g.set_filter_config(&id(100), FilterConfig::Tagged(TaggedFilterConfig::default())));
        assert!(!g.set_filter_config(&id(999), rgb_filter(true)));
    }

    #[test]
    fn a_controllers_setup_can_change_its_data_policy() {
        let mut b = Builder::new(1, 1, ids::DUMMY_LIGHTING);
        b.output(0, 0, 0);
        let mut g = b.build();
        g.evaluate(&[state_rgb(10, 20, 30)]);
        assert_eq!(g.commands(0), &[OutputCommand::U8(30)]);
        let config = crate::modules::DummyLightingConfig {
            render_style: crate::modules::RenderStyle::RGBSingleChannel,
            ..Default::default()
        };
        assert!(g.set_controller_config(0, ControllerConfig::DummyLighting(config)));
        g.evaluate(&[state_rgb(10, 20, 30)]);
        assert_eq!(g.commands(0), &[OutputCommand::Color(Color::rgb(10, 20, 30))]);
        assert_eq!(g.controllers()[0].data_policy(), DataPolicy::ONE_CHANNEL_COLOR);
        assert!(!g.set_controller_config(5, ControllerConfig::new_default(&ids::DEBUG_CONTROLLER, Uuid::nil()).unwrap()));
    }

    #[test]
    fn the_graph_can_move_to_the_engine_thread() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<OutputGraph>();
    }

    #[test]
    fn no_module_data_gives_the_default_filter() {
        let mut def = GraphDefinition { elements: vec![id(0)], ..Default::default() };
        def.filters.push(FilterDef { type_id: ids::COLOR_BREAKDOWN, instance_id: id(100) });
        let g = OutputGraph::build(&def, &NoModuleData);
        let FilterConfig::ColorBreakdown(c) = g.filter_config(&id(100)).unwrap() else { panic!() };
        assert!(c.breakdown_items.is_empty());
        assert_eq!(c.ids.instance_id, id(100));
    }
}
