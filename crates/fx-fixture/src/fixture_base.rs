//! Port of `FixtureEffectBase<T>` and `FixtureIndexEffectBase<T>` (Effect/FixtureEffectBase.cs and
//! Effect/FixtureIndexEffectBase.cs): finding the fixtures an effect targets and turning curves and index
//! values into range and command intents.
//!
//! In C# an effect instance is stateful: `TargetNodesChanged` runs `UpdateFixtureCapabilities`, which fixes
//! up the stored function names and caches which nodes support what. Rendering here is a pure function of
//! the parameters and the targets, so each effect runs that fix-up on local copies at the start of `render`.

use crate::spec::{
    parse_specification, FixtureFunction, FixtureIndexItem, FixtureSpecification, FuncId, FixtureFunctionType,
    INTELLIGENT_FIXTURE_TYPE_ID,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use vixen_core::element::PropertyData;
use vixen_core::value::RangeValue;
use vixen_core::{Curve, EffectIntents, Intent, IntentData, IntentNode, NodeId, TimeSpan};
use vixen_effects::framework::{scale_curve_to_value, ParamValue, Params, RenderCtx, SelectionValue, TargetInfo};

/// The fixtures behind a render's target nodes. Specifications are parsed from the property XML on first
/// use and then shared.
pub struct FixtureTargets<'a> {
    pub info: TargetInfo<'a>,
    cache: RefCell<HashMap<NodeId, Option<Rc<FixtureSpecification>>>>,
}

/// A node that supports a function, with the function's label (`Tuple<IElementNode, string>`).
pub type LabeledNode = (NodeId, String);

impl<'a> FixtureTargets<'a> {
    pub fn new(ctx: &'a RenderCtx) -> Self {
        Self::from_info(ctx.target_info())
    }

    /// For the editor rules, which have the effect's place but no render.
    pub fn from_info(info: TargetInfo<'a>) -> Self {
        FixtureTargets { info, cache: RefCell::new(HashMap::new()) }
    }

    /// `TargetNodes.FirstOrDefault()`.
    pub fn first_target(&self) -> Option<NodeId> {
        self.info.targets.first().copied()
    }

    /// `GetIntelligentFixtureProperty(node).FixtureSpecification`, or `None` when the node has no
    /// intelligent fixture property (or its data cannot be read).
    pub fn spec(&self, node: NodeId) -> Option<Rc<FixtureSpecification>> {
        if let Some(hit) = self.cache.borrow().get(&node) {
            return hit.clone();
        }
        let parsed = match self.info.tree.property(node, &INTELLIGENT_FIXTURE_TYPE_ID) {
            Some(p) => match &p.data {
                PropertyData::Unknown { raw_xml } => parse_specification(raw_xml).map(Rc::new),
                _ => None,
            },
            None => None,
        };
        self.cache.borrow_mut().insert(node, parsed.clone());
        parsed
    }

    /// `x.Properties.Contains(IntelligentFixtureDescriptor._typeId)`.
    pub fn has_fixture_property(&self, node: NodeId) -> bool {
        self.info.tree.property(node, &INTELLIGENT_FIXTURE_TYPE_ID).is_some()
    }

    /// `GetLeafNodesWithIntelligentFixtureProperty`. A leaf under two parents appears twice, as in C#.
    pub fn leaves_with_fixture(&self, node: NodeId) -> Vec<NodeId> {
        self.info.tree.leaves(node).into_iter().filter(|l| self.has_fixture_property(*l)).collect()
    }

    /// `GetFixtureFunctions()`: the functions on the channels of the first target's fixtures.
    pub fn fixture_functions(&self) -> Vec<FixtureFunction> {
        let mut out: Vec<FixtureFunction> = Vec::new();
        let Some(node) = self.first_target() else { return out };
        // `fixtureFunctions.Contains(func)` is reference equality: the same fixture (leaf) never adds a
        // function twice, but two fixtures with identical profiles each add their own.
        let mut seen: Vec<(usize, String)> = Vec::new();
        for leaf in self.info.tree.leaves(node) {
            let Some(spec) = self.spec(leaf) else { continue };
            let key = Rc::as_ptr(&spec) as usize;
            for channel in &spec.channels {
                // Guarding against a corrupted profile
                if channel.function.is_empty() {
                    continue;
                }
                // The C# uses `Single`, which throws for a channel naming no function.
                let Some(func) = spec.functions.iter().find(|f| f.name == channel.function) else { continue };
                if func.function_type != FixtureFunctionType::None && !seen.contains(&(key, func.name.clone())) {
                    seen.push((key, func.name.clone()));
                    out.push(func.clone());
                }
            }
        }
        out
    }

