//! The inline gradient row's operations, F44 to F50: `BaseInlineGradientEditor` as pure functions over the value.

use vixen_core::gradient::ColorPoint;
use vixen_core::{Color, ColorGradient};

/// Stops at one exact position share a handle (F44).
#[derive(Clone, Debug, PartialEq)]
pub struct GradientHandle {
    pub position: f64,
    /// Indexes into the gradient's stops, in sorted order.
    pub stops: Vec<u32>,
    pub color: Color,
}

/// `ReloadPoints`.
pub fn gradient_handles(g: &ColorGradient) -> Vec<GradientHandle> {
    let colors = g.colors();
    let mut order: Vec<usize> = (0..colors.len()).collect();
    // `Colors.SortedArray()`.
    order.sort_by(|a, b| colors[*a].position.total_cmp(&colors[*b].position));
    let mut handles: Vec<GradientHandle> = Vec::new();
    for i in order {
        let p = &colors[i];
        match handles.last_mut() {
            Some(last) if last.position == p.position => last.stops.push(i as u32),
            _ => handles.push(GradientHandle { position: p.position, stops: vec![i as u32], color: p.rgb_color() }),
        }
    }
    handles
}

/// A blend to draw across the strip's width.
#[derive(Clone, Debug, PartialEq)]
pub struct StripBand {
    pub colors: Vec<Color>,
    pub positions: Vec<f32>,
}

/// F91: one band normally. Discrete: one band per distinct colour, in first-seen stop order, each filtered to that colour, so overlapping stops
/// show as stacked ramps. A discrete gradient with no stops draws nothing.
pub fn gradient_strip_bands(g: &ColorGradient, discrete: bool) -> Vec<StripBand> {
    if !discrete {
        let blend = g.color_blend();
        return vec![StripBand { colors: blend.colors.clone(), positions: blend.positions.clone() }];
    }
    let mut seen: Vec<Color> = Vec::new();
    for p in g.colors() {
        let c = p.rgb_color();
        if !seen.contains(&c) {
            seen.push(c);
        }
    }
    seen.into_iter()
        .map(|c| {
            let blend = g.filtered_blend(c);
            StripBand { colors: blend.colors.clone(), positions: blend.positions.clone() }
        })
        .collect()
}

/// `AddColorPoint`: the first valid colour when discrete, otherwise the gradient's colour there. Discrete with no valid colours adds nothing.
pub fn gradient_add_stop(g: &ColorGradient, position: f64, discrete: bool, valid: &[Color]) -> ColorGradient {
    let mut out = g.clone();
    let mut colors = out.colors().to_vec();
    if discrete {
        if let Some(first) = valid.first() {
            colors.push(ColorPoint::new(*first, position));
        }
    } else {
        colors.push(ColorPoint::new(g.get_color_at_f32(position as f32), position));
    }
    out.set_colors(colors);
    out
}

/// F46: every stop under the handle moves. The setter throws outside 0..1 upstream; clamp instead (F94).
pub fn gradient_move_handle(g: &ColorGradient, stops: &[u32], position: f64) -> ColorGradient {
    let position = position.clamp(0.0, 1.0);
    let mut out = g.clone();
    let mut colors = out.colors().to_vec();
    for s in stops {
        if let Some(p) = colors.get_mut(*s as usize) {
            p.position = position;
        }
    }
    out.set_colors(colors);
    out
}

/// `DeletePoint`. Nothing stops this removing the last stop (KEEP, F47); a gradient with none blends to transparent.
pub fn gradient_delete_handle(g: &ColorGradient, stops: &[u32]) -> ColorGradient {
    let mut out = g.clone();
    let colors = out.colors().iter().enumerate().filter(|(i, _)| !stops.contains(&(*i as u32))).map(|(_, p)| *p).collect();
    out.set_colors(colors);
    out
}

/// F48, discrete: the handle's stops are replaced by one stop per ticked colour at the same position, in list order.
pub fn gradient_set_handle_colors(g: &ColorGradient, stops: &[u32], colors: &[Color]) -> ColorGradient {
    if colors.is_empty() {
        return gradient_delete_handle(g, stops);
    }
    let Some(position) = stops.first().and_then(|s| g.colors().get(*s as usize)).map(|p| p.position) else {
        return g.clone();
    };
    let mut out = gradient_delete_handle(g, stops);
    let mut points = out.colors().to_vec();
    points.extend(colors.iter().map(|c| ColorPoint::new(*c, position)));
    out.set_colors(points);
    out
}

