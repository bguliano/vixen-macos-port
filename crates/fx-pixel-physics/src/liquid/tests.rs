use super::audio::test_support::FakeAudio;
use super::*;
use crate::test_util::{in_contract_order, info, rgb, Grid};
use vixen_effects::testing;

fn enum_value(name: &str) -> ParamValue {
    ParamValue::Enum(name.to_string())
}

fn flat(v: f64) -> ParamValue {
    ParamValue::Curve(Curve::flat(v))
}

fn solid(color: Color) -> ParamValue {
    ParamValue::Gradient(ColorGradient::solid(color))
}

/// `new Emitter()`, standing still at these curve positions, with no spread and pointing nowhere.
fn still_emitter(x: f64, y: f64) -> Params {
    let mut e = Params::defaults(&gen::EMITTER_PARAMS);
    e.set("X", flat(x));
    e.set("Y", flat(y));
    e.set("ParticleVelocity", flat(0.0));
    e
}

fn with_emitters(p: &mut Params, emitters: Vec<Params>) {
    p.set("EmitterData", ParamValue::RecordList(emitters));
}

fn visible(p: &Params) -> Vec<&'static str> {
    let with_audio = TargetInfo { audio_loaded: true, ..TargetInfo::detached() };
    DESCRIPTOR.visible_params_for(p, &with_audio).into_iter().map(|i| DESCRIPTOR.params[i].name).collect()
}

/// A render context for a grid, with marks and audio.
fn ctx<'a>(
    grid: &'a Grid,
    targets: &'a [NodeId],
    marks: &'a [MarkCollection],
    audio: Option<&'a dyn AudioSource>,
    cancel: &'a AtomicBool,
    ms: i64,
) -> RenderCtx<'a> {
    RenderCtx { video_cache: None,
        tree: &grid.tree,
        targets,
        start_time: TimeSpan::ZERO,
        time_span: TimeSpan::from_ms_i(ms),
        frame_ms: 50,
        marks,
        audio,
        module_data_root: None,
        cancel,
    }
}

#[test]
fn smoke() {
    testing::smoke_test(&DESCRIPTOR);
    testing::assert_produces_light(&DESCRIPTOR);
}

// ---- the tables

#[test]
fn emitter_table_is_in_data_contract_order() {
    assert!(in_contract_order(&gen::EMITTER_PARAMS));
    assert_eq!(gen::EMITTER_DATA.xml_ns, "Liquid");
}

#[test]
fn a_new_effect_starts_with_the_two_emitters_of_liquid_data() {
    let p = DESCRIPTOR.new_params();
    let emitters = p.records("EmitterData");
    assert_eq!(emitters.len(), 2);

    // new EmitterData(), the first animated.
    assert!(emitters[0].boolean("Animate"));
    assert_eq!(emitters[0].curve("Y").get_value(50.0), 10.0);
    assert_eq!(emitters[0].gradient("Color").get_color_at(0.0), Color::BLUE);
    assert_eq!(emitters[0].curve("ParticleVelocity").get_value(50.0), 50.0);

    // The second static, purple and near the top.
    assert!(!emitters[1].boolean("Animate"));
    assert_eq!(emitters[1].curve("Y").get_value(50.0), 90.0);
    assert_eq!(emitters[1].gradient("Color").get_color_at(0.0), Color::rgb(128, 0, 128));

    for e in emitters {
        assert_eq!(e.enum_name("ParticleType"), "Water");
        assert_eq!(e.enum_name("FlowControl"), "Continuous");
        assert_eq!(e.enum_name("NozzleMovement"), "FixedAngle");
        assert_eq!(e.curve("Flow").get_value(50.0), 10.0);
        assert_eq!(e.curve("NozzleAngle").get_value(50.0), 75.0);
        assert_eq!(e.curve("OscillationSpeed").get_value(50.0), 5.0);
        assert_eq!(e.int("FramesPerColor"), 3);
        assert_eq!(e.int("OnTime"), 2000);
        assert_eq!(e.guid("MarkCollectionId"), Uuid::nil());
        assert!(e.boolean("ManualFlow"));
    }
}

#[test]
fn an_added_emitter_is_a_new_emitter() {
    let e = Params::defaults(&gen::EMITTER_PARAMS);
    assert_eq!(e.gradient("Color").get_color_at(0.0), Color::WHITE);
    assert_eq!(e.curve("ParticleVelocity").get_value(50.0), 100.0);
    assert_eq!(e.curve("Y").get_value(50.0), 50.0);
    assert_eq!(e.curve("Brightness").get_value(50.0), 100.0);
}

#[test]
fn particle_types_match_the_wrapper_enum() {
    let names: Vec<_> = gen::ENUM_ParticleType.members.iter().map(|m| m.name).collect();
    assert_eq!(names, ["Elastic", "Powder", "Tensile", "Spring", "Viscous", "StaticPressure", "Water", "Reactive", "Repulsive"]);
    for (i, name) in names.iter().enumerate() {
        assert_eq!(particle_type_index(name), i as i32);
    }
}

