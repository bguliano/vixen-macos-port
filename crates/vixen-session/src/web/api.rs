//! Application behavior behind the upstream WebServer HTTP/SignalR interface.
use crate::app::AppCore;
use crate::logging;
use crate::playback::Transport;
use crate::sequence::SequenceSession;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use uuid::Uuid;
use vixen_core::color::Rgb;
use vixen_core::gradient::ColorPoint;
use vixen_core::{Color, ColorGradient, Curve, ElementTree, NodeId, TimeSpan};
use vixen_effects::framework::{EffectDescriptor, ParamKind, ParamValue, Params, RenderCtx};
use vixen_web::{Action, ApiError, ApiInput, ApiResponse, WebApi};

const LIVE: &str = "Web Server";
const LOGGER: &str = "VixenModules.App.WebServer.Service.SequenceHelper";

pub struct SessionApi {
    core: Weak<AppCore>,
    root_id: Uuid,
}
impl SessionApi {
    pub fn new(core: &Arc<AppCore>) -> Self {
        Self {
            core: Arc::downgrade(core),
            root_id: Uuid::new_v4(),
        }
    }

    fn core(&self) -> Result<Arc<AppCore>, ApiError> {
        self.core
            .upgrade()
            .ok_or_else(|| ApiError::Internal("The profile is closed.".into()))
    }
}

fn bad(message: impl Into<String>) -> ApiError {
    ApiError::BadRequest(message.into())
}
fn status(message: Option<String>, details: Vec<String>, success: bool) -> Value {
    json!({"Message": message, "Details": details, "IsSuccessful": success})
}
fn message(text: impl Into<String>) -> Value {
    status(Some(text.into()), Vec::new(), true)
}
fn context_status() -> Value {
    json!({"Message": null, "Details": [], "IsSuccessful": true, "State": 0,
        "Sequence": {"Name": "", "FileName": ""}, "Position": "00:00:00"})
}
fn guid(text: Option<&str>) -> Uuid {
    text.and_then(|s| Uuid::parse_str(s.trim()).ok())
        .unwrap_or_default()
}
fn int(text: Option<&str>) -> i32 {
    text.and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}
fn double(text: Option<&str>) -> f64 {
    text.and_then(|s| s.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .unwrap_or(0.0)
}
fn boolean(text: Option<&str>) -> bool {
    text.is_some_and(|s| s.trim().eq_ignore_ascii_case("true"))
}
fn intensity(value: f64) -> f64 {
    if !(0.0..=100.0).contains(&value) {
        100.0
    } else {
        value
    }
}
fn node_name(core: &AppCore, id: Uuid) -> Option<String> {
    let system = core.system.read();
    system
        .tree
        .find(&id)
        .map(|node| system.tree.node(node).name.clone())
}
fn element(tree: &ElementTree, id: NodeId, recurse: bool) -> Value {
    let node = tree.node(id);
    let children = if recurse && !node.children.is_empty() {
        Value::Array(
            node.children
                .iter()
                .map(|&child| element(tree, child, true))
                .collect(),
        )
    } else {
        Value::Null
    };
    let colors: Vec<_> = tree
        .valid_colors(id, true)
        .iter()
        .map(|c| format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b))
        .collect();
    json!({"Id": node.id.to_string(), "Name": node.name, "Colors": colors, "Children": children})
}

/// Constant ("c") TimeSpan format used by System.Text.Json and the status messages.
fn time_text(time: TimeSpan) -> String {
    let ticks = time.ticks().unsigned_abs();
    let seconds = ticks / 10_000_000;
    let days = seconds / 86_400;
    let mut out = if time.ticks() < 0 {
        "-".to_string()
    } else {
        String::new()
    };
    if days > 0 {
        out.push_str(&format!("{days}."));
    }
    out.push_str(&format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60
    ));
    let fraction = ticks % 10_000_000;
    if fraction > 0 {
        out.push_str(&format!(".{fraction:07}"));
    }
    out
}

