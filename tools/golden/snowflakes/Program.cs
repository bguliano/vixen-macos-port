// Full unchanged upstream Snowflakes renderer with a controlled shared random stream.
using System.Globalization;
using System.Runtime.Serialization;
using System.Text;
using System.Text.Json;
using System.IO.Compression;
using System.Collections.ObjectModel;
using NLog;
using NLog.Config;
using NLog.Targets;
using Vixen.Data.Value;
using Vixen.Marks;
using Vixen.Module.Effect;
using Vixen.Native;
using Vixen.Services;
using Vixen.Sys;
using VixenModules.Effect.Snowflakes;
using System.Reflection;
using VixenGolden;
using VixenModules.Effect.Effect;
using VixenModules.App.Curves;
using VixenModules.Property.Color;
using StateEffect=VixenModules.Effect.Snowflakes.Snowflakes;
using EffectData=VixenModules.Effect.Snowflakes.SnowflakesData;
using EffectDescriptor=VixenModules.Effect.Snowflakes.SnowflakesDescriptor;

internal static class Program {
 static readonly JsonSerializerOptions Json = new() { IncludeFields = true };
 static readonly Dictionary<string,ElementNode> Nodes = new();
 static readonly List<object> Edges = new();
 static readonly List<object> Cases = new();
 static MemoryTarget Log;