// ---- visibility

#[test]
fn musical_emitters_show_the_audio_settings() {
    let mut p = DESCRIPTOR.new_params();
    let v = visible(&p);
    for name in ["Gain", "LowPass", "HighPass", "Range", "Normalize", "AttackTime", "DecayTime", "LowPassFreq", "HighPassFreq"] {
        assert!(!v.contains(&name), "{name}");
    }

    let mut e = Params::defaults(&gen::EMITTER_PARAMS);
    e.set("FlowControl", enum_value("Musical"));
    with_emitters(&mut p, vec![e]);
    let v = visible(&p);
    for name in ["Gain", "LowPass", "HighPass", "Range", "Normalize", "AttackTime", "DecayTime"] {
        assert!(v.contains(&name), "{name}");
    }
    assert!(!v.contains(&"LowPassFreq"), "follows its switch");

    p.set("LowPass", ParamValue::Bool(true));
    assert!(visible(&p).contains(&"LowPassFreq"));
}

#[test]
fn the_colour_list_shows_when_an_emitter_uses_it() {
    let mut p = DESCRIPTOR.new_params();
    assert!(!visible(&p).contains(&"Colors"));
    let mut e = Params::defaults(&gen::EMITTER_PARAMS);
    e.set("UseColorArray", ParamValue::Bool(true));
    with_emitters(&mut p, vec![e]);
    assert!(visible(&p).contains(&"Colors"));
}

#[test]
fn locations_swap_orientation_for_scale_factor() {
    let mut p = DESCRIPTOR.new_params();
    let v = visible(&p);
    assert!(v.contains(&"StringOrientation") && v.contains(&"TargetPositioning"));
    assert!(!v.contains(&"RenderScaleFactor"));
    p.set("TargetPositioning", enum_value("Locations"));
    let v = visible(&p);
    assert!(!v.contains(&"StringOrientation"));
    assert!(v.contains(&"RenderScaleFactor"));
    param_changed("TargetPositioning", &mut p);
    assert_eq!(p.enum_name("StringOrientation"), "Vertical");
}

#[test]
fn without_audio_a_musical_emitter_offers_no_audio_settings() {
    let mut e = Params::defaults(&gen::EMITTER_PARAMS);
    e.set("FlowControl", enum_value("Musical"));
    let mut p = DESCRIPTOR.new_params();
    p.set("EmitterData", ParamValue::RecordList(vec![e]));
    assert!(visible(&p).contains(&"Gain"));
    let names: Vec<&str> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
    assert!(!names.contains(&"Gain") && !names.contains(&"LowPass"));
}

#[test]
fn an_emitter_following_marks_selects_a_collection() {
    let mut e = Params::defaults(&gen::EMITTER_PARAMS);
    e.set("FlowControl", enum_value("UseMarks"));
    let continuous = Params::defaults(&gen::EMITTER_PARAMS);
    let marks = [MarkCollection::default()];
    let mut p = DESCRIPTOR.new_params();
    let grid = Grid::new(4, 4);
    let targets = [grid.root];
    let t = TargetInfo { marks: &marks, ..vixen_effects::testing::target_info(&grid.tree, &targets) };
    DESCRIPTOR.set_param(&mut p, "EmitterData", ParamValue::RecordList(vec![e, continuous]), &t);
    assert_eq!(p.records("EmitterData")[0].guid("MarkCollectionId"), marks[0].id);
    assert!(p.records("EmitterData")[1].guid("MarkCollectionId").is_nil());
}

#[test]
fn emitter_rows_carry_their_own_visibility() {
    assert!(gen::EMITTER_DATA.visibility.is_some());
}

#[test]
fn an_emitter_record_shows_what_its_modes_use() {
    let get = |v: &Visibility, name: &str| v.overrides.iter().find(|o| o.0 == name).unwrap().1;
    let mut e = Params::defaults(&gen::EMITTER_PARAMS);
    let mut v = Visibility::default();
    emitter_visibility(&e, &e, &mut v);
    assert!(get(&v, "X") && !get(&v, "VelocityX") && get(&v, "Flow") && !get(&v, "OnTime") && !get(&v, "MarkCollectionId"));

    e.set("Animate", ParamValue::Bool(true));
    e.set("RandomStartingPosition", ParamValue::Bool(false));
    e.set("FlowControl", enum_value("UseMarks"));
    e.set("NozzleMovement", enum_value("SpinClockwise"));
    let mut v = Visibility::default();
    emitter_visibility(&e, &e, &mut v);
    assert!(!get(&v, "X") && get(&v, "VelocityX") && get(&v, "AnimateXStart") && !get(&v, "Flow") && get(&v, "MarkCollectionId"));
    assert!(!get(&v, "NozzleAngle") && get(&v, "OscillationSpeed") && !get(&v, "OscillateStartAngle"));
}

