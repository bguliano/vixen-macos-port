// Original movement/sampling methods and FastPixel byte reads; no image decode or GDI draw.
using System.Globalization;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Text.Json;
using System.IO.Compression;
using VixenModules.Effect.Picture;
using VixenModules.Effect.Effect;
using VixenModules.App.Curves;
using Original=VixenModules.Effect.Picture.Picture;
internal static class Program {
 static readonly BindingFlags Flags=BindingFlags.Instance|BindingFlags.NonPublic;
 static readonly List<object> Cases=new();
 static FieldInfo Field(Type t,string name) {for(;t!=null;t=t.BaseType) {var f=t.GetField(name,Flags|BindingFlags.Public);if(f!=null)return f;}throw new MissingFieldException(name);}
 static void Set(object target,string name,object value)=>Field(target.GetType(),name).SetValue(target,value);
 static T Get<T>(object target,string name)=>(T)Field(target.GetType(),name).GetValue(target);
 static object Invoke(object target,string name,params object[] args)=>typeof(Original).GetMethod(name,Flags).Invoke(target,args);
 static Curve Flat(double value)=>new(new ZedGraph.PointPairList(new[]{0.0,100.0},new[]{value,value}));
 static void Main(string[] args) {
  CultureInfo.DefaultThreadCurrentCulture=CultureInfo.InvariantCulture;
  foreach(var type in Enum.GetValues<EffectType>()) foreach(var horizontal in new[]{false,true})
   foreach(var grid in new[]{(2,3),(8,6),(13,11)}) foreach(var img in new[]{(3,5),(8,4)})
    Add(type,horizontal,grid.Item1,grid.Item2,img.Item1,img.Item2,0,1,4,false,false,100,0,50,50);
  foreach(int direction in new[]{0,1,45,46,90,91,135,136,180,181,225,226,270,271,315,316,359,360})
   Add(EffectType.RenderPictureTiles,false,8,6,3,5,direction,3,4,false,false,100,0,50,50);
  foreach(var type in Enum.GetValues<EffectType>()) foreach(bool stop in new[]{false,true})
   Add(type,false,8,6,3,5,42,5,9,stop,true,30,80,15,85);
  foreach(var type in Enum.GetValues<EffectType>()) foreach(bool horizontal in new[]{false,true}) foreach(bool shifted in new[]{false,true})
   Add(type,horizontal,8,6,3,5,135,3,7,false,true,75,20,50,50,true,shifted?-3:0,shifted?5:0);
  string outDir=args.Length>0?args[0]:"target/golden/picture/data";Directory.CreateDirectory(outDir);
  using var file=File.Create(Path.Combine(outDir,"picture-sampling.json.gz"));using var gz=new GZipStream(file,CompressionLevel.SmallestSize);
  JsonSerializer.Serialize(gz,new {cases=Cases});Console.WriteLine($"{Cases.Count} Picture sampling cases written to {outDir}");
 }
 static void Add(EffectType type,bool horizontal,int width,int height,int iw,int ih,int direction,int speed,int rate,bool stop,bool custom,int level,int brightness,int xoffset,int yoffset,bool locations=false,int ox=0,int oy=0) {
  const int ms=1500,frameMs=50;
  var d=new PictureData {EffectType=type,Orientation=horizontal?StringOrientation.Horizontal:StringOrientation.Vertical,Direction=direction,Speed=speed,MovementRate=rate,CenterStop=stop,
   TargetPositioning=locations?TargetPositioningType.Locations:TargetPositioningType.Strings,ColorEffect=custom?ColorEffect.CustomColor:ColorEffect.None,LevelCurve=Flat(level),IncreaseBrightnessCurve=Flat(brightness),XOffsetCurve=Flat(xoffset),YOffsetCurve=Flat(yoffset)};
  var effect=(Original)RuntimeHelpers.GetUninitializedObject(typeof(Original));Set(effect,"_data",d);Set(effect,"FrameTime",frameMs);Set(effect,"_timeSpan",TimeSpan.FromMilliseconds(ms));
  Set(effect,"_bufferHt",horizontal?height:width);Set(effect,"_bufferWi",horizontal?width:height);
  Set(effect,"_bufferHtOffset",ox);Set(effect,"_bufferWiOffset",oy);
  Set(effect,"_pictures",Activator.CreateInstance(Field(typeof(Original),"_pictures").FieldType));
  var bytes=new byte[iw*ih*4];
  for(int y=0;y<ih;y++)for(int x=0;x<iw;x++){int i=(y*iw+x)*4;bytes[i]=(byte)(x*31+y*19);bytes[i+1]=(byte)(x*13+y*53);bytes[i+2]=(byte)(x*71+y*29);bytes[i+3]=255;}
  var fp=(FastPixel.FastPixel)RuntimeHelpers.GetUninitializedObject(typeof(FastPixel.FastPixel));
  Set(fp,"_width",iw);Set(fp,"_height",ih);Set(fp,"_isAlpha",true);Set(fp,"rgbValues",bytes);Set(fp,"locked",true);Set(effect,"_fp",fp);
  var coords=new List<int[]>();
  if(locations)for(int x=0;x<(horizontal?height:width);x++)for(int y=0;y<(horizontal?width:height);y++)coords.Add(new[]{x+ox,y+oy});
  var frames=new List<uint[]>();
  for(int frame=0;frame<ms/frameMs;frame++) {
   IPixelFrameBuffer buf;
   if(locations)buf=new CaptureBuffer();else {var b=new PixelFrameBuffer(width,height);b.ClearBuffer();buf=b;}
   object[] data={frame,0d,0d,0d};Invoke(effect,"InitFrameData",data);Invoke(effect,"InitialRender",frame,(double)data[1]);
   int h=Get<int>(effect,"_imageHt"),w=Get<int>(effect,"_imageWi");
   if(locations)foreach(var xy in coords)Invoke(effect,"CalculatePixel",xy[0],xy[1],buf,frame,(double)data[2],(double)data[3],height,width);
   else for(int x=0;x<w;x++)for(int y=0;y<h;y++) Invoke(effect,"CalculatePixel",x,y,buf,frame,(double)data[2],(double)data[3],height,width);
   var pixels=new List<uint>();
   if(locations)foreach(var xy in coords)pixels.Add(unchecked((uint)buf.GetColorAt(xy[0],xy[1]).ToArgb()));
   else for(int x=0;x<width;x++)for(int y=0;y<height;y++) pixels.Add(unchecked((uint)buf.GetColorAt(x,y).ToArgb()));
   frames.Add(pixels.ToArray());
  }
  Cases.Add(new {type=type.ToString(),horizontal,width,height,iw,ih,direction,speed,rate,stop,custom,level,brightness,xoffset,yoffset,locations,coords,ms,frameMs,frames});
 }
}

sealed class CaptureBuffer:IPixelFrameBuffer {
 readonly Dictionary<(int,int),Color> data=new();
 public void SetPixel(int x,int y,Color c)=>data[(x,y)]=c;
 public void SetPixel(int x,int y,Common.Controls.ColorManagement.ColorModels.HSV c)=>SetPixel(x,y,c.ToRGB());
 public Color GetColorAt(int x,int y)=>data.TryGetValue((x,y),out var c)?c:Color.Black;
}
