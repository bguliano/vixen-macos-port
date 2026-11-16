//! Port of VixenModules.Effect.Video. The C# shells out to ffmpeg to turn the video into scaled frame pictures on disk; here
//! `vixen-video` streams the same stretch into a persistent frame cache, and everything after that (the crop and
//! rotate filters, sizing, positioning, playback speed and frame stepping) is done here.

use crate::generated::video as gen;
use crate::picture::to_grayscale;
use std::path::Path;
use std::sync::Arc;
use vixen_effects::gfx::{Bitmap, Graphics};
use vixen_effects::prelude::*;
use vixen_video::{video_frames_cancelable, video_info_cancelable, VideoRequest};

const SPEED_FACTOR: i32 = 4;
const MAX_RENDER_WIDTH: i32 = 800;
const MAX_RENDER_HEIGHT: i32 = 600;

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
}

fn move_of(name: &str) -> Move {
    match name {
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
        _ => Move::None,
    }
}

/// `UpdateAttributes`, `UpdateQualityAttribute`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let ty = move_of(p.enum_name("Type"));
    v.set("ScalePercent", !p.boolean("ScaleToGrid") && !p.boolean("StretchToGrid"));
    v.set("ScaleToGrid", !p.boolean("StretchToGrid"));
    v.set("MovementRate", matches!(ty, Move::DownLeft | Move::DownRight | Move::UpLeft | Move::UpRight));
    v.set("Speed", ty != Move::None);
    v.set("VideoQuality", p.enum_name("TargetPositioning") == "Locations");
    v.set("TargetPositioning", true);
    v.set("StringOrientation", p.enum_name("TargetPositioning") == "Strings");
}

/// `Orientation` and `StringOrientation` are the same data member.
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
        _ => {}
    }
}

enum Frames { Memory(Arc<Vec<Arc<vixen_video::Frame>>>), Disk(Arc<vixen_video::cache::CachedFrames>) }
impl Default for Frames { fn default() -> Self { Self::Memory(Arc::default()) } }
impl Frames {
    fn len(&self) -> usize { match self { Self::Memory(frames) => frames.len(), Self::Disk(frames) => frames.len() } }
    fn is_empty(&self) -> bool { self.len() == 0 }
    fn frame(&self, index: usize) -> Option<Arc<vixen_video::Frame>> { match self { Self::Memory(frames) => frames.get(index).cloned(), Self::Disk(frames) => frames.frame(index) } }
}
fn load_frames(path: &Path, request: &VideoRequest, cache: Option<VideoCacheContext<'_>>, variant: &str, cancel: &std::sync::atomic::AtomicBool) -> Option<Frames> {
    match cache {
        Some(context) => Some(Frames::Disk(context.session.frames(context.instance, path, request, variant, cancel)?)),
        None => Some(Frames::Memory(Arc::new(video_frames_cancelable(path, request, cancel)?))),
    }
}

/// `scale=W:-1`.
fn keep_aspect_height(width: i32, (video_width, video_height): (i32, i32)) -> i32 {
    ((width as f64 * video_height as f64 / video_width.max(1) as f64).round_ties_even() as i32).max(1)
}

