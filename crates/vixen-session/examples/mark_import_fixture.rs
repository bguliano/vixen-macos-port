//! Private, controller-free fixture for the native imported-mark selection check.
use std::{path::PathBuf, sync::Arc};
use vixen_core::{marks::{Mark, MarkCollection}, TimeSpan};
use vixen_io::{DataPaths, SystemData};
use vixen_session::sequence::{SequenceSession, SessionEnvironment};

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("fresh target profile"));
    assert!(root.components().any(|part| part.as_os_str() == "target"));
    assert!(!root.exists(), "refuse existing profile");
    let paths = DataPaths::new(root);
    paths.ensure_directories().unwrap();
    let system = SystemData::default();
    assert!(system.controllers.is_empty());
    vixen_io::save_system(&system, &paths).unwrap();
    let env = SessionEnvironment {
        video_cache: None, tree: Arc::new(system.tree.clone()),
        store: Arc::new(vixen_io::system::effective_store(&system).into_owned()),
        module_data_root: Some(paths.module_data_dir()), frame_ms: 25, media_dir: paths.media_dir(),
    };
    let mut sequence = SequenceSession::new_untitled(&env);
    sequence.set_length(TimeSpan::from_secs(10.0));
    let sequence_path = paths.sequence_dir().join("Mark Import Verification.tim");
    sequence.save(Some(&sequence_path)).unwrap();
    let collections: Vec<_> = ["Zebra Source", "Alpha Source", "Middle Source"].into_iter().enumerate().map(|(i, name)| {
        let mut mark = Mark::new(TimeSpan::from_ms_i((i as i64 + 1) * 1000));
        mark.text = format!("source {}", i + 1);
        MarkCollection { name: name.into(), marks: vec![mark], ..MarkCollection::default() }
    }).collect();
    let input = paths.root().join("collections.v3m");
    std::fs::write(&input, vixen_io::marks_files::write_v3m(&collections.iter().collect::<Vec<_>>()).unwrap()).unwrap();
    let back = vixen_io::marks_files::read_v3m(&std::fs::read_to_string(&input).unwrap()).unwrap();
    assert_eq!(back.len(), collections.len());
    for (read, original) in back.iter().zip(&collections) {
        assert_eq!((read.id, &read.name), (original.id, &original.name));
        assert!(read.linked_mark_collection_id.is_none());
        assert_eq!(read.marks.len(), 1);
        assert_eq!((read.marks[0].start_time, read.marks[0].duration, &read.marks[0].text),
            (original.marks[0].start_time, original.marks[0].duration, &original.marks[0].text));
    }
    let manifest = serde_json::json!({"controllers":0,"muted":true,"sequence":sequence_path,"input":input,
        "candidates":collections.iter().map(|c| serde_json::json!({"id":c.id.to_string(),"name":c.name,
            "startTicks":c.marks[0].start_time.ticks(),"durationTicks":c.marks[0].duration.ticks(),"text":c.marks[0].text})).collect::<Vec<_>>()});
    std::fs::write(paths.root().join("fixture.json"), serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    println!("{manifest}");
}