    /// `GetMatchingFunctionNames(identity)`.
    pub fn matching_function_names(&self, identity: FuncId) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for f in self.fixture_functions() {
            if f.identity == identity && !names.contains(&f.name) {
                names.push(f.name);
            }
        }
        names
    }

    /// `GetRenderNodesForFunctionIdentity(type, tags)`: first target only. Fills `tags` with the function
    /// name per node and returns the nodes with their function labels.
    pub fn render_nodes_for_identity(&self, identity: FuncId, tags: &mut HashMap<NodeId, String>) -> Vec<LabeledNode> {
        let mut out = Vec::new();
        let Some(node) = self.first_target() else { return out };
        tags.clear();
        for leaf in self.leaves_with_fixture(node) {
            let Some(spec) = self.spec(leaf) else { continue };
            if spec.supports_function(identity) {
                if let Some(func) = spec.functions.iter().find(|f| f.identity == identity) {
                    if !tags.contains_key(&leaf) {
                        tags.insert(leaf, func.name.clone());
                        out.push((leaf, func.label.clone()));
                    }
                }
            }
        }
        out
    }

    /// `GetRenderNodesForFunctionName(functionName, identity)`: first target only.
    pub fn render_nodes_for_function_name(&self, name: &str, identity: FuncId) -> Vec<LabeledNode> {
        let mut out = Vec::new();
        let Some(node) = self.first_target() else { return out };
        for leaf in self.leaves_with_fixture(node) {
            let Some(spec) = self.spec(leaf) else { continue };
            if spec.is_function_used(name, identity) {
                if let Some(func) = spec.get_in_use_function(name, identity) {
                    out.push((leaf, func.label.clone()));
                }
            }
        }
        out
    }

    /// `GetRenderNodesForFunctionIdentity(type, nodeToFunction)`: every target, with `(label, tag)` per node.
    pub fn render_nodes_for_identity_all_targets(
        &self,
        identity: FuncId,
        node_to_function: &mut HashMap<NodeId, (String, String)>,
    ) -> Vec<NodeId> {
        let mut out = Vec::new();
        for &target in self.info.targets {
            for leaf in self.leaves_with_fixture(target) {
                let Some(spec) = self.spec(leaf) else { continue };
                if spec.supports_function(identity) {
                    if let Some(func) = spec.functions.iter().find(|f| f.identity == identity) {
                        if !node_to_function.contains_key(&leaf) {
                            node_to_function.insert(leaf, (func.label.clone(), func.name.clone()));
                            out.push(leaf);
                        }
                    }
                }
            }
        }
        out
    }

    /// `GetRenderNodesForIndexFunction`: first target's fixtures that define the function, use it on a
    /// channel, and have the index value.
    pub fn render_nodes_for_index_function(&self, function_name: &str, index_value: &str) -> Vec<NodeId> {
        let mut out = Vec::new();
        let Some(node) = self.first_target() else { return out };
        for leaf in self.leaves_with_fixture(node) {
            let Some(spec) = self.spec(leaf) else { continue };
            let Some(func) = spec.functions.iter().find(|f| f.name == function_name) else { continue };
            if spec.channels.iter().any(|c| c.function == func.name)
                && func.index_data_base().iter().any(|i| i.name == index_value)
            {
                out.push(leaf);
            }
        }
        out
    }

    /// `GetFunctionIndex`: the index item for the value on the first fixture that has the function in use.
    pub fn function_index(&self, index_value: &str, function_name: &str, identity: FuncId) -> Option<FixtureIndexItem> {
        let node = self.first_target()?;
        for leaf in self.leaves_with_fixture(node) {
            let Some(spec) = self.spec(leaf) else { continue };
            if let Some(func) = spec.get_in_use_function(function_name, identity) {
                return func.index_data_base().iter().find(|i| i.name == index_value).cloned();
            }
        }
        None
    }

    /// `UpdateSupportsCurve`: whether the selected index value maps to a curve.
    pub fn index_has_curve(&self, index_value: &str, function_name: &str, identity: FuncId) -> bool {
        if index_value.is_empty() || function_name.is_empty() {
            return false;
        }
        self.function_index(index_value, function_name, identity).map(|i| i.use_curve).unwrap_or(false)
    }
}

