// Golden data generator. Renders the original Vixen effects (compiled C#) through the real Vixen runtime and writes
// what every element's intents evaluate to, so crates/vixen-effect-registry/tests/golden.rs can compare the Rust port.
//
//   tools/golden/build.sh run [--only Effect,Effect] [--out dir]
//
// Cases come from Cases.cs. A case that uses no random numbers is stored frame by frame ("exact"). One that does stores
// statistics over many renders ("stats"), and one more render that reads a random stream the Rust port can reproduce.
// One that throws stores what the effect had built by then ("threw").

using System.Globalization;
using System.Reflection;
using System.Text;
using System.Text.Json;
using Common.Controls.ColorManagement.ColorModels;
using NLog;
using NLog.Config;
using NLog.Targets;
using Vixen.Data.Value;
using Vixen.Intent;
using Vixen.Marks;
using Vixen.Module.Effect;
using Vixen.Native;
using Vixen.Services;
using Vixen.Sys;
using VixenModules.Property.Color;
using VixenModules.Property.Location;
using VixenModules.Property.Orientation;

namespace VixenGolden;

public sealed class CaseDef
{
	public string Id;
	public string[] Targets;
	public int Ms;
	/// <summary>Where the effect starts on the sequence's time line. Mark-driven modes read marks relative to it.</summary>
	public int StartMs;
	/// <summary>VixenSystem.DefaultUpdateInterval. The user's real profile runs at 10 ms; Vixen's default is 50.</summary>
	public int FrameMs = 50;
	public List<(string name, object value)> Params = new();
	/// <summary>Mark collections as (id, name, marks of (start ms, end ms, text)).</summary>
	public List<MarkDef> Marks;
	/// <summary>The synthetic signal the effect finds in its media (see AudioSignal), if any.</summary>
	public string Audio;
}

public sealed class MarkDef
{
	public string Name;
	public Guid Id;
	public List<(double startMs, double endMs, string text)> Items = new();
}

public sealed class EffectSpec(string file, Guid typeId, Guid? upstreamTypeId = null, int forks = 1)
{
	public readonly string File = file;
	/// <summary>The module rendered here. A copy compiled by the extras project has an id of its own.</summary>
	public readonly Guid TypeId = typeId;
	/// <summary>The id of the original effect, which is how the Rust test finds its descriptor.</summary>
	public readonly Guid UpstreamTypeId = upstreamTypeId ?? typeId;
	/// <summary>How many <c>Rand::default()</c> calls the Rust port makes per render. See GoldenRandom.</summary>
	public readonly int Forks = forks;
	public readonly List<CaseDef> Cases = new();
	/// <summary>The update interval new cases get. Cases at anything but Vixen's 50 ms are tagged in their id.</summary>
	public int FrameMs = 50;
	/// <summary>
	/// Record the instance's second render. Whirlpool assigns PauseAtEnd after SetupRender reads it, so its first render
	/// ignores the setting. The port always applies it, deliberately, which is what the original settles on.
	/// </summary>
	public bool SettledOnSecondRender;
	private readonly HashSet<string> _ids = new();

	public Type EffectType
	{
		get
		{
			using var effect = ApplicationServices.Get<IEffectModuleInstance>(TypeId);
			return effect?.GetType() ?? throw new InvalidOperationException("Effect not loaded: " + File);
		}
	}

	public CaseDef Add(string id, string target, int ms, params (string, object)[] ps) => Add(id, target.Split('+'), ms, ps);

	public CaseDef Add(string id, string[] targets, int ms, params (string, object)[] ps)
	{
		var full = FrameMs == 50 ? id : $"f{FrameMs}/{id}";
		var unique = full;
		for (var n = 2; !_ids.Add(unique); n++) unique = full + "#" + n;
		var c = new CaseDef { Id = unique, Targets = targets, Ms = ms, FrameMs = FrameMs };
		c.Params.AddRange(ps);
		Cases.Add(c);
		return c;
	}
}

internal static class Program
{
	public const int RendersToClassify = 12;
	public const int StatsRenders = 24;
	public const int PaletteRenders = 100;
	public const int PaletteLitRenders = 60;
	public const int PaletteRendersMax = 2000;

	private static readonly Dictionary<string, ElementNode> Targets = new();
	private static MemoryTarget _log;

