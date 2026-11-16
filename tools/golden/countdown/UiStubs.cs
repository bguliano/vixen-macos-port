// Outside the numeric probe: fail loudly if a font/raster setup path is accidentally entered.
namespace Common.Controls.Scaling {
 public static class ScalingTools { public static double GetScaleFactor()=>throw new NotSupportedException("GDI setup is outside the numeric probe"); }
}
