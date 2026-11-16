//! End-to-end business API checks use private profiles and a muted execution engine.
use super::*;
use parking_lot::Mutex;
use std::time::Duration;
use vixen_core::element::{property_ids, ElementColorType, PropertyData, PropertyInstance};
use vixen_core::{CombinedState, ElementTree};
use vixen_engine::FrameSink;

struct Fixture {
    core: Arc<AppCore>,
    api: SessionApi,
    group: Uuid,
    leaf: Uuid,
    other: Uuid,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("vixen-web-api-{}", Uuid::new_v4()));
        let paths = vixen_io::DataPaths::new(root);
        paths.ensure_directories().unwrap();
        let mut tree = ElementTree::new();
        let group = tree.add_node(Uuid::new_v4(), "Group", None);
        let leaf = tree.add_element("Red Leaf");
        tree.add_child(group, leaf);
        let other = tree.add_element("Other");
        tree.node_mut(leaf).properties.push(PropertyInstance {
            type_id: property_ids::COLOR,
            instance_id: Uuid::new_v4(),
            data: PropertyData::Color {
                color_type: ElementColorType::SingleColor,
                single_color: Color::RED,
                color_set_name: None,
            },
        });
        let ids = (tree.node(group).id, tree.node(leaf).id, tree.node(other).id);
        let mut system = vixen_io::SystemData::default();
        system.tree = tree;
        system.default_update_interval = 2;
        let core = Arc::new(AppCore::assemble(
            paths,
            system,
            Default::default(),
            None,
            crate::boot::LaunchOptions {
                muted: true,
                ..Default::default()
            },
            None,
        ));
        Self {
            api: SessionApi::new(&core),
            core,
            group: ids.0,
            leaf: ids.1,
            other: ids.2,
        }
    }
    fn call(&self, action: Action, pairs: &[(&str, &str)]) -> Value {
        let ApiResponse::Json(value) = self.api.invoke(action, form(pairs)).unwrap() else {
            panic!("JSON expected")
        };
        value
    }
    fn save_sequence(&self, name: &str, length: TimeSpan) -> PathBuf {
        let path = self.core.paths.sequence_dir().join(name);
        let mut session = SequenceSession::new_untitled(&self.core.session_environment());
        session.set_length(length);
        session.save(Some(&path)).unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.core.playback.close_execution();
        self.core.live.release_all();
        // Even when execution was initially Closed, release independently started test contexts.
        for context in self.core.playback.context_states() {
            self.core
                .playback
                .control_contexts(&context.name, Transport::Stopped);
        }
        let _ = std::fs::remove_dir_all(self.core.paths.root());
    }
}
fn form(pairs: &[(&str, &str)]) -> ApiInput {
    ApiInput {
        fields: pairs
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect(),
        json: None,
    }
}
fn wait(mut done: impl FnMut() -> bool) -> bool {
    for _ in 0..400 {
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    false
}
struct Recorder(Arc<Mutex<Vec<CombinedState>>>);
impl FrameSink for Recorder {
    fn frame(&mut self, _: TimeSpan, states: &[CombinedState], _: bool) {
        *self.0.lock() = states.to_vec();
    }
}
fn colors(states: &Mutex<Vec<CombinedState>>) -> Vec<Color> {
    states
        .lock()
        .iter()
        .filter_map(|s| {
            s.lighting
                .map(|l| l.full_color())
                .or_else(|| s.rgb.map(|l| l.full_color()))
        })
        .collect()
}

#[test]
fn element_shapes_search_and_direct_relations_match_the_wire_contract() {
    let f = Fixture::new();
    let roots = f.call(Action::Elements, &[]);
    assert_eq!(roots[0]["Name"], "Group");
    assert_eq!(roots[0]["Colors"], json!(["#FF0000"]));
    assert_eq!(roots[0]["Children"][0]["Name"], "Red Leaf");
    assert!(roots[0]["Children"][0]["Children"].is_null());
    let search = f.call(Action::SearchElements, &[("q", "rEd")]);
    assert_eq!(search.as_array().unwrap().len(), 1);
    let children = f.call(Action::ChildElements, &[("id", &f.group.to_string())]);
    assert!(children[0]["Children"].is_null());
    let parents = f.call(Action::ParentElements, &[("id", &f.leaf.to_string())]);
    assert_eq!(parents[0]["Name"], "Group");
    assert!(parents[0]["Children"].is_null());
    assert_eq!(
        f.call(
            Action::ChildElements,
            &[("id", &Uuid::new_v4().to_string())]
        ),
        json!([])
    );
    let root = f.call(Action::ParentElements, &[("id", &f.group.to_string())]);
    assert_eq!(root[0]["Name"], "Root");
    assert_eq!(root[0]["Colors"], json!(["#FF0000"]));
}
#[test]
fn argument_error_messages_match_upstream() {
    let f = Fixture::new();
    for (action, input, error) in [
        (
            Action::SearchElements,
            form(&[]),
            "Value cannot be null. (Parameter 'query')",
        ),
        (
            Action::ChildElements,
            form(&[("id", "bad")]),
            "Invalid element id. (Parameter 'id')",
        ),
        (
            Action::ParentElements,
            form(&[]),
            "Value cannot be null. (Parameter 'id')",
        ),
        (
            Action::ParentElements,
            form(&[("id", "bad")]),
            "Invalid Element id.",
        ),
        (
            Action::ParentElements,
            form(&[("id", &Uuid::new_v4().to_string())]),
            "Element id does not exist. (Parameter 'id')",
        ),
        (
            Action::ElementOn,
            form(&[("id", "bad")]),
            "Invalid element id. (Parameter 'id')",
        ),
        (
            Action::ElementOff,
            form(&[("id", "bad")]),
            "Element id is invalid. (Parameter 'id')",
        ),
        (
            Action::ElementEffect,
            form(&[]),
            "Value cannot be null. (Parameter 'elementEffect')",
        ),
        (
            Action::ElementEffect,
            form(&[("id", "bad")]),
            "Invalid element id (Parameter 'elementEffect')",
        ),
    ] {
        assert_eq!(f.api.invoke(action, input), Err(bad(error)));
    }
}
#[test]
fn on_intensity_defaults_messages_and_color_failures() {
    let f = Fixture::new();
    let id = f.other.to_string();
    assert_eq!(
        f.call(Action::ElementOn, &[("id", &id), ("intensity", "150")])["Message"],
        "Other turned on at 100% intensity."
    );
    assert_eq!(
        f.call(
            Action::ElementOn,
            &[("id", &id), ("intensity", "-1"), ("duration", "-2")]
        )["Message"],
        "Other turned on for -2 seconds at 100% intensity."
    );
    assert_eq!(
        f.call(Action::ElementOn, &[("id", &id)])["Message"],
        "Other turned on at 0% intensity."
    );
    assert_eq!(
        f.api
            .invoke(Action::ElementOn, form(&[("id", &id), ("color", "red")])),
        Err(bad("Invalid color (Parameter 'red')"))
    );
    assert!(matches!(
        f.api.invoke(
            Action::ElementOn,
            form(&[("id", &id), ("color", "#GG0000")])
        ),
        Err(ApiError::Internal(_))
    ));
    assert_eq!(
        f.call(Action::ElementOff, &[("id", &id)])["Message"],
        "Other turned off."
    );
    assert_eq!(
        f.call(Action::ClearElements, &[])["Message"],
        "Clear active live context effects requested."
    );
}
#[test]
fn group_actions_keep_success_details_and_reject_invalid_body_atomically() {
    let f = Fixture::new();
    let unknown = Uuid::new_v4();
    let body = json!([{"id":unknown.to_string()},{"Id":f.other.to_string(),"Intensity":120,"Duration":0,"Color":"#00FF00"}]);
    let ApiResponse::Json(value) = f
        .api
        .invoke(
            Action::GroupOn,
            ApiInput {
                fields: vec![],
                json: Some(body.clone()),
            },
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(value["Message"], "1 elements turned on.");
    assert_eq!(
        value["Details"],
        json!([
            format!("Element not found. {unknown}"),
            "Other turned on at 100% intensity."
        ])
    );
    let ApiResponse::Json(value) = f
        .api
        .invoke(
            Action::GroupOff,
            ApiInput {
                fields: vec![],
                json: Some(body),
            },
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(value["Message"], "1 elements turned off.");
    for json in [None, Some(json!({})), Some(json!([{"Id":"invalid"}]))] {
        assert!(matches!(
            f.api.invoke(
                Action::GroupOn,
                ApiInput {
                    fields: vec![],
                    json
                }
            ),
            Err(ApiError::Internal(_))
        ));
    }
    let recorded = Arc::new(Mutex::new(Vec::new()));
    f.core
        .playback
        .add_sink(Box::new(Recorder(recorded.clone())));
    let invalid = json!([{"Id":f.other.to_string(),"Intensity":100,"Color":"#0000FF"},{"Id":f.group.to_string(),"Color":"bad"}]);
    assert!(f
        .api
        .invoke(
            Action::GroupOn,
            ApiInput {
                fields: vec![],
                json: Some(invalid)
            }
        )
        .is_err());
    std::thread::sleep(Duration::from_millis(20));
    assert!(!colors(&recorded).contains(&Color::BLUE));
}
#[test]
fn generic_effect_parameter_conversion_is_typed_and_unknown_effect_is_success() {
    let f = Fixture::new();
    let id = f.other.to_string();
    assert_eq!(
        f.call(
            Action::ElementEffect,
            &[("Id", &id), ("EffectName", "unknown")]
        ),
        status(None, vec![], true)
    );
    let input = form(&[
        ("Color[#0000FF]", "0"),
        ("Options[IntensityLevel]", "0.25"),
        ("Options[TargetPositioning]", "Locations"),
    ]);
    let params = effect_params(set_level(), &input).unwrap();
    assert_eq!(params.double("IntensityLevel"), 0.25);
    assert_eq!(params.color("Color"), Color::BLUE);
    assert_eq!(params.enum_name("TargetPositioning"), "Locations");
    for value in ["bad", "NaN", "inf"] {
        assert!(effect_params(
            set_level(),
            &form(&[("Color[#FF0000]", "0"), ("Options[IntensityLevel]", value)])
        )
        .is_err());
    }
    assert!(option(ParamKind::Bool, "test", "1").is_err());
    assert_eq!(
        option(ParamKind::Bool, "test", "TRUE").unwrap(),
        ParamValue::Bool(true)
    );
    assert!(effect_params(set_level(), &form(&[])).is_err());
    let status = f.call(
        Action::ElementEffect,
        &[
            ("Id", &id),
            ("EffectName", "Set Level"),
            ("Duration", "80"),
            ("Color[#0000FF]", "0"),
            ("Options[IntensityLevel]", "1"),
        ],
    );
    assert_eq!(
        status["Message"],
        "Other element(s) turned effect Set Level for 80 milliseconds."
    );
}
#[test]
fn gradients_and_curves_use_dictionary_entries() {
    let pulse = vixen_effect_registry::all()
        .iter()
        .find(|d| d.name == "Pulse")
        .unwrap();
    let input = form(&[
        ("Color[#FF0000]", "0"),
        ("Color[#0000FF]", "1"),
        ("LevelCurve[0]", "25"),
        ("LevelCurve[100]", "75"),
        ("Options[unused]", ""),
    ]);
    let params = effect_params(pulse, &input).unwrap();
    let gradient = pulse
        .params
        .iter()
        .find(|p| matches!(p.kind, ParamKind::Gradient))
        .unwrap();
    assert_eq!(params.gradient(gradient.name).colors().len(), 2);
    let curve = pulse
        .params
        .iter()
        .find(|p| matches!(p.kind, ParamKind::Curve))
        .unwrap();
    assert_eq!(
        params.curve(curve.name).points,
        vec![(0.0, 25.0), (100.0, 75.0)]
    );
}
#[test]
fn sequence_list_includes_backups_and_filters_extensions() {
    let f = Fixture::new();
    f.save_sequence("Song.tim", TimeSpan::from_secs(10.0));
    std::fs::write(f.core.paths.sequence_backup_dir().join("copy.tim"), "").unwrap();
    std::fs::write(f.core.paths.sequence_dir().join("Song.tim.8"), "").unwrap();
    let list = f.call(Action::Sequences, &[]);
    let names: Vec<_> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["Name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["Song", "copy"]);
}
#[test]
fn sequence_paths_preserve_plus_and_cannot_escape_root() {
    let f = Fixture::new();
    let literal = f.save_sequence("A+B.tim", TimeSpan::from_secs(10.0));
    let spaced = f.save_sequence("A B.tim", TimeSpan::from_secs(10.0));
    assert_eq!(
        sequence_path(&f.core, literal.to_str().unwrap()),
        Some(literal.canonicalize().unwrap())
    );
    assert_eq!(
        sequence_path(&f.core, &spaced.to_str().unwrap().replace(' ', "%20")),
        Some(spaced.canonicalize().unwrap())
    );
    let outside = f.core.paths.root().with_extension("tim");
    std::fs::write(&outside, "").unwrap();
    assert!(sequence_path(&f.core, outside.to_str().unwrap()).is_none());
    #[cfg(unix)]
    {
        let link = f.core.paths.sequence_dir().join("outside.tim");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        assert!(sequence_path(&f.core, link.to_str().unwrap()).is_none());
    }
    std::fs::remove_file(outside).unwrap();
}
#[test]
fn play_twice_mixes_copies_pause_and_resume_match_all_names() {
    let f = Fixture::new();
    let file = f.save_sequence("Song.tim", TimeSpan::from_secs(10.0));
    let file = file.to_str().unwrap();
    for _ in 0..2 {
        let status = f.call(
            Action::PlaySequence,
            &[("Name", "Song"), ("FileName", file)],
        );
        assert_eq!(status["State"], 1);
        assert_eq!(
            status["Message"],
            "Playing sequence Song of length 00:00:10"
        );
    }
    assert_eq!(f.api.context_states().len(), 2);
    assert_eq!(
        f.call(Action::PauseSequence, &[("Name", "Song")])["Message"],
        "Sequence Song paused."
    );
    assert!(f.api.context_states().iter().all(|s| s["State"] == 2));
    let resumed = f.call(
        Action::PlaySequence,
        &[("Name", "Song"), ("FileName", "missing")],
    );
    assert_eq!(resumed["Message"], "Resumed Song");
    assert_eq!(resumed["State"], 0);
    assert_eq!(resumed["Sequence"], json!({"Name":"","FileName":""}));
    assert_eq!(f.api.context_states().len(), 2);
    let stopped = f.call(
        Action::StopSequence,
        &[("Name", "Song"), ("FileName", file)],
    );
    assert_eq!(stopped["Message"], "Sequence Song stopped.");
    assert!(wait(|| f.api.context_states().is_empty()));
}
#[test]
fn sequence_failures_are_successful_statuses_and_natural_end_releases_context() {
    let f = Fixture::new();
    let missing = f.call(
        Action::PlaySequence,
        &[("Name", "X"), ("FileName", "missing.tim")],
    );
    assert_eq!(missing["Message"], "Sequence missing.tim not found.");
    assert_eq!(missing["IsSuccessful"], true);
    assert_eq!(missing["State"], 0);
    let bad = f.core.paths.sequence_dir().join("bad.tim");
    std::fs::write(&bad, "invalid").unwrap();
    assert_eq!(
        f.call(
            Action::PlaySequence,
            &[("Name", "bad"), ("FileName", bad.to_str().unwrap())]
        )["Message"],
        format!("An error occurred loading sequence {}.", bad.display())
    );
    let file = f.save_sequence("Short.tim", TimeSpan::from_ms_i(20));
    f.call(
        Action::PlaySequence,
        &[("Name", "Short"), ("FileName", file.to_str().unwrap())],
    );
    assert!(wait(|| f.api.context_states().is_empty()));
}
#[test]
fn controls_reach_editor_and_scheduled_contexts_and_skip_web_in_status() {
    let f = Fixture::new();
    let file = f.save_sequence("Editor.tim", TimeSpan::from_secs(10.0));
    let mut sequence = SequenceSession::open(&file, &f.core.session_environment()).unwrap();
    let length = sequence.length();
    f.core
        .playback
        .play(&mut sequence, TimeSpan::ZERO, length, false);
    let scheduled = f.core.playback.start_named_context(
        "Editor",
        "Editor.tim",
        sequence.schedule(),
        None,
        sequence.length(),
    );
    f.core.live.ensure(&f.core, LIVE);
    f.core.live.ensure(&f.core, "State Preview");
    assert_eq!(f.api.context_states().len(), 3);
    assert!(!f
        .api
        .context_states()
        .iter()
        .any(|s| s["Sequence"]["Name"] == LIVE));
    f.call(Action::PauseSequence, &[("Name", "Editor")]);
    assert_eq!(f.core.playback.transport(), Transport::Paused);
    assert_eq!(scheduled.transport(), Transport::Paused);
    f.call(Action::StopSequence, &[("Name", "Editor")]);
    assert!(scheduled.is_finished());
    assert_eq!(f.core.playback.transport(), Transport::Stopped);
}
#[test]
fn controller_queries_missing_ids_and_save_do_not_open_hardware() {
    let f = Fixture::new();
    assert_eq!(f.call(Action::Controllers, &[]), json!([]));
    assert_eq!(
        f.api
            .invoke(Action::Controller, form(&[("id", "bad")]))
            .unwrap(),
        ApiResponse::NoContent
    );
    assert_eq!(
        f.call(Action::SetControllerState, &[("id", "bad")]),
        status(Some("Controller not found.".into()), vec![], false)
    );
    assert_eq!(
        f.call(Action::SetAllControllersState, &[("isRunning", "true")])["Message"],
        "All controllers state Changed."
    );
    assert!(!f.core.playback.hardware_allowed());
    assert_eq!(
        f.call(Action::SaveSystem, &[])["Message"],
        "Save Successful"
    );
    assert!(f
        .core
        .paths
        .system_data_dir()
        .join("SystemConfig.xml")
        .exists());
}
#[test]
fn timespan_wire_format_and_weak_profile_lifetime() {
    assert_eq!(time_text(TimeSpan::ZERO), "00:00:00");
    assert_eq!(time_text(TimeSpan::from_ms_i(1234)), "00:00:01.2340000");
    assert_eq!(time_text(TimeSpan::from_secs(90061.0)), "1.01:01:01");
    let api = {
        let f = Fixture::new();
        SessionApi::new(&f.core)
    };
    assert!(api.context_states().is_empty());
    assert!(matches!(
        api.invoke(Action::Elements, form(&[])),
        Err(ApiError::Internal(_))
    ));
}

#[test]
fn element_and_generic_effect_commands_reach_the_shared_mixer_and_clear() {
    let f = Fixture::new();
    let recorded = Arc::new(Mutex::new(Vec::new()));
    f.core
        .playback
        .add_sink(Box::new(Recorder(recorded.clone())));
    let id = f.other.to_string();
    f.call(
        Action::ElementOn,
        &[("id", &id), ("intensity", "100"), ("color", "#0000FF")],
    );
    assert!(wait(|| colors(&recorded).contains(&Color::BLUE)));
    f.call(Action::ElementOff, &[("id", &id)]);
    assert!(wait(|| colors(&recorded).is_empty()));
    f.call(
        Action::ElementEffect,
        &[
            ("Id", &id),
            ("EffectName", "Set Level"),
            ("Duration", "80"),
            ("Color[#00FF00]", "0"),
            ("Options[IntensityLevel]", "1"),
        ],
    );
    assert!(wait(|| colors(&recorded).contains(&Color::LIME)));
    assert!(wait(|| colors(&recorded).is_empty()));
}

// Deterministic cooperative renderer exercises the real API rendering/publishing path,
// without depending on machine speed or requesting a huge pixel allocation.
static SLOW_EFFECT: EffectDescriptor = EffectDescriptor {
    type_id: Uuid::nil(),
    name: "Slow test effect",
    group: vixen_effects::framework::EffectGroup::Basic,
    author: "",
    description: "",
    version: "",
    data_type: "",
    data_ns: "",
    info_link: "",
    information: "",
    icon: "",
    params: &[],
    render: |context, _| {
        while !context.cancelled() {
            std::thread::sleep(Duration::from_millis(5));
        }
        vixen_core::EffectIntents::new()
    },
    visibility: None,
    param_changed: None,
    param_changed_ctx: None,
    targets_changed: None,
    validate: None,
    force_visual: false,
    force_visual_when: None,
    visual: None,
    selection: None,
    supports_location: false,
    ported: true,
};

#[test]
fn remote_render_work_is_bounded_before_allocation() {
    assert!(check_render_work(16_000, 50_000, 50).is_ok());
    assert!(check_render_work(16_000, 50_001, 50).is_err());
    assert!(check_render_work(usize::MAX, 2, 1).is_err());
    assert!(check_render_work(1, u64::MAX, 2).is_err());
    let f = Fixture::new();
    let error = f
        .api
        .invoke(
            Action::ElementEffect,
            form(&[
                ("Id", &f.other.to_string()),
                ("EffectName", "Set Level"),
                ("Duration", "2147483647"),
                ("Color[#FF0000]", "0"),
                ("Options[IntensityLevel]", "1"),
            ]),
        )
        .unwrap_err();
    assert!(
        matches!(error, ApiError::BadRequest(ref message) if message.contains("remote render limit"))
    );
    assert!(f.core.playback.context_states().is_empty());
    // Ordinary indefinite On remains sparse and supported.
    assert!(f.call(
        Action::ElementOn,
        &[("Id", &f.other.to_string()), ("intensity", "100")]
    )["IsSuccessful"]
        .as_bool()
        .unwrap());
}

#[test]
fn stopping_the_server_cancels_a_long_effect_render_and_joins_the_request() {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::sync::atomic::AtomicUsize;
    use std::time::Instant;
    struct TrackedApi {
        api: SessionApi,
        entered: Arc<AtomicUsize>,
    }
    impl WebApi for TrackedApi {
        fn invoke(&self, action: Action, input: ApiInput) -> Result<ApiResponse, ApiError> {
            self.api.invoke(action, input)
        }
        fn invoke_cancellable(
            &self,
            action: Action,
            input: ApiInput,
            stopping: &AtomicBool,
        ) -> Result<ApiResponse, ApiError> {
            let _ = action;
            let core = self.api.core()?;
            let id = guid(input.get("Id"));
            self.entered.fetch_add(1, Ordering::Release);
            render_effect(
                &core,
                id,
                &SLOW_EFFECT,
                SLOW_EFFECT.new_params(),
                TimeSpan::from_secs(1.0),
                stopping,
            )?;
            Ok(ApiResponse::Json(json!({})))
        }
        fn context_states(&self) -> Vec<Value> {
            self.api.context_states()
        }
    }
    let f = Fixture::new();
    let entered = Arc::new(AtomicUsize::new(0));
    let mut server = vixen_web::Server::start(
        "127.0.0.1:0".parse().unwrap(),
        f.core.paths.root().to_path_buf(),
        Arc::new(TrackedApi {
            api: SessionApi::new(&f.core),
            entered: entered.clone(),
        }),
        Arc::new(|_| {}),
    )
    .unwrap();
    let address = server.address();
    let id = f.other;
    let client = std::thread::spawn(move || {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let body = format!("Id={id}");
        write!(stream,"POST /api/element/effect HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",body.len()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    });
    assert!(wait(|| entered.load(Ordering::Acquire) > 0));
    std::thread::sleep(Duration::from_millis(30));
    assert!(
        !client.is_finished(),
        "the long effect must still be rendering before shutdown"
    );
    let started = Instant::now();
    server.stop();
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(client.join().unwrap().starts_with("HTTP/1.1 500"));
    assert!(f.api.context_states().is_empty());
    assert!(
        f.core.playback.context_states().is_empty(),
        "cancelled rendering must publish no live context"
    );
}

#[test]
fn listed_sequences_are_playable_when_profile_was_opened_with_a_relative_path() {
    let root = PathBuf::from("target").join(format!("web-relative-{}", Uuid::new_v4()));
    let paths = vixen_io::DataPaths::new(root.clone());
    paths.ensure_directories().unwrap();
    vixen_io::save_system(&Default::default(), &paths).unwrap();
    let core = Arc::new(AppCore::open(root.clone(), true).unwrap());
    let mut sequence = SequenceSession::new_untitled(&core.session_environment());
    sequence.set_length(TimeSpan::from_secs(60.0));
    sequence
        .save(Some(&paths.sequence_dir().join("Relative.tim")))
        .unwrap();
    let api = SessionApi::new(&core);
    let ApiResponse::Json(rows) = api.invoke(Action::Sequences, form(&[])).unwrap() else {
        panic!()
    };
    let file = rows[0]["FileName"].as_str().unwrap();
    assert!(Path::new(file).is_absolute());
    let ApiResponse::Json(status) = api
        .invoke(
            Action::PlaySequence,
            form(&[("Name", "Relative"), ("FileName", file)]),
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(status["State"], 1);
    core.playback
        .control_contexts("Relative", Transport::Stopped);
    drop(sequence);
    drop(api);
    drop(core);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn omitted_effect_dictionaries_preserve_internal_error_status() {
    let f = Fixture::new();
    for fields in [
        vec![("EffectName", "Set Level"), ("Color[#FF0000]", "0")],
        vec![
            ("EffectName", "Set Level"),
            ("Options[IntensityLevel]", "1"),
        ],
        vec![
            ("EffectName", "Pulse"),
            ("Color[#FF0000]", "0"),
            ("Options[unused]", ""),
        ],
    ] {
        let mut input = form(&fields);
        input.fields.push(("Id".into(), f.other.to_string()));
        assert!(matches!(
            f.api.invoke(Action::ElementEffect, input),
            Err(ApiError::Internal(_))
        ));
    }
    assert!(f.core.playback.context_states().is_empty());
}