	private static async Task<int> Main(string[] args)
	{
		CultureInfo.DefaultThreadCurrentCulture = CultureInfo.InvariantCulture;
		var root = Directory.GetCurrentDirectory();
		var outDir = Path.Combine(root, "tests/golden");
		HashSet<string> only = null;
		string chosenProfile = null, resaveFrom = null, resaveTo = null;
		for (var i = 0; i < args.Length; i++)
		{
			if (args[i] == "--only") only = args[++i].Split(',').ToHashSet();
			if (args[i] == "--out") outDir = args[++i];
			if (args[i] == "--profile") chosenProfile = Path.GetFullPath(args[++i]);
			if (args[i] == "--resave")
			{
				resaveFrom = Path.GetFullPath(args[++i]);
				resaveTo = Path.GetFullPath(args[++i]);
			}
		}

		if (resaveFrom != null) AppDomain.CurrentDomain.AssemblyResolve += FileCheck.FindMovedAssembly;
		var profile = chosenProfile ?? Path.Combine(root, "target/golden/profile", Guid.NewGuid().ToString("N"));
		Directory.CreateDirectory(profile);
		var configuration = new LoggingConfiguration();
		_log = new MemoryTarget("mem") { Layout = "${message} ${exception:format=tostring}" };
		configuration.AddRule(NLog.LogLevel.Warn, NLog.LogLevel.Fatal, _log);
		LogManager.Configuration = configuration;
		VixenSystem.UIContext = new SynchronizationContext();
		await using var runtime = await NativeVixenRuntime.StartAsync(profile);
		if (_log.Logs.Count > 0)
		{
			foreach (var l in _log.Logs) Console.WriteLine("startup log: " + Text.Trunc(l, 300));
			_log.Logs.Clear();
		}

		if (resaveFrom != null)
		{
			var failures = FileCheck.Resave(resaveFrom, resaveTo);
			await runtime.SaveProfileAsync();
			foreach (var l in _log.Logs.Take(20)) Console.WriteLine("log: " + Text.Trunc(l, 400));
			Console.WriteLine("unresolved assemblies: " + string.Join(", ", FileCheck.Unresolved.OrderBy(n => n, StringComparer.Ordinal)));
			return failures;
		}

		Directory.CreateDirectory(outDir);
		BuildTargets(Path.Combine(outDir, "targets.json"));

		var specs = Cases.All();
		long total = 0;
		foreach (var spec in specs)
		{
			if (only != null && !only.Contains(spec.File)) continue;
			var path = Path.Combine(outDir, spec.File + ".json.gz");
			var counts = WriteEffect(spec, path);
			var size = new FileInfo(path).Length;
			total += size;
			Console.WriteLine($"{spec.File,-14} cases {spec.Cases.Count,5}  exact {counts.exact,5}  stats {counts.stats,4}  (unseeded {counts.unseeded,3})  threw {counts.failed,3}  edited differs {counts.editedDiffers,3}  {size / 1024,7} KB");
		}

		if (only == null || only.Contains("layers"))
		{
			var layersPath = Path.Combine(outDir, "layers.json.gz");
			Layers.Write(layersPath);
			total += new FileInfo(layersPath).Length;
		}
		Console.WriteLine($"total {total / 1024 / 1024} MB");
		try { Directory.Delete(profile, true); } catch { /* the profile is scratch */ }
		return 0;
	}

	// ---------------------------------------------------------------------------------------------------------------
	// Targets. The element tree is written to targets.json so the Rust test builds exactly the same one.
	// ---------------------------------------------------------------------------------------------------------------

	private sealed class NodeSpec
	{
		public string Name;
		public List<NodeSpec> Children;
		public (int x, int y)? Location;
		public ColorV SingleColor;
		public string ColorSet;
		public string SharedWith;
		public Orientation? Orient;
		public ElementNode Built;
	}

	private static void BuildTargets(string path)
	{
		var defs = new List<(string key, NodeSpec spec)>
		{
			("single", new NodeSpec { Name = "Single", Location = (100, 100) }),
			("string30", StringOf("String", 30, i => (10 + i * 3, 40 + i * i % 7))),
			("string30b", StringOf("StringB", 30, i => (10 + i * 3, 80 + i % 5))),
			("grid", GridOf("Grid", 8, 25, (s, p) => (20 + s * 4, 200 - p * 3))),
			("discrete", StringOf("Discrete", 30, i => (10 + i * 2, 60), n => n.SingleColor = V.Red)),
			("discrete_rgb", new NodeSpec
			{
				Name = "DiscreteRGB",
				Children =
				[
					StringOf("DiscreteR", 10, i => (10 + i * 2, 70), n => n.SingleColor = V.Red),
					StringOf("DiscreteG", 10, i => (10 + i * 2, 74), n => n.SingleColor = V.Lime),
					StringOf("DiscreteB", 10, i => (10 + i * 2, 78), n => n.SingleColor = V.Blue),
				],
			}),
			("multi_discrete", StringOf("MultiDiscrete", 12, i => (10 + i * 2, 90), n => n.ColorSet = "RGB")),
			("tree4", new NodeSpec
			{
				Name = "Tree4",
				Children =
				[
					new NodeSpec { Name = "T4A", Children = [StringOf("T4A1", 4, i => (10 + i, 10)), StringOf("T4A2", 4, i => (10 + i, 20)), StringOf("T4A3", 4, i => (10 + i, 30))] },
					new NodeSpec { Name = "T4B", Children = [StringOf("T4B1", 4, i => (30 + i, 10)), StringOf("T4B2", 4, i => (30 + i, 20)), StringOf("T4B3", 4, i => (30 + i, 30))] },
				],
			}),
			("uneven", new NodeSpec
			{
				Name = "Uneven",
				Children =
				[
					StringOf("U1", 10, i => (10, 10 + i * 2)),
					StringOf("U2", 25, i => (14, 10 + i)),
					StringOf("U3", 5, i => (18, 10 + i * 3)),
					StringOf("U4", 17, i => (22, 12 + i)),
				],
			}),
			("shared", new NodeSpec
			{
				Name = "Shared",
				Children =
				[
					StringOf("SA", 4, i => (10 + i, 10)),
					new NodeSpec { Name = "SB", Children = [new NodeSpec { Name = "SB-1", Location = (10, 20) }, new NodeSpec { Name = "SB-2", SharedWith = "SA-2" }, new NodeSpec { Name = "SB-3", Location = (12, 20) }] },
				],
			}),
			// Missing and non-positive X locations, as real props without a preview position have.
			("partial_loc", StringOf("Partial", 12, i => (i * 4, 10 + i), n => { if (n.Name.EndsWith("-1") || n.Name.EndsWith("-4") || n.Name.EndsWith("-7") || n.Name.EndsWith("-10")) n.Location = null; })),
			// Several elements on the same pixel, which location rendering shares one frame array for.
			("dup_loc", StringOf("DupLoc", 10, i => (5 + i / 3 * 4, 8 + i / 3 * 2))),
			("mixed_depth", new NodeSpec
			{
				Name = "MixedDepth",
				Children =
				[
					new NodeSpec { Name = "MD-solo", Location = (5, 5) },
					StringOf("MD1", 6, i => (10 + i, 10)),
					new NodeSpec { Name = "MD-solo2", Location = (6, 5) },
				],
			}),
			// As long as the real show's light strips.
			("strip", StringOf("Strip", 554, i => (10 + i, 40))),
			// Props with an Orientation property, which pixel effects follow when given their targets.
			("oriented", WithOrientation(GridOf("Oriented", 6, 10, (s, p) => (20 + p * 4, 100 - s * 3)), Orientation.Horizontal)),
			("oriented_v", WithOrientation(GridOf("OrientedV", 6, 10, (s, p) => (20 + s * 4, 100 - p * 3)), Orientation.Vertical)),
		};

		var all = new Dictionary<string, NodeSpec>();
		foreach (var (key, spec) in defs)
		{
			Build(spec, null, all);
			Targets[key] = spec.Built;
		}

		var o = new JsonWriterOptions { Indented = false, Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping };
		using var fs = File.Create(path);
		using var w = new Utf8JsonWriter(fs, o);
		w.WriteStartObject();
		foreach (var (key, spec) in defs)
		{
			w.WritePropertyName(key);
			WriteNode(w, spec);
		}
		w.WriteEndObject();
	}

