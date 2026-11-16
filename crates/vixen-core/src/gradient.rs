//! Port of VixenModules.App.ColorGradients.ColorGradient: colour points, focus handling and the
//! GDI+ `ColorBlend` it derives. Effects sample this constantly, so the blend is cached.

use crate::color::{clip_value, Color, Rgb, Xyz};
use parking_lot::RwLock;
use std::sync::{Arc, OnceLock};

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct ColorPoint {
    pub color: Xyz,
    pub position: f64,
    pub focus: f64,
}

impl ColorPoint {
    /// `new ColorPoint(Color, double)`: focus defaults to .5.
    pub fn new(color: Color, position: f64) -> ColorPoint {
        ColorPoint { color: Xyz::from_rgb(Rgb::from_color(color)), position, focus: 0.5 }
    }

    pub fn from_xyz(color: Xyz, focus: f64, position: f64) -> ColorPoint {
        ColorPoint { color, position, focus }
    }

    /// `point.Color.ToRGB().ToArgb()`.
    #[inline]
    pub fn rgb_color(&self) -> Color {
        self.color.to_color()
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct AlphaPoint {
    pub alpha: f64,
    pub position: f64,
    pub focus: f64,
}

impl AlphaPoint {
    /// `new AlphaPoint(byte, double)`.
    pub fn new(alpha: u8, position: f64) -> AlphaPoint {
        AlphaPoint { alpha: alpha as f64 / 255.0, position, focus: 0.5 }
    }
}

/// `System.Drawing.Drawing2D.ColorBlend`.
#[derive(Clone, Debug, Default)]
pub struct ColorBlend {
    pub colors: Vec<Color>,
    pub positions: Vec<f32>,
}

#[derive(Debug, Default)]
struct Caches {
    blend: OnceLock<Arc<ColorBlend>>,
    filtered: RwLock<Vec<(Color, Arc<ColorBlend>)>>,
}

impl Clone for Caches {
    fn clone(&self) -> Self {
        Caches::default()
    }
}

#[derive(Clone, Debug)]
pub struct ColorGradient {
    colors: Vec<ColorPoint>,
    alphas: Vec<AlphaPoint>,
    pub gammacorrected: bool,
    pub title: Option<String>,
    pub library_reference_name: String,
    pub is_current_library_gradient: bool,
    caches: Caches,
}

impl PartialEq for ColorGradient {
    fn eq(&self, o: &Self) -> bool {
        self.colors == o.colors
            && self.alphas == o.alphas
            && self.gammacorrected == o.gammacorrected
            && self.title == o.title
            && self.library_reference_name == o.library_reference_name
            && self.is_current_library_gradient == o.is_current_library_gradient
    }
}

impl Default for ColorGradient {
    /// `new ColorGradient()`: a single white point with full alpha.
    fn default() -> Self {
        ColorGradient::with_points(vec![ColorPoint::new(Color::WHITE, 0.0)])
    }
}

impl ColorGradient {
    fn with_points(colors: Vec<ColorPoint>) -> ColorGradient {
        ColorGradient {
            colors,
            alphas: vec![AlphaPoint::new(255, 0.0), AlphaPoint::new(255, 1.0)],
            gammacorrected: false,
            title: None,
            library_reference_name: String::new(),
            is_current_library_gradient: false,
            caches: Caches::default(),
        }
    }

    /// `new ColorGradient(Color staticColor)`.
    pub fn solid(color: Color) -> ColorGradient {
        ColorGradient::with_points(vec![ColorPoint::new(color, 0.0)])
    }

    /// `new ColorGradient(IEnumerable<Color>)`: every colour at position 0.
    pub fn from_colors(colors: &[Color]) -> ColorGradient {
        ColorGradient::with_points(colors.iter().map(|&c| ColorPoint::new(c, 0.0)).collect())
    }

    /// `new ColorGradient(IEnumerable<Tuple<Color, float>>)`.
    pub fn from_colors_and_proportions(items: &[(Color, f32)]) -> ColorGradient {
        ColorGradient::with_points(
            items
                .iter()
                .map(|&(c, p)| {
                    let scaled = Color::from_rgb_i32(
                        (c.r as f32 * p) as i32,
                        (c.g as f32 * p) as i32,
                        (c.b as f32 * p) as i32,
                    );
                    ColorPoint::new(scaled, 0.0)
                })
                .collect(),
        )
    }

    /// Builds a gradient from explicit points, as a file load or the editor does.
    pub fn from_parts(colors: Vec<ColorPoint>, alphas: Vec<AlphaPoint>) -> ColorGradient {
        ColorGradient { colors, alphas, ..ColorGradient::with_points(Vec::new()) }
    }

    #[inline]
    pub fn colors(&self) -> &[ColorPoint] {
        &self.colors
    }

    #[inline]
    pub fn alphas(&self) -> &[AlphaPoint] {
        &self.alphas
    }

    /// Replaces the colour points and drops the cached blends.
    pub fn set_colors(&mut self, colors: Vec<ColorPoint>) {
        self.colors = colors;
        self.caches = Caches::default();
    }

    pub fn set_alphas(&mut self, alphas: Vec<AlphaPoint>) {
        self.alphas = alphas;
        self.caches = Caches::default();
    }

    /// `CloneDataFrom`: takes the other gradient's data and leaves this one's library link alone.
    pub fn clone_data_from(&mut self, other: &ColorGradient) {
        self.colors = other.colors.clone();
        self.alphas = other.alphas.clone();
        self.gammacorrected = other.gammacorrected;
        if other.title.is_some() {
            self.title = other.title.clone();
        }
        self.caches = Caches::default();
    }

    pub fn is_library_reference(&self) -> bool {
        !self.library_reference_name.is_empty()
    }

    /// `Colors.SortedArray()`. For the small arrays gradients use, `Array.Sort` is an insertion
    /// sort and therefore stable, which is what this reproduces.
    pub fn sorted_colors(&self) -> Vec<ColorPoint> {
        let mut v = self.colors.clone();
        v.sort_by(|a, b| a.position.total_cmp(&b.position));
        v
    }

    fn sorted_alphas(&self) -> Vec<AlphaPoint> {
        let mut v = self.alphas.clone();
        v.sort_by(|a, b| a.position.total_cmp(&b.position));
        v
    }

    /// `GetColorBlend()`.
    pub fn color_blend(&self) -> Arc<ColorBlend> {
        self.caches.blend.get_or_init(|| Arc::new(self.create_color_blend(None))).clone()
    }

    /// `GetColorBlend(filterColor)`.
    pub fn filtered_blend(&self, filter: Color) -> Arc<ColorBlend> {
        if let Some((_, b)) = self.caches.filtered.read().iter().find(|(c, _)| *c == filter) {
            return b.clone();
        }
        let blend = Arc::new(self.create_color_blend(Some(filter)));
        self.caches.filtered.write().push((filter, blend.clone()));
        blend
    }

    fn create_color_blend(&self, filter: Option<Color>) -> ColorBlend {
        let mut colpoints = self.sorted_colors();
        let alphapoints = self.sorted_alphas();
        let mut positions = SortedPositions::default();

        if let Some(filter_color) = filter {
            let mut new_points: Vec<ColorPoint> = Vec::new();
            for cp in &colpoints {
                let mut np = *cp;
                if np.rgb_color() != filter_color {
                    if !new_points.iter().any(|x| x.position == np.position) {
                        np.color = Xyz::from_color(Color::BLACK);
                        new_points.push(np);
                    }
                } else {
                    new_points.retain(|x| {
                        !(x.position == np.position && x.rgb_color().to_argb() == Color::BLACK.to_argb())
                    });
                    new_points.push(np);
                }
            }
            colpoints = new_points;
        }

        for c in 0..colpoints.len() {
            if c > 0 && colpoints[c].focus != 0.5 && filter.is_none() {
                let pos = colpoints[c - 1].position
                    + (colpoints[c].position - colpoints[c - 1].position) * colpoints[c].focus;
                add_position(&colpoints, &alphapoints, &mut positions, pos);
            }
            add_position(&colpoints, &alphapoints, &mut positions, colpoints[c].position);
        }

        if positions.len() < 1 {
            positions.add(0.0, Color::TRANSPARENT);
        }
        if !positions.contains(0.0) {
            if filter.is_some() {
                let earliest = positions.key(0);
                let mut c = Color::BLACK;
                let mut i = 0;
                while i < positions.len() && positions.key(i) == earliest {
                    if positions.value(i).to_argb() != Color::BLACK.to_argb() {
                        c = positions.value(i);
                        break;
                    }
                    i += 1;
                }
                positions.add(0.0, c);
            } else {
                let first = positions.value(0);
                positions.add(0.0, first);
            }
        }

        if positions.len() < 2 {
            let mut c = positions.value(0);
            if let Some(f) = filter {
                if c.to_argb() != f.to_argb() {
                    c = Color::BLACK;
                }
            }
            positions.add(1.0, c);
        }

        if !positions.contains(1.0) {
            if filter.is_some() {
                let latest = positions.key(positions.len() - 1);
                let mut c = Color::BLACK;
                let mut i = positions.len() as isize - 1;
                while i >= 0 && positions.key(i as usize) == latest {
                    if positions.value(i as usize).to_argb() != Color::BLACK.to_argb() {
                        c = positions.value(i as usize);
                        break;
                    }
                    i -= 1;
                }
                positions.add(1.0, c);
            } else {
                let last = positions.value(positions.len() - 1);
                positions.add(1.0, last);
            }
        }

        ColorBlend {
            colors: positions.items.iter().map(|x| x.1).collect(),
            positions: positions.items.iter().map(|x| x.0).collect(),
        }
    }

    /// `GetColorAt(double)`.
    #[inline]
    pub fn get_color_at(&self, pos: f64) -> Color {
        self.get_color_at_f32(pos as f32)
    }

    /// `GetColorAt(float)`.
    pub fn get_color_at_f32(&self, pos: f32) -> Color {
        let blend = self.color_blend();
        let pos = clamp01(pos);
        let (a, b) = search_pos(&blend.positions, pos);
        interpolate_blend(&blend.colors, &blend.positions, a, b, pos)
    }

    /// `GetColorsExactlyAt(float)`.
    pub fn get_colors_exactly_at(&self, pos: f32) -> Vec<Color> {
        let blend = self.color_blend();
        let pos = clamp01(pos);
        blend
            .colors
            .iter()
            .zip(blend.positions.iter())
            .filter(|(_, &p)| p == pos)
            .map(|(&c, _)| c)
            .collect()
    }

    /// `GetColorsInGradient()`: distinct colours, in point order.
    pub fn colors_in_gradient(&self) -> Vec<Color> {
        let mut out: Vec<Color> = Vec::with_capacity(self.colors.len());
        for cp in &self.colors {
            let c = cp.rgb_color();
            if !out.contains(&c) {
                out.push(c);
            }
        }
        out
    }

    /// `GetDiscreteColorsAndProportionsAt(double)`.
    pub fn get_discrete_colors_and_proportions_at(&self, pos: f64) -> Vec<(Color, f32)> {
        let pos = clamp01(pos as f32);
        let blend = self.color_blend();
        let (a, b) = search_pos(&blend.positions, pos);
        if b < a {
            return Vec::new();
        }
        let mut colors: Vec<Color> = Vec::new();
        let mut add = |list: Vec<Color>| {
            for c in list {
                if !colors.contains(&c) {
                    colors.push(c);
                }
            }
        };
        add(self.get_colors_exactly_at(blend.positions[a as usize]));
        if a != b {
            add(self.get_colors_exactly_at(blend.positions[b as usize]));
        }
        colors.into_iter().map(|c| (c, self.get_proportion_of_color_at_f32(pos, c))).collect()
    }

    /// `GetProportionOfColorAt(double, Color)`.
    #[inline]
    pub fn get_proportion_of_color_at(&self, pos: f64, color: Color) -> f64 {
        self.get_proportion_of_color_at_f32(pos as f32, color) as f64
    }

    /// `GetProportionOfColorAt(float, Color)`.
    pub fn get_proportion_of_color_at_f32(&self, pos: f32, color: Color) -> f32 {
        let blend = self.filtered_blend(color);
        let pos = clamp01(pos);
        let (a, b) = search_pos(&blend.positions, pos);
        interpolate_proportion(color, &blend.colors, &blend.positions, a, b, pos)
    }

    /// `GetReverseColorGradient()`.
    pub fn reversed(&self) -> ColorGradient {
        let mut value: Vec<ColorPoint> = Vec::with_capacity(self.colors.len());
        let mut color_position = 0usize;
        for cp in &self.colors {
            for (i, v) in value.iter().enumerate() {
                if cp.position > v.position {
                    color_position = i + 1;
                }
            }
            value.insert(color_position.min(value.len()), *cp);
        }
        let count = value.len();
        let mut new_points = value.clone();
        for i in (0..count).rev() {
            let idx = count - 1 - i;
            new_points[idx] = ColorPoint { position: 1.0 - value[i].position, ..value[i] };
            if i < count - 1 {
                new_points[idx].focus = 1.0 - value[i + 1].focus;
            }
        }
        let mut g = self.clone();
        g.set_colors(new_points);
        g
    }

    /// `GetSubGradient(double, double)`. Returns None where the C# throws for a reversed range.
    pub fn sub_gradient(&self, start: f64, end: f64) -> Option<ColorGradient> {
        let range = end - start;
        if range < 0.0 {
            return None;
        }
        let mut pts = vec![ColorPoint::new(self.get_color_at(start), 0.0)];
        for cp in &self.colors {
            if cp.position > start && cp.position < end {
                let scaled = (cp.position - start) / range;
                if !(0.0..=1.0).contains(&scaled) {
                    return None;
                }
                pts.push(ColorPoint::new(cp.rgb_color(), scaled));
            }
        }
        let mut d = start + 0.2;
        while d < end {
            let scaled = (d - start) / range;
            if !(0.0..=1.0).contains(&scaled) {
                return None;
            }
            pts.push(ColorPoint::new(self.get_color_at(d), scaled));
            d += 0.2;
        }
        pts.push(ColorPoint::new(self.get_color_at(end), 1.0));
        Some(ColorGradient::with_points(pts))
    }

    /// `GetSubGradientWithDiscreteColors(double, double)`.
    pub fn sub_gradient_with_discrete_colors(&self, start: f64, end: f64) -> Option<ColorGradient> {
        let range = end - start;
        if range < 0.0 {
            return None;
        }
        let mut pts = Vec::new();
        for (c, _) in self.get_discrete_colors_and_proportions_at(start) {
            pts.push(ColorPoint::new(c, 0.0));
        }
        for cp in &self.colors {
            if cp.position > start && cp.position < end {
                let scaled = (cp.position - start) / range;
                if !(0.0..=1.0).contains(&scaled) {
                    return None;
                }
                pts.push(ColorPoint::new(cp.rgb_color(), scaled));
            }
        }
        for (c, _) in self.get_discrete_colors_and_proportions_at(end) {
            pts.push(ColorPoint::new(c, 1.0));
        }
        Some(ColorGradient::with_points(pts))
    }
}

/// `GradientLevelPair`: a gradient and the brightness curve that goes with it.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct GradientLevelPair {
    pub color_gradient: ColorGradient,
    pub curve: crate::curve::Curve,
}

impl GradientLevelPair {
    /// `new GradientLevelPair(Color, CurveType.Flat100)`, the common default.
    pub fn solid(color: Color) -> GradientLevelPair {
        GradientLevelPair { color_gradient: ColorGradient::solid(color), curve: crate::curve::Curve::flat100() }
    }

