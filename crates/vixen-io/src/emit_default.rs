//! The effect members marked `[DataMember(EmitDefaultValue = false)]`.
//!
//! DataContractSerializer leaves these out while they hold their type's default, so writing them would add
//! members real Vixen never writes. They are mostly obsolete settings that later moved into curves.
//! `ParamDesc` does not record the attribute, so the list lives here, taken from the C# data classes.

/// (CLR namespace of the declaring class, member name).
const MEMBERS: [(&str, &str); 55] = [
    ("VixenModules.Effect.CountDown", "Colors"),
    ("VixenModules.Effect.Fire", "HueShift"),
    ("VixenModules.Effect.Fireworks", "EnableAudio"),
    ("VixenModules.Effect.Garlands", "Spacing"),
    ("VixenModules.Effect.Garlands", "Speed"),
    ("VixenModules.Effect.Life", "Speed"),
    ("VixenModules.Effect.LipSync", "AllowMarkGaps"),
    ("VixenModules.Effect.Meteors", "Length"),
    ("VixenModules.Effect.Meteors", "MaxSpeed"),
    ("VixenModules.Effect.Meteors", "MinSpeed"),
    ("VixenModules.Effect.Meteors", "PixelCount"),
    ("VixenModules.Effect.Meteors", "RandomMeteorPosition"),
    ("VixenModules.Effect.Meteors", "RandomSpeed"),
    ("VixenModules.Effect.Meteors", "Speed"),
    ("VixenModules.Effect.Picture", "IncreaseBrightness"),
    ("VixenModules.Effect.Picture", "XOffset"),
    ("VixenModules.Effect.Picture", "YOffset"),
    ("VixenModules.Effect.PinWheel", "CenterStart"),
    ("VixenModules.Effect.PinWheel", "PinWheel3D"),
    ("VixenModules.Effect.PinWheel", "Size"),
    ("VixenModules.Effect.PinWheel", "Speed"),
    ("VixenModules.Effect.PinWheel", "Thickness"),
    ("VixenModules.Effect.PinWheel", "Twist"),
    ("VixenModules.Effect.PinWheel", "XOffset"),
    ("VixenModules.Effect.PinWheel", "YOffset"),
    ("VixenModules.Effect.Plasma", "LineDensity"),
    ("VixenModules.Effect.Plasma", "Speed"),
    ("VixenModules.Effect.Shockwave", "Acceleration"),
    ("VixenModules.Effect.Shockwave", "CenterX"),
    ("VixenModules.Effect.Shockwave", "CenterY"),
    ("VixenModules.Effect.Shockwave", "EndRadius"),
    ("VixenModules.Effect.Shockwave", "EndWidth"),
    ("VixenModules.Effect.Shockwave", "StartRadius"),
    ("VixenModules.Effect.Shockwave", "StartWidth"),
    ("VixenModules.Effect.SnowStorm", "Count"),
    ("VixenModules.Effect.SnowStorm", "Length"),
    ("VixenModules.Effect.SnowStorm", "Speed"),
    ("VixenModules.Effect.Snowflakes", "FlakeCount"),
    ("VixenModules.Effect.Snowflakes", "MaxSpeed"),
    ("VixenModules.Effect.Snowflakes", "MinSpeed"),
    ("VixenModules.Effect.Snowflakes", "RandomSpeed"),
    ("VixenModules.Effect.Snowflakes", "Speed"),
    ("VixenModules.Effect.Spiral", "Rotation"),
    ("VixenModules.Effect.Spiral", "Thickness"),
    ("VixenModules.Effect.Spirograph", "Distance"),
    ("VixenModules.Effect.Spirograph", "Range"),
    ("VixenModules.Effect.Spirograph", "Speed"),
    ("VixenModules.Effect.Spirograph", "SpirographRange"),
    ("VixenModules.Effect.Text", "Position"),
    ("VixenModules.Effect.Text", "PositionX"),
    ("VixenModules.Effect.Text", "ScaleText"),
    ("VixenModules.Effect.Text", "YOffsetCurve"),
    ("VixenModules.Effect.Video", "IncreaseBrightness"),
    ("VixenModules.Effect.Video", "XOffset"),
    ("VixenModules.Effect.Video", "YOffset"),
];

/// Whether this member is left out of the file while it holds its type's default value.
pub(crate) fn skipped_when_default(clr_namespace: &str, member: &str) -> bool {
    MEMBERS.iter().any(|(n, m)| *n == clr_namespace && *m == member)
}
