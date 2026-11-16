//! What the renderer draws, read back from an offscreen target. No window involved.
//!
//! Expected values come from the original: the footprint from `FastPixel.DrawCircle`, blending from
//! `FastPixel.SetPixel`, dimming from `GDIControl.CreateAlphaBackground`, and positions from `PreviewPixel.Draw`.
//!
//! Set `VIXEN_RENDER_REQUIRE_GPU` to fail instead of skipping when there is no adapter.

use vixen_render::{rgba, FrameSkip, LightGeom, Overlay, OverlayLine, OverlaySquare, RenderError, Renderer};


const RED: u32 = rgba(255, 0, 0, 255);
const GREEN: u32 = rgba(0, 255, 0, 255);
const WHITE: u32 = rgba(255, 255, 255, 255);

const BLACK_PX: [u8; 4] = [0, 0, 0, 255];
const RED_PX: [u8; 4] = [255, 0, 0, 255];
const GREEN_PX: [u8; 4] = [0, 255, 0, 255];
const BLUE_PX: [u8; 4] = [0, 0, 255, 255];
const WHITE_PX: [u8; 4] = [255, 255, 255, 255];

fn gpu(width: u32, height: u32, scale: f32) -> Option<Renderer> {
    match Renderer::headless(width, height, scale) {
        Ok(renderer) => Some(renderer),
        Err(RenderError::NoAdapter) if std::env::var_os("VIXEN_RENDER_REQUIRE_GPU").is_none() => {
            eprintln!("skipping: no GPU adapter on this machine");
            None
        }
        Err(error) => panic!("{error}"),
    }
}

macro_rules! renderer {
    ($width:expr, $height:expr, $scale:expr) => {
        match gpu($width, $height, $scale) {
            Some(renderer) => renderer,
            None => return,
        }
    };
    ($width:expr, $height:expr) => {
        renderer!($width, $height, 1.0)
    };
}

struct Image {
    pixels: Vec<u8>,
    width: i32,
    height: i32,
}

impl Image {
    fn at(&self, x: i32, y: i32) -> [u8; 4] {
        assert!((0..self.width).contains(&x) && (0..self.height).contains(&y), "({x}, {y}) is off the image");
        let at = ((y * self.width + x) * 4) as usize;
        self.pixels[at..at + 4].try_into().unwrap()
    }

    /// Every pixel with `f`'s value, as (x, y).
    fn find(&self, f: impl Fn([u8; 4]) -> bool) -> Vec<(i32, i32)> {
        let mut found = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if f(self.at(x, y)) {
                    found.push((x, y));
                }
            }
        }
        found
    }

    fn count(&self, color: [u8; 4]) -> usize {
        self.find(|pixel| pixel == color).len()
    }
}

fn draw(renderer: &mut Renderer, width: u32, height: u32) -> Image {
    renderer.render().unwrap();
    let pixels = renderer.read_pixels();
    assert_eq!(pixels.len(), (width * height * 4) as usize);
    Image { pixels, width: width as i32, height: height as i32 }
}

fn light(x: f32, y: f32, size: f32) -> LightGeom {
    LightGeom { x, y, size }
}

fn solid_image(width: u32, height: u32, pixel: [u8; 4]) -> Vec<u8> {
    pixel.iter().copied().cycle().take((width * height * 4) as usize).collect()
}

fn assert_close(actual: [u8; 4], expected: [u8; 4], tolerance: i32, what: &str) {
    for channel in 0..4 {
        let difference = actual[channel] as i32 - expected[channel] as i32;
        assert!(difference.abs() <= tolerance, "{what}: got {actual:?}, expected {expected:?} within {tolerance}");
    }
}

/// The pixels `FastPixel.DrawCircle(Rectangle, Color)` writes, relative to the centre.
fn fastpixel_footprint(size: i32) -> Vec<(i32, i32)> {
    if size <= 0 {
        return Vec::new();
    }
    if size == 1 {
        return vec![(0, 0)];
    }
    let radius = size / 2;
    let mut pixels = Vec::new();
    for y in -radius..=radius {
        for x in -radius..=radius {
            if x * x + y * y <= radius * radius {
                pixels.push((x, y));
            }
        }
    }
    pixels
}

// ---------------------------------------------------------------------------------------------------------
// Lights
// ---------------------------------------------------------------------------------------------------------

#[test]
fn a_light_lands_on_its_position_and_the_background_shows_elsewhere() {
    let mut r = renderer!(64, 48);
    r.set_background(&solid_image(64, 48, BLUE_PX), 64, 48);
    r.set_lights(&[light(40.0, 12.0, 9.0)]);
    r.set_light_colors(&[RED]);
    let image = draw(&mut r, 64, 48);

    assert_eq!(image.at(40, 12), RED_PX);
    assert_eq!(image.at(43, 12), RED_PX);
    assert_eq!(image.at(40, 9), RED_PX);
    assert_eq!(image.at(46, 12), BLUE_PX, "beyond the radius");
    // Off to the side, and where a flipped or mirrored image would have put it.
    assert_eq!(image.at(5, 5), BLUE_PX);
    assert_eq!(image.at(40, 48 - 12), BLUE_PX, "not flipped vertically");
    assert_eq!(image.at(64 - 40, 12), BLUE_PX, "not flipped horizontally");
}

#[test]
fn channels_are_not_swapped() {
    let mut r = renderer!(16, 16);
    r.set_lights(&[light(8.0, 8.0, 5.0)]);
    r.set_light_colors(&[rgba(10, 20, 30, 255)]);
    assert_eq!(draw(&mut r, 16, 16).at(8, 8), [10, 20, 30, 255]);

    // The documented layout: the bytes of the u32 in memory are R, G, B, A.
    assert_eq!(rgba(1, 2, 3, 4).to_ne_bytes(), [1, 2, 3, 4]);
}

#[test]
fn background_image_appears_the_right_way_up() {
    let mut image = Vec::new();
    for pixel in [RED_PX, GREEN_PX, BLUE_PX, WHITE_PX] {
        image.extend_from_slice(&pixel);
    }
    let mut r = renderer!(2, 2);
    r.set_background(&image, 2, 2);
    let drawn = draw(&mut r, 2, 2);
    assert_eq!(drawn.at(0, 0), RED_PX);
    assert_eq!(drawn.at(1, 0), GREEN_PX);
    assert_eq!(drawn.at(0, 1), BLUE_PX);
    assert_eq!(drawn.at(1, 1), WHITE_PX);
}

