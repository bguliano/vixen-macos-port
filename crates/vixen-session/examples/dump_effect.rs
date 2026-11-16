//! Prints an effect's parameters and rendered intents.  dump_effect <data folder> <sequence name> <effect id> [element name part]
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use vixen_engine::{render_effect, EffectJob, RenderEnv};
use vixen_io::{load_sequence_with_store, load_system, system::effective_store, DataPaths};
use vixen_session::render::resolve_targets;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let paths = DataPaths::new(PathBuf::from(&args[1]));
    let system = load_system(&paths).unwrap();
    let store = effective_store(&system).into_owned();
    let seq = load_sequence_with_store(&paths.sequence_dir().join(format!("{}.tim", args[2])), &store).unwrap();
    let id: uuid::Uuid = args[3].parse().unwrap();
    let node = seq.effects.iter().find(|e| e.instance_id == id).expect("effect");
    let d = vixen_effect_registry::by_type_id(&node.type_id).unwrap();
    println!("{} start {} span {} targets {:?}", d.name, node.start.total_ms(), node.time_span.total_ms(), node.targets);
    for p in d.params {
        let s = format!("{:?}", node.params.get(p.name));
        println!("  {} = {}", p.name, &s[..s.len().min(300)]);
    }
    let targets = resolve_targets(&system.tree, node);
    let env = RenderEnv { video_cache: None, tree: &system.tree, frame_ms: system.default_update_interval, marks: &seq.marks, audio: None, module_data_root: None };
    let job = EffectJob { id, descriptor: d, params: &node.params, targets: &targets, start: node.start, span: node.time_span };
    let cancel = AtomicBool::new(false);
    let out = render_effect(&env, &job, &cancel).unwrap();
    println!("{} elements", out.map.len());
    let filter = args.get(4);
    let mut names: Vec<(String, uuid::Uuid)> = out.map.keys().map(|e| {
        let n = system.tree.find_by_element(e).map(|n| system.tree.node(n).name.clone()).unwrap_or_default();
        (n, *e)
    }).collect();
    names.sort();
    for (n, e) in names {
        if filter.is_some_and(|f| !n.contains(f.as_str())) { continue; }
        let list = &out.map[&e];
        println!(" {n}: {} intents", list.len());
        for i in list.iter().take(12) {
            println!("    {:.3}..{:.3} {:?}", i.start.total_ms(), i.end.total_ms(), i.intent.data);
        }
    }
}
