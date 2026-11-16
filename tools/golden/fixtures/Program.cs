// Direct upstream fixture effect oracle. Model, capability selection and render sources
// compile unchanged; only never-invoked editor/thumbnail APIs are stubbed in UiStubs.cs.
using System.Globalization;
using System.Reflection;
using System.Runtime.Serialization;
using System.Text;
using System.Text.Json;
using System.IO.Compression;
using NLog;
using NLog.Config;
using NLog.Targets;
using Vixen.Commands;
using Vixen.Data.Value;
using Vixen.Module;
using Vixen.Module.Effect;
using Vixen.Native;
using Vixen.Services;
using Vixen.Sys;
using VixenModules.App.Curves;
using VixenModules.App.Fixture;
using VixenModules.Effect.Effect;
using VixenModules.Effect.Fixture;
using VixenModules.Property.IntelligentFixture;
using ZedGraph;

internal static class Program {
 static readonly JsonSerializerOptions Json = new() { IncludeFields = true };
 static readonly List<object> Nodes = new();
 static readonly Dictionary<string, ElementNode> Targets = new();
 static readonly List<object> Cases = new();
 static MemoryTarget Log;
 static IEffectModuleDescriptor[] Descriptors = [
  new VixenModules.Effect.SetPosition.SetPositionDescriptor(), new VixenModules.Effect.SetZoom.SetZoomDescriptor(),
  new VixenModules.Effect.Frost.FrostDescriptor(), new VixenModules.Effect.Gobo.GoboDescriptor(),
  new VixenModules.Effect.Prism.PrismDescriptor(), new VixenModules.Effect.FixtureStrobe.FixtureStrobeDescriptor(),
  new VixenModules.Effect.SpinColorWheel.SpinColorWheelDescriptor(), new VixenModules.Effect.LineDance.LineDanceDescriptor(),
  new VixenModules.Effect.Fixture.SetPositionDescriptor()
 ];
 static async Task Main(string[] args) {
  CultureInfo.DefaultThreadCurrentCulture = CultureInfo.InvariantCulture;
  var outDir = args.Length > 0 ? args[0] : "target/golden/fixtures/data";
  Directory.CreateDirectory(outDir);
  Log = new MemoryTarget("oracle") { Layout="${message} ${exception:format=tostring}" };
  var config = new LoggingConfiguration(); config.AddRule(NLog.LogLevel.Warn,NLog.LogLevel.Fatal,Log); LogManager.Configuration=config;
  VixenSystem.UIContext = new SynchronizationContext();
  await using var runtime = await NativeVixenRuntime.StartAsync(Path.GetFullPath(Path.Combine(outDir,"profile",Guid.NewGuid().ToString("N"))));
  BuildTargets();
  GenerateCases();
  using var file = File.Create(Path.Combine(outDir,"fixtures.json.gz"));
  using var gzip = new GZipStream(file,CompressionLevel.SmallestSize);
  JsonSerializer.Serialize(gzip,new { nodes=Nodes, cases=Cases },Json);
  Console.WriteLine($"{Cases.Count} fixture cases written to {outDir}");
 }
 static string Xml(object value) {
  var serializer=new DataContractSerializer(value.GetType()); using var stream=new MemoryStream(); serializer.WriteObject(stream,value);
  return Encoding.UTF8.GetString(stream.ToArray());
 }
 static T ThroughFile<T>(T value) {
  var serializer=new DataContractSerializer(value.GetType()); using var stream=new MemoryStream(); serializer.WriteObject(stream,value);stream.Position=0;return (T)serializer.ReadObject(stream);
 }
 static FixtureSpecification Spec(string suffix="", int offset=0) {
  var spec=new FixtureSpecification { Name="Oracle Head "+suffix, CreatedBy="fixture oracle" };
  void Add(FunctionIdentity id,FixtureFunctionType type,string name) {
   var function=new FixtureFunction { Name=name+suffix, Label=id+" label "+suffix, FunctionIdentity=id, FunctionType=type };
   function.RotationLimits.StartPosition=-90;function.RotationLimits.StopPosition=450;
   spec.FunctionDefinitions.Add(function);spec.ChannelDefinitions.Add(new FixtureChannel {Name=name+suffix,Function=name+suffix,ChannelNumber=spec.ChannelDefinitions.Count+1});
   if(type==FixtureFunctionType.Indexed) {
    var indexType=id switch {FunctionIdentity.Shutter=>FixtureIndexType.Strobe,FunctionIdentity.Prism=>FixtureIndexType.Prism,FunctionIdentity.OpenClosePrism=>FixtureIndexType.PrismOpen,_=>FixtureIndexType.Custom};
    function.IndexData.Add(new FixtureIndex {Name="Fixed", StartValue=13+offset,EndValue=20+offset,IndexType=indexType,UseCurve=false});
    function.IndexData.Add(new FixtureIndex {Name="Ranged",StartValue=64+offset,EndValue=95+offset,IndexType=indexType,UseCurve=true});
    function.IndexData.Add(new FixtureIndex {Name="Reverse",StartValue=220-offset,EndValue=160-offset,IndexType=indexType,UseCurve=true});
   }
  }
  foreach(var id in new[]{FunctionIdentity.Pan,FunctionIdentity.Tilt,FunctionIdentity.Zoom,FunctionIdentity.Dim,FunctionIdentity.Frost}) Add(id,FixtureFunctionType.Range,id.ToString());
  foreach(var id in new[]{FunctionIdentity.Gobo,FunctionIdentity.Shutter,FunctionIdentity.Prism,FunctionIdentity.OpenClosePrism,FunctionIdentity.SpinColorWheel}) Add(id,FixtureFunctionType.Indexed,id.ToString());
  Add(FunctionIdentity.Custom,FixtureFunctionType.RGBColor,"RGB");
  var wheel=new FixtureFunction {Name="Wheel"+suffix,Label="Wheel label "+suffix,FunctionType=FixtureFunctionType.ColorWheel,FunctionIdentity=FunctionIdentity.Custom};
  wheel.ColorWheelData.Add(new FixtureColorWheel {Name="Red",StartValue=4,EndValue=10,Color1=Color.Red,IndexType=FixtureIndexType.ColorWheel});
  wheel.ColorWheelData.Add(new FixtureColorWheel {Name="Spin",StartValue=101,EndValue=199,UseCurve=true,IndexType=FixtureIndexType.Custom});
  spec.FunctionDefinitions.Add(wheel);spec.ChannelDefinitions.Add(new FixtureChannel {Name=wheel.Name,Function=wheel.Name,ChannelNumber=spec.ChannelDefinitions.Count+1});
  return spec;
 }
 static ElementNode Node(string name,ElementNode parent,bool leaf,FixtureSpecification spec=null) {
  var node=ElementNodeService.Instance.CreateSingle(parent,name,leaf,false);
  if(spec!=null) {
   // Match a loaded fixture property, including DataContract setter ordering.
   spec=ThroughFile(spec);
   var property=new IntelligentFixtureModule {Descriptor=new IntelligentFixtureDescriptor(), ModuleData=new IntelligentFixtureData {FixtureSpecification=spec}};
   node.Properties.AddWithoutDefaults(property);
  }
  Nodes.Add(new {name,parent=parent?.Name,leaf,fixture=spec==null?null:Xml(new IntelligentFixtureData {FixtureSpecification=spec})});
  Targets[name]=node;return node;
 }
 static void BuildTargets() {
  foreach(int count in new[]{1,2,3,4,5,6}) {
   var group=Node("heads"+count,null,false);
   for(int i=0;i<count;i++) Node(group.Name+"-"+i,group,true,Spec());
  }
  var heterogeneous=Node("heterogeneous",null,false);
  Node("heterogeneous-a",heterogeneous,true,Spec());Node("heterogeneous-b",heterogeneous,true,Spec(" B",7));Node("heterogeneous-c",heterogeneous,true,Spec("",15));
  var whitespace=Node("whitespace",null,false);Node("whitespace-a",whitespace,true,Spec(" "));
  var shared=Node("shared",null,false);var a=Node("shared-a",shared,false);var b=Node("shared-b",shared,false);
  var sharedLeaf=Node("shared-leaf",a,true,Spec());Node("shared-other",b,true,Spec());
  VixenSystem.Nodes.AddChildToParent(sharedLeaf,b);Nodes.Add(new {name=sharedLeaf.Name,parent=b.Name,shared=sharedLeaf.Name});
  var plain=Node("plain",null,false);Node("plain-a",plain,true);
  var partial=Spec();partial.ChannelDefinitions.RemoveAll(c=>c.Function=="Tilt"||c.Function=="Gobo");
  var mixed=Node("mixed",null,false);Node("mixed-a",mixed,true,partial);Node("mixed-b",mixed,true);Node("mixed-c",mixed,true,Spec());
 }
 static Curve Curve(double[][] points) => new(new PointPairList(points.Select(p=>p[0]).ToArray(),points.Select(p=>p[1]).ToArray()));
 static object ConvertValue(object value, Type type) => type.IsEnum ? Enum.Parse(type,(string)value) : type==typeof(Curve) ? Curve((double[][])value) : Convert.ChangeType(value,type,CultureInfo.InvariantCulture);