	private static NodeSpec StringOf(string name, int count, Func<int, (int, int)> loc, Action<NodeSpec> edit = null)
	{
		var s = new NodeSpec { Name = name, Children = new List<NodeSpec>() };
		for (var i = 0; i < count; i++)
		{
			var n = new NodeSpec { Name = $"{name}-{i + 1}", Location = loc(i) };
			edit?.Invoke(n);
			s.Children.Add(n);
		}
		return s;
	}

	private static NodeSpec GridOf(string name, int strings, int pixels, Func<int, int, (int, int)> loc)
	{
		var g = new NodeSpec { Name = name, Children = new List<NodeSpec>() };
		for (var s = 0; s < strings; s++)
		{
			var str = new NodeSpec { Name = $"{name} S{s}", Children = new List<NodeSpec>() };
			for (var p = 0; p < pixels; p++)
				str.Children.Add(new NodeSpec { Name = $"{name} S{s}-{p + 1}", Location = loc(s, p) });
			g.Children.Add(str);
		}
		return g;
	}

	private static NodeSpec WithOrientation(NodeSpec spec, Orientation orientation)
	{
		spec.Orient = orientation;
		return spec;
	}

	private static void Build(NodeSpec spec, ElementNode parent, Dictionary<string, NodeSpec> all)
	{
		if (spec.SharedWith != null)
		{
			spec.Built = all[spec.SharedWith].Built;
			VixenSystem.Nodes.AddChildToParent(spec.Built, parent);
			return;
		}
		var isLeaf = spec.Children == null;
		spec.Built = ElementNodeService.Instance.CreateSingle(parent, spec.Name, isLeaf, false);
		if (spec.Built.Name != spec.Name) throw new InvalidOperationException("Node renamed: " + spec.Name);
		all[spec.Name] = spec;
		if (spec.Location is { } l)
		{
			var property = (LocationModule)spec.Built.Properties.Add(LocationDescriptor.ModuleId);
			property.ModuleData = new LocationData { X = l.x, Y = l.y, Z = 1 };
		}
		if (spec.SingleColor != null)
		{
			var property = (ColorModule)spec.Built.Properties.Add(ColorDescriptor.ModuleId);
			property.ColorType = ElementColorType.SingleColor;
			property.SingleColor = spec.SingleColor.ToColor();
		}
		if (spec.ColorSet != null)
		{
			var property = (ColorModule)spec.Built.Properties.Add(ColorDescriptor.ModuleId);
			property.ColorType = ElementColorType.MultipleDiscreteColors;
			property.ColorSetName = spec.ColorSet;
		}
		if (spec.Orient is { } orientation)
		{
			var property = (OrientationModule)spec.Built.Properties.Add(OrientationDescriptor.ModuleId);
			property.Orientation = orientation;
		}
		if (spec.Children != null)
			foreach (var c in spec.Children) Build(c, spec.Built, all);
	}

	private static void WriteNode(Utf8JsonWriter w, NodeSpec spec)
	{
		w.WriteStartObject();
		w.WriteString("n", spec.Name);
		if (spec.SharedWith != null) w.WriteString("shared", spec.SharedWith);
		if (spec.Location is { } l)
		{
			w.WriteStartArray("loc");
			w.WriteNumberValue(l.x);
			w.WriteNumberValue(l.y);
			w.WriteEndArray();
		}
		if (spec.SingleColor != null) w.WriteString("color", spec.SingleColor.Hex);
		if (spec.ColorSet != null) w.WriteString("set", spec.ColorSet);
		if (spec.Orient is { } orientation) w.WriteString("orient", orientation.ToString());
		if (spec.Children != null)
		{
			w.WriteStartArray("c");
			foreach (var c in spec.Children) WriteNode(w, c);
			w.WriteEndArray();
		}
		w.WriteEndObject();
	}

