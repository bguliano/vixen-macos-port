// Scratch probe: how does gfx::Graphics rasterize GDI+-style calls with AA off?
use vixen_effects::gfx::Graphics;
use vixen_core::Color;

fn dump(name: &str, g: &Graphics) {
    println!("--- {name}");
    for y in 0..g.height() {
        let mut s = String::new();
        for x in 0..g.width() {
            let c = g.bitmap().get_pixel(x, y);
            s.push(if c.a == 0 { '.' } else if c.a == 255 { '#' } else { '+' });
        }
        println!("{s}");
    }
}

fn main() {
    let mut g = Graphics::new(12, 8);
    g.draw_line(Color::RED, 1.0, 1.0, 3.0, 10.0, 3.0);
    dump("draw_line horizontal (1,3)-(10,3)", &g);

    let mut g = Graphics::new(12, 8);
    g.draw_line(Color::RED, 1.0, 3.0, 1.0, 3.0, 6.0);
    dump("draw_line vertical (3,1)-(3,6)", &g);

    let mut g = Graphics::new(12, 12);
    g.draw_line(Color::RED, 1.0, 1.0, 1.0, 10.0, 10.0);
    dump("draw_line diagonal (1,1)-(10,10)", &g);

    let mut g = Graphics::new(12, 8);
    g.draw_line(Color::RED, 1.0, 1.0, 1.0, 10.0, 4.0);
    dump("draw_line shallow (1,1)-(10,4)", &g);

    let mut g = Graphics::new(12, 12);
    g.fill_polygon(Color::RED, &[(0.0, 0.0), (9.0, 0.0), (9.0, 9.0), (0.0, 9.0)]);
    dump("fill_polygon rect 0..9", &g);
    g.draw_polygon(Color::BLUE, 1.0, &[(0.0, 0.0), (9.0, 0.0), (9.0, 9.0), (0.0, 9.0)]);
    dump("+ draw_polygon rect 0..9", &g);

    let mut g = Graphics::new(14, 14);
    // bow tie / self intersecting star: GDI+ FillMode.Alternate leaves the double-wound centre empty
    let star: Vec<(f32, f32)> = (0..5).map(|i| {
        let a = -std::f32::consts::FRAC_PI_2 + i as f32 * 2.0 * 2.0 * std::f32::consts::PI / 5.0;
        (6.5 + 6.0 * a.cos(), 6.5 + 6.0 * a.sin())
    }).collect();
    g.fill_polygon(Color::RED, &star);
    dump("fill_polygon pentagram (GDI+ default is Alternate: centre empty)", &g);

    let mut g = Graphics::new(12, 12);
    g.translate(5.0, 5.0);
    g.fill_ellipse(Color::RED, -4.0, -4.0, 8.0, 8.0);
    dump("fill_ellipse centre (5,5) r4 (GDI+ centres on pixel 5,5)", &g);

    let mut g = Graphics::new(12, 12);
    g.translate(5.0, 5.0);
    g.draw_ellipse(Color::RED, 1.0, -4.0, -4.0, 8.0, 8.0);
    dump("draw_ellipse centre (5,5) r4", &g);
}