/// Every pixel the original lights is lit here too, and nothing strays further than the anti-aliased edge.
#[test]
fn footprint_matches_fastpixel_at_zoom_one() {
    const CENTER: i32 = 32;
    for size in 1..=20 {
        let mut r = renderer!(64, 64);
        r.set_lights(&[light(CENTER as f32, CENTER as f32, size as f32)]);
        r.set_light_colors(&[RED]);
        let image = draw(&mut r, 64, 64);

        let footprint = fastpixel_footprint(size);
        let radius = size / 2;
        for &(x, y) in &footprint {
            let [red, ..] = image.at(CENTER + x, CENTER + y);
            assert!(red >= 190, "size {size}: ({x}, {y}) should be lit, got {red}");
        }
        for y in 0..64 {
            for x in 0..64 {
                let (dx, dy) = (x - CENTER, y - CENTER);
                if ((dx * dx + dy * dy) as f32).sqrt() > radius as f32 + 1.0 {
                    assert_eq!(image.at(x, y), BLACK_PX, "size {size}: ({dx}, {dy}) should be dark");
                }
            }
        }
    }
}

#[test]
fn size_one_lights_exactly_one_pixel() {
    let mut r = renderer!(32, 32);
    r.set_lights(&[light(10.0, 20.0, 1.0)]);
    r.set_light_colors(&[RED]);
    let image = draw(&mut r, 32, 32);
    assert_eq!(image.find(|pixel| pixel != BLACK_PX), [(10, 20)]);
    assert_eq!(image.at(10, 20), RED_PX);
}

#[test]
fn size_two_draws_the_plus_of_the_original_at_full_colour() {
    for size in [2.0, 3.0] {
        let mut r = renderer!(32, 32);
        r.set_lights(&[light(10.0, 20.0, size)]);
        r.set_light_colors(&[RED]);
        let image = draw(&mut r, 32, 32);

        for (x, y) in [(10, 20), (9, 20), (11, 20), (10, 19), (10, 21)] {
            assert_eq!(image.at(x, y), RED_PX, "size {size}: ({x}, {y})");
        }
        for (x, y) in [(9, 19), (11, 19), (9, 21), (11, 21)] {
            let [red, ..] = image.at(x, y);
            assert!(red < 80, "size {size}: the corner ({x}, {y}) is only a soft edge, got {red}");
        }
        assert_eq!(image.at(8, 20), BLACK_PX);
        assert_eq!(image.at(10, 22), BLACK_PX);
    }
}

/// An analytic circle at a fractional position would smear one pixel's worth of light across four pixels.
#[test]
fn small_lights_keep_full_colour_at_fractional_zoom_and_truncate_like_the_original() {
    for zoom in [0.25f32, 0.7, 1.1, 1.37, 1.5, 1.75, 2.0] {
        for &(x, y) in &[(3.0f32, 7.0f32), (11.0, 13.0), (17.0, 29.0), (23.0, 31.0), (33.0, 2.0)] {
            let mut r = renderer!(128, 128);
            r.set_view(zoom, 0.0, 0.0);
            r.set_lights(&[light(x, y, 1.0)]);
            r.set_light_colors(&[RED]);
            let image = draw(&mut r, 128, 128);

            // new Rectangle((int)(Bounds.X * zoomLevel), (int)(Bounds.Y * zoomLevel), …)
            let expected = ((x * zoom) as i32, (y * zoom) as i32);
            assert_eq!(image.find(|pixel| pixel != BLACK_PX), [expected], "zoom {zoom}, ({x}, {y})");
            assert_eq!(image.at(expected.0, expected.1), RED_PX, "zoom {zoom}, ({x}, {y})");
        }
    }
}

#[test]
fn every_small_light_has_the_same_shape_at_fractional_zoom() {
    let mut r = renderer!(256, 64);
    r.set_view(1.37, 0.0, 0.0);
    let lights: Vec<_> = (0..16).map(|i| light(5.0 + i as f32 * 10.0, 20.0, 3.0)).collect();
    r.set_light_colors(&vec![RED; lights.len()]);
    r.set_lights(&lights);
    let image = draw(&mut r, 256, 64);
    assert_eq!(image.count(RED_PX), lights.len() * 5, "each is a five pixel plus, wherever it fell");
}

#[test]
fn sizes_below_one_stay_visible_and_zero_or_invalid_sizes_draw_nothing() {
    let mut r = renderer!(32, 32);
    r.set_lights(&[light(5.0, 5.0, 0.2), light(10.0, 5.0, 1.5)]);
    r.set_light_colors(&[RED, RED]);
    assert_eq!(draw(&mut r, 32, 32).find(|pixel| pixel == RED_PX), [(5, 5), (10, 5)]);

    r.set_lights(&[light(5.0, 5.0, 0.0), light(10.0, 5.0, -3.0), light(15.0, 5.0, f32::NAN)]);
    r.set_light_colors(&[RED, RED, RED]);
    assert_eq!(draw(&mut r, 32, 32).count(BLACK_PX), 32 * 32);
}

#[test]
fn zero_alpha_draws_nothing() {
    let mut r = renderer!(32, 32);
    r.set_background(&solid_image(32, 32, BLUE_PX), 32, 32);
    r.set_lights(&[light(16.0, 16.0, 11.0)]);
    r.set_light_colors(&[rgba(255, 0, 0, 0)]);
    assert_eq!(draw(&mut r, 32, 32).count(BLUE_PX), 32 * 32);
}

// ---------------------------------------------------------------------------------------------------------
// Blending
// ---------------------------------------------------------------------------------------------------------

/// `FastPixel.SetPixelAlpha`, which truncates to a byte where the GPU rounds.
fn fastpixel_blend(color: [u8; 4], background: [u8; 3]) -> [u8; 4] {
    let alpha_percent = color[3] as f32 / 255.0;
    let alpha_offset = 255.0 - color[3] as f32;
    let blend = |c: u8, b: u8| (c as f32 * alpha_percent + b as f32 * alpha_offset / 255.0) as u8;
    [blend(color[0], background[0]), blend(color[1], background[1]), blend(color[2], background[2]), 255]
}

#[test]
fn alpha_blends_over_the_background_like_fastpixel() {
    let background = [100, 150, 200];
    for color in [[255, 0, 50, 128], [10, 200, 30, 64], [255, 255, 255, 200], [40, 80, 120, 1], [90, 90, 90, 254]] {
        let mut r = renderer!(32, 32);
        r.set_background(&solid_image(32, 32, [background[0], background[1], background[2], 255]), 32, 32);
        r.set_lights(&[light(16.0, 16.0, 9.0)]);
        r.set_light_colors(&[rgba(color[0], color[1], color[2], color[3])]);
        let image = draw(&mut r, 32, 32);
        assert_close(image.at(16, 16), fastpixel_blend(color, background), 2, "blended centre");
        assert_eq!(image.at(2, 2), [100, 150, 200, 255], "the background beside it");
    }
}