	// ---------------------------------------------------------------------------------------------------------------
	// Rendering
	// ---------------------------------------------------------------------------------------------------------------

	private sealed class Snapshot
	{
		public string Failure;
		public long[] Times;
		public List<Element> Elements;
		public string[][] Frames; // [element][time] canonical state set
		public string[] Intents;  // [element] canonical intent list
		public double[][] Brightness; // [element][time]
		public int[][] ColorKey;      // [element][time], -1 for dark
		public List<(int kind, long start, long end)>[] RawIntents;
	}

	private static long[] TimesFor(int ms, int frameMs)
	{
		var set = new SortedSet<long> { 0, ms * 10_000L };
		for (long t = 0; t <= ms; t += frameMs) set.Add(t * 10_000L);
		var span = ms * 10_000L;
		foreach (var extra in new[] { 10_000L, 123_456L, 499_999L, span / 3, span * 7071 / 10000, span - 1 })
			if (extra > 0 && extra < span)
				set.Add(extra);
		return set.ToArray();
	}

	private static string Num(double d) => d.ToString("G10", CultureInfo.InvariantCulture);

	private static (string text, double brightness, int colorKey) StateOf(object state)
	{
		switch (state)
		{
			case RGBValue v:
				return ($"R{v.R:x2}{v.G:x2}{v.B:x2}", Math.Max(v.R, Math.Max(v.G, v.B)) / 255.0, v.R << 16 | v.G << 8 | v.B);
			case LightingValue v:
			{
				var c = v.Color;
				var hex = c.A == 255 ? $"{c.R:x2}{c.G:x2}{c.B:x2}" : $"{c.A:x2}{c.R:x2}{c.G:x2}{c.B:x2}";
				return ($"L{hex},{Num(v.Intensity)}", v.Intensity * Math.Max(c.R, Math.Max(c.G, c.B)) / 255.0, c.R << 16 | c.G << 8 | c.B);
			}
			case DiscreteValue v:
			{
				var c = v.Color;
				var hex = c.A == 255 ? $"{c.R:x2}{c.G:x2}{c.B:x2}" : $"{c.A:x2}{c.R:x2}{c.G:x2}{c.B:x2}";
				return ($"D{hex},{Num(v.Intensity)}", v.Intensity * Math.Max(c.R, Math.Max(c.G, c.B)) / 255.0, c.R << 16 | c.G << 8 | c.B);
			}
			case CommandValue v:
				return ($"C{CommandText(v.Command)}", 0, -1);
			default:
				throw new NotSupportedException("Unhandled state " + state?.GetType());
		}
	}

	/// <summary>A command's type and value, the text in hex so that it cannot be taken for the separators a state uses.</summary>
	private static string CommandText(Vixen.Commands.ICommand command) => command switch
	{
		Vixen.Commands._8BitCommand c => $"8:{c.CommandValue}",
		Vixen.Commands._16BitCommand c => $"16:{c.CommandValue}",
		Vixen.Commands._32BitCommand c => $"32:{c.CommandValue}",
		Vixen.Commands._64BitCommand c => $"64:{c.CommandValue}",
		Vixen.Commands.ColorCommand c => $"color:{c.CommandValue.R:x2}{c.CommandValue.G:x2}{c.CommandValue.B:x2}",
		Vixen.Commands.StringCommand c => $"text:{Convert.ToHexString(Encoding.UTF8.GetBytes(c.CommandValue)).ToLowerInvariant()}",
		_ => throw new NotSupportedException("Unhandled command " + command?.GetType()),
	};

	private static string KindOf(IIntent intent) => intent switch
	{
		CommandIntent => "command",
		LightingIntent => "L",
		StaticLightingIntent => "l",
		DiscreteLightingIntent => "D",
		StaticDiscreteLightingIntent => "d",
		StaticArrayIntent<RGBValue> => "A",
		StaticArrayIntent<LightingValue> => "B",
		StaticArrayIntent<DiscreteValue> => "C",
		StaticIntent<RGBValue> => "S",
		RGBIntent => "G",
		_ => intent.GetType().Name,
	};