/// F48, not discrete: the first stop takes the colour, keeping its place and focus.
pub fn gradient_set_stop_color(g: &ColorGradient, stop: u32, color: Color) -> ColorGradient {
    let mut out = g.clone();
    let mut points = out.colors().to_vec();
    if let Some(p) = points.get_mut(stop as usize) {
        *p = ColorPoint { color: vixen_core::color::Xyz::from_color(color), ..*p };
    }
    out.set_colors(points);
    out
}

/// F49: `None` when it wouldn't reverse: a linked gradient, or one of a single stop.
pub fn gradient_reverse(g: &ColorGradient) -> Option<ColorGradient> {
    (!g.is_library_reference() && g.colors().len() > 1).then(|| g.reversed())
}

/// `ColorGradient.GetFocusPosition`: where the selected fader's diamond sits, `None` for the first stop or one that isn't found.
pub fn focus_position(g: &ColorGradient, stop: usize) -> Option<f64> {
    let (sorted, i) = sorted_with(g, stop)?;
    if i < 1 {
        return None;
    }
    let (previous, this) = (&sorted[i - 1], &sorted[i]);
    Some(previous.position + this.focus * (this.position - previous.position))
}

/// `ColorGradient.SetFocusPosition`: clamped to the gap, and .5 in a gap of no width.
pub fn set_focus_position(g: &mut ColorGradient, stop: usize, position: f64) {
    let Some((sorted, i)) = sorted_with(g, stop) else { return };
    if i == 0 {
        return;
    }
    let width = sorted[i].position - sorted[i - 1].position;
    let focus = if width <= 0.0 { 0.5 } else { ((position - sorted[i - 1].position) / width).clamp(0.0, 1.0) };
    let mut points = g.colors().to_vec();
    points[stop].focus = focus;
    g.set_colors(points);
}

fn sorted_with(g: &ColorGradient, stop: usize) -> Option<(Vec<ColorPoint>, usize)> {
    let colors = g.colors();
    colors.get(stop)?;
    let mut order: Vec<usize> = (0..colors.len()).collect();
    order.sort_by(|a, b| colors[*a].position.total_cmp(&colors[*b].position));
    let i = order.iter().position(|o| *o == stop)?;
    Some((order.iter().map(|o| colors[*o]).collect(), i))
}