#[test]
fn blending_is_in_encoded_values_not_linear_light() {
    let mut r = renderer!(16, 16);
    r.set_lights(&[light(8.0, 8.0, 5.0)]);
    r.set_light_colors(&[rgba(255, 0, 0, 128)]);
    // A sRGB target would blend in linear light and give 188.
    assert_close(draw(&mut r, 16, 16).at(8, 8), [128, 0, 0, 255], 1, "50% red over black");
}

#[test]
fn later_lights_blend_over_earlier_ones() {
    let mut r = renderer!(32, 32);
    r.set_lights(&[light(16.0, 16.0, 9.0), light(16.0, 16.0, 9.0)]);
    r.set_light_colors(&[RED, rgba(0, 0, 255, 128)]);
    let expected = fastpixel_blend([0, 0, 255, 128], [255, 0, 0]);
    assert_close(draw(&mut r, 32, 32).at(16, 16), expected, 2, "second over first");

    r.set_light_colors(&[rgba(0, 0, 255, 128), RED]);
    assert_eq!(draw(&mut r, 32, 32).at(16, 16), RED_PX, "opaque second hides first");
}

#[test]
fn the_frame_stays_opaque_under_blending() {
    let mut r = renderer!(32, 32);
    r.set_lights(&[light(16.0, 16.0, 9.0), light(20.0, 16.0, 9.0)]);
    r.set_light_colors(&[rgba(255, 0, 0, 77), rgba(0, 255, 0, 77)]);
    let image = draw(&mut r, 32, 32);
    assert!(image.pixels.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 255));
}

// ---------------------------------------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------------------------------------

#[test]
fn background_alpha_dims_the_image_over_black() {
    let color = [200, 100, 50, 255];
    let mut r = renderer!(16, 16);
    r.set_background(&solid_image(16, 16, color), 16, 16);

    // CreateAlphaBackground: black at alpha (255 - BackgroundAlpha) over the image.
    assert_eq!(draw(&mut r, 16, 16).at(8, 8), color, "full by default");
    for alpha in [255u8, 200, 128, 50, 1, 0] {
        r.set_background_alpha(alpha);
        let dim = |c: u8| (c as f32 * alpha as f32 / 255.0).round() as u8;
        let expected = [dim(color[0]), dim(color[1]), dim(color[2]), 255];
        assert_close(draw(&mut r, 16, 16).at(8, 8), expected, 1, &format!("alpha {alpha}"));
    }
}

#[test]
fn background_alpha_leaves_lights_alone() {
    let mut r = renderer!(16, 16);
    r.set_background(&solid_image(16, 16, BLUE_PX), 16, 16);
    r.set_background_alpha(40);
    r.set_lights(&[light(8.0, 8.0, 5.0)]);
    r.set_light_colors(&[RED]);
    let image = draw(&mut r, 16, 16);
    assert_eq!(image.at(8, 8), RED_PX);
    assert_close(image.at(1, 1), [0, 0, 40, 255], 1, "dimmed background");
}

#[test]
fn transparency_in_the_image_shows_black() {
    let mut r = renderer!(8, 8);
    r.set_background(&solid_image(8, 8, [255, 255, 255, 128]), 8, 8);
    assert_close(draw(&mut r, 8, 8).at(4, 4), [128, 128, 128, 255], 1, "half transparent white");
    r.set_background_alpha(128);
    assert_close(draw(&mut r, 8, 8).at(4, 4), [64, 64, 64, 255], 1, "and dimmed");
}

#[test]
fn outside_the_image_is_black_and_clearing_removes_it() {
    let mut r = renderer!(16, 16);
    r.set_background(&solid_image(8, 8, WHITE_PX), 8, 8);
    let image = draw(&mut r, 16, 16);
    assert_eq!(image.at(7, 7), WHITE_PX);
    assert_eq!(image.at(8, 7), BLACK_PX);
    assert_eq!(image.at(7, 8), BLACK_PX);
    assert_eq!(image.at(15, 15), BLACK_PX);

    r.clear_background();
    assert_eq!(draw(&mut r, 16, 16).count(BLACK_PX), 16 * 16);
}

#[test]
fn replacing_the_background_takes_effect_and_works_after_many_sizes() {
    let mut r = renderer!(8, 8);
    for (size, color) in [(1, RED_PX), (2, GREEN_PX), (3, BLUE_PX), (64, WHITE_PX), (5, RED_PX)] {
        r.set_background(&solid_image(size, size, color), size, size);
        r.set_view(8.0 / size as f32, 0.0, 0.0);
        assert_close(draw(&mut r, 8, 8).at(4, 4), color, 1, &format!("{size} x {size}"));
    }
}

#[test]
fn zoomed_out_background_is_filtered_not_decimated() {
    // A checkerboard of single pixels averages to grey. Point-sampling it at 1/4 would give a solid colour.
    let size = 64;
    let mut image = Vec::new();
    for y in 0..size {
        for x in 0..size {
            image.extend_from_slice(if (x + y) % 2 == 0 { &WHITE_PX } else { &BLACK_PX });
        }
    }
    let mut r = renderer!(16, 16);
    r.set_background(&image, size, size);
    r.set_view(0.25, 0.0, 0.0);
    let drawn = draw(&mut r, 16, 16);
    for (x, y) in [(2, 2), (8, 8), (13, 5)] {
        let [value, ..] = drawn.at(x, y);
        assert!((100..=155).contains(&value), "({x}, {y}) should be near mid grey, got {value}");
    }
}

// ---------------------------------------------------------------------------------------------------------
// Overlay
// ---------------------------------------------------------------------------------------------------------

fn line(a: [f32; 2], b: [f32; 2], color: u32, width: f32) -> OverlayLine {
    OverlayLine { a, b, color, width }
}

fn lines(lines: Vec<OverlayLine>) -> Overlay {
    Overlay { lines, ..Default::default() }
}

fn lit_rows_in_column(image: &Image, x: i32) -> Vec<i32> {
    (0..image.height).filter(|&y| image.at(x, y) != BLACK_PX).collect()
}

fn lit_columns_in_row(image: &Image, y: i32) -> Vec<i32> {
    (0..image.width).filter(|&x| image.at(x, y) != BLACK_PX).collect()
}