fn set_level() -> &'static EffectDescriptor {
    vixen_effect_registry::all()
        .iter()
        .find(|effect| effect.name == "Set Level")
        .expect("registered core effect")
}
fn on_color(text: Option<&str>) -> Result<Option<Color>, ApiError> {
    let Some(text) = text.filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if text.len() != 7 || !text.starts_with('#') {
        return Err(bad(format!("Invalid color (Parameter '{text}')")));
    }
    let rgb = u32::from_str_radix(&text[1..], 16)
        .map_err(|_| ApiError::Internal("Invalid color.".into()))?;
    Ok(Some(Color::from_argb_u32(0xff00_0000 | rgb)))
}
fn on_message(name: &str, duration: i32, level: f64) -> String {
    if duration == 0 {
        format!("{name} turned on at {level}% intensity.")
    } else {
        format!("{name} turned on for {duration} seconds at {level}% intensity.")
    }
}
fn turn_on(
    core: &AppCore,
    id: Uuid,
    duration: i32,
    level: f64,
    color: Option<Color>,
    stopping: &AtomicBool,
) -> Result<(), ApiError> {
    let descriptor = set_level();
    let mut params = descriptor.new_params();
    params.set("IntensityLevel", ParamValue::Double(level / 100.0));
    if let Some(color) = color {
        params.set("Color", ParamValue::Rgb(Rgb::from_color(color)));
    }
    let duration = if duration > 0 {
        TimeSpan::from_secs(duration as f64)
    } else {
        crate::live::INDEFINITE
    };
    render_effect(core, id, descriptor, params, duration, stopping)
}

fn cancelled(stopping: &AtomicBool) -> Result<(), ApiError> {
    if stopping.load(Ordering::Acquire) {
        Err(ApiError::Internal("Web server is stopping.".into()))
    } else {
        Ok(())
    }
}

// Bound caller-controlled remote work before pixel renderers allocate frame buffers.
// Sparse indefinite Set Level requests use the separate On route and do not need this bound.
const MAX_REMOTE_SAMPLES: usize = 16_000_000;
fn check_render_work(leaves: usize, duration_ms: u64, frame_ms: u64) -> Result<(), ApiError> {
    let frames = duration_ms
        .checked_add(frame_ms.max(1) - 1)
        .map(|duration| duration / frame_ms.max(1))
        .and_then(|frames| usize::try_from(frames).ok());
    if frames
        .and_then(|frames| leaves.checked_mul(frames))
        .is_none_or(|samples| samples > MAX_REMOTE_SAMPLES)
    {
        return Err(bad("The requested effect exceeds the remote render limit."));
    }
    Ok(())
}

fn render_effect(
    core: &AppCore,
    id: Uuid,
    descriptor: &'static EffectDescriptor,
    mut params: Params,
    duration: TimeSpan,
    stopping: &AtomicBool,
) -> Result<(), ApiError> {
    cancelled(stopping)?;
    let env = core.session_environment();
    let node = env
        .tree
        .find(&id)
        .ok_or_else(|| bad("Invalid element id."))?;
    let targets = [node];
    let cache_lease = core.video_cache.session();
        let context = RenderCtx { video_cache: Some(vixen_video::cache::CacheContext { session: cache_lease.session(), instance: Uuid::new_v4() }),
        tree: &env.tree,
        targets: &targets,
        start_time: TimeSpan::ZERO,
        time_span: duration,
        frame_ms: env.frame_ms,
        marks: &[],
        audio: None,
        module_data_root: env.module_data_root.as_deref(),
        cancel: stopping,
    };
    descriptor.set_targets(&mut params, &context.target_info());
    let intents = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        (descriptor.render)(&context, &params)
    }))
    .map_err(|_| ApiError::Internal("The effect could not be rendered.".into()))?;
    cancelled(stopping)?;
    core.live.insert(core, LIVE, id, intents, duration);
    Ok(())
}