// ---- rendering

#[test]
fn particles_appear_at_the_emitter_on_the_first_frame() {
    // 25 wide, 8 high. X 50% rounds 12.5 to the even 12, Y 75% is 6. The world steps first and emits after, so
    // gravity hasn't touched them yet, and a source size of zero keeps them within half a unit.
    let grid = Grid::new(8, 25);
    let f = grid.render(&DESCRIPTOR, 500, |p| with_emitters(p, vec![still_emitter(50.0, 75.0)]));
    let lit = f.lit_pixels(0);
    assert!(!lit.is_empty());
    assert!(lit.iter().all(|p| (11..=13).contains(&p.0) && (5..=7).contains(&p.1)), "{lit:?}");
}

#[test]
fn no_flow_no_light() {
    let grid = Grid::new(8, 25);
    let mut e = still_emitter(50.0, 50.0);
    e.set("Flow", flat(0.0));
    assert!(grid.render(&DESCRIPTOR, 500, |p| with_emitters(p, vec![e])).is_dark());
}

#[test]
fn no_emitters_no_intents() {
    let grid = Grid::new(4, 4);
    assert!(grid.render(&DESCRIPTOR, 500, |p| with_emitters(p, Vec::new())).is_empty());
}

#[test]
fn particles_take_the_emitter_colour_and_brightness() {
    let grid = Grid::new(8, 25);
    let mut e = still_emitter(50.0, 75.0);
    e.set("Color", solid(Color::rgb(0, 0, 255)));
    e.set("Brightness", flat(50.0));
    let f = grid.render(&DESCRIPTOR, 100, |p| with_emitters(p, vec![e]));
    let (x, y) = f.lit_pixels(0)[0];
    // V is 0.5 and to_color truncates 127.5. The particle's float round trip keeps 127.
    assert_eq!(f.at(0, x, y), rgb(0, 0, 127));
}

#[test]
fn gravity_pulls_particles_down_the_frame_buffer() {
    let grid = Grid::new(30, 30);
    let mut e = still_emitter(50.0, 80.0);
    e.set("Flow", flat(1.0));
    let f = grid.render(&DESCRIPTOR, 2000, |p| {
        with_emitters(p, vec![e]);
        p.set("MixColors", ParamValue::Bool(false));
    });
    let lowest = |frame: usize| f.lit_pixels(frame).iter().map(|p| p.1).min().unwrap();
    assert!(lowest(30) < lowest(0), "{}", f.picture(30));
}

#[test]
fn a_bottom_barrier_catches_what_would_fall_out() {
    // Release one particle, then stop. Continuous emission on a small grid can
    // push caught particles out sideways, and occupied pixels are not particle
    // counts. Comparing two independently random final frames was flaky.
    let grid = Grid::new(30, 30);
    let targets = [grid.root];
    let cancel = AtomicBool::new(false);
    let c = ctx(&grid, &targets, &[], None, &cancel, 5000);
    let run = |barrier: bool, seed: u64| {
        let mut e = still_emitter(50.0, 15.0);
        e.set("Flow", flat(0.5)); // 0.5% of 200 = one particle.
        e.set("FlowControl", enum_value("Pulsating"));
        e.set("OnTime", ParamValue::Int(100)); // Only the first 50 ms frame is on.
        e.set("OffTime", ParamValue::Int(10000));
        let mut p = DESCRIPTOR.new_params();
        with_emitters(&mut p, vec![e]);
        p.set("BottomBarrier", ParamValue::Bool(barrier));
        grid.frames(&render_seeded(&c, &p, Rand::seeded(seed)))
    };
    for seed in 1..=16 {
        let caught = run(true, seed);
        let fallen = run(false, seed);
        assert_eq!(caught.lit_pixels(0), fallen.lit_pixels(0), "same release, seed {seed}");
        assert_eq!(caught.lit_pixels(0).len(), 1, "one initial particle, seed {seed}");
        assert_eq!(caught.lit_pixels(caught.frame_count - 1).len(), 1, "caught, seed {seed}");
        assert!(fallen.lit_pixels(fallen.frame_count - 1).is_empty(), "fallen, seed {seed}");
    }
}

#[test]
fn warm_up_frames_run_the_world_before_the_effect() {
    let grid = Grid::new(20, 20);
    let lit_below_emitter = |warm_up: i32| {
        let f = grid.render(&DESCRIPTOR, 100, |p| {
            with_emitters(p, vec![still_emitter(50.0, 90.0)]);
            p.set("WarmUpFrames", ParamValue::Int(warm_up));
        });
        f.lit_pixels(0).iter().filter(|p| p.1 < 15).count()
    };
    assert_eq!(lit_below_emitter(0), 0);
    assert!(lit_below_emitter(40) > 0);
}