/// `GetNumberFrames()`.
pub fn number_of_frames(ctx: &RenderCtx) -> i32 {
    if ctx.time_span != TimeSpan::ZERO && ctx.frame_ms > 0 {
        (ctx.time_span.total_ms() / ctx.frame_ms as f64) as i32
    } else {
        0
    }
}

/// `RenderCurve(nodes, curve, functionType, tag, tags, cancellationToken)`: one range intent per curve
/// segment per node. `tags` supplies each node's function name; without it `tag` is used.
pub fn render_curve(
    ctx: &RenderCtx,
    out: &mut EffectIntents,
    nodes: &[LabeledNode],
    curve: &Curve,
    function: FuncId,
    tag: &str,
    tags: Option<&HashMap<NodeId, String>>,
) -> bool {
    if nodes.is_empty() {
        return true;
    }
    // C# Curve.GetValue throws before emitting a segment when there are fewer than two points.
    // Signal the effect to stop too, so later functions do not render after that failure.
    if curve.points.len() < 2 {
        return false;
    }
    // The C# sorts the effect's own curve in place; sorting a copy gives the same segments.
    let mut curve = curve.clone();
    curve.sort_points();

    // HashSet<double> { 0.0, x..., 100.0 }, enumerated in insertion order
    let mut point_list: Vec<f64> = vec![0.0];
    for &(x, _) in &curve.points {
        if !point_list.contains(&x) {
            point_list.push(x);
        }
    }
    if !point_list.contains(&100.0) {
        point_list.push(100.0);
    }

    let identity = function.to_core();
    let mut start_time = TimeSpan::ZERO;
    for i in 1..point_list.len() {
        let time_span = TimeSpan::from_ms(ctx.time_span.total_ms() * ((point_list[i] - point_list[i - 1]) / 100.0));
        let start_v = curve.get_value(point_list[i - 1]) / 100.0;
        let end_v = curve.get_value(point_list[i]) / 100.0;
        for (node, label) in nodes {
            let node_tag: &str = match tags {
                Some(t) => t.get(node).map(|s| s.as_str()).unwrap_or(""),
                None => tag,
            };
            let start = RangeValue::new(identity, node_tag, start_v, label);
            let end = RangeValue::new(identity, node_tag, end_v, label);
            if ctx.cancelled() {
                return true;
            }
            if let Some(element) = ctx.tree.node(*node).element_id {
                out.add_intent_for_element(element, Intent::new(IntentData::Range { start, end }, time_span), start_time);
            }
        }
        start_time += time_span;
    }
    true
}

/// `RenderIndex`: one command intent per frame, holding the index value (scaled along the curve when the
/// index uses one).
pub fn render_index(
    ctx: &RenderCtx,
    out: &mut EffectIntents,
    node: NodeId,
    index: &FixtureIndexItem,
    curve: &Curve,
    function: &FixtureFunction,
) -> Result<(), ()> {
    let Some(element) = ctx.tree.node(node).element_id else { return Ok(()) };
    let frames = number_of_frames(ctx);
    if frames <= 0 {
        return Ok(());
    }
    if index.use_curve && curve.points.len() < 2 {
        return Err(());
    }
    let tag: std::sync::Arc<str> = std::sync::Arc::from(function.name.as_str());
    let label: std::sync::Arc<str> = std::sync::Arc::from(function.label.as_str());
    // The index range is only recorded when the index uses a curve.
    let (range_minimum, range_maximum) = if index.use_curve { (index.start_value as u8, index.end_value as u8) } else { (0, 0) };
    let frame_ts = ctx.frame_timespan();
    let total_ms = ctx.time_span.total_ms();
    let nodes = out.map.entry(element).or_default();
    nodes.reserve(frames as usize);
    let mut start_time = TimeSpan::ZERO;
    for frame in 0..frames {
        let value: i32 = if index.use_curve {
            // GetEffectTimeIntervalPosition(frame), then * 100
            let pos = if ctx.time_span != TimeSpan::ZERO { (frame as f64 * ctx.frame_ms as f64) / total_ms } else { 0.0 };
            scale_curve_to_value(curve.get_value(pos * 100.0), index.end_value as f64, index.start_value as f64)
                .round_ties_even() as i32
        } else {
            index.start_value
        };
        // Named8BitCommand(int) casts with `(byte)`
        let command = vixen_core::CommandValue::Named8(vixen_core::NamedCommand {
            value: value as u8,
            index_type: index.index_type.into(),
            tag: tag.clone(),
            label: label.clone(),
            range_minimum,
            range_maximum,
        });
        nodes.push(IntentNode::new(Intent::new(IntentData::Command(command), frame_ts), start_time));
        start_time += frame_ts;
    }
    Ok(())
}

