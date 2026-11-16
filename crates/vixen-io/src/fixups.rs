//! The `[OnDeserialized]` and `[OnDeserializing]` logic of the effect data classes.
//!
//! These decide what an older file renders as, so a port that skipped them would light the wrong thing: a
//! Plasma saved before speed became a curve keeps its speed in an `int`, and this is the code that moves it.
//! Each function follows its C# method line for line. A value this logic changes is written back to a file
//! only if the effect is edited afterwards, because `params.rs` records what typed encoding said at load.

use vixen_core::{Color, ColorGradient, Curve};
use vixen_effects::framework::{scale_value_to_curve, ParamKind, ParamValue, Params};
use vixen_xml::XElement;

/// Runs the logic for one data class. `absent` lists the members the file did not have, which is how a C#
/// `== null` test on a member looks here, since `absent_value` gives missing references a table default.
/// `originals` holds each table member's element from the file, parallel to the table.
pub(crate) fn on_deserialized(
    data_type: &str,
    params: &mut Params,
    absent: &[&'static str],
    originals: &[Option<&XElement>],
) {
    let mut f = Fix { p: params, absent, originals };
    match data_type {
        "BallData" => balls(&mut f),
        "BarsData" => bars(&mut f),
        "BorderData" => border(&mut f),
        "ButterflyData" => butterfly(&mut f),
        "CandleData" => candle(&mut f),
        "ColorWashData" => color_wash(&mut f),
        "CountDownData" => count_down(&mut f),
        "CurtainData" => curtain(&mut f),
        "FireData" => fire(&mut f),
        "FireworksData" => fireworks(&mut f),
        "GarlandsData" => garlands(&mut f),
        "LifeData" => life(&mut f),
        "LipSyncData" => lip_sync(&mut f),
        "MeteorsData" => meteors(&mut f),
        "WeaveData" => weave(&mut f),
        "PictureData" => picture(&mut f),
        "PinWheelData" => pin_wheel(&mut f),
        "PlasmaData" => plasma(&mut f),
        "ShockwaveData" => shockwave(&mut f),
        "SnowStormData" => snow_storm(&mut f),
        "SnowflakesData" => snowflakes(&mut f),
        "SpinData" => spin(&mut f),
        "SpiralData" => spiral(&mut f),
        "SpirographData" => spirograph(&mut f),
        "TextData" => text(&mut f),
        "TwinkleData" => twinkle(&mut f),
        "VideoData" => video(&mut f),
        "WipeData" => wipe(&mut f),
        // `Color ??= new ColorGradient()`: the table's default is the same gradient.
        // `CustomStateItems ??= []`: the table's default is the same empty list.
        _ => {}
    }
}

struct Fix<'a> {
    p: &'a mut Params,
    absent: &'a [&'static str],
    originals: &'a [Option<&'a XElement>],
}