#[test]
fn a_render_repeats_from_its_seed() {
    let grid = Grid::new(10, 10);
    let targets = [grid.root];
    let cancel = AtomicBool::new(false);
    let c = ctx(&grid, &targets, &[], None, &cancel, 1000);
    let p = DESCRIPTOR.new_params();
    let frames = |seed: u64| {
        let out = render_seeded(&c, &p, Rand::seeded(seed));
        let f = grid.frames(&out);
        (0..f.frame_count).flat_map(|frame| f.lit_pixels(frame)).collect::<Vec<_>>()
    };
    assert_eq!(frames(11), frames(11));
    assert_ne!(frames(11), frames(12));
}

#[test]
fn renders_run_in_parallel() {
    let handles: Vec<_> = (0..6)
        .map(|_| {
            std::thread::spawn(|| {
                let grid = Grid::new(10, 10);
                !grid.render(&DESCRIPTOR, 500, |_| {}).is_dark()
            })
        })
        .collect();
    for h in handles {
        assert!(h.join().unwrap());
    }
}

#[test]
fn horizontal_and_vertical_strings_swap_the_buffer() {
    // Vertical: 8 strings of 25 is 8 wide and 25 high, so a Y of 96% lands near the top of a string.
    let mut tree = ElementTree::new();
    let root = testing::grid(&mut tree, "G", 8, 25);
    let out = testing::render_with(&DESCRIPTOR, &tree, &[root], 100, |p| {
        p.set("StringOrientation", enum_value("Vertical"));
        with_emitters(p, vec![still_emitter(50.0, 96.0)]);
    });
    let elements = tree.distinct_elements(root);
    let lit = |string: usize, pixel: usize| match &out.nodes_for(&elements[string * 25 + pixel]).unwrap()[0].intent.data {
        IntentData::ArrayRgb(a) => a.values()[0] != RgbValue::BLACK,
        other => panic!("{other:?}"),
    };
    // X 50% of 8 is 4, Y 96% of 25 is 24.
    assert!((3..=5).any(|s| (23..25).any(|px| lit(s, px))));
    assert!(!lit(0, 0));
}

// ---- despeckle

#[test]
fn a_surrounded_speck_takes_the_average() {
    let mut buf = PixelFrameBuffer::new(3, 3);
    buf.fill(Color::rgb(100, 0, 200));
    buf.set_pixel(1, 1, Color::TRANSPARENT);
    assert_eq!(despeckle_color(&buf, 1, 1, 1, 3, 3), Color::rgb(100, 0, 200));
}

#[test]
fn enough_empty_neighbours_leave_it_empty() {
    let mut buf = PixelFrameBuffer::new(3, 3);
    buf.fill(Color::RED);
    buf.set_pixel(1, 1, Color::TRANSPARENT);
    buf.set_pixel(0, 0, Color::TRANSPARENT);
    assert_eq!(despeckle_color(&buf, 1, 1, 1, 3, 3), Color::TRANSPARENT);
}

#[test]
fn an_empty_neighbour_under_the_threshold_averages_as_white() {
    // Deliberate C#: Color.Transparent's components are 255. Seven reds and one transparent neighbour.
    let mut buf = PixelFrameBuffer::new(3, 3);
    buf.fill(Color::RED);
    buf.set_pixel(1, 1, Color::TRANSPARENT);
    buf.set_pixel(0, 0, Color::TRANSPARENT);
    assert_eq!(despeckle_color(&buf, 1, 1, 2, 3, 3), Color::rgb(255, 31, 31));
}

#[test]
fn despeckling_fills_holes_in_the_output() {
    let grid = Grid::new(8, 8);
    let count = |threshold: i32| {
        let mut e = still_emitter(50.0, 50.0);
        e.set("Flow", flat(100.0));
        let f = grid.render(&DESCRIPTOR, 500, |p| {
            with_emitters(p, vec![e]);
            p.set("DespeckleThreshold", ParamValue::Int(threshold));
            p.set("BottomBarrier", ParamValue::Bool(true));
        });
        f.lit_pixels(f.frame_count - 1).len()
    };
    assert!(count(4) >= count(0));
}

// ---- the emitter state machines, stepped through ConvertEmittersToEmitterWrappers

/// A `Liquid` set up on an 8 x 25 display for one emitter.
struct Rig<'a> {
    e: Liquid<'a>,
    info: PixelInfo,
}

fn rig<'a>(c: &RenderCtx<'a>, emitters: Vec<Params>, edit: impl FnOnce(&mut Params)) -> Rig<'a> {
    let mut p = DESCRIPTOR.new_params();
    with_emitters(&mut p, emitters);
    edit(&mut p);
    let mut e = Liquid::new(c, &p, Rand::seeded(1));
    let info = info(8, 25, c.time_span.ticks() / vixen_core::time::TICKS_PER_MS);
    e.setup_render(&info);
    Rig { e, info }
}