struct Video<'a> {
    cache: Option<VideoCacheContext<'a>>,
    cancel: &'a std::sync::atomic::AtomicBool,
    move_type: Move,
    grey_scale: bool,
    file_name: String,
    speed: i32,
    movement_rate: i32,
    play_back_speed: i32,
    start_time_seconds: f64,
    rotate_video: i32,
    scale_to_grid: bool,
    stretch_to_grid: bool,
    maintain_aspect: bool,
    scale_percent: i32,
    video_quality: i32,
    horizontal: bool,
    locations: bool,
    x_offset_curve: Curve,
    y_offset_curve: Curve,
    level_curve: Curve,
    increase_brightness_curve: Curve,
    module_dir: Option<std::path::PathBuf>,
    // State
    video_file_detected: bool,
    frames: Frames,
    render_width: i32,
    render_height: i32,
    ratio_width: f64,
    ratio_height: f64,
    crop: Option<(i32, i32, i32, i32)>,
    current_movie_image_num: f64,
    fp: Option<Bitmap>,
    last_index: Option<usize>,
    position: f64,
    x_offset_adj: i32,
    y_offset_adj: i32,
    image_ht: i32,
    image_wi: i32,
    xoffset: i32,
    yoffset: i32,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut e = Video {
        cache: ctx.video_cache,
        cancel: ctx.cancel,
        move_type: move_of(p.enum_name("Type")),
        grey_scale: p.enum_name("EffectColorType") == "RenderGreyScale",
        file_name: p.string("FileName").to_string(),
        speed: p.int("Speed"),
        movement_rate: p.int("MovementRate"),
        play_back_speed: p.int("PlayBackSpeed"),
        start_time_seconds: p.double("StartTimeSeconds"),
        rotate_video: p.int("RotateVideo"),
        scale_to_grid: p.boolean("ScaleToGrid"),
        stretch_to_grid: p.boolean("StretchToGrid"),
        maintain_aspect: p.boolean("MaintainAspect"),
        scale_percent: p.int("ScalePercent"),
        video_quality: p.int("VideoQuality"),
        horizontal: p.enum_name("Orientation") == "Horizontal",
        locations: TargetPositioning::from_name(p.enum_name("TargetPositioning")) == TargetPositioning::Locations,
        x_offset_curve: p.curve("XOffsetCurve"),
        y_offset_curve: p.curve("YOffsetCurve"),
        level_curve: p.curve("LevelCurve"),
        increase_brightness_curve: p.curve("IncreaseBrightnessCurve"),
        module_dir: ctx.module_data_root.map(|r| r.join("Video")),
        video_file_detected: false,
        frames: Frames::default(),
        render_width: 0,
        render_height: 0,
        ratio_width: 1.0,
        ratio_height: 1.0,
        crop: None,
        current_movie_image_num: 0.0,
        fp: None,
        last_index: None,
        position: 0.0,
        x_offset_adj: 0,
        y_offset_adj: 0,
        image_ht: 0,
        image_wi: 0,
        xoffset: 0,
        yoffset: 0,
    };
    // The data member `Orientation` is what the C# `StringOrientation` returns.
    let orientation = StringOrientation::from_name(p.enum_name("Orientation"));
    let positioning = TargetPositioning::from_name(p.enum_name("TargetPositioning"));
    render_pixel(ctx, orientation, positioning, &mut e)
}

struct FrameData {
    level: f64,
    adjusted_brightness: f64,
}