impl Fix<'_> {
    fn index(&self, xml: &str) -> Option<usize> {
        let i = self.p.desc.iter().position(|d| d.xml == xml);
        // A wrong name here would silently do nothing, so the tests below catch it instead.
        #[cfg(test)]
        assert!(i.is_some(), "this table has no member {xml}");
        i
    }

    fn value(&self, xml: &str) -> &ParamValue {
        self.index(xml).map_or(&ParamValue::None, |i| &self.p.values[i])
    }

    /// `Member == null` for a reference type.
    fn is_null(&self, xml: &str) -> bool {
        self.absent.contains(&xml) || matches!(self.value(xml), ParamValue::None)
    }

    fn set(&mut self, xml: &str, v: ParamValue) {
        if let Some(i) = self.index(xml) {
            self.p.values[i] = v;
        }
    }

    fn set_int(&mut self, xml: &str, v: i32) {
        self.set(xml, ParamValue::Int(v));
    }

    fn set_bool(&mut self, xml: &str, v: bool) {
        self.set(xml, ParamValue::Bool(v));
    }

    fn set_enum(&mut self, xml: &str, v: &str) {
        self.set(xml, ParamValue::Enum(v.to_string()));
    }

    fn set_curve(&mut self, xml: &str, points: Vec<(f64, f64)>) {
        self.set(xml, ParamValue::Curve(Curve::from_points(points)));
    }

    /// `new Curve(new PointPairList(new[] { 0.0, 100.0 }, new[] { v, v }))`.
    fn set_flat(&mut self, xml: &str, v: f64) {
        self.set_curve(xml, vec![(0.0, v), (100.0, v)]);
    }

    /// `if (curve == null) curve = flat(v)`.
    fn flat_if_null(&mut self, xml: &str, v: f64) {
        if self.is_null(xml) {
            self.set_flat(xml, v);
        }
    }

    fn int(&self, xml: &str) -> i32 {
        match self.value(xml) {
            ParamValue::Int(v) => *v,
            ParamValue::Long(v) => *v as i32,
            ParamValue::Double(v) => *v as i32,
            _ => 0,
        }
    }

    fn double(&self, xml: &str) -> f64 {
        match self.value(xml) {
            ParamValue::Double(v) | ParamValue::Percentage(v) => *v,
            ParamValue::Int(v) => *v as f64,
            ParamValue::Long(v) => *v as f64,
            _ => 0.0,
        }
    }

    fn boolean(&self, xml: &str) -> bool {
        matches!(self.value(xml), ParamValue::Bool(true))
    }

    fn string(&self, xml: &str) -> &str {
        match self.value(xml) {
            ParamValue::Str(s) | ParamValue::Enum(s) => s,
            _ => "",
        }
    }

    /// `double value = ScaleValueToCurve(old, max, min); curve = flat(value); old = 0;`.
    fn move_to_curve(&mut self, curve: &str, old: &str, max: f64, min: f64) {
        let value = scale_value_to_curve(self.int(old) as f64, max, min);
        self.set_flat(curve, value);
        self.set_int(old, 0);
    }

    /// `move_to_curve` guarded by `if (curve == null)`.
    fn move_if_null(&mut self, curve: &str, old: &str, max: f64, min: f64) {
        if self.is_null(curve) {
            self.move_to_curve(curve, old, max, min);
        }
    }

    /// `if (!Enum.IsDefined(member)) member = fallback`.
    fn enum_or(&mut self, xml: &str, fallback: &str) {
        let Some(i) = self.index(xml) else { return };
        let ParamKind::Enum(e) = self.p.desc[i].kind else { return };
        if !e.members.iter().any(|m| m.name == self.string(xml)) {
            self.set_enum(xml, fallback);
        }
    }

    fn at_least_zero(&mut self, xml: &str) {
        if self.int(xml) < 0 {
            self.set_int(xml, 0);
        }
    }

    /// `Equals(member, new Curve(new PointPairList(new[] { 0.0, 100.0 }, new[] { v, v })))`. `Curve.Equals`
    /// compares points by value and the library name as a string, so a nil name in the file does not match
    /// the new curve's empty one.
    fn curve_equals_flat(&self, xml: &str, v: f64) -> bool {
        let Some(i) = self.index(xml) else { return false };
        let ParamValue::Curve(c) = &self.p.values[i] else { return false };
        let name_is_nil = self
            .originals
            .get(i)
            .copied()
            .flatten()
            .and_then(|e| e.child("_libraryReferenceName"))
            .is_some_and(|n| n.nil);
        c.points == [(0.0, v), (100.0, v)]
            && c.library_reference_name.is_empty()
            && !name_is_nil
            && !c.is_current_library_curve
    }

    fn white_if_empty(&mut self, xml: &str) {
        if matches!(self.value(xml), ParamValue::Color(c) if *c == Color::EMPTY) {
            self.set(xml, ParamValue::Color(Color::WHITE));
        }
    }
}

fn gradient(c: Color) -> ColorGradient {
    ColorGradient::solid(c)
}

fn balls(f: &mut Fix) {
    f.flat_if_null("RandomMaxCurve", 50.0);
}

fn bars(f: &mut Fix) {
    f.flat_if_null("SpeedCurve", 50.0);
    f.flat_if_null("RotationAngle", 50.0);
}

fn border(f: &mut Fix) {
    if f.is_null("BorderHeightCurve") {
        let size = f.value("BorderSizeCurve").clone();
        f.set("BorderHeightCurve", size);
        f.flat_if_null("XOffsetCurve", 50.0);
        f.flat_if_null("YOffsetCurve", 50.0);
    }
    // An effect created before the Marquee mode has no speed.
    if f.is_null("Speed") {
        f.set_enum("RenderLevel", "Level0");
        f.set_int("BandSize", 3);
        f.set_int("SkipSize", 0);
        f.set_int("Thickness", 1);
        f.set_int("Stagger", 0);
        f.set_flat("Speed", 10.0);
        f.set_int("XScale", 100);
        f.set_int("YScale", 100);
        f.set_bool("Reverse", false);
        f.set_bool("WrapX", false);
        f.set_bool("WrapY", false);
        f.set_int("RenderScaleFactor", 4);
        f.set(
            "Colors",
            ParamValue::GradientList(vec![gradient(Color::RED), gradient(Color::LIME), gradient(Color::WHITE)]),
        );
    }
}