#[test]
fn a_line_covers_its_pixels_end_to_end() {
    let mut r = renderer!(100, 100);
    r.set_overlay(lines(vec![line([10.0, 50.0], [89.0, 50.0], WHITE, 1.0)]));
    let image = draw(&mut r, 100, 100);

    assert_eq!(lit_rows_in_column(&image, 50), [50], "one pixel thick");
    assert_eq!(lit_columns_in_row(&image, 50), (10..=89).collect::<Vec<_>>(), "both ends included");
    assert_eq!(image.at(50, 50), WHITE_PX, "at full colour");
}

#[test]
fn line_width_and_position_under_zoom() {
    for zoom in [0.5f32, 1.0, 1.25, 1.37, 2.0, 3.5] {
        for width in [1u32, 2, 3, 5] {
            let mut r = renderer!(200, 200);
            r.set_view(zoom, 0.0, 0.0);
            r.set_overlay(lines(vec![
                line([4.0, 20.0], [40.0, 20.0], WHITE, width as f32),
                line([20.0, 4.0], [20.0, 40.0], WHITE, width as f32),
            ]));
            let image = draw(&mut r, 200, 200);

            let column = (8.0 * zoom) as i32; // Away from the crossing.
            let rows = lit_rows_in_column(&image, column);
            assert_eq!(rows.len() as u32, width, "zoom {zoom}, width {width}: {rows:?}");
            assert!(
                rows.contains(&((20.0 * zoom) as i32)),
                "zoom {zoom}, width {width}: {rows:?} should include the scene position"
            );
            assert!(rows.iter().all(|&y| image.at(column, y) == WHITE_PX), "crisp at zoom {zoom}");

            let row = (8.0 * zoom) as i32;
            let columns = lit_columns_in_row(&image, row);
            assert_eq!(columns.len() as u32, width, "zoom {zoom}, width {width}: {columns:?}");
            assert!(columns.contains(&((20.0 * zoom) as i32)), "zoom {zoom}, width {width}");
        }
    }
}

#[test]
fn sloped_lines_keep_their_width_under_zoom() {
    // Coverage summed across a line, along a row, is width / cos(45 degrees).
    for zoom in [1.0f32, 2.0, 3.3] {
        let mut r = renderer!(256, 256);
        r.set_view(zoom, 0.0, 0.0);
        r.set_overlay(lines(vec![line([5.0, 5.0], [70.0, 70.0], WHITE, 2.0)]));
        let image = draw(&mut r, 256, 256);

        let y = (40.0 * zoom) as i32;
        let coverage: f32 = (0..256).map(|x| image.at(x, y)[0] as f32 / 255.0).sum();
        let expected = 2.0 * std::f32::consts::SQRT_2;
        assert!((coverage - expected).abs() < 0.3, "zoom {zoom}: covers {coverage}, expected {expected}");
    }
}

#[test]
fn a_rubber_band_of_four_lines_closes_its_corners_at_any_zoom() {
    for zoom in [1.0f32, 1.25, 1.37] {
        let mut r = renderer!(128, 128);
        r.set_view(zoom, 0.0, 0.0);
        let (left, top, right, bottom) = (10.0, 12.0, 60.0, 50.0);
        r.set_overlay(lines(vec![
            line([left, top], [right, top], WHITE, 1.0),
            line([right, top], [right, bottom], WHITE, 1.0),
            line([right, bottom], [left, bottom], WHITE, 1.0),
            line([left, bottom], [left, top], WHITE, 1.0),
        ]));
        let image = draw(&mut r, 128, 128);
        for (x, y) in [(left, top), (right, top), (right, bottom), (left, bottom)] {
            let corner = ((x * zoom) as i32, (y * zoom) as i32);
            assert_eq!(image.at(corner.0, corner.1), WHITE_PX, "zoom {zoom}: corner {corner:?}");
        }
        assert_eq!(image.at((30.0 * zoom) as i32, (30.0 * zoom) as i32), BLACK_PX, "hollow");
    }
}

#[test]
fn a_zero_length_line_is_a_dot() {
    let mut r = renderer!(32, 32);
    r.set_overlay(lines(vec![line([10.0, 10.0], [10.0, 10.0], WHITE, 1.0)]));
    assert_eq!(draw(&mut r, 32, 32).find(|pixel| pixel != BLACK_PX), [(10, 10)]);
}

/// The pixels `FastPixel.DrawRectangle(X, Y, width, color)` writes, relative to the centre.
fn fastpixel_rectangle(width: i32) -> Vec<(i32, i32)> {
    let half = width / 2;
    let mut pixels = Vec::new();
    for y in -half..=half {
        for x in -half..=half {
            if x.abs() == half || y.abs() == half {
                pixels.push((x, y));
            }
        }
    }
    pixels
}

#[test]
fn a_square_is_the_hollow_handle_of_the_original() {
    for size in [1, 2, 4, 5, 6, 9, 10] {
        let mut r = renderer!(64, 64);
        r.set_overlay(Overlay {
            squares: vec![OverlaySquare { center: [30.0, 20.0], size: size as f32, color: WHITE }],
            ..Default::default()
        });
        let image = draw(&mut r, 64, 64);

        let mut expected: Vec<_> =
            fastpixel_rectangle(size).into_iter().map(|(x, y)| (30 + x, 20 + y)).collect();
        expected.sort_by_key(|&(x, y)| (y, x));
        assert_eq!(image.find(|pixel| pixel != BLACK_PX), expected, "size {size}");
        assert_eq!(image.count(WHITE_PX), expected.len(), "size {size}: crisp");
    }
}

#[test]
fn a_square_moves_with_zoom_but_keeps_its_size() {
    for zoom in [0.5f32, 1.25, 2.0, 3.0] {
        let mut r = renderer!(256, 256);
        r.set_view(zoom, 0.0, 0.0);
        r.set_overlay(Overlay {
            squares: vec![OverlaySquare { center: [40.0, 30.0], size: 6.0, color: WHITE }],
            ..Default::default()
        });
        let image = draw(&mut r, 256, 256);
        let center = ((40.0 * zoom) as i32, (30.0 * zoom) as i32);
        assert_eq!(image.count(WHITE_PX), fastpixel_rectangle(6).len(), "zoom {zoom}");
        for (x, y) in fastpixel_rectangle(6) {
            assert_eq!(image.at(center.0 + x, center.1 + y), WHITE_PX, "zoom {zoom}: ({x}, {y})");
        }
    }
}