/// `FixtureEffectBase.RenderIndexed`: renders `function` for every first-target fixture that has the index.
pub fn render_indexed(
    ctx: &RenderCtx,
    tf: &FixtureTargets,
    out: &mut EffectIntents,
    index_value: &str,
    curve: &Curve,
    function: &FixtureFunction,
) -> Result<(), ()> {
    for node in tf.render_nodes_for_index_function(&function.name, index_value) {
        // `First`
        let Some(index) = function.index_data_base().iter().find(|i| i.name == index_value) else { continue };
        render_index(ctx, out, node, index, curve, function)?;
    }
    Ok(())
}

/// `FixtureIndexEffectBase.PreRenderInternal(functionName, indexValue, curve, identity)`. `Err` stands for
/// the exceptions the C# throws (a missing function or a duplicated index name), which leave the effect with
/// no output.
pub fn render_index_effect(
    ctx: &RenderCtx,
    tf: &FixtureTargets,
    out: &mut EffectIntents,
    function_name: &str,
    index_value: &str,
    curve: &Curve,
    identity: FuncId,
) -> Result<(), ()> {
    for node in tf.render_nodes_for_index_function(function_name, index_value) {
        let spec = tf.spec(node).ok_or(())?;
        let func = spec.get_in_use_function(function_name, identity).ok_or(())?;
        let mut matches = func.index_data_base().iter().filter(|i| i.name == index_value);
        let index = matches.next().ok_or(())?;
        if matches.next().is_some() {
            return Err(());
        }
        render_index(ctx, out, node, index, curve, func)?;
    }
    Ok(())
}

/// The function-name half of `UpdateFixtureCapabilities` for effects without an index value (Frost): drops a
/// name no target supports, then defaults to the first function with the identity.
pub fn repair_function_name(tf: &FixtureTargets, function_name: &mut String, identity: FuncId) {
    if tf.info.targets.is_empty() {
        return;
    }
    let names = tf.matching_function_names(identity);
    if !function_name.is_empty() && !names.contains(function_name) {
        function_name.clear();
    }
    if function_name.is_empty() {
        if let Some(first) = names.first() {
            *function_name = first.clone();
        }
    }
}

/// `DetermineFixtureCapabilities`: repairs the selected function name and index value against the targets.
/// `compatible` is the effect's `GetCompatibleIndexValues` filter. Returns whether any fixture supports the
/// function (`SupportsIndexFunction`).
pub fn determine_fixture_capabilities(
    tf: &FixtureTargets,
    function_name: &mut String,
    index_value: &mut String,
    identity: FuncId,
    compatible: fn(&FixtureIndexItem) -> bool,
) -> bool {
    if tf.info.targets.is_empty() {
        return false;
    }
    let supports = tf.fixture_functions().iter().any(|f| f.identity == identity);
    let names = tf.matching_function_names(identity);

    if !function_name.is_empty() && !names.contains(function_name) {
        function_name.clear();
    }
    if function_name.is_empty() {
        if let Some(first) = names.first() {
            *function_name = first.clone();
        }
    }

    let values = index_values(tf, function_name, identity, compatible);
    if supports {
        if index_value.is_empty() && !values.is_empty() {
            *index_value = values[0].clone();
        }
        if !values.contains(index_value) {
            *index_value = values.first().cloned().unwrap_or_default();
        }
    }
    supports
}

/// `UpdateFixtureCapabilities` of an index effect, for `targets_changed`: only when targets are assigned does the effect
/// repair its saved function name and index value. `store_index` is false for Gobo, which writes back its own index value
/// instead of the repaired one (deliberate in the original).
pub fn update_index_capabilities(
    p: &mut Params,
    t: &TargetInfo,
    name_param: &str,
    index_param: &str,
    identity: FuncId,
    compatible: fn(&FixtureIndexItem) -> bool,
    store_index: bool,
) {
    let tf = FixtureTargets::from_info(*t);
    let mut name = p.string(name_param).to_string();
    let mut index = p.string(index_param).to_string();
    determine_fixture_capabilities(&tf, &mut name, &mut index, identity, compatible);
    p.set(name_param, ParamValue::Str(name));
    if store_index {
        p.set(index_param, ParamValue::Str(index));
    }
}