    pub fn new(color_gradient: ColorGradient, curve: crate::curve::Curve) -> GradientLevelPair {
        GradientLevelPair { color_gradient, curve }
    }
}

/// `ColorGradient.Clamp`.
#[inline]
fn clamp01(v: f32) -> f32 {
    if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    }
}

/// `SortedList<float, Color>`.
#[derive(Default)]
struct SortedPositions {
    items: Vec<(f32, Color)>,
}

impl SortedPositions {
    fn len(&self) -> usize {
        self.items.len()
    }

    fn key(&self, i: usize) -> f32 {
        self.items[i].0
    }

    fn value(&self, i: usize) -> Color {
        self.items[i].1
    }

    fn index_of(&self, k: f32) -> Result<usize, usize> {
        self.items.binary_search_by(|x| x.0.total_cmp(&k))
    }

    fn contains(&self, k: f32) -> bool {
        self.index_of(k).is_ok()
    }

    fn add(&mut self, k: f32, c: Color) {
        match self.index_of(k) {
            Ok(i) => self.items[i].1 = c,
            Err(i) => self.items.insert(i, (k, c)),
        }
    }

    fn remove(&mut self, k: f32) {
        if let Ok(i) = self.index_of(k) {
            self.items.remove(i);
        }
    }

    fn get(&self, k: f32) -> Option<Color> {
        self.index_of(k).ok().map(|i| self.items[i].1)
    }
}