#[test]
fn a_circle_is_a_filled_disc_that_moves_with_zoom_but_keeps_its_size() {
    for zoom in [1.0f32, 2.0] {
        let mut r = renderer!(128, 128);
        r.set_view(zoom, 0.0, 0.0);
        r.set_overlay(Overlay {
            circles: vec![([30.0, 20.0], 5.0, GREEN)],
            ..Default::default()
        });
        let image = draw(&mut r, 128, 128);

        let (x, y) = ((30.0 * zoom) as i32, (20.0 * zoom) as i32);
        assert_eq!(image.at(x, y), GREEN_PX, "zoom {zoom}: filled at the centre");
        assert_eq!(image.at(x + 4, y), GREEN_PX, "zoom {zoom}: inside");
        assert_eq!(image.at(x, y - 4), GREEN_PX, "zoom {zoom}: inside");
        assert_eq!(image.at(x + 7, y), BLACK_PX, "zoom {zoom}: the radius didn't scale");
        assert_eq!(image.at(x, y + 7), BLACK_PX, "zoom {zoom}: the radius didn't scale");
    }
}

#[test]
fn the_overlay_is_drawn_above_the_lights_and_blends() {
    let mut r = renderer!(32, 32);
    r.set_lights(&[light(16.0, 16.0, 15.0)]);
    r.set_light_colors(&[RED]);
    r.set_overlay(lines(vec![line([0.0, 16.0], [31.0, 16.0], WHITE, 1.0)]));
    let image = draw(&mut r, 32, 32);
    assert_eq!(image.at(16, 16), WHITE_PX, "over the light");
    assert_eq!(image.at(16, 14), RED_PX, "the light beside it");

    r.set_overlay(lines(vec![line([0.0, 16.0], [31.0, 16.0], rgba(0, 0, 255, 128), 1.0)]));
    let expected = fastpixel_blend([0, 0, 255, 128], [255, 0, 0]);
    assert_close(draw(&mut r, 32, 32).at(16, 16), expected, 2, "translucent stroke");

    r.set_overlay(Overlay::default());
    assert_eq!(draw(&mut r, 32, 32).at(16, 16), RED_PX, "and gone again");
}

#[test]
fn overlay_widths_double_on_hidpi_but_not_under_zoom() {
    let mut r = renderer!(200, 200, 2.0);
    r.set_view(2.0, 0.0, 0.0);
    r.set_overlay(Overlay {
        lines: vec![line([5.0, 10.0], [45.0, 10.0], WHITE, 1.0)],
        squares: vec![OverlaySquare { center: [30.0, 30.0], size: 6.0, color: GREEN }],
        circles: vec![],
    });
    let image = draw(&mut r, 200, 200);

    // Scene 10 at zoom 2 is logical row 20, which is physical rows 40 and 41.
    assert_eq!(lit_rows_in_column(&image, 60), [40, 41]);
    assert_eq!(image.at(60, 40), WHITE_PX);

    // Logical pixels 57 to 63 across: physical 114 to 127, with a 2 pixel outline.
    let columns: Vec<_> = (0..200).filter(|&x| image.at(x, 120) == GREEN_PX).collect();
    assert_eq!(columns, [114, 115, 126, 127]);
}

// ---------------------------------------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------------------------------------

#[test]
fn zoom_moves_lights_without_resizing_them() {
    let footprint = |zoom: f32| -> Option<(i32, i32, usize)> {
        let mut r = gpu(200, 200, 1.0)?;
        r.set_view(zoom, 0.0, 0.0);
        r.set_lights(&[light(30.0, 40.0, 7.0)]);
        r.set_light_colors(&[RED]);
        let image = draw(&mut r, 200, 200);
        let lit = image.find(|pixel| pixel == RED_PX);
        let center_x = lit.iter().map(|p| p.0).sum::<i32>() / lit.len() as i32;
        let center_y = lit.iter().map(|p| p.1).sum::<i32>() / lit.len() as i32;
        Some((center_x, center_y, lit.len()))
    };
    let Some((x1, y1, area1)) = footprint(1.0) else { return };
    let (x2, y2, area2) = footprint(2.0).unwrap();
    let (x3, y3, area3) = footprint(0.5).unwrap();
    assert_eq!((x1, y1), (30, 40));
    assert_eq!((x2, y2), (60, 80));
    assert_eq!((x3, y3), (15, 20));
    assert_eq!(area1, area2, "same size at zoom 2");
    assert_eq!(area1, area3, "same size at zoom 0.5");
}

#[test]
fn pan_is_applied_after_zoom_in_logical_pixels() {
    let mut r = renderer!(100, 100);
    r.set_view(2.0, 7.0, -3.0);
    r.set_lights(&[light(10.0, 20.0, 1.0)]);
    r.set_light_colors(&[RED]);
    assert_eq!(draw(&mut r, 100, 100).find(|pixel| pixel == RED_PX), [(27, 37)]);

    r.set_view(1.0, 0.0, 0.0);
    assert_eq!(draw(&mut r, 100, 100).find(|pixel| pixel == RED_PX), [(10, 20)], "and back");
}

#[test]
fn hidpi_scale_applies_to_positions_sizes_and_pan() {
    let mut r = renderer!(200, 200, 2.0);
    r.set_view(1.0, 5.0, 5.0);
    r.set_lights(&[light(20.0, 10.0, 1.0)]);
    r.set_light_colors(&[RED]);
    let image = draw(&mut r, 200, 200);
    // Logical pixel (25, 15) is the 2 x 2 block starting at physical (50, 30), all of it at full colour.
    assert_eq!(image.find(|pixel| pixel != BLACK_PX), [(50, 30), (51, 30), (50, 31), (51, 31)]);
    assert_eq!(image.count(RED_PX), 4);

    r.set_lights(&[light(20.0, 10.0, 9.0)]);
    let image = draw(&mut r, 200, 200);
    // Logical radius about 4, so physical radius about 8, around physical (51, 31).
    assert_eq!(image.at(51 + 7, 31), RED_PX);
    assert_eq!(image.at(51, 31 - 7), RED_PX);
    assert_eq!(image.at(51 + 10, 31), BLACK_PX);
    assert_eq!(image.at(51, 31 + 10), BLACK_PX);
}

#[test]
fn hidpi_frame_matches_the_same_scene_at_scale_one() {
    // Every lit logical pixel at scale 1 is lit in the middle of its 2 x 2 block at scale 2.
    let scene = |renderer: &mut Renderer| {
        renderer.set_view(1.5, 4.0, 2.0);
        renderer.set_lights(&[light(10.0, 12.0, 5.0), light(30.0, 8.0, 7.0), light(20.0, 30.0, 11.0)]);
        renderer.set_light_colors(&[RED, RED, RED]);
    };
    let mut one = renderer!(100, 100, 1.0);
    let mut two = renderer!(200, 200, 2.0);
    scene(&mut one);
    scene(&mut two);
    let (one, two) = (draw(&mut one, 100, 100), draw(&mut two, 200, 200));

    for (x, y) in one.find(|pixel| pixel == RED_PX) {
        let block = [(0, 0), (1, 0), (0, 1), (1, 1)];
        let brightest = block.iter().map(|&(dx, dy)| two.at(x * 2 + dx, y * 2 + dy)[0]).max().unwrap();
        assert!(brightest > 200, "logical ({x}, {y}) at scale 1 is lit but not at scale 2: {brightest}");
    }
}