fn butterfly(f: &mut Fix) {
    f.flat_if_null("SpeedCurve", 60.0);
}

fn candle(f: &mut Fix) {
    if f.int("GroupLevel") <= 0 {
        f.set_int("GroupLevel", 1);
    }
    f.white_if_empty("Color");
}

fn color_wash(f: &mut Fix) {
    f.flat_if_null("SpeedCurve", 23.0);
}

fn count_down(f: &mut Fix) {
    let colors = match f.value("Colors") {
        ParamValue::Gradient(g) => Some(g.clone()),
        _ => None,
    };
    let list = || ParamValue::GradientList(colors.iter().cloned().collect());
    if f.is_null("GradientColors") {
        f.set("GradientColors", list());
    }
    if f.is_null("SpinnerColors") {
        f.set("SpinnerColors", list());
    }
}

fn curtain(f: &mut Fix) {
    if f.is_null("PositionCurve") {
        f.set_curve("PositionCurve", vec![(0.0, 100.0), (50.0, 0.0), (100.0, 100.0)]);
    }
}

fn fire(f: &mut Fix) {
    f.move_if_null("HueShiftCurve", "HueShift", 100.0, 0.0);
    f.enum_or("TargetNodeSelection", "Group");
    f.at_least_zero("DepthOfEffect");
}

fn fireworks(f: &mut Fix) {
    if f.boolean("EnableAudio") {
        f.set_enum("FireworksSource", "Audio");
    }
    f.set_bool("EnableAudio", false);
    if f.int("DecayTime") == 0 {
        f.set_int("ExplosionSensitivity", 5);
        f.set_int("Sensitivity", -70);
        f.set_bool("LowPass", false);
        f.set_int("LowPassFreq", 1000);
        f.set_bool("HighPass", false);
        f.set_int("HighPassFreq", 500);
        f.set_bool("Normalize", true);
        f.set_int("DecayTime", 1500);
        f.set_int("AttackTime", 52);
    }
}

fn garlands(f: &mut Fix) {
    if f.is_null("SpacingCurve") {
        f.move_to_curve("SpacingCurve", "Spacing", 20.0, 1.0);
        f.move_if_null("SpeedCurve", "Speed", 100.0, 1.0);
    }
}

fn life(f: &mut Fix) {
    f.move_if_null("SpeedCurve", "Speed", 20.0, 1.0);
}

fn lip_sync(f: &mut Fix) {
    if f.is_null("XOffsetCurve") {
        f.set_flat("YOffsetCurve", 50.0);
        f.set_flat("XOffsetCurve", 50.0);
    }
}

fn meteors(f: &mut Fix) {
    if f.is_null("GroundLevelCurve") {
        f.set_flat("GroundLevelCurve", 0.0);
        f.set("GroundColor", ParamValue::Gradient(gradient(Color::rgb(34, 139, 34))));

        if f.is_null("SpeedVariationCurve") {
            let variation = if f.boolean("RandomSpeed") { (f.int("MaxSpeed") - f.int("MinSpeed")) as f64 } else { 0.0 };
            let value = scale_value_to_curve(variation, 200.0, 1.0);
            f.set_flat("SpeedVariationCurve", value);
        }
        if f.is_null("CenterSpeedCurve") {
            // `(double)(MaxSpeed + MinSpeed) / 2`, against `Speed` as an int.
            let center = if f.boolean("RandomSpeed") {
                (f.int("MaxSpeed") + f.int("MinSpeed")) as f64 / 2.0
            } else {
                f.int("Speed") as f64
            };
            let value = scale_value_to_curve(center, 200.0, 1.0);
            f.set_flat("CenterSpeedCurve", value);
            f.set_int("MinSpeed", 0);
            f.set_int("MaxSpeed", 0);
            f.set_int("Speed", 0);
            f.set_bool("RandomSpeed", true);
        }
        f.move_if_null("PixelCountCurve", "PixelCount", 200.0, 1.0);
        f.move_if_null("LengthCurve", "Length", 100.0, 1.0);
    }
    if f.is_null("TailTaperCurve") {
        f.set_curve("TailTaperCurve", vec![(0.0, 100.0), (100.0, 0.0)]);
    }
    if f.is_null("XCenterSpeedCurve") {
        f.set_flat("XCenterSpeedCurve", 50.0);
        f.flat_if_null("YCenterSpeedCurve", 50.0);
        f.flat_if_null("XSpeedVariationCurve", 0.0);
        f.flat_if_null("YSpeedVariationCurve", 0.0);
        if f.is_null("WobbleCurve") {
            f.set_curve("WobbleCurve", vec![(0.0, 30.0), (33.0, 70.0), (66.0, 30.0), (100.0, 70.0)]);
        }
        f.flat_if_null("WobbleVariationCurve", 10.0);
    }
}

