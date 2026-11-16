//! Moving-head live state, independent of the playback and GPU threads. One clock per preview services
//! all shared strobe timers and finishing pan/tilt travel; a stationary dark preview sleeps indefinitely.

use parking_lot::Mutex;
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use vixen_core::{CombinedState, PropertyData};
use vixen_engine::schedule::ElementMap;
use vixen_io::SystemData;
use vixen_output::filters::FilterConfig;
use vixen_preview::fixture_state::{ColorWheel, FixtureSet, FixtureSetup, WheelEntry};
use vixen_preview::shape::{MountingPosition, ShapeKind};
use vixen_preview::PreviewData;
use vixen_render::{rgba, BeamWidthModel, LiveRenderer, MovingHeadInstance, MovingHeadParams};

#[derive(Clone)]
struct Source {
    index: usize,
    setup: FixtureSetup,
    instance: MovingHeadInstance,
    legend: bool,
}

#[derive(Clone, Default)]
pub struct FixturePreparation {
    sources: Vec<Source>,
    interval_ms: u64,
}

impl FixturePreparation {
    pub fn new(data: &PreviewData, system: &SystemData, elements: &ElementMap) -> Self {
        if !data.use_open_gl {
            return Self::default();
        }
        let mut sources = Vec::new();
        for item in &data.display_items {
            let ShapeKind::MovingHead(d) = item.shape.kind() else {
                continue;
            };
            let node = system.tree.find(&d.node_id);
            let element = node.and_then(|n| system.tree.node(n).element_id);
            let mut setup = FixtureSetup::from_shape(d, system.default_update_interval.max(1));
            if let Some(element) = element {
                // Only the immediate destinations are inspected. A deeper filter is not automation for this head.
                let mut patches: Vec<_> = system
                    .patches
                    .iter()
                    .filter(|p| p.source_id == Some(element))
                    .collect();
                patches.sort_by_key(|p| p.output);
                for patch in patches {
                    let Some(filter) = system
                        .filters
                        .iter()
                        .find(|f| f.instance_id == patch.component_id)
                    else {
                        continue;
                    };
                    let Some(data) = system
                        .module_store
                        .instance(filter.type_id, filter.instance_id)
                        .and_then(|m| m.data())
                    else {
                        continue;
                    };
                    match FilterConfig::from_xml(&filter.type_id, data) {
                        Some(FilterConfig::Shutter(c)) => {
                            setup.convert_color_into_shutter = c.convert_color_into_shutter_intents
                        }
                        Some(FilterConfig::Dimming(c)) => {
                            setup.convert_color_into_dimmer = c.convert_rgb_into_dimming_intents
                        }
                        _ => {}
                    }
                }
            }
            if let Some(PropertyData::Unknown { raw_xml }) = node
                .and_then(|n| {
                    system
                        .tree
                        .property(n, &fx_fixture::spec::INTELLIGENT_FIXTURE_TYPE_ID)
                })
                .map(|p| &p.data)
            {
                if let Some(profile) = fx_fixture::spec::parse_specification(raw_xml) {
                    setup.color_wheels = profile
                        .functions
                        .iter()
                        .filter(|f| {
                            f.function_type == fx_fixture::spec::FixtureFunctionType::ColorWheel
                        })
                        .map(|f| ColorWheel {
                            function: f.name.clone(),
                            entries: f
                                .color_wheel_data
                                .iter()
                                .map(|i| WheelEntry {
                                    start_value: i.start_value,
                                    use_curve: i.use_curve,
                                    color1: i.color1,
                                    color2: i.color2,
                                })
                                .collect(),
                        })
                        .collect();
                }
            }
            let mounted_top = d.mounting_position == MountingPosition::Top;
            sources.push(Source {
                index: element
                    .and_then(|e| elements.index_of(&e))
                    .map_or(usize::MAX, |n| n as usize),
                setup,
                instance: MovingHeadInstance {
                    position: [
                        ((item.shape.left() + item.shape.right()) as f32 / 2.0).trunc(),
                        ((item.shape.top() + item.shape.bottom()) as f32 / 2.0).trunc(),
                    ],
                    length: (item.shape.right() - item.shape.left())
                        .min(item.shape.bottom() - item.shape.top())
                        as f32,
                    params: MovingHeadParams {
                        mounted_top,
                        tilt_offset_deg: if mounted_top { 0.0 } else { 180.0 },
                        max_beam_length: -1.0,
                        beam_alpha: (100 - d.beam_transparency) as f32 / 100.0,
                        beam_width: BeamWidthModel::PreviewMultiplier(
                            d.beam_width_multiplier as f32,
                        ),
                        ..Default::default()
                    },
                },
                legend: d.show_legend,
            });
        }
        Self {
            sources,
            interval_ms: system.default_update_interval.max(1) as u64,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FixtureLegend {
    pub text: String,
    pub position: [f32; 3],
}

enum Command {
    Frame(Vec<CombinedState>),
    Quit,
}

pub struct FixturePlayer {
    sources: Vec<usize>,
    sender: mpsc::Sender<Command>,
    legends: Arc<Mutex<Vec<FixtureLegend>>>,
    thread: Option<JoinHandle<()>>,
}

impl FixturePlayer {
    pub fn new(prepared: FixturePreparation, renderer: Arc<LiveRenderer>) -> Option<Self> {
        if prepared.sources.is_empty() {
            renderer.set_moving_heads(Vec::new());
            return None;
        }
        let sources = prepared.sources.iter().map(|s| s.index).collect();
        let legends = Arc::new(Mutex::new(Vec::new()));
        let output = legends.clone();
        let (sender, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("vixen-fixtures".into())
            .spawn(move || run(prepared, renderer, receiver, output))
            .ok()?;
        Some(Self {
            sources,
            sender,
            legends,
            thread: Some(thread),
        })
    }
    pub fn frame(&self, states: &[CombinedState]) {
        let _ = self.sender.send(Command::Frame(
            self.sources
                .iter()
                .map(|i| states.get(*i).cloned().unwrap_or_default())
                .collect(),
        ));
    }
    pub fn legends(&self) -> Vec<FixtureLegend> {
        self.legends.lock().clone()
    }
}

impl Drop for FixturePlayer {
    fn drop(&mut self) {
        let _ = self.sender.send(Command::Quit);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn run(
    prepared: FixturePreparation,
    renderer: Arc<LiveRenderer>,
    receiver: mpsc::Receiver<Command>,
    legends: Arc<Mutex<Vec<FixtureLegend>>>,
) {
    let epoch = Instant::now();
    let mut set = FixtureSet::new(prepared.sources.iter().map(|s| s.setup.clone()).collect());
    let mut current = vec![CombinedState::default(); set.len()];
    let interval = prepared.interval_ms as f64;
    let mut travel_at = interval;
    for i in 0..set.len() {
        set.frame(i, None, true, 0.0);
    }
    publish(&set, &prepared.sources, &renderer, &legends);
    loop {
        let now = epoch.elapsed().as_secs_f64() * 1000.0;
        let deadline = set
            .next_deadline()
            .into_iter()
            .chain(set.any_moving().then_some(travel_at))
            .min_by(f64::total_cmp);
        let command = match deadline {
            Some(at) => match receiver
                .recv_timeout(Duration::from_secs_f64(((at - now).max(0.0)) / 1000.0))
            {
                Ok(c) => Some(c),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(_) => break,
            },
            None => match receiver.recv() {
                Ok(c) => Some(c),
                Err(_) => break,
            },
        };
        let now = epoch.elapsed().as_secs_f64() * 1000.0;
        let mut standard = false;
        match command {
            Some(Command::Quit) => break,
            Some(Command::Frame(states)) => {
                current = states;
                standard = true;
            }
            None => {}
        }
        // Coalesce pending engine frames, keeping the newest state without delaying playback.
        while let Ok(command) = receiver.try_recv() {
            match command {
                Command::Quit => return,
                Command::Frame(states) => {
                    current = states;
                    standard = true;
                }
            }
        }
        standard |= set.any_moving() && now >= travel_at;
        if standard {
            for i in 0..set.len() {
                set.frame(i, current.get(i), true, now);
            }
            travel_at = now + interval;
        }
        let strobe = set.tick(now);
        if strobe && !standard {
            for i in 0..set.len() {
                set.frame(i, current.get(i), false, now);
            }
        }
        if standard || strobe {
            publish(&set, &prepared.sources, &renderer, &legends);
        }
    }
}

fn publish(
    set: &FixtureSet,
    sources: &[Source],
    renderer: &LiveRenderer,
    legends: &Mutex<Vec<FixtureLegend>>,
) {
    let mut texts = Vec::new();
    let heads = sources
        .iter()
        .enumerate()
        .map(|(i, source)| {
            let state = set.state(i);
            let mut head = source.instance.clone();
            let p = &mut head.params;
            p.pan_deg = state.pan_angle as f32;
            p.tilt_deg = state.tilt_angle as f32;
            p.beam_length_pct = state.beam_length as f32;
            p.focus = state.focus as f32;
            p.intensity = state.intensity as f32;
            p.on = state.on_off;
            let c = state.beam_color_left;
            p.color_left = rgba(c.r, c.g, c.b, c.a);
            let c = state.beam_color_right;
            p.color_right = rgba(c.r, c.g, c.b, c.a);
            if source.legend && !state.legend.is_empty() {
                texts.push(FixtureLegend {
                    text: state.legend.clone(),
                    position: [
                        head.position[0] - 0.5625 * head.length,
                        head.position[1] + 0.5 * head.length,
                        -0.45 * head.length,
                    ],
                });
            }
            head
        })
        .collect();
    *legends.lock() = texts;
    renderer.set_moving_heads(heads);
}

#[cfg(test)]
mod tests {
    use super::*;
    use fx_fixture::spec::{FixtureFunction,FixtureFunctionType,FixtureIndexItem,FixtureSpecification,FuncId};
    use vixen_core::{Color,CommandValue,FixtureIndexType,FunctionIdentity,LightingValue,NamedCommand,RangeValue,StateValue};
    use vixen_io::{ModuleEntry,OutputFilter,Patch};
    use vixen_output::{ids,ModuleIds};
    use vixen_output::filters::{DimmingFilterConfig,ShutterFilterConfig};
    use vixen_preview::{DisplayItem,Shape,ShapeId};
    use vixen_preview::point::PreviewPoint;
    use vixen_preview::shape::Sel;
    use uuid::Uuid;

    #[test]
    fn fixture_preparation_resolves_profiles_and_only_direct_automation_filters() {
        let mut system=SystemData::default();
        let node=system.tree.add_element("Head");
        let element=system.tree.node(node).element_id.unwrap();
        let mut spec=FixtureSpecification::default();
        spec.functions.push(FixtureFunction{name:"Wheel".into(),function_type:FixtureFunctionType::ColorWheel,
            color_wheel_data:vec![FixtureIndexItem{start_value:42,color1:Color::RED,color2:Color::BLUE,..FixtureIndexItem::color_wheel()}],..Default::default()});
        spec.functions.push(FixtureFunction{identity:FuncId::Pan,rotation_limits:fx_fixture::spec::FixtureRotationLimits{start_position:-90,stop_position:450},..Default::default()});
        spec.functions.push(FixtureFunction{identity:FuncId::Tilt,rotation_limits:fx_fixture::spec::FixtureRotationLimits{start_position:10,stop_position:270},..Default::default()});
        spec.functions.push(FixtureFunction{identity:FuncId::Zoom,..Default::default()});
        fx_fixture::fixture_base::test_support::add_fixture_property(&mut system.tree,node,&spec);
        let direct=Uuid::new_v4();
        let configs=[
            FilterConfig::Shutter(ShutterFilterConfig{ids:ModuleIds::new(ids::SHUTTER_FILTER,direct),convert_color_into_shutter_intents:true,..Default::default()}),
            FilterConfig::Dimming(DimmingFilterConfig{ids:ModuleIds::new(ids::DIMMING_FILTER,Uuid::new_v4()),convert_rgb_into_dimming_intents:true,..Default::default()}),
            FilterConfig::Shutter(ShutterFilterConfig{ids:ModuleIds::new(ids::SHUTTER_FILTER,Uuid::new_v4()),convert_color_into_shutter_intents:false,..Default::default()}),
        ];
        for (i,c) in configs.into_iter().enumerate() {
            let m=c.ids();
            system.filters.push(OutputFilter{type_id:m.type_id,instance_id:m.instance_id});
            system.module_store.set_instance(ModuleEntry::new("Test",&m.type_id,&m.instance_id,"Test",c.to_xml()));
            system.patches.push(Patch{component_id:m.instance_id,source_id:Some(if i==2 {direct}else{element}),output:0});
        }
        let shape=Shape::new_moving_head(PreviewPoint::new(10,10),Some(Sel::new(&system.tree,node)),1.0);
        let mut data=PreviewData::new();data.use_open_gl=true;
        data.display_items.push(DisplayItem::new(ShapeId(1),shape));
        let prepared=FixturePreparation::new(&data,&system,&ElementMap::from_tree(&system.tree));
        let source=&prepared.sources[0];
        assert_eq!(source.index,0);
        assert_eq!((source.setup.pan_start,source.setup.pan_stop,source.setup.tilt_start,source.setup.tilt_stop),(-90,450,10,270));
        assert!(source.setup.zoom_narrow_to_wide);
        assert!(source.setup.convert_color_into_shutter && source.setup.convert_color_into_dimmer);
        assert_eq!(source.setup.color_wheels[0].entries[0].color2,Color::BLUE);
        assert_eq!(source.setup.color_wheels[0].entries[0].start_value,42);
        data.use_open_gl=false;
        assert!(FixturePreparation::new(&data,&system,&ElementMap::from_tree(&system.tree)).sources.is_empty());
    }

    #[test]
    fn fixture_worker_strobes_without_engine_frames_and_parks_when_cleared() {
        let renderer=match LiveRenderer::headless(200,200,1.0) {
            Ok(r)=>Arc::new(r),Err(vixen_render::RenderError::NoAdapter) if std::env::var_os("VIXEN_RENDER_REQUIRE_GPU").is_none()=>return,
            Err(e)=>panic!("{e}"),
        };
        renderer.set_camera_depth(241.42);
        let shape=Shape::new_moving_head(PreviewPoint::new(0,0),None,1.0);
        let mut setup=FixtureSetup::from_shape(shape.moving_head().unwrap(),20);
        setup.convert_color_into_shutter=true;setup.strobe_rate_minimum=20;setup.strobe_rate_maximum=20;
        setup.max_pan_travel_time=0.02;setup.max_tilt_travel_time=0.02;
        let prepared=FixturePreparation{interval_ms:20,sources:vec![Source{index:0,setup,legend:true,
            instance:MovingHeadInstance{position:[100.0,75.0],length:40.0,params:MovingHeadParams{max_beam_length:100.0,..Default::default()}}}]};
        let player=FixturePlayer::new(prepared,renderer.clone()).unwrap();
        let mut state=CombinedState{lighting:Some(LightingValue::new(Color::RED,1.0)),..Default::default()};
        state.other.push(StateValue::Range(RangeValue::new(FunctionIdentity::Pan,"Pan",0.5,"Pan ")));
        state.other.push(StateValue::Command(CommandValue::Named8(NamedCommand{value:100,index_type:FixtureIndexType::Strobe,
            tag:"Strobe".into(),label:"Strobe ".into(),range_minimum:0,range_maximum:255})));
        player.frame(&[state]);
        let deadline=Instant::now()+Duration::from_secs(2);
        while player.legends().is_empty()&&Instant::now()<deadline {thread::sleep(Duration::from_millis(5));}
        assert!(player.legends()[0].text.contains("Pan 127"));
        renderer.read_pixels();
        let first=renderer.stats().frames_rendered;
        thread::sleep(Duration::from_millis(220));
        renderer.read_pixels();
        assert!(renderer.stats().frames_rendered>=first+4,"strobe clock draws after just one engine frame");
        player.frame(&[]);
        while !player.legends().is_empty()&&Instant::now()<deadline {thread::sleep(Duration::from_millis(5));}
        thread::sleep(Duration::from_millis(100));
        renderer.read_pixels();
        let stopped=renderer.stats().frames_rendered;
        thread::sleep(Duration::from_millis(120));
        renderer.read_pixels();
        assert_eq!(renderer.stats().frames_rendered,stopped,"dark stationary fixtures park");
    }
}