#[test]
fn the_background_follows_zoom_pan_and_scale() {
    // Left half red, right half green.
    let mut image = Vec::new();
    for _ in 0..4 {
        image.extend_from_slice(&RED_PX);
        image.extend_from_slice(&RED_PX);
        image.extend_from_slice(&GREEN_PX);
        image.extend_from_slice(&GREEN_PX);
    }

    let mut r = renderer!(32, 32);
    r.set_background(&image, 4, 4);
    let drawn = draw(&mut r, 32, 32);
    assert_eq!(drawn.at(1, 1), RED_PX);
    assert_eq!(drawn.at(3, 1), GREEN_PX);
    assert_eq!(drawn.at(4, 1), BLACK_PX, "one image pixel is one scene unit");

    r.set_view(4.0, 0.0, 0.0);
    let drawn = draw(&mut r, 32, 32);
    assert_eq!(drawn.at(3, 3), RED_PX);
    assert_eq!(drawn.at(12, 3), GREEN_PX);
    assert_eq!(drawn.at(16, 3), BLACK_PX);

    r.set_view(4.0, 8.0, 4.0);
    let drawn = draw(&mut r, 32, 32);
    assert_eq!(drawn.at(7, 3), BLACK_PX, "panned away from the corner");
    assert_eq!(drawn.at(9, 5), RED_PX);
    assert_eq!(drawn.at(20, 5), GREEN_PX);

    r.resize(64, 64, 2.0);
    let drawn = draw(&mut r, 64, 64);
    assert_eq!(drawn.at(18, 10), RED_PX, "pan and zoom are in logical pixels");
    assert_eq!(drawn.at(40, 10), GREEN_PX);
    assert_eq!(drawn.at(14, 10), BLACK_PX);
}

#[test]
fn lights_line_up_with_the_image_pixel_they_name() {
    let mut r = renderer!(64, 64);
    // A single white image pixel at (20, 30).
    let mut image = solid_image(40, 40, BLACK_PX);
    image[((30 * 40 + 20) * 4) as usize..((30 * 40 + 20) * 4 + 4) as usize].copy_from_slice(&WHITE_PX);
    r.set_background(&image, 40, 40);
    let lit = draw(&mut r, 64, 64).find(|pixel| pixel == WHITE_PX);
    assert_eq!(lit, [(20, 30)]);

    r.set_lights(&[light(20.0, 30.0, 1.0)]);
    r.set_light_colors(&[RED]);
    let drawn = draw(&mut r, 64, 64);
    assert_eq!(drawn.at(20, 30), RED_PX, "right on top of it");
    assert_eq!(drawn.count(WHITE_PX), 0);
}

// ---------------------------------------------------------------------------------------------------------
// Updates
// ---------------------------------------------------------------------------------------------------------

#[test]
fn update_light_range_changes_only_those_lights() {
    let mut r = renderer!(100, 100);
    let lights = [light(10.0, 10.0, 5.0), light(30.0, 10.0, 5.0), light(50.0, 10.0, 5.0), light(70.0, 10.0, 5.0)];
    r.set_lights(&lights);
    r.set_light_colors(&[RED; 4]);
    let before = draw(&mut r, 100, 100);

    r.update_light_range(1, &[light(30.0, 60.0, 9.0), light(50.0, 80.0, 3.0)]);
    let after = draw(&mut r, 100, 100);

    // Untouched lights are identical, pixel for pixel.
    for (center_x, center_y) in [(10, 10), (70, 10)] {
        for y in center_y - 5..=center_y + 5 {
            for x in center_x - 5..=center_x + 5 {
                assert_eq!(before.at(x, y), after.at(x, y), "({x}, {y}) of an untouched light");
            }
        }
    }
    assert_eq!(after.at(30, 10), BLACK_PX, "light 1 left");
    assert_eq!(after.at(50, 10), BLACK_PX, "light 2 left");
    assert_eq!(after.at(30, 60), RED_PX, "light 1 arrived, with its new size");
    assert_eq!(after.at(33, 60), RED_PX);
    assert_eq!(after.at(50, 80), RED_PX, "light 2 arrived");
    assert_eq!(after.at(53, 80), BLACK_PX, "with its new size");
}

#[test]
fn update_light_range_clips_to_the_lights_that_exist() {
    let mut r = renderer!(32, 32);
    r.set_lights(&[light(5.0, 5.0, 1.0), light(10.0, 5.0, 1.0)]);
    r.set_light_colors(&[RED, RED]);

    r.update_light_range(1, &[light(20.0, 20.0, 1.0), light(25.0, 25.0, 1.0)]);
    r.update_light_range(2, &[light(1.0, 1.0, 1.0)]);
    r.update_light_range(100, &[light(1.0, 1.0, 1.0)]);
    r.update_light_range(0, &[]);
    assert_eq!(draw(&mut r, 32, 32).find(|pixel| pixel != BLACK_PX), [(5, 5), (20, 20)]);
}

#[test]
fn set_lights_keeps_colours_and_growing_past_the_buffer_loses_nothing() {
    let mut r = renderer!(64, 64);
    r.set_lights(&[light(5.0, 5.0, 1.0)]);
    r.set_light_colors(&[RED]);
    assert_eq!(draw(&mut r, 64, 64).at(5, 5), RED_PX);

    // Far past the initial 4 KiB colour buffer, which forces a new one.
    let many: Vec<_> = (0..50_000).map(|i| light(5.0 + (i % 50) as f32, 5.0, 1.0)).collect();
    r.set_lights(&many);
    let image = draw(&mut r, 64, 64);
    assert_eq!(image.at(5, 5), RED_PX, "light 0 kept its colour");
    assert_eq!(image.at(6, 5), BLACK_PX, "new lights start transparent");

    r.set_lights(&[light(7.0, 7.0, 1.0)]);
    let image = draw(&mut r, 64, 64);
    assert_eq!(image.at(7, 7), RED_PX, "moved, keeping its colour");
    assert_eq!(image.at(5, 5), BLACK_PX);
}