/// Anything `SearchPos` can compare against a position.
trait HasPos<K> {
    fn cmp_pos(&self, pos: K) -> i32;
}

impl HasPos<f32> for f32 {
    #[inline]
    fn cmp_pos(&self, pos: f32) -> i32 {
        float_compare(*self as f64, pos as f64)
    }
}

impl HasPos<f64> for ColorPoint {
    #[inline]
    fn cmp_pos(&self, pos: f64) -> i32 {
        float_compare(self.position, pos)
    }
}

impl HasPos<f64> for AlphaPoint {
    #[inline]
    fn cmp_pos(&self, pos: f64) -> i32 {
        float_compare(self.position, pos)
    }
}

#[inline]
fn float_compare(a: f64, b: f64) -> i32 {
    if a < b {
        -1
    } else if a > b {
        1
    } else if a == b {
        0
    } else if a.is_nan() {
        if b.is_nan() {
            0
        } else {
            -1
        }
    } else {
        1
    }
}

/// `SearchPos`: returns the bounding indices; `a == b` on an exact hit.
/// Indices are signed because an empty list yields (0, -1), which callers treat as "b < a".
fn search_pos<T: HasPos<K>, K: Copy>(list: &[T], pos: K) -> (isize, isize) {
    let mut a: isize = 0;
    let mut b: isize = list.len() as isize - 1;
    let mut start: isize = 0;
    let mut end: isize = b;
    while end >= start {
        let mid = start + (end - start) / 2;
        match list[mid as usize].cmp_pos(pos) {
            0 => return (mid, mid),
            1 => {
                end = mid - 1;
                b = mid;
            }
            _ => {
                start = mid + 1;
                a = mid;
            }
        }
    }
    (a, b)
}

