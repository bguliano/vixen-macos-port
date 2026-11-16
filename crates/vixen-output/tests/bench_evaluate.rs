//! How long `evaluate` takes on the owner's real show. Run in release to see the number:
//!
//! `cargo test --release -p vixen-output --test bench_evaluate -- --nocapture`
//!
//! The limit is only asserted in release, as the test profile isn't what ships.

mod common;

use std::time::{Duration, Instant};
use vixen_core::{Color, CombinedState, DiscreteValue, LightingValue, RgbValue};
use vixen_output::graph::OutputGraph;
use vixen_output::{ControllerRunner, OutputCommand};

fn measure(graph: &mut OutputGraph, frames: &[Vec<CombinedState>], iterations: usize) -> (Duration, Duration) {
    let mut worst = Duration::ZERO;
    let started = Instant::now();
    for i in 0..iterations {
        let frame = &frames[i % frames.len()];
        let before = Instant::now();
        graph.evaluate(frame);
        worst = worst.max(before.elapsed());
        std::hint::black_box(graph.commands(0));
    }
    (started.elapsed() / iterations as u32, worst)
}

fn report(label: &str, mean: Duration, worst: Duration) {
    println!("{label:<34} mean {:>8.1} us   worst {:>8.1} us", mean.as_secs_f64() * 1e6, worst.as_secs_f64() * 1e6);
}

#[test]
fn evaluate_on_the_real_graph_is_under_a_millisecond() {
    let Some(real) = common::load_real() else { return };
    let build_started = Instant::now();
    let mut graph = OutputGraph::build(&real.def, &real.modules);
    println!(
        "build: {:.1} ms for {} elements, {} filters, {} outputs",
        build_started.elapsed().as_secs_f64() * 1000.0,
        graph.element_count(),
        graph.filter_count(),
        graph.output_count()
    );

    let n = graph.element_count();
    let rgb: Vec<Vec<CombinedState>> = (0..8u32)
        .map(|f| {
            (0..n)
                .map(|e| {
                    let v = (e as u32 + f * 7) as u8;
                    CombinedState {
                        rgb: Some(RgbValue { r: v, g: v.wrapping_mul(3), b: v.wrapping_mul(5) }),
                        ..Default::default()
                    }
                })
                .collect()
        })
        .collect();
    let lighting: Vec<Vec<CombinedState>> = vec![(0..n)
        .map(|e| CombinedState { lighting: Some(LightingValue::new(Color::rgb(e as u8, 128, 64), 0.75)), ..Default::default() })
        .collect()];
    let discrete: Vec<Vec<CombinedState>> = vec![(0..n)
        .map(|_| {
            let mut s = CombinedState::default();
            s.discrete.push(DiscreteValue::new(Color::RED, 0.5));
            s.discrete.push(DiscreteValue::new(Color::BLUE, 0.25));
            s
        })
        .collect()];
    let dark: Vec<Vec<CombinedState>> = vec![vec![CombinedState::default(); n]];

    let iterations = if cfg!(debug_assertions) { 200 } else { 5000 };
    let (rgb_mean, rgb_worst) = measure(&mut graph, &rgb, iterations);
    report("all elements mixed RGB", rgb_mean, rgb_worst);
    let (lit_mean, lit_worst) = measure(&mut graph, &lighting, iterations);
    report("all elements lone lighting", lit_mean, lit_worst);
    let (dis_mean, dis_worst) = measure(&mut graph, &discrete, iterations);
    report("all elements two discrete", dis_mean, dis_worst);
    let (dark_mean, dark_worst) = measure(&mut graph, &dark, iterations);
    report("all elements dark", dark_mean, dark_worst);

    // With the hand-off to the controllers' threads, which is what a frame really costs the engine thread.
    let mut runner = ControllerRunner::new(Duration::from_millis(10));
    for c in graph.controllers() {
        let mut module = vixen_output::create_module(
            vixen_output::ControllerConfig::new_default(&vixen_output::ids::DEBUG_CONTROLLER, c.hardware_instance_id).unwrap(),
        );
        module.set_output_count(c.output_count());
        runner.add_controller(c.id, &c.name, module, true);
    }
    runner.start_all();
    std::thread::sleep(Duration::from_millis(5));
    let mut worst = Duration::ZERO;
    let started = Instant::now();
    for i in 0..iterations {
        let before = Instant::now();
        graph.evaluate(&rgb[i % rgb.len()]);
        runner.publish(&graph);
        worst = worst.max(before.elapsed());
    }
    let mean = started.elapsed() / iterations as u32;
    runner.stop_all();
    report("RGB, evaluate and publish", mean, worst);

    assert_eq!(graph.commands(1).iter().filter(|c| matches!(c, OutputCommand::U8(_))).count(), 3351);
    if !cfg!(debug_assertions) {
        for (label, m) in
            [("RGB", rgb_mean), ("lighting", lit_mean), ("discrete", dis_mean), ("dark", dark_mean), ("publish", mean)]
        {
            assert!(m < Duration::from_millis(1), "{label} took {m:?} per frame");
        }
    }
}
