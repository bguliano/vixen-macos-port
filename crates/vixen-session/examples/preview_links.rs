//! For each preview: how its lights link to elements, and how much they change while a sequence plays.
use std::path::PathBuf;
use std::sync::Arc;
use vixen_core::{ElementTree, TimeSpan};
use vixen_engine::schedule::EvalScratch;
use vixen_io::{load_system, system::effective_store, DataPaths};
use vixen_preview::Scene;
use vixen_session::preview::{load_preview_data, preview_entries};
use vixen_session::sequence::{SequenceSession, SessionEnvironment};

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).unwrap());
    let seq_name = std::env::args().nth(2).unwrap();
    let paths = DataPaths::new(&root);
    let system = load_system(&paths).unwrap();
    let store = effective_store(&system).into_owned();
    let tree: &ElementTree = &system.tree;
    let env = SessionEnvironment { video_cache: None,
        tree: Arc::new(tree.clone()),
        store: Arc::new(store.clone()),
        module_data_root: Some(paths.module_data_dir()),
        frame_ms: system.default_update_interval,
        media_dir: paths.media_dir(),
    };
    let mut session = SequenceSession::open(&paths.sequence_dir().join(format!("{seq_name}.tim")), &env).unwrap();
    session.wait_for_renders();
    let schedule = session.schedule();
    for entry in preview_entries(&system) {
        let data = load_preview_data(&store, &entry).unwrap();
        let scene = Scene::build(&data, Some(tree));
        let resolved = scene.resolve_nodes(tree);
        let unresolved_nodes = resolved.iter().filter(|n| n.is_none()).count();
        let mut no_node = 0;
        let mut no_element = 0;
        let mut mapped = Vec::new();
        for link in &scene.links {
            if link.node == vixen_preview::scene::NO_NODE {
                no_node += 1;
                mapped.push(None);
                continue;
            }
            match resolved.get(link.node as usize).copied().flatten() {
                Some(n) => {
                    let idx = tree.node(n).element_id.and_then(|e| schedule.elements().index_of(&e));
                    if idx.is_none() {
                        no_element += 1;
                    }
                    mapped.push(idx);
                }
                None => {
                    no_element += 1;
                    mapped.push(None);
                }
            }
        }
        println!(
            "{} ({}): {} lights, {} shapes, {} nodes ({} unresolved), lights unlinked {}, unresolved element {}, bg {:?} alpha {}, gl {}, setup {}x{}",
            entry.name,
            entry.id,
            scene.lights.len(),
            data.display_items.len(),
            scene.nodes().len(),
            unresolved_nodes,
            no_node,
            no_element,
            data.background_file_name,
            data.background_alpha,
            data.use_open_gl,
            data.setup_width,
            data.setup_height
        );
        let (mut xmin, mut xmax, mut ymin, mut ymax) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        for l in &scene.lights {
            xmin = xmin.min(l.x);
            xmax = xmax.max(l.x);
            ymin = ymin.min(l.y);
            ymax = ymax.max(l.y);
        }
        println!("  light bounds x {xmin}..{xmax} y {ymin}..{ymax}, zoom {}", scene.zoom());
        // How much it changes.
        let mut scratch = EvalScratch::default();
        let mut states = Vec::new();
        let mut last: Vec<u32> = Vec::new();
        let mut changed_frames = 0;
        let mut frames = 0;
        let mut total_lit = 0u64;
        let mut at = TimeSpan::ZERO;
        let step = TimeSpan::from_ms_i(10);
        while at < session.length() {
            schedule.evaluate(at, &mut scratch, &mut states);
            let colors: Vec<u32> = mapped
                .iter()
                .map(|m| {
                    m.and_then(|i| states.get(i as usize)).map_or(0, |s| {
                        if let Some(rgb) = s.rgb {
                            rgb.full_color_with_alpha().to_argb_u32()
                        } else if let Some(l) = s.lighting {
                            l.full_color_with_alpha().to_argb_u32()
                        } else {
                            0
                        }
                    })
                })
                .collect();
            total_lit += colors.iter().filter(|c| **c >> 24 != 0).count() as u64;
            if colors != last {
                changed_frames += 1;
                last = colors;
            }
            frames += 1;
            at = at + step;
        }
        println!("  over the song: {frames} frames at 100 Hz, {changed_frames} changed ({:.1} per second), average lit lights {:.0}", changed_frames as f64 / (frames as f64 / 100.0), total_lit as f64 / frames as f64);
    }
}
