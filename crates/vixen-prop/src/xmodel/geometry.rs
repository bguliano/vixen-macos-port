use super::*;

pub(super) fn parse(
    e: &XElement,
    kind: Kind,
    warnings: &mut Vec<String>,
) -> Result<Geometry, PropError> {
    match kind {
        Kind::Custom => custom(e, warnings),
        Kind::Circle => circle(e, warnings),
    }
}
fn scale(e: &XElement, width: i32, height: i32, warnings: &mut Vec<String>) -> (f64, f64) {
    let fallback = if width < 100 && height < 100 {
        4.0
    } else if width < 200 && height < 200 {
        2.0
    } else {
        1.0
    };
    let mut read = |key| {
        let raw = attr(e, key);
        raw.trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && *v > 0.0)
            .unwrap_or_else(|| {
                if !raw.trim().is_empty() {
                    warnings.push(format!("Invalid {key}; using {fallback}."));
                }
                fallback
            })
    };
    (read("ScaleX"), read("ScaleY"))
}
fn coordinate(value: f64) -> Result<i32, PropError> {
    // Upstream explicitly requests AwayFromZero here, unlike ordinary Math.Round.
    let value = value.round();
    if !value.is_finite() || value < i32::MIN as f64 || value > i32::MAX as f64 - 1.0 {
        return Err(failed("Model coordinates exceed the supported range."));
    }
    Ok(value as i32)
}
fn custom(e: &XElement, warnings: &mut Vec<String>) -> Result<Geometry, PropError> {
    let width = positive(attr(e, "CustomWidth"))
        .or_else(|| positive(attr(e, "parm1")))
        .unwrap_or(0);
    let height = positive(attr(e, "CustomHeight"))
        .or_else(|| positive(attr(e, "parm2")))
        .unwrap_or(0);
    let scale = scale(e, width, height, warnings);
    let compressed = attr(e, "CustomModelCompressed");
    let plain = attr(e, "CustomModel");
    let parsed = if !compressed.trim().is_empty() {
        match parse_custom(compressed, scale, true) {
            Ok(nodes) => Ok(nodes),
            Err(_) => {
                warnings.push(
                    "Unable to parse CustomModelCompressed. Falling back to CustomModel.".into(),
                );
                parse_custom(plain, scale, false)
            }
        }
    } else {
        parse_custom(plain, scale, false)
    };
    let nodes = parsed.map_err(|error| {
        PropError::new(
            &format!(
                "Unable to parse a valid CustomModel or CustomModelCompressed for model '{}'.",
                attr(e, "name")
            ),
            error.detail,
        )
    })?;
    Ok(Geometry {
        nodes,
        width,
        height,
        rings: vec![],
    })
}
fn parse_custom(
    text: &str,
    scale: (f64, f64),
    compressed: bool,
) -> Result<BTreeMap<i32, (i32, i32)>, PropError> {
    if text.trim().is_empty() {
        return Err(failed("The model definition is empty."));
    }
    let mut nodes = BTreeMap::new();
    let bad = || failed("The model definition contains an invalid number or entry.");
    if compressed {
        for entry in text.split(';').filter(|v| !v.is_empty()) {
            let parts: Vec<_> = entry.split(',').collect();
            if parts.len() != 3 {
                return Err(bad());
            }
            let order = integer(parts[0]).ok_or_else(bad)?;
            let row = integer(parts[1]).ok_or_else(bad)?;
            let col = integer(parts[2]).ok_or_else(bad)?;
            nodes.insert(
                order,
                (
                    coordinate(col as f64 * scale.0)? + 1,
                    coordinate(row as f64 * scale.1)? + 1,
                ),
            );
            if nodes.len() > MAX_NODES {
                return Err(failed("The model contains too many nodes."));
            }
        }
    } else {
        for (y, row) in text.split(';').enumerate() {
            for (x, cell) in row.split(',').enumerate() {
                if cell.trim().is_empty() {
                    continue;
                }
                let order = integer(cell).ok_or_else(bad)?;
                nodes.insert(
                    order,
                    (
                        coordinate(x as f64 * scale.0)? + 1,
                        coordinate(y as f64 * scale.1)? + 1,
                    ),
                );
                if nodes.len() > MAX_NODES {
                    return Err(failed("The model contains too many nodes."));
                }
            }
        }
    }
    Ok(nodes)
}
fn circle(e: &XElement, warnings: &mut Vec<String>) -> Result<Geometry, PropError> {
    let invalid = |key| {
        failed(format!(
            "Circle model '{}' has an invalid {key} value.",
            attr(e, "name")
        ))
    };
    let layers: Vec<_> = attr(e, "LayerSizes")
        .split(',')
        .map(|v| positive(v).ok_or_else(|| invalid("LayerSizes")))
        .collect::<Result<_, _>>()?;
    let inside = match attr(e, "InsideOut") {
        "0" => false,
        "1" => true,
        _ => return Err(invalid("InsideOut")),
    };
    let start = attr(e, "StartSide")
        .trim()
        .chars()
        .next()
        .unwrap_or('\0')
        .to_ascii_uppercase();
    let dir = attr(e, "Dir")
        .trim()
        .chars()
        .next()
        .unwrap_or('\0')
        .to_ascii_uppercase();
    if !matches!(start, 'T' | 'B') {
        return Err(invalid("StartSide"));
    }
    if !matches!(dir, 'L' | 'R') {
        return Err(invalid("Dir"));
    }
    let total: u64 = layers.iter().map(|n| *n as u64).sum();
    if total > MAX_NODES as u64 {
        return Err(failed("The model contains too many nodes."));
    }
    if let Some(count) = positive(attr(e, "PixelCount")) {
        if count as u64 != total {
            return Err(failed(format!(
                "Circle model '{}' PixelCount does not match the sum of LayerSizes.",
                attr(e, "name")
            )));
        }
    }
    let mut center = attr(e, "centerPercent");
    if center.trim().is_empty() {
        center = attr(e, "parm3");
    }
    let center = integer(center)
        .unwrap_or_else(|| {
            if !center.trim().is_empty() {
                warnings.push("Invalid centerPercent; using 0.".into());
            }
            0
        })
        .clamp(0, 100);
    let max = *layers.iter().max().unwrap();
    let scale = scale(e, max, max, warnings);
    let max_radius = max as f64 / 2.0;
    let min_radius = center as f64 / 100.0 * max_radius;
    let mut nodes = BTreeMap::new();
    let mut rings = Vec::new();
    let mut order = 1;
    for layer in 0..layers.len() {
        let physical = if inside {
            layer
        } else {
            layers.len() - 1 - layer
        };
        let count = layers[physical];
        let radius = if layers.len() == 1 {
            max_radius
        } else {
            min_radius + (max_radius - min_radius) * physical as f64 / (layers.len() - 1) as f64
        };
        let mut ring = Vec::with_capacity(count as usize);
        for index in 0..count {
            let angle = if start == 'T' {
                std::f64::consts::PI
            } else {
                0.0
            } + if dir == 'L' { -1.0 } else { 1.0 } * std::f64::consts::TAU
                / count as f64
                * index as f64;
            nodes.insert(
                order,
                (
                    coordinate(angle.sin() * radius * scale.0)?,
                    coordinate(angle.cos() * radius * scale.1)?,
                ),
            );
            ring.push(order);
            order += 1;
        }
        rings.push(ring);
    }
    let min_x = nodes.values().map(|p| p.0).min().unwrap_or(0);
    let min_y = nodes.values().map(|p| p.1).min().unwrap_or(0);
    for p in nodes.values_mut() {
        p.0 =
            p.0.checked_sub(min_x)
                .ok_or_else(|| failed("Model coordinates exceed the supported range."))?;
        p.1 =
            p.1.checked_sub(min_y)
                .ok_or_else(|| failed("Model coordinates exceed the supported range."))?;
    }
    Ok(Geometry {
        nodes,
        width: max,
        height: max,
        rings,
    })
}
