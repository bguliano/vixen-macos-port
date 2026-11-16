use super::profile::*;
use super::*;
use vixen_io::system::{Controller, ControllerOutput};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vixen-wizard-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn controller(id: u128, name: &str, outputs: usize) -> Controller {
    Controller {
        name: name.to_string(),
        hardware_type_id: Uuid::nil(),
        hardware_instance_id: Uuid::from_u128(id + 100),
        id: Uuid::from_u128(id),
        outputs: (0..outputs).map(|i| ControllerOutput { name: format!("Output {}", i + 1), id: Uuid::new_v4() }).collect(),
    }
}

fn system() -> SystemData {
    let mut system = SystemData::default();
    system.controllers = vec![controller(1, "Arduino", 16), controller(2, "Light Strip Controller", 3351)];
    system
}

fn live(system: &SystemData) -> Vec<ControllerInfo> {
    crate::export::default_controller_info(system)
}

const REAL_STORE: &str = "/Users/developer/Documents/Vixen 3/SystemData/ModuleStore.xml";

#[test]
fn the_owners_profile_loads_and_saves_back_unchanged() {
    let Ok(store) = vixen_io::load_module_store_file(Path::new(REAL_STORE)) else { return };
    let profiles = load_profiles(&store);
    assert_eq!(profiles.len(), 1);
    let p = &profiles[0];
    assert_eq!(p.name_text(), "Default");
    assert_eq!(p.id, uuid::uuid!("d997511a-3c1e-4491-9f84-4fd9ddb4bec9"));
    assert_eq!(p.format_text(), "Falcon Player Sequence");
    assert!(p.is_falcon_format() && !p.is_falcon_2x_format());
    assert_eq!(p.interval, 50);
    assert!(p.include_audio && p.rename_audio && p.backup_universe_file);
    assert!(!p.create_universe_file && !p.enable_compression && !p.fpp_direct_upload);
    assert_eq!(p.sequence_files.len(), 7);
    assert!(p.sequence_files[4].ends_with("Sandstorm.tim"));
    assert_eq!(p.falcon_output_folder.as_deref(), Some("C:\\Users\\olduser\\Desktop\\Stuff\\Christmas Tree Lights 2018\\BACKUP"));
    assert_eq!(p.controllers.as_ref().unwrap(), &[ProfileController { id: uuid::uuid!("970a3e5d-3764-4536-ac84-4be2bebed9d6"), index: 0, is_active: true }]);

    let original = store.type_data(MODULE_TYPE).unwrap().data().unwrap().clone();
    let mut copy = store.clone();
    save_profiles(&mut copy, &profiles);
    let saved = copy.type_data(MODULE_TYPE).unwrap().data().unwrap();
    assert_eq!(saved.first_difference(&original, String::new()), None, "untouched, it goes back as it came");

    // One change touches one member.
    let mut changed = profiles.clone();
    changed[0].interval = 25;
    let mut copy = store.clone();
    save_profiles(&mut copy, &changed);
    let reloaded = load_profiles(&copy);
    assert_eq!(reloaded[0].interval, 25);
    assert_eq!(reloaded[0].sequence_files, p.sequence_files);
    assert_eq!(reloaded[0].audio_output_folder, p.audio_output_folder);
}

#[test]
fn a_new_profile_writes_the_data_contract_shape() {
    let root = temp_dir("shape");
    let mut store = vixen_io::ModuleStore::new();
    let profile = default_profile("Default", &root, &system());
    save_profiles(&mut store, std::slice::from_ref(&profile));
    let entry = store.type_data(MODULE_TYPE).unwrap();
    assert_eq!(entry.data_model_type(), Some("VixenModules.App.ExportWizard.BulkExportWizardData, ExportWizard"));
    let data = entry.data().unwrap();
    let members: Vec<&str> = data.child("Profiles").unwrap().child("ExportProfile").unwrap().elements().map(|e| e.local.as_str()).collect();
    assert_eq!(
        members,
        ["AudioOutputFolder", "Controllers", "EnableCompression", "FalconOutputFolder", "Format", "Id", "Interval", "Name", "OutputFolder", "SequenceFiles"]
    );
    let p = data.child("Profiles").unwrap().child("ExportProfile").unwrap();
    assert!(p.child("FalconOutputFolder").unwrap().nil);
    assert_eq!(p.child_text("Format").as_deref(), Some("Falcon Player Sequence 2.6+"));
    assert_eq!(load_profiles(&store), vec![profile]);
}

