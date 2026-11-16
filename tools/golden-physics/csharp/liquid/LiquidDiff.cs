// Runs the real Liquid.cs against a recording stand-in for the C++/CLI wrapper: what goes to the physics each step is
// what the Rust side has to match. The physics itself is LiquidFun on both sides.
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Drawing;
using System.Text;
using System.Text.Json;
using Liquid;
using NLog.Targets;
using Vixen.Marks;
using Vixen.Module.Media;
using Vixen.Module.Timing;
using Vixen.Sys;
using VixenModules.App.ColorGradients;
using VixenModules.App.Curves;
using VixenModules.Effect.Effect;
using VixenModules.Effect.Liquid;
using VixenModules.Media.Audio;

namespace VixenModules.Effect.Liquid
{
	public enum WrapperParticleType { Elastic, Powder, Tensile, Spring, Viscous, StaticPressure, Water, Reactive, Repulsive }

	public class EmitterWrapper
	{
		public int X { get; set; }
		public int Y { get; set; }
		public Color Color { get; set; }
		public int Brightness { get; set; }
		public int Lifetime { get; set; }
		public int Velocity { get; set; }
		public int SourceSize { get; set; }
		public int Flow { get; set; }
		public WrapperParticleType ParticleType { get; set; }
		public int Angle { get; set; }
	}

	public class LiquidFunWrapper
	{
		[ThreadStatic] public static StringBuilder Log;

		public void Initialize(int width, int height, bool bottom, bool top, bool left, bool right, int particleSize, int lifetime)
		{
			Log?.Append($"init {width} {height} {bottom} {top} {left} {right} {particleSize} {lifetime}\t");
		}

		public void FinalizeWorld() { }

		public Tuple<int, int, Color>[] GetParticles() => Array.Empty<Tuple<int, int, Color>>();

		public void StepWorld(float stepSeconds, bool mixColors, EmitterWrapper[] emitters)
		{
			Log?.Append($"{stepSeconds:R} {mixColors}");
			foreach (var e in emitters)
				Log?.Append($";{e.X},{e.Y},{e.Color.R},{e.Color.G},{e.Color.B},{e.Lifetime},{e.Velocity},{e.SourceSize},{e.Flow},{(int)e.ParticleType},{e.Angle}");
			Log?.Append('|');
		}
	}
}

namespace VixenModules.Media.Audio
{
	public class Audio : MediaModuleInstanceBase
	{
		public float[] Samples = Array.Empty<float>();
		public float Frequency { get; set; }
		public int Channels { get; set; } = 1;
		public TimeSpan Position => TimeSpan.Zero;
		public double[] GetMonoSamples(int startSample, int numSamples)
		{
			var buffer = new double[numSamples];
			for (var i = 0; i < numSamples && startSample + i < Samples.Length; i++)
				buffer[i] = Samples[startSample + i];
			return buffer;
		}
		public override string MediaFilePath { get; set; }
		public override void LoadMedia(TimeSpan startTime) { }
		public override ITiming TimingSource => null;
		public override void Start() { }
		public override void Stop() { }
		public override void Pause() { }
		public override void Resume() { }
	}
}

internal sealed class DiffMark(TimeSpan start, TimeSpan duration) : IMark
{
	public TimeSpan StartTime { get; set; } = start;
	public TimeSpan EndTime => StartTime + Duration;
	public TimeSpan Duration { get; set; } = duration;
	public string Text { get; set; } = "";
	public IMarkCollection Parent { get; set; }
	public int CompareTo(IMark other) => StartTime.CompareTo(other.StartTime);
	public object Clone() => new DiffMark(StartTime, Duration);
	public event PropertyChangedEventHandler PropertyChanged { add { } remove { } }
}

internal sealed class DiffMarkCollection : IMarkCollection
{
	private readonly ObservableCollection<IMark> _marks = new();
	public DiffMarkCollection() => Marks = new ReadOnlyObservableCollection<IMark>(_marks);
	public Guid Id { get; set; }
	public string Name { get; set; }
	public bool IsDefault { get; set; }
	public bool IsVisible => true;
	public bool ShowGridLines { get; set; }
	public bool ShowTailGridLines { get; set; }
	public bool ShowMarkBar { get; set; }
	public bool Locked { get; set; }
	public int Level { get; set; } = 1;
	public ReadOnlyObservableCollection<IMark> Marks { get; }
	public IMarkDecorator Decorator { get; set; }
	public MarkCollectionType CollectionType { get; set; } = MarkCollectionType.Generic;
	public Guid LinkedMarkCollectionId { get; set; }
	public void RemoveMark(IMark mark) => _marks.Remove(mark);
	public void RemoveAll(Func<IMark, bool> condition) { foreach (var m in _marks.Where(condition).ToArray()) _marks.Remove(m); }
	public void AddMark(IMark mark) { mark.Parent = this; _marks.Add(mark); }
	public void AddMarks(IEnumerable<IMark> marks) { foreach (var m in marks) AddMark(m); }
	public void SwapPlaces(IMark lhs, IMark rhs) => throw new NotSupportedException();
	public void EnsureOrder() { }
	public List<IMark> MarksInclusiveOfTime(TimeSpan start, TimeSpan end)
	{
		var marks = new List<IMark>();
		foreach (var mark in _marks.ToArray())
		{
			if (mark.EndTime < start) continue;
			if (mark.StartTime > end) break;
			marks.Add(mark);
		}
		return marks;
	}
	public void FillGapTimes(IMark mark) => throw new NotSupportedException();
	public object Clone() => throw new NotSupportedException();
	public event PropertyChangedEventHandler PropertyChanged { add { } remove { } }
}