impl Rig<'_> {
    fn step(&mut self, frame: i32) -> Vec<LfEmitter> {
        let pos = self.info.interval_position(frame);
        self.e.convert_emitters(frame, pos, pos * 100.0, &self.info);
        self.e.wrappers.clone()
    }

    fn flows(&mut self, frames: i32) -> Vec<i32> {
        (0..frames).map(|f| self.step(f)[0].flow).collect()
    }
}

#[test]
fn continuous_flow_scales_the_curve_to_200() {
    let grid = Grid::new(8, 25);
    let targets = [grid.root];
    let cancel = AtomicBool::new(false);
    let c = ctx(&grid, &targets, &[], None, &cancel, 1000);
    let mut rig = rig(&c, vec![still_emitter(50.0, 50.0)], |_| {});
    // The default flow curve is 10 of 100, so 20.
    assert_eq!(rig.flows(2), [20, 20]);
    let w = rig.step(2)[0];
    assert_eq!(w.lifetime, 1000);
    assert_eq!(w.velocity, 0);
    assert_eq!(w.source_size, 0);
    assert_eq!(w.particle_type, 6);
    assert_eq!((w.x, w.y), (12, 4), "12.5 rounds to the even 12");
    // 75 of 100 of 360 degrees.
    assert_eq!(w.angle, 270);
}

#[test]
fn pulsating_flow_times_on_and_off_in_frames() {
    let grid = Grid::new(8, 25);
    let targets = [grid.root];
    let cancel = AtomicBool::new(false);
    let c = ctx(&grid, &targets, &[], None, &cancel, 1000);
    let mut e = still_emitter(50.0, 50.0);
    e.set("FlowControl", enum_value("Pulsating"));
    e.set("OnTime", ParamValue::Int(100));
    e.set("OffTime", ParamValue::Int(150));
    let mut rig = rig(&c, vec![e], |_| {});
    // It starts on, and the on timer reaches 100 ms on the second frame, which is already off. The off timer reaches
    // 150 ms three frames later, which is already on. So on lasts two frames, except the first time.
    assert_eq!(rig.flows(10), [20, 0, 0, 0, 20, 20, 0, 0, 0, 20]);
}

fn marks_at(name: &str, ranges: &[(i64, i64)]) -> MarkCollection {
    let mut m = MarkCollection { name: name.to_string(), ..MarkCollection::default() };
    for (start, duration) in ranges {
        let mut mark = Mark::new(TimeSpan::from_ms_i(*start));
        mark.duration = TimeSpan::from_ms_i(*duration);
        m.add_mark(mark);
    }
    m
}

fn mark_emitter(id: Uuid) -> Params {
    let mut e = still_emitter(50.0, 50.0);
    e.set("FlowControl", enum_value("UseMarks"));
    e.set("MarkCollectionId", ParamValue::Guid(id));
    e
}

#[test]
fn mark_flow_is_on_during_marks() {
    let grid = Grid::new(8, 25);
    let targets = [grid.root];
    let cancel = AtomicBool::new(false);
    let marks = [marks_at("Beats", &[(100, 50), (300, 100)])];
    let c = ctx(&grid, &targets, &marks, None, &cancel, 500);
    let mut rig = rig(&c, vec![mark_emitter(marks[0].id)], |_| {});
    // Inclusive at both ends: frame 2 (100 ms) to frame 3 (150 ms), and frame 6 (300 ms) to frame 8 (400 ms).
    assert_eq!(rig.flows(10), [0, 0, 20, 20, 0, 0, 20, 20, 20, 0]);
}

#[test]
fn mark_flow_without_a_selection_takes_the_first_collection() {
    // Upstream's Emitter_UseMarks_SelectsTheFirstCollectionWhenNoneIsSelected.
    let first = marks_at("First", &[(0, 50)]);
    let second = marks_at("Second", &[(200, 50)]);
    let collections = [first, second];
    assert_eq!(selected_collection(Uuid::nil(), &collections).map(|m| m.id), Some(collections[0].id));
    assert_eq!(selected_collection(collections[1].id, &collections).map(|m| m.id), Some(collections[1].id));
    assert_eq!(selected_collection(Uuid::new_v4(), &collections).map(|m| m.id), Some(collections[0].id), "an unknown id is not valid");
    assert!(selected_collection(Uuid::nil(), &[]).is_none());
}

#[test]
fn mark_flow_with_no_collection_keeps_flowing() {
    // Deliberate C#: with no name, the branch that turns the emitter off never runs, and it starts on.
    let grid = Grid::new(8, 25);
    let targets = [grid.root];
    let cancel = AtomicBool::new(false);
    let c = ctx(&grid, &targets, &[], None, &cancel, 200);
    let mut rig = rig(&c, vec![mark_emitter(Uuid::nil())], |_| {});
    assert_eq!(rig.flows(3), [20, 20, 20]);

    let unnamed = [marks_at("", &[(500, 50)])];
    let c = ctx(&grid, &targets, &unnamed, None, &cancel, 200);
    let mut rig2 = super::tests::rig(&c, vec![mark_emitter(unnamed[0].id)], |_| {});
    assert_eq!(rig2.flows(3), [20, 20, 20]);
}