#[test]
fn names_are_trimmed_and_must_be_new() {
    let names = ["Default", "Pi"];
    assert_eq!(validate_profile_name(&names, "  Yard ", None), Ok("Yard".to_string()));
    assert_eq!(validate_profile_name(&names, "   ", None), Err(NameError::Blank));
    assert_eq!(validate_profile_name(&names, "Pi", None), Err(NameError::Duplicate("Pi".to_string())));
    assert_eq!(validate_profile_name(&names, "pi", None), Ok("pi".to_string()), "an exact compare");
    assert_eq!(validate_profile_name(&names, "Pi", Some("Pi")), Ok("Pi".to_string()));
    assert_eq!(NameError::Blank.message(), "Profile name can not be blank.");
    assert_eq!(NameError::Duplicate("Pi".into()).message(), "A profile with the name Pi already exists.");
    assert_eq!(NameError::Duplicate("Pi".into()).title(), "Warning");
}

#[test]
fn controllers_sync_dropping_gone_ones_and_appending_new_ones_ticked() {
    let system = system();
    let mut profile = default_profile("P", Path::new("/x"), &system);
    profile.controllers = Some(vec![
        ProfileController { id: Uuid::from_u128(2), index: 0, is_active: false },
        ProfileController { id: Uuid::from_u128(9), index: 1, is_active: true },
    ]);
    synchronize_controllers(&mut profile, &system);
    assert_eq!(
        profile.controllers.unwrap(),
        [ProfileController { id: Uuid::from_u128(2), index: 0, is_active: false }, ProfileController { id: Uuid::from_u128(1), index: 1, is_active: true }]
    );
    let mut empty = default_profile("P", Path::new("/x"), &system);
    empty.controllers = None;
    synchronize_controllers(&mut empty, &system);
    assert_eq!(empty.controllers.unwrap().len(), 2);
}

#[test]
fn an_old_falcon_profile_finds_its_falcon_folder() {
    let mut p = default_profile("P", Path::new("/x"), &system());
    p.format = Some("Falcon Player Sequence".to_string());
    p.output_folder = Some("C:\\Show\\BACKUP\\sequences".to_string());
    p.falcon_output_folder = None;
    p.on_deserialized();
    assert_eq!(p.falcon_output_folder.as_deref(), Some("C:\\Show\\BACKUP"));
    p.falcon_output_folder = Some(String::new());
    p.output_folder = Some("/Volumes/fpp/sequences/".to_string());
    p.on_deserialized();
    assert_eq!(p.falcon_output_folder.as_deref(), Some("/Volumes/fpp/sequences"), "KEEP: GetParent of a trailing separator");
    let mut csv = default_profile("P", Path::new("/x"), &system());
    csv.format = Some("CSV File".to_string());
    csv.output_folder = Some("/a/sequences".to_string());
    csv.on_deserialized();
    assert_eq!(csv.falcon_output_folder, None);
}

#[test]
fn add_all_takes_both_extensions_everywhere_but_the_backups() {
    let root = temp_dir("all");
    let paths = DataPaths::new(&root);
    paths.ensure_directories().unwrap();
    for file in ["Sequence/A.tim", "Sequence/B.tim", "Sequence/A.tim_backup20241209", "Sequence/A.tim.8", "Sequence/C.TIM", "Export/Old.vix", "Sequence/auto_backup/A.tim"] {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "").unwrap();
    }
    let found: Vec<String> = all_sequence_files(&paths, true).iter().map(|p| p.strip_prefix(&root).unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(found, ["Export/Old.vix", "Sequence/A.tim", "Sequence/B.tim"]);
    assert_eq!(all_sequence_files(&paths, false).len(), 4);
}