impl Video<'_> {
    fn calculate_x_offset(&self, pos: f64) -> i32 {
        scale_curve_to_value(self.x_offset_curve.get_value(pos), 100.0, -100.0).round_ties_even() as i32
    }

    fn calculate_y_offset(&self, pos: f64) -> i32 {
        scale_curve_to_value(self.y_offset_curve.get_value(pos), 100.0, -100.0).round_ties_even() as i32
    }

    /// `GetNewImageSize`, using the video's own size.
    fn new_image_size(&self, video: (i32, i32), max_w: i32, max_h: i32) -> (i32, i32) {
        let ratio_x = max_w as f64 / video.0 as f64;
        let ratio_y = max_h as f64 / video.1 as f64;
        let ratio = if max_h > max_w && !self.maintain_aspect { ratio_x.max(ratio_y) } else { ratio_x.min(ratio_y) };
        let mut w = (video.0 as f64 * ratio) as i32;
        let mut h = (video.1 as f64 * ratio) as i32;
        if h <= 0 {
            h = 1;
        }
        if w <= 0 {
            w = 1;
        }
        (w, h)
    }

    /// `ProcessMovie`, with decoded frames standing in for the ffmpeg thumbnails.
    fn process_movie(&mut self, path: &Path, info: &PixelInfo) {
        self.video_file_detected = false;
        let (buffer_wi, buffer_ht) = (info.buffer_wi(), info.buffer_ht());
        let fps = 1000.0 / info.frame_time as f64;
        let Some(video) = video_info_cancelable(path, self.cancel) else { return };
        if video.width <= 0 || video.height <= 0 {
            return;
        }
        let video_size = (video.width, video.height);
        let video_length = video.duration_seconds;

        let (mut rw, mut rh);
        if self.stretch_to_grid {
            rw = buffer_wi;
            rh = buffer_ht;
        } else {
            (rw, rh) = self.new_image_size(video_size, buffer_wi, buffer_ht);
            if !self.scale_to_grid {
                // Scale and crop the image by the user's scale setting.
                rw = (rw as f64 * (self.scale_percent as f64 / 100.0 + 1.0)) as i32;
                rh = (rh as f64 * (self.scale_percent as f64 / 100.0 + 1.0)) as i32;
            }
        }

        let video_quality = if self.locations { self.video_quality as f64 / 100.0 } else { 1.0 };
        if rw > MAX_RENDER_WIDTH || rh > MAX_RENDER_HEIGHT {
            self.ratio_width = rw as f64 / MAX_RENDER_WIDTH as f64 / video_quality;
            self.ratio_height = rh as f64 / MAX_RENDER_HEIGHT as f64 / video_quality;
            rh = (MAX_RENDER_HEIGHT as f64 * video_quality) as i32;
            rw = (MAX_RENDER_WIDTH as f64 * video_quality) as i32;
        } else {
            self.ratio_width = 1.0;
            self.ratio_height = 1.0;
        }

        self.crop = None;
        if !self.stretch_to_grid && !self.scale_to_grid {
            let cw = rw.min(buffer_wi);
            let ch = rh.min(buffer_ht);
            self.crop = Some((cw, ch, (rw - cw) / 2, (rh - ch) / 2));
        }

        // Elements below 10 pixels are rescaled, as ffmpeg could refuse to scale.
        if rh < 10 || rw < 10 {
            if rh <= 2 || rw <= 2 {
                return;
            }
            (rw, rh) = self.new_image_size(video_size, 50, (50.0 * (rw as f64 / rh as f64)) as i32);
        }

        let play = self.play_back_speed as f64 / 100.0 + 1.0;
        let duration = info.time_span.total_secs() * play;
        // The video must be longer than the start time plus the effect (at its playback speed).
        if video_length > self.start_time_seconds + duration {
            // Height and width must be even for ffmpeg.
            if rh % 2 != 0 {
                rh += 1;
            }
            if rw % 2 != 0 {
                rw += 1;
            }
            self.render_width = rw;
            self.render_height = rh;
            // `-ss start -t duration -r fps -vf scale=W:(maintainAspect ? -1 : H)`.
            let height = if self.maintain_aspect { keep_aspect_height(rw, video_size) } else { rh };
            let request = VideoRequest { start_seconds: self.start_time_seconds, duration_seconds: duration, fps, width: rw, height };
            let variant = format!("{}|{:?}|{}|{}|{}|{}", self.rotate_video, self.crop, self.maintain_aspect, self.scale_to_grid, self.stretch_to_grid, self.play_back_speed);
            let Some(frames) = load_frames(path, &request, self.cache, &variant, self.cancel) else { return };
            self.frames = frames;
            self.video_file_detected = !self.frames.is_empty();
            self.current_movie_image_num = 0.0;
        }
    }

    /// One ffmpeg thumbnail's `crop=..., rotate=R*(PI/180)`. Its `scale` has been done by the decoder.
    fn make_thumbnail(&self, src: &Bitmap) -> Bitmap {
        let mut img = src.clone();
        if let Some((cw, ch, cx, cy)) = self.crop {
            let (cw, ch) = (cw.min(img.width()).max(1), ch.min(img.height()).max(1));
            let mut cropped = Bitmap::new(cw, ch);
            for y in 0..ch {
                for x in 0..cw {
                    cropped.set_pixel(x, y, img.get_pixel(cx + x, cy + y));
                }
            }
            img = cropped;
        }
        if self.rotate_video != 0 {
            // The rotate filter keeps the frame size and fills the corners with black.
            let mut g = Graphics::new(img.width(), img.height());
            g.clear(Color::BLACK);
            g.translate(img.width() as f32 / 2.0, img.height() as f32 / 2.0);
            g.rotate(self.rotate_video as f32);
            g.translate(-(img.width() as f32) / 2.0, -(img.height() as f32) / 2.0);
            g.draw_image(&img, 0.0, 0.0, img.width() as f32, img.height() as f32);
            img = g.into_bitmap();
        }
        img
    }

    fn init_frame_data(&self, frame: i32, info: &PixelInfo) -> (f64, f64, FrameData) {
        let interval_pos = info.interval_position(frame);
        let pos = interval_pos * 100.0;
        let level = self.level_curve.get_value(pos) / 100.0;
        let adjusted = scale_curve_to_value(self.increase_brightness_curve.get_value(pos), 100.0, 10.0) / 10.0;
        (interval_pos, pos, FrameData { level, adjusted_brightness: adjusted })
    }

    fn initial_render(&mut self, interval_pos: f64, pos: f64, info: &PixelInfo) {
        if self.frames.is_empty() {
            return;
        }
        let (buffer_ht, buffer_wi) = (info.buffer_ht(), info.buffer_wi());
        self.position = (interval_pos * self.speed as f64) % 1.0;

        let picture_count = self.frames.len();
        let mut current = self.current_movie_image_num as i32;
        if current >= picture_count as i32 {
            current = picture_count as i32 - 1;
            self.current_movie_image_num = current as f64;
        } else if current < 0 {
            current = 0;
            self.current_movie_image_num = 0.0;
        }
        let index = current as usize;
        if self.last_index != Some(index) || self.fp.is_none() {
            let Some(frame) = self.frames.frame(index) else { self.fp = None; return };
            let bitmap = Bitmap::from_rgba(frame.width, frame.height, &frame.rgba);
            let thumb = self.make_thumbnail(&bitmap);
            let (tw, th) = (((self.render_width as f64 * self.ratio_width) as i32).max(1), ((self.render_height as f64 * self.ratio_height) as i32).max(1));
            // `new Bitmap(image, w, h)` rescales back up when the render size was capped.
            let scaled = if thumb.width() != tw || thumb.height() != th { thumb.resized(tw, th) } else { thumb };
            self.fp = Some(if self.grey_scale { to_grayscale(&scaled) } else { scaled });
            self.last_index = Some(index);
        }

        if self.play_back_speed > 0 {
            self.current_movie_image_num += (self.play_back_speed as f64 * 0.01) + 1.0;
        } else if self.play_back_speed < 0 {
            self.current_movie_image_num += (100 + self.play_back_speed) as f64 * 0.01;
        } else {
            self.current_movie_image_num += 1.0;
        }

        let fp = self.fp.as_ref().unwrap();
        self.image_wi = fp.width();
        self.image_ht = fp.height();
        self.yoffset = (buffer_ht + self.image_ht) / 2;
        self.xoffset = (self.image_wi - buffer_wi) / 2;
        self.x_offset_adj = self.calculate_x_offset(pos) * buffer_wi / 100;
        self.y_offset_adj = self.calculate_y_offset(pos) * buffer_ht / 100;

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
            _ => {}
        }
    }

    /// `GetIntensity`.
    fn get_intensity(level: f64, fp: Color, adjusted_brightness: f64) -> Color {
        if level < 1.0 || adjusted_brightness > 1.0 {
            let mut hsv = Hsv::from_color(fp);
            let mut temp_v = hsv.v * level * adjusted_brightness;
            if temp_v > 1.0 {
                temp_v = 1.0;
            }
            hsv.set_v(temp_v);
            return hsv.to_color();
        }
        fp
    }

    #[allow(clippy::too_many_arguments)]
    fn calculate_pixel<B: PixelBuffer>(&self, x_in: i32, y_in: i32, buf: &mut B, frame: i32, info: &PixelInfo, fd: &FrameData) {
        let Some(fp) = self.fp.as_ref() else { return };
        let (y_coord, x_coord) = (y_in, x_in);
        let (mut x, mut y) = (x_in, y_in);
        let (buffer_ht, buffer_wi) = (info.buffer_ht(), info.buffer_wi());
        let (location_y, location_x): (i32, i32);
        let mut fp_color = Color::EMPTY;
        let (xo, yo, xa, ya) = (self.xoffset, self.yoffset, self.x_offset_adj, self.y_offset_adj);
        let (iw, ih) = (self.image_wi, self.image_ht);

        if self.locations {
            // Flip and offset the coordinates so the location version acts like the string version.
            y = ((info.buffer_ht_offset() - y) + (buffer_ht - 1 + info.buffer_ht_offset())).abs();
            y -= info.buffer_ht_offset();
            x -= info.buffer_wi_offset();
        } else {
            fp_color = Self::get_intensity(fd.level, fp.get_pixel(x, y), fd.adjusted_brightness);
        }
        let pos = self.position;
        let cycle = ((iw + buffer_wi) * SPEED_FACTOR).max(1);
        let state = self.movement_rate.wrapping_mul(frame) % cycle;

        macro_rules! set {
            ($px:expr, $py:expr) => {{
                buf.set_pixel($px, $py, fp_color);
                return;
            }};
        }

        match self.move_type {
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
                let moved = (pos * (iw + buffer_wi) as f64) as i32;
                if self.locations {
                    location_y = yo - y + ya;
                    location_x = ((x - buffer_wi) + moved) + xa;
                } else {
                    set!(x_coord + (buffer_wi - moved) + xa, yo - y + ya);
                }
            }
            Move::Right => {
                let moved = (pos * (iw + buffer_wi) as f64) as i32;
                if self.locations {
                    location_y = yo - y + ya;
                    location_x = ((x + iw) - moved) + xa;
                } else {
                    set!(x_coord + -iw + moved + xa, yo - y_coord + ya);
                }
            }
            Move::Up => {
                let moved = ((ih + buffer_ht) as f64 * pos) as i32;
                if self.locations {
                    location_y = (moved - y) + ya;
                    location_x = x + xo - xa;
                } else {
                    set!(x_coord - xo + xa, (moved - y) + ya);
                }
            }
            Move::Down => {
                let moved = ((ih + buffer_ht) as f64 * pos) as i32;
                if self.locations {
                    location_y = ((buffer_ht + ih - 1) - moved - y) + ya;
                    location_x = x + xo - xa;
                } else {
                    set!(x - xo + xa, ((buffer_ht + ih - 1) - moved - y_coord) + ya);
                }
            }
            Move::UpLeft => {
                let moved = ((ih + buffer_ht) as f64 * pos) as i32;
                if self.locations {
                    location_y = (moved - y) + ya;
                    location_x = (x - buffer_wi + state / SPEED_FACTOR) + xa;
                } else {
                    set!(x_coord + buffer_wi - state / SPEED_FACTOR + xa, (moved - y_coord) + ya);
                }
            }
            Move::DownLeft => {
                let moved = ((ih + buffer_ht) as f64 * pos) as i32;
                if self.locations {
                    location_y = ((buffer_ht + ih - 1) - moved - y) + ya;
                    location_x = (x - buffer_wi + state / SPEED_FACTOR) + xa;
                } else {
                    set!(x_coord + buffer_wi - state / SPEED_FACTOR + xa, (buffer_ht + ih - moved - y_coord) + ya);
                }
            }
            Move::UpRight => {
                let moved = ((ih + buffer_ht) as f64 * pos) as i32;
                if self.locations {
                    location_y = (moved - y) + ya;
                    location_x = (x + iw - state / SPEED_FACTOR) + xa;
                } else {
                    set!(x_coord + state / SPEED_FACTOR - iw + xa, (moved - y_coord) + ya);
                }
            }
            Move::DownRight => {
                let moved = ((ih + buffer_ht) as f64 * pos) as i32;
                if self.locations {
                    location_y = ((buffer_ht + ih - 1) - moved - y) + ya;
                    location_x = (x + iw - state / SPEED_FACTOR) + xa;
                } else {
                    set!(x_coord + state / SPEED_FACTOR - iw + xa, (buffer_ht + ih - moved - y_coord) + ya);
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
        }

        // Only locations reach here.
        if location_x < fp.width() && location_y < fp.height() && location_x >= 0 && location_y >= 0 {
            let c = Self::get_intensity(fd.level, fp.get_pixel(location_x, location_y), fd.adjusted_brightness);
            buf.set_pixel(x_coord, y_coord, c);
        }
    }
}

