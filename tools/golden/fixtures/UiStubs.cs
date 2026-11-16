// The oracle never opens editors or draws thumbnails. These UI-only dependencies
// deliberately throw if reached; all fixture model/effect/render code is unchanged upstream.
namespace VixenModules.App.FixtureSpecificationManager {
 public interface IFixtureSpecificationManager { List<VixenModules.App.Fixture.FixtureSpecification> FixtureSpecifications { get; } string GetGoboImageDirectory(); }
 public class FixtureSpecificationManager : IFixtureSpecificationManager {
  static readonly FixtureSpecificationManager instance = new();
  public static IFixtureSpecificationManager Instance() => instance;
  public List<VixenModules.App.Fixture.FixtureSpecification> FixtureSpecifications {get;} = new();
  public string GetGoboImageDirectory() => throw new NotSupportedException("UI forbidden in fixture oracle");
 }
}
namespace VixenModules.Editor.FixturePropertyEditor.Views {
 public class FixturePropertyEditorWindowView(VixenModules.App.Fixture.FixtureSpecification specification) {
  public bool? ShowDialog() => throw new NotSupportedException("UI forbidden in fixture oracle");
  public VixenModules.App.Fixture.FixtureSpecification GetFixtureSpecification() => throw new NotSupportedException("UI forbidden in fixture oracle");
 }
}
namespace VixenModules.App.Curves {
 public static class FixtureOracleDrawing {
  public static System.Drawing.Bitmap GenerateGenericCurveImage(this Curve curve, System.Drawing.Size size, bool a, bool b, bool c, System.Drawing.Color color, List<System.Drawing.Color> colors = null) => throw new NotSupportedException("UI forbidden in fixture oracle");
 }
}