#[test]
fn the_stages_run_in_order_with_their_rules() {
    let root = temp_dir("stages");
    let paths = DataPaths::new(&root);
    paths.ensure_directories().unwrap();
    std::fs::write(root.join("Sequence/Song.tim"), "").unwrap();
    let system = system();
    let exists = |p: &str| Path::new(p).is_dir();

    let mut wizard = ExportWizard::new(paths.clone(), Vec::new(), &system);
    assert_eq!(wizard.stage(), Stage::Config);
    assert_eq!(wizard.profile_names(), ["Default"], "none saved, so one is made");
    assert!(!wizard.can_move_previous());
    assert!(wizard.next(&system, &exists));

    assert_eq!(wizard.stage(), Stage::Sources);
    assert!(!wizard.can_move_next(&exists), "no sequences yet");
    wizard.add_sequences(&[root.join("Sequence/Song.tim"), PathBuf::from("/elsewhere/X.tim")]);
    wizard.add_sequences(&[root.join("Sequence/Song.tim")]);
    assert_eq!(wizard.sequence_files().len(), 1, "outside the folder refused, duplicate skipped");
    assert!(wizard.next(&system, &exists));

    assert_eq!(wizard.stage(), Stage::Controllers);
    let rows = wizard.controller_rows(&live(&system));
    assert_eq!(rows.iter().map(|r| (r.start, r.end)).collect::<Vec<_>>(), [(Some(1), Some(16)), (Some(17), Some(3367))]);
    wizard.set_all_controllers(false, &live(&system));
    assert!(!wizard.can_move_next(&exists), "GUARD: something must be ticked");
    assert!(wizard.enable_all_enabled() && !wizard.disable_all_enabled());
    wizard.set_controllers_active(&[1], true, &live(&system));
    assert!(wizard.can_move_next(&exists));
    wizard.move_controllers(&[1], 0, &live(&system));
    assert_eq!(wizard.controller_rows(&live(&system))[0].name, "Light Strip Controller");
    assert!(wizard.next(&system, &exists));

    assert_eq!(wizard.stage(), Stage::OutputFormat);
    assert_eq!(wizard.interval_text(), "50");
    // The default profile's folders are `<root>/Export`, which `ensure_directories` doesn't make.
    let falcon = root.join("Falcon");
    assert!(!wizard.can_move_next(&exists), "2.6+ checks the Falcon folder, which isn't set");
    std::fs::create_dir_all(&falcon).unwrap();
    wizard.set_falcon_folder(&falcon.to_string_lossy());
    assert!(wizard.can_move_next(&exists));
    assert_eq!(wizard.active().unwrap().output_folder.as_deref(), Some(falcon.join("sequences").to_str().unwrap()));
    assert_eq!(wizard.active().unwrap().audio_output_folder.as_deref(), Some(falcon.join("music").to_str().unwrap()));
    wizard.set_interval_text("abc");
    assert!(!wizard.can_move_next(&exists));
    wizard.set_interval_text("25");
    assert_eq!(wizard.active().unwrap().interval, 25);
    wizard.set_include_audio(true, true);
    assert!(wizard.active().unwrap().rename_audio, "on 2.6+ Include Audio renames too");
    assert!(wizard.next(&system, &exists));

    assert_eq!(wizard.stage(), Stage::Summary);
    let summary = wizard.summary(&live(&system)).unwrap();
    assert_eq!(summary.sequences, 1);
    assert_eq!(summary.timing, "25 ms");
    assert_eq!(summary.audio, "Rename to match sequence name.");
    assert!(summary.audio_destination.is_some() && summary.universe_destination.is_none() && summary.warning.is_none());
    wizard.set_create_universe_file(true);
    let summary = wizard.summary(&live(&system)).unwrap();
    assert_eq!(summary.universe_destination.as_deref(), Some(falcon.to_str().unwrap()));
    assert_eq!(summary.warning.as_deref(), Some(SUMMARY_WARNING), "these controllers have no universes");

    wizard.save_settings = true;
    wizard.save_name = "  ".to_string();
    assert!(!wizard.can_move_next(&exists), "GUARD: no nameless profile");
    wizard.save_name = "Pi".to_string();
    assert!(wizard.apply_save_settings());
    assert_eq!(wizard.profile_names(), ["Default", "Pi"]);
    wizard.save_name = "Default".to_string();
    assert!(wizard.apply_save_settings());
    assert_eq!(wizard.profile_names(), ["Default", "Pi"], "a name in the list replaces that profile");
    assert_eq!(wizard.profiles()[0].interval, 25);

    wizard.finish("Completed");
    assert_eq!(wizard.stage(), Stage::Finished);
    assert_eq!(wizard.final_text(), "Export Complete");
    wizard.finish("Cancelled");
    assert_eq!(wizard.final_text(), "Cancelled");
}

#[test]
fn previous_to_the_first_stage_drops_the_edits() {
    let root = temp_dir("previous");
    let paths = DataPaths::new(&root);
    let system = system();
    let mut wizard = ExportWizard::new(paths, Vec::new(), &system);
    wizard.add_profile("Second", &system).unwrap();
    assert_eq!(wizard.selected(), 1);
    assert!(wizard.next(&system, &|_| true));
    assert!(wizard.previous(&system));
    assert_eq!(wizard.selected(), 0, "KEEP: back on the first stage, the first profile is chosen again");
    assert_eq!(wizard.active().unwrap().name_text(), "Default");
}

