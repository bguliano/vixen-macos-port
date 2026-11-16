// Full unchanged upstream Glediator raw RGB-file renderer.
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
using VixenModules.Effect.Glediator;
using VixenModules.Effect.Effect;
using VixenModules.App.Curves;
using VixenModules.Property.Color;
using StateEffect=VixenModules.Effect.Glediator.Glediator;
using EffectData=VixenModules.Effect.Glediator.GlediatorData;
using EffectDescriptor=VixenModules.Effect.Glediator.GlediatorDescriptor;

internal static class Program {
 static readonly JsonSerializerOptions Json = new() { IncludeFields = true };
 static readonly Dictionary<string,ElementNode> Nodes = new();
 static readonly List<object> Edges = new();
 static readonly List<object> Cases = new();
 static MemoryTarget Log;

 static string MediaRoot;
 static readonly Dictionary<string,byte[]> Files=new();
 static async Task Main(string[] args) {
  if(args.Length>0 && args[0]=="--gdi-probe") {Environment.ExitCode=GdiProbe.Run()?0:1;return;}
  CultureInfo.DefaultThreadCurrentCulture=CultureInfo.InvariantCulture;
  string outDir=args.Length>0?args[0]:"target/golden/media/data";Directory.CreateDirectory(outDir);
  Log=new MemoryTarget("oracle") {Layout="${message} ${exception:format=tostring}"};
  var logging=new LoggingConfiguration();logging.AddRule(LogLevel.Warn,LogLevel.Fatal,Log);LogManager.Configuration=logging;
  VixenSystem.UIContext=new SynchronizationContext();
  await using var runtime=await NativeVixenRuntime.StartAsync(Path.GetFullPath(Path.Combine(outDir,"profile",Guid.NewGuid().ToString("N"))));
  MediaRoot=Path.GetFullPath(Path.Combine(outDir,"Module Data"));Directory.CreateDirectory(Path.Combine(MediaRoot,"Glediator"));
  BuildTargets();GenerateCases();
  var nodes=Nodes.Values.Select(n=>new {name=n.Name,id=n.Id,element=n.Element?.Id,
   color=(n.Properties.Get(ColorDescriptor.ModuleId) as ColorModule)?.SingleColor.ToArgb(),
   colorType=(n.Properties.Get(ColorDescriptor.ModuleId) as ColorModule)?.ColorType.ToString(),colorSet=(n.Properties.Get(ColorDescriptor.ModuleId) as ColorModule)?.ColorSetName}).ToArray();
  using var file=File.Create(Path.Combine(outDir,"glediator.json.gz"));using var gzip=new GZipStream(file,CompressionLevel.SmallestSize);
  JsonSerializer.Serialize(gzip,new {nodes,edges=Edges,cases=Cases,files=Files.ToDictionary(p=>p.Key,p=>p.Value.Select(b=>(int)b).ToArray())},Json);Console.WriteLine($"{Cases.Count} Glediator cases written to {outDir}");
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
  foreach(var (w,h) in new[]{(1,1),(1,5),(2,3),(3,2),(4,4)}) {
   string name=$"grid{w}x{h}";Node(name);
   for(int x=0;x<w;x++) {Node(name+"-"+x,name);for(int y=0;y<h;y++)Node($"{name}-{x}-{y}",name+"-"+x,true);}
  }
  Node("uneven");Node("uneven-a","uneven");Node("uneven-b","uneven");
  Node("uneven-0","uneven-a",true);Node("uneven-1","uneven-b",true);Node("uneven-2","uneven-b",true);
  foreach(var (name,length) in new[]{("frames",1200),("one",3),("empty",0),("short",2),("tail",37)}) {
   var bytes=Enumerable.Range(0,length).Select(i=>(byte)((i*73+i/7*29+17)%256)).ToArray();Files[name+".gled"]=bytes;
   File.WriteAllBytes(Path.Combine(MediaRoot,"Glediator",name+".gled"),bytes);
  }
 }
 static EffectData Data()=>new() {FileName="frames.gled"};
 static void GenerateCases() {
  foreach(var target in new[]{"grid1x1","grid1x5","grid2x3","grid3x2","grid4x4","uneven","grid2x3+grid3x2","grid2x3+grid2x3"})
   foreach(var orientation in Enum.GetValues<StringOrientation>()) foreach(var movement in Enum.GetValues<MovementType>())
    foreach(var value in new[]{1,2,3,7,20,100}) {
     var d=Data();d.Orientation=orientation;d.MovementType=movement;d.Speed=value;d.Iterations=value;
     Add($"play/{target}/{orientation}/{movement}/{value}",target,1234,25,d);
    }
  foreach(var duration in new[]{0,1,49,50,51,999,1000,1234}) foreach(var frame in new[]{10,25,50,100})
   foreach(var movement in Enum.GetValues<MovementType>()) {
    var d=Data();d.MovementType=movement;Add($"timing/{duration}/{frame}/{movement}","grid2x3",duration,frame,d);
   }
  foreach(var points in new double[][][]{[[0,0],[100,100]],[[0,100],[100,0]],[[0,20],[13,85],[67,10],[100,76]],[[12,25],[85,90]]})
   foreach(var target in new[]{"grid1x1","grid2x3","uneven"}) {
    var d=Data();d.LevelCurve=new Curve(new ZedGraph.PointPairList(points.Select(p=>p[0]).ToArray(),points.Select(p=>p[1]).ToArray()));
    Add($"curve/{Cases.Count}/{target}",target,1234,25,d);
   }
  foreach(var mode in Enum.GetValues<MovementType>()) {
   var d=Data();d.MovementType=mode;d.Speed=2;Add("rerender/"+mode,"grid2x3",1234,25,d,true);
  }
  foreach(var file in new[]{"empty.gled","short.gled","one.gled"}) {
   var d=Data();d.FileName=file;Add($"invalid-file/{file}","grid2x3",1000,50,d);
  }
  foreach(var file in new[]{"missing.gled","one.gled","tail.gled",""}) {
   var d=Data();d.FileName=file;Add($"file/{file}","grid1x1",1000,50,d);
  }
 }
 static StateEffect Create(string[] targets,int duration,int frame,EffectData data) {
  VixenSystem.DefaultUpdateInterval=frame;
  var e=new StateEffect();e.TargetNodes=targets.Select(t=>(IElementNode)Nodes[t]).ToArray();e.Descriptor=new EffectDescriptor();
  GlediatorDescriptor.ModulePath=Path.Combine(MediaRoot,"Glediator");
  e.TimeSpan=TimeSpan.FromMilliseconds(duration);e.ModuleData=data;return e;
 }
 static object State(object state)=>state switch {
  LightingValue v=>new {kind="lighting",color=v.Color.ToArgb(),value=v.Intensity},
  DiscreteValue v=>new {kind="discrete",color=v.Color.ToArgb(),value=v.Intensity},
  RGBValue v=>new {kind="rgb",r=(int)v.R,g=(int)v.G,b=(int)v.B},
  _=>throw new NotSupportedException(state?.GetType().FullName)
 };
 static IEnumerable<IIntentNode> IntentNodes(EffectIntents intents,Guid id) {var list=intents?.GetIntentNodesForElement(id);if(list!=null) foreach(var i in list)yield return i;}
 static object Snapshot(StateEffect effect,string[] targets,int duration,int frame,string id) {
  Log.Logs.Clear();bool ok=effect.PreRender();var intents=effect.Render();
  if((!ok||Log.Logs.Count>0) && !(id.StartsWith("invalid-file/",StringComparison.Ordinal) && !ok && Log.Logs.Any(s=>s.Contains("DivideByZeroException",StringComparison.Ordinal)))) throw new InvalidOperationException($"Unexpected Glediator failure {id}: {string.Join("\n",Log.Logs)}");
  var times=new SortedSet<long>{0,duration*10000L};for(long t=0;t<duration;t+=frame) times.Add(t*10000L);
  foreach(var t in new[]{1L,123456L,duration*10000L/3,duration*10000L-1}) if(t>0&&t<duration*10000L)times.Add(t);
  var leaves=targets.SelectMany(t=>Nodes[t].GetLeafEnumerator()).Where(n=>n.Element!=null).DistinctBy(n=>n.Element.Id).ToArray();
  return new {ok,errors=Log.Logs.ToArray(),times=times.ToArray(),elements=leaves.Select(n=>new {name=n.Name,
   intents=IntentNodes(intents,n.Element.Id).Select(i=>new {start=i.StartTime.Ticks,end=i.EndTime.Ticks,kind=i.Intent.GetType().Name}).OrderBy(i=>i.start).ThenBy(i=>i.end).ThenBy(i=>i.kind).ToArray(),
   frames=times.Select(t=>(intents?.GetElementIntentsAtTime(n.Element.Id,new TimeSpan(t))??[]).Select(i=>State(i.Intent.GetStateAt(new TimeSpan(t)-i.StartTime))).OrderBy(s=>JsonSerializer.Serialize(s,Json),StringComparer.Ordinal).ToArray()).ToArray()}).ToArray()};
 }
 static void Add(string id,string target,int duration,int frame,EffectData data,bool rerender=false) {
  var targets=target.Split('+');
  using var edited=Create(targets,duration,frame,data);string xml=Xml(edited.ModuleData);var before=Snapshot(edited,targets,duration,frame,id);
  using var loaded=Create(targets,duration,frame,ThroughFile((EffectData)edited.ModuleData));
  object repeated=null;if(rerender) {edited.MarkDirty();repeated=Snapshot(edited,targets,duration,frame,id);}
  Cases.Add(new {effect="Glediator",id,targets,startMs=0,ms=duration,frameMs=frame,data=xml,marks=Array.Empty<object>(),edited=before,loaded=Snapshot(loaded,targets,duration,frame,id),repeated});
 }
}
