using System.Drawing;
using System.Text.Json;
using Vixen.Sys;
using VixenModules.SequenceType.Vixen2x;
var input=JsonSerializer.Deserialize<List<List<int[]>>>(File.ReadAllText(args[0]));
var output=new List<object>();
foreach(var events in input) {
 var sequence=new Sequence();var sm=new Vixen2xSequenceImportSM(sequence,25);sm.OpenChannel(Guid.NewGuid(),25);
 for(uint n=0;n<events.Count;n++)sm.processEvent(n,Color.FromArgb(events[(int)n][0]),(byte)events[(int)n][1]);sm.closeChannel();
 output.Add(new{events,effects=sequence.Nodes.Select(n=>new{start=(long)n.Start.TotalMilliseconds,duration=(long)n.Effect.TimeSpan.TotalMilliseconds,ramp=n.Effect.Kind.ToString().StartsWith("cbd7"),color=((Color)(n.Effect.ParameterValues.Last() is VixenModules.App.ColorGradients.ColorGradient g?g.Color:n.Effect.ParameterValues.Last())).ToArgb(),first=n.Effect.ParameterValues[0] is double d?d*255:((VixenModules.App.Curves.Curve)n.Effect.ParameterValues[0]).Points.Y[0]*2.55,last=n.Effect.ParameterValues[0] is double d2?d2*255:((VixenModules.App.Curves.Curve)n.Effect.ParameterValues[0]).Points.Y[1]*2.55}).ToArray()});
}
Console.Write(JsonSerializer.Serialize(output));
namespace NLog {public class Logger{public void Debug(string s){}public void Error(string s){}} public static class LogManager{public static Logger GetCurrentClassLogger()=>new();}}
namespace Vixen.Sys {
 public class ElementNode{} public static class VixenSystem{public static class Nodes{public static ElementNode GetElementNode(Guid id)=>new();}}
 public interface ISequence{void InsertData(EffectNode n);} public class Sequence:ISequence{public List<EffectNode> Nodes=new();public void InsertData(EffectNode n)=>Nodes.Add(n);}
 public class EffectNode{public Vixen.Module.Effect.IEffectModuleInstance Effect;public TimeSpan Start;public EffectNode(Vixen.Module.Effect.IEffectModuleInstance effect,TimeSpan start){Effect=effect;Start=start;}}
}
namespace Vixen.Module.Effect {public interface IEffectModuleInstance {Guid Kind{get;set;} Vixen.Sys.ElementNode[] TargetNodes{get;set;}TimeSpan TimeSpan{get;set;}object[] ParameterValues{get;set;}} public class Effect:IEffectModuleInstance {public Guid Kind{get;set;}public Vixen.Sys.ElementNode[] TargetNodes{get;set;}public TimeSpan TimeSpan{get;set;}public object[] ParameterValues{get;set;}}}
namespace Vixen.Services {public static class ApplicationServices {public static T Get<T>(Guid id)=>(T)(object)new Vixen.Module.Effect.Effect{Kind=id};}}
namespace ZedGraph{public class PointPairList{public double[] X,Y;public PointPairList(double[] x,double[] y){X=x;Y=y;}}}
namespace VixenModules.App.Curves{public class Curve{public ZedGraph.PointPairList Points;public Curve(ZedGraph.PointPairList p){Points=p;}}}
namespace VixenModules.App.ColorGradients{public class ColorGradient{public Color Color;public ColorGradient(Color c){Color=c;}}}