 static async Task Main(string[] args) {
  CultureInfo.DefaultThreadCurrentCulture=CultureInfo.InvariantCulture;
  string outDir=args.Length>0?args[0]:"target/golden/snowflakes/data";Directory.CreateDirectory(outDir);
  Log=new MemoryTarget("oracle") {Layout="${message} ${exception:format=tostring}"};
  var logging=new LoggingConfiguration();logging.AddRule(LogLevel.Warn,LogLevel.Fatal,Log);LogManager.Configuration=logging;
  VixenSystem.UIContext=new SynchronizationContext();
  await using var runtime=await NativeVixenRuntime.StartAsync(Path.GetFullPath(Path.Combine(outDir,"profile",Guid.NewGuid().ToString("N"))));
  BuildTargets();GenerateCases();
  var nodes=Nodes.Values.Select(n=>new {name=n.Name,id=n.Id,element=n.Element?.Id,
   color=(n.Properties.Get(ColorDescriptor.ModuleId) as ColorModule)?.SingleColor.ToArgb(),
   colorType=(n.Properties.Get(ColorDescriptor.ModuleId) as ColorModule)?.ColorType.ToString(),colorSet=(n.Properties.Get(ColorDescriptor.ModuleId) as ColorModule)?.ColorSetName}).ToArray();
  using var file=File.Create(Path.Combine(outDir,"snowflakes.json.gz"));using var gzip=new GZipStream(file,CompressionLevel.SmallestSize);
  JsonSerializer.Serialize(gzip,new {nodes,edges=Edges,cases=Cases},Json);Console.WriteLine($"{Cases.Count} Snowflakes cases written to {outDir}");
 }
 static string Xml(object value) {var s=new DataContractSerializer(value.GetType());using var stream=new MemoryStream();s.WriteObject(stream,value);return Encoding.UTF8.GetString(stream.ToArray());}
 static T ThroughFile<T>(T value) {var s=new DataContractSerializer(value.GetType());using var stream=new MemoryStream();s.WriteObject(stream,value);stream.Position=0;return (T)s.ReadObject(stream);}
 static ElementNode Node(string name,string parent=null,bool leaf=false,Color? color=null) {
  var n=ElementNodeService.Instance.CreateSingle(parent==null?null:Nodes[parent],name,leaf,false);Nodes[name]=n;
  if(parent!=null) Edges.Add(new {parent,child=name});
  if(color is {} c) {var p=(ColorModule)n.Properties.Add(ColorDescriptor.ModuleId);p.ColorType=ElementColorType.SingleColor;p.SingleColor=c;}
  return n;
 }
 static void BuildTargets() {
  foreach(var (w,h) in new[]{(1,1),(2,3),(8,8),(12,16),(21,23)}) {
   string name=$"grid{w}x{h}";Node(name);
   for(int x=0;x<w;x++) {Node(name+"-"+x,name);for(int y=0;y<h;y++)Node($"{name}-{x}-{y}",name+"-"+x,true);}
  }
 }
 static EffectData Data()=>new();
 static Curve Flat(double value)=>new(new ZedGraph.PointPairList(new[]{0.0,100.0},new[]{value,value}));
 static void GenerateCases() {
  foreach(var target in new[]{"grid1x1","grid2x3","grid8x8","grid12x16"}) foreach(var seed in new ulong[]{0,42})
   foreach(var type in Enum.GetValues<SnowflakeType>()) {
    var d=Data();d.SnowflakeType=type;Add($"shape/{target}/{type}/{seed}",target,1000,50,d,seed);
   }
  foreach(var effect in Enum.GetValues<SnowflakeEffect>()) foreach(var movement in Enum.GetValues<SnowFlakeMovement>())
   foreach(var orientation in Enum.GetValues<StringOrientation>()) foreach(var seed in new ulong[]{1,123456}) {
    var d=Data();d.SnowflakeEffect=effect;d.SnowFlakeMovement=movement;d.Orientation=orientation;
    Add($"movement/{effect}/{movement}/{orientation}/{seed}","grid12x16",1234,25,d,seed);
   }
  foreach(var color in Enum.GetValues<SnowflakeColorType>()) foreach(var fade in Enum.GetValues<FadeType>())
   foreach(var cycle in new[]{false,true}) foreach(var brightness in new[]{false,true}) {
    var d=Data();d.ColorType=color;d.FadeType=fade;d.CycleColor=cycle;d.RandomBrightness=brightness;
    d.InnerColor=[new VixenModules.App.ColorGradients.ColorGradient(Color.Red),new VixenModules.App.ColorGradients.ColorGradient(Color.Lime)];
    d.OutSideColor=[new VixenModules.App.ColorGradients.ColorGradient(Color.Blue),new VixenModules.App.ColorGradients.ColorGradient(Color.Orange)];
    Add($"color/{color}/{fade}/{cycle}/{brightness}","grid12x16",1234,25,d,42);
   }
  foreach(var initial in new[]{0,25,50,100}) foreach(var speed in new[]{0,100}) foreach(var type in new[]{SnowflakeType.Single,SnowflakeType.FortyFive}) {
   var d=Data();d.SnowBuildUp=true;d.InitialBuildUp=initial;d.BuildUpSpeedCurve=Flat(speed);d.SnowflakeType=type;
   Add($"buildup/{initial}/{speed}/{type}","grid12x16",1500,25,d,17);
  }
  foreach(var parameter in new[]{"FlakeCountCurve","LevelCurve","SpeedVariationCurve","CenterSpeedCurve","XCenterSpeedCurve","YCenterSpeedCurve","XSpeedVariationCurve","YSpeedVariationCurve","WobbleCurve","WobbleVariationCurve","FadeSpeed"}) foreach(var value in new[]{0,50,100}) {
   var d=Data();typeof(EffectData).GetProperty(parameter).SetValue(d,Flat(value));d.SnowFlakeMovement=SnowFlakeMovement.Wobble2;d.FadeType=FadeType.InOut;
   Add($"curve/{parameter}/{value}","grid8x8",1234,25,d,42);
  }
  foreach(var duration in new[]{0,1,49,50,51,999}) foreach(var frame in new[]{10,25,50,100})
   Add($"timing/{duration}/{frame}","grid8x8",duration,frame,Data(),123456);
  foreach(var effect in new[]{SnowflakeEffect.Grid,SnowflakeEffect.GridOffset}) foreach(var count in new[]{1,4,10}) {
   var d=Data();d.SnowflakeEffect=effect;d.HFlakeCount=count;d.VFlakeCount=count;d.PointFlake45=true;
   Add($"grid/{effect}/{count}","grid21x23",1234,25,d,1);
  }

 }
 static StateEffect Create(string[] targets,int duration,int frame,EffectData data) {
  VixenSystem.DefaultUpdateInterval=frame;
  var e=new StateEffect();e.TargetNodes=targets.Select(t=>(IElementNode)Nodes[t]).ToArray();e.Descriptor=new EffectDescriptor();
  e.TimeSpan=TimeSpan.FromMilliseconds(duration);e.ModuleData=data;return e;
 }
 static object State(object state)=>state switch {
  LightingValue v=>new {kind="lighting",color=v.Color.ToArgb(),value=v.Intensity},
  DiscreteValue v=>new {kind="discrete",color=v.Color.ToArgb(),value=v.Intensity},
  RGBValue v=>new {kind="rgb",r=(int)v.R,g=(int)v.G,b=(int)v.B},
  _=>throw new NotSupportedException(state?.GetType().FullName)
 };
 static IEnumerable<IIntentNode> IntentNodes(EffectIntents intents,Guid id) {var list=intents?.GetIntentNodesForElement(id);if(list!=null) foreach(var i in list)yield return i;}
 static object Snapshot(StateEffect effect,string[] targets,int duration,int frame,string id,ulong seed) {
  var field=typeof(ThreadSafeRandom).GetField("ThreadRandom",BindingFlags.NonPublic|BindingFlags.Static);
  var random=new GoldenRandom(seed,1);((ThreadLocal<Random>)field.GetValue(null)).Value=random;
  Log.Logs.Clear();bool ok=effect.PreRender();var intents=effect.Render();
  if(!ok || Log.Logs.Count>0) throw new InvalidOperationException($"Unexpected Snowflakes failure {id}: {string.Join("\n",Log.Logs)}");
  var times=new SortedSet<long>{0,duration*10000L};for(long t=0;t<duration;t+=frame) times.Add(t*10000L);
  foreach(var t in new[]{1L,123456L,duration*10000L/3,duration*10000L-1}) if(t>0&&t<duration*10000L)times.Add(t);
  var leaves=targets.SelectMany(t=>Nodes[t].GetLeafEnumerator()).Where(n=>n.Element!=null).DistinctBy(n=>n.Element.Id).ToArray();
  return new {ok,errors=Log.Logs.ToArray(),randomDraws=random.Draws,times=times.ToArray(),elements=leaves.Select(n=>new {name=n.Name,
   intents=IntentNodes(intents,n.Element.Id).Select(i=>new {start=i.StartTime.Ticks,end=i.EndTime.Ticks,kind=i.Intent.GetType().Name}).OrderBy(i=>i.start).ThenBy(i=>i.end).ThenBy(i=>i.kind).ToArray(),
   frames=times.Select(t=>(intents?.GetElementIntentsAtTime(n.Element.Id,new TimeSpan(t))??[]).Select(i=>State(i.Intent.GetStateAt(new TimeSpan(t)-i.StartTime))).OrderBy(s=>JsonSerializer.Serialize(s,Json),StringComparer.Ordinal).ToArray()).ToArray()}).ToArray()};
 }
 static void Add(string id,string target,int duration,int frame,EffectData data,ulong seed) {
  var targets=target.Split('+');
  using var edited=Create(targets,duration,frame,data);string xml=Xml(edited.ModuleData);var before=Snapshot(edited,targets,duration,frame,id,seed);
  using var loaded=Create(targets,duration,frame,ThroughFile((EffectData)edited.ModuleData));
  Cases.Add(new {effect="Snowflakes",id,seed,targets,startMs=0,ms=duration,frameMs=frame,data=xml,marks=Array.Empty<object>(),edited=before,loaded=Snapshot(loaded,targets,duration,frame,id,seed)});
 }
}
