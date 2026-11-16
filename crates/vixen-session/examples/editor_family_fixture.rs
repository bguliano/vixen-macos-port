//! Private valid-XML source values for Color/Gradient editor-to-library gestures.
use std::{path::PathBuf, sync::Arc};
use vixen_core::{Color, ColorGradient, Curve, TimeSpan};
use vixen_core::gradient::{ColorPoint, AlphaPoint};
use vixen_effects::framework::ParamValue;
use vixen_io::{CurveLibrary, DataPaths, GradientLibrary, SystemData};
use vixen_session::sequence::{SequenceSession, SessionEnvironment};

fn gradient() -> ColorGradient {
    ColorGradient::from_parts(
        vec![ColorPoint::new(Color::rgb(19,91,173),0.0), ColorPoint::new(Color::rgb(211,53,97),0.35), ColorPoint::new(Color::rgb(61,199,113),1.0)],
        vec![AlphaPoint::new(255,0.0),AlphaPoint::new(153,0.6),AlphaPoint::new(207,1.0)])
}
fn gradient_json(value: &ColorGradient) -> serde_json::Value {
    serde_json::json!({"colors":value.colors().iter().map(|p| serde_json::json!({"xyz":[p.color.x,p.color.y,p.color.z],"position":p.position,"focus":p.focus})).collect::<Vec<_>>(),
        "alphas":value.alphas().iter().map(|p|serde_json::json!({"alpha":p.alpha,"position":p.position,"focus":p.focus})).collect::<Vec<_>>(),
        "gamma":value.gammacorrected,"title":value.title,"link":value.library_reference_name,"current":value.is_current_library_gradient})
}
fn main() {
    let mut args=std::env::args().skip(1);
    let root=PathBuf::from(args.next().expect("fresh target profile"));
    let kind=args.next().expect("color or gradient");assert!(kind=="color" || kind=="gradient");
    let inspect=args.next();assert!(inspect.is_none() || inspect.as_deref()==Some("--inspect"));
    let palette_override=args.next().map(PathBuf::from);assert!(args.next().is_none());
    assert!(root.is_absolute() && root.components().any(|p|p.as_os_str()=="target"));
    let paths=DataPaths::new(root);
    let title=if kind=="color" {"Editor Color"} else {"Editor Gradient"};
    let sequence_path=paths.sequence_dir().join(format!("{title}.tim"));
    let colors=[Color::RED,Color::rgb(0,255,0),Color::BLUE,Color::rgb(255,255,0)];
    let system=if inspect.is_some() {vixen_io::load_system(&paths).unwrap()} else {
        assert!(!paths.root().exists() && palette_override.is_none(),"refuse existing profile");paths.ensure_directories().unwrap();
        let mut system=SystemData::default();system.tree.add_element("Private family drag element");
        let curves=CurveLibrary{curves:['A','B','C','D'].into_iter().enumerate().map(|(i,c)|{let mut v=Curve::flat(10.0*(i+1) as f64);v.is_current_library_curve=true;(format!("Curve {c}"),v)}).collect()};
        let gradients=GradientLibrary{gradients:['A','B','C','D'].into_iter().zip(colors).map(|(c,color)|{let mut v=ColorGradient::solid(color);v.is_current_library_gradient=true;(format!("Gradient {c}"),v)}).collect()};
        vixen_io::set_curve_library(&mut system.module_store,&curves);vixen_io::set_gradient_library(&mut system.module_store,&gradients);
        vixen_session::web::store_settings(&mut system.module_store,vixen_session::web::Settings{port:8888,enabled:false});
        vixen_io::save_system(&system,&paths).unwrap();
        vixen_session::sequencer::libraries::write_vfc(&paths.root().join("ColorPalette.xml"),&colors).unwrap();system
    };
    assert!(system.controllers.is_empty());
    let environment=SessionEnvironment{video_cache:None,tree:Arc::new(system.tree.clone()),store:Arc::new(vixen_io::system::effective_store(&system).into_owned()),module_data_root:Some(paths.module_data_dir()),frame_ms:50,media_dir:paths.media_dir()};
    let color=Color::rgb(17,119,203);let expected_gradient=gradient();
    if inspect.is_none(){
        let (_,node)=system.tree.all_nodes().find(|(_,n)|n.name=="Private family drag element").unwrap();
        let mut seq=SequenceSession::new_untitled(&environment);seq.set_length(TimeSpan::from_secs(3.0));
        let name=if kind=="color" {"Set Level"} else {"Pulse"};let descriptor=vixen_effect_registry::all().iter().find(|e|e.name==name).unwrap();
        let id=seq.add_new_effect(descriptor.type_id,node.id,&node.name,TimeSpan::ZERO,TimeSpan::from_secs(2.0)).unwrap();let mut params=seq.effect(id).unwrap().node.params.clone();
        if kind=="color" {params.set("Color",ParamValue::Rgb(vixen_core::color::Rgb::from_color(color)));} else {params.set("ColorGradient",ParamValue::Gradient(expected_gradient.clone()));}
        seq.set_params(id,params);seq.save(Some(&sequence_path)).unwrap();
    }
    let seq=SequenceSession::open(&sequence_path,&environment).unwrap();assert_eq!(seq.effect_count(),1);let effect=seq.effects().next().unwrap();
    if kind=="color" {assert_eq!(effect.node.params.color("Color"),color);} else {assert_eq!(effect.node.params.gradient("ColorGradient"),expected_gradient);}
    let palette_path=palette_override.unwrap_or_else(||paths.root().join("ColorPalette.xml"));
    let palette=vixen_session::sequencer::libraries::read_vfc(&palette_path).unwrap();
    let gradients=vixen_io::gradient_library(&system.module_store);
    let report=serde_json::json!({"kind":kind,"controllers":0,"effects":1,"title":title,"sequence":sequence_path,"sourceColor":color.to_argb_u32(),"sourceGradient":gradient_json(&expected_gradient),"palette":palette.iter().map(|c|c.to_argb_u32()).collect::<Vec<_>>(),"gradients":gradients.gradients.iter().map(|(name,g)|serde_json::json!({"name":name,"value":gradient_json(g)})).collect::<Vec<_>>()});
    if inspect.is_none(){std::fs::write(paths.root().join("fixture.json"),serde_json::to_vec_pretty(&report).unwrap()).unwrap();}
    println!("{report}");
}