 static readonly double[][] Ramp = [[0,0],[100,100]];
 static readonly double[][] Bent = [[0,20],[13,85],[67,10],[100,76]];
 static readonly double[][] Interior = [[12,25],[85,90]];
 static void GenerateCases() {
  foreach(var descriptor in Descriptors) {
   foreach(var target in new[]{"heads1","heads2","heads3","heads4","heads5","heads6","heterogeneous","mixed","plain","heads2+heads3","plain+heads3","heads3+plain","heads2+heads2","shared","whitespace"})
    Add(descriptor.EffectName,"target/"+target,target,1000,50,[]);
   foreach(var duration in new[]{0,1,49,50,51,99,999,1250}) foreach(var frame in new[]{10,25,50,100})
    Add(descriptor.EffectName,$"timing/{duration}/{frame}","heads3",duration,frame,[]);
  }
  foreach(var (effect,parameter) in new[]{("Set Position","Pan"),("Set Zoom","Zoom"),("Frost","Frost"),("Line Dance","IncrementAngle")}) {
   var curves=new double[][][]{[],[[50,25]],[[0,-20],[100,120]],[[0,10],[30,50],[100,120]],[[0,120],[100,-20]],[[120,20],[150,60]]};
   for(int i=0;i<curves.Length;i++) Add(effect,$"edge-curve/{i}","heads3",1234,25,[(parameter,curves[i])]);
  }
  foreach(var (effect,prefix,function) in new[]{("Gobo","Gobo","Gobo"),("Prism","Prism","Prism"),("Fixture Strobe","FixtureStrobe","Shutter"),("Spin Color Wheel","SpinColorWheel","SpinColorWheel")})
   foreach(var index in new[]{"Fixed","Ranged"}) foreach(var curve in new double[][][]{[],[[50,25]]})
    Add(effect,$"edge-index/{index}/{curve.Length}","heads3",1234,25,[(prefix+"FunctionName",function),(prefix+"IndexValue",index),(prefix+"Curve",curve)]);
  foreach(var curve in new double[][][]{[],[[50,25]]}) {
   Add("Spin Color Wheel",$"edge-intensity/{curve.Length}","heads3",1234,25,[("Intensity",curve)]);
   foreach(var mode in new[]{"Stagger","Concurrent"}) Add("Line Dance",$"edge-mode/{mode}/{curve.Length}","heads5",1234,25,[("FanMode",mode),("IncrementAngle",curve)]);
   foreach(var name in new[]{"Pan","Gobo","RGB","Wheel"})
    Add("Fixture",$"edge-row/{name}/{curve.Length}","heads3",1234,25,[],effect=>{
     var spec=Spec();var function=spec.FunctionDefinitions.Single(f=>f.Name==name);
     effect.ModuleData=new FixtureData {FixtureFunctions=spec.FunctionDefinitions,FunctionData=[new FixtureFunctionData {
      FunctionName=name,FunctionType=function.FunctionType,FunctionIdentity=function.FunctionIdentity,
      Range=Curve(curve),IndexValue=function.FunctionType==FixtureFunctionType.Indexed?"Ranged":null,ColorIndexValue=function.FunctionType==FixtureFunctionType.ColorWheel?"Spin":null,Color=new VixenModules.App.ColorGradients.ColorGradient(Color.Red)
     }]};
    });
  }
  foreach(var name in new[]{"Frost","Gobo","RGB","Wheel"})
   Add("Fixture",$"edge-multi-row/{name}","heads3",1234,25,[],effect=>{
    var spec=Spec();var rows=new List<FixtureFunctionData>();
    foreach(var rowName in new[]{"Pan",name,"Tilt"}) {
     var function=spec.FunctionDefinitions.Single(f=>f.Name==rowName);
     rows.Add(new FixtureFunctionData { FunctionName=rowName,FunctionType=function.FunctionType,FunctionIdentity=function.FunctionIdentity,
      Range=Curve(rowName==name?[]:Ramp),IndexValue=function.FunctionType==FixtureFunctionType.Indexed?"Ranged":null,
      ColorIndexValue=function.FunctionType==FixtureFunctionType.ColorWheel?"Spin":null,Color=new VixenModules.App.ColorGradients.ColorGradient(Color.Red)});
    }
    effect.ModuleData=new FixtureData {FixtureFunctions=spec.FunctionDefinitions,FunctionData=rows};
   });
  foreach(var target in new[]{"heads3","heterogeneous","mixed","heads2+heads3"}) {
   foreach(var curve in new[]{Ramp,Bent,Interior,new double[][]{[100,0],[0,100]},new double[][]{[0,25],[50,25],[50,75],[100,75]}}) {
    var name=Array.IndexOf(new[]{Ramp,Bent,Interior},curve);if(name<0) name=Cases.Count;
    Add("Set Position",$"curves/{target}/{name}",target,1234,25,[("Pan",curve),("Tilt",Bent)]);
    Add("Set Zoom",$"curves/{target}/{name}",target,1234,25,[("Zoom",curve)]);
    Add("Frost",$"curves/{target}/{name}",target,1234,25,[("Frost",curve)]);
   }
   foreach(bool pan in new[]{false,true}) foreach(bool tilt in new[]{false,true})
    Add("Set Position",$"enable/{target}/{pan}/{tilt}",target,1200,50,[("EnablePan",pan),("EnableTilt",tilt)]);
  }
  foreach(var (effect,prefix,function) in new[]{("Gobo","Gobo","Gobo"),("Prism","Prism","Prism"),("Fixture Strobe","FixtureStrobe","Shutter"),("Spin Color Wheel","SpinColorWheel","SpinColorWheel")})
   foreach(var target in new[]{"heads3","heterogeneous","mixed","heads2+heads3"})
    foreach(var index in new[]{"Fixed","Ranged","Reverse"}) foreach(var curve in new[]{Ramp,Bent,Interior}) {
     Add(effect,$"index/{target}/{index}/{Array.IndexOf(new[]{Ramp,Bent,Interior},curve)}",target,1234,25,[(prefix+"FunctionName",function),(prefix+"IndexValue",index),(prefix+"Curve",curve)]);
    }
  foreach(var count in new[]{1,2,3,4,5,6}) foreach(var mode in new[]{"Synchronized","Stagger","Concurrent"})
   foreach(var direction in new[]{"FanFromEdges","FanFromCenter"}) foreach(var center in new[]{"Centered","Left","Right"}) foreach(bool invert in new[]{false,true})
    Add("Line Dance",$"fan/{count}/{mode}/{direction}/{center}/{invert}","heads"+count,1234,25,[("FanMode",mode),("FanDirection",direction),("CenterHandling",center),("InvertPan",invert)]);
  foreach(var mode in new[]{"Synchronized","Stagger","Concurrent"}) foreach(var parameter in new[]{"HoldTime","PanStartAngle","PanIncrement","PanSpeed"}) foreach(var value in new[]{0,1,25,75,99,100})
   Add("Line Dance",$"override/{mode}/{parameter}/{value}","heads5",1234,25,[("FanMode",mode),("AdvancedOverrides",true),(parameter,value),("IncrementAngle",Bent)]);
  foreach(var target in new[]{"heads3","heterogeneous","mixed","heads2+heads3"}) foreach(var name in new[]{"Pan","Tilt","Zoom","Frost","Gobo","Shutter","Prism","SpinColorWheel","RGB","Wheel"})
   foreach(var index in new[]{"Fixed","Ranged"}) {
    Add("Fixture",$"row/{target}/{name}/{index}",target,1234,25,[],effect=>{
     var spec=Spec();var function=spec.FunctionDefinitions.Single(f=>f.Name==name);
     var data=new FixtureData {FixtureFunctions=spec.FunctionDefinitions,FunctionData=[new FixtureFunctionData {
      FunctionName=name,FunctionType=function.FunctionType,FunctionIdentity=function.FunctionIdentity,
      Range=Curve(Bent),IndexValue=function.FunctionType==FixtureFunctionType.Indexed?index:null,ColorIndexValue=function.FunctionType==FixtureFunctionType.ColorWheel?(index=="Fixed"?"Red":"Spin"):null,Color=new VixenModules.App.ColorGradients.ColorGradient(Color.Red)
     }]};effect.ModuleData=data;
    });
   }
 }