/// `[OnDeserializing]`: these defaults go in before the members are read, so only a missing member keeps them.
fn weave(f: &mut Fix) {
    if f.absent.contains(&"MortarHeight") {
        f.set_int("MortarHeight", 5);
    }
    if f.absent.contains(&"MortarColor") {
        f.set("MortarColor", ParamValue::Gradient(gradient(Color::BLACK)));
    }
    if f.absent.contains(&"BrickHeight") {
        f.set_int("BrickHeight", 5);
    }
    if f.absent.contains(&"BrickWidth") {
        f.set_int("BrickWidth", 15);
    }
}

fn picture(f: &mut Fix) {
    if f.string("FileName").is_empty() {
        f.set_enum("Source", "Embedded");
    }
    if f.string("TilePictures") == "None" {
        f.set_enum("TilePictures", "BlueGlowDots");
    }
    if f.is_null("ScalingCurve") {
        f.set_flat("ScalingCurve", 100.0);
    }
    if f.is_null("XOffsetCurve") {
        f.move_to_curve("XOffsetCurve", "XOffset", 100.0, -100.0);
        f.move_if_null("YOffsetCurve", "YOffset", 100.0, -100.0);
        f.move_if_null("IncreaseBrightnessCurve", "IncreaseBrightness", 100.0, 10.0);
    }
}

fn pin_wheel(f: &mut Fix) {
    if f.boolean("PinWheel3D") {
        f.set_enum("PinWheelBladeType", "ThreeD");
        f.set_bool("PinWheel3D", false);
    }
    if f.is_null("TwistCurve") {
        f.move_to_curve("TwistCurve", "Twist", 500.0, -500.0);
        f.move_if_null("ThicknessCurve", "Thickness", 100.0, 1.0);
        f.move_if_null("SizeCurve", "Size", 400.0, 1.0);
        f.move_if_null("SpeedCurve", "Speed", 50.0, 1.0);
        f.move_if_null("XOffsetCurve", "XOffset", 100.0, -100.0);
        f.move_if_null("YOffsetCurve", "YOffset", 100.0, -100.0);
        f.move_if_null("CenterHubCurve", "CenterStart", 100.0, 0.0);
    }
    if f.is_null("RotationCurve") {
        let value = if f.string("Rotation") == "Forward" { 100.0 } else { 0.0 };
        f.set_flat("RotationCurve", value);
        f.set_enum("Rotation", "Forward");
    }
    if !f.boolean("OffsetPercentage")
        && f.curve_equals_flat("XOffsetCurve", 50.0)
        && f.curve_equals_flat("YOffsetCurve", 50.0)
    {
        f.set_bool("OffsetPercentage", true);
    }
}

fn plasma(f: &mut Fix) {
    if f.is_null("LineDensityCurve") {
        f.move_to_curve("LineDensityCurve", "LineDensity", 10.0, 1.0);
        f.move_if_null("SpeedCurve", "Speed", 100.0, 1.0);
    }
}

fn shockwave(f: &mut Fix) {
    if f.is_null("WidthCurve") {
        let value = scale_value_to_curve(f.int("StartWidth") as f64, 255.0, 0.0);
        let end_value = scale_value_to_curve(f.int("EndWidth") as f64, 255.0, 0.0);
        f.set_curve("WidthCurve", vec![(0.0, value), (100.0, end_value)]);
        f.set_int("StartWidth", 0);
        f.set_int("EndWidth", 0);

        if f.is_null("CenterYCurve") {
            if f.string("TargetPositioning") == "Locations" {
                let y = 100 - f.int("CenterY");
                f.set_int("CenterY", y);
            } else if f.string("Orientation") == "Vertical" {
                // Fixes the backwards x y adjustment when strings are vertical.
                let (x, y) = (f.int("CenterX"), f.int("CenterY"));
                f.set_int("CenterX", y);
                f.set_int("CenterY", x);
            }
            let y = f.int("CenterY") as f64;
            f.set_flat("CenterYCurve", y);
            f.set_int("CenterY", 0);
        }
        if f.is_null("CenterXCurve") {
            let x = f.int("CenterX") as f64;
            f.set_flat("CenterXCurve", x);
            f.set_int("CenterX", 0);
        }
        f.move_if_null("AccelerationCurve", "Acceleration", 10.0, -10.0);
        if f.is_null("RadiusCurve") {
            let value = scale_value_to_curve(f.int("StartRadius") as f64, 750.0, 0.0);
            let end_value = scale_value_to_curve(f.int("EndRadius") as f64, 750.0, 0.0);
            f.set_curve("RadiusCurve", vec![(0.0, value), (100.0, end_value)]);
            f.set_int("EndRadius", 0);
            f.set_int("StartRadius", 0);
        }
    }
}