#[test]
fn musical_flow_follows_the_volume() {
    let grid = Grid::new(8, 25);
    let targets = [grid.root];
    let cancel = AtomicBool::new(false);
    // Half a second of silence, then full scale.
    let rate = 8000;
    let samples: Vec<f32> = (0..rate).map(|i| if i < rate / 2 { 0.0 } else { 1.0 }).collect();
    let audio = FakeAudio { samples, rate: rate as u32 };
    let c = ctx(&grid, &targets, &[], Some(&audio), &cancel, 1000);
    let mut e = still_emitter(50.0, 50.0);
    e.set("FlowControl", enum_value("Musical"));
    let mut rig = rig(&c, vec![e], |p| {
        p.set("AttackTime", ParamValue::Int(0));
    });
    let flows = rig.flows(20);
    // `_frameVolumes[frame]` is the volume at (frame + 1) * 50 ms, scaled between the effect's quietest and loudest.
    assert_eq!(flows[0], 0, "silence");
    assert_eq!(flows[8], 0, "the last silent one");
    assert_eq!(flows[10], 100, "full scale at the maximum flow");
    assert_eq!(flows[18], 100);
}

#[test]
fn musical_flow_without_audio_is_zero() {
    let grid = Grid::new(8, 25);
    let targets = [grid.root];
    let cancel = AtomicBool::new(false);
    let c = ctx(&grid, &targets, &[], None, &cancel, 500);
    let mut e = still_emitter(50.0, 50.0);
    e.set("FlowControl", enum_value("Musical"));
    let mut rig = rig(&c, vec![e], |_| {});
    assert_eq!(rig.flows(3), [0, 0, 0]);
}

#[test]
fn gain_is_the_stored_value_times_ten() {
    let grid = Grid::new(8, 25);
    let targets = [grid.root];
    let cancel = AtomicBool::new(false);
    let c = ctx(&grid, &targets, &[], None, &cancel, 200);
    let rig = rig(&c, vec![still_emitter(50.0, 50.0)], |p| {
        p.set("Gain", ParamValue::Int(3));
    });
    assert_eq!(rig.e.audio_settings.gain, 30.0);
}

#[test]
fn the_colour_list_cycles_by_frame() {
    let grid = Grid::new(8, 25);
    let targets = [grid.root];
    let cancel = AtomicBool::new(false);
    let c = ctx(&grid, &targets, &[], None, &cancel, 1000);
    let mut e = still_emitter(50.0, 50.0);
    e.set("UseColorArray", ParamValue::Bool(true));
    e.set("FramesPerColor", ParamValue::Int(2));
    let mut rig = rig(&c, vec![e], |p| {
        p.set("Colors", ParamValue::GradientList(vec![ColorGradient::solid(Color::RED), ColorGradient::solid(Color::BLUE)]));
    });
    let reds: Vec<bool> = (0..6).map(|f| rig.step(f)[0].r == 255).collect();
    assert_eq!(reds, [true, true, false, false, true, true]);
}

#[test]
fn an_empty_colour_list_stays_dark() {
    let grid = Grid::new(4, 4);
    let mut e = still_emitter(50.0, 50.0);
    e.set("UseColorArray", ParamValue::Bool(true));
    let f = grid.render(&DESCRIPTOR, 200, |p| {
        with_emitters(p, vec![e]);
        p.set("Colors", ParamValue::GradientList(Vec::new()));
    });
    assert!(f.is_empty());
}

#[test]
fn oscillating_nozzles_turn_around_at_their_limits() {
    let grid = Grid::new(8, 25);
    let targets = [grid.root];
    let cancel = AtomicBool::new(false);
    let c = ctx(&grid, &targets, &[], None, &cancel, 1000);
    let mut e = still_emitter(50.0, 50.0);
    e.set("NozzleMovement", enum_value("Oscillate"));
    e.set("OscillateStartAngle", ParamValue::Int(0));
    e.set("OscillateEndAngle", ParamValue::Int(8));
    let mut rig = rig(&c, vec![e], |_| {});
    // The angle starts at 5 going clockwise. The speed curve's 5 of 100 of 45 degrees is 2.25, rounded to 2.
    let angles: Vec<i32> = (0..10).map(|f| rig.step(f)[0].angle).collect();
    assert_eq!(angles, [3, 1, -1, 1, 3, 5, 7, 9, 7, 5]);
}