#[test]
fn profiles_add_rename_and_delete() {
    let root = temp_dir("profiles");
    let system = system();
    let mut wizard = ExportWizard::new(DataPaths::new(&root), Vec::new(), &system);
    assert_eq!(wizard.add_profile("Default", &system), Err(NameError::Duplicate("Default".to_string())));
    wizard.add_profile("Yard", &system).unwrap();
    assert_eq!(wizard.rename_profile("Yard"), Ok(false));
    assert_eq!(wizard.rename_profile("Front"), Ok(true));
    assert_eq!(wizard.profile_names(), ["Default", "Front"]);
    assert_eq!(wizard.active().unwrap().name_text(), "Yard", "KEEP: the copy keeps the old name");
    wizard.delete_profile(&system);
    wizard.delete_profile(&system);
    assert_eq!(wizard.profile_names(), ["Default"], "deleting the last makes a new Default");
}

#[test]
fn format_changes_follow_the_original() {
    let root = temp_dir("formats");
    let system = system();
    let paths = DataPaths::new(&root);
    paths.ensure_directories().unwrap();
    std::fs::write(root.join("Sequence/Song.tim"), "").unwrap();
    let mut wizard = ExportWizard::new(paths, Vec::new(), &system);
    wizard.next(&system, &|_| true);
    wizard.add_sequences(&[root.join("Sequence/Song.tim")]);
    wizard.next(&system, &|_| true);
    wizard.next(&system, &|_| true);
    assert_eq!(wizard.stage(), Stage::OutputFormat);
    assert!(wizard.shows_falcon_group() && wizard.shows_universe_options() && wizard.compress_enabled());
    wizard.set_format(2);
    assert_eq!(wizard.active().unwrap().format_text(), "Falcon Player Sequence");
    assert!(!wizard.shows_falcon_group(), "KEEP: the older FSEQ shows the Sequence and Audio groups");
    assert!(!wizard.shows_universe_options() && !wizard.compress_enabled());
    wizard.set_include_audio(false, true);
    assert!(wizard.active().unwrap().include_audio && !wizard.active().unwrap().rename_audio);
    wizard.set_format(1);
    assert!(wizard.shows_falcon_group() && !wizard.shows_universe_options());
    assert!(wizard.falcon_audio_box(), "the Falcon box takes Include Audio");
    wizard.set_include_audio(true, false);
    assert!(!wizard.active().unwrap().include_audio, "FIX: the visible box works on Falcon Player Effect");
    wizard.set_falcon_folder("C:\\FPP");
    assert_eq!(wizard.active().unwrap().output_folder.as_deref(), Some("C:\\FPP\\effects"));
    assert!(!wizard.can_move_next(&|_| true), "GUARD: a Windows folder never counts as there");
}

/// A wizard on its format stage with one sequence, for F26's direct upload.
fn at_output_format(name: &str) -> (ExportWizard, SystemData, PathBuf) {
    let root = temp_dir(name);
    let paths = DataPaths::new(&root);
    paths.ensure_directories().unwrap();
    std::fs::write(root.join("Sequence/Song.tim"), "").unwrap();
    let system = system();
    let exists = |p: &str| Path::new(p).is_dir();
    let mut wizard = ExportWizard::new(paths, Vec::new(), &system);
    assert!(wizard.next(&system, &exists));
    wizard.add_sequences(&[root.join("Sequence/Song.tim")]);
    assert!(wizard.next(&system, &exists));
    assert!(wizard.next(&system, &exists));
    assert_eq!(wizard.stage(), Stage::OutputFormat);
    (wizard, system, root)
}

