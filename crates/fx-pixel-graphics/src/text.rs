//! Port of VixenModules.Effect.Text: scrolling, rotating, exploding or falling text drawn with gradient brushes,
//! optionally driven by a mark collection.
//!
//! The C# draws with GDI+ and `SingleBitPerPixelGridFit`, so glyphs are not anti-aliased. `textdraw::Canvas`
//! reproduces that on top of Core Text. GDI+ pads `MeasureString` results on both sides; Core Text does not, so
//! text sits a pixel or two differently inside the same layout. Layout uses the same measurements as drawing,
//! so centring and scrolling stay self-consistent.

use crate::generated::text as gen;
use crate::textdraw::{char_ranges, line_height, measure, measure_precise, Brush, Canvas, GradMode};
use vixen_core::gradient::{ColorPoint, ColorGradient};
use vixen_core::Xyz;
use vixen_effects::gfx::Graphics;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    param_changed_ctx: Some(param_changed_ctx),
    supports_location: true,
    force_visual: true,
    visual: Some(visual),
    ported: true,
    ..gen::BASE
};

/// `Environment.NewLine` as used to separate stacked characters.
const NEWLINE: &str = "\n";

const D_LEFT: i32 = 0;
const D_RIGHT: i32 = 1;
const D_UP: i32 = 2;
const D_DOWN: i32 = 3;
const D_ROTATE: i32 = 4;
const D_EXPLODE: i32 = 6;

fn direction_ord(name: &str) -> i32 {
    match name {
        "Left" => D_LEFT,
        "Right" => D_RIGHT,
        "Up" => D_UP,
        "Down" => D_DOWN,
        "Rotate" => D_ROTATE,
        "None" => 5,
        "Explode" => D_EXPLODE,
        "Fall" => 7,
        _ => 0,
    }
}

/// The lines of `TextLines`, a `List<string>`. It is stored as the member's raw XML children
/// (`<d2p1:string>…</d2p1:string>`), or as plain text with newlines.
pub fn text_lines_of(v: &ParamValue) -> Vec<String> {
    match v {
        ParamValue::Raw(raw) => {
            let wrapped = format!(
                "<r xmlns:d2p1=\"http://schemas.microsoft.com/2003/10/Serialization/Arrays\" xmlns:i=\"http://www.w3.org/2001/XMLSchema-instance\">{raw}</r>"
            );
            match roxmltree::Document::parse(&wrapped) {
                Ok(doc) => doc
                    .root_element()
                    .children()
                    .filter(|c| c.is_element())
                    .map(|c| c.text().unwrap_or("").to_string())
                    .collect(),
                Err(_) => {
                    if raw.trim().is_empty() {
                        Vec::new()
                    } else {
                        raw.split('\n').map(|s| s.to_string()).collect()
                    }
                }
            }
        }
        ParamValue::Str(s) => s.split('\n').map(|s| s.to_string()).collect(),
        _ => Vec::new(),
    }
}

/// Builds the `TextLines` value for lines of text.
pub fn text_lines_value(lines: &[&str]) -> ParamValue {
    let mut s = String::new();
    for l in lines {
        let esc = l.replace('&', "&amp;").replace('<', "&lt;");
        s.push_str(&format!("<d2p1:string xmlns:d2p1=\"http://schemas.microsoft.com/2003/10/Serialization/Arrays\">{esc}</d2p1:string>"));
    }
    ParamValue::Raw(s)
}

fn param_changed_ctx(c: &ParamChange, p: &mut Params) {
    match c.name {
        // The setter calls ActivateMarkCollectionSelections.
        MARK_COLLECTIONS | "TextSource" => {
            normalize_mark_selection(p, c.info, p.enum_name("TextSource") != "None", None, true);
        }
        // Only one of the two stops at a time.
        "CenterStop" => {
            if p.boolean("CenterStop") {
                p.set("EndStop", ParamValue::Bool(false));
            }
        }
        "EndStop" => {
            if p.boolean("EndStop") {
                p.set("CenterStop", ParamValue::Bool(false));
            }
        }
        "CycleColor" => {
            if !p.boolean("CycleColor") {
                p.set("CycleCharacterColor", ParamValue::Bool(false));
            }
        }
        _ => {}
    }
}

/// `UpdateAllAttributes`: base colour, position and text mode attributes, in that order.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let dir = direction_ord(p.enum_name("Direction"));
    let src = p.enum_name("TextSource");
    let use_base = p.boolean("UseBaseColor");
    let per_word = p.boolean("DirectionPerWord");
    let cycle = p.boolean("CycleColor");

    v.set("BaseColor", use_base);
    v.set("BaseLevelCurve", use_base);

    v.set("XOffsetCurve", !(dir == D_LEFT || dir == D_RIGHT));
    v.set("YOffsetCurve", !(dir == D_UP || dir == D_DOWN));
    v.set("AngleCurve", dir == D_ROTATE);
    v.set("CenterStop", dir < D_ROTATE);
    v.set("EndStop", dir < D_ROTATE);
    v.set("ExplodePositionCurve", dir == D_EXPLODE);
    v.set("FallSpeedCurve", dir == 7);

    v.set("MarkCollectionId", src != "None");
    v.set("DirectionPerWord", src != "None" && dir < D_ROTATE);
    v.set("TimeVisibleLength", src != "None" && p.enum_name("TextDuration") == "UserDefined");
    v.set("TextDuration", src != "None");
    v.set("RepeatText", src == "MarkCollection");
    v.set("TextFade", src != "None");
    v.set("Speed", (src == "None" || !per_word) && dir < D_ROTATE);
    v.set("TextMode", src == "None");
    v.set("TextLines", src != "MarkCollectionLabels");
    v.set("CycleCharacterColor", false);
    v.set("CycleColor", true);
    v.set("CycleColorMode", cycle);

    v.set("TargetPositioning", true);
    v.set("StringOrientation", p.enum_name("TargetPositioning") == "Strings");
}

