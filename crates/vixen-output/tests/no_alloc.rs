//! `evaluate` and the hand-off to controllers run on the engine thread 100 times a second, and may not allocate. This binary
//! installs a counting allocator, so it only holds this one test's worth of checks.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use vixen_core::value::{FixtureIndexType, NamedCommand};
use vixen_core::{
    Color, CombinedState, CommandValue, DiscreteValue, FunctionIdentity, LightingValue, RangeValue, RgbValue, StateValue,
};
use vixen_output::filters::{
    CoarseFineConfig, ColorBreakdownConfig, CurveData, DimmingCurveConfig, DimmingFilterConfig, FilterConfig,
    ShutterFilterConfig, TaggedFilterConfig,
};
use vixen_output::graph::{ControllerDef, FilterDef, GraphDefinition, OutputDef, OutputGraph, PatchDef};
use vixen_output::{ids, ModuleIds};
use vixen_xml::XElement;

struct Counting;

thread_local! {
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

// SAFETY: defers everything to the system allocator. The thread-local has a const initialiser and no destructor, so reading it
// can't allocate or re-enter.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|c| c.set(c.get() + 1));
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|c| c.set(c.get() + 1));
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn allocations_during(f: impl FnOnce()) -> u64 {
    let before = ALLOCATIONS.with(Cell::get);
    f();
    ALLOCATIONS.with(Cell::get) - before
}

/// Every filter type in one element's path, with states of every kind, including the reference-counted ones.
#[test]
fn evaluating_every_filter_type_allocates_nothing_once_settled() {
    let id = Uuid::from_u128;
    let ctrl = id(9000);
    let instance = id(9001);
    let outputs: Vec<OutputDef> = (0..8u128).map(|o| OutputDef { id: id(1000 + o), name: format!("Output {}", o + 1) }).collect();
    let configs: Vec<(u128, FilterConfig)> = vec![
        (
            100,
            FilterConfig::DimmingCurve(DimmingCurveConfig {
                curve: CurveData::from_points(&[(0.0, 0.0), (100.0, 50.0)]),
                ..Default::default()
            }),
        ),
        (
            101,
            FilterConfig::ColorBreakdown(ColorBreakdownConfig {
                breakdown_items: ColorBreakdownConfig::template("RGBW").unwrap(),
                mix_colors: true,
                ..Default::default()
            }),
        ),
        (102, FilterConfig::Tagged(TaggedFilterConfig { tag: Some("Pan".to_string()), ..Default::default() })),
        (103, FilterConfig::CoarseFine(CoarseFineConfig::default())),
        (
            104,
            FilterConfig::Dimming(DimmingFilterConfig {
                tag: Some("Dim".to_string()),
                convert_rgb_into_dimming_intents: true,
                ..Default::default()
            }),
        ),
        (
            105,
            FilterConfig::Shutter(ShutterFilterConfig {
                tag: Some("Shutter".to_string()),
                convert_color_into_shutter_intents: true,
                open_shutter_index_value: 255,
                ..Default::default()
            }),
        ),
    ];
    let mut data = HashMap::new();
    let mut filters = Vec::new();
    for (n, config) in &configs {
        let mut c = config.clone();
        let m = ModuleIds::new(c.ids().type_id, id(*n));
        *c.ids_mut() = m;
        data.insert((m.type_id, m.instance_id), c.to_xml() as XElement);
        filters.push(FilterDef { type_id: m.type_id, instance_id: m.instance_id });
    }
    let patch = |component: u128, source: u128, output: i32| PatchDef {
        component_id: id(component),
        source_id: Some(id(source)),
        output_index: output,
    };
    let def = GraphDefinition {
        elements: vec![id(0)],
        controllers: vec![ControllerDef {
            id: ctrl,
            name: "c".to_string(),
            hardware_type_id: ids::E131,
            hardware_instance_id: instance,
            outputs,
        }],
        filters,
        patches: vec![
            patch(100, 0, 0),
            patch(101, 100, 0),
            patch(1000, 101, 0),
            patch(1001, 101, 3),
            patch(102, 0, 0),
            patch(103, 102, 0),
            patch(1002, 103, 0),
            patch(1003, 103, 1),
            patch(104, 0, 0),
            patch(1004, 104, 0),
            patch(105, 0, 0),
            patch(1005, 105, 0),
            patch(1006, 0, 0),
        ],
    };
    let mut graph = OutputGraph::build(&def, &data);
    assert_eq!(graph.warnings(), &[]);

    let named = |tag: &str, t: FixtureIndexType| {
        StateValue::Command(CommandValue::Named8(NamedCommand {
            value: 5,
            index_type: t,
            tag: Arc::from(tag),
            label: Arc::from(""),
            range_minimum: 0,
            range_maximum: 0,
        }))
    };
    let frames: Vec<Vec<CombinedState>> = vec![
        vec![{
            let mut s = CombinedState { rgb: Some(RgbValue { r: 200, g: 100, b: 50 }), ..Default::default() };
            s.other.push(StateValue::Range(RangeValue::new(FunctionIdentity::Pan, "Pan", 0.5, "")));
            s.other.push(named("Shutter", FixtureIndexType::Strobe));
            s.discrete.push(DiscreteValue::new(Color::WHITE, 0.5));
            s
        }],
        vec![CombinedState { lighting: Some(LightingValue::new(Color::WHITE, 0.5)), ..Default::default() }],
        vec![{
            let mut s = CombinedState::default();
            s.other.push(StateValue::Range(RangeValue::new(FunctionIdentity::Dim, "Dim", 0.25, "")));
            s
        }],
        vec![CombinedState::default()],
    ];
    for frame in &frames {
        graph.evaluate(frame);
    }
    let allocations = allocations_during(|| {
        for _ in 0..5 {
            for frame in &frames {
                graph.evaluate(frame);
            }
        }
    });
    assert_eq!(allocations, 0);
}

#[test]
fn the_runner_keeps_its_interval_constant_public() {
    assert_eq!(vixen_output::runner::DEFAULT_UPDATE_INTERVAL, Duration::from_millis(50));
}