internal static class LiquidDiff
{
	public static int NodeNames;

	private static ElementNode Grid(int strings, int pixels, bool located, out List<ElementNode> leaves)
	{
		var name = "G" + (++NodeNames);
		var root = Vixen.Services.ElementNodeService.Instance.CreateSingle(null, name, false, false);
		leaves = new List<ElementNode>();
		var i = 0;
		for (var s = 0; s < strings; s++)
		{
			var str = Vixen.Services.ElementNodeService.Instance.CreateSingle(root, $"{name} S{s}", false, false);
			for (var p = 0; p < pixels; p++)
			{
				var leaf = Vixen.Services.ElementNodeService.Instance.CreateSingle(str, $"{name} S{s}-{p}", true, false);
				if (located)
				{
					var property = (VixenModules.Property.Location.LocationModule)leaf.Properties.Add(VixenModules.Property.Location.LocationDescriptor.ModuleId);
					property.ModuleData = new VixenModules.Property.Location.LocationData { X = (i % 10) * 4, Y = (i / 10) * 4, Z = 0 };
				}
				leaves.Add(leaf);
				i++;
			}
		}
		return root;
	}

	private static ColorGradient Gradient(JsonElement stops)
	{
		var g = new ColorGradient();
		g.Colors.Clear();
		foreach (var s in stops.EnumerateArray())
		{
			var rgb = s[1];
			var c = Color.FromArgb(rgb[0].GetInt32(), rgb[1].GetInt32(), rgb[2].GetInt32());
			g.Colors.Add(new ColorPoint(Common.Controls.ColorManagement.ColorModels.XYZ.FromRGB(new Common.Controls.ColorManagement.ColorModels.RGB(c)), 0.5, s[0].GetDouble()));
		}
		return g;
	}

	private static Curve CurveOf(JsonElement points)
	{
		var xs = points.EnumerateArray().Select(p => p[0].GetDouble()).ToArray();
		var ys = points.EnumerateArray().Select(p => p[1].GetDouble()).ToArray();
		return new Curve(new ZedGraph.PointPairList(xs, ys));
	}
	private static Curve C(JsonElement e, string name, Curve fallback) => e.TryGetProperty(name, out var v) ? CurveOf(v) : fallback;
	private static int I(JsonElement e, string name, int fallback) => e.TryGetProperty(name, out var v) ? v.GetInt32() : fallback;
	private static bool B(JsonElement e, string name, bool fallback) => e.TryGetProperty(name, out var v) ? v.GetBoolean() : fallback;