fn grad_mode_of(name: &str) -> (GradMode, bool) {
    // (brush mode, across element)
    match name {
        "AcrossText" => (GradMode::Horizontal, false),
        "AcrossElement" => (GradMode::Horizontal, true),
        "VerticalAcrossText" => (GradMode::Vertical, false),
        "VerticalAcrossElement" => (GradMode::Vertical, true),
        "DiagonalAcrossText" => (GradMode::ForwardDiagonal, false),
        "DiagonalAcrossElement" => (GradMode::ForwardDiagonal, true),
        "BackwardDiagonalAcrossText" => (GradMode::BackwardDiagonal, false),
        "BackwardDiagonalAcrossElement" => (GradMode::BackwardDiagonal, true),
        _ => (GradMode::Horizontal, true),
    }
}

#[derive(Clone, Default)]
struct TextClass {
    text: Vec<String>,
    start_frame: i32,
    frame: i32,
    end_frame: i32,
    character_start: i32,
}

#[derive(Clone, Copy)]
struct DirectionClass {
    x_offset: f64,
    y_offset: f64,
    delta: f64,
}

struct MarkInfo {
    start_ms: f64,
    end_ms: f64,
    text: String,
}

#[derive(Copy, Clone, PartialEq)]
enum Fade {
    None,
    In,
    Out,
    InOut,
}

#[derive(Copy, Clone, PartialEq)]
enum Duration {
    AutoFit,
    MarkDuration,
    UserDefined,
}

struct Text {
    colors: Vec<ColorGradient>,
    level_curve: Curve,
    base_color: Color,
    base_level_curve: Curve,
    use_base_color: bool,
    direction: i32,
    speed: i32,
    center_stop: bool,
    end_stop: bool,
    center_text: bool,
    grad: (GradMode, bool),
    rotated_mode: bool,
    x_offset_curve: Curve,
    y_offset_curve: Curve,
    angle_curve: Curve,
    font_scale_curve: Curve,
    explode_curve: Curve,
    fall_curve: Curve,
    font: FontDesc,
    text_lines: Vec<String>,
    source: u8, // 0 None, 1 MarkCollection, 2 Labels
    fade_mode: Fade,
    time_visible_length: i32,
    direction_per_word: bool,
    repeat_text: bool,
    duration: Duration,
    cycle_color: bool,
    cycle_word: bool,
    mark_collection_id: Uuid,
    locations: bool,
    // Per group
    marks: Vec<MarkInfo>,
    start_ms: f64,
    string_count: i32,
    // State
    base_font: FontDesc,
    new_font: FontDesc,
    text: Vec<String>,
    text_class: Vec<TextClass>,
    direction_class: Vec<DirectionClass>,
    direction_position: f64,
    fade: f64,
    level: f64,
    word_iteration: i32,
    explode_position: f64,
    character_number: i32,
    fall_speed: f64,
    max_text_size: i32,
    buffer_wi: i32,
    buffer_ht: i32,
    num_frames: i32,
    rand: Rand,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut e = Text {
        colors: p.gradients("Colors"),
        level_curve: p.curve("LevelCurve"),
        base_color: p.color("BaseColor"),
        base_level_curve: p.curve("BaseLevelCurve"),
        use_base_color: p.boolean("UseBaseColor"),
        direction: direction_ord(p.enum_name("Direction")),
        speed: p.int("Speed"),
        center_stop: p.boolean("CenterStop"),
        end_stop: p.boolean("EndStop"),
        center_text: p.boolean("CenterText"),
        grad: grad_mode_of(p.enum_name("GradientMode")),
        rotated_mode: p.enum_name("TextMode") == "Rotated",
        x_offset_curve: p.curve("XOffsetCurve"),
        y_offset_curve: p.curve("YOffsetCurve"),
        angle_curve: p.curve("AngleCurve"),
        font_scale_curve: p.curve("FontScaleCurve"),
        explode_curve: p.curve("ExplodePositionCurve"),
        fall_curve: p.curve("FallSpeedCurve"),
        font: p.font("Font"),
        text_lines: text_lines_of(p.get("TextLines")),
        source: match p.enum_name("TextSource") {
            "MarkCollection" => 1,
            "MarkCollectionLabels" => 2,
            _ => 0,
        },
        fade_mode: match p.enum_name("TextFade") {
            "In" => Fade::In,
            "Out" => Fade::Out,
            "InOut" => Fade::InOut,
            _ => Fade::None,
        },
        time_visible_length: p.int("TimeVisibleLength"),
        direction_per_word: p.boolean("DirectionPerWord"),
        repeat_text: p.boolean("RepeatText"),
        duration: match p.enum_name("TextDuration") {
            "MarkDuration" => Duration::MarkDuration,
            "UserDefined" => Duration::UserDefined,
            _ => Duration::AutoFit,
        },
        cycle_color: p.boolean("CycleColor"),
        cycle_word: p.enum_name("CycleColorMode") == "Word",
        mark_collection_id: p.guid("MarkCollectionId"),
        locations: TargetPositioning::from_name(p.enum_name("TargetPositioning")) == TargetPositioning::Locations,
        marks: Vec::new(),
        start_ms: 0.0,
        string_count: 0,
        base_font: FontDesc::new("Arial", 8.0),
        new_font: FontDesc::new("Arial", 8.0),
        text: Vec::new(),
        text_class: Vec::new(),
        direction_class: Vec::new(),
        direction_position: 0.0,
        fade: 1.0,
        level: 1.0,
        word_iteration: 0,
        explode_position: 0.0,
        character_number: 0,
        fall_speed: 0.0,
        max_text_size: 0,
        buffer_wi: 0,
        buffer_ht: 0,
        num_frames: 0,
        rand: Rand::default(),
    };
    let _ = ctx;
    render_pixel_effect(ctx, p, &mut e)
}

impl Text {
    fn is_character_cycle(&self) -> bool {
        self.cycle_color && !self.cycle_word
    }

    fn is_word_cycle(&self) -> bool {
        self.cycle_color && self.cycle_word
    }

    fn is_mark_word_cycle(&self) -> bool {
        self.source != 0 && self.is_word_cycle()
    }

