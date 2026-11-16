//! The owner's real show, end to end through `OutputGraph`. The files are only ever read.

mod common;

use common::{load_real, RealData};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;
use vixen_core::{Color, CombinedState, DiscreteValue, LightingValue, RgbValue};
use vixen_output::filters::{ColorBreakdownConfig, FilterConfig};
use vixen_output::graph::{OutputAddress, OutputGraph};
use vixen_output::{ids, ControllerConfig, DotNetColor, OutputCommand};

fn build(real: &RealData) -> OutputGraph {
    OutputGraph::build(&real.def, &real.modules)
}

/// Expected reach, worked out from the raw patches without going through the graph: for each element, each output it reaches,
/// with the filter output it comes through (`None` when patched straight to the element).
fn expected_reach(real: &RealData) -> HashMap<Uuid, Vec<(OutputAddress, Option<i32>)>> {
    let mut output_address = HashMap::new();
    for (c, controller) in real.def.controllers.iter().enumerate() {
        for (o, output) in controller.outputs.iter().enumerate() {
            output_address.insert(output.id, OutputAddress { controller: c, output: o });
        }
    }
    let mut by_source: HashMap<Uuid, Vec<(Uuid, i32)>> = HashMap::new();
    for p in &real.def.patches {
        by_source.entry(p.source_id.expect("real patches all have sources")).or_default().push((p.component_id, p.output_index));
    }
    let mut reach = HashMap::new();
    for element in &real.def.elements {
        let mut outputs = Vec::new();
        let mut walk = vec![*element];
        while let Some(component) = walk.pop() {
            for (child, index) in by_source.get(&component).into_iter().flatten() {
                match output_address.get(child) {
                    Some(address) => outputs.push((*address, (component != *element).then_some(*index))),
                    None => walk.push(*child),
                }
            }
        }
        reach.insert(*element, outputs);
    }
    reach
}

#[test]
fn the_real_graph_builds_with_nothing_left_over() {
    let Some(real) = load_real() else { return };
    assert_eq!(real.def.elements.len(), 1133);
    assert_eq!(real.def.filters.len(), 1117);
    assert_eq!(real.def.patches.len(), 4484);
    let graph = build(&real);
    assert_eq!(graph.warnings(), &[]);
    assert_eq!(graph.element_count(), 1133);
    assert_eq!(graph.filter_count(), 1117);
    assert_eq!(graph.active_filter_count(), 1117);
    assert_eq!(graph.output_count(), 3367);

    let controllers = graph.controllers();
    assert_eq!(controllers.len(), 2);
    assert_eq!(controllers[0].name, "Arduino");
    assert_eq!(controllers[0].hardware_type_id, ids::GENERIC_SERIAL);
    assert_eq!(controllers[0].output_count(), 16);
    assert_eq!(controllers[1].name, "Light Strip Controller");
    assert_eq!(controllers[1].hardware_type_id, ids::DUMMY_LIGHTING);
    assert_eq!(controllers[1].output_count(), 3351);

    // Module data was found and read, not defaulted.
    let Some(ControllerConfig::GenericSerial(arduino)) = &controllers[0].config else {
        panic!("Arduino should have GenericSerial data")
    };
    assert_eq!(arduino.serial.port_name.as_deref(), Some("COM9"));
    assert_eq!(arduino.serial.baud_rate, 9600);
    assert_eq!(arduino.serial.data_bits, 8);
    assert_eq!(arduino.header.as_deref(), Some(""));
}

#[test]
fn every_real_breakdown_is_the_rgb_template() {
    let Some(real) = load_real() else { return };
    let graph = build(&real);
    let template = ColorBreakdownConfig::template("RGB").unwrap();
    for f in &real.def.filters {
        let Some(FilterConfig::ColorBreakdown(c)) = graph.filter_config(&f.instance_id) else {
            panic!("{} should be a breakdown", f.instance_id)
        };
        assert!(c.mix_colors);
        assert!(!c.sixteen_bit);
        assert_eq!(c.breakdown_items, template);
        assert_eq!(c.breakdown_items[0].color, DotNetColor::RED);
    }
}

