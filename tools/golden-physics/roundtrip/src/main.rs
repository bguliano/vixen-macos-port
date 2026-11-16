use fx_pixel_physics::generated::{liquid as lg, morph as mg};
use fx_pixel_physics::morph::polygon::{ellipse_record, line_record, polygon_record};
use fx_pixel_physics::{liquid, morph};
use uuid::Uuid;
use vixen_effects::framework::*;
use vixen_io::params::{effect_model_to_xml, effect_params_from_xml};

fn main() {
    // Morph with one of each shape.
    let mut p = morph::DESCRIPTOR.new_params();
    let mut poly = Params::defaults(&mg::MORPH_POLYGON_PARAMS);
    poly.set("Time", ParamValue::Double(0.25));
    let mut line = Params::defaults(&mg::MORPH_POLYGON_PARAMS);
    line.set("Polygon", ParamValue::None);
    line.set("Line", ParamValue::Record(Box::new(line_record(&[(1.0, 2.5), (3.0, 4.0)]))));
    let mut ell = Params::defaults(&mg::MORPH_POLYGON_PARAMS);
    ell.set("Polygon", ParamValue::None);
    ell.set("Ellipse", ParamValue::Record(Box::new(ellipse_record(&[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)], (2.0, 2.0), 4.0, 4.0, 30.0))));
    p.set("PolygonType", ParamValue::Enum("FreeForm".to_string()));
    p.set("MorphPolygonData", ParamValue::RecordList(vec![poly, line, ell]));
    let _ = polygon_record(&[]);

    let xml = effect_model_to_xml(&morph::DESCRIPTOR.type_id, &Uuid::new_v4(), &p).unwrap();
    let text = xml.to_fragment();
    println!("{text}");
    let back = effect_params_from_xml(&xml, &morph::DESCRIPTOR);
    let hints: Vec<_> = back.unknown_members.iter().map(|m| m.0.clone()).collect();
    println!("morph unknown members/hints: {hints:?}");
    let (a, b) = (p.records("MorphPolygonData"), back.records("MorphPolygonData"));
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b) {
        for (i, d) in x.desc.iter().enumerate() {
            assert_eq!(x.values[i], y.values[i], "{}", d.name);
        }
        println!("record hints: {:?}", y.unknown_members.iter().map(|m| m.0.clone()).collect::<Vec<_>>());
    }
    for (i, d) in p.desc.iter().enumerate() {
        if !d.xml.is_empty() {
            assert_eq!(p.values[i], back.values[i], "{}", d.name);
        }
    }

    // Liquid's default two emitters.
    let p = liquid::DESCRIPTOR.new_params();
    let xml = effect_model_to_xml(&liquid::DESCRIPTOR.type_id, &Uuid::new_v4(), &p).unwrap();
    let text = xml.to_fragment();
    println!("{}", &text[..text.len().min(2600)]);
    let back = effect_params_from_xml(&xml, &liquid::DESCRIPTOR);
    println!("liquid unknown members/hints: {:?}", back.unknown_members.iter().map(|m| m.0.clone()).collect::<Vec<_>>());
    for (i, d) in p.desc.iter().enumerate() {
        if !d.xml.is_empty() {
            assert_eq!(p.values[i], back.values[i], "{}", d.name);
        }
    }
    let _ = (lg::EMITTER_DATA.xml_type,);
    println!("round trips ok");
}