#[test]
fn direct_upload_needs_a_valid_host_that_answered() {
    let (mut wizard, system, _root) = at_output_format("direct");
    let exists = |p: &str| Path::new(p).is_dir();
    assert!(wizard.shows_direct_upload() && !wizard.direct_upload());
    assert!(!wizard.can_move_next(&exists), "File Path: the Falcon folder isn't set");
    assert_eq!(wizard.host_to_check(), None);

    wizard.set_direct_upload(true);
    assert!(wizard.direct_upload());
    assert!(!wizard.can_move_next(&exists), "no host yet");
    wizard.set_host_text("  fpp:80 ");
    assert_eq!(wizard.host_text(), "fpp:80", "trimmed");
    assert_eq!(wizard.host_to_check(), None, "not a host name, so nothing is pinged");
    wizard.set_host_text("192.168.1.50");
    assert_eq!(wizard.host_to_check().as_deref(), Some("192.168.1.50"));
    assert!(!wizard.can_move_next(&exists), "valid but not checked");

    assert!(!wizard.set_host_reachable("192.168.1.51", true), "an answer for an address since changed is dropped");
    assert!(!wizard.host_reachable());
    assert!(wizard.set_host_reachable("192.168.1.50", false));
    assert!(!wizard.can_move_next(&exists));
    assert!(wizard.set_host_reachable("192.168.1.50", true));
    assert!(wizard.can_move_next(&exists), "the folder isn't asked for in Direct Upload");
    wizard.set_host_text("192.168.1.50 ");
    assert!(!wizard.host_reachable(), "any edit clears the check");
    assert!(wizard.set_host_reachable("192.168.1.50", true));
    wizard.set_direct_upload(false);
    wizard.set_direct_upload(true);
    assert!(!wizard.host_reachable(), "so does changing the mode");
    assert!(wizard.set_host_reachable("192.168.1.50", true));
    wizard.set_interval_text("x");
    assert!(!wizard.can_move_next(&exists), "the Timing box still counts");
    wizard.set_interval_text("50");

    // FIX: on another format the flag left ticked means nothing.
    let other = sorted_formats().iter().position(|f| f.description() == "Falcon Player Sequence").unwrap();
    wizard.set_format(other);
    assert!(!wizard.shows_direct_upload() && !wizard.direct_upload());
    assert_eq!(wizard.host_to_check(), None);
    assert!(wizard.active().unwrap().fpp_direct_upload, "the profile keeps the choice for when 2.6+ comes back");
    let v2 = sorted_formats().iter().position(|f| f.description() == "Falcon Player Sequence 2.6+").unwrap();
    wizard.set_format(v2);
    assert!(wizard.direct_upload() && !wizard.host_reachable());

    // Coming back to the stage asks again.
    assert!(wizard.set_host_reachable("192.168.1.50", true));
    assert!(wizard.next(&system, &exists));
    assert_eq!(wizard.stage(), Stage::Summary);
    assert_eq!(wizard.host_to_check(), None, "only the format stage pings");
    assert!(wizard.previous(&system));
    assert!(!wizard.host_reachable());
}

#[test]
fn the_summary_says_what_a_direct_upload_sends_where() {
    let (mut wizard, system, _root) = at_output_format("direct-summary");
    let exists = |p: &str| Path::new(p).is_dir();
    wizard.set_direct_upload(true);
    wizard.set_host_text("fpp.local");
    assert_eq!(wizard.device_host(), None, "not on the Summary yet");
    assert!(wizard.set_host_reachable("fpp.local", true));
    wizard.set_include_audio(true, true);
    wizard.set_backup_universe_file(true);
    assert!(wizard.next(&system, &exists));

    let summary = wizard.summary(&live(&system)).unwrap();
    assert_eq!(summary.destination, "fpp.local");
    assert_eq!(summary.audio_destination.as_deref(), Some("fpp.local"));
    assert_eq!(summary.universe_destination, None);
    assert_eq!((summary.device_heading, summary.device_rows.len()), (None, 0), "nothing until the device answers");

    assert_eq!(wizard.device_host().as_deref(), Some("fpp.local"));
    assert!(!wizard.set_device_info("other", DeviceInfo::Unavailable), "an answer from another host is dropped");
    assert!(wizard.set_device_info("fpp.local", DeviceInfo::Unavailable));
    let summary = wizard.summary(&live(&system)).unwrap();
    assert_eq!(summary.device_heading.as_deref(), Some("FPP Device Info (unavailable \u{2014} check host address)"));
    assert!(summary.device_rows.is_empty());

    let known = |platform: &str, zip: bool| DeviceInfo::Known {
        host_name: "FPP".to_string(),
        description: "Roof".to_string(),
        platform: platform.to_string(),
        variant: "Pi 4".to_string(),
        zip,
    };
    assert!(wizard.set_device_info("fpp.local", known("Raspberry Pi", true)));
    let summary = wizard.summary(&live(&system)).unwrap();
    assert_eq!(summary.device_heading.as_deref(), Some("FPP Device Info"));
    let rows: Vec<(&str, &str)> = summary.device_rows.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    assert_eq!(rows, [("Host Name", "FPP"), ("Description", "Roof"), ("Platform", "Raspberry Pi"), ("Variant", "Pi 4")]);
    assert_eq!(summary.audio, "Rename to match sequence name.");
    assert_eq!(summary.warning, None, "a Falcon Player's ZIP flag means nothing");

    // KEEP: the platform is matched exactly.
    assert!(wizard.set_device_info("fpp.local", known("espixelstick", true)));
    assert_eq!(wizard.summary(&live(&system)).unwrap().warning, None);

    assert!(wizard.set_device_info("fpp.local", known("ESPixelStick", false)));
    let summary = wizard.summary(&live(&system)).unwrap();
    assert_eq!(summary.audio, "Not uploaded to ESPixelStick.");
    assert_eq!(summary.audio_destination, None);
    assert_eq!(summary.universe_destination.as_deref(), Some("Not uploaded to ESPixelStick."), "Backup alone is enough to say so");
    let warning = summary.warning.unwrap();
    assert!(warning.starts_with("ESPixelStick direct upload sends FSEQ sequences only; audio and universe settings are not uploaded. "), "{warning}");
    assert_eq!(warning, esp_warning(false));
    assert_ne!(esp_warning(false), esp_warning(true));

    // The run's own detection wins over a Summary answer that never came.
    assert!(wizard.set_device_info("fpp.local", DeviceInfo::Unavailable));
    assert_eq!(wizard.summary(&live(&system)).unwrap().warning, None);
    wizard.set_esp_detected(true);
    assert_eq!(wizard.summary(&live(&system)).unwrap().warning, Some(esp_warning(true)));

    // Leaving and returning forgets the device.
    assert!(wizard.previous(&system));
    assert!(wizard.set_host_reachable("fpp.local", true));
    assert!(wizard.next(&system, &exists));
    let summary = wizard.summary(&live(&system)).unwrap();
    assert_eq!((summary.device_heading, summary.warning), (None, None));
}