#[derive(Debug)]
struct GroupState {
    id: Uuid,
    duration: i32,
    intensity: f64,
    color: Option<String>,
}
fn json_field<'a>(object: &'a serde_json::Map<String, Value>, name: &str) -> Option<&'a Value> {
    object
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value)
}
fn group_states(input: &ApiInput) -> Result<Vec<GroupState>, ApiError> {
    let malformed = || ApiError::Internal("The element-state list is null or malformed.".into());
    input
        .json
        .as_ref()
        .and_then(Value::as_array)
        .ok_or_else(malformed)?
        .iter()
        .map(|value| {
            let object = value.as_object().ok_or_else(malformed)?;
            let id = match json_field(object, "Id") {
                None => Uuid::nil(),
                Some(v) => {
                    Uuid::parse_str(v.as_str().ok_or_else(malformed)?).map_err(|_| malformed())?
                }
            };
            let duration = match json_field(object, "Duration") {
                None => 0,
                Some(v) => {
                    i32::try_from(v.as_i64().ok_or_else(malformed)?).map_err(|_| malformed())?
                }
            };
            let intensity = match json_field(object, "Intensity") {
                None => 0.0,
                Some(v) => v.as_f64().ok_or_else(malformed)?,
            };
            let color = match json_field(object, "Color") {
                None | Some(Value::Null) => None,
                Some(v) => Some(v.as_str().ok_or_else(malformed)?.into()),
            };
            Ok(GroupState {
                id,
                duration,
                intensity,
                color,
            })
        })
        .collect()
}

fn html_color(text: &str) -> Result<Color, ApiError> {
    crate::setup::tags::parse_html_color(text)
        .map(Color::from_argb_u32)
        .ok_or_else(|| bad(format!("Invalid effect color '{text}'.")))
}
fn finite(text: &str) -> Option<f64> {
    text.trim().parse::<f64>().ok().filter(|n| n.is_finite())
}
fn option(kind: ParamKind, name: &str, value: &str) -> Result<ParamValue, ApiError> {
    let parsed = match kind {
        ParamKind::Bool => match value.trim().to_ascii_lowercase().as_str() {
            "true" => Some(ParamValue::Bool(true)),
            "false" => Some(ParamValue::Bool(false)),
            _ => None,
        },
        ParamKind::Int => value.trim().parse().ok().map(ParamValue::Int),
        ParamKind::Long => value.trim().parse().ok().map(ParamValue::Long),
        ParamKind::Double => finite(value).map(ParamValue::Double),
        ParamKind::Percentage => finite(value).map(ParamValue::Percentage),
        ParamKind::Str => Some(ParamValue::Str(value.into())),
        ParamKind::Enum(desc) => desc
            .members
            .iter()
            .find(|member| member.name == value)
            .map(|member| ParamValue::Enum(member.name.into())),
        ParamKind::Guid => Uuid::parse_str(value.trim()).ok().map(ParamValue::Guid),
        ParamKind::Time => TimeSpan::parse_xml(value.trim()).map(ParamValue::Time),
        _ => None,
    };
    parsed.ok_or_else(|| {
        bad(format!(
            "Invalid value '{value}' for effect parameter '{name}'."
        ))
    })
}
fn effect_params(
    descriptor: &'static EffectDescriptor,
    input: &ApiInput,
) -> Result<Params, ApiError> {
    let mut params = descriptor.new_params();
    let colors = input.dictionary("Color");
    let curve = input.dictionary("LevelCurve");
    let options = input.dictionary("Options");
    for property in descriptor.params {
        let value = match property.kind {
            ParamKind::Color | ParamKind::Rgb => {
                let first = colors.first().ok_or_else(|| {
                    ApiError::Internal("Color is required for this effect.".into())
                })?;
                let color = html_color(first.0)?;
                Some(if matches!(property.kind, ParamKind::Rgb) {
                    ParamValue::Rgb(Rgb::from_color(color))
                } else {
                    ParamValue::Color(color)
                })
            }
            ParamKind::Gradient => {
                if colors.is_empty() {
                    return Err(ApiError::Internal(
                        "Color is required for this effect.".into(),
                    ));
                }
                let mut points = Vec::new();
                for (name, at) in &colors {
                    let position = finite(at)
                        .ok_or_else(|| bad(format!("Invalid gradient position '{at}'.")))?;
                    points.push(ColorPoint::new(html_color(name)?, position));
                }
                let mut gradient = ColorGradient::default();
                gradient.set_colors(points);
                Some(ParamValue::Gradient(gradient))
            }
            ParamKind::Curve => {
                if curve.is_empty() {
                    return Err(ApiError::Internal(
                        "LevelCurve is required for this effect.".into(),
                    ));
                }
                let mut points = Vec::new();
                for (x, y) in &curve {
                    let point = finite(x)
                        .zip(finite(y))
                        .ok_or_else(|| bad("Invalid LevelCurve point."))?;
                    points.push(point);
                }
                Some(ParamValue::Curve(Curve::from_points(points)))
            }
            _ => None,
        };
        if let Some(value) = value {
            params.set(property.name, value);
        }
        if let Some((_, value)) = options.iter().find(|(name, _)| *name == property.name) {
            params.set(property.name, option(property.kind, property.name, value)?);
        }
    }
    // Upstream dereferences a null Options dictionary once per effect parameter.
    // Preserve the wire-level 500 for omitted dictionaries; invalid supplied values
    // above remain the spec's safe 400 conversion failures.
    if !descriptor.params.is_empty() && options.is_empty() {
        return Err(ApiError::Internal(
            "Options is required for this effect.".into(),
        ));
    }
    Ok(params)
}

