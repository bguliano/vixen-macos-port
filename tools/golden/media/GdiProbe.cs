// Representative probes for the unchanged upstream bitmap/font render paths.
internal static class GdiProbe {
 public static bool Run() {
  bool available=Probe("Bitmap/Graphics",()=>{using var image=new Bitmap(8,8);using var graphics=Graphics.FromImage(image);graphics.Clear(Color.Black);using var pen=new Pen(Color.Red);graphics.DrawLine(pen,0,0,7,7);Console.WriteLine($"pixel={image.GetPixel(0,0).ToArgb():X8}");});
  // Keep probing after one failure so the readiness report covers both paths.
  available &= Probe("Font",()=>{using var font=new Font("Arial",12);Console.WriteLine(font.SizeInPoints);});
  return available;
 }
 static bool Probe(string name,Action run) {
  try {run();Console.WriteLine(name+": available");return true;}
  catch(Exception e) {Console.WriteLine(name+": "+e);return false;}
 }
}
