using System.Globalization;
using System.Text.Json;
using System.IO.Compression;
using Svg;
using Original=VixenModules.Effect.Shapes.GetGeometricShape;
internal static class Program {
 static readonly List<object> Cases=new();
 static void Add(string type,int radius,int count,int skip,int inside,float ratio,bool north,object output) {
  object shape=output switch {
   SvgCircle c=>new {tag="circle",values=new double[]{c.CenterX,c.CenterY,c.Radius}},
   SvgEllipse e=>new {tag="ellipse",values=new double[]{e.CenterX,e.CenterY,e.RadiusX,e.RadiusY}},
   SvgRectangle r=>new {tag="rect",values=new double[]{r.X,r.Y,r.Width,r.Height}},
   SvgPolygon p=>new {tag="polygon",values=p.Points.Select(x=>(double)x).ToArray()},
   _=>throw new InvalidOperationException()
  };
  Cases.Add(new {type,radius,count,skip,inside,ratio,north,shape});
 }
 static void Main(string[] args) {
  CultureInfo.DefaultThreadCurrentCulture=CultureInfo.InvariantCulture;
  foreach(int r in new[]{1,7,25,50,75,98,100}) {
   Add("circle",r,0,0,0,0,false,Original.CreateSvgCircle(r));
   Add("triangle",r,0,0,0,0,false,Original.CreateSvgTriangle(r));
   foreach(float ratio in new[]{0.01f,0.25f,0.33333334f,0.5f,0.75f,1f}) {
    Add("ellipse",r,0,0,0,ratio,false,Original.CreateSvgEllipse(r,ratio));
    Add("rect",r,0,0,0,ratio,false,Original.CreateSvgRectangle(r,ratio));
   }
   for(int n=3;n<=30;n++) {
    Add("polygon",r,n,0,0,0,false,Original.CreateSvgPolygon(r,n));
    for(int skip=1;skip<n;skip++) Add("star",r,n,skip,0,0,false,Original.CreateSvgMultiStar(r,n,skip));
    foreach(int inside in new[]{1,25,42,75,100}) foreach(bool north in new[]{false,true})
     Add("nonintersecting",r,n,0,inside,0,north,Original.CreateSvgNonIntersectingStar(r,n,inside,north));
   }
  }
  for(int ratio=1;ratio<=100;ratio++) Add("cross",0,0,0,0,ratio/100f,false,Original.CreateSvgCross(ratio/100f));
  Add("arrow",0,0,0,0,0,false,Original.CreateSvgArrow());
  string outDir=args.Length>0?args[0]:"target/golden/shapes/data";Directory.CreateDirectory(outDir);
  using var file=File.Create(Path.Combine(outDir,"shapes-geometry.json.gz"));using var gz=new GZipStream(file,CompressionLevel.SmallestSize);
  JsonSerializer.Serialize(gz,new {cases=Cases});Console.WriteLine($"{Cases.Count} shape geometry cases written to {outDir}");
 }
}
