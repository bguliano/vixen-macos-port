//! Port of VixenModules.Effect.Picture: shows a picture (a file, a GIF or an embedded tile) that can move, wiggle,
//! peek, resize or tile across the element.

use crate::generated::picture as gen;
use vixen_effects::gfx::{Bitmap, Graphics};
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    param_changed: Some(param_changed),
    targets_changed: Some(crate::orientation_targets_changed),
    param_changed_ctx: Some(crate::orientation_param_changed),
    supports_location: true,
    ported: true,
    ..gen::BASE
};

#[derive(Copy, Clone, PartialEq, Debug)]
enum Move {
    None,
    Tiles,
    Left,
    Right,
    Up,
    Down,
    UpLeft,
    DownLeft,
    UpRight,
    DownRight,
    Peekaboo0,
    Peekaboo90,
    Peekaboo180,
    Peekaboo270,
    Wiggle,
    Resize,
}

fn move_of(name: &str) -> Move {
    match name {
        "RenderPictureTiles" => Move::Tiles,
        "RenderPictureLeft" => Move::Left,
        "RenderPictureRight" => Move::Right,
        "RenderPictureUp" => Move::Up,
        "RenderPictureDown" => Move::Down,
        "RenderPictureUpleft" => Move::UpLeft,
        "RenderPictureDownleft" => Move::DownLeft,
        "RenderPictureUpright" => Move::UpRight,
        "RenderPictureDownright" => Move::DownRight,
        "RenderPicturePeekaboo0" => Move::Peekaboo0,
        "RenderPicturePeekaboo90" => Move::Peekaboo90,
        "RenderPicturePeekaboo180" => Move::Peekaboo180,
        "RenderPicturePeekaboo270" => Move::Peekaboo270,
        "RenderPictureWiggle" => Move::Wiggle,
        "RenderPictureResize" => Move::Resize,
        _ => Move::None,
    }
}

/// `UpdateAttributes`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let ty = move_of(p.enum_name("Type"));
    let stretch = p.boolean("StretchToGrid");
    let scale_to_grid = p.boolean("ScaleToGrid");
    v.set("Colors", p.enum_name("ColorEffect") == "CustomColor");
    v.set("Direction", ty == Move::Tiles);
    v.set("Speed", ty != Move::None);
    // `_gifSpeed` is set while rendering (true for a single image); before a render it is false.
    v.set("GifSpeed", true);
    v.set("ScalePercent", !scale_to_grid && !stretch);
    v.set("ScaleToGrid", !stretch);
    v.set("MovementRate", matches!(ty, Move::DownLeft | Move::DownRight | Move::UpLeft | Move::UpRight));
    v.set("FileName", p.enum_name("Source") == "File");
    v.set("TilePictures", p.enum_name("Source") != "File");
    v.set("CenterStop", matches!(ty, Move::Up | Move::Down | Move::Left | Move::Right));
    v.set("SizingCurve", ty == Move::Resize);
    v.set("TargetPositioning", true);
    v.set("StringOrientation", p.enum_name("TargetPositioning") == "Strings");
}

/// The `Orientation` setter also sets `StringOrientation`, and choosing the Checkers tile turns on custom colour.
fn param_changed(name: &str, p: &mut Params) {
    match name {
        "Orientation" => {
            let v = p.enum_name("Orientation").to_string();
            p.set("StringOrientation", ParamValue::Enum(v));
        }
        "StringOrientation" => {
            let v = p.enum_name("StringOrientation").to_string();
            p.set("Orientation", ParamValue::Enum(v));
        }
        "TilePictures" => {
            if p.enum_name("TilePictures") == "Checkers" {
                p.set("ColorEffect", ParamValue::Enum("CustomColor".to_string()));
            }
        }
        _ => {}
    }
}

fn tile_bytes(name: &str) -> &'static [u8] {
    match name {
        "Bubbles" => include_bytes!("../assets/picture_tiles/Bubbles.png"),
        "Checkers" => include_bytes!("../assets/picture_tiles/Checkers.png"),
        "Rain" => include_bytes!("../assets/picture_tiles/Rain.png"),
        "Damask" => include_bytes!("../assets/picture_tiles/Damask.png"),
        "VintageDamask" => include_bytes!("../assets/picture_tiles/VintageDamask.png"),
        "Diamonds" => include_bytes!("../assets/picture_tiles/Diamonds.png"),
        "Snowflakes1" => include_bytes!("../assets/picture_tiles/Snowflakes1.png"),
        "Snowflakes2" => include_bytes!("../assets/picture_tiles/Snowflakes2.png"),
        "Stripes1" => include_bytes!("../assets/picture_tiles/Stripes1.png"),
        "Stripes2" => include_bytes!("../assets/picture_tiles/Stripes2.png"),
        "TuttiFruitti" => include_bytes!("../assets/picture_tiles/TuttiFruitti.png"),
        // "None" is a legacy value that loads as the first tile.
        _ => include_bytes!("../assets/picture_tiles/BlueGlowDots.png"),
    }
}