    fn has_colors(&self) -> bool {
        !self.colors.is_empty()
    }

    fn calculate_x_offset(&self, pos: f64) -> i32 {
        scale_curve_to_value(self.x_offset_curve.get_value(pos), 100.0, -100.0) as i32
    }

    fn calculate_y_offset(&self, pos: f64) -> i32 {
        scale_curve_to_value(self.y_offset_curve.get_value(pos), -100.0, 100.0) as i32
    }

    fn calculate_angle(&self, pos: f64) -> i32 {
        scale_curve_to_value(self.angle_curve.get_value(pos), 360.0, 0.0) as i32
    }

    fn calculate_font_scale(&self, pos: f64) -> f32 {
        scale_curve_to_value(self.font_scale_curve.get_value(pos), 100.0, 1.0) as f32
    }

    fn calculate_fall_speed_scale(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.fall_curve.get_value(pos), 20.0, 0.0)
    }

    fn calculate_explode_speed(&self, pos: f64) -> f32 {
        let buffer_length = ((self.buffer_wi as f64).powi(2) + (self.buffer_ht as f64).powi(2)).sqrt() as i32;
        scale_curve_to_value(self.explode_curve.get_value(pos), buffer_length as f64, -(buffer_length as f64)) as f32
    }

    /// `GetNewGolorGradient`: scales every colour point by the fade and level.
    fn adjusted_gradient(&self, cg: &ColorGradient) -> ColorGradient {
        let points: Vec<ColorPoint> = cg
            .colors()
            .iter()
            .map(|pt| {
                let mut hsv = Hsv::from_color(pt.color.to_color());
                if self.fade_mode != Fade::None {
                    hsv.set_v(hsv.v * self.fade);
                }
                hsv.set_v(hsv.v * self.level);
                ColorPoint::from_xyz(Xyz::from_color(hsv.to_color()), pt.focus, pt.position)
            })
            .collect();
        ColorGradient::from_parts(points, cg.alphas().to_vec())
    }

    /// The gradient for colour index `i`, with the fade and level applied when they change anything.
    fn gradient_for(&self, i: i32) -> ColorGradient {
        let n = self.colors.len() as i32;
        let cg = &self.colors[i.rem_euclid(n) as usize];
        if self.level < 1.0 || self.fade_mode != Fade::None {
            self.adjusted_gradient(cg)
        } else {
            cg.clone()
        }
    }

    fn prepare_text_lines(&self) -> Vec<String> {
        if self.source == 0 && !self.rotated_mode {
            return self.text_lines.iter().map(|r| if r.trim().is_empty() { " ".to_string() } else { r.clone() }).collect();
        }
        if !self.rotated_mode || self.source == 1 {
            return self.text_lines.iter().filter(|x| !x.is_empty()).cloned().collect();
        }
        // SplitTextIntoCharacters
        let mut split = Vec::new();
        for line in &self.text_lines {
            split.extend(line.chars().map(|c| c.to_string()));
            split.push(NEWLINE.to_string());
        }
        // The C# throws removing from an empty list; here there is nothing to draw.
        split.pop();
        split
    }

    fn setup_marks(&mut self, info: &PixelInfo) {
        self.text_class = Vec::new();
        let n_frames = info.number_frames();
        if (!self.text.is_empty() || self.source == 2) && !self.marks.is_empty() {
            let mut i = 0usize;
            let mut current_mark = 0usize;
            let mark_count = self.marks.len();
            for mi in 0..mark_count {
                let (m_start, m_end, m_text) = (self.marks[mi].start_ms, self.marks[mi].end_ms, self.marks[mi].text.clone());
                let mut t = TextClass::default();
                if self.source == 2 {
                    t.text.push(m_text);
                } else {
                    for t1 in &self.text {
                        let parts: Vec<&str> = t1.split(' ').collect();
                        if parts.len() > i {
                            t.text.push(parts[i].to_string());
                        }
                    }
                }
                let mark_time = m_start - self.start_ms;
                t.frame = (mark_time as i32) / 50;
                t.end_frame = ((m_end - self.start_ms) as i32) / 50;
                if self.text_class.is_empty() {
                    t.start_frame = 0;
                    t.character_start = 0;
                } else {
                    let last = self.text_class.len() - 1;
                    t.start_frame = ((t.frame - self.text_class[last].frame) / 2) + self.text_class[last].frame;
                    let last_len = self.text_class[last].text.first().map(|s| s.chars().count() as i32).unwrap_or(0);
                    t.character_start = self.text_class[last].character_start + last_len;
                    if self.duration != Duration::MarkDuration {
                        self.text_class[last].end_frame = t.start_frame;
                    }
                }
                if mark_count == current_mark + 1 && self.duration != Duration::MarkDuration {
                    t.end_frame = n_frames;
                }
                i += 1;
                if t.text.is_empty() && self.repeat_text {
                    if let Some(first) = self.text_class.first() {
                        t.text = first.text.clone();
                    }
                    i = 1;
                }
                self.text_class.push(t);
                current_mark += 1;
            }
        }

        // Adjusts the start and end frames of each mark for the fade settings.
        let count = self.text_class.len();
        for i in 0..count {
            if self.text_class[i].text.is_empty() {
                self.text_class[i].text.push(" ".to_string());
            } else if self.text_class[i].text[0].is_empty() {
                self.text_class[i].text[0] = " ".to_string();
            }
            let frame = self.text_class[i].frame;
            let prev_frame = if i != 0 { Some(self.text_class[i - 1].frame) } else { None };
            let next_frame = if i != count - 1 { Some(self.text_class[i + 1].frame) } else { None };
            let tvl = self.time_visible_length as f64;
            let text = &mut self.text_class[i];
            match self.fade_mode {
                Fade::In => match self.duration {
                    Duration::AutoFit => {
                        if let Some(pf) = prev_frame {
                            text.start_frame = pf;
                        }
                        text.end_frame = text.frame;
                    }
                    Duration::MarkDuration => text.start_frame = text.frame,
                    Duration::UserDefined => {
                        let min_frame_offset = match prev_frame {
                            Some(pf) => frame - pf,
                            None => frame,
                        };
                        text.end_frame = text.frame;
                        text.start_frame = (frame as f64 - (tvl / 50.0).min(min_frame_offset as f64)) as i32;
                    }
                },
                Fade::None | Fade::Out => match self.duration {
                    Duration::AutoFit => {
                        if let Some(nf) = next_frame {
                            text.end_frame = nf;
                        }
                        text.start_frame = text.frame;
                    }
                    Duration::MarkDuration => text.start_frame = text.frame,
                    Duration::UserDefined => {
                        let max_frame_offset = match next_frame {
                            Some(nf) => nf - frame,
                            None => n_frames - frame,
                        };
                        text.start_frame = text.frame;
                        text.end_frame = (frame as f64 + (tvl / 50.0).min(max_frame_offset as f64)) as i32;
                    }
                },
                Fade::InOut => match self.duration {
                    Duration::MarkDuration => text.start_frame = text.frame,
                    Duration::UserDefined => {
                        let max_frame_offset = match next_frame {
                            Some(nf) => nf - frame,
                            None => n_frames - frame,
                        };
                        let min_frame_offset = match prev_frame {
                            Some(pf) => frame - pf,
                            None => frame,
                        };
                        text.end_frame = (frame as f64 + (tvl / 100.0).min(max_frame_offset as f64)) as i32;
                        text.start_frame = (frame as f64 - (tvl / 100.0).min(min_frame_offset as f64)) as i32;
                    }
                    Duration::AutoFit => {}
                },
            }
        }
    }

    fn create_direction_class(&mut self) {
        let angle = self.rand.next_range(0, 360);
        // The C# passes the integer straight to Math.Cos, so it is treated as radians.
        self.direction_class.push(DirectionClass { x_offset: (angle as f64).cos(), y_offset: (angle as f64).sin(), delta: 0.0 });
    }

    fn set_fade_position_level(&mut self, frame: i32, info: &PixelInfo) {
        self.direction_position = (info.interval_position(frame) * self.speed as f64) % 1.0;
        self.fade = 1.0;
        if self.source != 0 {
            let mut clear_text = true;
            for i in 0..self.text_class.len() {
                let t = self.text_class[i].clone();
                if frame >= t.start_frame && frame < t.end_frame {
                    self.explode_position =
                        self.calculate_explode_speed((frame - t.start_frame) as f64 / (t.end_frame - t.start_frame) as f64 * 100.0) as f64;
                    self.word_iteration = i as i32;
                    if self.source != 1 {
                        self.character_number = t.character_start;
                    }
                    self.text = t.text.clone();
                    match self.fade_mode {
                        Fade::In => {
                            self.fade = if self.duration != Duration::MarkDuration {
                                (frame - t.start_frame) as f64 / (t.frame - t.start_frame) as f64
                            } else {
                                (frame - t.frame) as f64 / (t.end_frame - t.frame) as f64
                            };
                        }
                        Fade::Out => self.fade = 1.0 - (frame - t.frame) as f64 / (t.end_frame - t.frame) as f64,
                        Fade::InOut => {
                            if self.duration != Duration::MarkDuration {
                                self.fade = if frame < t.frame {
                                    (frame - t.start_frame) as f64 / (t.frame - t.start_frame) as f64
                                } else {
                                    1.0 - (frame - t.frame) as f64 / (t.end_frame - t.frame) as f64
                                };
                            } else {
                                self.fade = if frame < t.frame + (t.end_frame - t.frame) / 2 {
                                    (frame - t.frame) as f64 / (t.end_frame - t.frame) as f64
                                } else {
                                    1.0 - (frame - t.frame) as f64 / (t.end_frame - t.frame) as f64
                                };
                            }
                        }
                        Fade::None => self.fade = 1.0,
                    }
                    if self.direction_per_word {
                        self.direction_position = (frame - t.start_frame) as f64 / (t.end_frame - t.start_frame) as f64;
                    }
                    clear_text = false;
                    break;
                }
            }
            if clear_text {
                self.text.clear();
            }
        }
    }

    /// `InitialRender`: draws the frame's text onto a new canvas.
    fn initial_render(&mut self, frame: i32, info: &PixelInfo) -> Canvas {
        let (buffer_ht, buffer_wi) = (info.buffer_ht(), info.buffer_wi());
        let mut canvas = Canvas::new(buffer_wi, buffer_ht);
        let interval_pos = info.interval_position(frame);
        let pos = interval_pos * 100.0;
        let text_angle = self.calculate_angle(pos);
        self.explode_position = self.calculate_explode_speed(pos) as f64;
        self.fall_speed = self.calculate_fall_speed_scale(pos);
        self.character_number = 0;

        // Fade level and text position offsets.
        self.set_fade_position_level(frame, info);

        if frame == 0 && ((self.direction != D_ROTATE && self.is_character_cycle()) || self.direction > 5) {
            self.direction_class = Vec::new();
            if self.source != 0 {
                let counts: Vec<usize> = self.text_class.iter().flat_map(|t| t.text.iter().map(|s| s.chars().count())).collect();
                for len in counts {
                    for _ in 0..=len {
                        self.create_direction_class();
                    }
                }
            } else {
                let counts: Vec<usize> = self.text.iter().map(|s| s.chars().count()).collect();
                for len in counts {
                    for _ in 0..(len + 1) {
                        self.create_direction_class();
                    }
                }
            }
        }

        if self.text.is_empty() {
            return canvas;
        }

        let number_lines = self.text.len() as i32;
        let mut textsize = (0.0f32, 0.0f32);

        // Font size from the font scale curve.
        let new_size = self.base_font.size * (self.calculate_font_scale(pos) / 100.0);
        self.new_font = FontDesc { name: self.font.name.clone(), size: new_size.max(0.1), bold: self.font.bold, italic: self.font.italic };

        for t in &self.text {
            if !t.is_empty() {
                let size = if t == NEWLINE { (0.0, line_height(&self.new_font)) } else { measure(t, &self.new_font) };
                if size.0 > textsize.0 {
                    textsize = size;
                }
            }
        }

        self.max_text_size = (textsize.0 as f64 * 0.95).round_ties_even() as i32;
        let maxht = (textsize.1 as f64 * number_lines as f64).round_ties_even() as i32;

        let mut x_offset = self.calculate_x_offset(pos);
        let mut y_offset = self.calculate_y_offset(pos);

        // Rotate the text around the centre of the image.
        if self.direction == D_ROTATE {
            canvas.set_rotation(text_angle as f32, buffer_wi as f32 / 2.0 + x_offset as f32, buffer_ht as f32 / 2.0 + (y_offset as f32 / 2.0));
        }

        match self.direction {
            D_LEFT | D_RIGHT => x_offset = 0,
            D_UP | D_DOWN => y_offset = 0,
            _ => {}
        }

        let offset_left = (((buffer_wi - self.max_text_size) / 2) * 2 + x_offset) / 2;
        let offset_top = (((buffer_ht - maxht) / 2) * 2 + y_offset) / 2;
        let tw = textsize.0;
        let th = textsize.1;
        let point = match self.direction {
            D_LEFT => {
                let mut left_x = buffer_wi - (self.direction_position * (tw as f64 + buffer_wi as f64)) as i32;
                if self.center_stop {
                    left_x = left_x.max((buffer_wi - tw as i32) / 2);
                } else if self.end_stop {
                    left_x = left_x.max(buffer_wi - tw as i32);
                }
                (left_x, offset_top)
            }
            D_RIGHT => {
                let mut right_x = -self.max_text_size + (self.direction_position * (self.max_text_size as f64 + buffer_wi as f64)) as i32;
                if self.center_stop {
                    right_x = right_x.min((buffer_wi - tw as i32) / 2);
                } else if self.end_stop {
                    right_x = right_x.min(0);
                }
                (right_x, offset_top)
            }
            D_UP => {
                let block = th as f64 * number_lines as f64;
                let mut up_y = buffer_ht - ((block + buffer_ht as f64) * self.direction_position) as i32;
                if self.center_stop {
                    up_y = up_y.max((buffer_ht - block as i32) / 2);
                } else if self.end_stop {
                    up_y = up_y.max(buffer_ht - th as i32);
                }
                (offset_left, up_y)
            }
            D_DOWN => {
                let block = th as f64 * number_lines as f64;
                let mut down_y = -(block as i32) + ((block + buffer_ht as f64) * self.direction_position) as i32;
                if self.center_stop {
                    down_y = down_y.min((buffer_ht - block as i32) / 2);
                } else if self.end_stop {
                    down_y = down_y.min(0);
                }
                (offset_left, down_y)
            }
            _ => ((buffer_wi - self.max_text_size) / 2 + x_offset, offset_top),
        };

        let lines = self.text.clone();
        self.draw_text(&lines, &mut canvas, point, buffer_wi, buffer_ht);
        canvas
    }

    fn draw_text(&mut self, lines: &[String], canvas: &mut Canvas, p: (i32, i32), buffer_wi: i32, buffer_ht: i32) {
        if !self.has_colors() {
            return;
        }
        let (mode, across_element) = self.grad;
        let mut p = p;
        let mut i = 0i32;
        for text in lines {
            let size = if text == NEWLINE { (0.0, line_height(&self.new_font)) } else { measure(text, &self.new_font) };
            let offset = self.max_text_size - size.0 as i32;
            let offset_point = (p.0 + offset / 2, p.1);
            let cg = if self.is_mark_word_cycle() { self.gradient_for(self.word_iteration) } else { self.gradient_for(i) };
            let draw_at = if self.center_text { offset_point } else { p };

            let rect = if across_element {
                (0, 0, buffer_wi, buffer_ht)
            } else {
                let mut brush_x = p.0;
                if self.center_text && self.rotated_mode && self.source == 0 {
                    brush_x = p.0 - offset / 2;
                } else if self.center_text {
                    brush_x = offset_point.0;
                }
                (brush_x, p.1, if self.rotated_mode && self.source == 0 { self.max_text_size } else { size.0 as i32 }, size.1 as i32)
            };
            let brush = Brush::new(rect, mode, cg);
            self.draw_text_with_brush(text, brush.as_ref(), canvas, draw_at, mode, size, buffer_ht);

            p.1 += size.1 as i32;
            if !self.rotated_mode || self.source == 1 {
                i += 1;
            } else if text == NEWLINE {
                i += 1;
            }
        }
    }

    fn draw_text_with_brush(
        &mut self,
        text: &str,
        brush: Option<&Brush>,
        canvas: &mut Canvas,
        p: (i32, i32),
        mode: GradMode,
        _size: (f32, f32),
        buffer_ht: i32,
    ) {
        let mut p = p;
        let font = self.new_font.clone();
        if text == NEWLINE {
            return;
        }
        if self.is_word_cycle() && self.direction <= 5 && self.has_colors() {
            self.draw_text_with_word_cycle(text, canvas, p, mode);
        } else if (self.direction != D_ROTATE && self.is_character_cycle()) || self.direction > 5 {
            // Each character is drawn on its own.
            for s in text.split(' ') {
                let word = format!("{s} ");
                let text_size = measure(&word, &font);
                let ranges = char_ranges(&word, &font);
                let row_h = measure_precise(&word, &font).1;
                for (i, (rx, rw)) in ranges.iter().enumerate() {
                    let rect_x = (p.0 as f32 + rx) as i32;
                    let w = ((rw + 1.0) as f64 * 1.6) as i32;
                    let h = row_h as i32 + 1;
                    let (x, y) = match self.direction {
                        D_EXPLODE => match self.direction_class.get(self.character_number as usize) {
                            Some(dc) => (
                                (rect_x as f64 + dc.x_offset * self.explode_position) as i32,
                                (p.1 as f64 + dc.y_offset * self.explode_position) as i32,
                            ),
                            None => {
                                self.character_number += 1;
                                continue;
                            }
                        },
                        7 => match self.direction_class.get_mut(self.character_number as usize) {
                            Some(dc) => {
                                dc.delta += self.rand.next_double() * self.fall_speed;
                                let wrapped = if buffer_ht != 0 { dc.delta % buffer_ht as f64 } else { 0.0 };
                                (rect_x, (p.1 as f64 + wrapped) as i32)
                            }
                            None => {
                                self.character_number += 1;
                                continue;
                            }
                        },
                        _ => (rect_x, p.1),
                    };
                    let ch: String = word.chars().skip(i).take(1).collect();
                    if self.is_character_cycle() && self.has_colors() {
                        let cg = self.gradient_for(self.character_number);
                        if let Some(b) = Brush::new((x, y, w, h), mode, cg) {
                            canvas.draw_string(&ch, &font, &b, x, y);
                        }
                    } else if let Some(b) = brush {
                        canvas.draw_string(&ch, &font, b, x, y);
                    }
                    self.character_number += 1;
                }
                p.0 += text_size.0 as i32;
            }
        } else if let Some(b) = brush {
            canvas.draw_string(text, &font, b, p.0, p.1);
        }
    }

    fn draw_text_with_word_cycle(&mut self, text: &str, canvas: &mut Canvas, p: (i32, i32), mode: GradMode) {
        let font = self.new_font.clone();
        if self.source != 0 {
            let size = measure(text, &font);
            let cg = self.gradient_for(self.word_iteration);
            if let Some(b) = Brush::new((p.0, p.1, (size.0 as i32).max(1), (size.1 as i32).max(1)), mode, cg) {
                canvas.draw_string(text, &font, &b, p.0, p.1);
            }
            return;
        }
        let mut p = p;
        let mut word_index = 0;
        for run in split_space_preserving_word_runs(text) {
            let run_size = measure(&run, &font);
            if run.trim().is_empty() {
                p.0 += run_size.0 as i32;
                continue;
            }
            let cg = self.gradient_for(word_index);
            if let Some(b) = Brush::new((p.0, p.1, (run_size.0 as i32).max(1), (run_size.1 as i32).max(1)), mode, cg) {
                canvas.draw_string(&run, &font, &b, p.0, p.1);
            }
            p.0 += run_size.0 as i32;
            word_index += 1;
        }
    }
}