#[test]
fn colours_set_before_their_lights_still_apply() {
    let mut r = renderer!(32, 32);
    r.set_light_colors(&[RED, GREEN]);
    r.render().unwrap();
    r.set_lights(&[light(5.0, 5.0, 1.0), light(10.0, 5.0, 1.0)]);
    let image = draw(&mut r, 32, 32);
    assert_eq!(image.at(5, 5), RED_PX);
    assert_eq!(image.at(10, 5), GREEN_PX);
}

#[test]
fn a_mismatched_colour_array_blanks_the_rest_and_ignores_the_excess() {
    let mut r = renderer!(32, 32);
    r.set_lights(&[light(5.0, 5.0, 1.0), light(10.0, 5.0, 1.0)]);
    r.set_light_colors(&[RED, RED]);
    assert_eq!(draw(&mut r, 32, 32).count(RED_PX), 2);

    r.set_light_colors(&[GREEN]);
    let image = draw(&mut r, 32, 32);
    assert_eq!(image.at(5, 5), GREEN_PX);
    assert_eq!(image.at(10, 5), BLACK_PX);

    r.set_light_colors(&[RED, RED, RED, RED, RED]);
    assert_eq!(draw(&mut r, 32, 32).count(RED_PX), 2);
}

#[test]
fn a_frame_draws_in_at_most_three_calls_and_uploads_only_what_changed() {
    let mut r = renderer!(64, 64);
    let empty = r.render().unwrap();
    assert_eq!(empty.draw_calls, 0);
    assert!(empty.skipped.is_none());

    let count = 10_000;
    let lights: Vec<_> = (0..count).map(|i| light((i % 64) as f32, (i / 64 % 64) as f32, 3.0)).collect();
    r.set_lights(&lights);
    let mut colors = vec![RED; count];
    r.set_light_colors(&colors);
    let first = r.render().unwrap();
    assert_eq!(first.draw_calls, 1);
    assert_eq!(first.lights_drawn, count as u32);
    assert_eq!(first.lights_uploaded, count as u32);

    // Nothing changed, nothing goes up, whether or not the array was passed in again.
    assert_eq!(r.render().unwrap().lights_uploaded, 0);
    r.set_light_colors(&colors);
    assert_eq!(r.render().unwrap().lights_uploaded, 0);

    // One light changed: one block of 64.
    colors[5000] = GREEN;
    r.set_light_colors(&colors);
    assert_eq!(r.render().unwrap().lights_uploaded, 64);

    // Everything changed.
    colors.fill(GREEN);
    r.set_light_colors(&colors);
    assert_eq!(r.render().unwrap().lights_uploaded, count as u32);

    // Moving one shape doesn't re-send colours.
    r.update_light_range(100, &lights[..10]);
    assert_eq!(r.render().unwrap().lights_uploaded, 0);

    r.set_background(&solid_image(8, 8, BLUE_PX), 8, 8);
    r.set_overlay(lines(vec![line([0.0, 0.0], [5.0, 5.0], WHITE, 1.0)]));
    assert_eq!(r.render().unwrap().draw_calls, 3);

    r.set_background_alpha(0);
    assert_eq!(r.render().unwrap().draw_calls, 2, "a fully dimmed image isn't drawn at all");
}

// ---------------------------------------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------------------------------------

#[test]
fn a_scene_with_no_lights_renders() {
    let mut r = renderer!(16, 16);
    assert_eq!(draw(&mut r, 16, 16).count(BLACK_PX), 16 * 16);
    r.set_lights(&[]);
    r.set_light_colors(&[]);
    r.set_overlay(Overlay::default());
    assert_eq!(draw(&mut r, 16, 16).count(BLACK_PX), 16 * 16);

    r.set_lights(&[light(5.0, 5.0, 3.0)]);
    r.set_light_colors(&[RED]);
    assert_eq!(draw(&mut r, 16, 16).at(5, 5), RED_PX);
    r.set_lights(&[]);
    assert_eq!(draw(&mut r, 16, 16).count(BLACK_PX), 16 * 16);
}

#[test]
fn zero_size_requests_do_not_crash_and_recover() {
    for (width, height) in [(0, 0), (0, 16), (16, 0)] {
        let mut r = renderer!(width, height);
        assert_eq!(r.render().unwrap().skipped, Some(FrameSkip::ZeroSize));
        assert!(r.read_pixels().is_empty());

        // Everything still works with nothing to draw on.
        r.set_lights(&[light(5.0, 5.0, 3.0)]);
        r.set_light_colors(&[RED]);
        r.set_background(&solid_image(4, 4, BLUE_PX), 4, 4);
        r.set_overlay(lines(vec![line([0.0, 6.0], [3.0, 6.0], WHITE, 1.0)]));
        r.set_view(2.0, 1.0, 1.0);
        assert_eq!(r.render().unwrap().skipped, Some(FrameSkip::ZeroSize));

        // And nothing was lost once there is.
        r.resize(16, 16, 1.0);
        let image = draw(&mut r, 16, 16);
        assert_eq!(image.at(11, 11), RED_PX, "the light, moved by the view");
        assert_eq!(image.at(14, 2), BLACK_PX, "past the zoomed image");
        assert_eq!(image.at(2, 2), BLUE_PX, "the image");
        assert_eq!(image.at(4, 13), WHITE_PX, "the overlay");

        r.resize(0, 0, 1.0);
        assert_eq!(r.render().unwrap().skipped, Some(FrameSkip::ZeroSize));
        assert!(r.read_pixels().is_empty());
    }
}

