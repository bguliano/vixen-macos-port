// Only non-raster helpers are invoked. The unchanged original effect is compiled in full.
using System.Globalization;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Text.Json;
using System.IO.Compression;
using VixenGolden;
using VixenModules.Effect.CountDown;
using VixenModules.Effect.Effect;
using Original=VixenModules.Effect.CountDown.CountDown;

internal static class Program {
 static readonly BindingFlags Flags=BindingFlags.Instance|BindingFlags.NonPublic;
 static readonly List<object> Cases=new();
 static FieldInfo Field(Type t,string name) {
  for(;t!=null;t=t.BaseType) {var f=t.GetField(name,Flags);if(f!=null)return f;}
  throw new MissingFieldException(name);
 }
 static void Set(object target,string name,object value)=>Field(target.GetType(),name).SetValue(target,value);
 static T Get<T>(object target,string name)=>(T)Field(target.GetType(),name).GetValue(target);
 static object Invoke(object target,string name,params object[] args)=>typeof(Original).GetMethod(name,Flags).Invoke(target,args);
 static void Main(string[] args) {
  CultureInfo.DefaultThreadCurrentCulture=CultureInfo.InvariantCulture;
  foreach(var type in Enum.GetValues<CountDownType>()) foreach(var fade in Enum.GetValues<CountDownFade>())
   foreach(bool per in new[]{false,true}) foreach(int frame in new[]{10,25,33,50,100})
    Add($"timing/{type}/{fade}/{per}/{frame}",type,fade,per,2537,frame,"10",false,3,CountDownDirection.Random,42);
  foreach(string count in new[]{"-32768","-1","0","1","59","60","61","3599","3600","32767"," +12 "})
   foreach(var type in new[]{CountDownType.CountDown,CountDownType.CountUp}) foreach(var fade in Enum.GetValues<CountDownFade>())
    Add($"numbers/{type}/{fade}/{count}",type,fade,true,3000,50,count,true,1,CountDownDirection.None,0);
  foreach(int duration in new[]{1,24,25,49,50,51,99,999,1000,1001,1025,1234,1999,2000})
   foreach(var type in Enum.GetValues<CountDownType>()) foreach(var fade in Enum.GetValues<CountDownFade>())
    Add($"boundary/{type}/{fade}/{duration}",type,fade,true,duration,25,"0",false,1,CountDownDirection.Random,7);
  foreach(var direction in Enum.GetValues<CountDownDirection>()) foreach(int speed in new[]{0,1,5,20})
   Add($"direction/{direction}/{speed}",CountDownType.Effect,CountDownFade.None,false,1500,50,"10",false,speed,direction,123);
  Add("spinner/every-millisecond",CountDownType.Effect,CountDownFade.None,false,1000,1,"10",false,1,CountDownDirection.None,0);
  string outDir=args.Length>0?args[0]:"target/golden/countdown/data";Directory.CreateDirectory(outDir);
  using var file=File.Create(Path.Combine(outDir,"countdown-math.json.gz"));using var gz=new GZipStream(file,CompressionLevel.SmallestSize);
  JsonSerializer.Serialize(gz,new {cases=Cases});Console.WriteLine($"{Cases.Count} non-raster CountDown cases written to {outDir}");
 }
 static void Add(string label,CountDownType type,CountDownFade fade,bool per,int ms,int frameMs,string count,bool minutes,int speed,CountDownDirection direction,ulong seed) {
  // Constructors allocate GDI fonts. Skip them and initialize exactly the fields read by these helpers.
  var d=(CountDownData)RuntimeHelpers.GetUninitializedObject(typeof(CountDownData));
  d.CountDownType=type;d.CountDownFade=fade;d.PerIteration=per;d.CountDownTime=count;d.TimeFormat=minutes?TimeFormat.Minutes:TimeFormat.Seconds;
  d.Speed=speed;d.Direction=direction;
  var effect=(Original)RuntimeHelpers.GetUninitializedObject(typeof(Original));
  Set(effect,"_data",d);Set(effect,"FrameTime",frameMs);Set(effect,"_timeSpan",TimeSpan.FromMilliseconds(ms));
  Set(effect,"_fps",1000d/frameMs);Set(effect,"_countDownNumberIteration",-1);Set(effect,"_direction",direction);Set(effect,"_sizeAdjust",0);
  var random=typeof(ThreadSafeRandom).GetField("ThreadRandom",BindingFlags.Static|BindingFlags.NonPublic);
  ((ThreadLocal<Random>)random.GetValue(null)).Value=new GoldenRandom(seed,1);
  var frames=new List<object>();
  for(int i=0;i<ms/frameMs;i++) {
   Invoke(effect,"SetFadePositionLevel",i);
   frames.Add(new {frame=i,text=Get<string>(effect,"_text"),position=Get<double>(effect,"_directionPosition").ToString("R"),fade=Get<double>(effect,"_fade").ToString("R"),
    iteration=Get<int>(effect,"_countDownNumberIteration"),previous=Get<int>(effect,"_previousCountDownNumber"),direction=(int)Get<CountDownDirection>(effect,"_direction"),
    size=Get<int>(effect,"_sizeAdjust"),angle=(int)Invoke(effect,"GetFilmSpinnerRadialLineAngle",i)});
  }
  Cases.Add(new {label,type=type.ToString(),fade=fade.ToString(),per,ms,frameMs,count,minutes,speed,direction=direction.ToString(),seed,frames});
 }
}
