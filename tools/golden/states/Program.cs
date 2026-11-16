// Full unchanged upstream State renderer, planner, discovery, coalescer and mark models.
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
using VixenModules.Effect.State;
using VixenModules.Property.Color;
using VixenModules.Property.State;
using StateEffect = VixenModules.Effect.State.State;
using EffectData = VixenModules.Effect.State.StateData;
using PropertyData = VixenModules.Property.State.StateData;
using EffectDescriptor = VixenModules.Effect.State.StateDescriptor;
using PropertyDescriptor = VixenModules.Property.State.StateDescriptor;

internal static class Program {
 static readonly JsonSerializerOptions Json = new() { IncludeFields = true };
 static readonly Dictionary<string,ElementNode> Nodes = new();
 static readonly List<object> Edges = new();
 static readonly List<object> Cases = new();
 static MemoryTarget Log;
 static PropertyData Property;
 static StateDefinitionData Definition;
 static Guid MarkId=Guid.NewGuid();
 static async Task Main(string[] args) {
  CultureInfo.DefaultThreadCurrentCulture=CultureInfo.InvariantCulture;
  bool discreteProbe=args.Length>0 && args[0]=="--discrete-color-probe";
  string outDir=discreteProbe?(args.Length>1?args[1]:"target/golden/states/discrete-probe"):(args.Length>0?args[0]:"target/golden/states/data");Directory.CreateDirectory(outDir);
  Log=new MemoryTarget("oracle") {Layout="${message} ${exception:format=tostring}"};
  var logging=new LoggingConfiguration();logging.AddRule(LogLevel.Warn,LogLevel.Fatal,Log);LogManager.Configuration=logging;
  VixenSystem.UIContext=new SynchronizationContext();
  await using var runtime=await NativeVixenRuntime.StartAsync(Path.GetFullPath(Path.Combine(outDir,"profile",Guid.NewGuid().ToString("N"))));
  BuildTargets();
  if(discreteProbe) {
   var multi=Node("multi","root",true);var p=(ColorModule)multi.Properties.Add(ColorDescriptor.ModuleId);p.ColorType=ElementColorType.MultipleDiscreteColors;p.ColorSetName="RGB";
   Definition.Items=[Item("Named",Color.Blue,"multi"),Item("ARGB",Color.FromArgb(0,0,255),"multi")];
   foreach(var item in Definition.Items) {var d=Data();d.SelectedStateItemId=item.Id;Add("discrete-identity/"+item.Name,"root",0,1000,50,d,[]);}
  } else GenerateCases();
  var nodes=Nodes.Values.Select(n=>new {name=n.Name,id=n.Id,element=n.Element?.Id,
   color=(n.Properties.Get(ColorDescriptor.ModuleId) as ColorModule)?.SingleColor.ToArgb(),
   colorType=(n.Properties.Get(ColorDescriptor.ModuleId) as ColorModule)?.ColorType.ToString(),colorSet=(n.Properties.Get(ColorDescriptor.ModuleId) as ColorModule)?.ColorSetName,
   state=n.Properties.Get(PropertyDescriptor.ModuleId) is StateModule state?Xml(state.ModuleData):null}).ToArray();
  using var file=File.Create(Path.Combine(outDir,"states.json.gz"));using var gzip=new GZipStream(file,CompressionLevel.SmallestSize);
  JsonSerializer.Serialize(gzip,new {nodes,edges=Edges,cases=Cases,colors=ColorContracts()},Json);Console.WriteLine($"{Cases.Count} State cases written to {outDir}");
 }
 static object[] ColorContracts() {
  var colors=Enumerable.Range(27,141).Append(175).Select(i=>Color.FromKnownColor((KnownColor)i)).Concat(new[]{Color.Empty,Color.FromName("Unrecognized"),Color.FromArgb(127,20,30,40),Color.FromArgb(0,200,100,30)});
  return colors.Select(c=>new {argb=c.ToArgb(),state=Xml(new PropertyData {StateDefinitions=[new StateDefinitionData {Items=[new StateItemData {Color=c}]}]}),
   face=Xml(new VixenModules.Property.Face.FaceData {DefaultColor=c,PhonemeList=new Dictionary<string,bool>(StringComparer.OrdinalIgnoreCase) {{"AI",true},{" AI ",false},{"e",true}}})}).Cast<object>().ToArray();
 }
 static string Xml(object value) {var s=new DataContractSerializer(value.GetType());using var stream=new MemoryStream();s.WriteObject(stream,value);return Encoding.UTF8.GetString(stream.ToArray());}
 static T ThroughFile<T>(T value) {var s=new DataContractSerializer(value.GetType());using var stream=new MemoryStream();s.WriteObject(stream,value);stream.Position=0;return (T)s.ReadObject(stream);}
 static ElementNode Node(string name,string parent=null,bool leaf=false,Color? color=null) {
  var n=ElementNodeService.Instance.CreateSingle(parent==null?null:Nodes[parent],name,leaf,false);Nodes[name]=n;
  if(parent!=null) Edges.Add(new {parent,child=name});
  if(color is {} c) {var p=(ColorModule)n.Properties.Add(ColorDescriptor.ModuleId);p.ColorType=ElementColorType.SingleColor;p.SingleColor=c;}
  return n;
 }
 static StateItemData Item(string name,Color color,params string[] targets)=>new() {Name=name,Color=color,ElementNodeIds=targets.Select(t=>Nodes[t].Id).ToList()};
 static void BuildTargets() {
  Node("root");Node("rgb","root",true);Node("red","root",true,Color.Red);Node("blue","root",true,Color.Blue);
  Node("branch","root");Node("other","branch",true);Node("outside",null,true);
  VixenSystem.Nodes.AddChildToParent(Nodes["rgb"],Nodes["branch"]);Edges.Add(new {parent="branch",child="rgb"});
  Definition=new StateDefinitionData {Name="Steps",Items=[
   Item("A",Color.Red,"rgb","red"),Item("B",Color.Blue,"branch"),Item("A",Color.Green,"blue"),
   Item("",Color.FromArgb(127,200,50,20),"other"),Item(" C ",Color.Yellow,"root","rgb","outside") ]};
  Definition.Items[0].ElementNodeIds.Add(Guid.NewGuid());
  Property=ThroughFile(new PropertyData {StateDefinitions=[Definition,new StateDefinitionData{Name="Empty",Items=[]}]});
  Definition=Property.StateDefinitions[0];
  Nodes["root"].Properties.AddWithoutDefaults(new StateModule {Descriptor=new PropertyDescriptor(),ModuleData=Property});
 }
 static EffectData Data()=>new() {SelectedStateDefinitionId=Definition.Id,StatePropertyId=Property.Id,StateOwnerElementId=Nodes["root"].Id};
 static void GenerateCases() {
  foreach(var target in new[]{"root","branch","rgb","root+branch","root+root","outside","outside+root"})
   foreach(var selected in new[]{Guid.Empty,Definition.Items[0].Id,Definition.Items[2].Id,Guid.NewGuid()})
    foreach(var mode in new[]{PlaybackMode.Default,PlaybackMode.Iterate}) {
     var d=Data();d.SelectedStateItemId=selected;d.PlaybackMode=mode;d.Iterations=3;
     Add($"scope/{target}/{selected}/{mode}",target,0,1234,25,d,[]);
    }
  foreach(var duration in new[]{0,1,2,49,50,51,999,1234}) foreach(var iterations in new[]{0,1,2,3,7})
   foreach(var offset in new[]{-1,0,1,2,5,100,101}) {
    var d=Data();d.PlaybackMode=PlaybackMode.Iterate;d.Iterations=iterations;d.CycleOffset=offset;
    Add($"timing/{duration}/{iterations}/{offset}","root",0,duration,25,d,[]);
   }
  foreach(var duration in new[]{1,1234}) {
   var d=Data();d.PlaybackMode=PlaybackMode.Iterate;d.Iterations=5001;d.CycleOffset=101;
   Add($"maximum-iterations/{duration}","root",0,duration,25,d,[]);
  }
  var markSets=new (int,int,string)[][] {
   [(0,1234,"A")],[(0,1234,"A,B")],[(0,1234,"A, A, B,")],[(0,1234,"")],[(0,1234," C ")],
   [(-100,600,"B"),(400,900,"A"),(900,1400,"A,B")],[(0,300,"A"),(300,600,"A"),(600,1234,"B")],
   [(0,1234,"missing"),(0,600,"B"),(0,300,"A")],[(200,200,"A"),(500,499,"B")],[(0,500,"a"),(500,1234,"A")]
  };
  for(int i=0;i<markSets.Length;i++) foreach(var mode in new[]{PlaybackMode.Default,PlaybackMode.Iterate})
   foreach(var iterations in new[]{1,3,7}) foreach(var offset in new[]{0,1,5}) foreach(var start in new[]{0,300}) {
    var d=Data();d.RenderSource=StateRenderSource.MarkCollection;d.MarkCollectionId=MarkId;d.PlaybackMode=mode;d.Iterations=iterations;d.CycleOffset=offset;
    Add($"marks/{i}/{mode}/{iterations}/{offset}/{start}","root",start,1234,50,d,markSets[i]);
   }
  var custom=new int[][] {[],[0],[0,1,2],[0,2,1],[0,0,1],[0,-1,1],[-1],[-2,1],[3,4,0]};
  for(int i=0;i<custom.Length;i++) foreach(var mode in new[]{PlaybackMode.Default,PlaybackMode.Iterate})
   foreach(var individual in new[]{true,false}) foreach(var iterations in new[]{1,3,7}) foreach(var offset in new[]{0,1,5}) {
    var d=Data();d.RenderSource=StateRenderSource.Custom;d.PlaybackMode=mode;d.CycleIndividually=individual;d.Iterations=iterations;d.CycleOffset=offset;
    d.CustomStateItems=custom[i].Select((index,row)=>new CustomStateItemData {StateItemId=index==-1?Guid.Empty:index==-2?Guid.NewGuid():Definition.Items[index].Id,Color=row%2==0?Color.Cyan:Color.Magenta}).ToList();
    Add($"custom/{i}/{mode}/{individual}/{iterations}/{offset}","root",0,1234,25,d,[]);
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
  if(!ok||Log.Logs.Count>0) throw new InvalidOperationException($"Unexpected State failure {id}: {string.Join("\n",Log.Logs)}");
  var times=new SortedSet<long>{0,duration*10000L};for(long t=0;t<duration;t+=frame) times.Add(t*10000L);
  foreach(var t in new[]{1L,123456L,duration*10000L/3,duration*10000L-1}) if(t>0&&t<duration*10000L)times.Add(t);
  var leaves=targets.SelectMany(t=>Nodes[t].GetLeafEnumerator()).Where(n=>n.Element!=null).DistinctBy(n=>n.Element.Id).ToArray();
  return new {ok,errors=Log.Logs.ToArray(),times=times.ToArray(),elements=leaves.Select(n=>new {name=n.Name,
   intents=IntentNodes(intents,n.Element.Id).Select(i=>new {start=i.StartTime.Ticks,end=i.EndTime.Ticks,kind=i.Intent.GetType().Name}).OrderBy(i=>i.start).ThenBy(i=>i.end).ThenBy(i=>i.kind).ToArray(),
   frames=times.Select(t=>(intents?.GetElementIntentsAtTime(n.Element.Id,new TimeSpan(t))??[]).Select(i=>State(i.Intent.GetStateAt(new TimeSpan(t)-i.StartTime))).OrderBy(s=>JsonSerializer.Serialize(s,Json),StringComparer.Ordinal).ToArray()).ToArray()}).ToArray()};
 }
 static void Add(string id,string target,int start,int duration,int frame,EffectData data,(int,int,string)[] marks) {
  var mc=new MarkCollection {Id=MarkId,Name="States",CollectionType=MarkCollectionType.State};
  foreach(var (a,b,text) in marks)mc.AddMark(new Mark(TimeSpan.FromMilliseconds(a)) {Duration=TimeSpan.FromMilliseconds(b-a),Text=text});mc.EnsureOrder();
  var collections=new ObservableCollection<IMarkCollection>{mc};var targets=target.Split('+');
  using var edited=Create(targets,start,duration,frame,data,collections);string xml=Xml(edited.ModuleData);var before=Snapshot(edited,targets,duration,frame,id);
  using var loaded=Create(targets,start,duration,frame,ThroughFile((EffectData)edited.ModuleData),collections,true);
  Cases.Add(new {effect="State",id,targets,startMs=start,ms=duration,frameMs=frame,data=xml,marks=new[]{new {id=mc.Id,name=mc.Name,items=mc.Marks.Select(m=>new {start=m.StartTime.Ticks,duration=m.Duration.Ticks,text=m.Text}).ToArray()}},edited=before,loaded=Snapshot(loaded,targets,duration,frame,id)});
 }
}