/// `SplitTextIntoSpacePreservingWordRuns`.
fn split_space_preserving_word_runs(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut runs = Vec::new();
    if chars.is_empty() {
        return runs;
    }
    let mut run_start = 0;
    let mut current_is_space = chars[0] == ' ';
    for i in 1..chars.len() {
        let is_space = chars[i] == ' ';
        if is_space == current_is_space {
            continue;
        }
        runs.push(chars[run_start..i].iter().collect());
        run_start = i;
        current_is_space = is_space;
    }
    runs.push(chars[run_start..].iter().collect());
    runs
}

impl PixelEffect for Text {
    fn begin_group(&mut self, targets: &[NodeId], ctx: &RenderCtx) {
        self.string_count = ctx.tree.leaf_parents(targets).len() as i32;
        self.start_ms = ctx.start_time.total_ms();
        self.marks.clear();
        if self.source != 0 {
            // `MarksInclusiveOfTime(StartTime, StartTime + TimeSpan)`
            if let Some(mc) = ctx.mark_collection(self.mark_collection_id) {
                for m in mc.marks_inclusive_of_time(ctx.start_time, ctx.start_time + ctx.time_span) {
                    self.marks.push(MarkInfo { start_ms: m.start_time.total_ms(), end_ms: m.end_time().total_ms(), text: m.text.clone() });
                }
            }
        }
    }