#[test]
fn every_real_element_reaches_exactly_the_outputs_its_patches_say() {
    let Some(real) = load_real() else { return };
    let mut graph = build(&real);
    let reach = expected_reach(&real);

    let mut total = 0;
    let mut states = vec![CombinedState::default(); real.def.elements.len()];
    let mut bytes: Vec<Vec<u8>> = graph.controllers().iter().map(|c| vec![0; c.output_count()]).collect();
    for (e, id) in real.def.elements.iter().enumerate() {
        let expected: Vec<OutputAddress> = reach[id].iter().map(|r| r.0).collect();
        assert!(!expected.is_empty(), "{} should reach an output", real.element_names[e]);

        // The graph's own idea of the same thing.
        let mut sorted = expected.clone();
        sorted.sort();
        assert_eq!(graph.outputs_for_element(e), sorted, "{}", real.element_names[e]);

        // And what the data really does: light the element, and only those outputs change.
        states[e].rgb = Some(RgbValue { r: 255, g: 255, b: 255 });
        graph.evaluate(&states);
        states[e].rgb = None;
        for (c, out) in bytes.iter_mut().enumerate() {
            graph.copy_bytes(c, out);
        }
        let mut lit = Vec::new();
        for (c, out) in bytes.iter().enumerate() {
            for (o, v) in out.iter().enumerate() {
                if *v != 0 {
                    assert_eq!(*v, 255);
                    lit.push(OutputAddress { controller: c, output: o });
                }
            }
        }
        assert_eq!(lit, sorted, "{}", real.element_names[e]);
        total += lit.len();
    }
    // Every patched output was seen exactly once.
    assert_eq!(total, 3367);
}

/// (R, G, B) for an element, different for every element so a swapped wire can't pass by accident.
fn colour_for(element: usize) -> (u8, u8, u8) {
    ((element * 3 % 256) as u8, ((element * 5 + 1) % 256) as u8, ((element * 7 + 2) % 256) as u8)
}

