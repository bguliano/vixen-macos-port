// Windows property-dialog entry points only. Never used by the render oracle.
namespace System.Windows.Forms.Integration {
 public static class ElementHost { public static void EnableModelessKeyboardInterop(object window)=>throw new NotSupportedException(); }
}
namespace System.Windows.Interop {
 public sealed class WindowInteropHelper(object window) { public IntPtr Owner {get=>throw new NotSupportedException();set=>throw new NotSupportedException();} }
}
namespace System.Windows.Forms {
 public class Form {public static Form ActiveForm=>throw new NotSupportedException();public IntPtr Handle=>throw new NotSupportedException();}
}
namespace VixenModules.Property.State.Setup.Services { public sealed class StateColorPickerService {} }
namespace VixenModules.Property.State.Setup.ViewModels {
 public sealed class StateMapperViewModel(Vixen.Sys.IElementNode node, VixenModules.Property.State.StateData data, VixenModules.Property.State.Setup.Services.StateColorPickerService service) {}
}
namespace VixenModules.Property.State.Setup.Views {
 public sealed class StateMapperView(object model) { public bool? ShowDialog()=>throw new NotSupportedException(); }
}