fn snow_storm(f: &mut Fix) {
    if f.is_null("LengthCurve") {
        f.move_to_curve("LengthCurve", "Length", 20.0, 1.0);
        f.move_if_null("CountCurve", "Count", 100.0, 1.0);
        f.move_if_null("SpeedCurve", "Speed", 100.0, 1.0);
    }
}

fn snowflakes(f: &mut Fix) {
    if f.is_null("FadeSpeed") {
        f.set_flat("FadeSpeed", 40.0);
        f.set_bool("CycleColor", true);
        f.set_int("HFlakeCount", 4);
        f.set_int("VFlakeCount", 4);
        f.set_enum("FadeType", "None");
    }
    if f.is_null("BuildUpSpeedCurve") {
        f.set_flat("BuildUpSpeedCurve", 15.0);
        if f.is_null("SpeedVariationCurve") {
            let variation = if f.boolean("RandomSpeed") { (f.int("MaxSpeed") - f.int("MinSpeed")) as f64 } else { 0.0 };
            let value = scale_value_to_curve(variation, 60.0, 1.0);
            f.set_flat("SpeedVariationCurve", value);
        }
        f.move_if_null("FlakeCountCurve", "FlakeCount", 100.0, 1.0);
        if f.is_null("CenterSpeedCurve") {
            let center = if f.boolean("RandomSpeed") {
                (f.int("MaxSpeed") + f.int("MinSpeed")) as f64 / 2.0
            } else {
                f.int("Speed") as f64
            };
            let value = scale_value_to_curve(center, 60.0, 1.0);
            f.set_flat("CenterSpeedCurve", value);
            f.set_int("MinSpeed", 0);
            f.set_int("MaxSpeed", 0);
            f.set_int("Speed", 0);
            f.set_bool("RandomSpeed", true);
        }
    }
    if f.is_null("XCenterSpeedCurve") {
        f.set_flat("XCenterSpeedCurve", 50.0);
        f.flat_if_null("YCenterSpeedCurve", 50.0);
        f.flat_if_null("XSpeedVariationCurve", 0.0);
        f.flat_if_null("YSpeedVariationCurve", 0.0);
        f.flat_if_null("WobbleCurve", 50.0);
        f.flat_if_null("WobbleVariationCurve", 0.0);
    }
}

fn spin(f: &mut Fix) {
    f.white_if_empty("StaticColor");
}

fn spiral(f: &mut Fix) {
    if f.is_null("RotationCurve") {
        f.move_to_curve("RotationCurve", "Rotation", 150.0, -150.0);
        f.move_if_null("ThicknessCurve", "Thickness", 100.0, 1.0);
    }
    f.flat_if_null("SpeedCurve", 70.0);
}

fn spirograph(f: &mut Fix) {
    if f.is_null("DistanceCurve") {
        f.move_to_curve("DistanceCurve", "Distance", 100.0, 1.0);
        f.move_if_null("RangeCurve", "Range", 200.0, 1.0);
        f.move_if_null("SpirographRangeCurve", "SpirographRange", 300.0, 1.0);
        f.move_if_null("SpeedCurve", "Speed", 100.0, 1.0);
    }
}

fn text(f: &mut Fix) {
    // `[OnDeserializing]` starts `_cycleColorMode` at Character and the property setter notes that the member
    // was read, which is the same test as the member being in the file.
    let cycle_mode_was_read = !f.absent.contains(&"CycleColorMode");
    if !cycle_mode_was_read {
        f.set_enum("CycleColorMode", "Character");
    }

    if f.int("TimeVisibleLength") == 0 {
        f.set_int("TimeVisibleLength", 750);
    }
    if f.is_null("LevelCurve") {
        f.set_flat("LevelCurve", 100.0);
    }
    if f.is_null("BaseLevelCurve") {
        f.set_flat("BaseLevelCurve", 100.0);
    }
    if f.is_null("FontScaleCurve") {
        f.set_curve("AngleCurve", vec![(0.0, 0.0), (100.0, 100.0)]);
        f.set_flat("FontScaleCurve", 100.0);
        f.set_int("ScaleText", 0);
        f.move_if_null("XOffsetCurve", "PositionX", 100.0, -100.0);
        f.move_if_null("YOffsetCurve", "Position", 100.0, -100.0);
    }
    if f.is_null("YOffSetCurve") {
        // Copies the old Y offset curve inverted, since it had been set the wrong way around.
        if let ParamValue::Curve(old) = f.value("YOffsetCurve").clone() {
            let mut inverted = old;
            for p in &mut inverted.points {
                p.1 = 100.0 - p.1;
            }
            f.set("YOffSetCurve", ParamValue::Curve(inverted));
        }
    }
    if f.is_null("ExplodePosition") {
        f.set_flat("ExplodePosition", 50.0);
        f.set_flat("FallSpeedCurve", 10.0);
    }
    if f.boolean("CycleCharacterColor") && !cycle_mode_was_read {
        f.set_bool("CycleColor", true);
        f.set_enum("CycleColorMode", "Character");
    } else if f.boolean("CycleColor") && f.string("TextSource") != "None" && !cycle_mode_was_read {
        f.set_enum("CycleColorMode", "Word");
    }
}

