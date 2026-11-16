// Value-only output containers. No drawing, parsing, geometry or conversion algorithms.
namespace Svg {
 public sealed class SvgCircle { public float CenterX {get;set;}public float CenterY {get;set;}public float Radius {get;set;} }
 public sealed class SvgEllipse { public float CenterX {get;set;}public float CenterY {get;set;}public float RadiusX {get;set;}public float RadiusY {get;set;} }
 public sealed class SvgRectangle { public float X {get;set;}public float Y {get;set;}public float Width {get;set;}public float Height {get;set;} }
 public sealed class SvgPolygon { public SvgPointCollection Points {get;set;} }
 public sealed class SvgPointCollection:List<int> {}
}