	public static string RenderCase(JsonElement c, MemoryTarget log)
	{
		var strings = c.GetProperty("strings").GetInt32();
		var pixels = c.GetProperty("pixels").GetInt32();
		var ms = c.GetProperty("ms").GetInt32();
		var located = c.TryGetProperty("located", out var l) && l.GetBoolean();
		var root = Grid(strings, pixels, located, out var leaves);

		var data = new LiquidData();
		var s = c.GetProperty("settings");
		data.BottomBarrier = B(s, "BottomBarrier", false);
		data.TopBarrier = B(s, "TopBarrier", false);
		data.LeftBarrier = B(s, "LeftBarrier", false);
		data.RightBarrier = B(s, "RightBarrier", false);
		data.MixColors = B(s, "MixColors", true);
		data.ParticleSize = I(s, "ParticleSize", 500);
		data.WarmUpFrames = I(s, "WarmUpFrames", 0);
		data.DespeckleThreshold = I(s, "DespeckleThreshold", 0);
		data.RenderScaleFactor = I(s, "RenderScaleFactor", 4);
		data.Gain = I(s, "Gain", 0);
		data.AttackTime = I(s, "AttackTime", 50);
		data.DecayTime = I(s, "DecayTime", 1500);
		data.Normalize = B(s, "Normalize", true);
		data.LowPass = B(s, "LowPass", false);
		data.LowPassFreq = I(s, "LowPassFreq", 100);
		data.HighPass = B(s, "HighPass", false);
		data.HighPassFreq = I(s, "HighPassFreq", 500);
		if (c.TryGetProperty("orientation", out var o)) data.Orientation = Enum.Parse<StringOrientation>(o.GetString());
		if (located)
		{
			data.TargetPositioning = TargetPositioningType.Locations;
			data.Orientation = StringOrientation.Vertical;
		}
		if (c.TryGetProperty("colors", out var cols))
			data.Colors = cols.EnumerateArray().Select(Gradient).ToList();

		// Mark collections, by index.
		var collections = new ObservableCollection<IMarkCollection>();
		if (c.TryGetProperty("marks", out var markDefs))
		{
			foreach (var def in markDefs.EnumerateArray())
			{
				var mc = new DiffMarkCollection { Id = Guid.Parse(def.GetProperty("id").GetString()), Name = def.GetProperty("name").GetString() };
				foreach (var m in def.GetProperty("items").EnumerateArray())
					mc.AddMark(new DiffMark(TimeSpan.FromMilliseconds(m[0].GetInt32()), TimeSpan.FromMilliseconds(m[1].GetInt32())));
				collections.Add(mc);
			}
		}

		data.EmitterData.Clear();
		foreach (var e in c.GetProperty("emitters").EnumerateArray())
		{
			var d = new EmitterData();
			if (e.TryGetProperty("ParticleType", out var pt)) d.ParticleType = Enum.Parse<EmitterData.ParticleSerializationType>(pt.GetString());
			d.UseColorArray = B(e, "UseColorArray", false);
			d.FramesPerColor = I(e, "FramesPerColor", 3);
			if (e.TryGetProperty("Color", out var col)) d.Color = Gradient(col);
			d.Lifetime = C(e, "Lifetime", d.Lifetime);
			d.ParticleVelocity = C(e, "ParticleVelocity", d.ParticleVelocity);
			d.Animate = B(e, "Animate", false);
			d.RandomStartingPosition = false;
			d.AnimateXStart = I(e, "AnimateXStart", 0);
			d.AnimateYStart = I(e, "AnimateYStart", 0);
			if (e.TryGetProperty("EdgeHandling", out var eh)) d.EdgeHandling = Enum.Parse<EmitterData.EdgeHandlingSerializationType>(eh.GetString());
			d.VelocityX = C(e, "VelocityX", d.VelocityX);
			d.VelocityY = C(e, "VelocityY", d.VelocityY);
			d.X = C(e, "X", d.X);
			d.Y = C(e, "Y", d.Y);
			d.Flow = C(e, "Flow", d.Flow);
			d.SourceSize = C(e, "SourceSize", d.SourceSize);
			if (e.TryGetProperty("NozzleMovement", out var nm)) d.NozzleMovement = Enum.Parse<EmitterData.NozzleMovementSerializationType>(nm.GetString());
			d.NozzleAngle = C(e, "NozzleAngle", d.NozzleAngle);
			d.OscillateStartAngle = I(e, "OscillateStartAngle", 0);
			d.OscillateEndAngle = I(e, "OscillateEndAngle", 90);
			d.OscillationSpeed = C(e, "OscillationSpeed", d.OscillationSpeed);
			if (e.TryGetProperty("FlowControl", out var fc)) d.FlowControl = Enum.Parse<EmitterData.FlowControlSerializationType>(fc.GetString());
			if (e.TryGetProperty("MarkCollectionId", out var mi)) d.MarkCollectionId = Guid.Parse(mi.GetString());
			d.OnTime = I(e, "OnTime", 2000);
			d.OffTime = I(e, "OffTime", 2000);
			d.Brightness = C(e, "Brightness", d.Brightness);
			data.EmitterData.Add(d);
		}

		var effect = new VixenModules.Effect.Liquid.Liquid();
		effect.ModuleData = data;
		effect.MarkCollections = collections;
		effect.TargetNodes = new IElementNode[] { root };
		effect.TimeSpan = TimeSpan.FromMilliseconds(ms);
		if (c.TryGetProperty("startMs", out var sm)) effect.StartTime = TimeSpan.FromMilliseconds(sm.GetInt32());
		if (c.TryGetProperty("audio", out var a))
		{
			var audio = new Audio { Frequency = a.GetProperty("rate").GetInt32() };
			audio.Samples = a.GetProperty("samples").EnumerateArray().SelectMany(seg => Enumerable.Repeat((float)seg[1].GetDouble(), seg[0].GetInt32())).ToArray();
			effect.Media = new List<IMediaModuleInstance> { audio };
		}

		var sb = new StringBuilder();
		LiquidFunWrapper.Log = sb;
		log.Logs.Clear();
		var ok = effect.PreRender();
		LiquidFunWrapper.Log = null;
		if (!ok)
		{
			var msg = log.Logs.Count > 0 ? log.Logs[0] : "";
			return "FAILED " + (msg.Split(' ', ':').FirstOrDefault(x => x.EndsWith("Exception")) ?? "?");
		}
		return sb.ToString();
	}
}
