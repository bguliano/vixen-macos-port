//! Private, silent, loopback-only verification of the unchanged production engine.
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    net::{Ipv4Addr, UdpSocket},
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use uuid::Uuid;
use vixen_core::{Color, TimeSpan};
use vixen_effects::framework::ParamValue;
use vixen_engine::{ElementMap, Transport};
use vixen_io::{Channel, Controller, ControllerOutput, DataPaths, ModuleEntry, Patch, SystemData};
use vixen_output::modules::e131::{E131Config, UniverseEntry};
use vixen_output::{ids, ControllerConfig, ModuleIds, ModuleStatus};
use vixen_session::{
    playback::ShowPlayback,
    sequence::{SequenceSession, SessionEnvironment},
};
const CID: Uuid = uuid::uuid!("00112233-4455-6677-8899-aabbccddeeff");
const CTRL: Uuid = uuid::uuid!("a0000000-0000-0000-0000-000000000001");
const LIT: [u8; 3] = [51, 102, 204];
const DARK: [u8; 3] = [0, 0, 0];
const SECONDS: u64 = 1200;
type Res<T = ()> = Result<T, String>;
fn need(ok: bool, msg: impl Into<String>) -> Res {
    if ok {
        Ok(())
    } else {
        Err(msg.into())
    }
}
fn emit(path: &Path, value: &Value) -> Res {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    fs::rename(tmp, path).map_err(|e| e.to_string())
}
fn env_guard(out: &Path) -> Res {
    need(
        out.is_absolute() && out.is_dir(),
        "Absolute prepared output required",
    )?;
    need(
        std::env::var_os("VIXEN_UI_TEST").is_none(),
        "UI_TEST forbidden",
    )?;
    need(
        std::env::var_os("VIXEN_SETTINGS_DIR").as_deref() == Some(out.join("settings").as_os_str()),
        "Private settings required",
    )?;
    need(
        fs::read_dir(out.join("settings"))
            .map_err(|e| e.to_string())?
            .next()
            .is_none(),
        "Settings must remain empty",
    )?;
    let p = vixen_io::load_audio_preferences(&out.join("settings"));
    need(
        p.sound_out_device_id == "-0",
        "No device enumeration allowed",
    )
}
fn fixture(out: &Path) -> Res {
    env_guard(out)?;
    let paths = DataPaths::new(out.join("profile"));
    need(!paths.root().exists(), "Fresh profile only")?;
    paths.ensure_directories().map_err(|e| e.to_string())?;
    let mut system = SystemData::default();
    system.default_update_interval = 10;
    let mut targets = vec![];
    for i in 0..3 {
        let name = format!("Loopback {}", i + 1);
        let n = system.tree.add_element(&name);
        let node = system.tree.node(n);
        let element = node.element_id.unwrap();
        targets.push((node.id, name.clone()));
        system.channels.push(Channel {
            id: element,
            name: name.clone(),
        });
        system.patches.push(Patch {
            component_id: Uuid::from_u128(100 + i),
            source_id: Some(element),
            output: 0,
        });
    }
    system.controllers.push(Controller {
        name: "Private loopback endurance".into(),
        hardware_type_id: ids::E131,
        hardware_instance_id: CID,
        id: CTRL,
        outputs: (0..3)
            .map(|i| ControllerOutput {
                id: Uuid::from_u128(100 + i),
                name: format!("Slot{}", i + 1),
            })
            .collect(),
    });
    let config = E131Config {
        ids: ModuleIds::new(ids::E131, CID),
        unicast: Some("127.0.0.1".into()),
        multicast: None,
        auto_populate: false,
        output_count: 3,
        universes: Some(vec![UniverseEntry::new(true, 63999, 0, 3)]),
        ..Default::default()
    };
    system.module_store.set_instance(ModuleEntry::new(
        "VixenModules.Output.E131.E131ModuleDataModel, E131",
        &ids::E131,
        &CID,
        "E131",
        config.to_xml(),
    ));
    vixen_io::save_system(&system, &paths).map_err(|e| e.to_string())?;
    let env = SessionEnvironment {
        video_cache: None,
        tree: Arc::new(system.tree.clone()),
        store: Arc::new(system.module_store.clone()),
        module_data_root: Some(paths.module_data_dir()),
        frame_ms: 10,
        media_dir: paths.media_dir(),
    };
    let mut sequence = SequenceSession::new_untitled(&env);
    sequence.set_length(TimeSpan::from_ms_i(2000));
    for (i, (node, name)) in targets.iter().enumerate() {
        let id = sequence
            .add_new_effect(
                uuid::uuid!("32cff8e0-5b10-4466-a093-0d232c55aac0"),
                *node,
                name,
                TimeSpan::ZERO,
                TimeSpan::from_ms_i(1000),
            )
            .ok_or("Effect not added")?;
        sequence.set_param(id, "IntensityLevel", ParamValue::Double([0.2, 0.4, 0.8][i]));
        sequence.set_param(
            id,
            "Color",
            ParamValue::Rgb(vixen_core::color::Rgb::from_color(Color::WHITE)),
        );
    }
    sequence.wait_for_renders();
    need(sequence.take_problems().is_empty(), "Render problems")?;
    need(sequence.audio().is_none(), "Audio forbidden")?;
    sequence
        .save(Some(&paths.sequence_dir().join("Endurance.tim")))
        .map_err(|e| e.to_string())?;
    emit(
        &out.join("fixture.json"),
        &json!({"silent":true,"audio":false,"previews":0,"servicesStarted":false,"webStarted":false,"destination":"127.0.0.1:5568","universe":63999,"outputs":3,"lit":LIT,"dark":DARK,"lengthMs":2000,"litSpanMs":1000,"frameMs":10}),
    )
}
fn expected(seq: u8, slots: &[u8]) -> Vec<u8> {
    // Independent wire layout, copied from the accepted handwritten packet contract, no production encoder call.
    let n = slots.len();
    let dmp = 11 + n;
    let framing = 77 + dmp;
    let root = 22 + framing;
    let mut p = vec![0, 16, 0, 0];
    p.extend_from_slice(b"ASC-E1.17\0\0\0");
    p.extend_from_slice(&(0x7000u16 | root as u16).to_be_bytes());
    p.extend_from_slice(&[0, 0, 0, 4]);
    p.extend_from_slice(&[
        0x33, 0x22, 0x11, 0, 0x55, 0x44, 0x77, 0x66, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
    ]);
    p.extend_from_slice(&(0x7000u16 | framing as u16).to_be_bytes());
    p.extend_from_slice(&[0, 0, 0, 2]);
    let mut name = [0; 64];
    name[..7].copy_from_slice(b"Vixen 3");
    p.extend_from_slice(&name);
    p.push(100);
    p.extend_from_slice(&[0, 0, seq, 0]);
    p.extend_from_slice(&63999u16.to_be_bytes());
    p.extend_from_slice(&(0x7000u16 | dmp as u16).to_be_bytes());
    p.extend_from_slice(&[2, 0xa1, 0, 0, 0, 1]);
    p.extend_from_slice(&((n + 1) as u16).to_be_bytes());
    p.push(0);
    p.extend_from_slice(slots);
    p
}
fn validate_packet(p: &[u8], previous: Option<u8>) -> Res<[u8; 3]> {
    need(p.len() == 129, "Wrong packet length")?;
    let slots: [u8; 3] = p[126..129].try_into().unwrap();
    need(
        slots == LIT || slots == DARK,
        format!("Unexpected slots {slots:?}"),
    )?;
    need(
        p == expected(p[111], &slots),
        "Packet bytes differ from handwritten E1.31 contract",
    )?;
    if let Some(last) = previous {
        need(
            p[111] == last.wrapping_add(1),
            format!("Sequence discontinuity {last} -> {}", p[111]),
        )?;
    }
    Ok(slots)
}
struct Receiver {
    socket: UdpSocket,
    count: u64,
    lit: u64,
    dark: u64,
    previous: Option<u8>,
    last: [u8; 3],
}
impl Receiver {
    fn drain(&mut self) -> Res {
        let mut data = [0; 2048];
        for _ in 0..10000 {
            match self.socket.recv_from(&mut data) {
                Ok((n, source)) => {
                    need(source.ip() == Ipv4Addr::LOCALHOST, "Nonloopback sender")?;
                    let slots = validate_packet(&data[..n], self.previous)?;
                    self.previous = Some(data[111]);
                    self.last = slots;
                    self.count += 1;
                    if slots == LIT {
                        self.lit += 1
                    } else {
                        self.dark += 1
                    };
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(()),
                Err(e) => return Err(e.to_string()),
            }
        }
        Err("Receiver could not drain bounded queue".into())
    }
    fn wait(&mut self, seconds: f64, mut check: impl FnMut() -> Res) -> Res {
        let deadline = Instant::now() + Duration::from_secs_f64(seconds);
        while Instant::now() < deadline {
            self.drain()?;
            check()?;
            std::thread::sleep(Duration::from_millis(5));
        }
        self.drain()?;
        check()
    }
    fn plateau(&mut self, value: [u8; 3]) -> Res {
        let end = Instant::now() + Duration::from_secs(2);
        loop {
            self.drain()?;
            if self.last == value && self.count > 0 {
                break;
            }
            need(Instant::now() < end, "Expected plateau did not arrive")?;
            std::thread::sleep(Duration::from_millis(5));
        }
        let start = self.count;
        let opposite = if value == LIT { self.dark } else { self.lit };
        self.wait(0.15, || Ok(()))?;
        let after = if value == LIT { self.dark } else { self.lit };
        need(
            self.count > start && self.last == value && opposite == after,
            "Plateau absent or interrupted",
        )
    }
}
struct Owned {
    playback: Arc<ShowPlayback>,
}
impl Drop for Owned {
    fn drop(&mut self) {
        self.playback.stop_all_contexts();
        self.playback.stop();
        self.playback.close_execution();
    }
}
fn config_guard(playback: &ShowPlayback) -> Res {
    playback.with_output(|graph, runner| {
        need(
            graph.warnings().is_empty(),
            format!("Graph warnings {:?}", graph.warnings()),
        )?;
        need(
            runner.controller_count() == 1 && runner.controller_id(0) == Some(CTRL),
            "Only owned controller allowed",
        )?;
        let Some(ControllerConfig::E131(c)) = runner.config(0) else {
            return Err("Only E131 allowed".into());
        };
        need(
            c.unicast.as_deref() == Some("127.0.0.1")
                && c.multicast.is_none()
                && c.output_count == 3
                && !c.blind
                && !c.warnings
                && !c.statistics,
            "Loopback configuration mismatch",
        )?;
        let u = c.universes.ok_or("Missing universes")?;
        need(
            u.len() == 1
                && u[0].active
                && u[0].universe == 63999
                && u[0].start == 0
                && u[0].size == 3
                && u[0].unicast.is_none()
                && u[0].multicast.is_none(),
            "Unexpected universe routing",
        )
    })
}
fn run(out: &Path) -> Res<Value> {
    env_guard(out)?;
    let _claim = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out.join("run.claim"))
        .map_err(|e| e.to_string())?;
    let paths = DataPaths::new(out.join("profile"));
    let system = vixen_io::load_system(&paths).map_err(|e| e.to_string())?;
    need(
        system.controllers.len() == 1
            && system.previews.is_empty()
            && system.channels.len() == 3
            && system.filters.is_empty()
            && system.disabled_devices.is_empty()
            && system.default_update_interval == 10,
        "Fixture configuration mismatch",
    )?;
    let env = SessionEnvironment {
        video_cache: None,
        tree: Arc::new(system.tree.clone()),
        store: Arc::new(system.module_store.clone()),
        module_data_root: Some(paths.module_data_dir()),
        frame_ms: 10,
        media_dir: paths.media_dir(),
    };
    let mut sequence = SequenceSession::open(&paths.sequence_dir().join("Endurance.tim"), &env)
        .map_err(|e| e.to_string())?;
    sequence.wait_for_renders();
    need(
        sequence.take_problems().is_empty()
            && sequence.audio().is_none()
            && sequence.length() == TimeSpan::from_ms_i(2000),
        "Sequence mismatch",
    )?;
    // Bind exclusively before the only production controller may start. No reuse, multicast, DNS or nonloopback endpoint.
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 5568))
        .map_err(|e| format!("Private loopback receiver unavailable: {e}"))?;
    socket.set_nonblocking(true).map_err(|e| e.to_string())?;
    let mut rx = Receiver {
        socket,
        count: 0,
        lit: 0,
        dark: 0,
        previous: None,
        last: DARK,
    };
    let playback = Arc::new(ShowPlayback::new(
        &system,
        Arc::new(ElementMap::from_tree(&system.tree)),
        false,
    ));
    let owned = Owned {
        playback: playback.clone(),
    };
    config_guard(&playback)?;
    let started = Instant::now();
    let mut cycles = 0;
    let mut wraps = 0;
    let mut events = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(out.join("cycles.jsonl"))
        .map_err(|e| e.to_string())?;
    while started.elapsed() < Duration::from_secs(SECONDS) {
        need(
            started.elapsed() < Duration::from_secs(1230),
            "Internal run limit",
        )?;
        config_guard(&playback)?;
        rx.previous = None;
        playback.open_execution();
        let old_lit = rx.lit;
        let old_dark = rx.dark;
        playback.play(
            &mut sequence,
            TimeSpan::ZERO,
            TimeSpan::from_ms_i(2000),
            true,
        );
        let mut last = TimeSpan::ZERO;
        let mut looped = false;
        rx.wait(3.25, || {
            let p = playback.position();
            if p < last {
                looped = true;
            }
            last = p;
            need(
                playback.transport() == Transport::Playing,
                "Loop unexpectedly stopped",
            )
        })?;
        need(
            looped && rx.lit > old_lit && rx.dark > old_dark,
            "Loop failed to wrap and produce both plateaus",
        )?;
        wraps += 1;
        playback.pause();
        need(playback.transport() == Transport::Paused, "Pause failed")?;
        playback.seek(TimeSpan::from_ms_i(250));
        rx.plateau(LIT)?;
        let paused = playback.position();
        rx.wait(0.1, || {
            need(playback.position() == paused, "Paused playhead moved")
        })?;
        playback.seek(TimeSpan::from_ms_i(1500));
        rx.plateau(DARK)?;
        playback.resume();
        rx.wait(0.2, || {
            need(playback.transport() == Transport::Playing, "Resume failed")
        })?;
        playback.stop();
        rx.plateau(DARK)?;
        let context = playback.start_named_context(
            "Natural context",
            "Endurance.tim",
            sequence.schedule(),
            None,
            TimeSpan::from_ms_i(400),
        );
        let weak = Arc::downgrade(&context);
        rx.plateau(LIT)?;
        let end = Instant::now() + Duration::from_secs(2);
        while !context.is_finished() {
            need(Instant::now() < end, "Natural context did not finish")?;
            rx.wait(0.01, || Ok(()))?;
        }
        drop(context);
        need(weak.upgrade().is_none(), "Context owner retained")?;
        rx.plateau(DARK)?;
        need(
            playback.context_states().is_empty(),
            "Finished named context retained",
        )?;
        let context = playback.start_named_context(
            "Stopped context",
            "Endurance.tim",
            sequence.schedule(),
            None,
            TimeSpan::from_ms_i(2000),
        );
        let weak = Arc::downgrade(&context);
        rx.plateau(LIT)?;
        context.stop();
        need(context.is_finished(), "Explicit context stop failed")?;
        drop(context);
        need(weak.upgrade().is_none(), "Stopped context owner retained")?;
        rx.plateau(DARK)?;
        need(
            playback.context_states().is_empty(),
            "Stopped named context retained",
        )?;
        playback.play(
            &mut sequence,
            TimeSpan::ZERO,
            TimeSpan::from_ms_i(400),
            false,
        );
        rx.plateau(LIT)?;
        let end = Instant::now() + Duration::from_secs(2);
        while playback.transport() != Transport::Stopped {
            need(Instant::now() < end, "Primary did not finish")?;
            rx.wait(0.01, || Ok(()))?;
        }
        need(
            playback.take_finished() && !playback.take_finished(),
            "Completion must be observed exactly once",
        )?;
        rx.plateau(DARK)?;
        let close = Instant::now();
        playback.close_execution();
        let close_ms = close.elapsed().as_millis();
        need(close_ms < 5000, "Controller shutdown exceeded bound")?;
        rx.drain()?;
        let count = rx.count;
        rx.wait(0.12, || Ok(()))?;
        need(count == rx.count, "Packets after closed engine")?;
        let (updates, errors, panics, sent, replaced) = playback.with_output(|_, r| {
            let s = r.stats(0);
            (
                s.updates,
                s.errors,
                s.panics,
                s.packets_sent,
                s.frames_replaced,
            )
        });
        need(errors == 0 && panics == 0, "Controller error/panic")?;
        need(
            playback.with_output(|_, r| !r.is_running(0) && r.status(0) == ModuleStatus::Stopped),
            "Controller did not stop cleanly",
        )?;
        need(
            sent == rx.count,
            format!("Sent/received mismatch {sent}/{}", rx.count),
        )?;
        cycles += 1;
        let s = playback.stats();
        let row = json!({"cycle":cycles,"elapsedSeconds":started.elapsed().as_secs_f64(),"loopWraps":wraps,"packets":rx.count,"litPackets":rx.lit,"darkPackets":rx.dark,"controllerUpdates":updates,"packetsSent":sent,"replacedFramesDiagnostic":replaced,"engineFrames":s.frames,"engineLateFramesDiagnostic":s.late_frames,"engineMaxFrameUsDiagnostic":s.max_frame_us,"shutdownMs":close_ms,"errors":errors,"panics":panics,"namedContexts":playback.context_states().len()});
        writeln!(events, "{}", row).map_err(|e| e.to_string())?;
        events.sync_data().map_err(|e| e.to_string())?;
        emit(&out.join("progress.json"), &row)?;
    }
    need(
        started.elapsed().as_secs() >= SECONDS && cycles >= 100,
        "Insufficient endurance coverage",
    )?;
    drop(sequence);
    let weak_playback = Arc::downgrade(&playback);
    drop(owned);
    drop(playback);
    need(
        weak_playback.upgrade().is_none(),
        "Playback owner retained after teardown",
    )?;
    rx.drain()?;
    let count = rx.count;
    rx.wait(0.12, || Ok(()))?;
    need(rx.count == count, "Packet after object destruction")?;
    Ok(
        json!({"passed":true,"durationSeconds":started.elapsed().as_secs_f64(),"cycles":cycles,"loopWraps":wraps,"packets":count,"litPackets":rx.lit,"darkPackets":rx.dark,"engineObjectsDropped":true,"controllerStopped":true,"loopbackOnly":true,"audioOpened":false,"nativeUI":false,"hardwareVerification":false}),
    )
}
fn controls() -> Res<Value> {
    let p = expected(0, &LIT);
    need(validate_packet(&p, None).is_ok(), "Positive packet")?;
    let mut bad = p.clone();
    bad[20] ^= 1;
    need(
        validate_packet(&bad, None).is_err(),
        "Wrong header accepted",
    )?;
    need(
        validate_packet(&p, Some(0)).is_err(),
        "Duplicate sequence accepted",
    )?;
    need(
        validate_packet(&expected(0, &LIT), Some(255)).is_ok(),
        "Sequence wrap rejected",
    )?;
    need(
        validate_packet(&expected(1, &[1, 2, 3]), None).is_err(),
        "Wrong payload accepted",
    )?;
    need(
        validate_packet(&p[..128], None).is_err(),
        "Truncated packet accepted",
    )?;
    Ok(json!({"passed":true,"checks":6,"pureOnly":true}))
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() == 2 && args[1] == "controls" {
        println!("{}", controls().unwrap());
        return;
    }
    if args.len() != 3 {
        eprintln!("prepare|run ABSOLUTE_OUTPUT");
        std::process::exit(2)
    }
    let out = Path::new(&args[2]);
    let result = if args[1] == "prepare" {
        fixture(out).map(|_| json!({"passed":true,"prepared":true}))
    } else if args[1] == "run" {
        let watch = out.to_path_buf();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(1240));
            let _ = emit(
                &watch.join("watchdog.json"),
                &json!({"passed":false,"error":"1240-second independent deadline"}),
            );
            std::process::exit(97);
        });
        run(out)
    } else {
        Err("Unknown command".into())
    };
    let ok = result.is_ok();
    let value = result.unwrap_or_else(|e| json!({"passed":false,"error":e}));
    if args[1] == "run" {
        if let Err(e) = emit(&out.join("native-result.json"), &value) {
            eprintln!("Result write: {e}");
            std::process::exit(2)
        }
    }
    println!("{value}");
    if !ok {
        std::process::exit(1)
    }
}
