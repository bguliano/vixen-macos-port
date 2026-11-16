use super::*;
use std::net::TcpListener;
use vixen_io::DataPaths;

fn root() -> PathBuf { std::env::temp_dir().join(format!("vixen-web-test-{}", Uuid::new_v4())) }
fn setup(under_test: bool) -> (Arc<AppCore>, PathBuf) {
    let root = root();
    let paths = DataPaths::new(root.clone());
    paths.ensure_directories().unwrap();
    let wwwroot = root.join("wwwroot");
    std::fs::create_dir_all(&wwwroot).unwrap();
    std::fs::write(wwwroot.join("index.htm"), "test page").unwrap();
    let core = AppCore::assemble(paths, Default::default(), Default::default(), None,
        crate::boot::LaunchOptions { muted: true, under_test, ..Default::default() }, None);
    (Arc::new(core), wwwroot)
}
fn service(core: &Arc<AppCore>, root: PathBuf) -> Arc<WebService> {
    WebService::new(core, root, Some("127.0.0.1:0".parse().unwrap()))
}

#[test]
fn settings_preserve_unknown_members_and_clamp_old_ports() {
    let mut store = ModuleStore::new();
    assert_eq!(load_settings(&store), Settings { port: 8888, enabled: true });
    let mut data = encode(Settings::default());
    data.child_mut("HttpPort").unwrap().children = vec![vixen_xml::XNode::Text("99999".into())];
    data.push(XElement::with_text(NS, "Future", "keep"));
    store.set_type_data(ModuleEntry::new("VixenModules.App.WebServer.Data, WebServer", &MODULE_TYPE, &MODULE_TYPE, "Module.App.WebServer", data));
    assert_eq!(load_settings(&store).port, 64000);
    store_settings(&mut store, Settings { port: 9876, enabled: false });
    assert_eq!(load_settings(&store), Settings { port: 9876, enabled: false });
    assert_eq!(store.type_data(MODULE_TYPE).unwrap().data().unwrap().child_text("Future").as_deref(), Some("keep"));
}

#[test]
fn ui_tests_never_start_a_server_even_after_settings_or_execution_changes() {
    let (core, wwwroot) = setup(true);
    let service = service(&core, wwwroot);
    service.start(&core).unwrap();
    service.save(Settings { port: 9999, enabled: true }).unwrap();
    core.execution().open();
    assert!(!service.is_running());
    assert!(core.playback.context_states().is_empty());
    assert!(core.paths.root().join("SystemData/ModuleStore.xml").is_file());
}

#[test]
fn bind_failure_is_reported_without_a_live_context_or_app_failure() {
    let (core, wwwroot) = setup(false);
    let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
    let service = WebService::new(&core, wwwroot, Some(occupied.local_addr().unwrap()));
    assert!(service.start(&core).is_err());
    assert!(service.error().is_some());
    assert!(!service.is_running());
    assert!(core.playback.context_states().is_empty());
    drop(occupied);
    service.start(&core).unwrap();
    assert!(service.is_running());
    assert!(service.error().is_none());
    service.stop(&core);
}

#[test]
fn disabling_stops_listener_and_live_context_without_stopping_a_sequence() {
    let (core, wwwroot) = setup(false);
    let service = service(&core, wwwroot);
    service.start(&core).unwrap();
    let address = service.address().unwrap();
    let sequence = crate::sequence::SequenceSession::new_untitled(&core.session_environment());
    let playing = core.playback.start_named_context("Song", "Song.tim", sequence.schedule(), None, vixen_core::TimeSpan::from_secs(60.0));
    service.save(Settings { port: 8888, enabled: false }).unwrap();
    assert!(!service.is_running());
    assert!(!playing.is_finished());
    assert_eq!(core.playback.context_states().len(), 1);
    let _rebound = TcpListener::bind(address).unwrap();
}

#[test]
fn reopening_execution_recreates_the_live_context_without_restarting_the_listener() {
    let (core, wwwroot) = setup(false);
    let service = service(&core, wwwroot);
    core.execution().open();
    service.start(&core).unwrap();
    let address = service.address();
    assert_eq!(core.playback.context_states()[0].name, "Web Server");
    core.execution().close();
    assert!(core.playback.context_states().is_empty());
    core.execution().open();
    assert_eq!(core.playback.context_states()[0].name, "Web Server");
    assert_eq!(service.address(), address);
    service.stop(&core);
}

#[test]
fn reload_reads_new_settings_and_shutdown_joins_the_server() {
    let (core, wwwroot) = setup(false);
    let service = service(&core, wwwroot);
    core.add_service(service.clone());
    core.start_services();
    assert!(service.is_running());
    store_settings(&mut core.system.write().module_store, Settings { port: 8888, enabled: false });
    service.reloaded(&core);
    assert!(!service.is_running());
    service.save(Settings::default()).unwrap();
    let address = service.address().unwrap();
    core.shutdown();
    assert!(!service.is_running());
    let _rebound = TcpListener::bind(address).unwrap();
}

#[test]
fn failed_settings_write_restores_the_store_and_does_not_apply() {
    let (core, wwwroot) = setup(true);
    let service = service(&core, wwwroot);
    let directory = core.paths.root().join("SystemData/ModuleStore.xml");
    std::fs::create_dir(&directory).unwrap();
    assert!(service.save(Settings { port: 1234, enabled: false }).is_err());
    assert_eq!(service.settings(), Settings::default());
    assert!(core.system.read().module_store.type_data(MODULE_TYPE).is_none());
    assert!(!core.is_saving());
}

#[test]
fn shutdown_clears_contexts_started_by_an_in_flight_service_request() {
    struct LateRequest;
    impl AppService for LateRequest {
        fn name(&self) -> &str { "late request" }
        fn start(&self, _: &AppCore) -> Result<(), String> { Ok(()) }
        fn stop(&self, core: &AppCore) { core.live.ensure(core, "Late Request"); }
    }
    let (core, _) = setup(false);
    core.add_service(Arc::new(LateRequest));
    core.start_services();
    core.execution().open();
    core.shutdown();
    assert!(core.playback.context_states().is_empty());
}