#[test]
fn empty_and_invalid_inputs_do_not_crash() {
    let mut r = renderer!(16, 16);
    r.set_background(&[], 0, 0);
    r.set_background(&[], 0, 100);
    r.set_background(&[], 100, 0);
    assert_eq!(draw(&mut r, 16, 16).count(BLACK_PX), 16 * 16);

    // A wrong length is an error from render, and otherwise just no background.
    r.set_background(&solid_image(16, 16, BLUE_PX), 16, 16);
    r.set_background(&[0; 10], 16, 16);
    assert!(matches!(r.render(), Err(RenderError::InvalidInput(_))));
    assert_eq!(draw(&mut r, 16, 16).count(BLACK_PX), 16 * 16);

    // Sizes that overflow if multiplied carelessly.
    r.set_background(&[0; 4], u32::MAX, u32::MAX);
    assert!(matches!(r.render(), Err(RenderError::InvalidInput(_))));

    for scale in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        r.resize(16, 16, scale);
        r.render().unwrap();
    }
    for (zoom, x, y) in [(f32::NAN, 0.0, 0.0), (1.0, f32::INFINITY, 0.0), (1.0, 0.0, f32::NEG_INFINITY)] {
        r.set_view(zoom, x, y);
    }
    r.set_lights(&[light(5.0, 5.0, 1.0)]);
    r.set_light_colors(&[RED]);
    assert_eq!(draw(&mut r, 16, 16).at(5, 5), RED_PX, "non-finite views were ignored");

    r.set_view(0.0, 0.0, 0.0);
    r.render().unwrap();
    r.set_view(-1.0, 0.0, 0.0);
    r.render().unwrap();
    r.set_lights(&[light(f32::NAN, f32::INFINITY, 3.0), light(1e30, -1e30, 1e30)]);
    r.set_light_colors(&[RED, RED]);
    r.set_overlay(Overlay {
        lines: vec![line([f32::NAN, 0.0], [0.0, f32::NAN], WHITE, f32::NAN)],
        squares: vec![OverlaySquare { center: [f32::INFINITY, 0.0], size: -5.0, color: WHITE }],
        circles: vec![([0.0, 0.0], f32::NAN, WHITE), ([0.0, 0.0], -1.0, WHITE)],
    });
    r.render().unwrap();
}

#[test]
fn extreme_sizes_do_not_crash() {
    let mut r = renderer!(1, 1);
    r.set_lights(&[light(0.0, 0.0, 1.0)]);
    r.set_light_colors(&[RED]);
    assert_eq!(draw(&mut r, 1, 1).at(0, 0), RED_PX);

    let mut r = renderer!(32, 32);
    r.resize(u32::MAX, u32::MAX, 1.0);
    r.render().unwrap();

    let mut r = renderer!(32, 32);
    r.set_lights(&[light(16.0, 16.0, 100_000.0)]);
    r.set_light_colors(&[RED]);
    assert_eq!(draw(&mut r, 32, 32).count(RED_PX), 32 * 32, "a huge light covers the frame");
}

#[test]
fn resize_keeps_the_scene_and_changes_the_output_size() {
    let mut r = renderer!(32, 32);
    r.set_lights(&[light(5.0, 5.0, 1.0)]);
    r.set_light_colors(&[RED]);
    assert_eq!(draw(&mut r, 32, 32).at(5, 5), RED_PX);
    r.resize(64, 16, 1.0);
    assert_eq!(draw(&mut r, 64, 16).at(5, 5), RED_PX);
    r.resize(7, 9, 1.0);
    assert_eq!(draw(&mut r, 7, 9).at(5, 5), RED_PX);
}

#[test]
fn half_a_million_lights_all_draw() {
    let (width, height) = (1024u32, 512u32);
    let mut r = renderer!(width, height);
    let lights: Vec<_> = (0..500_000u32)
        .map(|i| light((i % width) as f32, (i / width % height) as f32, 1.0))
        .collect();
    r.set_lights(&lights);
    r.set_light_colors(&vec![RED; lights.len()]);
    let image = draw(&mut r, width, height);
    assert_eq!(image.at(0, 0), RED_PX);
    assert_eq!(image.at(287, 488), RED_PX, "the last light");
    assert_eq!(image.at(288, 488), BLACK_PX, "one past it");
    assert_eq!(image.count(RED_PX), 500_000, "every light, none dropped");
}

#[test]
fn moving_heads_use_perspective_depth_two_colors_and_background_dimming() {
    use vixen_render::{MovingHeadInstance,MovingHeadParams};
    let mut r=renderer!(300,300);
    let mut head=MovingHeadInstance{position:[150.0,130.0],length:60.0,
        params:MovingHeadParams{tilt_deg:35.0,pan_deg:25.0,max_beam_length:220.0,beam_length_pct:55.0,
            color_left:rgba(255,0,0,255),color_right:rgba(0,0,255,255),beam_alpha:0.6,..Default::default()}};
    r.set_moving_heads(vec![head.clone()]);
    assert!(draw(&mut r,300,300).find(|p|p!=BLACK_PX).is_empty(),"flat views omit fixtures");
    r.set_camera_depth(362.132);
    let lit=draw(&mut r,300,300);
    let red=|p:[u8;4]|p[0]>20&&p[0] as u16>p[1] as u16*2&&p[2]<10;
    let blue=|p:[u8;4]|p[2]>20&&p[2] as u16>p[1] as u16*2&&p[0]<10;
    assert!(lit.find(red).len()>20,"left beam color is visible");
    assert!(lit.find(blue).len()>20,"right beam color is visible");
    assert!(lit.find(|p|p[0]>25&&p[0]==p[1]&&p[1]==p[2]).len()>50,"fixture housing is shaded gray");
    if let Some(path)=std::env::var_os("VIXEN_FIXTURE_SNAPSHOT") {
        let mut ppm=b"P6\n300 300\n255\n".to_vec();
        ppm.extend(lit.pixels.chunks_exact(4).flat_map(|p|p[..3].iter().copied()));
        std::fs::write(path,ppm).unwrap();
    }
    r.set_background_alpha(0);
    let dim=draw(&mut r,300,300);
    assert_eq!(dim.find(red),lit.find(red));
    assert!(dim.find(|p|p[0]>25&&p[0]==p[1]&&p[1]==p[2]).is_empty());
    head.params.on=false;
    r.set_moving_heads(vec![head]);
    let off=draw(&mut r,300,300);
    assert!(off.find(red).is_empty()&&off.find(blue).is_empty());
}

#[test]
fn moving_head_thumbnails_draw_in_a_flat_canvas_and_clip_to_their_square() {
    use vixen_render::{MovingHeadInstance,MovingHeadParams};
    let mut r=renderer!(200,200);
    r.set_moving_heads(vec![MovingHeadInstance{position:[100.0,100.0],length:100.0,
        params:MovingHeadParams{thumbnail:true,beam_length_pct:100.0,max_beam_length:300.0,
            color_left:RED,color_right:RED,..Default::default()}}]);
    let image=draw(&mut r,200,200);
    let visible=image.find(|p|p!=BLACK_PX);
    assert!(visible.len()>100);
    assert!(visible.iter().all(|&(x,y)|(50..151).contains(&x)&&(50..151).contains(&y)),"local viewport clips a long beam");
    assert!(image.find(|p|p[0]>20&&p[1]<5).len()>10,"colored beam is present");
    r.set_view(0.5,20.0,30.0);
    let moved=draw(&mut r,200,200).find(|p|p!=BLACK_PX);
    assert!(moved.iter().all(|&(x,y)|(45..96).contains(&x)&&(55..106).contains(&y)),"thumbnail follows canvas zoom and pan");
}
