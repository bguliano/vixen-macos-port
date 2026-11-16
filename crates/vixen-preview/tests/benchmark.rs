//! Timings on a 100,000 light scene. Run with:
//!
//!     cargo test -p vixen-preview --test benchmark -- --nocapture
//!
//! Add `--release` for the speed the app will have. This is one test so the allocation counter below sees
//! nothing else, and the limits it asserts have room for a machine busy compiling other things.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use uuid::Uuid;
use vixen_preview::data::{PreviewData, ShapeId};
use vixen_preview::overlay::Overlay;
use vixen_preview::point::PreviewPoint;
use vixen_preview::scene::Scene;
use vixen_preview::session::DesignSession;
use vixen_preview::shape::{Modifiers, Shape};

/// Counts bytes allocated, to catch anything that copies the scene.
struct Counting;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATED.fetch_add(new_size.saturating_sub(layout.size()), Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

const NONE: Modifiers = Modifiers::NONE;

/// 1000 lines of 100 lights, each linked, in a 50 by 20 grid.
fn lines_scene() -> PreviewData {
    let mut d = PreviewData::new();
    for i in 0..1000 {
        let (x, y) = ((i % 50) * 150 + 10, (i / 50) * 100 + 10);
        let mut s = Shape::new_line(PreviewPoint::new(x, y), PreviewPoint::new(x + 120, y + 60), 100, None, 1.0);
        let node = Uuid::from_u128(i as u128 + 1);
        s.for_each_pixel_mut(&mut |p| p.node_id = node);
        d.add_shape(s);
    }
    d
}

/// 100 mega trees of 16 strings of 62 lights, 99,200 lights in shapes of a realistic size.
fn trees_scene() -> PreviewData {
    let mut d = PreviewData::new();
    for i in 0..100 {
        let (x, y) = ((i % 10) * 300, (i / 10) * 300);
        let mut s = Shape::new_mega_tree(PreviewPoint::new(x, y), None, 1.0);
        s.set_property("Lights Per String", vixen_preview::shape::properties::PropertyValue::Int(62), "").unwrap();
        s.set_property("Bottom Right", vixen_preview::shape::properties::PropertyValue::Point { x: x + 200, y: y + 280 }, "").unwrap();
        d.add_shape(s);
    }
    d
}

/// One pixel grid of a thousand strings of a hundred lights: the worst case for a single shape.
fn grid_scene() -> PreviewData {
    let mut d = PreviewData::new();
    let mut s = Shape::new_pixel_grid(PreviewPoint::new(0, 0), None, 1.0);
    s.set_property("Lights Per String", vixen_preview::shape::properties::PropertyValue::Int(100), "").unwrap();
    s.set_property("String Count", vixen_preview::shape::properties::PropertyValue::Int(1000), "").unwrap();
    s.set_property("Bottom Right", vixen_preview::shape::properties::PropertyValue::Point { x: 7000, y: 4000 }, "").unwrap();
    d.add_shape(s);
    d
}

#[derive(Clone, Copy)]
struct Stats {
    mean: Duration,
    median: Duration,
    p99: Duration,
    max: Duration,
}

fn time_each(n: usize, mut f: impl FnMut(usize)) -> Stats {
    let mut samples = Vec::with_capacity(n);
    for i in 0..n {
        let t = Instant::now();
        f(i);
        samples.push(t.elapsed());
    }
    samples.sort();
    let total: Duration = samples.iter().sum();
    Stats { mean: total / n as u32, median: samples[n / 2], p99: samples[n * 99 / 100], max: samples[n - 1] }
}

fn show(name: &str, s: Stats) {
    println!("{name:<62} mean {:>9.1?}  median {:>9.1?}  p99 {:>9.1?}  max {:>9.1?}", s.mean, s.median, s.p99, s.max);
}

fn check(name: &str, s: Stats, limit_us: u128) {
    show(name, s);
    assert!(s.median.as_micros() < limit_us, "{name}: median {:?} is over {limit_us} us", s.median);
}

fn click(s: &mut DesignSession, x: i32, y: i32, m: Modifiers) {
    s.mouse_down(x, y, m);
    s.mouse_up(x, y, m);
}

#[test]
fn hundred_thousand_lights() {
    println!();
    println!("build profile: {}", if cfg!(debug_assertions) { "dev (opt-level 1, overflow checks on). Use --release for app speed" } else { "release" });

    // ----- loading and building
    let data = lines_scene();
    assert_eq!(data.pixel_count(), 100_000);
    let t = Instant::now();
    let scene = Scene::build(&data, None);
    println!("Scene::build, 100,000 lights in 1000 shapes:                    {:>9.1?}", t.elapsed());
    assert_eq!(scene.light_count(), 100_000);
    let t = Instant::now();
    let xml = data.to_xml();
    println!("to_xml, the same:                                               {:>9.1?}", t.elapsed());
    let t = Instant::now();
    let loaded = PreviewData::from_xml(&xml).unwrap();
    println!("from_xml with all shapes laid out, the same:                    {:>9.1?}", t.elapsed());
    assert_eq!(loaded.pixel_count(), 100_000);

    // ----- dragging one 100 light shape among 100,000
    let mut s = DesignSession::new(data, None);
    click(&mut s, 10, 10, NONE);
    let id = s.selected_item().expect("clicked on the first line");
    s.mouse_down(10, 10, NONE);
    let (lights_at, links_at, cap) = (s.scene().lights.as_ptr(), s.scene().links.as_ptr(), s.scene().lights.capacity());
    let before = ALLOCATED.load(Ordering::Relaxed);
    let n = 2000;
    let drag = time_each(n, |i| {
        let ch = s.mouse_move(10 + (i % 300) as i32, 10 + (i % 200) as i32, NONE);
        assert_eq!(ch.scene.len(), 1);
    });
    let allocated = ALLOCATED.load(Ordering::Relaxed) - before;
    assert_eq!(s.scene().lights.as_ptr(), lights_at, "the scene's buffer stays where it is");
    assert_eq!(s.scene().links.as_ptr(), links_at);
    assert_eq!(s.scene().lights.capacity(), cap);
    check("drag: one 100 light line among 100,000 lights, per mouse event", drag, 300);
    let per_event = allocated / n;
    println!("   allocated per mouse event: {per_event} bytes. The scene's lights alone are {} bytes", 100_000 * std::mem::size_of::<vixen_preview::scene::LightGeom>());
    assert!(per_event < 2048, "a mouse event allocated {per_event} bytes");
    s.mouse_up(10, 10, NONE);

    // Each event reports only that shape's range.
    let range = s.scene().range_of(id).unwrap();
    assert_eq!(range.len(), 100);

    // ----- a shape of one light shows the cost that is not the shape's own layout
    let mut d = lines_scene();
    let single = d.add_shape(Shape::new_single(PreviewPoint::new(9000, 9000), None, 1.0));
    let mut s2 = DesignSession::new(d, None);
    click(&mut s2, 9000, 9000, NONE);
    assert_eq!(s2.selected_item(), Some(single));
    s2.mouse_down(9000, 9000, NONE);
    let overhead = time_each(n, |i| {
        s2.mouse_move(9000 + (i % 300) as i32, 9000 + (i % 200) as i32, NONE);
    });
    check("drag: one light among 100,000: the cost besides the shape's layout", overhead, 50);
    s2.mouse_up(9000, 9000, NONE);

    // ----- hit testing
    click(&mut s, 5000, 5000, NONE); // deselect
    let miss = time_each(2000, |i| {
        // On the far side of every shape's bounds.
        s.mouse_down(9000 + (i % 100) as i32, 9000, NONE);
        s.mouse_up(9000 + (i % 100) as i32, 9000, NONE);
    });
    check("click on empty space: every shape's bounds rejected", miss, 200);
    let hit_last = time_each(2000, |_| {
        // The last shape in the list, the most shapes to get through.
        s.mouse_down(7360, 1910, NONE);
        s.mouse_up(7360, 1910, NONE);
        s.mouse_down(5000, 5000, NONE);
        s.mouse_up(5000, 5000, NONE);
    });
    check("click to select the last of 1000 shapes, then deselect", hit_last, 1500);
    click(&mut s, 7360, 1910, NONE);
    assert!(s.selected_item().is_some());
    let hover = time_each(5000, |i| {
        s.mouse_move(7360 + (i % 60) as i32, 1910 + (i % 30) as i32, NONE);
    });
    check("hover with a shape selected: handles, then its pixels", hover, 50);
    click(&mut s, 5000, 5000, NONE);
    let idle = time_each(5000, |i| {
        s.mouse_move((i % 7000) as i32, (i % 2000) as i32, NONE);
    });
    check("hover with nothing selected: no hit testing at all", idle, 5);

    // ----- rubber band
    s.mouse_down(5, 5, NONE);
    let band = time_each(500, |i| {
        s.mouse_move(100 + (i % 3000) as i32, 100 + (i % 1000) as i32, NONE);
    });
    check("rubber band: a mouse event over 1000 shapes", band, 1500);
    s.mouse_up(100, 100, NONE);
    s.clear_selection();

    // ----- multi-select drag
    let ids: Vec<ShapeId> = s.data().display_items.iter().skip(1).take(100).map(|i| i.id).collect();
    s.select_items(&ids);
    assert_eq!(s.selected_items().len(), 100);
    // The second line in the list still starts at its original place.
    s.mouse_down(160, 10, NONE);
    let multi = time_each(500, |i| {
        let ch = s.mouse_move(160 + (i % 300) as i32, 10 + (i % 200) as i32, NONE);
        assert_eq!(ch.scene.len(), 100);
    });
    check("multi-select drag of 100 lines, 10,000 lights moved", multi, 3000);
    s.mouse_up(160, 10, NONE);

    // ----- the overlay
    let over = time_each(5000, |_| {
        let o: Overlay = s.overlay();
        assert!(!o.light_runs.is_empty());
    });
    check("overlay(): shared runs, handles only", over, 20);
    s.clear_selection();
    click(&mut s, 7360, 1910, NONE);
    let over = time_each(5000, |_| {
        let o: Overlay = s.overlay();
        assert_eq!(o.squares.len(), 2);
    });
    check("overlay() with one shape selected", over, 20);

    // ----- selection change
    let select = time_each(200, |i| {
        s.mouse_down(5000, 5000, NONE);
        s.mouse_up(5000, 5000, NONE);
        let _ = i;
        s.mouse_down(7360, 1910, NONE);
        s.mouse_up(7360, 1910, NONE);
    });
    check("select and deselect, with the colour runs rebuilt each time", select, 5000);

    // ----- undo of a move
    click(&mut s, 7360, 1910, NONE);
    s.mouse_down(7360, 1910, NONE);
    s.mouse_move(7380, 1920, NONE);
    s.mouse_up(7380, 1920, NONE);
    let undo = time_each(200, |i| {
        if i % 2 == 0 {
            s.undo();
        } else {
            s.redo();
        }
    });
    check("undo or redo of a move", undo, 2000);

    // ----- zoom: every light moves, once per zoom step, not per mouse move
    let zoom = time_each(20, |i| {
        s.set_zoom(1.0 + (i % 4) as f64 * 0.25);
    });
    show("set_zoom: every shape lays out again", zoom);
    s.set_zoom(1.0);

    // ----- larger shapes
    let d = trees_scene();
    println!("--- 100 mega trees, {} lights", d.pixel_count());
    let mut t = DesignSession::new(d, None);
    click(&mut t, 100, 280, NONE);
    let tree = t.selected_item();
    assert!(tree.is_some(), "clicked on a light of the first tree");
    t.mouse_down(100, 280, NONE);
    let drag_tree = time_each(500, |i| {
        t.mouse_move(100 + (i % 300) as i32, 280 + (i % 200) as i32, NONE);
    });
    check("drag: one 992 light mega tree", drag_tree, 400);
    t.mouse_up(100, 280, NONE);

    let d = grid_scene();
    println!("--- one pixel grid, {} lights", d.pixel_count());
    let mut g = DesignSession::new(d, None);
    click(&mut g, 7, 4000, NONE);
    assert!(g.selected_item().is_some(), "clicked on a light");
    g.mouse_down(7, 4000, NONE);
    let drag_grid = time_each(50, |i| {
        g.mouse_move(7 + (i % 300) as i32, 4000 + (i % 200) as i32, NONE);
    });
    show("drag: the whole 100,000 light shape, all its own layout", drag_grid);
    g.mouse_up(7, 4000, NONE);
    // The next hit test builds the shape's grid, once, and after that a hover does not scan its lights.
    g.mouse_move(4000, 2000, NONE);
    let hover_grid = time_each(5000, |i| {
        g.mouse_move(1000 + (i % 3000) as i32, 1000 + (i % 2000) as i32, NONE);
    });
    check("hover over a 100,000 light shape: its grid, not its lights", hover_grid, 50);
    println!();
}
