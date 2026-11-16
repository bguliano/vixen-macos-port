// Availability only: the existing oracle's unchanged Bitmap/Graphics and Font
// probes. No Vixen runtime, effects, profile, devices or playback are started.
Console.WriteLine(System.Runtime.InteropServices.RuntimeInformation.OSDescription);
Console.WriteLine(System.Runtime.InteropServices.RuntimeInformation.FrameworkDescription);
Console.WriteLine(typeof(System.Drawing.Bitmap).Assembly.FullName);
return GdiProbe.Run() ? 0 : 1;