	/// <summary>
	/// Gets an effect ready to render, in one of the two ways a real one gets there. Either way the values are set as the
	/// editor sets them: through the property setters, on an effect that already has its targets and mark collections.
	/// With <paramref name="reloaded"/> false that same instance renders, as it does while a sequence is being edited.
	/// With it true the data moves to a new instance in the order a saved sequence loads: EffectNodeSurrogate gives the
	/// new effect its targets while it still holds default data, the ModuleData setter follows, then SequenceService.Load
	/// assigns the mark collections.
	/// </summary>
	private static IEffectModuleInstance Create(EffectSpec spec, CaseDef c, bool reloaded)
	{
		var targets = c.Targets.Select(t => (IElementNode)Targets[t]).ToArray();
		var effect = ApplicationServices.Get<IEffectModuleInstance>(spec.TypeId) ?? throw new InvalidOperationException("Effect not loaded: " + spec.File);
		effect.TimeSpan = TimeSpan.FromMilliseconds(c.Ms);
		effect.StartTime = TimeSpan.FromMilliseconds(c.StartMs);
		effect.TargetNodes = targets;
		// A sequence always hands its effects its mark collections, even when it has none.
		var marks = BuildMarks(c.Marks ?? new List<MarkDef>());
		effect.MarkCollections = marks;
		AttachAudio(effect, c);
		ReadVisibleProperties(effect);
		foreach (var (name, value) in c.Params)
		{
			var property = name switch { "MarkCollectionByName" => "MarkCollectionId", "WaveformData" => "Waves", "WhirlData" => "Whirls", _ => name };
			var pi = effect.GetType().GetProperty(property) ?? throw new InvalidOperationException($"{spec.File} has no property {property}");
			if (value is RecordListV records)
			{
				// Keep the original collection, including its parent/marks and child-change subscriptions.
				var collection = pi.GetValue(effect);
				var type = collection.GetType();
				var itemType = (Type)type.GetMethod("GetItemType")!.Invoke(collection, null);
				type.GetMethod("Clear")!.Invoke(collection, null);
				foreach (var record in records.Items)
				{
					var item = Activator.CreateInstance(itemType);
					type.GetMethod("Add")!.Invoke(collection, [item]);
					foreach (var (field, fieldValue) in record.Values)
					{
						var member = itemType.GetProperty(field) ?? throw new InvalidOperationException($"{itemType.Name} has no property {field}");
						member.SetValue(item, Convert2.To(fieldValue, member.PropertyType));
					}
				}
			}
			else pi.SetValue(effect, Convert2.To(value, pi.PropertyType));
			ReadVisibleProperties(effect);
		}
		if (!reloaded) return effect;

		var loaded = ApplicationServices.Get<IEffectModuleInstance>(spec.TypeId);
		loaded.TimeSpan = effect.TimeSpan;
		loaded.StartTime = effect.StartTime;
		loaded.TargetNodes = targets;
		loaded.ModuleData = ThroughFile(effect.ModuleData);
		loaded.MarkCollections = marks;
		AttachAudio(loaded, c);
		return loaded;
	}

	private static readonly Lazy<MethodInfo> AudioNamed = new(() =>
		AppDomain.CurrentDomain.GetAssemblies().First(a => a.GetName().Name == "Module.Effect.GoldenExtras")
			.GetType("VixenGolden.GoldenAudio")!.GetMethod("Named"));

	/// <summary>The media a sequence hands its effects: here, the one synthetic track.</summary>
	private static void AttachAudio(IEffectModuleInstance effect, CaseDef c)
	{
		if (c.Audio != null)
			effect.Media = new List<Vixen.Module.Media.IMediaModuleInstance> { (Vixen.Module.Media.IMediaModuleInstance)AudioNamed.Value.Invoke(null, [c.Audio]) };
	}

	/// <summary>
	/// What the editor's property grid does whenever it refreshes. It matters: some getters correct the data they return,
	/// such as Chase's StaticColor calling CheckForInvalidColorData.
	/// </summary>
	private static void ReadVisibleProperties(IEffectModuleInstance effect)
	{
		foreach (System.ComponentModel.PropertyDescriptor property in System.ComponentModel.TypeDescriptor.GetProperties(effect))
		{
			if (!property.IsBrowsable) continue;
			try { property.GetValue(effect); }
			catch { /* A getter the grid would also have failed on. */ }
		}
	}

	/// <summary>The data as a sequence file would give it back, through its own data contract.</summary>
	private static Vixen.Module.IModuleDataModel ThroughFile(Vixen.Module.IModuleDataModel data)
	{
		var serializer = new System.Runtime.Serialization.DataContractSerializer(data.GetType());
		using var stream = new MemoryStream();
		serializer.WriteObject(stream, data);
		stream.Position = 0;
		return (Vixen.Module.IModuleDataModel)serializer.ReadObject(stream);
	}

	private static System.Collections.ObjectModel.ObservableCollection<IMarkCollection> BuildMarks(List<MarkDef> marks)
	{
		var collections = new System.Collections.ObjectModel.ObservableCollection<IMarkCollection>();
		foreach (var def in marks)
		{
			var mc = new GoldenMarkCollection { Name = def.Name, Id = def.Id };
			foreach (var (startMs, endMs, text) in def.Items)
				mc.AddMark(new GoldenMark(TimeSpan.FromMilliseconds(startMs), TimeSpan.FromMilliseconds(endMs - startMs), text));
			collections.Add(mc);
		}
		return collections;
	}

