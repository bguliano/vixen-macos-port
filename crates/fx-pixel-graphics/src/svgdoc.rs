//! A minimal mutable SVG document model for the Shapes effect. The C# edits an `Svg.SvgDocument` in place every
//! frame (fills, strokes, fades) and draws it. Here the document is a small element tree that is serialised
//! and rendered with resvg.

use vixen_effects::gfx::Bitmap;

#[derive(Clone, Debug, Default)]
pub struct SvgNode {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<SvgNode>,
    pub text: String,
}

impl SvgNode {
    pub fn new(tag: &str) -> SvgNode {
        SvgNode { tag: tag.to_string(), ..SvgNode::default() }
    }

    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    pub fn set_attr(&mut self, key: &str, value: &str) {
        match self.attrs.iter_mut().find(|(k, _)| k == key) {
            Some(a) => a.1 = value.to_string(),
            None => self.attrs.push((key.to_string(), value.to_string())),
        }
    }

    pub fn remove_attr(&mut self, key: &str) {
        self.attrs.retain(|(k, _)| k != key);
    }

    pub fn with_attr(mut self, key: &str, value: &str) -> SvgNode {
        self.set_attr(key, value);
        self
    }

    /// Parses an SVG file. Like Svg.NET, `style="a:b;c:d"` is split into individual attributes, which then
    /// replace attributes of the same name.
    pub fn parse(xml: &str) -> Option<SvgNode> {
        let xml = xml.trim_start_matches('\u{feff}');
        let opt = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
        let doc = roxmltree::Document::parse_with_options(xml, opt).ok()?;
        let root = doc.root_element();
        if root.tag_name().name() != "svg" {
            return None;
        }
        Some(Self::from_xml(root))
    }

    fn from_xml(n: roxmltree::Node) -> SvgNode {
        let mut out = SvgNode::new(n.tag_name().name());
        for a in n.attributes() {
            let name = match a.namespace() {
                Some("http://www.w3.org/1999/xlink") => format!("xlink:{}", a.name()),
                Some("http://www.w3.org/XML/1998/namespace") => format!("xml:{}", a.name()),
                _ => a.name().to_string(),
            };
            out.set_attr(&name, a.value());
        }
        if let Some(style) = out.attr("style").map(|s| s.to_string()) {
            out.remove_attr("style");
            for decl in style.split(';') {
                if let Some((k, v)) = decl.split_once(':') {
                    let (k, v) = (k.trim(), v.trim());
                    if !k.is_empty() {
                        out.set_attr(k, v);
                    }
                }
            }
        }
        for c in n.children() {
            if c.is_element() {
                out.children.push(Self::from_xml(c));
            } else if c.is_text() && (out.tag == "style" || out.tag == "text" || out.tag == "tspan") {
                out.text.push_str(c.text().unwrap_or(""));
            }
        }
        out
    }

    /// Serialises to SVG text. The root gets the SVG namespaces.
    pub fn to_svg(&self) -> String {
        let mut s = String::with_capacity(1024);
        self.write(&mut s, true);
        s
    }

    fn write(&self, s: &mut String, root: bool) {
        s.push('<');
        s.push_str(&self.tag);
        if root {
            s.push_str(" xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\"");
        }
        for (k, v) in &self.attrs {
            if root && (k == "xmlns" || k.starts_with("xmlns:")) {
                continue;
            }
            s.push(' ');
            s.push_str(k);
            s.push_str("=\"");
            for ch in v.chars() {
                match ch {
                    '&' => s.push_str("&amp;"),
                    '<' => s.push_str("&lt;"),
                    '"' => s.push_str("&quot;"),
                    c => s.push(c),
                }
            }
            s.push('"');
        }
        if self.children.is_empty() && self.text.is_empty() {
            s.push_str("/>");
            return;
        }
        s.push('>');
        for ch in self.text.chars() {
            match ch {
                '&' => s.push_str("&amp;"),
                '<' => s.push_str("&lt;"),
                c => s.push(c),
            }
        }
        for c in &self.children {
            c.write(s, false);
        }
        s.push_str("</");
        s.push_str(&self.tag);
        s.push('>');
    }