 static IEffectModuleInstance Create(IEffectModuleDescriptor descriptor,string[] targets,int ms,int frameMs) {
  VixenSystem.DefaultUpdateInterval=frameMs;
  var effect=(IEffectModuleInstance)Activator.CreateInstance(descriptor.ModuleClass);
  effect.ModuleData=(IModuleDataModel)Activator.CreateInstance(descriptor.ModuleDataClass);
  effect.TimeSpan=TimeSpan.FromMilliseconds(ms);effect.StartTime=TimeSpan.Zero;effect.TargetNodes=targets.Select(t=>(IElementNode)Targets[t]).ToArray();
  effect.Descriptor=descriptor;effect.MarkCollections=new();return effect;
 }
 static object State(object state) => state switch {
  RangeValue<FunctionIdentity> v => new {kind="range", value=v.Value, identity=v.TagType.ToString(),tag=v.Tag,label=v.Label},
  CommandValue v when v.Command is Named8BitCommand<FixtureIndexType> c => new {kind="named8", value=(int)c.CommandValue, index=c.IndexType.ToString(),tag=c.Tag,label=c.Label,min=(int)c.RangeMinimum,max=(int)c.RangeMaximum},
  LightingValue v => new {kind="lighting",color=v.Color.ToArgb(),value=v.Intensity},
  RGBValue v => new {kind="rgb",r=(int)v.R,g=(int)v.G,b=(int)v.B},
  DiscreteValue v=>new {kind="discrete",color=v.Color.ToArgb(),value=v.Intensity},
  _=>throw new NotSupportedException(state?.GetType().FullName)
 };
 static IEnumerable<IIntentNode> IntentNodes(EffectIntents intents,Guid id) {
  var nodes=intents?.GetIntentNodesForElement(id);if(nodes!=null) foreach(var node in nodes) yield return node;
 }
 static object Snapshot(IEffectModuleInstance effect,string[] targets,int ms,int frameMs,string id) {
  Log.Logs.Clear();bool ok=effect.PreRender();var intents=effect.Render();
  if ((!ok || Log.Logs.Count>0) && !(id.StartsWith("edge-",StringComparison.Ordinal) && !ok && Log.Logs.Any(s=>s.Contains("Error: Not enough points in curve to interpolate",StringComparison.Ordinal)) && Log.Logs.All(s=>s.StartsWith("Error rendering Effect Property settings:",StringComparison.Ordinal) || s.Contains("Error: Not enough points in curve to interpolate",StringComparison.Ordinal))))
   throw new InvalidOperationException($"Unexpected render failure in {effect.EffectName}/{id}: {string.Join("\n",Log.Logs)}");
  var times=new SortedSet<long>{0,ms*10000L};
  for(long t=0;t<ms;t+=frameMs) times.Add(t*10000);
  foreach(var t in new[]{1L,123456L,ms*10000L/3,ms*10000L-1}) if(t>0 && t<ms*10000L) times.Add(t);
  var elements=targets.SelectMany(t=>Targets[t].GetLeafEnumerator()).Where(n=>n.Element!=null).DistinctBy(n=>n.Element.Id).ToArray();
  var rows=elements.Select(n=>new {name=n.Name,
   intents=IntentNodes(intents,n.Element.Id).Select(i=>new {start=i.StartTime.Ticks,end=i.EndTime.Ticks,kind=i.Intent.GetType().Name}).OrderBy(i=>i.start).ThenBy(i=>i.end).ThenBy(i=>i.kind).ToArray(),
   frames=times.Select(t=>(intents?.GetElementIntentsAtTime(n.Element.Id,new TimeSpan(t))??[]).Select(i=>State(i.Intent.GetStateAt(new TimeSpan(t)-i.StartTime))).OrderBy(s=>JsonSerializer.Serialize(s,Json),StringComparer.Ordinal).ToArray()).ToArray()}).ToArray();
  return new {ok,errors=Log.Logs.ToArray(),times=times.ToArray(),elements=rows};
 }
 static void Add(string effectName,string id,string target,int ms,int frameMs,(string,object)[] parameters,Action<IEffectModuleInstance> configure=null) {
  var descriptor=Descriptors.Single(d=>d.EffectName==effectName);var targets=target.Split('+');
  using var effect=Create(descriptor,targets,ms,frameMs);
  foreach(var (name,value) in parameters) {var property=effect.GetType().GetProperty(name)??throw new Exception(name);property.SetValue(effect,ConvertValue(value,property.PropertyType));}
  configure?.Invoke(effect);
  string data=Xml(effect.ModuleData);var edited=Snapshot(effect,targets,ms,frameMs,id);
  using var loaded=Create(descriptor,targets,ms,frameMs);loaded.ModuleData=ThroughFile(effect.ModuleData);
  Cases.Add(new {effect=effectName,id,targets,ms,frameMs,data,configured=configure!=null,parameters=parameters.Select(p=>new {name=p.Item1,value=p.Item2}).ToArray(),edited,loaded=Snapshot(loaded,targets,ms,frameMs,id)});
 }
}