/// Decodes an image file into its frames: every frame of an animated GIF, otherwise one.
pub(crate) fn decode_frames(bytes: &[u8]) -> Vec<Bitmap> {
    use image::AnimationDecoder;
    if bytes.starts_with(b"GIF8") {
        if let Ok(dec) = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(bytes)) {
            let frames: Vec<Bitmap> = dec
                .into_frames()
                .filter_map(|f| f.ok())
                .map(|f| {
                    let b = f.into_buffer();
                    Bitmap::from_rgba(b.width() as i32, b.height() as i32, b.as_raw())
                })
                .collect();
            if !frames.is_empty() {
                return frames;
            }
        }
    }
    Bitmap::from_bytes(bytes).map(|b| vec![b]).unwrap_or_default()
}

/// `Picture.ScalePictureImage`: fit inside `max_width` x `max_height`, keeping the aspect ratio.
pub(crate) fn scale_picture_image(image: &Bitmap, max_width: i32, max_height: i32) -> Bitmap {
    let ratio_x = max_width as f64 / image.width() as f64;
    let ratio_y = max_height as f64 / image.height() as f64;
    let ratio = ratio_x.min(ratio_y);
    let new_w = ((image.width() as f64 * ratio) as i32).max(1);
    let new_h = ((image.height() as f64 * ratio) as i32).max(1);
    image.resized(new_w, new_h)
}

/// `Picture.ScaleImage`.
pub(crate) fn scale_image(image: &Bitmap, scale: f64) -> Bitmap {
    let max_w = (image.width() as f64 * scale).round_ties_even() as i32;
    let max_h = (image.height() as f64 * scale).round_ties_even() as i32;
    let ratio_x = max_w as f64 / image.width() as f64;
    let ratio_y = max_h as f64 / image.height() as f64;
    let ratio = ratio_x.min(ratio_y);
    let new_w = ((image.width() as f64 * ratio) as i32).max(1);
    let new_h = ((image.height() as f64 * ratio) as i32).max(1);
    image.resized(new_w, new_h)
}

// Numerical candidate fitted to independently captured Windows ARGB bytes.
// This precision/order is empirical, not a claim about GDI+ implementation.
// The caller must supply committed straight RGB, not a premultiplied readback.
fn grayscale_channel(alpha: u8, red: u8, green: u8, blue: u8) -> u8 {
    let red_weight = (0.299f32 / 255.0f32) as f64;
    let blue_weight = (0.114f32 / 255.0f32) as f64;
    let red_blue = (red as f64 * red_weight + blue as f64 * blue_weight) as f32;
    let green = green as f32 * (0.587f32 / 255.0f32);
    let gray = (((red_blue + green) as f64) * 255.0).round().clamp(0.0, 255.0) as u32;
    let alpha = alpha as u32;
    let premultiplied = (gray * alpha + 127) / 255;
    if alpha == 0 {
        0
    } else {
        ((premultiplied * ((255u32 << 16) / alpha)) >> 16).min(255) as u8
    }
}

/// `Picture.ConvertToGrayScale`: the colour matrix gives every channel 0.299 R + 0.587 G + 0.114 B.
pub(crate) fn to_grayscale(image: &Bitmap) -> Bitmap {
    let mut out = Bitmap::new(image.width(), image.height());
    for y in 0..image.height() {
        for x in 0..image.width() {
            let c = image.get_pixel(x, y);
            // Commit the matrix drawing result as straight ARGB. Bitmap must
            // preserve the input's straight bytes and this readback byte exactly.
            let g = grayscale_channel(c.a, c.r, c.g, c.b);
            out.set_pixel(x, y, Color::argb(c.a, g, g, g));
        }
    }
    out
}

struct PictureClass {
    frame: i32,
    bitmap: Bitmap,
}