#[test]
fn network_preparation_blocks_next_and_ignores_abandoned_visits() {
    let (mut wizard, system, _root) = at_output_format("network-preparation");
    let exists = |p: &str| Path::new(p).is_dir();
    wizard.set_direct_upload(true);
    wizard.set_host_text("127.0.0.1");
    wizard.set_host_reachable("127.0.0.1", true);
    wizard.set_create_universe_file(true);
    assert!(wizard.next(&system, &exists));
    let first = wizard.network_request().unwrap();
    assert!(!wizard.can_move_next(&exists));
    assert!(wizard.complete_network(first, Err("fixture failure".into())));
    assert_eq!(wizard.network_error(), Some("Controller network preparation failed: fixture failure. Go back and return to retry."));
    assert!(!wizard.can_move_next(&exists));
    assert!(wizard.previous(&system));
    wizard.set_host_reachable("127.0.0.1", true);
    assert!(wizard.next(&system, &exists));
    let second = wizard.network_request().unwrap();
    assert_ne!(first, second);
    assert!(!wizard.complete_network(first, Ok(live(&system))));
    assert_eq!(wizard.network_request(), Some(second));
    let mut prepared = live(&system);
    for controller in &mut prepared {
        controller.network = Some(vixen_export::fpp_config::NetworkConfig {
            protocol: vixen_export::fpp_config::ProtocolType::Sacn,
            transmission: vixen_export::fpp_config::TransmissionMethod::Multicast,
            ip_address: String::new(), throttling_enabled: false, universes: Vec::new(),
        });
    }
    assert!(wizard.complete_network(second, Ok(prepared)));
    assert_eq!(wizard.summary(&live(&system)).unwrap().warning, None, "the cached network configuration supplies the summary");
    assert!(wizard.can_move_next(&exists));
    assert_eq!(wizard.network_error(), None);
    assert!(!wizard.complete_network(second, Err("duplicate completion".into())));
    assert!(wizard.can_move_next(&exists));
    assert!(wizard.previous(&system));
    assert!(!wizard.complete_network(second, Err("cancelled visit".into())));
    wizard.set_create_universe_file(false);
    wizard.set_host_reachable("127.0.0.1", true);
    assert!(wizard.next(&system, &exists));
    assert_eq!(wizard.network_request(), None);
    assert!(wizard.can_move_next(&exists));
    assert!(wizard.previous(&system));
    wizard.set_create_universe_file(true);
    wizard.set_host_reachable("127.0.0.1", true);
    assert!(wizard.next(&system, &exists));
    let cancelled = wizard.network_request().unwrap();
    wizard.cancel_network();
    assert!(!wizard.complete_network(cancelled, Ok(live(&system))));
}
