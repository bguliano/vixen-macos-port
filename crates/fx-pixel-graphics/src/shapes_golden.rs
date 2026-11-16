//! Geometry-only differential against unchanged upstream GeometricShapes.cs.
use super::*;
use flate2::read::GzDecoder;
use serde_json::Value;
use std::fs::File;

#[test]
fn original_shape_geometry_vertices_and_dimensions() {
    let path = std::env::var_os("SHAPES_GOLDEN")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/golden/shapes-geometry.json.gz")
        });
    let fixture: Value =
        serde_json::from_reader(GzDecoder::new(File::open(path).unwrap())).unwrap();
    let mut mismatches = 0;
    let mut coordinates = 0;
    let cases = fixture["cases"].as_array().unwrap();
    for case in cases {
        let radius = case["radius"].as_i64().unwrap() as i32;
        let count = case["count"].as_i64().unwrap() as i32;
        let skip = case["skip"].as_i64().unwrap() as i32;
        let inside = case["inside"].as_i64().unwrap() as i32;
        let ratio = case["ratio"].as_f64().unwrap() as f32;
        let north = case["north"].as_bool().unwrap();
        let shape = match case["type"].as_str().unwrap() {
            "circle" => svg_circle(radius),
            "ellipse" => svg_ellipse(radius, ratio),
            "rect" => svg_rectangle(radius, ratio),
            "triangle" => polygon(&calculate_vertices(radius, 3)),
            "polygon" => polygon(&calculate_vertices(radius, count)),
            "star" => polygon(&make_star_points(radius, count, skip)),
            "nonintersecting" => {
                polygon(&non_intersecting_star_points(radius, count, inside, north))
            }
            "cross" => svg_cross(ratio),
            "arrow" => svg_arrow(),
            _ => panic!("unknown case"),
        };
        assert_eq!(shape.tag, case["shape"]["tag"].as_str().unwrap());
        let attrs: &[&str] = match shape.tag.as_str() {
            "circle" => &["cx", "cy", "r"],
            "ellipse" => &["cx", "cy", "rx", "ry"],
            "rect" => &["x", "y", "width", "height"],
            "polygon" => &["points"],
            _ => unreachable!(),
        };
        let actual: Vec<f64> = attrs
            .iter()
            .flat_map(|a| {
                shape
                    .attr(a)
                    .unwrap()
                    .split(|c: char| c == ',' || c.is_whitespace())
                    .filter(|v| !v.is_empty())
                    .map(|v| v.parse::<f64>().unwrap())
            })
            .collect();
        let expected = case["shape"]["values"].as_array().unwrap();
        assert_eq!(actual.len(), expected.len());
        for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            coordinates += 1;
            // SVG decimal spelling may differ, but it encodes the same f32 dimension.
            let equal = if shape.tag == "polygon" {
                *actual == expected.as_f64().unwrap()
            } else {
                *actual as f32 == expected.as_f64().unwrap() as f32
            };
            if !equal {
                mismatches += 1;
                if mismatches <= 20 {
                    eprintln!("{} r={radius} n={count} skip={skip} inside={inside} north={north} coord={index}: C#={expected} Rust={actual}", case["type"]);
                }
            }
        }
    }
    println!(
        "{} cases / {coordinates} coordinates / {mismatches} mismatches",
        cases.len()
    );
    assert_eq!(mismatches, 0);
}