struct Picture {
    move_type: Move,
    custom_color: bool,
    grey_scale: bool,
    embedded: bool,
    tile: String,
    file_name: String,
    speed: i32,
    movement_rate: i32,
    gif_speed: i32,
    scale_to_grid: bool,
    stretch_to_grid: bool,
    scale_percent: i32,
    center_stop: bool,
    direction: i32,
    horizontal: bool,
    locations: bool,
    x_offset_curve: Curve,
    y_offset_curve: Curve,
    level_curve: Curve,
    sizing_curve: Curve,
    increase_brightness_curve: Curve,
    colors: ColorGradient,
    module_dir: Option<std::path::PathBuf>,
    // State
    pictures: Vec<PictureClass>,
    fp: Option<Bitmap>,
    movement_x: f64,
    movement_y: f64,
    position: f64,
    x_offset_adj: i32,
    y_offset_adj: i32,
    image_width: i32,
    image_height: i32,
    image_ht: i32,
    image_wi: i32,
    xoffset: i32,
    yoffset: i32,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut e = from_params(ctx, p);
    // The data member `Orientation` is what the C# `StringOrientation` returns.
    let orientation = StringOrientation::from_name(p.enum_name("Orientation"));
    let positioning = TargetPositioning::from_name(p.enum_name("TargetPositioning"));
    render_pixel(ctx, orientation, positioning, &mut e)
}

fn from_params(ctx: &RenderCtx, p: &Params) -> Picture {
    Picture {
        move_type: move_of(p.enum_name("Type")),
        custom_color: p.enum_name("ColorEffect") == "CustomColor",
        grey_scale: p.enum_name("ColorEffect") == "GreyScale",
        embedded: p.enum_name("Source") == "Embedded",
        tile: p.enum_name("TilePictures").to_string(),
        file_name: p.string("FileName").to_string(),
        speed: p.int("Speed"),
        movement_rate: p.int("MovementRate"),
        gif_speed: p.int("GifSpeed"),
        scale_to_grid: p.boolean("ScaleToGrid"),
        stretch_to_grid: p.boolean("StretchToGrid"),
        scale_percent: p.int("ScalePercent"),
        center_stop: p.boolean("CenterStop"),
        direction: p.int("Direction"),
        // The C# reads `Orientation` (the data member) here.
        horizontal: p.enum_name("Orientation") == "Horizontal",
        locations: TargetPositioning::from_name(p.enum_name("TargetPositioning")) == TargetPositioning::Locations,
        x_offset_curve: p.curve("XOffsetCurve"),
        y_offset_curve: p.curve("YOffsetCurve"),
        level_curve: p.curve("LevelCurve"),
        sizing_curve: p.curve("SizingCurve"),
        increase_brightness_curve: p.curve("IncreaseBrightnessCurve"),
        colors: p.gradient("Colors"),
        module_dir: ctx.module_data_root.map(|r| r.join("Picture")),
        pictures: Vec::new(),
        fp: None,
        movement_x: 0.0,
        movement_y: 0.0,
        position: 0.0,
        x_offset_adj: 0,
        y_offset_adj: 0,
        image_width: 0,
        image_height: 0,
        image_ht: 0,
        image_wi: 0,
        xoffset: 0,
        yoffset: 0,
    }
}

struct FrameData {
    level: f64,
    adjusted_brightness: f64,
}

impl Picture {
    fn calculate_x_offset(&self, pos: f64) -> i32 {
        scale_curve_to_value(self.x_offset_curve.get_value(pos), 100.0, -100.0).round_ties_even() as i32
    }

    fn calculate_y_offset(&self, pos: f64) -> i32 {
        scale_curve_to_value(self.y_offset_curve.get_value(pos), 100.0, -100.0).round_ties_even() as i32
    }

