//! Produce actual port library files; never initializes a profile or application.
use std::path::PathBuf;
use vixen_core::{AlphaPoint, Color, ColorGradient, ColorPoint, Curve};
use vixen_session::sequencer::libraries::{read_vcl, read_vgl, write_vcl, write_vgl};

fn data() -> (Vec<(String, Curve)>, Vec<(String, ColorGradient)>) {
    let mut curves = vec![
        ("Ramp & ties".into(), Curve::ramp_up()),
        ("Three points Café".into(), Curve::from_points(vec![(0., 10.), (25., 90.), (100., 30.)])),
        ("Clamp".into(), Curve::from_points(vec![(0., -20.), (100., 120.)])),
        ("Flat 13.5".into(), Curve::flat(13.5)),
    ];
    for (_, c) in &mut curves { c.is_current_library_curve = true; }
    let red = ColorGradient::solid(Color::RED);
    let mut linear = ColorGradient::default();
    linear.set_colors(vec![ColorPoint::new(Color::BLACK, 0.), ColorPoint::new(Color::WHITE, 1.)]);
    let mut focus = linear.clone();
    let mut points = focus.colors().to_vec(); points[1].focus = 0.25; focus.set_colors(points);
    focus.gammacorrected = true;
    let mut alpha = linear.clone();
    alpha.set_alphas(vec![AlphaPoint::new(0, 0.), AlphaPoint::new(255, 1.)]);
    let mut gradients = vec![("Red".into(), red), ("Black to white".into(), linear),
        ("Focus Café & <quarter>".into(), focus), ("Alpha ramp".into(), alpha)];
    for (n, g) in &mut gradients { g.title = Some(format!("Private {n}")); g.is_current_library_gradient = true; }
    (curves, gradients)
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert!(args.len() == 2 && ["--write", "--readback"].contains(&args[0].as_str()));
    let root = PathBuf::from(&args[1]);
    assert!(root.is_absolute());
    let (curves, gradients) = data();
    if args[0] == "--write" {
        assert!(!root.exists(), "refuse existing output");
        std::fs::create_dir(&root).unwrap();
        write_vcl(&root.join("Mac.vcl"), &curves).unwrap();
        write_vgl(&root.join("Mac.vgl"), &gradients).unwrap();
    }
    let curves_back = read_vcl(&root.join("Mac.vcl")).unwrap();
    let gradients_back = read_vgl(&root.join("Mac.vgl")).unwrap();
    assert_eq!(curves_back, curves);
    assert_eq!(gradients_back, gradients);
    // Expected values are independent hand-derived interpolation/ties-to-even
    // examples. Never infer expected Windows output from a Mac sample result.
    let cx = vec![vec![-10.,0.,0.5,1.5,2.5,25.,50.,75.,100.,110.],
        vec![-1.,0.,12.5,25.,62.5,100.,101.], vec![0.,25.,50.,75.,100.], vec![0.,50.,100.]];
    let cv = vec![vec![0.,0.,0.5,1.5,2.5,25.,50.,75.,100.,100.],
        vec![10.,10.,50.,90.,60.,30.,30.], vec![0.,15.,50.,85.,100.], vec![13.5,13.5,13.5]];
    let gx = vec![vec![-1.,0.,0.5,1.,2.], vec![-1.,0.,0.25,0.5,0.75,1.,2.],
        vec![0.,0.125,0.25,0.625,1.], vec![0.,0.25,0.5,0.75,1.]];
    let gc: Vec<Vec<u32>> = vec![vec![0xffff0000;5],
        vec![0xff000000,0xff000000,0xff3f3f3f,0xff7f7f7f,0xffbfbfbf,0xffffffff,0xffffffff],
        vec![0xff000000,0xff40403f,0xff80807f,0xffbfbfbf,0xffffffff],
        vec![0,0x3f3f3f3f,0x7f7f7f7f,0xbfbfbfbf,0xffffffff]];
    let cr: Vec<_> = curves.iter().enumerate().map(|(i,(name,c))| {
        let samples: Vec<_> = cx[i].iter().zip(&cv[i]).map(|(&x,&v)| {
            assert_eq!(c.get_value(x),v); assert_eq!(c.get_int_value(x),v.round_ties_even() as i32);
            serde_json::json!({"x":x,"value":v,"intValue":v.round_ties_even() as i32}) }).collect();
        serde_json::json!({"name":name,"points":c.points,"current":true,"reference":"","samples":samples})
    }).collect();
    let gr: Vec<_> = gradients.iter().enumerate().map(|(i,(name,g))| {
        let samples: Vec<_> = gx[i].iter().zip(&gc[i]).map(|(&x,&argb)| {
            assert_eq!(g.get_color_at(x).to_argb_u32(),argb,"{name} at {x}");
            serde_json::json!({"x":x,"argb":argb}) }).collect();
        serde_json::json!({"name":name,"title":g.title,"gamma":g.gammacorrected,
            "current":true,"reference":"","samples":samples})
    }).collect();
    let result = serde_json::json!({"schema":1,"scope":"Production Mac library serializer/readback and independent arithmetic sample assertions; no Windows claim", "curves":cr,"gradients":gr});
    if args[0] == "--write" { std::fs::write(root.join("expected.json"),serde_json::to_vec_pretty(&result).unwrap()).unwrap(); }
    println!("{result}");
}