	private static Snapshot RenderOnce(EffectSpec spec, CaseDef c, bool reloaded = true)
	{
		_log.Logs.Clear();
		// BaseEffect reads the interval when it is constructed.
		VixenSystem.DefaultUpdateInterval = c.FrameMs;
		using var effect = Create(spec, c, reloaded);
		var snapshot = new Snapshot { Times = TimesFor(c.Ms, c.FrameMs) };
		var ok = effect.PreRender();
		if (spec.SettledOnSecondRender)
		{
			effect.Render();
			((EffectModuleInstanceBase)effect).MarkDirty();
			ok = effect.PreRender();
		}
		// After a failed PreRender, Render tries again and returns whatever the effect had built when it threw, as it does
		// in a real show: basic effects build their intents as they go, pixel effects assign them only when finished.
		var intents = effect.Render();
		if (!ok || intents == null)
			snapshot.Failure = _log.Logs.Count > 0 ? Text.Trunc(_log.Logs[0], 300) : "PreRender returned false";
		if (intents == null) return snapshot;

		var elements = new List<Element>();
		var seen = new HashSet<Guid>();
		foreach (var t in c.Targets)
			foreach (var leaf in Targets[t].GetLeafEnumerator())
				if (leaf.Element != null && seen.Add(leaf.Element.Id))
					elements.Add(leaf.Element);
		foreach (var id in intents.Keys)
			if (!seen.Contains(id))
				throw new InvalidOperationException($"{spec.File}/{c.Id}: intents for an element outside the target");

		snapshot.Elements = elements;
		snapshot.Frames = new string[elements.Count][];
		snapshot.Intents = new string[elements.Count];
		snapshot.Brightness = new double[elements.Count][];
		snapshot.ColorKey = new int[elements.Count][];
		snapshot.RawIntents = new List<(int, long, long)>[elements.Count];
		var times = snapshot.Times;
		var states = new List<(string text, double b, int key)>(8);
		for (var e = 0; e < elements.Count; e++)
		{
			var id = elements[e].Id;
			var frames = new string[times.Length];
			var bright = new double[times.Length];
			var keys = new int[times.Length];
			for (var ti = 0; ti < times.Length; ti++)
			{
				states.Clear();
				var active = intents.GetElementIntentsAtTime(id, new TimeSpan(times[ti]));
				if (active != null)
					foreach (var node in active)
						states.Add(StateOf(node.Intent.GetStateAt(new TimeSpan(times[ti]) - node.StartTime)));
				states.Sort((a, b) => string.CompareOrdinal(a.text, b.text));
				frames[ti] = states.Count == 0 ? "" : states.Count == 1 ? states[0].text : string.Join("|", states.Select(s => s.text));
				var best = -1.0;
				var key = -1;
				foreach (var s in states)
					if (s.b > best) { best = s.b; key = s.key; }
				bright[ti] = Math.Max(best, 0);
				keys[ti] = best > 0 ? key : -1;
			}
			snapshot.Frames[e] = frames;
			snapshot.Brightness[e] = bright;
			snapshot.ColorKey[e] = keys;

			var nodes = intents.GetIntentNodesForElement(id);
			var raw = new List<(int, long, long)>();
			if (nodes != null)
			{
				var parts = nodes.Select(n => (KindOf(n.Intent), n.StartTime.Ticks, n.EndTime.Ticks)).ToList();
				parts.Sort();
				snapshot.Intents[e] = string.Join(";", parts.Select(p => $"{p.Item1}:{p.Item2}:{p.Item3}"));
				raw.AddRange(parts.Select(p => (0, p.Item2, p.Item3)));
			}
			else
				snapshot.Intents[e] = "";
			snapshot.RawIntents[e] = raw;
		}
		return snapshot;
	}

	private static bool SameFrames(Snapshot a, Snapshot b)
	{
		if ((a.Failure == null) != (b.Failure == null)) return false;
		if ((a.Frames == null) != (b.Frames == null)) return false;
		if (a.Frames == null) return true;
		for (var e = 0; e < a.Frames.Length; e++)
		{
			if (a.Intents[e] != b.Intents[e]) return false;
			var fa = a.Frames[e];
			var fb = b.Frames[e];
			for (var t = 0; t < fa.Length; t++)
				if (fa[t] != fb[t])
					return false;
		}
		return true;
	}

	private static readonly FieldInfo ThreadRandomField =
		typeof(VixenModules.Effect.Effect.ThreadSafeRandom).GetField("ThreadRandom", BindingFlags.NonPublic | BindingFlags.Static);

	// Vixen's module loader loads the extras assembly by path, so it is found among the loaded ones.
	private static readonly Lazy<FieldInfo> ExtrasHook = new(() =>
		AppDomain.CurrentDomain.GetAssemblies().First(a => a.GetName().Name == "Module.Effect.GoldenExtras")
			.GetType("VixenGolden.GoldenRandomHook")!.GetField("Current"));

	/// <summary>Renders with every random source this effect uses reading the scripted stream.</summary>
	private static Snapshot RenderSeeded(EffectSpec spec, CaseDef c, ulong seed, bool reloaded = true)
	{
		var threadLocal = (ThreadLocal<Random>)ThreadRandomField.GetValue(null);
		var previous = threadLocal!.Value;
		var scripted = new GoldenRandom(seed, spec.Forks);
		UseRandom(scripted);
		try
		{
			return RenderOnce(spec, c, reloaded);
		}
		finally
		{
			UseRandom(previous);
			_lastDraws = scripted.Draws;
		}
	}

	/// <summary>Makes every random source the effects use read this one.</summary>
	private static void UseRandom(Random random)
	{
		((ThreadLocal<Random>)ThreadRandomField.GetValue(null))!.Value = random;
		ExtrasHook.Value.SetValue(null, random);
	}

	/// <summary>A generator of ordinary random numbers that starts the same way every run, so the golden files do too.</summary>
	private static Random FixedRandomFor(EffectSpec spec, CaseDef c)
	{
		var hash = 2166136261u;
		foreach (var b in Encoding.UTF8.GetBytes(spec.File + "/" + c.Id))
			hash = unchecked((hash ^ b) * 16777619u);
		return new Random(unchecked((int)hash));
	}

	private static long _lastDraws;

	/// <summary>
	/// Renders that come out the same every time are not proof of an effect that uses no random numbers: one can draw them
	/// and only rarely light anything. Whether it draws any is, so that decides how a case is classified, along with the renders.
	/// </summary>
	private static bool DrawsRandomNumbers(EffectSpec spec, CaseDef c)
	{
		RenderSeeded(spec, c, 1);
		return _lastDraws > 0;
	}