fn twinkle(f: &mut Fix) {
    f.white_if_empty("StaticColor");
    if f.double("MaximumLevel") > 1.0 {
        f.set("MaximumLevel", ParamValue::Double(1.0));
    }
}

fn video(f: &mut Fix) {
    if f.is_null("XOffsetCurve") {
        f.move_to_curve("XOffsetCurve", "XOffset", 100.0, -100.0);
        f.move_if_null("YOffsetCurve", "YOffset", 100.0, -100.0);
        f.move_if_null("IncreaseBrightnessCurve", "IncreaseBrightness", 100.0, 10.0);
    }
}

fn wipe(f: &mut Fix) {
    f.enum_or("TargetNodeSelection", "Group");
    f.at_least_zero("DepthOfEffect");
}

#[cfg(test)]
mod tests {
    use crate::params::{effect_params_from_xml, params_to_members};
    use crate::xml::same_tree;
    use vixen_core::{Color, Curve};
    use vixen_effects::framework::ParamValue;
    use vixen_xml::{contract_ns, XElement, ARRAYS_NS};

    fn model(data_type: &str, members: &[(&str, &str)]) -> XElement {
        let d = vixen_effect_registry::by_data_type(data_type).unwrap();
        let ns = contract_ns(d.data_ns);
        let mut e = XElement::new(ARRAYS_NS, "anyType");
        e.xsi_type = Some((ns.clone(), d.data_type.to_string()));
        e.push(XElement::with_text("", "ModuleInstanceId", "6879a66e-7199-45ae-ab30-874d6be50246"));
        e.push(XElement::with_text("", "ModuleTypeId", d.type_id.to_string()));
        for (name, text) in members {
            e.push(XElement::with_text(&ns, name, *text));
        }
        e
    }

    fn write_back(data_type: &str, e: &XElement, p: &vixen_effects::framework::Params) -> XElement {
        let d = vixen_effect_registry::by_data_type(data_type).unwrap();
        let mut out = XElement::new(ARRAYS_NS, "anyType");
        out.xsi_type = Some((contract_ns(d.data_ns), d.data_type.to_string()));
        out.push(e.child("ModuleInstanceId").unwrap().clone());
        out.push(e.child("ModuleTypeId").unwrap().clone());
        for m in params_to_members(p) {
            out.push(m);
        }
        out
    }

    #[test]
    fn plasma_moves_old_ints_into_curves() {
        let d = vixen_effect_registry::by_data_type("PlasmaData").unwrap();
        let e = model("PlasmaData", &[("LineDensity", "5"), ("Speed", "20")]);
        let p = effect_params_from_xml(&e, d);
        // ScaleValueToCurve(5, 10, 1) and ScaleValueToCurve(20, 100, 1).
        let expected_density = (5.0 - 1.0) * (100.0 / 9.0);
        let expected_speed = (20.0 - 1.0) * (100.0 / 99.0);
        assert_eq!(p.curve("LineDensityCurve").points, vec![(0.0, expected_density), (100.0, expected_density)]);
        assert_eq!(p.curve("SpeedCurve").points, vec![(0.0, expected_speed), (100.0, expected_speed)]);
        assert_eq!(p.int("LineDensity"), 0);
        assert_eq!(p.int("Speed"), 0);
        // The file is untouched until the effect changes, so Vixen can do this conversion itself.
        assert!(
            same_tree(&e, &write_back("PlasmaData", &e, &p)),
            "{:?}",
            e.first_difference(&write_back("PlasmaData", &e, &p), String::new())
        );
    }