    fn setup_render(&mut self, info: &PixelInfo) {
        self.buffer_wi = info.buffer_wi();
        self.buffer_ht = info.buffer_ht();
        self.num_frames = info.number_frames();
        self.text_class = Vec::new();
        self.text = self.prepare_text_lines();
        if self.source != 0 {
            self.setup_marks(info);
        }
        if self.locations {
            // Keeps the text the same visual size when switching between strings and locations.
            let sc = self.string_count.max(1) as f64;
            let fs = self.font.size as f64;
            let new_size = (sc - ((sc - fs) / 100.0) * 100.0) * (info.buffer_ht() as f64 / sc);
            self.base_font = FontDesc { name: self.font.name.clone(), size: (new_size as i32).max(1) as f32, bold: self.font.bold, italic: self.font.italic };
        } else {
            // `ScalingTools.GetScaleFactor()` is 1 here: sizes are 96 DPI points.
            self.base_font = self.font.clone();
        }
        self.new_font = self.base_font.clone();
    }

    fn cleanup_render(&mut self) {
        self.text.clear();
        self.text_class = Vec::new();
        self.direction_class = Vec::new();
    }

    fn base_color(&self) -> Option<(Color, Curve)> {
        if self.use_base_color {
            Some((self.base_color, self.base_level_curve.clone()))
        } else {
            None
        }
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let (ht, wi) = (info.buffer_ht(), info.buffer_wi());
        self.level = self.level_curve.get_value(info.interval_position(frame) * 100.0) / 100.0;
        let canvas = self.initial_render(frame, info);
        if self.text.is_empty() && !self.use_base_color {
            return;
        }
        for x in 0..wi {
            for y in 0..ht {
                let c = canvas.get(x, ht - y - 1);
                if c.a != 0 {
                    buf.set_pixel(x, y, c);
                }
            }
        }
    }

    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        let (ht, _wi) = (info.buffer_ht(), info.buffer_wi());
        let mut nodes = buf.element_locations().to_vec();
        nodes.sort_by_key(|l| (l.x, l.y));
        for frame in 0..num_frames {
            buf.current_frame = frame;
            self.level = self.level_curve.get_value(info.interval_position(frame) * 100.0) / 100.0;
            let canvas = self.initial_render(frame, info);
            if self.text.is_empty() && !self.use_base_color {
                continue;
            }
            for l in &nodes {
                // Flip and offset the coordinates so the location version acts like the string version.
                let mut y = ((info.buffer_ht_offset() - l.y) + (ht - 1 + info.buffer_ht_offset())).abs();
                y -= info.buffer_ht_offset();
                let x = l.x - info.buffer_wi_offset();
                let c = canvas.get(x, ht - y - 1);
                if c.a != 0 {
                    buf.set_pixel(l.x, l.y, c);
                } else {
                    // Unlit locations take the base colour or transparent.
                    buf.set_pixel(l.x, l.y, if self.use_base_color { self.base_color } else { Color::TRANSPARENT });
                }
            }
        }
    }
}