/// F94's equality: linked gradients compare by library name, a linked and an unlinked are unequal, otherwise stops and alphas in list order.
pub fn gradient_same_value(a: &ColorGradient, b: &ColorGradient) -> bool {
    if a.is_library_reference() && b.is_library_reference() && a.library_reference_name == b.library_reference_name {
        return true;
    }
    if a.is_library_reference() || b.is_library_reference() {
        return false;
    }
    a.colors() == b.colors() && a.alphas() == b.alphas()
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::gradient::AlphaPoint;

    fn black_white() -> ColorGradient {
        ColorGradient::from_parts(
            vec![ColorPoint::new(Color::BLACK, 0.0), ColorPoint::new(Color::WHITE, 1.0)],
            vec![AlphaPoint::new(255, 0.0), AlphaPoint::new(255, 1.0)],
        )
    }

    #[test]
    fn default_gradient_and_shared_handles() {
        let g = ColorGradient::default();
        assert_eq!(g.colors().len(), 1);
        assert_eq!(g.colors()[0].position, 0.0);
        assert_eq!(g.colors()[0].focus, 0.5);
        let s = ColorGradient::from_parts(
            vec![ColorPoint::new(Color::RED, 0.5), ColorPoint::new(Color::BLUE, 0.0), ColorPoint::new(Color::LIME, 0.5)],
            Vec::new(),
        );
        let h = gradient_handles(&s);
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].stops, vec![1]);
        assert_eq!(h[1].stops, vec![0, 2]);
        assert_eq!(h[1].color, Color::RED);
    }

    #[test]
    fn add_a_stop() {
        let g = gradient_add_stop(&black_white(), 0.5, false, &[]);
        let added = g.colors()[2].rgb_color();
        assert!(added.r > 100 && added.r < 150 && added.r == added.g && added.g == added.b, "{added:?}");
        let g = gradient_add_stop(&black_white(), 0.5, true, &[Color::RED, Color::GREEN]);
        assert_eq!(g.colors()[2].rgb_color(), Color::RED);
        let g = gradient_add_stop(&black_white(), 0.5, true, &[]);
        assert_eq!(g.colors().len(), 2);
    }

    #[test]
    fn delete_a_shared_handle_and_the_last_stop() {
        let s = ColorGradient::from_parts(
            vec![ColorPoint::new(Color::RED, 0.5), ColorPoint::new(Color::LIME, 0.5)],
            vec![AlphaPoint::new(255, 0.0), AlphaPoint::new(255, 1.0)],
        );
        let h = gradient_handles(&s);
        let empty = gradient_delete_handle(&s, &h[0].stops);
        assert!(empty.colors().is_empty());
        assert_eq!(empty.get_color_at(0.0).a, 0);
        assert_eq!(empty.get_color_at(1.0).a, 0);
    }

    #[test]
    fn discrete_picker_results() {
        let g = black_white();
        let none = gradient_set_handle_colors(&g, &[1], &[]);
        assert_eq!(none.colors().len(), 1);
        let two = gradient_set_handle_colors(&g, &[1], &[Color::RED, Color::BLUE]);
        assert_eq!(two.colors().len(), 3);
        assert_eq!(two.colors()[1].position, 1.0);
        assert_eq!(two.colors()[2].position, 1.0);
        assert_eq!(two.colors()[2].rgb_color(), Color::BLUE);
    }

    #[test]
    fn reversal_needs_more_than_one_stop_and_no_link() {
        assert!(gradient_reverse(&ColorGradient::solid(Color::RED)).is_none());
        let mut g = black_white();
        assert!(gradient_reverse(&g).is_some());
        g.library_reference_name = "L".into();
        assert!(gradient_reverse(&g).is_none());
    }

    #[test]
    fn equality_for_no_op_checks() {
        let mut a = black_white();
        let mut b = ColorGradient::solid(Color::RED);
        assert!(!gradient_same_value(&a, &b));
        a.library_reference_name = "L".into();
        b.library_reference_name = "L".into();
        assert!(gradient_same_value(&a, &b));
        b.library_reference_name = String::new();
        assert!(!gradient_same_value(&a, &b));
    }

    #[test]
    fn focus_positions() {
        let mut g = black_white();
        assert_eq!(focus_position(&g, 0), None);
        assert_eq!(focus_position(&g, 1), Some(0.5));
        set_focus_position(&mut g, 1, 0.25);
        assert_eq!(g.colors()[1].focus, 0.25);
        assert_eq!(focus_position(&g, 1), Some(0.25));
        let mut same = ColorGradient::from_parts(
            vec![ColorPoint::new(Color::RED, 0.5), ColorPoint::new(Color::LIME, 0.5)],
            Vec::new(),
        );
        same.set_colors(vec![ColorPoint { focus: 0.1, ..same.colors()[0] }, ColorPoint { focus: 0.1, ..same.colors()[1] }]);
        set_focus_position(&mut same, 1, 0.9);
        assert_eq!(same.colors()[1].focus, 0.5);
    }

    #[test]
    fn discrete_bands_are_one_per_distinct_colour_in_first_seen_order() {
        let s = ColorGradient::from_parts(
            vec![ColorPoint::new(Color::RED, 0.0), ColorPoint::new(Color::LIME, 0.5), ColorPoint::new(Color::RED, 1.0)],
            vec![AlphaPoint::new(255, 0.0), AlphaPoint::new(255, 1.0)],
        );
        let bands = gradient_strip_bands(&s, true);
        assert_eq!(bands.len(), 2);
        assert_eq!(gradient_strip_bands(&s, false).len(), 1);
        assert!(gradient_strip_bands(&ColorGradient::from_parts(Vec::new(), Vec::new()), true).is_empty());
    }
}
