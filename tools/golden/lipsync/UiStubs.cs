// Property-mode-only oracle. Matrix rendering and thumbnail bitmaps are unavailable.
namespace VixenModules.Property.Face {
 public sealed class FaceSetupHelper {public bool Perform(IEnumerable<Vixen.Sys.IElementNode> nodes)=>throw new NotSupportedException();}
}
namespace Common.Resources.Properties {
 public static class Resources {
  public static Bitmap AI=>null;public static Bitmap E=>null;public static Bitmap etc=>null;
  public static Bitmap FV=>null;public static Bitmap L=>null;public static Bitmap MBP=>null;
  public static Bitmap O=>null;public static Bitmap rest=>null;public static Bitmap U=>null;public static Bitmap WQ=>null;
 }
}
namespace VixenModules.App.LipSyncApp {
 public sealed class LipSyncMapLibrary:Vixen.Module.App.AppModuleInstanceBase,IEnumerable<KeyValuePair<string,LipSyncMapData>> {
  public override void Loading()=>throw new NotSupportedException();public override void Unloading()=>throw new NotSupportedException();
  public override Vixen.Sys.IApplication Application {set=>throw new NotSupportedException();}
  public Dictionary<string,LipSyncMapData> Library=>throw new NotSupportedException();
  public string DefaultMappingName=>throw new NotSupportedException();
  public IEnumerator<KeyValuePair<string,LipSyncMapData>> GetEnumerator()=>throw new NotSupportedException();
  System.Collections.IEnumerator System.Collections.IEnumerable.GetEnumerator()=>GetEnumerator();
 }
}
namespace VixenModules.Effect.LipSync {
 public sealed class FastPictureEffect {
  public FastPictureEffect()=>throw new NotSupportedException("Matrix render outside property-mode oracle");
  public TimeSpan StartTime {get;set;}public TimeSpan TimeSpan {get;set;}public TimeSpan EffectEndTime {get;set;}
  public Image Image {get;set;}public Vixen.Sys.IElementNode[] TargetNodes {get;set;}
  public VixenModules.Effect.Effect.StringOrientation StringOrientation {get;set;}
  public bool ScaleToGrid {get;set;}public int ScalePercent {get;set;}public int TotalFrames {get;set;}public int Level {get;set;}
  public VixenModules.App.Curves.Curve XOffsetCurve {get;set;}public VixenModules.App.Curves.Curve YOffsetCurve {get;set;}
  public void MarkDirty()=>throw new NotSupportedException();public Vixen.Sys.EffectIntents Render()=>throw new NotSupportedException();
  public void CacheElementEnumerator()=>throw new NotSupportedException();public void UloadElementCache()=>throw new NotSupportedException();
 }
}