fn decoded(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => result.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
                result.push(u8::from_str_radix(hex, 16).ok()?);
                i += 2;
            }
            b'%' => return None,
            b => result.push(b),
        }
        i += 1;
    }
    String::from_utf8(result).ok()
}
fn sequence_path(core: &AppCore, text: &str) -> Option<PathBuf> {
    let root = core.paths.root().canonicalize().ok()?;
    let path = |text: &str| {
        let p = Path::new(text);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            root.join(p)
        }
    };
    let given = path(text);
    // Preserve literal '+' and '%' in existing names before the legacy extra decode.
    let candidate = if given.exists() {
        given
    } else {
        path(&decoded(text)?)
    };
    let candidate = candidate.canonicalize().ok()?;
    (candidate.starts_with(root) && candidate.is_file()).then_some(candidate)
}

fn set_controller(core: &AppCore, id: Uuid, running: bool) {
    core.playback.set_controller_running(&id, running);
}

impl WebApi for SessionApi {
    fn invoke(&self, action: Action, input: ApiInput) -> Result<ApiResponse, ApiError> {
        self.invoke_cancellable(action, input, &AtomicBool::new(false))
    }

    fn invoke_cancellable(
        &self,
        action: Action,
        input: ApiInput,
        stopping: &AtomicBool,
    ) -> Result<ApiResponse, ApiError> {
        cancelled(stopping)?;
        let core = self.core()?;
        let value = match action {
            Action::Elements | Action::SearchElements | Action::ChildElements | Action::ParentElements => {
                let system = core.system.read(); let tree = &system.tree;
                let ids = match action {
                    Action::Elements => tree.roots.clone(),
                    Action::SearchElements => {
                        let query = input.get("q").filter(|s| !s.is_empty()).ok_or_else(|| bad("Value cannot be null. (Parameter 'query')"))?.to_lowercase();
                        let mut nodes: Vec<_> = tree.all_nodes().filter(|(_, node)| node.name.to_lowercase().starts_with(&query)).map(|(id,_)| id).collect();
                        nodes.sort_by_cached_key(|&id| tree.node(id).name.to_lowercase()); nodes
                    },
                    Action::ChildElements => {
                        let id = Uuid::parse_str(input.get("id").unwrap_or("").trim()).map_err(|_| bad("Invalid element id. (Parameter 'id')"))?;
                        tree.find(&id).map(|id| tree.node(id).children.clone()).unwrap_or_default()
                    },
                    Action::ParentElements => {
                        let text = input.get("id").filter(|s| !s.is_empty()).ok_or_else(|| bad("Value cannot be null. (Parameter 'id')"))?;
                        let id = Uuid::parse_str(text.trim()).map_err(|_| bad("Invalid Element id."))?;
                        let id = tree.find(&id).ok_or_else(|| bad("Element id does not exist. (Parameter 'id')"))?;
                        tree.node(id).parents.clone()
                    },
                    _ => unreachable!(),
                };
                let mut rows: Vec<Value> = ids.into_iter().map(|id| element(tree,id,matches!(action,Action::Elements|Action::SearchElements))).collect();
                if action == Action::ParentElements && tree.find(&guid(input.get("id"))).is_some_and(|id| tree.roots.contains(&id)) {
                    // C# keeps an unregistered private RootNode, also present in root rows' Parents.
                    let mut colors = Vec::new();
                    for &root in &tree.roots { for color in tree.valid_colors(root,true) { if !colors.contains(&color) {colors.push(color);} } }
                    let colors: Vec<_> = colors.iter().map(|c|format!("#{:02X}{:02X}{:02X}",c.r,c.g,c.b)).collect();
                    rows.push(json!({"Id":self.root_id.to_string(),"Name":"Root","Colors":colors,"Children":null}));
                }
                Value::Array(rows)
            },
            Action::ElementOn => {
                let id = guid(input.get("id")); let name = node_name(&core,id).ok_or_else(|| bad("Invalid element id. (Parameter 'id')"))?;
                let duration = int(input.get("duration")); let level = intensity(double(input.get("intensity")));
                turn_on(&core,id,duration,level,on_color(input.get("color"))?,stopping)?;
                message(on_message(&name,duration,level))
            },
            Action::ElementOff => {
                let id = guid(input.get("id")); let name = node_name(&core,id).ok_or_else(|| bad("Element id is invalid. (Parameter 'id')"))?;
                core.live.turn_off(LIVE,id); message(format!("{name} turned off."))
            },
            Action::GroupOn | Action::GroupOff => {
                let states = group_states(&input)?; let mut details = Vec::new(); let mut pending = Vec::new();
                for state in states {
                    let Some(name) = node_name(&core,state.id) else { details.push(format!("Element not found. {}",state.id)); continue };
                    if action == Action::GroupOn {
                        let color = on_color(state.color.as_deref())?; let level = intensity(state.intensity);
                        details.push(on_message(&name,state.duration,level)); pending.push((state.id,state.duration,level,color));
                    } else { details.push(format!("{name} turned off.")); pending.push((state.id,0,0.0,None)); }
                }
                for &(id,duration,level,color) in &pending {
                    if action == Action::GroupOn { turn_on(&core,id,duration,level,color,stopping)?; } else { core.live.turn_off(LIVE,id); }
                }
                status(Some(format!("{} elements turned {}.",pending.len(),if action == Action::GroupOn {"on"} else {"off"})),details,true)
            },
            Action::ClearElements => { core.live.clear(LIVE); message("Clear active live context effects requested.") },
            Action::ElementEffect => {
                if input.fields.is_empty() { return Err(bad("Value cannot be null. (Parameter 'elementEffect')")); }
                let id = guid(input.get("id")); let name = node_name(&core,id).ok_or_else(|| bad("Invalid element id (Parameter 'elementEffect')"))?;
                let effect_name = input.get("EffectName").unwrap_or("");
                if let Some(descriptor) = vixen_effect_registry::all().iter().find(|d| d.name == effect_name) {
                    let duration = int(input.get("duration"));
                    let params = effect_params(descriptor,&input)?;
                    let system = core.system.read();
                    let leaves = system.tree.find(&id).map(|node| system.tree.leaves(node).len()).unwrap_or(0);
                    check_render_work(leaves, duration.max(0) as u64, system.default_update_interval.max(1) as u64)?;
                    drop(system);
                    render_effect(&core,id,descriptor,params,TimeSpan::from_ms_i(duration as i64),stopping)?;
                    message(format!("{name} element(s) turned effect {effect_name} for {duration} milliseconds."))
                } else { status(None,Vec::new(),true) }
            },
            Action::PlayStatus => Value::Array(self.context_states()),
            Action::Sequences => Value::Array(crate::export_wizard::all_sequence_files(&core.paths,false).into_iter().map(|path| path.canonicalize().unwrap_or(path)).map(|path| json!({
                "Name": path.file_stem().unwrap_or_default().to_string_lossy(), "FileName": path.to_string_lossy()
            })).collect()),
            Action::PlaySequence => {
                if input.fields.is_empty() { return Err(bad("Value cannot be null. (Parameter 'sequence')")); }
                let name = input.get("Name").unwrap_or(""); let file = input.get("FileName").unwrap_or(""); let mut response = context_status();
                if core.playback.control_contexts(name,Transport::Playing) > 0 { response["Message"] = json!(format!("Resumed {name}")); }
                else if let Some(path) = sequence_path(&core,file) {
                    logging::info(LOGGER,&format!("Web - Loading sequence {name} from filename {file}"));
                    match SequenceSession::open_cancellable(&path,&core.session_environment(),stopping) {
                        Ok(mut sequence) => {
                            logging::info(LOGGER,&format!("Web - Prerendering effects for sequence: {name}"));
                            loop {
                                cancelled(stopping)?;
                                sequence.apply_finished_renders();
                                if sequence.unrendered_count() == 0 { break; }
                                std::thread::sleep(std::time::Duration::from_millis(5));
                            }
                            // Dropping a cancelled fresh session signals its rendering workers.
                            cancelled(stopping)?;
                            let context = core.playback.start_named_context(&sequence.name(),path.file_name().unwrap_or_default().to_string_lossy().as_ref(),sequence.schedule(),sequence.audio().cloned(),sequence.length());
                            let length = sequence.length(); let sequence_name = sequence.name();
                            let video_cache = sequence.take_video_cache_lease();
                            match std::thread::Builder::new().name("web-sequence".into()).spawn(move || { context.wait(); drop(video_cache); }) {
                                Ok(_) => { response["State"] = json!(1); response["Sequence"] = json!({"Name":sequence_name,"FileName":path.to_string_lossy()}); response["Message"] = json!(format!("Playing sequence {name} of length {}",time_text(length))); },
                                Err(error) => { logging::error(LOGGER,"Error loading the sequence.",Some(&error.to_string())); response["Message"] = json!(format!("An error occurred loading sequence {file}.")); },
                            }
                        },
                        Err(error) => { cancelled(stopping)?; logging::error(LOGGER,"Error loading the sequence.",Some(&error.to_string())); response["Message"] = json!(format!("An error occurred loading sequence {file}.")); },
                    }
                } else { response["Message"] = json!(format!("Sequence {file} not found.")); }
                response
            },
            Action::PauseSequence | Action::StopSequence => {
                let name = input.get("Name").unwrap_or(""); let mut response = context_status();
                let stop = action == Action::StopSequence;
                if stop { response["Sequence"] = json!({"Name":input.get("Name"),"FileName":input.get("FileName")}); }
                let count = core.playback.control_contexts(name,if stop {Transport::Stopped} else {Transport::Paused});
                response["Message"] = if count == 0 { json!("Sequence not found.") } else { json!(format!("Sequence {name} {}.",if stop {"stopped"} else {"paused"})) };
                response
            },
            Action::Controllers | Action::Controller | Action::SetControllerState | Action::SetAllControllersState => {
                let system = core.system.read();
                let states = core.playback.controller_states();
                let state = |id: Uuid| states.iter().find(|(key,_,_)| *key == id).map(|(_,running,paused)| (*running,*paused)).unwrap_or((false,false));
                let controller = |c: &vixen_io::system::Controller| { let (running,paused) = state(c.id); json!({"Id":c.id.to_string(),"Name":c.name,"IsRunning":running,"IsPaused":paused}) };
                match action {
                    Action::Controllers => Value::Array(system.controllers.iter().map(controller).collect()),
                    Action::Controller => { let Some(c) = system.controllers.iter().find(|c| c.id == guid(input.get("id"))) else { return Ok(ApiResponse::NoContent) }; controller(c) },
                    Action::SetAllControllersState => { let running = boolean(input.get("isRunning")); for c in &system.controllers { set_controller(&core,c.id,running); } message("All controllers state Changed.") },
                    _ => {
                        if let Some(c) = system.controllers.iter().find(|c| c.id == guid(input.get("id"))) {
                            let running = boolean(input.get("isRunning")); let (was_running,paused) = state(c.id);
                            let changed = if running { !was_running || paused } else { was_running };
                            if changed { set_controller(&core,c.id,running); }
                            status(Some(format!("{} state {}.",c.name,if changed {"Changed"} else {"Not Changed"})),Vec::new(),changed)
                        } else { status(Some("Controller not found.".into()),Vec::new(),false) }
                    },
                }
            },
            Action::SaveSystem => { let saved = core.save_system_config().unwrap_or(false); status(Some(if saved {"Save Successful"} else {"Save Failed"}.into()),Vec::new(),saved) },
        };
        Ok(ApiResponse::Json(value))
    }

    fn context_states(&self) -> Vec<Value> {
        let Ok(core) = self.core() else {
            return Vec::new();
        };
        core.playback.context_states().into_iter().filter(|state| state.name != LIVE && state.transport != Transport::Stopped).map(|state| {
            let mut result = context_status();
            result["State"] = json!(if state.transport == Transport::Paused {2} else {1});
            result["Sequence"] = json!({"Name":state.name,"FileName":if state.file_name.is_empty() {Value::Null} else {json!(state.file_name)}});
            result["Position"] = json!(time_text(state.position)); result
        }).collect()
    }
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