impl PixelEffect for Video<'_> {
    fn setup_render(&mut self, info: &PixelInfo) {
        self.video_file_detected = false;
        self.frames = Frames::default();
        self.fp = None;
        self.last_index = None;
        if self.file_name.is_empty() {
            return;
        }
        // A missing file, or no registered extractor, leaves the effect dark.
        let Some(dir) = &self.module_dir else { return };
        let path = dir.join(&self.file_name);
        if !path.exists() {
            return;
        }
        self.process_movie(&path, info);
        if self.video_file_detected {
            self.current_movie_image_num = 0.0;
        }
    }

    fn cleanup_render(&mut self) {
        self.fp = None;
        self.frames = Frames::default();
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        if !self.video_file_detected {
            return;
        }
        let (interval_pos, pos, fd) = self.init_frame_data(frame, info);
        self.initial_render(interval_pos, pos, info);
        if self.fp.is_some() {
            for x in 0..self.image_wi {
                for y in 0..self.image_ht {
                    self.calculate_pixel(x, y, buf, frame, info, &fd);
                }
            }
        }
    }

    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        if !self.video_file_detected {
            return;
        }
        let locations = buf.element_locations().to_vec();
        for frame in 0..num_frames {
            buf.current_frame = frame;
            let (interval_pos, pos, fd) = self.init_frame_data(frame, info);
            self.initial_render(interval_pos, pos, info);
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

    /// A real 3 second clip, 160x90: one solid colour per second (red, green, blue).
    fn with_file(name: &str) -> (std::path::PathBuf, ()) {
        let clip = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../vixen-video/tests/assets/colors.mov");
        let dir = std::env::temp_dir().join(format!("video_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("Video")).unwrap();
        std::fs::copy(clip, dir.join("Video").join(name)).unwrap();
        (dir, ())
    }

    /// Video compression leaves solid colours slightly off.
    fn assert_near(actual: (u8, u8, u8), expected: (u8, u8, u8)) {
        let close = actual.0.abs_diff(expected.0) < 40 && actual.1.abs_diff(expected.1) < 40 && actual.2.abs_diff(expected.2) < 40;
        assert!(close, "{actual:?} isn't near {expected:?}");
    }

    fn ctx_for<'a>(tree: &'a ElementTree, targets: &'a [NodeId], dir: &'a std::path::Path, cancel: &'a std::sync::atomic::AtomicBool, ms: i64) -> RenderCtx<'a> {
        RenderCtx { video_cache: None,
            tree,
            targets,
            start_time: TimeSpan::ZERO,
            time_span: TimeSpan::from_ms_i(ms),
            frame_ms: 50,
            marks: &[],
            audio: None,
            module_data_root: Some(dir),
            cancel,
        }
    }

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
    }

    #[test]
    fn plays_the_frames_in_order() {
        let (dir, _) = with_file("clip.mp4");
        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 16, 8);
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let targets = [m];
        let ctx = ctx_for(&tree, &targets, &dir, &cancel, 2000);
        let mut p = DESCRIPTOR.new_params();
        p.set("FileName", ParamValue::Str("clip.mp4".into()));
        let out = (DESCRIPTOR.render)(&ctx, &p);
        // Scaled to the grid: the picture covers the element, red then green over the 2 s effect.
        assert_near(color_at(&tree, m, &out, 8, 4, 0), (255, 0, 0));
        assert_near(color_at(&tree, m, &out, 8, 4, 1500), (0, 255, 0));
        assert!(lit_count(&tree, m, &out, 0) >= 100);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn start_time_skips_into_the_video() {
        let (dir, _) = with_file("clip2.mp4");
        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 16, 8);
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let targets = [m];
        let ctx = ctx_for(&tree, &targets, &dir, &cancel, 1000);
        let mut p = DESCRIPTOR.new_params();
        p.set("FileName", ParamValue::Str("clip2.mp4".into()));
        p.set("StartTimeSeconds", ParamValue::Double(1.0));
        let out = (DESCRIPTOR.render)(&ctx, &p);
        // The video is 3 s; starting at 1 s with a 1 s effect shows green first.
        assert_near(color_at(&tree, m, &out, 8, 4, 0), (0, 255, 0));
        // Starting too late (video not long enough) renders nothing.
        p.set("StartTimeSeconds", ParamValue::Double(2.5));
        let out = (DESCRIPTOR.render)(&ctx, &p);
        assert_eq!(lit_count(&tree, m, &out, 0), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_and_no_name_render_nothing() {
        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 8, 8);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[m], 500, |p| {
            p.set("FileName", ParamValue::Str("nope.mp4".into()));
        });
        assert_eq!(lit_count(&tree, m, &out, 0), 0);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[m], 500, |_| {});
        assert_eq!(lit_count(&tree, m, &out, 0), 0);
    }

    #[test]
    fn every_movement_renders_with_a_file() {
        let (dir, _) = with_file("clip3.mp4");
        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 12, 10);
        let located = testing::grid(&mut tree, "L", 4, 6);
        testing::add_locations(&mut tree, located);
        let cancel = std::sync::atomic::AtomicBool::new(false);
        for ty in gen::ENUM_EffectType.members {
            for (targets, loc) in [([m], false), ([located], true)] {
                let ctx = ctx_for(&tree, &targets, &dir, &cancel, 1500);
                let mut p = DESCRIPTOR.new_params();
                p.set("FileName", ParamValue::Str("clip3.mp4".into()));
                p.set("Type", ParamValue::Enum(ty.name.into()));
                if loc {
                    p.set("TargetPositioning", ParamValue::Enum("Locations".into()));
                }
                let _ = (DESCRIPTOR.render)(&ctx, &p);
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn quality_slider_is_for_locations() {
        let mut p = DESCRIPTOR.new_params();
        let vis: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(!vis.contains(&"VideoQuality"));
        p.set("TargetPositioning", ParamValue::Enum("Locations".into()));
        let vis: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(vis.contains(&"VideoQuality"));
    }
}