    fn calculate_sizing(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.sizing_curve.get_value(pos), 100.0, 0.0) / 100.0
    }

    fn init_frame_data(&self, frame: i32, info: &PixelInfo) -> FrameData {
        let pos = info.interval_position(frame) * 100.0;
        FrameData {
            level: self.level_curve.get_value(pos) / 100.0,
            adjusted_brightness: scale_curve_to_value(self.increase_brightness_curve.get_value(pos), 100.0, 10.0) / 10.0,
        }
    }

    /// `CustomColor`.
    fn custom_color_of(&self, frame: i32, info: &PixelInfo, level: f64, mut fp: Color, adjusted_brightness: f64) -> Color {
        if self.custom_color {
            let new_color = self.colors.get_color_at((info.interval_position(frame) * 100.0) / 100.0);
            let hsv_level = (fp.brightness() * 255.0).round_ties_even() as f64;
            let mut hsv = Hsv::from_color(new_color);
            hsv.set_v(hsv_level / 100.0);
            fp = hsv.to_color();
        }
        if level < 1.0 || adjusted_brightness > 1.0 {
            let mut hsv = Hsv::from_color(fp);
            let mut temp_v = hsv.v * level * adjusted_brightness;
            if temp_v > 1.0 {
                temp_v = 1.0;
            }
            hsv.set_v(temp_v);
            fp = hsv.to_color();
        }
        fp
    }

    fn calculate_position(&self, buffer_length: i32, image_length: i32) -> i32 {
        // The image travels the length of the buffer plus its own length, so it enters and leaves.
        let mut x_position = (self.position * (image_length + buffer_length) as f64) as i32;
        if self.center_stop {
            let center = buffer_length - (buffer_length - image_length) / 2;
            if x_position > center {
                x_position = center;
            }
        }
        x_position
    }

    fn initial_render(&mut self, frame: i32, info: &PixelInfo) {
        if self.fp.is_none() && self.pictures.is_empty() {
            return;
        }
        let (buffer_ht, buffer_wi) = (info.buffer_ht(), info.buffer_wi());
        let pos = info.interval_position(frame) * 100.0;
        let dir = 360 - self.direction;

        if self.move_type != Move::Tiles && self.move_type != Move::None {
            self.position = (info.interval_position(frame) * self.speed as f64) % 1.0;
        } else if frame == 0 {
            self.position = (info.interval_position(frame + 1) * self.speed as f64) % 1.0;
        }

        if !self.pictures.is_empty() {
            if self.move_type == Move::Resize && self.pictures.len() == 1 {
                self.fp = Some(scale_image(&self.pictures[0].bitmap, self.calculate_sizing(info.interval_position(frame) * 100.0)));
            } else {
                for i in 0..self.pictures.len() {
                    if frame >= self.pictures[i].frame {
                        let pic = self.pictures.remove(i);
                        if self.move_type == Move::Resize {
                            self.fp = Some(scale_image(&pic.bitmap, self.calculate_sizing(info.interval_position(frame) * 100.0)));
                        } else {
                            self.fp = Some(pic.bitmap);
                        }
                        break;
                    }
                }
            }
        }

        let Some(fp) = self.fp.as_ref() else { return };
        self.image_width = fp.width();
        self.image_height = fp.height();
        let (mut delta_x, mut delta_y) = (0.0f64, 0.0f64);
        let angle_offset: f64 = if dir > 45 && dir <= 90 {
            -((dir - 90) as f64)
        } else if dir > 90 && dir <= 135 {
            (dir - 90) as f64
        } else if dir > 135 && dir <= 180 {
            -((dir - 180) as f64)
        } else if dir > 180 && dir <= 225 {
            (dir - 180) as f64
        } else if dir > 225 && dir <= 270 {
            -((dir - 270) as f64)
        } else if dir > 270 && dir <= 315 {
            (dir - 270) as f64
        } else if dir > 315 && dir <= 360 {
            -((dir - 360) as f64)
        } else {
            dir as f64
        };

        let image_speed = self.position * (self.image_width as f64 / ((std::f64::consts::PI / 180.0) * angle_offset).cos());

        // Moving left and right.
        if dir > 0 && dir <= 90 {
            delta_x = (dir as f64 / 90.0) * image_speed;
        } else if dir > 90 && dir <= 180 {
            delta_x = ((dir - 180).abs() as f64 / 90.0) * image_speed;
        } else if dir > 180 && dir <= 270 {
            delta_x = -1.0 * (((dir - 180).abs() as f64 / 90.0) * image_speed);
        } else if dir > 270 && dir <= 360 {
            delta_x = -1.0 * (((dir - 360).abs() as f64 / 90.0) * image_speed);
        }
        // Moving up and down.
        if dir >= 0 && dir <= 90 {
            delta_y = ((dir - 90).abs() as f64 / 90.0) * image_speed;
        } else if dir > 90 && dir <= 180 {
            delta_y = -1.0 * (((dir - 90).abs() as f64 / 90.0) * image_speed);
        } else if dir > 180 && dir <= 270 {
            delta_y = -1.0 * (((dir - 270).abs() as f64 / 90.0) * image_speed);
        } else if dir > 270 && dir <= 360 {
            delta_y = ((270 - dir).abs() as f64 / 90.0) * image_speed;
        }

        self.movement_x += delta_x;
        self.movement_y += delta_y;

        // Subtract 1: zero based, with the y inverted.
        self.yoffset = ((buffer_ht + self.image_height) / 2) - 1;
        self.xoffset = (self.image_width - buffer_wi) / 2;
        self.x_offset_adj = self.calculate_x_offset(pos) * buffer_wi / 100;
        self.y_offset_adj = self.calculate_y_offset(pos) * buffer_ht / 100;
        self.image_ht = self.image_height;
        self.image_wi = self.image_width;

        match self.move_type {
            Move::Peekaboo0 | Move::Peekaboo180 => {
                self.yoffset = -buffer_ht + ((buffer_ht + 5) as f64 * self.position * 2.0) as i32;
                if self.yoffset > 10 {
                    self.yoffset = -self.yoffset + 10; // reverse direction
                } else if self.yoffset >= -1 {
                    self.yoffset = -1; // pause in the middle
                }
            }
            Move::Wiggle => {
                if self.position >= 0.5 {
                    self.xoffset += (self.image_wi as f64 * ((1.0 - self.position) * 2.0 - 0.5)) as i32;
                } else {
                    self.xoffset += (self.image_wi as f64 * (self.position * 2.0 - 0.5)) as i32;
                }
            }
            Move::Peekaboo90 | Move::Peekaboo270 => {
                if self.horizontal || self.locations {
                    self.yoffset = (buffer_ht - self.image_wi) / 2;
                    self.xoffset = -buffer_wi + ((buffer_wi + 5) as f64 * self.position * 2.0) as i32;
                } else {
                    self.yoffset = buffer_wi / 2;
                    self.xoffset = -buffer_ht + ((buffer_ht + 5) as f64 * self.position * 2.0) as i32;
                }
                if self.xoffset > 10 {
                    self.xoffset = -self.xoffset + 10;
                } else if self.xoffset >= -1 {
                    self.xoffset = -1;
                }
            }
            Move::Tiles => {
                self.image_ht = buffer_ht;
                self.image_wi = buffer_wi;
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn calculate_pixel<B: PixelBuffer>(&self, x_in: i32, y_in: i32, buf: &mut B, frame: i32, info: &PixelInfo, fd: &FrameData) {
        let Some(fp) = self.fp.as_ref() else { return };
        let (y_coord, x_coord) = (y_in, x_in);
        let (mut x, mut y) = (x_in, y_in);
        let (buffer_ht, buffer_wi) = (info.buffer_ht(), info.buffer_wi());
        let speed_factor = 4;
        let state = self.movement_rate.wrapping_mul(frame);
        let (location_y, location_x): (i32, i32);
        let mut fp_color = Color::EMPTY;
        let (xo, yo, xa, ya) = (self.xoffset, self.yoffset, self.x_offset_adj, self.y_offset_adj);
        let (iw, ih) = (self.image_wi, self.image_ht);

        if self.locations {
            // Flip and offset the coordinates so the location version acts like the string version.
            y = ((info.buffer_ht_offset() - y) + (buffer_ht - 1 + info.buffer_ht_offset())).abs();
            y -= info.buffer_ht_offset();
            x -= info.buffer_wi_offset();
        } else if self.move_type != Move::Tiles {
            fp_color = self.custom_color_of(frame, info, fd.level, fp.get_pixel(x, y), fd.adjusted_brightness);
        }
        // Image sizes are never 0 (scaling keeps at least 1), but guard the remainders anyway.
        let cycle = ((iw + buffer_wi) * speed_factor).max(1);

        macro_rules! set {
            ($px:expr, $py:expr) => {{
                buf.set_pixel($px, $py, fp_color);
                return;
            }};
        }

        match self.move_type {
            Move::Resize => {
                if self.locations {
                    location_y = yo - y + ya;
                    location_x = x + xo - xa;
                } else {
                    set!(x_coord - xo + xa, yo - y_coord + ya);
                }
            }
            Move::Peekaboo0 => {
                if self.locations {
                    location_y = buffer_ht + yo - y + ya;
                    location_x = x + xo - xa;
                } else {
                    set!(x_coord - xo + xa, buffer_ht + yo - y_coord + ya);
                }
            }
            Move::Wiggle => {
                if self.locations {
                    location_y = yo - y + ya;
                    location_x = x - xo - xa - buffer_wi + iw;
                } else {
                    set!(x_coord + xo + xa + buffer_wi - iw, yo - y_coord + ya);
                }
            }
            Move::Peekaboo90 => {
                if self.locations {
                    location_y = xo + buffer_wi - x - xa;
                    location_x = buffer_ht - y - ya - yo;
                } else {
                    set!(buffer_wi + xo - y_coord + xa, buffer_ht - x_coord - yo + ya);
                }
            }
            Move::Peekaboo180 => {
                if self.locations {
                    location_y = y - ya + buffer_ht - ih + yo;
                    location_x = (buffer_wi - x) + xo + xa;
                } else if self.horizontal {
                    set!((iw - x_coord) - xo + xa - buffer_wi + buffer_wi, y_coord - yo + ya);
                } else {
                    set!((buffer_wi - x_coord) - xo + xa - iw + buffer_wi, y_coord - yo + ya);
                }
            }
            Move::Peekaboo270 => {
                if self.locations {
                    location_y = xo + x - xa;
                    location_x = y - ya - yo;
                } else {
                    set!(y_coord - xo + xa, buffer_ht + yo - (buffer_ht - x_coord) + ya);
                }
            }
            Move::Left => {
                let movement_x = self.calculate_position(buffer_wi, iw);
                if self.locations {
                    location_y = yo - y + ya;
                    location_x = ((x - buffer_wi) + movement_x) + xa;
                } else {
                    let left_x = x_coord + (buffer_wi - movement_x);
                    set!(left_x + xa, yo - y + ya);
                }
            }
            Move::Right => {
                let movement_x = self.calculate_position(buffer_wi, iw);
                if self.locations {
                    location_y = yo - y + ya;
                    location_x = ((x + iw) - movement_x) + xa;
                } else {
                    let right_x = x_coord + -iw + movement_x;
                    set!(right_x + xa, yo - y_coord + ya);
                }
            }
            Move::Up => {
                let movement_y = self.calculate_position(buffer_ht, ih);
                if self.locations {
                    location_y = movement_y - y + ya;
                    location_x = x + xo - xa;
                } else {
                    set!(x_coord - xo + xa, movement_y - y + ya);
                }
            }
            Move::Down => {
                let movement_y = self.calculate_position(buffer_ht, ih);
                if self.locations {
                    location_y = ((buffer_ht + ih - 1) - movement_y - y) + ya;
                    location_x = x + xo - xa;
                } else {
                    let down_y = (buffer_ht + ih - 1) - movement_y - y_coord;
                    set!(x - xo + xa, down_y + ya);
                }
            }
            Move::UpLeft => {
                if self.locations {
                    location_y = ((((ih + buffer_ht) as f64) * self.position) as i32 - y) + ya;
                    location_x = (x - buffer_wi + (state % cycle) / speed_factor) + xa;
                } else {
                    let up_left_y = (((ih + buffer_ht) as f64) * self.position) as i32 - y_coord;
                    set!(x_coord + buffer_wi - (state % cycle) / speed_factor + xa, up_left_y + ya);
                }
            }
            Move::DownLeft => {
                if self.locations {
                    location_y = ((buffer_ht + ih - 1) - (((ih + buffer_ht) as f64) * self.position) as i32 - y) + ya;
                    location_x = (x - buffer_wi + (state % cycle) / speed_factor) + xa;
                } else {
                    let down_left_y = buffer_ht + ih - (((ih + buffer_ht) as f64) * self.position) as i32 - y_coord;
                    set!(x_coord + buffer_wi - (state % cycle) / speed_factor + xa, down_left_y + ya);
                }
            }
            Move::UpRight => {
                if self.locations {
                    location_y = ((((ih + buffer_ht) as f64) * self.position) as i32 - y) + ya;
                    location_x = (x + iw - (state % cycle) / speed_factor) + xa;
                } else {
                    let up_right_y = (((ih + buffer_ht) as f64) * self.position) as i32 - y_coord;
                    set!(x_coord + (state % cycle) / speed_factor - iw + xa, up_right_y + ya);
                }
            }
            Move::DownRight => {
                if self.locations {
                    location_y = ((buffer_ht + ih - 1) - (((ih + buffer_ht) as f64) * self.position) as i32 - y) + ya;
                    location_x = (x + iw - (state % cycle) / speed_factor) + xa;
                } else {
                    let down_right_y = buffer_ht + ih - (((ih + buffer_ht) as f64) * self.position) as i32 - y_coord;
                    set!(x_coord + (state % cycle) / speed_factor - iw + xa, down_right_y + ya);
                }
            }
            Move::None => {
                if self.locations {
                    location_y = yo - y + ya;
                    location_x = x + xo - xa;
                } else {
                    set!(x_coord - xo + xa, yo - y_coord + ya);
                }
            }
            Move::Tiles => {
                let mut lx = x + self.movement_x.round_ties_even() as i32 - (xa * buffer_wi / 100);
                let mut ly = (buffer_ht - y) + self.movement_y.round_ties_even() as i32 + (ya * buffer_ht / 100);
                let (w, h) = (self.image_width.max(1), self.image_height.max(1));
                if lx >= 0 {
                    lx %= w;
                } else {
                    lx = (lx % w) + w - 1;
                }
                if ly >= 0 {
                    ly %= h;
                } else {
                    ly = (ly % h) + h - 1;
                }
                location_x = lx;
                location_y = ly;
            }
        }

        // Only Location and Tiles reach here.
        if location_x < fp.width() && location_y < fp.height() && location_x >= 0 && location_y >= 0 {
            let c = self.custom_color_of(frame, info, fd.level, fp.get_pixel(location_x, location_y), fd.adjusted_brightness);
            buf.set_pixel(x_coord, y_coord, c);
        }
    }
}

impl PixelEffect for Picture {
    fn setup_render(&mut self, info: &PixelInfo) {
        self.pictures = Vec::with_capacity(32);
        self.fp = None;
        let mut frames: Vec<Bitmap> = Vec::new();
        if !self.file_name.is_empty() && !self.embedded {
            if let Some(dir) = &self.module_dir {
                if let Ok(bytes) = std::fs::read(dir.join(&self.file_name)) {
                    frames = decode_frames(&bytes);
                } else {
                    // A missing file leaves nothing to draw.
                    self.file_name = String::new();
                }
            }
        } else if self.embedded {
            if let Ok(b) = Bitmap::from_bytes(tile_bytes(&self.tile)) {
                frames.push(b);
            }
        }

        if !frames.is_empty() {
            let frame_count = frames.len() as i32;
            let (buffer_wi, buffer_ht) = (info.buffer_wi(), info.buffer_ht());
            let mut current_image = -1;
            for i in 0..info.number_frames() {
                let position = (info.interval_position(i) * self.gif_speed as f64) % 1.0;
                let current_gif_image_num = (position * (frame_count - 1) as f64).round_ties_even() as i32;
                if current_gif_image_num > current_image {
                    let image = &frames[(current_gif_image_num as usize).min(frames.len() - 1)];
                    let mut bitmap = if self.stretch_to_grid {
                        let mut g = Graphics::new(buffer_wi, buffer_ht);
                        g.draw_image(image, 0.0, 0.0, buffer_wi as f32, buffer_ht as f32);
                        g.into_bitmap()
                    } else if self.scale_to_grid {
                        scale_picture_image(image, buffer_wi, buffer_ht)
                    } else {
                        scale_image(image, self.scale_percent as f64 / 100.0)
                    };
                    if self.grey_scale {
                        bitmap = to_grayscale(&bitmap);
                    }
                    self.pictures.push(PictureClass { frame: i, bitmap });
                }
                current_image = current_gif_image_num;
                if frame_count == 1 {
                    break;
                }
            }
        }
        self.movement_x = 0.0;
        self.movement_y = 0.0;
    }

    fn cleanup_render(&mut self) {
        self.fp = None;
        self.pictures = Vec::new();
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let fd = self.init_frame_data(frame, info);
        self.initial_render(frame, info);
        if self.fp.is_some() {
            for x in 0..self.image_wi {
                for y in 0..self.image_ht {
                    self.calculate_pixel(x, y, buf, frame, info, &fd);
                }
            }
        }
    }

    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        let locations = buf.element_locations().to_vec();
        for frame in 0..num_frames {
            buf.current_frame = frame;
            let fd = self.init_frame_data(frame, info);
            self.initial_render(frame, info);
            if self.fp.is_some() {
                for l in &locations {
                    self.calculate_pixel(l.x, l.y, buf, frame, info, &fd);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use vixen_effects::testing;

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        testing::assert_produces_light(&DESCRIPTOR);
    }

    #[test]
    fn every_movement_renders() {
        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 16, 20);
        let located = testing::grid(&mut tree, "L", 4, 6);
        testing::add_locations(&mut tree, located);
        for ty in gen::ENUM_EffectType.members {
            let out = testing::render_with(&DESCRIPTOR, &tree, &[m], 2000, |p| {
                p.set("Type", ParamValue::Enum(ty.name.into()));
                p.set("TilePictures", ParamValue::Enum("Stripes1".into()));
            });
            let mut any = false;
            for ms in (0..2000).step_by(100) {
                any |= lit_count(&tree, m, &out, ms) > 0;
            }
            assert!(any, "{} drew nothing", ty.name);
            testing::render_with(&DESCRIPTOR, &tree, &[located], 1000, |p| {
                p.set("Type", ParamValue::Enum(ty.name.into()));
                p.set("TargetPositioning", ParamValue::Enum("Locations".into()));
            });
        }
    }

    #[test]
    fn missing_file_renders_nothing() {
        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 8, 8);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[m], 500, |p| {
            p.set("Source", ParamValue::Enum("File".into()));
            p.set("FileName", ParamValue::Str("nope.png".into()));
        });
        assert_eq!(lit_count(&tree, m, &out, 0), 0);
    }

    #[test]
    fn file_picture_is_centred_and_scaled_to_grid() {
        // A solid red 4x4 PNG scaled to a 10x10 grid with no movement fills the grid.
        let dir = std::env::temp_dir().join(format!("pic_test_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("Picture")).unwrap();
        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 0, 255]));
        img.save(dir.join("Picture").join("red.png")).unwrap();
        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 10, 10);
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let ctx = RenderCtx { video_cache: None,
            tree: &tree,
            targets: &[m],
            start_time: TimeSpan::ZERO,
            time_span: TimeSpan::from_ms_i(200),
            frame_ms: 50,
            marks: &[],
            audio: None,
            module_data_root: Some(&dir),
            cancel: &cancel,
        };
        let mut p = DESCRIPTOR.new_params();
        p.set("Source", ParamValue::Enum("File".into()));
        p.set("FileName", ParamValue::Str("red.png".into()));
        let out = (DESCRIPTOR.render)(&ctx, &p);
        assert_eq!(color_at(&tree, m, &out, 0, 0, 0), (255, 0, 0));
        assert_eq!(color_at(&tree, m, &out, 9, 9, 0), (255, 0, 0));
        assert_eq!(lit_count(&tree, m, &out, 0), 100);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn gif_frames_are_decoded() {
        use image::codecs::gif::GifEncoder;
        let mut buf = Vec::new();
        {
            let mut enc = GifEncoder::new(&mut buf);
            for c in [[255, 0, 0, 255], [0, 255, 0, 255]] {
                let f = image::Frame::new(image::RgbaImage::from_pixel(3, 3, image::Rgba(c)));
                enc.encode_frame(f).unwrap();
            }
        }
        let frames = decode_frames(&buf);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].get_pixel(1, 1).r, 255);
        assert_eq!(frames[1].get_pixel(1, 1).g, 255);
    }

    #[test]
    fn checkers_turns_on_custom_colour() {
        let mut p = DESCRIPTOR.new_params();
        p.set("TilePictures", ParamValue::Enum("Checkers".into()));
        param_changed("TilePictures", &mut p);
        assert_eq!(p.enum_name("ColorEffect"), "CustomColor");
    }

    #[test]
    fn grayscale_uses_luma_weights() {
        let mut b = Bitmap::new(1, 1);
        b.set_pixel(0, 0, Color::rgb(255, 0, 0));
        let g = to_grayscale(&b).get_pixel(0, 0);
        assert_eq!((g.r, g.g, g.b), (76, 76, 76));
    }

    #[test]
    fn grayscale_matches_captured_windows_opaque_pixels() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/golden/picture-grayscale-windows.json"
        )).unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let input = case["inputArgb"].as_array().unwrap();
            let expected = case["outputArgb"].as_array().unwrap();
            let mut bitmap = Bitmap::new((input.len() / 4) as i32, 1);
            for (x, pixel) in input.chunks_exact(4).enumerate() {
                let c: Vec<u8> = pixel.iter().map(|v| v.as_u64().unwrap() as u8).collect();
                bitmap.set_pixel(x as i32, 0, Color::argb(c[0], c[1], c[2], c[3]));
            }
            let result = to_grayscale(&bitmap);
            for (x, pixel) in expected.chunks_exact(4).enumerate() {
                let c = result.get_pixel(x as i32, 0);
                let actual = [c.a, c.r, c.g, c.b];
                let expected: Vec<u8> = pixel.iter().map(|v| v.as_u64().unwrap() as u8).collect();
                assert_eq!(actual.as_slice(), expected, "{} pixel {x}", case["id"]);
            }
        }
    }
    #[test]
    fn grayscale_matches_captured_windows_alpha_and_half_byte_samples() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/golden/picture-grayscale-alpha-windows.json"
        )).unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let width = case["width"].as_i64().unwrap() as i32;
            let height = case["height"].as_i64().unwrap() as i32;
            let input: Vec<u8> = case["inputArgb"].as_array().unwrap().iter()
                .map(|v| v.as_u64().unwrap() as u8).collect();
            let expected: Vec<u8> = case["outputArgb"].as_array().unwrap().iter()
                .map(|v| v.as_u64().unwrap() as u8).collect();
            assert_eq!(input.len(), (width * height * 4) as usize);
            assert_eq!(expected.len(), input.len());
            let mut bitmap = Bitmap::new(width, height);
            for (i, p) in input.chunks_exact(4).enumerate() {
                bitmap.set_pixel(i as i32 % width, i as i32 / width,
                    Color::argb(p[0], p[1], p[2], p[3]));
            }
            let result = to_grayscale(&bitmap);
            for (i, (src, expected)) in input.chunks_exact(4)
                .zip(expected.chunks_exact(4)).enumerate() {
                let x = i as i32 % width;
                let y = i as i32 / width;
                let c = result.get_pixel(x, y);
                assert_eq!([c.a, c.r, c.g, c.b].as_slice(), expected,
                    "{} pixel {i}", case["id"]);
                let retained = bitmap.get_pixel(x, y);
                assert_eq!([retained.a, retained.r, retained.g, retained.b].as_slice(), src,
                    "{} source changed at pixel {i}", case["id"]);
            }
        }
    }

}

#[cfg(test)]
#[path = "picture_golden.rs"]
mod golden;