/// `IndexValues`: the compatible index names of the function on the first target's fixtures.
pub fn index_values(tf: &FixtureTargets, function_name: &str, identity: FuncId, compatible: fn(&FixtureIndexItem) -> bool) -> Vec<String> {
    let mut values: Vec<String> = Vec::new();
    if let Some(first) = tf.first_target() {
        for leaf in tf.leaves_with_fixture(first) {
            let Some(spec) = tf.spec(leaf) else { continue };
            if spec.is_function_used(function_name, identity) {
                if let Some(func) = spec.get_in_use_function(function_name, identity) {
                    for item in func.index_data_base().iter().filter(|i| compatible(i)) {
                        if !values.contains(&item.name) {
                            values.push(item.name.clone());
                        }
                    }
                }
            }
        }
    }
    values
}

/// The two drop downs of an index effect: its `FunctionNameConverter` and `FixtureIndexCollectionNameConverter`. The index
/// list follows the stored function name, as `IndexValues` was last worked out for it.
pub fn index_effect_selection(
    name: &str,
    p: &Params,
    t: &TargetInfo,
    (name_param, index_param): (&str, &str),
    identity: FuncId,
    compatible: fn(&FixtureIndexItem) -> bool,
) -> Option<Vec<SelectionValue>> {
    let tf = FixtureTargets::from_info(*t);
    if name == name_param {
        Some(tf.matching_function_names(identity).into_iter().map(SelectionValue::text).collect())
    } else if name == index_param {
        Some(index_values(&tf, p.string(name_param), identity, compatible).into_iter().map(SelectionValue::text).collect())
    } else {
        None
    }
}

/// `SupportsIndexFunction`.
pub fn supports_index_function(tf: &FixtureTargets, identity: FuncId) -> bool {
    tf.fixture_functions().iter().any(|f| f.identity == identity)
}

/// Builders for tests and tools: elements that carry an intelligent fixture property.
pub mod test_support {
    use super::*;
    use crate::spec::{FixtureChannel, FixtureFunction, FixtureFunctionType, FixtureSpecification, FuncId};
    use vixen_core::element::PropertyInstance;
    use vixen_core::{ElementTree, Uuid};

    /// Gives every leaf under `node` an IntelligentFixture property holding `spec`.
    pub fn add_fixture_property(tree: &mut ElementTree, node: NodeId, spec: &FixtureSpecification) {
        let xml = spec.to_raw_xml();
        for leaf in tree.distinct_leaves(node) {
            tree.node_mut(leaf).properties.push(PropertyInstance {
                type_id: INTELLIGENT_FIXTURE_TYPE_ID,
                instance_id: Uuid::new_v4(),
                data: PropertyData::Unknown { raw_xml: xml.clone() },
            });
        }
    }

    /// A group of `count` moving heads, each with `spec`.
    pub fn fixtures(tree: &mut ElementTree, name: &str, count: usize, spec: &FixtureSpecification) -> NodeId {
        let group = tree.add_string(name, count);
        add_fixture_property(tree, group, spec);
        group
    }

    /// A moving head with pan, tilt, dimmer, zoom, frost, a gobo wheel and a shutter, each used on a channel.
    pub fn sample_spec() -> FixtureSpecification {
        let mut spec = FixtureSpecification { name: "Test Head".into(), ..Default::default() };
        let mut add = |name: &str, ty: FixtureFunctionType, id: FuncId, label: &str| {
            spec.functions.push(FixtureFunction {
                name: name.into(),
                function_type: ty,
                identity: id,
                label: label.into(),
                ..Default::default()
            });
            let n = spec.channels.len() as i32 + 1;
            spec.channels.push(FixtureChannel { name: name.into(), function: name.into(), channel_number: n });
        };
        add("Pan", FixtureFunctionType::Range, FuncId::Pan, "P");
        add("Tilt", FixtureFunctionType::Range, FuncId::Tilt, "T");
        add("Dimmer", FixtureFunctionType::Range, FuncId::Dim, "D");
        add("Zoom", FixtureFunctionType::Range, FuncId::Zoom, "Z");
        add("Frost", FixtureFunctionType::Range, FuncId::Frost, "F");
        add("Gobo Wheel", FixtureFunctionType::Indexed, FuncId::Gobo, "G");
        add("Shutter", FixtureFunctionType::Indexed, FuncId::Shutter, "S");
        spec
    }
}