// ---- Timeline thumbnail (GenerateVisualRepresentation) ----

/// `Vixen.Common.Graphics.GetAdjustedFont`.
pub fn adjusted_font(text: &str, clip: (i32, i32, i32, i32), font: &FontDesc, max_size: i32, min_size: i32) -> FontDesc {
    let size = measure(text, font);
    let ratio = (clip.3 as f64 / size.1 as f64).min((clip.2 as f64 + 15.0) / size.0 as f64);
    let mut new_size = (font.size as f64 * ratio) as f32 - 2.0;
    if new_size < min_size as f32 {
        new_size = min_size as f32;
    } else if new_size > max_size as f32 {
        new_size = max_size as f32;
    }
    FontDesc { name: font.name.clone(), size: new_size, bold: font.bold, italic: font.italic }
}

/// Draws anti-aliased gradient text into a region of `g`, as `Graphics.DrawString` with a brush does on the
/// timeline thumbnail.
pub fn blend_text(g: &mut Graphics, region: (i32, i32, i32, i32), text: &str, font: &FontDesc, brush: &Brush, x: i32, y: i32) {
    if text.is_empty() || region.2 <= 0 || region.3 <= 0 {
        return;
    }
    let mut mask = Graphics::new(region.2, region.3);
    mask.draw_string(text, font, Color::WHITE, x as f32, y as f32);
    blend_mask(g, region, mask.bitmap(), brush);
}