	private static (int exact, int stats, int failed, int unseeded, int editedDiffers) WriteEffect(EffectSpec spec, string path)
	{
		var counts = (exact: 0, stats: 0, failed: 0, unseeded: 0, editedDiffers: 0);
		using var fs = File.Create(path);
		using var gz = new System.IO.Compression.GZipStream(fs, System.IO.Compression.CompressionLevel.SmallestSize);
		var o = new JsonWriterOptions { Indented = false, SkipValidation = true, Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping };
		using var w = new Utf8JsonWriter(gz, o);
		w.WriteStartObject();
		w.WriteString("effect", spec.File);
		w.WriteString("type_id", spec.UpstreamTypeId.ToString());
		WriteDefaults(w, spec);
		w.WriteStartArray("cases");
		foreach (var c in spec.Cases)
		{
			UseRandom(FixedRandomFor(spec, c));
			var first = RenderOnce(spec, c);
			var deterministic = true;
			for (var i = 1; i < RendersToClassify && deterministic; i++)
				deterministic = SameFrames(first, RenderOnce(spec, c));
			if (first.Failure == null && deterministic && DrawsRandomNumbers(spec, c))
				deterministic = false;

			w.WriteStartObject();
			w.WriteString("id", c.Id);
			w.WriteStartArray("targets");
			foreach (var t in c.Targets) w.WriteStringValue(t);
			w.WriteEndArray();
			w.WriteNumber("ms", c.Ms);
			if (c.StartMs != 0) w.WriteNumber("start_ms", c.StartMs);
			if (c.FrameMs != 50) w.WriteNumber("frame_ms", c.FrameMs);
			if (c.Audio != null) w.WriteString("audio", c.Audio);
			// An array of pairs: setters can have side effects, so the order they ran in matters.
			w.WriteStartArray("params");
			foreach (var (name, value) in c.Params)
			{
				w.WriteStartArray();
				w.WriteStringValue(name);
				Json.WriteValue(w, value);
				w.WriteEndArray();
			}
			w.WriteEndArray();
			if (c.Marks != null)
			{
				w.WriteStartArray("marks");
				foreach (var m in c.Marks)
				{
					w.WriteStartObject();
					w.WriteString("name", m.Name);
					w.WriteString("id", m.Id.ToString());
					w.WriteStartArray("items");
					foreach (var (s, e, text) in m.Items)
					{
						w.WriteStartArray();
						w.WriteNumberValue(s);
						w.WriteNumberValue(e);
						w.WriteStringValue(text ?? "");
						w.WriteEndArray();
					}
					w.WriteEndArray();
					w.WriteEndObject();
				}
				w.WriteEndArray();
			}
			w.WriteStartArray("times");
			foreach (var t in first.Times) w.WriteNumberValue(t);
			w.WriteEndArray();

			if (first.Failure != null)
			{
				counts.failed++;
				w.WriteString("check", "threw");
				w.WriteString("failure", first.Failure);
				if (first.Frames == null) w.WriteBoolean("dark", true);
				else if (deterministic) WriteExact(w, first);
				else w.WriteBoolean("random_partial", true);
			}
			else if (deterministic)
			{
				counts.exact++;
				w.WriteString("check", "exact");
				WriteExact(w, first);
				// The instance that was edited, without the reload. Stored only where that renders differently.
				var edited = RenderOnce(spec, c, false);
				if (edited.Failure != null)
				{
					w.WriteStartObject("edited");
					w.WriteString("failure", edited.Failure);
					if (edited.Frames == null) w.WriteBoolean("dark", true);
					else WriteExact(w, edited);
					w.WriteEndObject();
				}
				else if (!SameFrames(first, edited))
				{
					counts.editedDiffers++;
					w.WriteStartObject("edited");
					WriteExact(w, edited);
					w.WriteEndObject();
				}
			}
			else
			{
				counts.stats++;
				w.WriteString("check", "stats");
				var runs = new List<Snapshot> { first };
				for (var i = 1; i < StatsRenders; i++) runs.Add(RenderOnce(spec, c));
				var statRuns = runs.Select(r => new Stats.Run(r.Brightness, r.ColorKey, r.RawIntents)).ToList();
				var extraColors = new SortedSet<int>();
				// A small palette is compared as a set, so sample until it has had a fair chance to show. Cases that rarely light
				// anything get more renders, up to a limit, or a colour would only be missing by luck.
				if (Stats.ColorsOf(statRuns).Count <= 32)
				{
					var litRenders = 0;
					for (var i = 0; i < PaletteRenders || (litRenders > 0 && litRenders < PaletteLitRenders && i < PaletteRendersMax); i++)
					{
						var r = RenderOnce(spec, c);
						if (r.Failure != null) continue;
						var colors = Stats.ColorsOf([new Stats.Run(r.Brightness, r.ColorKey, r.RawIntents)]);
						if (colors.Count > 0) litRenders++;
						extraColors.UnionWith(colors);
					}
				}
				Stats.Write(w, statRuns, first.Times, extraColors);

				// The same render again with a random stream the Rust port can reproduce.
				var seed = (ulong)(0x1234_5678_9abc_def0L + spec.Cases.IndexOf(c));
				var seeded = RenderSeeded(spec, c, seed);
				if (seeded.Failure == null && SameFrames(seeded, RenderSeeded(spec, c, seed)))
				{
					w.WriteStartObject("seeded");
					w.WriteString("seed", seed.ToString());
					WriteExact(w, seeded);
					w.WriteEndObject();
					// The same stream on the instance that was edited, stored only where that renders differently.
					var edited = RenderSeeded(spec, c, seed, false);
					if (edited.Failure != null || !SameFrames(seeded, edited))
					{
						counts.editedDiffers++;
						w.WriteStartObject("seeded_edited");
						if (edited.Failure != null) w.WriteString("failure", edited.Failure);
						if (edited.Frames == null) w.WriteBoolean("dark", true);
						else WriteExact(w, edited);
						w.WriteEndObject();
					}
				}
				else
				{
					counts.unseeded++;
				}
			}
			w.WriteEndObject();
			w.Flush();
		}
		w.WriteEndArray();
		w.WriteEndObject();
		return counts;
	}