fn add_position(colpoints: &[ColorPoint], alphapoints: &[AlphaPoint], positions: &mut SortedPositions, pos: f64) {
    let key = pos as f32;
    if let Some(existing) = positions.get(key) {
        if existing.to_argb() == Color::BLACK.to_argb() {
            positions.remove(key);
        } else {
            return;
        }
    }
    let (alpha_a, alpha_b) = search_pos(alphapoints, pos);
    let (color_a, color_b) = search_pos(colpoints, pos);
    let alpha = interpolate_alpha(alphapoints, alpha_a, alpha_b, pos);
    let color = interpolate_color(colpoints, color_a, color_b, pos);
    positions.add(key, color.with_alpha(alpha as i32));
}

fn focus_to_balance(a: f64, b: f64, focus: f64, pos: f64) -> f64 {
    if a >= b || pos <= a {
        return 0.0;
    }
    let ret = (pos - a) / (b - a);
    if focus != 0.5 {
        if ret == focus {
            0.5
        } else if ret < focus {
            0.5 * ret / focus
        } else {
            0.5 * (1.0 + (ret - focus) / (1.0 - focus))
        }
    } else {
        ret
    }
}

fn interpolate_alpha(list: &[AlphaPoint], a: isize, b: isize, pos: f64) -> u8 {
    if b < a {
        return 0;
    }
    let (ai, bi) = (a as usize, b as usize);
    if a == b {
        return (list[ai].alpha * 255.0) as u8;
    }
    clip_value(
        (list[ai].alpha
            + focus_to_balance(list[ai].position, list[bi].position, list[bi].focus, pos)
                * (list[bi].alpha - list[ai].alpha))
            * 255.0,
        0.0,
        255.0,
    ) as u8
}