#[test]
fn spinning_nozzles_keep_turning() {
    let grid = Grid::new(8, 25);
    let targets = [grid.root];
    let cancel = AtomicBool::new(false);
    let c = ctx(&grid, &targets, &[], None, &cancel, 1000);
    let spin = |name: &str| {
        let mut e = still_emitter(50.0, 50.0);
        e.set("NozzleMovement", enum_value(name));
        e.set("OscillationSpeed", flat(100.0));
        e
    };
    let mut clockwise = rig(&c, vec![spin("SpinClockwise")], |_| {});
    assert_eq!((0..3).map(|f| clockwise.step(f)[0].angle).collect::<Vec<_>>(), [-40, -85, -130]);
    let mut counter = super::tests::rig(&c, vec![spin("SpinCounterClockwise")], |_| {});
    assert_eq!((0..3).map(|f| counter.step(f)[0].angle).collect::<Vec<_>>(), [50, 95, 140]);
}

fn animated(x_start: i32, y_start: i32) -> Params {
    let mut e = Params::defaults(&gen::EMITTER_PARAMS);
    e.set("Animate", ParamValue::Bool(true));
    e.set("RandomStartingPosition", ParamValue::Bool(false));
    e.set("AnimateXStart", ParamValue::Int(x_start));
    e.set("AnimateYStart", ParamValue::Int(y_start));
    e
}

#[test]
fn an_animated_emitter_moves_by_its_velocity_and_aims_behind_itself() {
    let grid = Grid::new(8, 25);
    let targets = [grid.root];
    let cancel = AtomicBool::new(false);
    let c = ctx(&grid, &targets, &[], None, &cancel, 1000);
    // The start is 40% of 25 and 25% of 8: (10, 2). The velocity curves' 10 of 100 of 10 is 1 per pass.
    let mut rig = rig(&c, vec![animated(40, 25)], |_| {});
    assert_eq!((rig.e.emitters[0].location_x, rig.e.emitters[0].location_y), (10.0, 2.0));
    let w = rig.step(0)[0];
    assert_eq!((w.x, w.y), (11, 3));
    // Moving up and right at 45 degrees, and the fixed nozzle follows the movement.
    assert_eq!(w.angle, 45);
}

#[test]
fn two_animated_emitters_each_move_twice_a_frame() {
    // Deliberate C#: AnimateEmitters runs for every emitter, and moves every emitter.
    let grid = Grid::new(8, 25);
    let targets = [grid.root];
    let cancel = AtomicBool::new(false);
    let c = ctx(&grid, &targets, &[], None, &cancel, 1000);
    let mut rig = rig(&c, vec![animated(0, 0), animated(40, 0)], |_| {});
    // Still in the first emitter's update, the first has moved once. By the end of the frame both moved twice.
    let w = rig.step(0);
    assert_eq!(w[0].x, 1);
    assert_eq!(w[1].x, 12);
    assert_eq!(rig.e.emitters[0].location_x, 2.0);
    assert_eq!(rig.e.emitters[1].location_x, 12.0);
}

#[test]
fn bounce_turns_an_emitter_around_at_the_edge() {
    let grid = Grid::new(8, 25);
    let targets = [grid.root];
    let cancel = AtomicBool::new(false);
    let c = ctx(&grid, &targets, &[], None, &cancel, 1000);
    let mut rig = rig(&c, vec![animated(92, 0)], |_| {});
    // From x 23: 24, then 25 is off the matrix, so it sits at 24 going left.
    assert_eq!(rig.step(0)[0].x, 24);
    assert_eq!(rig.step(1)[0].x, 24);
    assert!(rig.e.emitters[0].vel_x < 0.0);
    assert_eq!(rig.step(2)[0].x, 23);
}

#[test]
fn wrap_carries_an_emitter_around() {
    let grid = Grid::new(8, 25);
    let targets = [grid.root];
    let cancel = AtomicBool::new(false);
    let c = ctx(&grid, &targets, &[], None, &cancel, 1000);
    let mut e = animated(92, 0);
    e.set("EdgeHandling", enum_value("Wrap"));
    let mut rig = rig(&c, vec![e], |_| {});
    // 23 up to 25 stays. 26 is past the width, so it wraps by the width.
    assert_eq!((0..4).map(|f| rig.step(f)[0].x).collect::<Vec<_>>(), [24, 25, 1, 2]);
}

#[test]
fn emitters_that_meet_turn_each_other_around() {
    let grid = Grid::new(8, 25);
    let targets = [grid.root];
    let cancel = AtomicBool::new(false);
    let c = ctx(&grid, &targets, &[], None, &cancel, 1000);
    let mut a = animated(40, 25);
    let mut b = animated(44, 25);
    a.set("SourceSize", flat(1.0));
    b.set("SourceSize", flat(1.0));
    // Sources of 1 and 1 touch within 2. They start 1 apart, so the first pass already flips both.
    let mut rig = rig(&c, vec![a, b], |_| {});
    rig.step(0);
    assert!(rig.e.emitters[0].vel_x != 1.0 || rig.e.emitters[1].vel_x != 1.0);
}