	/// <summary>The effect's default property values, so the Rust parameter tables can be checked too.</summary>
	private static void WriteDefaults(Utf8JsonWriter w, EffectSpec spec)
	{
		using var effect = ApplicationServices.Get<IEffectModuleInstance>(spec.TypeId);
		w.WriteStartObject("defaults");
		foreach (var p in effect.GetType().GetProperties(BindingFlags.Public | BindingFlags.Instance).Where(p => p.GetCustomAttributes().Any(a => a.GetType().Name == "ValueAttribute")))
		{
			if (p.Name == "MarkCollectionId") continue;
			var value = FromRuntime(p.GetValue(effect));
			if (value == null) continue;
			w.WritePropertyName(p.Name);
			Json.WriteValue(w, value);
		}
		w.WriteEndObject();
	}

	private static object FromRuntime(object v)
	{
		switch (v)
		{
			case null: return null;
			case bool or int or double or string: return v;
			case float f: return (double)f;
			case Enum e: return e.ToString();
			case Color c: return new ColorV(c.R, c.G, c.B);
			case RGB rgb: { var c = rgb.ToArgb(); return new ColorV(c.R, c.G, c.B); }
			case Common.ValueTypes.Percentage p: return (double)p.Value;
			case VixenModules.App.Curves.Curve c: return new CurveV(c.Points.Select(p => (p.X, p.Y)).ToArray());
			case VixenModules.App.ColorGradients.ColorGradient g:
				return new GradV(
					g.Colors.Select(p => { var rgb = p.Color.ToRGB().ToArgb(); return new Stop(p.Position, new ColorV(rgb.R, rgb.G, rgb.B), p.Focus); }).ToArray(),
					g.Alphas.Select(a => new AlphaStop(a.Position, a.Alpha, a.Focus)).ToArray());
			case VixenModules.App.ColorGradients.GradientLevelPair pair:
				return new GlpV((GradV)FromRuntime(pair.ColorGradient), (CurveV)FromRuntime(pair.Curve));
			case System.Collections.IEnumerable list:
				return list.Cast<object>().Select(FromRuntime).ToArray();
			default:
				Console.WriteLine("note: default value of type " + v.GetType().Name + " not recorded");
				return null;
		}
	}

	private static void WriteExact(Utf8JsonWriter w, Snapshot s)
	{
		// Distinct state sets, distinct per-element timelines over them, and each element's timeline.
		var palette = new Interner();
		var tracks = new Interner();
		var trackEncoded = new List<List<int>>();
		var elementTrack = new int[s.Frames.Length];
		var buffer = new List<int>(64);
		for (var e = 0; e < s.Frames.Length; e++)
		{
			buffer.Clear();
			var frames = s.Frames[e];
			var i = 0;
			while (i < frames.Length)
			{
				var id = palette.Id(frames[i]);
				var run = 1;
				while (i + run < frames.Length && frames[i + run] == frames[i]) run++;
				buffer.Add(id);
				if (run > 1) buffer.Add(-(run - 1));
				i += run;
			}
			var key = Text.Join(buffer);
			var before = tracks.Items.Count;
			elementTrack[e] = tracks.Id(key);
			if (tracks.Items.Count > before) trackEncoded.Add(new List<int>(buffer));
		}

		w.WriteStartArray("pal");
		foreach (var p in palette.Items) w.WriteStringValue(p);
		w.WriteEndArray();
		w.WriteStartArray("tracks");
		foreach (var t in trackEncoded)
		{
			w.WriteStartArray();
			foreach (var v in t) w.WriteNumberValue(v);
			w.WriteEndArray();
		}
		w.WriteEndArray();
		WriteRuns(w, "el", elementTrack);

		var intents = new Interner();
		var intentIds = s.Intents.Select(intents.Id).ToArray();
		w.WriteStartArray("ints");
		foreach (var p in intents.Items) w.WriteStringValue(p);
		w.WriteEndArray();
		WriteRuns(w, "iel", intentIds);
	}

	/// <summary>Pairs of value and count: most targets share one timeline across many elements.</summary>
	private static void WriteRuns(Utf8JsonWriter w, string name, int[] values)
	{
		w.WriteStartArray(name);
		for (var i = 0; i < values.Length;)
		{
			var run = 1;
			while (i + run < values.Length && values[i + run] == values[i]) run++;
			w.WriteNumberValue(values[i]);
			w.WriteNumberValue(run);
			i += run;
		}
		w.WriteEndArray();
	}
}