fn interpolate_color(list: &[ColorPoint], a: isize, b: isize, pos: f64) -> Color {
    if b < a {
        return Color::BLACK;
    }
    let (ai, bi) = (a as usize, b as usize);
    if a == b {
        return list[ai].rgb_color();
    }
    let bal = focus_to_balance(list[ai].position, list[bi].position, list[bi].focus, pos);
    let col_a = list[ai].color.to_rgb();
    let col_b = list[bi].color.to_rgb();
    Rgb::new(
        col_a.r + bal * (col_b.r - col_a.r),
        col_a.g + bal * (col_b.g - col_a.g),
        col_a.b + bal * (col_b.b - col_a.b),
    )
    .to_color()
}

#[inline]
fn interpolate_blend(list: &[Color], positions: &[f32], a: isize, b: isize, pos: f32) -> Color {
    if b < a {
        return Color::BLACK;
    }
    let (ai, bi) = (a as usize, b as usize);
    if a == b {
        return list[ai];
    }
    if positions[ai] >= positions[bi] {
        return list[ai];
    }
    let bal = (pos - positions[ai]) / (positions[bi] - positions[ai]);
    let (ca, cb) = (list[ai], list[bi]);
    Color::from_argb_i32(
        (ca.a as f32 + bal * (cb.a as f32 - ca.a as f32)) as i32,
        (ca.r as f32 + bal * (cb.r as f32 - ca.r as f32)) as i32,
        (ca.g as f32 + bal * (cb.g as f32 - ca.g as f32)) as i32,
        (ca.b as f32 + bal * (cb.b as f32 - ca.b as f32)) as i32,
    )
}