#[test]
fn a_random_start_is_inside_the_display() {
    let grid = Grid::new(8, 25);
    let targets = [grid.root];
    let cancel = AtomicBool::new(false);
    let c = ctx(&grid, &targets, &[], None, &cancel, 1000);
    let mut e = animated(0, 0);
    e.set("RandomStartingPosition", ParamValue::Bool(true));
    let rig = rig(&c, vec![e], |_| {});
    let (x, y) = (rig.e.emitters[0].location_x, rig.e.emitters[0].location_y);
    assert!((0.0..25.0).contains(&x) && (0.0..8.0).contains(&y));
}

// ---- locations

fn located_grid() -> (ElementTree, NodeId) {
    let mut tree = ElementTree::new();
    let root = testing::grid(&mut tree, "G", 4, 10);
    testing::add_locations(&mut tree, root);
    (tree, root)
}

fn lit_at(out: &EffectIntents, tree: &ElementTree, root: NodeId, frame: usize, x: i32, y: i32) -> bool {
    for leaf in tree.distinct_leaves(root) {
        if tree.location(leaf) == (x, y) {
            let id = tree.node(leaf).element_id.unwrap();
            return match &out.nodes_for(&id).unwrap()[0].intent.data {
                IntentData::ArrayRgb(a) => a.values()[frame] != RgbValue::BLACK,
                other => panic!("{other:?}"),
            };
        }
    }
    panic!("nothing at ({x},{y})");
}

#[test]
fn location_mode_flips_and_offsets_the_particles() {
    // Locations are x 0..=36 and y 0..=12 in steps of 4. With a scale factor of 1 the render area is 37 x 13. An emitter
    // at the lower left of the render area lights the location with the smallest x and the largest y.
    let (tree, root) = located_grid();
    let mut e = still_emitter(0.0, 92.4);
    e.set("Flow", flat(100.0));
    let out = testing::render_with(&DESCRIPTOR, &tree, &[root], 100, |p| {
        with_emitters(p, vec![e]);
        p.set("TargetPositioning", enum_value("Locations"));
        p.set("RenderScaleFactor", ParamValue::Int(1));
    });
    // Y 92.4% of 13 is 12 in render terms, and 12 flips to location y 0.
    assert!(lit_at(&out, &tree, root, 0, 0, 0));
    assert!(!lit_at(&out, &tree, root, 0, 0, 12));
    assert!(!lit_at(&out, &tree, root, 0, 36, 0));
}

#[test]
fn the_render_scale_factor_shrinks_the_world_and_widens_each_particle() {
    let (tree, root) = located_grid();
    let targets = [root];
    let cancel = AtomicBool::new(false);
    let ctx = RenderCtx { video_cache: None,
        tree: &tree,
        targets: &targets,
        start_time: TimeSpan::ZERO,
        time_span: TimeSpan::from_ms_i(100),
        frame_ms: 50,
        marks: &[],
        audio: None,
        module_data_root: None,
        cancel: &cancel,
    };
    let mut p = DESCRIPTOR.new_params();
    with_emitters(&mut p, vec![still_emitter(0.0, 50.0)]);
    p.set("TargetPositioning", enum_value("Locations"));
    p.set("RenderScaleFactor", ParamValue::Int(4));
    let mut e = Liquid::new(&ctx, &p, Rand::seeded(1));
    let out = render_pixel(&ctx, StringOrientation::Vertical, TargetPositioning::Locations, &mut e);
    // 37 / 4 is 9, plus 1 for the remainder. 13 / 4 is 3, plus 1.
    assert_eq!((e.buffer_wt, e.buffer_ht), (10, 4));
    // Y 50% of 4 is 2 in render terms, and within half a unit of that rounds to 2 either way. Location y flips to 12 - y
    // and divides by 4, so that bitmap row is location y 4, on a grid with a step of 4.
    assert!(lit_at(&out, &tree, root, 0, 0, 4));
    assert!(!lit_at(&out, &tree, root, 0, 0, 0));
    assert!(!lit_at(&out, &tree, root, 0, 0, 8));
    assert!(!lit_at(&out, &tree, root, 0, 4, 4));
}

#[test]
fn a_scale_factor_of_zero_stays_dark() {
    let (tree, root) = located_grid();
    let out = testing::render_with(&DESCRIPTOR, &tree, &[root], 100, |p| {
        p.set("TargetPositioning", enum_value("Locations"));
        p.set("RenderScaleFactor", ParamValue::Int(0));
    });
    assert!(out.is_empty());
}

#[test]
fn location_mode_renders_without_panicking_on_a_small_target() {
    let mut tree = ElementTree::new();
    let root = testing::grid(&mut tree, "G", 1, 2);
    testing::add_locations(&mut tree, root);
    let out = testing::render_with(&DESCRIPTOR, &tree, &[root], 200, |p| {
        p.set("TargetPositioning", enum_value("Locations"));
    });
    assert!(!out.is_empty());
}
