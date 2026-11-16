use super::*;
#[test]
fn original_csharp_state_machine_golden() {
    // Generated from the unchanged upstream Vixen2xSequenceImportSM.cs under .NET 10;
    // only its sequence/effect services are recording stubs. 150 deterministic channels.
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/vixen2-effects.json")).unwrap();
    for (index, case) in cases.as_array().unwrap().iter().enumerate() {
        let events = case["events"].as_array().unwrap().iter().map(|v| {
            (
                Color::from_argb_u32(v[0].as_i64().unwrap() as u32),
                v[1].as_u64().unwrap() as u8,
            )
        });
        let actual = channel_effects(Uuid::nil(), 25, events);
        let expected = case["effects"].as_array().unwrap();
        assert_eq!(actual.len(), expected.len(), "case {index}");
        for (a, e) in actual.iter().zip(expected) {
            assert_eq!(a.start_ms, e["start"].as_u64().unwrap());
            assert_eq!(a.duration_ms, e["duration"].as_u64().unwrap());
            assert_eq!(a.ramp, e["ramp"].as_bool().unwrap());
            assert_eq!(a.color.to_argb(), e["color"].as_i64().unwrap() as i32);
            assert!((a.start_level as f64 - e["first"].as_f64().unwrap()).abs() < 1e-9);
            assert!((a.end_level as f64 - e["last"].as_f64().unwrap()).abs() < 1e-9);
        }
    }
}
fn mapping(n: usize, target: Uuid, color: Color) -> ChannelMapping {
    ChannelMapping {
        name: format!("Channel {n}"),
        number: n.to_string(),
        output: (n - 1).to_string(),
        color,
        destination: Some(target),
        import_color: color,
        mixing: false,
    }
}
fn fixture(bytes: &[u8], channels: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("vixen2-{}.vix", Uuid::new_v4()));
    std::fs::write(&p,format!("<Program><Time>200</Time><EventPeriodInMilliseconds>50</EventPeriodInMilliseconds><EventValues>{}</EventValues><Channels>{channels}</Channels></Program>",vixen_xml::nbfx::base64_encode(bytes))).unwrap();
    p
}
#[test]
fn parse_flattened_and_old_names() {
    let p = fixture(
        &[0, 4, 5, 255],
        r#"<Channel color="-65536" output="0">A &amp; B</Channel>"#,
    );
    let f = Vixen2File::read(&p).unwrap().finish(None, None).unwrap();
    std::fs::remove_file(p).unwrap();
    assert_eq!(f.channels[0].name, "A & B");
    assert_eq!(f.events_per_channel, 4);
    assert_eq!(f.channels[0].color, Color::RED);
    assert_eq!(f.profile_name, FLATTENED);
}
#[test]
fn reject_incomplete_channels() {
    let p = fixture(&[1, 2, 3], r#"<Channel name="A" color="-1" output="0"/>"#);
    assert!(Vixen2File::read(&p).unwrap().finish(None, None).is_err());
    std::fs::remove_file(p).unwrap();
}
#[test]
fn state_machine_thresholds_reprocess_and_close() {
    let e = channel_effects(
        Uuid::nil(),
        50,
        [0, 4, 5, 5, 6, 7, 6, 5, 0, 100].map(|v| (Color::RED, v)),
    );
    assert_eq!(
        e.iter()
            .map(|e| (
                e.start_ms,
                e.duration_ms,
                e.ramp,
                e.start_level,
                e.end_level
            ))
            .collect::<Vec<_>>(),
        vec![
            (100, 100, false, 5, 5),
            (200, 100, true, 6, 7),
            (300, 50, false, 6, 6),
            (350, 50, false, 5, 5),
            (450, 50, false, 100, 100)
        ]
    );
}
#[test]
fn ramp_down_and_color_changes() {
    let e = channel_effects(
        Uuid::nil(),
        25,
        [
            (Color::RED, 100),
            (Color::RED, 50),
            (Color::RED, 6),
            (Color::BLUE, 6),
        ],
    );
    assert_eq!(e.len(), 2);
    assert!(e[0].ramp);
    assert_eq!(e[0].duration_ms, 75);
    assert_eq!(e[0].end_level, 6);
    assert_eq!(e[1].color, Color::BLUE);
}
#[test]
fn drop_order_repeats_and_normalization() {
    let mut tree = ElementTree::new();
    let id = Uuid::new_v4();
    tree.add_node(id, "A", Some(Uuid::new_v4()));
    let mut rows = (1..=5)
        .map(|n| mapping(n, id, Color::WHITE))
        .collect::<Vec<_>>();
    apply_drop(&mut rows, 1, &[id], 1, true, "BGR", &tree).unwrap();
    assert_eq!(rows[1].import_color, Color::BLUE);
    assert_eq!(rows[2].import_color, Color::GREEN);
    assert_eq!(rows[3].import_color, Color::RED);
    assert!(!rows[4].mixing);
    rows[0].destination = Some(Uuid::new_v4());
    assert_eq!(maps::normalized(&rows, &tree)[0].destination, None);
}
#[test]
fn rgb_conversion_does_not_mutate_map() {
    let mut tree = ElementTree::new();
    let id = Uuid::new_v4();
    tree.add_node(id, "Pixel", Some(Uuid::new_v4()));
    let p = fixture(
        &[100, 100, 100, 100, 50, 50, 50, 50, 25, 25, 25, 25],
        r#"<Channel name="R" color="-65536" output="0"/><Channel name="G" color="-16744448" output="1"/><Channel name="B" color="-16776961" output="2"/>"#,
    );
    let f = Vixen2File::read(&p).unwrap().finish(None, None).unwrap();
    std::fs::remove_file(p).unwrap();
    let rows = [Color::RED, Color::GREEN, Color::BLUE]
        .into_iter()
        .enumerate()
        .map(|(n, c)| {
            let mut m = mapping(n + 1, id, c);
            m.mixing = true;
            m
        })
        .collect::<Vec<_>>();
    let before = rows.clone();
    let (out, warnings) = convert(&f, &rows, &tree, |_| {}).unwrap();
    assert!(warnings.is_empty());
    assert_eq!(out.effects.len(), 1);
    assert_eq!(rows, before);
    assert_eq!(
        out.effects[0].params.double("IntensityLevel"),
        100.0 / 255.0
    );
    assert_eq!(
        out.effects[0].params.color("Color"),
        Color::rgb(255, 127, 64)
    );
    let (_, warnings) = convert(&f, &rows[..2], &tree, |_| {}).unwrap();
    assert_eq!(warnings.len(), 2);
    assert_eq!(rows, before);
}
#[test]
fn map_serialization_preserves_known_mix_colors_and_unknown_argb() {
    let id = Uuid::new_v4();
    let mut row = mapping(1, id, Color::rgb(12, 34, 56));
    row.import_color = Color::GREEN;
    row.mixing = true;
    let maps = maps::Maps::from([("Map".into(), vec![row])]);
    let mut store = vixen_io::ModuleStore::new();
    maps::save(&mut store, &maps);
    assert_eq!(maps::load(&store), maps);
    let xml = maps::xml(&maps);
    let item = xml
        .child("vixen2xMappings")
        .unwrap()
        .elements()
        .next()
        .unwrap();
    assert_eq!(item.local, "KeyValueOfstringArrayOfChannelMapping3YEWi_PPm");
    assert_eq!(
        item.child("Value")
            .unwrap()
            .elements()
            .next()
            .unwrap()
            .child("DestinationColor")
            .unwrap()
            .child_text("knownColor")
            .as_deref(),
        Some("79")
    );
}
#[test]
fn named_profile_channels_and_audio_paths() {
    let p = fixture(&[0, 5, 10, 20], "");
    let text = std::fs::read_to_string(&p).unwrap().replace(
        "<Channels></Channels>",
        "<Profile>Old Profile</Profile><Audio filename=\"song.mp3\"/>",
    );
    std::fs::write(&p, text).unwrap();
    let profile = p.with_extension("pro");
    std::fs::write(&profile,r#"<Profile><ChannelObjects><Channel name="" color="-1" output="12"/></ChannelObjects></Profile>"#).unwrap();
    let first = Vixen2File::read(&p).unwrap();
    assert_eq!(first.profile_name, "Old Profile");
    assert_eq!(first.audio_name, "song.mp3");
    assert!(first.clone().finish(None, None).is_err());
    let f = first.finish(None, Some(&profile)).unwrap();
    assert_eq!(f.channels[0].name, "Unnamed Output 12");
    assert_eq!(f.profile, Some(profile.clone()));
    std::fs::remove_file(p).unwrap();
    std::fs::remove_file(profile).unwrap();
}
#[test]
fn malformed_data_is_a_parse_error() {
    let p = fixture(
        &[1, 2, 3, 4],
        r#"<Channel name="A" color="-1" output="0"/>"#,
    );
    let original = std::fs::read_to_string(&p).unwrap();
    for bad in ["AQ==ignored", "!@#$", "AQ=", ""] {
        let text = original.replace("AQIDBA==", bad);
        std::fs::write(&p, text).unwrap();
        assert!(Vixen2File::read(&p).is_err(), "{bad}");
    }
    std::fs::remove_file(p).unwrap();
}
#[test]
fn audio_copy_keeps_existing_files_and_adds_media_model() {
    let root = std::env::temp_dir().join(format!("vixen2-audio-{}", Uuid::new_v4()));
    let media = root.join("Media");
    std::fs::create_dir_all(&media).unwrap();
    let audio = root.join("song.mp3");
    std::fs::write(&audio, b"new audio").unwrap();
    std::fs::write(media.join("song.mp3"), b"existing audio").unwrap();
    let mut sequence = vixen_io::new_sequence();
    associate_audio(&mut sequence, &audio, &media).unwrap();
    assert_eq!(sequence.media[0].file_name, "song (1).mp3");
    assert_eq!(
        std::fs::read(media.join("song.mp3")).unwrap(),
        b"existing audio"
    );
    assert_eq!(
        std::fs::read(media.join("song (1).mp3")).unwrap(),
        b"new audio"
    );
    assert!(sequence.media[0]
        .data
        .as_ref()
        .unwrap()
        .child("FilePath")
        .is_some());
    std::fs::remove_dir_all(root).unwrap();
}