#[test]
fn every_real_output_carries_the_right_channel_of_the_right_element() {
    let Some(real) = load_real() else { return };
    let mut graph = build(&real);
    let reach = expected_reach(&real);
    let states: Vec<CombinedState> = (0..real.def.elements.len())
        .map(|e| {
            let (r, g, b) = colour_for(e);
            CombinedState { rgb: Some(RgbValue { r, g, b }), ..Default::default() }
        })
        .collect();
    graph.evaluate(&states);

    let mut checked = 0;
    for (e, id) in real.def.elements.iter().enumerate() {
        let (r, g, b) = colour_for(e);
        for (address, via) in &reach[id] {
            let expected = match via {
                Some(0) => r,
                Some(1) => g,
                Some(2) => b,
                Some(other) => panic!("unexpected breakdown output {other}"),
                // Straight to a controller output: the 8-bit evaluator takes the highest component.
                None => r.max(g).max(b),
            };
            assert_eq!(
                graph.commands(address.controller)[address.output],
                OutputCommand::U8(expected),
                "{} {:?}",
                real.element_names[e],
                address
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 3367);
}

#[test]
fn a_real_full_colour_element_gives_its_rgb_bytes() {
    let Some(real) = load_real() else { return };
    let mut graph = build(&real);
    let e = real.element("L Pixel-518");
    let strip = 1;
    assert_eq!(graph.controllers()[strip].name, "Light Strip Controller");
    let address = |output| OutputAddress { controller: strip, output };
    assert_eq!(graph.outputs_for_element(e), vec![address(1551), address(1552), address(1553)]);
    assert_eq!(graph.controllers()[strip].output_names[1551], "Output 1552");

    let mut states = vec![CombinedState::default(); real.def.elements.len()];
    let channels = |g: &OutputGraph| g.commands(strip)[1551..=1553].to_vec();
    let u8s = |r, g, b| vec![OutputCommand::U8(r), OutputCommand::U8(g), OutputCommand::U8(b)];

    // A mixed full-colour state.
    states[e].rgb = Some(RgbValue { r: 200, g: 100, b: 50 });
    graph.evaluate(&states);
    assert_eq!(channels(&graph), u8s(200, 100, 50));

    // A lone lighting state, through the mixing filter: `intensity * (component / 255f)`, truncated to a byte.
    states[e] = CombinedState { lighting: Some(LightingValue::new(Color::RED, 0.5)), ..Default::default() };
    graph.evaluate(&states);
    assert_eq!(channels(&graph), u8s(127, 0, 0));
    states[e] = CombinedState { lighting: Some(LightingValue::new(Color::rgb(255, 128, 64), 1.0)), ..Default::default() };
    graph.evaluate(&states);
    assert_eq!(channels(&graph), u8s(255, 128, 64));
    states[e] = CombinedState { lighting: Some(LightingValue::new(Color::WHITE, 0.25)), ..Default::default() };
    graph.evaluate(&states);
    assert_eq!(channels(&graph), u8s(63, 63, 63));

    // The mixing filters have no discrete handler, and a breakdown output is a command, not null, when it's dark.
    let mut discrete = CombinedState::default();
    discrete.discrete.push(DiscreteValue::new(Color::RED, 1.0));
    states[e] = discrete;
    graph.evaluate(&states);
    assert_eq!(channels(&graph), u8s(0, 0, 0));
    states[e] = CombinedState::default();
    graph.evaluate(&states);
    assert_eq!(channels(&graph), u8s(0, 0, 0));
}

#[test]
fn a_real_discrete_single_colour_element_gives_its_one_output() {
    let Some(real) = load_real() else { return };
    let mut graph = build(&real);
    let arduino = 0;
    assert_eq!(graph.controllers()[arduino].name, "Arduino");

    // These two are crossed in the real patches.
    for (name, output) in [
        ("White Bottom", 0),
        ("White Top", 2),
        ("Star", 7),
        ("Mini Tree L", 8),
        ("Mini Tree R", 9),
        ("Plug 16", 15),
        ("Plug 11", 10),
    ] {
        let e = real.element(name);
        assert_eq!(graph.outputs_for_element(e), vec![OutputAddress { controller: arduino, output }], "{name}");

        let mut states = vec![CombinedState::default(); real.def.elements.len()];
        states[e].discrete.push(DiscreteValue::new(Color::WHITE, 0.5));
        graph.evaluate(&states);
        let commands = graph.commands(arduino);
        assert_eq!(commands[output], OutputCommand::U8(127), "{name}");
        // Everything else on the controller has no state, and a direct patch with no state is a null command.
        for (o, c) in commands.iter().enumerate() {
            if o != output {
                assert_eq!(*c, OutputCommand::None, "{name} output {o}");
            }
        }

        // Highest of several discrete colours, then the state cleared.
        states[e].discrete.push(DiscreteValue::new(Color::RED, 0.75));
        graph.evaluate(&states);
        assert_eq!(graph.commands(arduino)[output], OutputCommand::U8(191), "{name}");
        states[e].discrete.clear();
        graph.evaluate(&states);
        assert_eq!(graph.commands(arduino)[output], OutputCommand::None, "{name}");
    }
}

#[test]
fn a_dark_frame_after_a_lit_one_is_all_zero_on_the_wire() {
    let Some(real) = load_real() else { return };
    let mut graph = build(&real);
    let lit: Vec<CombinedState> = (0..real.def.elements.len())
        .map(|_| CombinedState { rgb: Some(RgbValue { r: 255, g: 255, b: 255 }), ..Default::default() })
        .collect();
    graph.evaluate(&lit);
    assert!(graph.commands(1).iter().all(|c| *c == OutputCommand::U8(255)));
    assert!(graph.commands(0).iter().all(|c| *c == OutputCommand::U8(255)));

    graph.evaluate(&vec![CombinedState::default(); real.def.elements.len()]);
    assert!(graph.commands(0).iter().all(|c| c.byte() == 0));
    assert!(graph.commands(1).iter().all(|c| c.byte() == 0));
    // No state at all means the same.
    graph.evaluate(&[]);
    assert!(graph.commands(1).iter().all(|c| c.byte() == 0));
}

#[test]
fn a_real_export_row_runs_controller_after_controller() {
    let Some(real) = load_real() else { return };
    let mut graph = build(&real);
    let mut states = vec![CombinedState::default(); real.def.elements.len()];
    states[real.element("White Middle")].discrete.push(DiscreteValue::new(Color::WHITE, 1.0));
    states[real.element("L Pixel-518")].rgb = Some(RgbValue { r: 1, g: 2, b: 3 });
    graph.evaluate(&states);
    let mut row = vec![9u8; graph.output_count()];
    assert_eq!(graph.fill_row(&[0, 1], &mut row), 3367);
    assert_eq!(row[1], 255);
    assert_eq!(&row[16 + 1551..16 + 1554], &[1, 2, 3]);
    assert_eq!(row.iter().filter(|v| **v != 0).count(), 4);
}

#[test]
fn real_output_counts_per_filter_output_are_three_each() {
    let Some(real) = load_real() else { return };
    let reach = expected_reach(&real);
    let mut census: BTreeMap<usize, usize> = BTreeMap::new();
    for outputs in reach.values() {
        *census.entry(outputs.len()).or_default() += 1;
    }
    // 16 direct elements and 1117 RGB elements.
    assert_eq!(census, BTreeMap::from([(1, 16), (3, 1117)]));
}

#[test]
fn the_real_files_say_what_the_runner_should_do() {
    let Some(real) = load_real() else { return };
    assert_eq!(real.default_update_interval_ms, 10);
    let graph = build(&real);
    // Both controllers are disabled in this show's `DisabledDevices`.
    assert!(graph.controllers().iter().all(|c| real.disabled.contains(&c.id)));
}