    #[test]
    fn plasma_with_curves_is_left_alone() {
        let d = vixen_effect_registry::by_data_type("PlasmaData").unwrap();
        let curve = crate::values::curve_to_xml(&contract_ns(d.data_ns), "LineDensityCurve", &Curve::flat(7.0));
        let mut e = model("PlasmaData", &[]);
        e.push(curve);
        let p = effect_params_from_xml(&e, d);
        assert_eq!(p.curve("LineDensityCurve").points, Curve::flat(7.0).points);
    }

    #[test]
    fn spin_gets_white_when_the_colour_is_missing() {
        let d = vixen_effect_registry::by_data_type("SpinData").unwrap();
        let e = model("SpinData", &[]);
        let p = effect_params_from_xml(&e, d);
        assert_eq!(p.color("StaticColor"), Color::WHITE);
        assert!(same_tree(&e, &write_back("SpinData", &e, &p)));
    }

    #[test]
    fn twinkle_clamps_old_levels_without_rewriting_them() {
        let d = vixen_effect_registry::by_data_type("TwinkleData").unwrap();
        let e = model("TwinkleData", &[("MaximumLevel", "100")]);
        let p = effect_params_from_xml(&e, d);
        assert_eq!(p.get("MaximumLevel"), &ParamValue::Double(1.0));
        assert!(same_tree(&e, &write_back("TwinkleData", &e, &p)));
    }

    #[test]
    fn candle_restores_its_group_level_and_colour() {
        let d = vixen_effect_registry::by_data_type("CandleData").unwrap();
        let p = effect_params_from_xml(&model("CandleData", &[]), d);
        assert_eq!(p.int("GroupLevel"), 1);
        assert_eq!(p.color("Color"), Color::WHITE);
    }

    #[test]
    fn wipe_rejects_an_undefined_selection_and_a_negative_depth() {
        let d = vixen_effect_registry::by_data_type("WipeData").unwrap();
        let e = model("WipeData", &[("DepthOfEffect", "-1"), ("TargetNodeSelection", "Nonsense")]);
        let p = effect_params_from_xml(&e, d);
        let by_xml = |xml: &str| &p.values[d.params.iter().position(|x| x.xml == xml).unwrap()];
        assert_eq!(by_xml("DepthOfEffect"), &ParamValue::Int(0));
        assert_eq!(by_xml("TargetNodeSelection"), &ParamValue::Enum("Group".to_string()));
    }

    #[test]
    fn pin_wheel_treats_centred_offsets_as_percentages() {
        let d = vixen_effect_registry::by_data_type("PinWheelData").unwrap();
        // XOffset 0 scales to a flat 50, and both curves then equal the default.
        let p = effect_params_from_xml(&model("PinWheelData", &[]), d);
        assert_eq!(p.curve("XOffsetCurve").points, vec![(0.0, 50.0), (100.0, 50.0)]);
        assert!(p.boolean("OffsetPercentage"));
        // A nil library name does not equal the new curve's empty one.
        let mut e = model("PinWheelData", &[]);
        let ns = contract_ns(d.data_ns);
        for name in ["XOffsetCurve", "YOffsetCurve", "TwistCurve", "RotationCurve"] {
            let mut c = crate::values::curve_to_xml(&ns, name, &Curve::flat(50.0));
            *c.child_mut("_libraryReferenceName").unwrap() =
                XElement::nil(crate::values::NS_CURVES, "_libraryReferenceName");
            e.push(c);
        }
        assert!(!effect_params_from_xml(&e, d).boolean("OffsetPercentage"));
    }

    #[test]
    fn text_keeps_the_cycle_mode_it_read() {
        let d = vixen_effect_registry::by_data_type("TextData").unwrap();
        let p = effect_params_from_xml(&model("TextData", &[("CycleColor", "true"), ("TextSource", "Marks")]), d);
        assert_eq!(p.string("CycleColorMode"), "Word");
        let p = effect_params_from_xml(
            &model("TextData", &[("CycleColor", "true"), ("CycleColorMode", "Character"), ("TextSource", "Marks")]),
            d,
        );
        assert_eq!(p.string("CycleColorMode"), "Character");
    }

    #[test]
    fn every_member_these_functions_touch_is_in_its_table() {
        // `Fix::index` fails a test on a name the table lacks. With every member missing, each function takes
        // the branches that read and write the most members.
        for data_type in [
            "BallData",
            "BarsData",
            "BorderData",
            "ButterflyData",
            "CandleData",
            "ColorWashData",
            "CountDownData",
            "CurtainData",
            "FireData",
            "FireworksData",
            "GarlandsData",
            "LifeData",
            "LipSyncData",
            "MeteorsData",
            "WeaveData",
            "PictureData",
            "PinWheelData",
            "PlasmaData",
            "ShockwaveData",
            "SnowStormData",
            "SnowflakesData",
            "SpinData",
            "SpiralData",
            "SpirographData",
            "TextData",
            "TwinkleData",
            "VideoData",
            "WipeData",
        ] {
            let d = vixen_effect_registry::by_data_type(data_type).unwrap();
            let _ = effect_params_from_xml(&model(data_type, &[]), d);
        }
    }