    /// The root's `viewBox` as (min x, min y, width, height).
    pub fn view_box(&self) -> Option<(f32, f32, f32, f32)> {
        let v = self.attr("viewBox")?;
        let n: Vec<f32> = v.split(|c: char| c == ',' || c.is_whitespace()).filter(|t| !t.is_empty()).filter_map(|t| t.parse().ok()).collect();
        if n.len() == 4 {
            Some((n[0], n[1], n[2], n[3]))
        } else {
            None
        }
    }

    pub fn set_view_box(&mut self, vb: (f32, f32, f32, f32)) {
        self.set_attr("viewBox", &format!("{} {} {} {}", vb.0, vb.1, vb.2, vb.3));
    }

    /// A root `width`/`height` attribute as a plain number of pixels, ignoring percentages.
    pub fn length_attr(&self, key: &str) -> Option<f32> {
        let v = self.attr(key)?.trim();
        if v.ends_with('%') {
            return None;
        }
        let end = v.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E')).unwrap_or(v.len());
        v[..end].parse().ok()
    }
}

/// Pre-order walk over the descendants of `root` (not the root itself), as `SvgElement.Descendants()` does.
/// `f` gets each node and the fill it inherits from its ancestors as already modified by earlier calls.
pub fn walk_descendants(root: &mut SvgNode, f: &mut impl FnMut(&mut SvgNode, Option<&str>)) {
    let inherited = root.attr("fill").map(|s| s.to_string());
    for c in root.children.iter_mut() {
        walk(c, inherited.as_deref(), f);
    }
}

fn walk(n: &mut SvgNode, inherited: Option<&str>, f: &mut impl FnMut(&mut SvgNode, Option<&str>)) {
    f(n, inherited);
    let own = n.attr("fill").map(|s| s.to_string()).or_else(|| inherited.map(|s| s.to_string()));
    for c in n.children.iter_mut() {
        walk(c, own.as_deref(), f);
    }
}

/// `#rrggbb`.
pub fn hex(c: vixen_core::Color) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

/// Parses an SVG colour value.
pub fn parse_color(s: &str) -> Option<vixen_core::Color> {
    let c: svgtypes::Color = s.trim().parse().ok()?;
    Some(vixen_core::Color::rgb(c.red, c.green, c.blue))
}

/// A parsed render tree, cached while the SVG text does not change.
pub struct RenderCache {
    text: String,
    tree: Option<resvg::usvg::Tree>,
}

impl RenderCache {
    pub fn new() -> RenderCache {
        RenderCache { text: String::new(), tree: None }
    }

    /// Renders `svg` (a root sized to its viewBox) onto a `w` x `h` bitmap, scaled to fill it and then
    /// rotated by `angle` degrees clockwise around the centre. Anything unparseable yields `None`.
    pub fn render(&mut self, svg: &str, w: i32, h: i32, angle: f32) -> Option<Bitmap> {
        if w <= 0 || h <= 0 {
            return None;
        }
        if self.tree.is_none() || self.text != svg {
            let opt = resvg::usvg::Options::default();
            self.tree = resvg::usvg::Tree::from_str(svg, &opt).ok();
            self.text = svg.to_string();
        }
        let tree = self.tree.as_ref()?;
        let size = tree.size();
        if size.width() <= 0.0 || size.height() <= 0.0 {
            return None;
        }
        let mut pm = tiny_skia::Pixmap::new(w as u32, h as u32)?;
        let scale = tiny_skia::Transform::from_scale(w as f32 / size.width(), h as f32 / size.height());
        let ts = if angle != 0.0 {
            scale.post_concat(tiny_skia::Transform::from_rotate_at(angle, w as f32 / 2.0, h as f32 / 2.0))
        } else {
            scale
        };
        resvg::render(tree, ts, &mut pm.as_mut());
        let mut rgba = Vec::with_capacity(pm.pixels().len() * 4);
        for p in pm.pixels() {
            let d = p.demultiply();
            rgba.extend_from_slice(&[d.red(), d.green(), d.blue(), d.alpha()]);
        }
        Some(Bitmap::from_rgba(w, h, &rgba))
    }
}

impl Default for RenderCache {
    fn default() -> Self {
        Self::new()
    }
}