/// Composites a white-on-transparent mask's coverage over `g`, coloured by `brush` (region coordinates).
pub fn blend_mask(g: &mut Graphics, region: (i32, i32, i32, i32), mask: &vixen_effects::gfx::Bitmap, brush: &Brush) {
    for yy in 0..region.3 {
        for xx in 0..region.2 {
            let cov = mask.get_pixel(xx, yy).a as u32;
            if cov == 0 {
                continue;
            }
            let src = brush.color_at(xx as f32 + 0.5, yy as f32 + 0.5);
            let a = cov * src.a as u32 / 255;
            if a == 0 {
                continue;
            }
            let (px, py) = (region.0 + xx, region.1 + yy);
            let dst = g.bitmap().get_pixel(px, py);
            let out = if dst.a == 0 {
                Color::argb(a as u8, src.r, src.g, src.b)
            } else {
                let inv = 255 - a;
                let mix = |s: u8, d: u8| ((s as u32 * a + d as u32 * inv) / 255) as u8;
                Color::argb(
                    (a + dst.a as u32 * inv / 255).min(255) as u8,
                    mix(src.r, dst.r),
                    mix(src.g, dst.g),
                    mix(src.b, dst.b),
                )
            };
            g.bitmap_mut().set_pixel(px, py, out);
        }
    }
}

/// `GenerateVisualRepresentation`: with marks, the text of each one where it falls in the effect.
fn visual(p: &Params, t: &TargetInfo, g: &mut Graphics, rect: (i32, i32, i32, i32)) {
    let lines = text_lines_of(p.get("TextLines"));
    let first = lines.first().map_or("", |l| l.as_str());
    let colors = p.gradients("Colors");
    let mode = match p.enum_name("GradientMode") {
        "VerticalAcrossElement" | "VerticalAcrossText" => GradMode::Vertical,
        "DiagonalAcrossElement" | "DiagonalAcrossText" => GradMode::ForwardDiagonal,
        "BackwardDiagonalAcrossElement" | "BackwardDiagonalAcrossText" => GradMode::BackwardDiagonal,
        _ => GradMode::Horizontal,
    };
    // `_marks`, from SetupMarks.
    let marks: Vec<&Mark> =
        t.mark_collection(p.guid("MarkCollectionId")).map_or(Vec::new(), |mc| mc.marks_inclusive_of_time(t.start_time, t.start_time + t.time_span));
    let start_of = |mark: &Mark| ((mark.start_time.ticks() - t.start_time.ticks()) as f64 / t.time_span.ticks() as f64 * rect.2 as f64) as i32;
    let word_cycle = p.boolean("CycleColor") && p.enum_name("CycleColorMode") == "Word";
    match p.enum_name("TextSource") {
        "MarkCollectionLabels" => {
            let mut word = -1;
            for mark in marks {
                word = if word_cycle { word + 1 } else { 0 };
                if !mark.text.is_empty() {
                    draw_visual_text(p, g, rect, &colors, mode, &mark.text, start_of(mark), word);
                }
            }
        }
        "MarkCollection" => {
            if first.is_empty() {
                return;
            }
            // `string.Split()`: on any white space, with the empty strings between repeats.
            let words: Vec<&str> = first.split(char::is_whitespace).collect();
            let mut i = 0;
            let mut word = 0;
            for mark in marks {
                draw_visual_text(p, g, rect, &colors, mode, words[i], start_of(mark), word);
                i += 1;
                if word_cycle {
                    word += 1;
                }
                if p.boolean("RepeatText") {
                    if i % words.len() == 0 {
                        i = 0;
                    }
                } else if words.len() <= i {
                    return;
                }
            }
        }
        _ => {
            if !first.is_empty() {
                draw_visual_text(p, g, rect, &colors, mode, first, 2, 0);
            }
        }
    }
}