    #[test]
    fn the_branches_a_missing_member_does_not_reach() {
        let d = vixen_effect_registry::by_data_type("FireworksData").unwrap();
        let p = effect_params_from_xml(&model("FireworksData", &[("EnableAudio", "true")]), d);
        let by_xml = |xml: &str| &p.values[d.params.iter().position(|x| x.xml == xml).unwrap()];
        assert_eq!(by_xml("FireworksSource"), &ParamValue::Enum("Audio".to_string()));
        assert_eq!(by_xml("EnableAudio"), &ParamValue::Bool(false));
        assert_eq!(by_xml("DecayTime"), &ParamValue::Int(1500));
        assert_eq!(by_xml("Sensitivity"), &ParamValue::Int(-70));

        let d = vixen_effect_registry::by_data_type("PinWheelData").unwrap();
        let p = effect_params_from_xml(&model("PinWheelData", &[("PinWheel3D", "true"), ("Rotation", "Backward")]), d);
        let by_xml = |xml: &str| &p.values[d.params.iter().position(|x| x.xml == xml).unwrap()];
        assert_eq!(by_xml("PinWheelBladeType"), &ParamValue::Enum("ThreeD".to_string()));
        assert_eq!(by_xml("PinWheel3D"), &ParamValue::Bool(false));
        assert_eq!(by_xml("Rotation"), &ParamValue::Enum("Forward".to_string()));
        assert_eq!(by_xml("RotationCurve"), &ParamValue::Curve(Curve::flat(0.0)));

        let d = vixen_effect_registry::by_data_type("PictureData").unwrap();
        let p = effect_params_from_xml(&model("PictureData", &[("TilePictures", "None")]), d);
        let by_xml = |xml: &str| &p.values[d.params.iter().position(|x| x.xml == xml).unwrap()];
        assert_eq!(by_xml("TilePictures"), &ParamValue::Enum("BlueGlowDots".to_string()));
        assert_eq!(by_xml("Source"), &ParamValue::Enum("Embedded".to_string()));

        // Vertical strings swap a Shockwave's old centre, which was backwards.
        let d = vixen_effect_registry::by_data_type("ShockwaveData").unwrap();
        let p = effect_params_from_xml(
            &model("ShockwaveData", &[("CenterX", "10"), ("CenterY", "20"), ("Orientation", "Vertical")]),
            d,
        );
        let by_xml = |xml: &str| &p.values[d.params.iter().position(|x| x.xml == xml).unwrap()];
        assert_eq!(by_xml("CenterXCurve"), &ParamValue::Curve(Curve::flat(20.0)));
        assert_eq!(by_xml("CenterYCurve"), &ParamValue::Curve(Curve::flat(10.0)));
        assert_eq!(by_xml("CenterX"), &ParamValue::Int(0));
    }

    #[test]
    fn weave_defaults_apply_only_to_missing_members() {
        let d = vixen_effect_registry::by_data_type("WeaveData").unwrap();
        let p = effect_params_from_xml(&model("WeaveData", &[("BrickWidth", "9")]), d);
        let by_xml = |xml: &str| &p.values[d.params.iter().position(|x| x.xml == xml).unwrap()];
        assert_eq!(by_xml("BrickWidth"), &ParamValue::Int(9));
        assert_eq!(by_xml("BrickHeight"), &ParamValue::Int(5));
        assert_eq!(by_xml("MortarHeight"), &ParamValue::Int(5));
    }

    #[test]
    fn a_saved_model_with_every_member_is_left_alone() {
        // A new object's values are already what these functions would set.
        for d in vixen_effect_registry::all() {
            let e = crate::params::effect_model_to_xml(&d.type_id, &uuid::Uuid::from_u128(1), &d.new_params()).unwrap();
            let p = effect_params_from_xml(&e, d);
            assert_eq!(crate::params::hint_count(&p), 0, "{}: {:?}", d.name, crate::params::hinted_members(&p));
            let out = crate::params::effect_model_to_xml(&d.type_id, &uuid::Uuid::from_u128(1), &p).unwrap();
            assert!(same_tree(&e, &out), "{}: {:?}", d.name, e.first_difference(&out, String::new()));
        }
    }
}
