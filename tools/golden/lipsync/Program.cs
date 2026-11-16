// Full unchanged upstream LipSync FaceDefinition renderer and original mark models.
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
using VixenModules.App.Marks;
using VixenModules.Effect.LipSync;
using VixenModules.App.LipSyncApp;
using VixenModules.Property.Color;
using VixenModules.Property.Face;
using StateEffect = VixenModules.Effect.LipSync.LipSync;
using EffectData = VixenModules.Effect.LipSync.LipSyncData;
using PropertyData = VixenModules.Property.Face.FaceData;
using EffectDescriptor = VixenModules.Effect.LipSync.LipSyncDescriptor;
using PropertyDescriptor = VixenModules.Property.Face.FaceDescriptor;

internal static class Program {
 static readonly JsonSerializerOptions Json = new() { IncludeFields = true };
 static readonly Dictionary<string,ElementNode> Nodes = new();
 static readonly List<object> Edges = new();
 static readonly List<object> Cases = new();
 static MemoryTarget Log;

 static Guid MarkId=Guid.NewGuid();
 static async Task Main(string[] args) {
  CultureInfo.DefaultThreadCurrentCulture=CultureInfo.InvariantCulture;
  string outDir=args.Length>0?args[0]:"target/golden/lipsync/data";Directory.CreateDirectory(outDir);
  Log=new MemoryTarget("oracle") {Layout="${message} ${exception:format=tostring}"};
  var logging=new LoggingConfiguration();logging.AddRule(LogLevel.Warn,LogLevel.Fatal,Log);LogManager.Configuration=logging;
  VixenSystem.UIContext=new SynchronizationContext();
  await using var runtime=await NativeVixenRuntime.StartAsync(Path.GetFullPath(Path.Combine(outDir,"profile",Guid.NewGuid().ToString("N"))));
  BuildTargets();GenerateCases();
  var nodes=Nodes.Values.Select(n=>new {name=n.Name,id=n.Id,element=n.Element?.Id,
   color=(n.Properties.Get(ColorDescriptor.ModuleId) as ColorModule)?.SingleColor.ToArgb(),
   colorType=(n.Properties.Get(ColorDescriptor.ModuleId) as ColorModule)?.ColorType.ToString(),colorSet=(n.Properties.Get(ColorDescriptor.ModuleId) as ColorModule)?.ColorSetName,
   face=n.Properties.Get(PropertyDescriptor.ModuleId) is FaceModule face?Xml(face.ModuleData):null}).ToArray();
  using var file=File.Create(Path.Combine(outDir,"lipsync.json.gz"));using var gzip=new GZipStream(file,CompressionLevel.SmallestSize);
  JsonSerializer.Serialize(gzip,new {nodes,edges=Edges,cases=Cases},Json);Console.WriteLine($"{Cases.Count} LipSync cases written to {outDir}");
 }
 static string Xml(object value) {var s=new DataContractSerializer(value.GetType());using var stream=new MemoryStream();s.WriteObject(stream,value);return Encoding.UTF8.GetString(stream.ToArray());}
 static T ThroughFile<T>(T value) {var s=new DataContractSerializer(value.GetType());using var stream=new MemoryStream();s.WriteObject(stream,value);stream.Position=0;return (T)s.ReadObject(stream);}
 static ElementNode Node(string name,string parent=null,bool leaf=false,Color? color=null) {
  var n=ElementNodeService.Instance.CreateSingle(parent==null?null:Nodes[parent],name,leaf,false);Nodes[name]=n;
  if(parent!=null) Edges.Add(new {parent,child=name});
  if(color is {} c) {var p=(ColorModule)n.Properties.Add(ColorDescriptor.ModuleId);p.ColorType=ElementColorType.SingleColor;p.SingleColor=c;}
  return n;
 }
 static void Face(string name,Color color,FaceComponent[] components,string[] phonemes) {
  var data=ThroughFile(new FaceData {DefaultColor=color,FaceComponents=components.ToDictionary(c=>c,c=>true),PhonemeList=phonemes.ToDictionary(p=>p,p=>true,StringComparer.OrdinalIgnoreCase)});
  Nodes[name].Properties.AddWithoutDefaults(new FaceModule {Descriptor=new PropertyDescriptor(),ModuleData=data});
 }
 static void BuildTargets() {
  Node("root");Node("outline","root");Node("outline-rgb","outline",true);Node("outline-red","outline",true,Color.FromArgb(255,0,0));
  Face("outline",Color.FromArgb(255,0,0),[FaceComponent.Outlines],["AI","REST"]);
  Node("mouth","root");Node("mouth-rgb","mouth",true);Node("mouth-blue","mouth",true,Color.FromArgb(0,0,255));
  Face("mouth",Color.FromArgb(0,0,255),[FaceComponent.Mouth],["AI","E","REST"]);
  Node("eyes-open","root",true);Face("eyes-open",Color.Lime,[FaceComponent.EyesOpen],["E","O"]);
  Node("eyes-closed","root",true);Face("eyes-closed",Color.Magenta,[FaceComponent.EyesClosed],["REST"]);
  Node("combined","root",true);Face("combined",Color.Orange,[FaceComponent.Outlines,FaceComponent.EyesOpen,FaceComponent.EyesClosed,FaceComponent.Mouth],["AI","E","O","U","FV","L","MBP","WQ","ETC","REST","Unknown"]);
  Node("space","root",true);Face("space",Color.Cyan,[FaceComponent.Mouth],[" AI "]);
  Node("empty",null,true);Node("shared-a");Node("shared-b");
  var shared=Node("shared-leaf","shared-a",true);Face("shared-leaf",Color.Yellow,[FaceComponent.Mouth],["AI","REST"]);
  VixenSystem.Nodes.AddChildToParent(shared,Nodes["shared-b"]);Edges.Add(new {parent="shared-b",child="shared-leaf"});
 }
 static EffectData Data()=>new() {MappingType=MappingType.FaceDefinition,MarkCollectionId=MarkId};
 static void GenerateCases() {
  foreach(var target in new[]{"root","outline","mouth","mouth-rgb","empty","root+root","shared-a+shared-b"})
   foreach(var phoneme in Enum.GetValues<PhonemeType>()) foreach(var eyes in Enum.GetValues<EyeMode>()) foreach(var outline in new[]{true,false}) {
    var d=Data();d.LipSyncMode=LipSyncMode.Phoneme;d.StaticPhoneme=phoneme;d.EyeMode=eyes;d.ShowOutline=outline;
    Add($"static/{target}/{phoneme}/{eyes}/{outline}",target,0,1234,25,d,[]);
   }
  var sets=new (int,int,string)[][] {
   [],[(0,1234,"AI")],[(100,600,"AI"),(700,1000,"E")],[(0,1234," AI ")],[(0,1234,"ai")],[(0,1234,"missing")],
   [(-100,600,"E"),(400,900,"AI"),(900,1400,"REST")],[(0,300,"AI"),(300,600,"AI"),(600,1234,"E")],
   [(0,1234,"REST"),(0,600,"E"),(0,300,"AI")],[(200,200,"AI"),(500,499,"E")],
   [(10,100,"AI"),(110,200,"E"),(211,300,"O"),(310,1224,"ETC")]
  };
  for(int i=0;i<sets.Length;i++) foreach(var eyes in Enum.GetValues<EyeMode>()) foreach(var gaps in new[]{true,false}) foreach(var start in new[]{0,300}) {
   var d=Data();d.EyeMode=eyes;d.AllowMarkGaps=gaps;
   Add($"marks/{i}/{eyes}/{gaps}/{start}","root",start,1234,50,d,sets[i]);
  }
  foreach(var duration in new[]{0,1,9,10,11,25,49,50,51,1000}) foreach(var frame in new[]{10,25,50,100}) foreach(var mode in Enum.GetValues<LipSyncMode>()) {
   var d=Data();d.LipSyncMode=mode;d.StaticPhoneme=PhonemeType.REST;
   Add($"timing/{duration}/{frame}/{mode}","root",0,duration,frame,d,[]);
  }
  foreach(var type in Enum.GetValues<MarkCollectionType>()) foreach(var missing in new[]{true,false}) {
   var d=Data();d.StaticPhoneme=PhonemeType.E;if(missing)d.MarkCollectionId=Guid.NewGuid();
   Add($"collection/{type}/{missing}","root",0,1234,50,d,[(100,800,"AI")],type);
  }
 }
 static StateEffect Create(string[] targets,int start,int duration,int frame,EffectData data,ObservableCollection<IMarkCollection> marks,bool reloaded=false) {
  VixenSystem.DefaultUpdateInterval=frame;
  var e=new StateEffect();e.TargetNodes=targets.Select(t=>(IElementNode)Nodes[t]).ToArray();e.Descriptor=new EffectDescriptor();
  e.StartTime=TimeSpan.FromMilliseconds(start);e.TimeSpan=TimeSpan.FromMilliseconds(duration);if(reloaded) {e.ModuleData=data;e.MarkCollections=marks;} else {e.MarkCollections=marks;e.ModuleData=data;}return e;
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
  if(!ok||Log.Logs.Count>0) throw new InvalidOperationException($"Unexpected LipSync failure {id}: {string.Join("\n",Log.Logs)}");
  var times=new SortedSet<long>{0,duration*10000L};for(long t=0;t<duration;t+=frame) times.Add(t*10000L);
  foreach(var t in new[]{1L,123456L,duration*10000L/3,duration*10000L-1}) if(t>0&&t<duration*10000L)times.Add(t);
  var leaves=targets.SelectMany(t=>Nodes[t].GetLeafEnumerator()).Where(n=>n.Element!=null).DistinctBy(n=>n.Element.Id).ToArray();
  return new {ok,errors=Log.Logs.ToArray(),times=times.ToArray(),elements=leaves.Select(n=>new {name=n.Name,
   intents=IntentNodes(intents,n.Element.Id).Select(i=>new {start=i.StartTime.Ticks,end=i.EndTime.Ticks,kind=i.Intent.GetType().Name}).OrderBy(i=>i.start).ThenBy(i=>i.end).ThenBy(i=>i.kind).ToArray(),
   frames=times.Select(t=>(intents?.GetElementIntentsAtTime(n.Element.Id,new TimeSpan(t))??[]).Select(i=>State(i.Intent.GetStateAt(new TimeSpan(t)-i.StartTime))).OrderBy(s=>JsonSerializer.Serialize(s,Json),StringComparer.Ordinal).ToArray()).ToArray()}).ToArray()};
 }
 static void Add(string id,string target,int start,int duration,int frame,EffectData data,(int,int,string)[] marks,MarkCollectionType type=MarkCollectionType.Phoneme) {
  var mc=new MarkCollection {Id=MarkId,Name="States",CollectionType=type};
  foreach(var (a,b,text) in marks)mc.AddMark(new Mark(TimeSpan.FromMilliseconds(a)) {Duration=TimeSpan.FromMilliseconds(b-a),Text=text});mc.EnsureOrder();
  var collections=new ObservableCollection<IMarkCollection>{mc};var targets=target.Split('+');
  using var edited=Create(targets,start,duration,frame,data,collections);string xml=Xml(edited.ModuleData);var before=Snapshot(edited,targets,duration,frame,id);
  using var loaded=Create(targets,start,duration,frame,ThroughFile((EffectData)edited.ModuleData),collections,true);
  Cases.Add(new {effect="LipSync",id,targets,startMs=start,ms=duration,frameMs=frame,data=xml,marks=new[]{new {id=mc.Id,name=mc.Name,type=mc.CollectionType.ToString(),items=mc.Marks.Select(m=>new {start=m.StartTime.Ticks,duration=m.Duration.Ticks,text=m.Text}).ToArray()}},edited=before,loaded=Snapshot(loaded,targets,duration,frame,id)});
 }
}