fn interpolate_proportion(color: Color, list: &[Color], positions: &[f32], a: isize, b: isize, pos: f32) -> f32 {
    if b < a {
        return 0.0;
    }
    let (ai, bi) = (a as usize, b as usize);
    let argb = color.to_argb();
    if a == b || positions[ai] >= positions[bi] {
        return if list[ai].to_argb() == argb { 1.0 } else { 0.0 };
    }
    if list[ai].to_argb() == argb && list[bi].to_argb() == argb {
        return 1.0;
    }
    if list[ai].to_argb() != argb && list[bi].to_argb() != argb {
        return 0.0;
    }
    let bal = (pos - positions[ai]) / (positions[bi] - positions[ai]);
    if list[ai].to_argb() == argb {
        return (1.0_f64 - bal as f64) as f32;
    }
    bal
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_stop(a: Color, b: Color) -> ColorGradient {
        ColorGradient::from_parts(
            vec![ColorPoint::new(a, 0.0), ColorPoint::new(b, 1.0)],
            vec![AlphaPoint::new(255, 0.0), AlphaPoint::new(255, 1.0)],
        )
    }

    #[test]
    fn solid_is_solid_everywhere() {
        let g = ColorGradient::solid(Color::RED);
        for p in [0.0, 0.25, 0.5, 1.0] {
            assert_eq!(g.get_color_at(p), Color::RED);
        }
    }

    #[test]
    fn blends_linearly_between_stops() {
        let g = two_stop(Color::BLACK, Color::WHITE);
        assert_eq!(g.get_color_at(0.0), Color::BLACK);
        assert_eq!(g.get_color_at(1.0), Color::WHITE);
        let mid = g.get_color_at(0.5);
        assert_eq!(mid.r, 127);
        assert_eq!(mid.a, 255);
    }

    #[test]
    fn out_of_range_positions_clamp() {
        let g = two_stop(Color::RED, Color::BLUE);
        assert_eq!(g.get_color_at(-1.0), Color::RED);
        assert_eq!(g.get_color_at(2.0), Color::BLUE);
    }

    #[test]
    fn focus_shifts_the_midpoint() {
        let mut g = two_stop(Color::BLACK, Color::WHITE);
        let mut pts = g.colors().to_vec();
        pts[1].focus = 0.25;
        g.set_colors(pts);
        // With focus .25 the 50% colour sits at position .25.
        let at_focus = g.get_color_at(0.25);
        assert!((at_focus.r as i32 - 128).abs() <= 1, "{at_focus:?}");
    }

    #[test]
    fn proportion_of_discrete_colour() {
        let g = two_stop(Color::RED, Color::BLUE);
        assert_eq!(g.get_proportion_of_color_at(0.0, Color::RED), 1.0);
        assert_eq!(g.get_proportion_of_color_at(1.0, Color::RED), 0.0);
        assert!((g.get_proportion_of_color_at(0.25, Color::RED) - 0.75).abs() < 1e-6);
        assert_eq!(g.get_proportion_of_color_at(0.5, Color::LIME), 0.0);
    }

    #[test]
    fn discrete_colours_and_proportions_mid_gradient() {
        let g = two_stop(Color::RED, Color::BLUE);
        let at = g.get_discrete_colors_and_proportions_at(0.5);
        assert_eq!(at.len(), 2);
        let red = at.iter().find(|x| x.0 == Color::RED).unwrap();
        assert!((red.1 - 0.5).abs() < 1e-6);
    }

    #[test]
    fn reverse_flips_positions() {
        let g = two_stop(Color::RED, Color::BLUE).reversed();
        assert_eq!(g.get_color_at(0.0), Color::BLUE);
        assert_eq!(g.get_color_at(1.0), Color::RED);
    }

    #[test]
    fn sub_gradient_rejects_reversed_range() {
        assert!(two_stop(Color::RED, Color::BLUE).sub_gradient(0.8, 0.2).is_none());
    }

    #[test]
    fn colors_in_gradient_are_distinct() {
        let g = ColorGradient::from_colors(&[Color::RED, Color::RED, Color::BLUE]);
        assert_eq!(g.colors_in_gradient(), vec![Color::RED, Color::BLUE]);
    }

    #[test]
    fn mutation_invalidates_the_cached_blend() {
        let mut g = ColorGradient::solid(Color::RED);
        assert_eq!(g.get_color_at(0.5), Color::RED);
        g.set_colors(vec![ColorPoint::new(Color::BLUE, 0.0)]);
        assert_eq!(g.get_color_at(0.5), Color::BLUE);
    }
}