/// `DrawText`.
#[allow(clippy::too_many_arguments)]
fn draw_visual_text(
    p: &Params,
    g: &mut Graphics,
    rect: (i32, i32, i32, i32),
    colors: &[ColorGradient],
    mode: GradMode,
    text: &str,
    start_x: i32,
    word_iteration: i32,
) {
    if colors.is_empty() {
        return;
    }
    let by_word = |i: i32| colors[i.rem_euclid(colors.len() as i32) as usize].clone();
    let cycle = p.boolean("CycleColor");
    let word_mode = p.enum_name("CycleColorMode") == "Word";
    let local = (0, 0, rect.2, rect.3);
    let font = adjusted_font(text, local, &p.font("Font"), 48, 10);
    let size = measure(text, &font);

    if cycle && !word_mode {
        let mut x0 = start_x;
        for s in text.split(' ') {
            let word = format!("{s} ");
            let text_size = measure(&word, &font);
            let row_h = measure_precise(&word, &font).1;
            for (i, (rx, rw)) in char_ranges(&word, &font).iter().enumerate() {
                let rx = (x0 as f32 + rx) as i32;
                let (w, h) = (((rw + 1.0) as f64 * 1.6) as i32, row_h as i32 + 1);
                let ch: String = word.chars().skip(i).take(1).collect();
                if let Some(b) = Brush::new((rx, 2, w, h), mode, colors[i % colors.len()].clone()) {
                    blend_text(g, rect, &ch, &font, &b, rx, 2);
                }
            }
            x0 += text_size.0 as i32;
        }
    } else if cycle && p.enum_name("TextSource") == "None" {
        let mut x0 = start_x;
        let mut word_index = 0;
        for run in split_space_preserving_word_runs(text) {
            let run_size = measure(&run, &font);
            if run.trim().is_empty() {
                x0 += run_size.0 as i32;
                continue;
            }
            if let Some(b) = Brush::new((x0, 2, (run_size.0 as i32).max(1), (run_size.1 as i32).max(1)), mode, by_word(word_index)) {
                blend_text(g, rect, &run, &font, &b, x0, 2);
            }
            x0 += run_size.0 as i32;
            word_index += 1;
        }
    } else {
        // The word cycle with marks keeps its one word whole, at least a pixel wide.
        let (w, h) = if cycle { ((size.0 as i32).max(1), (size.1 as i32).max(1)) } else { (size.0 as i32, size.1 as i32) };
        if let Some(b) = Brush::new((start_x, 2, w, h), mode, by_word(word_iteration)) {
            blend_text(g, rect, text, &font, &b, start_x, 2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use vixen_effects::testing;

    fn lines(p: &mut Params, l: &[&str]) {
        p.set("TextLines", text_lines_value(l));
    }

    #[test]
    fn smoke() {
        // Default text is empty, so give it something to draw.
        testing::smoke_test(&DESCRIPTOR);
        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 12, 30);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[m], 2000, |p| {
            lines(p, &["HI"]);
            p.set("Direction", ParamValue::Enum("None".into()));
            p.set("Font", ParamValue::Font(FontDesc::new("Arial", 9.0)));
        });
        assert!(lit_count(&tree, m, &out, 0) > 3);
    }

    #[test]
    fn every_direction_and_cycle_renders() {
        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 12, 30);
        for dir in ["Left", "Right", "Up", "Down", "Rotate", "None", "Explode", "Fall"] {
            for cycle in [false, true] {
                testing::render_with(&DESCRIPTOR, &tree, &[m], 1500, |p| {
                    lines(p, &["Hi there", "again"]);
                    p.set("Direction", ParamValue::Enum(dir.into()));
                    p.set("CycleColor", ParamValue::Bool(cycle));
                });
            }
        }
    }

    #[test]
    fn text_lines_round_trip() {
        let v = text_lines_value(&["a & b", "", "c"]);
        assert_eq!(text_lines_of(&v), vec!["a & b".to_string(), "".to_string(), "c".to_string()]);
        let d = DESCRIPTOR.new_params();
        assert_eq!(text_lines_of(d.get("TextLines")), vec!["".to_string()]);
    }

    #[test]
    fn left_scroll_enters_from_the_right() {
        // Direction Left starts the text off the right edge, so the first frame is dark and the text then
        // appears at the right-hand end of the element.
        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 12, 40);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[m], 2000, |p| {
            lines(p, &["I"]);
            p.set("Direction", ParamValue::Enum("Left".into()));
            p.set("Font", ParamValue::Font(FontDesc::new("Arial", 9.0)));
        });
        assert_eq!(lit_count(&tree, m, &out, 0), 0);
        // Late in the effect some column (string index is the Y axis in vertical orientation) holds lit pixels.
        let late = lit_count(&tree, m, &out, 1800);
        assert!(late > 0 || lit_count(&tree, m, &out, 1000) > 0);
    }

    #[test]
    fn word_runs_keep_spaces() {
        assert_eq!(split_space_preserving_word_runs("ab  cd e"), vec!["ab", "  ", "cd", " ", "e"]);
    }

    #[test]
    fn thumbnail_draws_coloured_text() {
        let mut g = Graphics::new(80, 20);
        let mut p = DESCRIPTOR.new_params();
        lines(&mut p, &["Hi"]);
        visual(&p, &TargetInfo::detached(), &mut g, (0, 0, 80, 20));
        let lit = (0..20).flat_map(|y| (0..80).map(move |x| (x, y))).filter(|(x, y)| g.bitmap().get_pixel(*x, *y).a > 0).count();
        assert!(lit > 10);
        // Default colour is red.
        let c = (0..80).flat_map(|x| (0..20).map(move |y| (x, y))).map(|(x, y)| g.bitmap().get_pixel(x, y)).find(|c| c.a > 200).unwrap();
        assert!(c.r > 200 && c.g < 50 && c.b < 50, "{c:?}");
    }

    #[test]
    fn visibility_rules() {
        let mut p = DESCRIPTOR.new_params();
        p.set("Direction", ParamValue::Enum("Left".into()));
        let vis: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(!vis.contains(&"XOffsetCurve") && vis.contains(&"YOffsetCurve") && vis.contains(&"CenterStop"));
        p.set("Direction", ParamValue::Enum("Rotate".into()));
        let vis: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(vis.contains(&"AngleCurve") && !vis.contains(&"Speed"));
    }
}
