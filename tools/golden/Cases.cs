// The parameter variations each effect renders: one factor at a time from a few base configurations, the cross product of
// the enum and bool settings, and hand-picked combinations for branches those cannot reach.

using System.Reflection;
using VixenModules.App.ColorGradients;
using VixenModules.App.Curves;
using VixenModules.Effect.Alternating;
using VixenModules.Effect.Borders;
using VixenModules.Effect.Chase;
using VixenModules.Effect.Circles;
using VixenModules.Effect.ColorWash;
using VixenModules.Effect.Curtain;
using VixenModules.Effect.Dissolve;
using VixenModules.Effect.Garlands;
using VixenModules.Effect.Life;
using VixenModules.Effect.Meteors;
using VixenModules.Effect.PinWheel;
using VixenModules.Effect.Plasma;
using VixenModules.Effect.Pulse;
using VixenModules.Effect.SetLevel;
using VixenModules.Effect.SnowStorm;
using VixenModules.Effect.Spin;
using VixenModules.Effect.Spirograph;
using VixenModules.Effect.Strobe;
using VixenModules.Effect.Tree;
using VixenModules.Effect.Wipe;

namespace VixenGolden;

public static partial class Cases
{
	private static readonly Dictionary<object, string> Names = BuildNames();

	private static Dictionary<object, string> BuildNames()
	{
		var d = new Dictionary<object, string>(ReferenceEqualityComparer.Instance);
		foreach (var f in typeof(V).GetFields(BindingFlags.Public | BindingFlags.Static))
			d[f.GetValue(null)] = f.Name;
		return d;
	}

	private static string NameOf(object v) => v switch
	{
		null => "null",
		bool b => b ? "true" : "false",
		string s => s,
		_ when Names.TryGetValue(v, out var n) => n,
		double d => Json.Num(d),
		CurveV => "curve",
		GradV => "gradient",
		_ => v.ToString(),
	};

	private static (string, object)[] P(params (string, object)[] p) => p;

	private static (string, object)[] Merge((string, object)[] a, params (string, object)[] b)
	{
		var list = a.Where(x => b.All(y => y.Item1 != x.Item1)).ToList();
		list.AddRange(b);
		return list.ToArray();
	}

	private static string Label((string, object)[] p) => string.Join(",", p.Select(x => $"{x.Item1}={NameOf(x.Item2)}"));

	private static IEnumerable<PropertyInfo> ValueProperties(Type t) =>
		t.GetProperties(BindingFlags.Public | BindingFlags.Instance).Where(p => p.GetCustomAttributes().Any(a => a.GetType().Name == "ValueAttribute"));

	private static object[] ValuesFor(PropertyInfo p)
	{
		var t = p.PropertyType;
		if (t.IsEnum) return Enum.GetNames(t).Cast<object>().ToArray();
		if (t == typeof(bool)) return [true, false];
		if (t == typeof(Curve)) return [V.RampUp, V.RampDown, V.Tri, V.Multi];
		if (t == typeof(ColorGradient)) return [V.Red1, V.RedBlue, V.Rainbow, V.Focused, V.Hard, V.Faded, V.Inset];
		if (t == typeof(List<ColorGradient>)) return [V.One, V.Two, V.Four];
		if (t == typeof(List<GradientLevelPair>)) return [V.Glp1, V.Glp2, V.Glp3];
		if (t == typeof(Color)) return [V.Orange, V.Red];
		var range = p.GetCustomAttributes().FirstOrDefault(a => a.GetType().Name == "NumberRangeAttribute");
		if (range != null && t == typeof(int))
		{
			var min = Convert.ToInt32(range.GetType().GetProperty("Minimum")!.GetValue(range));
			var max = Convert.ToInt32(range.GetType().GetProperty("Maximum")!.GetValue(range));
			if (max - min <= 10) return Enumerable.Range(min, max - min + 1).Cast<object>().ToArray();
			return new[] { min, min + (max - min) / 4, (min + max) / 2, max }.Distinct().Cast<object>().ToArray();
		}
		if (range != null && t == typeof(double))
		{
			var min = Convert.ToDouble(range.GetType().GetProperty("Minimum")!.GetValue(range));
			var max = Convert.ToDouble(range.GetType().GetProperty("Maximum")!.GetValue(range));
			return [min, max, min + (max - min) * 0.3];
		}
		Console.WriteLine($"note: no values for {p.DeclaringType?.Name}.{p.Name} ({t.Name})");
		return [];
	}

	private static readonly string[] SpecialProperties = ["TargetPositioning", "StringOrientation", "MarkCollectionId"];

	/// <summary>Every variation of each property, one at a time, on top of a base configuration.</summary>
	private static void Ofat(EffectSpec s, string target, int ms, (string, object)[] baseParams,
		Dictionary<string, object[]> overrides = null, string[] skip = null, string[] only = null)
	{
		var prefix = baseParams.Length == 0 ? "" : Label(baseParams) + ",";
		foreach (var prop in ValueProperties(s.EffectType))
		{
			if (SpecialProperties.Contains(prop.Name)) continue;
			if (skip != null && skip.Contains(prop.Name)) continue;
			if (only != null && !only.Contains(prop.Name)) continue;
			if (baseParams.Any(b => b.Item1 == prop.Name)) continue;
			object[] values = null;
			overrides?.TryGetValue(prop.Name, out values);
			values ??= ValuesFor(prop);
			foreach (var value in values)
				s.Add($"{target}/{ms}/{prefix}{prop.Name}={NameOf(value)}", target, ms, Merge(baseParams, (prop.Name, value)));
		}
	}

	/// <summary>
	/// The cross product of the enum and bool properties, or an evenly spread sample of it when it passes <paramref name="cap"/>.
	/// </summary>
	private static void Cross(EffectSpec s, string target, int ms, (string, object)[] baseParams, int cap, string[] skip = null, Dictionary<string, object[]> overrides = null)
	{
		var props = ValueProperties(s.EffectType)
			.Where(p => !SpecialProperties.Contains(p.Name) && (skip == null || !skip.Contains(p.Name)) && baseParams.All(b => b.Item1 != p.Name))
			.Where(p => (p.PropertyType.IsEnum || p.PropertyType == typeof(bool)) || (overrides != null && overrides.ContainsKey(p.Name)))
			.ToArray();
		var values = props.Select(p => overrides != null && overrides.TryGetValue(p.Name, out var o) ? o : ValuesFor(p)).ToArray();
		long total = 1;
		foreach (var v in values) total *= v.Length;
		// A stride coprime with the total walks through all the combinations evenly.
		var stride = 1L;
		if (total > cap)
		{
			stride = total / cap;
			while (Gcd(stride, total) != 1) stride++;
		}
		var count = Math.Min(total, cap);
		for (long n = 0; n < count; n++)
		{
			var index = n * stride % total;
			var ps = new List<(string, object)>(baseParams);
			var label = "";
			var rest = index;
			for (var i = 0; i < props.Length; i++)
			{
				var value = values[i][rest % values[i].Length];
				rest /= values[i].Length;
				ps.Add((props[i].Name, value));
				label += NameOf(value) + ",";
			}
			s.Add($"cross/{target}/{Label(baseParams)}/{label}", target, ms, ps.ToArray());
		}
	}

	private static long Gcd(long a, long b)
	{
		while (b != 0) (a, b) = (b, a % b);
		return a;
	}

	/// <summary>Cases every pixel effect gets: defaults over all targets and orientations, location mode and timing.</summary>
	private static void PixelCommon(EffectSpec s, bool location, params (string, object)[] baseParams)
	{
		var v = Merge(baseParams);
		foreach (var target in new[] { "single", "string30", "grid", "discrete", "discrete_rgb", "multi_discrete", "uneven", "shared", "tree4", "mixed_depth", "partial_loc", "dup_loc" })
			s.Add($"defaults/{target}", target, 1000, v);
		s.Add("defaults/string30/long", "string30", 3000, v);
		s.Add("defaults/grid/long", "grid", 2000, v);
		s.Add("defaults/grid/horizontal", "grid", 1000, Merge(v, ("StringOrientation", "Horizontal")));
		s.Add("defaults/string30/horizontal", "string30", 1000, Merge(v, ("StringOrientation", "Horizontal")));
		s.Add("defaults/uneven/horizontal", "uneven", 1000, Merge(v, ("StringOrientation", "Horizontal")));
		s.Add("defaults/shared/horizontal", "shared", 1000, Merge(v, ("StringOrientation", "Horizontal")));
		s.Add("defaults/grid/1230ms", "grid", 1230, v);
		s.Add("defaults/grid/49ms", "grid", 49, v);
		s.Add("defaults/grid/50ms", "grid", 50, v);
		s.Add("defaults/grid/zero-length", "grid", 0, v);
		s.Add("defaults/two-targets", "string30+string30b", 1000, v);
		s.Add("defaults/string30/start-offset", "string30", 1000, v).StartMs = 777;
		s.Add("defaults/two-targets-grid", "grid+string30", 1000, v);
		s.Add("defaults/oriented", "oriented", 1000, v);
		s.Add("defaults/oriented_v", "oriented_v", 1000, v);
		s.Add("defaults/oriented/set-back", "oriented", 1000, Merge(v, ("StringOrientation", "Vertical")));
		if (location)
		{
			foreach (var target in new[] { "grid", "string30", "uneven", "single", "partial_loc", "dup_loc", "discrete", "tree4" })
				s.Add($"location/{target}", target, 1000, Merge(v, ("TargetPositioning", "Locations")));
			s.Add("location/grid/1230ms", "grid", 1230, Merge(v, ("TargetPositioning", "Locations")));
			s.Add("location/two-targets", "string30+string30b", 1000, Merge(v, ("TargetPositioning", "Locations")));
		}
	}

	private static bool Has(EffectSpec s, string property) => s.EffectType.GetProperty(property) != null;

	/// <summary>The defaults and a sample of the settings at the 10 ms update interval the user's real profile runs at.</summary>
	private static void Frame10(EffectSpec s, bool location, int cap = 24, string[] crossSkip = null, Dictionary<string, object[]> overrides = null, params (string, object)[] baseParams)
	{
		s.FrameMs = 10;
		foreach (var target in new[] { "string30", "grid", "uneven", "discrete" })
			s.Add($"{target}/defaults", target, 500, baseParams);
		s.Add("grid/horizontal", "grid", 500, Merge(baseParams, ("StringOrientation", "Horizontal")));
		s.Add("grid/730ms", "grid", 730, baseParams);
		s.Add("string30/3ms", "string30", 3, baseParams);
		if (location) s.Add("grid/location", "grid", 500, Merge(baseParams, ("TargetPositioning", "Locations")));
		Cross(s, "grid", 500, baseParams, cap, crossSkip, overrides);
		Ofat(s, "grid", 500, baseParams, overrides, null, ValueProperties(s.EffectType).Where(p => p.PropertyType == typeof(Curve) || p.PropertyType == typeof(int)).Select(p => p.Name).ToArray());
		s.FrameMs = 50;
	}

	/// <summary>Effects that move at a user-set speed run backwards below the curve's midpoint.</summary>
	private static void SpeedCases(EffectSpec s, bool location, int cap, string[] crossSkip)
	{
		if (!Has(s, "SpeedCurve") || !Has(s, "MovementType")) return;
		if (!Enum.GetNames(s.EffectType.GetProperty("MovementType")!.PropertyType).Contains("Speed")) return;
		foreach (var speed in new[] { 0.0, 20.0, 45.0, 50.0, 90.0 })
		{
			var b = P(("MovementType", "Speed"), ("SpeedCurve", V.Flat(speed)));
			s.Add($"speed/{speed}", "grid", 1000, b);
			s.Add($"speed/{speed}/horizontal", "grid", 1000, Merge(b, ("StringOrientation", "Horizontal")));
			s.Add($"speed/{speed}/string30", "string30", 1000, b);
			if (location) s.Add($"speed/{speed}/location", "grid", 1000, Merge(b, ("TargetPositioning", "Locations")));
		}
		Cross(s, "grid", 500, P(("MovementType", "Speed"), ("SpeedCurve", V.Flat(20))), cap, crossSkip);
		if (location) Cross(s, "grid", 500, P(("MovementType", "Speed"), ("SpeedCurve", V.Flat(20)), ("TargetPositioning", "Locations")), cap, crossSkip);
	}

	private static EffectSpec Generic(EffectSpec s, bool location, int cap = 200, Dictionary<string, object[]> overrides = null, string[] crossSkip = null)
	{
		PixelCommon(s, location);
		Ofat(s, "grid", 1000, P(), overrides);
		var notEnums = ValueProperties(s.EffectType).Where(p => !p.PropertyType.IsEnum).Select(p => p.Name).ToArray();
		var notSwitches = ValueProperties(s.EffectType).Where(p => !p.PropertyType.IsEnum && p.PropertyType != typeof(bool)).Select(p => p.Name).ToArray();
		Ofat(s, "string30", 500, P(), overrides, null, notEnums);
		Ofat(s, "grid", 500, P(("StringOrientation", "Horizontal")), overrides, null, notSwitches);
		if (location) Ofat(s, "grid", 500, P(("TargetPositioning", "Locations")), overrides, null, notSwitches);
		Cross(s, "grid", 1000, P(), cap, crossSkip, null);
		Cross(s, "grid", 500, P(("StringOrientation", "Horizontal")), cap / 2, crossSkip);
		if (location) Cross(s, "grid", 500, P(("TargetPositioning", "Locations")), cap / 2, crossSkip);
		SpeedCases(s, location, cap / 4, crossSkip);
		if (Has(s, "LevelCurve"))
		{
			Cross(s, "grid", 500, P(("LevelCurve", V.Flat50)), cap / 4, crossSkip);
			if (location) Cross(s, "grid", 500, P(("LevelCurve", V.Flat50), ("TargetPositioning", "Locations")), cap / 4, crossSkip);
		}
		if (s.EffectType.GetProperty("Colors")?.PropertyType.IsGenericType == true)
		{
			s.Add("empty-colors", "grid", 500, ("Colors", Array.Empty<object>()));
			foreach (var selector in new[] { "ColorType", "Type", "ColorScheme" })
				if (s.EffectType.GetProperty(selector)?.PropertyType.IsEnum == true)
					foreach (var value in Enum.GetNames(s.EffectType.GetProperty(selector)!.PropertyType))
						s.Add($"empty-colors/{value}", "grid", 500, (selector, value), ("Colors", Array.Empty<object>()));
			if (location) s.Add("empty-colors/location", "grid", 500, ("Colors", Array.Empty<object>()), ("TargetPositioning", "Locations"));
		}
		Frame10(s, location, 24, crossSkip, overrides);
		return s;
	}

	public static List<EffectSpec> All()
	{
		var list = new List<EffectSpec>
		{
			SetLevelCases(),
			PulseCases(),
			WipeCases(),
			DissolveCases(),
			TwinkleCases(),
			ChaseCases(),
			SpinCases(),
			Generic(new EffectSpec("Plasma", new PlasmaDescriptor().TypeId), true),
			BarsCases(),
			CandleCases(),
			StrobeCases(),
			Generic(new EffectSpec("ColorWash", new ColorWashDescriptor().TypeId), true),
			AlternatingCases(),
			Generic(new EffectSpec("Butterfly", new Guid("e2ef89d5-c3b3-47e7-a776-f31b4193fc76")), true),
			Generic(new EffectSpec("Spiral", new Guid("3f629e8c-cd8d-467e-afab-1a3836b24342")), true),
			Generic(new EffectSpec("Circles", new CirclesDescriptor().TypeId), true),
			CurtainCases(),
			GarlandsCases(),
			Generic(new EffectSpec("Life", new LifeDescriptor().TypeId), false),
			MeteorsCases(),
			PinWheelCases(),
			Generic(new EffectSpec("SnowStorm", new SnowStormDescriptor().TypeId), false),
			Generic(new EffectSpec("Spirograph", new SpirographDescriptor().TypeId), false),
			Generic(new EffectSpec("Tree", new TreeDescriptor().TypeId), false),
			Generic(new EffectSpec("Border", new BorderDescriptor().TypeId), true),
			FireCases(),
			BallsCases(),
			Generic(new EffectSpec("Shockwave", new Guid("880280ea-c38b-442c-8f09-395db4eb698c")), true),
			FireworksCases(),
			Generic(new EffectSpec("Pattern", new Guid("4633de41-bb29-443f-a390-23ea0c464ab2")), true),
			WaveListCases(Generic(new EffectSpec("Wave", new Guid("e53bbb2d-d415-4e64-8f88-8cca96f2c8ff")), true)),
			WhirlListCases(Generic(new EffectSpec("Whirlpool", new Guid("c2c2ced8-a2a8-46f5-a4d4-43441e68c2bc")) { SettledOnSecondRender = true }, true)),
			CustomValueCases(),
			LauncherCases(),
			RdsCases(),
			MeterCases(new EffectSpec("VerticalMeter", new Guid("af92929a-f35e-4762-ba86-a901a1f4eb53"))),
			MeterCases(new EffectSpec("VUMeter", new Guid("8a360703-bf3f-456a-a798-13cd67287dde"))),
			MeterCases(new EffectSpec("Waveform", new Guid("2bedbc3f-b908-4b62-bf4d-6c77ce34be56"))),
		};
		return list;
	}

	private static EffectSpec FireCases()
	{
		var s = Generic(new EffectSpec("Fire", new Guid("7e1b81bd-1b69-4238-9f55-06462cdc374c")), true);
		// One simulation per target or per node at the effect depth.
		var individual = P(("TargetNodeHandling", "Individual"));
		s.Add("individual/two-targets", "string30+string30b", 1000, individual);
		s.Add("individual/two-targets-grid", "grid+string30", 1000, individual);
		s.Add("individual/same-target-twice", "string30+string30", 1000, individual);
		foreach (var target in new[] { "single", "string30", "grid", "tree4", "mixed_depth", "uneven", "shared", "discrete" })
			foreach (var depth in new[] { 0, 1, 2 })
				s.Add($"individual/{target}/depth{depth}", target, 1000, Merge(individual, ("DepthOfEffect", depth)));
		foreach (var location in new[] { "Bottom", "Top", "Left", "Right" })
			s.Add($"individual/grid/{location}", "grid", 1000, Merge(individual, ("Location", location)));
		return s;
	}

	private static EffectSpec BallsCases()
	{
		var s = Generic(new EffectSpec("Balls", new Guid("59ba6e70-dc4d-4108-b287-72adf0fd4f7c"), null, 2), true);
		foreach (var fill in Enum.GetNames(s.EffectType.GetProperty("BallFill")!.PropertyType))
		{
			s.Add($"empty-colors/{fill}", "grid", 500, ("BallFill", fill), ("Colors", Array.Empty<object>()));
			s.Add($"empty-colors/{fill}/location", "grid", 500, ("BallFill", fill), ("Colors", Array.Empty<object>()), ("TargetPositioning", "Locations"));
		}
		return s;
	}

	private static EffectSpec CustomValueCases()
	{
		var s = new EffectSpec("CustomValue", new Guid("0c4095bf-a9b4-425b-97b8-c0251dd5ad47"));
		foreach (var target in BasicTargets)
			s.Add($"defaults/{target}", target, 1000);
		s.Add("defaults/two-targets", "string30+string30b", 500);
		(string type, string property, object value)[] values =
		[
			("_8Bit", "Value8Bit", 0), ("_8Bit", "Value8Bit", 77), ("_8Bit", "Value8Bit", 255),
			("_16Bit", "Value16Bit", 0), ("_16Bit", "Value16Bit", 1000), ("_16Bit", "Value16Bit", 65535),
			("_32Bit", "Value32Bit", 0), ("_32Bit", "Value32Bit", 70000), ("_32Bit", "Value32Bit", 4294967295.0),
			("_64Bit", "Value64Bit", "0"), ("_64Bit", "Value64Bit", "5000000000"), ("_64Bit", "Value64Bit", "18446744073709551615"),
			("Color", "ColorValue", V.Orange), ("Color", "ColorValue", V.White),
			("String", "StringValue", "hello"), ("String", "StringValue", ""), ("String", "StringValue", "a|b,c <&>"),
		];
		for (var i = 0; i < values.Length; i++)
			s.Add($"{values[i].type}/{i}", "string30", 500, ("ValueType", values[i].type), (values[i].property, values[i].value));
		return s;
	}

	private static EffectSpec LauncherCases()
	{
		var s = new EffectSpec("Launcher", new Guid("635364fd-b942-47e0-8b82-55d5e77a27c8"));
		foreach (var target in BasicTargets)
			s.Add($"defaults/{target}", target, 1000);
		s.Add("defaults/two-targets", "string30+string30b", 500);
		s.Add("executable", "string30", 500, ("Executable", "/bin/show"));
		s.Add("arguments", "string30", 500, ("Arguments", "--go now"));
		s.Add("both", "string30", 500, ("Executable", "/bin/show"), ("Arguments", "--go, now|later"), ("Description", "Not part of the command"));
		return s;
	}

	private static EffectSpec RdsCases()
	{
		var s = new EffectSpec("RDS", new Guid("afb67313-a17b-4fc3-9e0d-b564934423d5"));
		// It sends to the targets themselves, not their leaves, so a group target throws.
		foreach (var target in new[] { "single", "string30", "single+string30", "string30+single" })
		{
			s.Add($"defaults/{target}", target, 1000);
			s.Add($"title/{target}", target, 1000, ("Title", "Jingle | Bells"));
		}
		s.Add("artist", "single", 500, ("Title", "Song"), ("Artist", "Not part of the command"));
		return s;
	}

	private const string Stereo = "stereo44";
	private const string Mono = "mono22";

	/// <summary>Effects driven by the sequence's audio. Every case has the audio unless it says otherwise.</summary>
	private static EffectSpec MeterCases(EffectSpec s)
	{
		foreach (var target in BasicTargets.Append("partial_loc"))
			s.Add($"defaults/{target}", target, 2000);
		s.Add("defaults/two-targets", "string30+string30b", 1000);
		s.Add("defaults/grid+string", "grid+string30", 1000);
		s.Add("no-audio", "string30", 1000).Audio = NoAudio;
		s.Add("mono", "string30", 2000).Audio = Mono;
		s.Add("mono/grid", "grid", 1000).Audio = Mono;
		s.Add("mono/start", "string30", 1000).Audio = Mono;
		s.Cases[^1].StartMs = 1111;
		// Where the effect sits in the track: quiet passages, silence, a start that lands between stereo frames, the end.
		foreach (var (start, ms) in new[] { (0, 5000), (777, 1000), (1000, 1000), (1333, 1000), (2000, 500), (2000, 1000), (2500, 500), (5500, 1000), (6000, 500), (7000, 500) })
			s.Add($"time/{start}+{ms}", "string30", ms).StartMs = start;
		foreach (var ms in new[] { 0, 1, 29, 30, 31, 59, 60, 61, 90 })
			s.Add($"short/{ms}", "string30", ms);
		Ofat(s, "string30", 2000, P(), new Dictionary<string, object[]>
		{
			["AttackTime"] = [1, 10, 100, 300],
			["DecayTime"] = [1, 10, 100, 1500, 5000],
			["LowPassFreq"] = [100, 1000, 8000, 20000],
			["HighPassFreq"] = [100, 1000, 8000, 20000],
		});
		foreach (var target in new[] { "grid", "discrete", "discrete_rgb", "multi_discrete", "single", "tree4" })
			Ofat(s, target, 1000, P(), null, null, ValueProperties(s.EffectType).Where(p => p.PropertyType.IsEnum || p.PropertyType == typeof(ColorGradient) || p.PropertyType == typeof(bool)).Select(p => p.Name).ToArray());
		foreach (var freq in new[] { 100, 1500, 3000, 6000, 9000, 15000, 20000 })
		{
			s.Add($"filters/low/{freq}", "string30", 1000, ("LowPass", true), ("LowPassFreq", freq));
			s.Add($"filters/high/{freq}", "string30", 1000, ("HighPass", true), ("HighPassFreq", freq));
			s.Add($"filters/both/{freq}", "string30", 1000, ("LowPass", true), ("LowPassFreq", freq), ("HighPass", true), ("HighPassFreq", 100));
			s.Add($"filters/low/{freq}/mono", "string30", 1000, ("LowPass", true), ("LowPassFreq", freq)).Audio = Mono;
		}
		foreach (var depth in new[] { 0, 1, 2, 5 })
			foreach (var target in new[] { "grid", "tree4", "mixed_depth", "uneven" })
				s.Add($"depth/{target}/{depth}", target, 1000, ("DepthOfEffect", depth));
		foreach (var position in new[] { (0, 100), (25, 95), (50, 50), (90, 10) })
			foreach (var style in new[] { "Linear", "Discrete" })
				s.Add($"positions/{style}/{position.Item1}-{position.Item2}", "string30", 1000, ("MeterColorStyle", style), ("GreenColorPosition", position.Item1), ("RedColorPosition", position.Item2));
		Cross(s, "string30", 1000, P(("Gain", 100)), 60);
		Cross(s, "grid", 1000, P(), 40);
		s.FrameMs = 10;
		s.Add("string30/defaults", "string30", 1000);
		s.Add("grid/defaults", "grid", 1000);
		s.Add("discrete/defaults", "discrete", 1000);
		s.FrameMs = 50;
		foreach (var c in s.Cases)
			c.Audio = c.Audio == NoAudio ? null : c.Audio ?? Stereo;
		return s;
	}

	/// <summary>Stands in for "no audio" until <see cref="MeterCases"/> finishes giving the other cases theirs.</summary>
	private const string NoAudio = "none";

	private static EffectSpec FireworksCases()
	{
		var s = Generic(new EffectSpec("Fireworks", new Guid("c2bae192-d884-454f-84e0-61b7d25b59d5"), null, 2), false);
		var firstAudio = s.Cases.Count;
		var audio = P(("FireworksSource", "Audio"));
		foreach (var target in new[] { "grid", "string30", "uneven", "discrete", "single" })
			s.Add($"audio/{target}", target, 3000, audio);
		s.Add("audio/start", "grid", 1000, audio).StartMs = 777;
		s.Add("audio/horizontal", "grid", 1000, Merge(audio, ("StringOrientation", "Horizontal")));
		foreach (var (sensitivity, spread) in new[] { (-200, 0), (-200, 3), (-100, 1), (-50, 10), (0, 2), (0, 40) })
			s.Add($"audio/sensitivity/{sensitivity}/{spread}", "grid", 2000, Merge(audio, ("Sensitivity", sensitivity), ("ExplosionSensitivity", spread)));
		Ofat(s, "grid", 2000, audio, new Dictionary<string, object[]>
		{
			["AttackTime"] = [1, 100, 300],
			["DecayTime"] = [1, 100, 1500],
			["LowPassFreq"] = [100, 8000],
			["HighPassFreq"] = [100, 8000],
		}, null, ["Gain", "Normalize", "HighPass", "HighPassFreq", "LowPass", "LowPassFreq", "AttackTime", "DecayTime", "ColorType", "ColorGradients"]);
		s.Add("audio/low-pass", "grid", 1000, Merge(audio, ("LowPass", true), ("LowPassFreq", 500)));
		s.Add("audio/high-pass", "grid", 1000, Merge(audio, ("HighPass", true), ("HighPassFreq", 5000)));
		foreach (var freq in new[] { 100, 1500, 3000, 6000, 9000, 15000, 20000 })
		{
			s.Add($"audio/low-pass/{freq}", "grid", 500, Merge(audio, ("LowPass", true), ("LowPassFreq", freq)));
			s.Add($"audio/high-pass/{freq}", "grid", 500, Merge(audio, ("HighPass", true), ("HighPassFreq", freq)));
		}
		s.Add("audio/mono", "grid", 1000, audio);
		s.Add("audio/past-the-end", "grid", 1000, audio).StartMs = 7000;
		s.Add("audio/runs-off-the-end", "grid", 1000, audio).StartMs = 5500;
		for (var i = firstAudio; i < s.Cases.Count; i++) s.Cases[i].Audio = s.Cases[i].Id == "audio/mono" ? Mono : Stereo;
		s.Add("audio/no-audio", "grid", 1000, audio);
		// The audio loads whatever the source is, so audio that cannot be read stops the other sources too.
		foreach (var source in new[] { "None", "MarkCollection" })
			foreach (var (name, start, ms) in new[] { ("past-the-end", 7000, 1000), ("zero-length", 0, 0), ("in-range", 500, 1000) })
			{
				var c = s.Add($"audio-present/{source}/{name}", "grid", ms, ("FireworksSource", source));
				c.StartMs = start;
				c.Audio = Stereo;
			}
		foreach (var source in new[] { "None", "MarkCollection", "Audio" })
		{
			var c = s.Add($"no-gradients/{source}", "grid", 1000, ("FireworksSource", source), ("ColorGradients", Array.Empty<object>()));
			c.Audio = Stereo;
			c.Marks = SampleMarks().ToList();
		}
		s.Add("no-gradients/no-audio", "grid", 1000, ("FireworksSource", "Audio"), ("ColorGradients", Array.Empty<object>()));

		var marks = P(("FireworksSource", "MarkCollection"));
		foreach (var collection in new[] { "Odd", "Beats", "Late" })
		{
			AddMarked(s, $"marks/{collection}", "grid", 1000, collection, marks);
			AddMarked(s, $"marks/{collection}/random", "grid", 1000, collection, Merge(marks, ("RandomVelocity", true), ("RandomParticles", true)));
			AddMarked(s, $"marks/{collection}/string30", "string30", 1000, collection, marks);
		}
		s.Add("marks/none", "grid", 1000, marks).Marks = SampleMarks().ToList();
		s.Add("marks/sequence-has-none", "grid", 1000, marks).Marks = new List<MarkDef>();
		return s;
	}

	private static EffectSpec PinWheelCases()
	{
		var s = Generic(new EffectSpec("PinWheel", new PinWheelDescriptor().TypeId, null, 2), true);
		foreach (var blade in new[] { "Flat", "ThreeD", "Inverted3D", "Fan" })
			foreach (var rotation in new[] { 0.0, 10.0, 45.0, 90.0 })
			{
				s.Add($"blade/{blade}/{rotation}", "grid", 500, ("PinWheelBladeType", blade), ("RotationCurve", V.Flat(rotation)));
				s.Add($"blade/{blade}/{rotation}/location", "grid", 500, ("PinWheelBladeType", blade), ("RotationCurve", V.Flat(rotation)), ("TargetPositioning", "Locations"));
				s.Add($"blade/{blade}/{rotation}/hub", "grid", 500, ("PinWheelBladeType", blade), ("RotationCurve", V.Flat(rotation)), ("CenterHubCurve", V.Flat(40)));
			}
		return s;
	}

	private static EffectSpec CurtainCases()
	{
		var s = Generic(new EffectSpec("Curtain", new CurtainDescriptor().TypeId), true);
		foreach (var edge in new[] { "Left", "Center", "Right", "Bottom", "Middle", "Top" })
			foreach (var direction in new[] { "CurtainOpen", "CurtainClose", "CurtainOpenClose", "CurtainCloseOpen" })
				foreach (var swag in new[] { 0, 1, 4, 10 })
					foreach (var movement in new[] { "Iterations", "Position" })
					{
						var b = P(("Edge", edge), ("Direction", direction), ("Swag", swag), ("MovementType", movement));
						s.Add($"swag/{edge}/{direction}/{swag}/{movement}", "grid", 500, b);
						s.Add($"swag/{edge}/{direction}/{swag}/{movement}/location", "grid", 500, Merge(b, ("TargetPositioning", "Locations")));
						if (swag == 4) s.Add($"swag/{edge}/{direction}/{swag}/{movement}/horizontal", "grid", 500, Merge(b, ("StringOrientation", "Horizontal")));
					}
		return s;
	}

	private static EffectSpec GarlandsCases()
	{
		var s = Generic(new EffectSpec("Garlands", new GarlandsDescriptor().TypeId), true);
		foreach (var type in new[] { 0, 1, 2, 3, 4 })
			foreach (var direction in new[] { "Up", "Down", "Left", "Right" })
				foreach (var movement in new[] { "Iterations", "Speed" })
				{
					var b = P(("Type", type), ("Direction", direction), ("MovementType", movement));
					s.Add($"type/{type}/{direction}/{movement}", "grid", 500, b);
					s.Add($"type/{type}/{direction}/{movement}/location", "grid", 500, Merge(b, ("TargetPositioning", "Locations")));
					s.Add($"type/{type}/{direction}/{movement}/horizontal", "grid", 500, Merge(b, ("StringOrientation", "Horizontal")));
					s.Add($"type/{type}/{direction}/{movement}/spacing", "grid", 500, Merge(b, ("SpacingCurve", V.Tri)));
				}
		return s;
	}

	private static EffectSpec MeteorsCases()
	{
		var s = Generic(new EffectSpec("Meteors", new MeteorsDescriptor().TypeId, null, 2), false, 300);
		var movements = new[] { "None", "Bounce", "Wrap", "Speed", "Wobble", "Wobble2" };
		foreach (var effect in new[] { "None", "RandomDirection", "Explode" })
		{
			foreach (var movement in movements)
			{
				var b = P(("MeteorEffect", effect), ("MeteorMovement", movement));
				s.Add($"m/{effect}/{movement}", "grid", 1000, b);
				s.Add($"m/{effect}/{movement}/fast", "grid", 1000, Merge(b, ("CenterSpeedCurve", V.Flat(80)), ("SpeedVariationCurve", V.Flat(60)), ("PixelCountCurve", V.Flat(40))));
				s.Add($"m/{effect}/{movement}/horizontal", "grid", 1000, Merge(b, ("StringOrientation", "Horizontal")));
				s.Add($"m/{effect}/{movement}/string30", "string30", 1000, b);
				s.Add($"m/{effect}/{movement}/zero-start", "grid", 1000, Merge(b, ("MeteorStartPosition", "ZeroPosition")));
				s.Add($"m/{effect}/{movement}/random-start", "grid", 1000, Merge(b, ("MeteorStartPosition", "Random")));
				s.Add($"m/{effect}/{movement}/per-string", "grid", 1000, Merge(b, ("MeteorPerString", true)));
				s.Add($"m/{effect}/{movement}/per-string-count", "grid", 1000, Merge(b, ("MeteorPerString", true), ("CountPerString", true), ("FlipDirection", true)));
			}
			// The inverted range is "upward motion".
			s.Add($"inverted-range/{effect}", "grid", 1000, ("MeteorEffect", effect), ("MinDirection", 200), ("MaxDirection", 120));
			s.Add($"range/{effect}", "grid", 1000, ("MeteorEffect", effect), ("MinDirection", 45), ("MaxDirection", 300));
			foreach (var direction in new[] { 1, 45, 90, 135, 180, 225, 270, 315, 360 })
				s.Add($"direction/{effect}/{direction}", "grid", 1000, ("MeteorEffect", effect), ("Direction", direction));
			// Meteors that burn holes in the ground.
			foreach (var ground in new[] { 10.0, 50.0 })
			{
				var b = P(("MeteorEffect", effect), ("EnableGroundLevel", true), ("GroundLevelCurve", V.Flat(ground)), ("GroundColor", V.RedBlue));
				s.Add($"ground/{effect}/{ground}", "grid", 2000, b);
				s.Add($"ground/{effect}/{ground}/varying", "grid", 2000, Merge(b, ("GroundLevelCurve", V.Tri)));
				s.Add($"ground/{effect}/{ground}/fast", "grid", 2000, Merge(b, ("CenterSpeedCurve", V.Flat(80)), ("PixelCountCurve", V.Flat(60)), ("Direction", 190)));
				s.Add($"ground/{effect}/{ground}/horizontal", "grid", 2000, Merge(b, ("StringOrientation", "Horizontal")));
			}
		}
		// The speed and wobble movements with everything turned up.
		foreach (var color in new[] { "RainBow", "Range", "Palette", "Gradient" })
		{
			s.Add($"speed-curves/{color}", "grid", 1000, ("MeteorMovement", "Speed"), ("ColorType", color), ("XCenterSpeedCurve", V.RampDown), ("XSpeedVariationCurve", V.Flat(80)), ("YCenterSpeedCurve", V.Tri), ("YSpeedVariationCurve", V.Flat(100)));
			s.Add($"speed-limits/{color}", "grid", 1000, ("MeteorMovement", "Speed"), ("ColorType", color), ("XCenterSpeedCurve", V.Flat(100)), ("XSpeedVariationCurve", V.Flat(100)), ("YCenterSpeedCurve", V.Flat(0)), ("YSpeedVariationCurve", V.Flat(100)));
			s.Add($"wobble-curves/{color}", "grid", 1000, ("MeteorMovement", "Wobble"), ("ColorType", color), ("WobbleCurve", V.Multi), ("WobbleVariationCurve", V.Flat(100)));
			s.Add($"wobble2-curves/{color}", "grid", 1000, ("MeteorMovement", "Wobble2"), ("ColorType", color), ("WobbleCurve", V.Tri), ("WobbleVariationCurve", V.Flat(100)));
			s.Add($"explode-trails/{color}", "grid", 1000, ("MeteorEffect", "Explode"), ("ColorType", color), ("LengthCurve", V.Flat(60)), ("TailTaperCurve", V.RampUp));
			s.Add($"bright/{color}", "grid", 1000, ("RandomBrightness", true), ("ColorType", color), ("Colors", V.Four));
		}
		return s;
	}

	private static readonly string[] BasicTargets = ["single", "string30", "grid", "discrete", "discrete_rgb", "multi_discrete", "tree4", "uneven", "shared", "mixed_depth"];

	// Curves that exercise the unusual shapes: unsorted points, points missing at the ends, steps.
	private static readonly CurveV Reversed = new((100, 0), (0, 100));
	private static readonly CurveV Inner = new((20, 30), (80, 70));
	private static readonly CurveV Step = new((0, 0), (50, 0), (50, 100), (100, 100));
	private static readonly object[] ExtraCurves = [Reversed, Inner, Step];

	private static EffectSpec SetLevelCases()
	{
		// The copy in the extras module: the compiled one changed what several targets do.
		var s = new EffectSpec("SetLevel", new Guid("32cff8e0-5b10-4466-a093-0d232c55aac1"), new SetLevelDescriptor().TypeId);
		foreach (var target in BasicTargets)
		{
			s.Add($"{target}/defaults", target, 1000);
			s.Add($"{target}/half-orange", target, 500, ("IntensityLevel", 0.5), ("Color", V.Orange));
			s.Add($"{target}/red", target, 500, ("IntensityLevel", 0.333), ("Color", V.Red));
			s.Add($"{target}/lime", target, 500, ("IntensityLevel", 1.0), ("Color", V.Lime));
			s.Add($"{target}/blue", target, 500, ("IntensityLevel", 0.75), ("Color", V.Blue));
		}
		s.Add("zero", "string30", 500, ("IntensityLevel", 0.0));
		s.Add("over-range", "string30", 500, ("IntensityLevel", 1.5));
		s.Add("under-range", "string30", 500, ("IntensityLevel", -0.5));
		s.Add("1230ms", "string30", 1230);
		s.Add("30ms", "string30", 30);
		s.Add("zero-length", "string30", 0);
		s.Add("two-targets", "string30+string30b", 500);
		s.Add("two-targets/discrete", "string30+discrete", 500, ("Color", V.Red));
		s.Add("two-targets/discrete-last-mismatch", "discrete+string30", 500, ("Color", V.Lime));
		return s;
	}

	private static EffectSpec PulseCases()
	{
		// The copy in the extras module: the compiled one changed what several targets do.
		var s = new EffectSpec("Pulse", new Guid("cbd76d3b-c924-40ff-bad6-d1437b3dbdc1"), new PulseDescriptor().TypeId);
		object[] curves = [V.RampUp, V.RampDown, V.Flat100, V.Flat50, V.Flat0, V.Tri, V.Multi, .. ExtraCurves];
		object[] gradients = [V.White1, V.Red1, V.Lime1, V.RedBlue, V.RgbStops, V.Rainbow, V.Focused, V.Hard, V.Faded, V.Inset];
		foreach (var target in new[] { "single", "string30", "discrete", "discrete_rgb", "multi_discrete" })
		{
			foreach (var c in curves)
				s.Add($"{target}/curve={NameOf(c)}", target, 1000, ("LevelCurve", c), ("ColorGradient", V.RedBlue));
			foreach (var c in curves)
				s.Add($"{target}/curve={NameOf(c)}/rgb", target, 1000, ("LevelCurve", c), ("ColorGradient", V.RgbStops));
			foreach (var g in gradients)
				s.Add($"{target}/gradient={NameOf(g)}", target, 1000, ("LevelCurve", V.Tri), ("ColorGradient", g));
		}
		foreach (var target in BasicTargets)
			s.Add($"{target}/defaults", target, 1000);
		foreach (var ms in new[] { 1230, 30, 3333, 1, 0 })
			s.Add($"single/{ms}ms", "single", ms, ("LevelCurve", V.Multi), ("ColorGradient", V.Rainbow));
		s.Add("string30/start-offset", "string30", 1000).StartMs = 777;
		s.Add("two-targets", "string30+string30b", 500, ("ColorGradient", V.RedBlue));
		s.Add("two-targets/discrete", "string30+discrete", 500, ("ColorGradient", V.RgbStops));
		s.Add("two-targets/discrete-first", "discrete+string30", 500, ("ColorGradient", V.RgbStops));
		return s;
	}

	private static EffectSpec WipeCases()
	{
		var s = new EffectSpec("Wipe", new WipeDescriptor().TypeId);
		var overrides = new Dictionary<string, object[]>
		{
			["PulseTime"] = [100, 400, 2500],
			["PassCount"] = [2, 5],
			["PulsePercent"] = [10.0, 66.6, 100.0],
			["DepthOfEffect"] = [1, 2],
			["XOffset"] = [-50.0, 50.0],
			["YOffset"] = [-30.0, 70.0],
			["Curve"] = [V.RampUp, V.RampDown, V.Flat100, V.Tri, V.Multi, .. ExtraCurves],
			["MovementCurve"] = [V.RampDown, V.Tri, V.Multi, .. ExtraCurves],
			["ColorGradient"] = [V.Red1, V.RedBlue, V.Rainbow, V.Hard, V.Focused],
		};
		var directions = new[] { "Vertical", "Horizontal", "DiagonalUp", "DiagonalDown", "Burst", "Circle", "Dimaond" };
		var movements = new[] { "Count", "PulseLength", "Movement" };
		foreach (var direction in directions)
		{
			foreach (var movement in movements)
			{
				var b = P(("Direction", direction), ("WipeMovement", movement));
				s.Add($"grid/{direction}/{movement}", "grid", 1000, b);
				s.Add($"grid/{direction}/{movement}/reverse", "grid", 1000, Merge(b, ("ReverseDirection", true)));
				s.Add($"string30/{direction}/{movement}", "string30", 1000, b);
				s.Add($"uneven/{direction}/{movement}", "uneven", 1000, b);
				s.Add($"partial_loc/{direction}/{movement}", "partial_loc", 1000, b);
				s.Add($"dup_loc/{direction}/{movement}", "dup_loc", 1000, b);
				s.Add($"grid/{direction}/{movement}/across", "grid", 1000, Merge(b, ("ColorHandling", "ColorAcrossItems"), ("ColorGradient", V.Rainbow)));
				s.Add($"grid/{direction}/{movement}/across-rev", "grid", 1000, Merge(b, ("ColorHandling", "ColorAcrossItems"), ("ColorGradient", V.Rainbow), ("ReverseColorDirection", false), ("ColorAcrossItemPerCount", false)));
				s.Add($"grid/{direction}/{movement}/gradient", "grid", 1000, Merge(b, ("ColorGradient", V.Rainbow)));
				s.Add($"grid/{direction}/{movement}/on", "grid", 1000, Merge(b, ("WipeOn", true)));
				s.Add($"grid/{direction}/{movement}/off", "grid", 1000, Merge(b, ("WipeOff", true)));
				s.Add($"grid/{direction}/{movement}/on-off", "grid", 1000, Merge(b, ("WipeOn", true), ("WipeOff", true)));
				s.Add($"grid/{direction}/{movement}/passes", "grid", 1000, Merge(b, ("PassCount", 3)));
				s.Add($"grid/{direction}/{movement}/offset", "grid", 1000, Merge(b, ("XOffset", 37.0), ("YOffset", -21.0)));
				s.Add($"grid/{direction}/{movement}/pct", "grid", 1000, Merge(b, ("PulsePercent", 12.5)));
				s.Add($"grid/{direction}/{movement}/time", "grid", 1000, Merge(b, ("PulseTime", 150)));
				s.Add($"discrete/{direction}/{movement}", "discrete", 1000, Merge(b, ("ColorGradient", V.Red1)));
				s.Add($"discrete_rgb/{direction}/{movement}", "discrete_rgb", 1000, Merge(b, ("ColorGradient", V.RedBlue)));
				s.Add($"multi_discrete/{direction}/{movement}", "multi_discrete", 1000, Merge(b, ("ColorGradient", V.RgbStops)));
				s.Add($"discrete_rgb/{direction}/{movement}/across", "discrete_rgb", 1000, Merge(b, ("ColorHandling", "ColorAcrossItems"), ("ColorGradient", V.RgbStops)));
			}
			s.Add($"grid/{direction}/individual", "grid", 1000, ("Direction", direction), ("TargetNodeHandling", "Individual"));
			s.Add($"grid/{direction}/individual-depth1", "grid", 1000, ("Direction", direction), ("TargetNodeHandling", "Individual"), ("DepthOfEffect", 1));
			s.Add($"grid/{direction}/depth1", "grid", 1000, ("Direction", direction), ("DepthOfEffect", 1));
			s.Add($"tree4/{direction}/individual", "tree4", 1000, ("Direction", direction), ("TargetNodeHandling", "Individual"));
			s.Add($"tree4/{direction}/individual-depth2", "tree4", 1000, ("Direction", direction), ("TargetNodeHandling", "Individual"), ("DepthOfEffect", 2));
		}
		Ofat(s, "grid", 1000, P(), overrides, ["Direction", "WipeMovement"]);
		foreach (var m in movements[1..])
			Ofat(s, "grid", 1000, P(("WipeMovement", m)), overrides, ["Direction", "WipeMovement"], ["PulseTime", "PassCount", "Curve", "MovementCurve", "PulsePercent"]);
		foreach (var d in new[] { "Burst", "Circle", "Dimaond" })
			Ofat(s, "grid", 1000, P(("Direction", d), ("XOffset", 20.0)), overrides, ["Direction", "XOffset"], ["YOffset", "PassCount", "ReverseDirection"]);
		foreach (var target in new[] { "tree4", "uneven", "shared", "single", "mixed_depth", "discrete", "discrete_rgb", "multi_discrete", "partial_loc", "dup_loc" })
			s.Add($"{target}/defaults", target, 1000);
		s.Add("tree4/depth1", "tree4", 1000, ("DepthOfEffect", 1));
		s.Add("tree4/individual", "tree4", 1000, ("TargetNodeHandling", "Individual"));
		s.Add("tree4/individual-depth1", "tree4", 1000, ("TargetNodeHandling", "Individual"), ("DepthOfEffect", 1));
		s.Add("two-targets", "string30+string30b", 1000);
		s.Add("two-targets/individual", "string30+string30b", 1000, ("TargetNodeHandling", "Individual"));
		foreach (var ms in new[] { 1230, 30, 3000, 1, 0 })
			s.Add($"grid/{ms}ms", "grid", ms);
		// Wipe on and off with colours across items, each with one and several passes.
		foreach (var direction in new[] { "Horizontal", "Burst" })
		{
			foreach (var passes in new[] { 1, 3 })
			{
				var across = P(("Direction", direction), ("ColorHandling", "ColorAcrossItems"), ("PassCount", passes));
				foreach (var (target, gradient) in new[] { ("grid", V.Rainbow), ("discrete_rgb", V.RgbStops), ("multi_discrete", V.RgbStops), ("discrete", V.Red1) })
				{
					s.Add($"across/{direction}/{passes}/{target}/on", target, 1000, Merge(across, ("ColorGradient", gradient), ("WipeOn", true)));
					s.Add($"across/{direction}/{passes}/{target}/off", target, 1000, Merge(across, ("ColorGradient", gradient), ("WipeOff", true)));
					s.Add($"across/{direction}/{passes}/{target}/both", target, 1000, Merge(across, ("ColorGradient", gradient), ("WipeOn", true), ("WipeOff", true)));
				}
				s.Add($"count/{direction}/{passes}/on", "grid", 1000, ("Direction", direction), ("PassCount", passes), ("WipeOn", true));
				s.Add($"count/{direction}/{passes}/off", "grid", 1000, ("Direction", direction), ("PassCount", passes), ("WipeOff", true));
			}
		}
		foreach (var (name, handling) in new[] { ("gradient", "GradientThroughWholeEffect"), ("across", "ColorAcrossItems") })
		{
			s.Add($"zero-curve/{name}/on", "grid", 1000, ("Curve", V.Flat0), ("ColorHandling", handling), ("WipeOn", true));
			s.Add($"zero-curve/{name}/off", "grid", 1000, ("Curve", V.Flat0), ("ColorHandling", handling), ("WipeOff", true));
			s.Add($"zero-curve/{name}/plain", "grid", 1000, ("Curve", V.Flat0), ("ColorHandling", handling));
		}
		s.Add("string30/individual", "string30", 1000, ("TargetNodeHandling", "Individual"));
		s.Add("string30/individual-depth", "string30", 1000, ("TargetNodeHandling", "Individual"), ("DepthOfEffect", 1));
		s.Add("grid/individual-depth-too-deep", "grid", 1000, ("TargetNodeHandling", "Individual"), ("DepthOfEffect", 7));
		s.Add("grid+tree4/defaults", "grid+tree4", 1000);
		s.Add("grid+tree4/individual", "grid+tree4", 1000, ("TargetNodeHandling", "Individual"));
		s.FrameMs = 10;
		foreach (var direction in directions)
			foreach (var movement in movements)
				s.Add($"grid/{direction}/{movement}", "grid", 500, ("Direction", direction), ("WipeMovement", movement));
		s.FrameMs = 50;
		return s;
	}

	private static MarkDef[] SampleMarks() =>
	[
		new MarkDef
		{
			Name = "Odd",
			Id = Guid.Parse("11111111-2222-3333-4444-777777777777"),
			Items = [(0, 50, "-5"), (100, 150, "12"), (250, 300, "-50"), (400, 450, "x"), (600, 650, "80"), (800, 1500, "5")],
		},
		new MarkDef
		{
			Name = "Beats",
			Id = Guid.Parse("11111111-2222-3333-4444-555555555555"),
			Items = [(0, 100, "100"), (150, 300, "50"), (400, 650, "75"), (700, 900, "10"), (950, 1200, "200")],
		},
		new MarkDef
		{
			Name = "Late",
			Id = Guid.Parse("11111111-2222-3333-4444-666666666666"),
			Items = [(120, 220, "20"), (333, 433, "x"), (500, 900, "30")],
		},
	];

	private static void AddMarked(EffectSpec s, string id, string target, int ms, string collection, params (string, object)[] ps)
	{
		s.Add(id, target, ms, Merge(ps, ("MarkCollectionByName", collection))).Marks = SampleMarks().ToList();
		// The same on an effect that starts later, so one mark is still running when it starts and others end past it.
		foreach (var start in new[] { 200, 420 })
		{
			var late = s.Add($"{id}/start{start}", target, ms, Merge(ps, ("MarkCollectionByName", collection)));
			late.Marks = SampleMarks().ToList();
			late.StartMs = start;
		}
	}

	private static EffectSpec DissolveCases()
	{
		var s = new EffectSpec("Dissolve", new DissolveDescriptor().TypeId);
		var overrides = new Dictionary<string, object[]>
		{
			["DepthOfEffect"] = [1, 2],
			["GroupLevel"] = [1, 3, 10],
			["StartingNode"] = [1, 5, 20, 45],
			["DissolveCurve"] = [V.RampUp, V.RampDown, V.Tri, V.Multi, .. ExtraCurves],
			["Colors"] = [V.Glp1, V.Glp2, V.Glp3],
			["DissolveMode"] = ["TimeInterval"],
		};
		foreach (var target in BasicTargets)
			s.Add($"{target}/defaults", target, 1000);
		// Random order off is the deterministic path.
		var det = P(("RandomDissolve", false), ("RandomColor", false));
		foreach (var target in BasicTargets)
		{
			s.Add($"{target}/ordered", target, 1000, det);
			s.Add($"{target}/ordered-colors", target, 1000, Merge(det, ("Colors", V.Glp3)));
			s.Add($"{target}/ordered-group", target, 1000, Merge(det, ("GroupColors", true), ("Colors", V.Glp2)));
		}
		Ofat(s, "string30", 1000, det, overrides, ["RandomDissolve", "RandomColor"]);
		Ofat(s, "grid", 1000, det, overrides, ["RandomDissolve", "RandomColor", "DissolveMarkType", "Colors"]);
		Ofat(s, "string30", 1000, P(), overrides, ["DissolveMarkType"]);
		Cross(s, "string30", 1000, det, 64, ["DissolveMode", "DissolveMarkType"]);
		s.Add("ordered/both", "string30", 1000, Merge(det, ("BothDirections", true)));
		s.Add("ordered/both-together", "string30", 1000, Merge(det, ("BothDirections", true), ("DirectionsTogether", true)));
		s.Add("ordered/both-flip", "string30", 1000, Merge(det, ("BothDirections", true), ("DissolveFlip", false)));
		s.Add("ordered/both-colors", "string30", 1000, Merge(det, ("BothDirections", true), ("ColorPerStep", false), ("Colors", V.Glp3)));
		s.Add("ordered/depth", "grid", 1000, Merge(det, ("EnableDepth", true), ("DepthOfEffect", 1)));
		s.Add("ordered/depth2", "tree4", 1000, Merge(det, ("EnableDepth", true), ("DepthOfEffect", 2)));
		s.Add("ordered/depth-too-deep", "tree4", 1000, Merge(det, ("EnableDepth", true), ("DepthOfEffect", 7)));
		s.Add("ordered/two-targets", "string30+string30b", 1000, det);
		s.Add("empty-colors", "string30", 1000, Merge(det, ("Colors", Array.Empty<object>())));
		s.Add("ordered/1230ms", "string30", 1230, det);
		s.Add("ordered/30ms", "string30", 30, det);
		s.Add("ordered/zero-length", "string30", 0, det);
		foreach (var type in new[] { "PerMark", "PerMarkFill", "PerMarkDissolve", "MarkLabelValue", "MarkLabelPixels" })
		{
			var b = Merge(det, ("DissolveMode", "MarkCollection"), ("DissolveMarkType", type));
			foreach (var collection in new[] { "Beats", "Late", "Odd" })
			{
				AddMarked(s, $"marks/{type}/{collection}", "string30", 1000, collection, b);
				AddMarked(s, $"marks/{type}/{collection}/single-color-step", "string30", 1000, collection, Merge(b, ("ColorPerStep", false)));
				AddMarked(s, $"marks/{type}/{collection}/both", "string30", 1000, collection, Merge(b, ("BothDirections", true)));
				AddMarked(s, $"marks/{type}/{collection}/both-together", "string30", 1000, collection, Merge(b, ("BothDirections", true), ("DirectionsTogether", true)));
				AddMarked(s, $"marks/{type}/{collection}/colors", "string30", 1000, collection, Merge(b, ("Colors", V.Glp3)));
				AddMarked(s, $"marks/{type}/{collection}/group", "string30", 1000, collection, Merge(b, ("GroupLevel", 4)));
				AddMarked(s, $"marks/{type}/{collection}/grid", "grid", 1000, collection, Merge(b, ("EnableDepth", true), ("DepthOfEffect", 1)));
				AddMarked(s, $"marks/{type}/{collection}/random", "string30", 1000, collection, ("DissolveMode", "MarkCollection"), ("DissolveMarkType", type));
			}
			s.Add($"marks/{type}/no-collection", "string30", 1000, Merge(b)).Marks = SampleMarks().ToList();
			s.Add($"marks/{type}/sequence-has-none", "string30", 1000, Merge(b)).Marks = new List<MarkDef>();
			s.Add($"marks/{type}/sequence-has-none/single-step", "string30", 1000, Merge(b, ("ColorPerStep", false))).Marks = new List<MarkDef>();
		}
		s.FrameMs = 10;
		s.Add("string30/ordered", "string30", 500, det);
		s.Add("grid/ordered", "grid", 500, det);
		s.Add("string30/random", "string30", 500);
		s.FrameMs = 50;
		RealDissolve(s);
		return s;
	}

	private static EffectSpec TwinkleCases()
	{
		// The copy in the extras module, identical but for a random source that can be scripted.
		var s = new EffectSpec("Twinkle", new Guid("83bdd6f7-19c7-4598-b8e3-7ce28c44e7dc"), new Guid("83bdd6f7-19c7-4598-b8e3-7ce28c44e7db"));
		var overrides = new Dictionary<string, object[]>
		{
			["MinimumLevel"] = [0.2, 0.5],
			["MaximumLevel"] = [0.5, 0.8],
			["LevelVariation"] = [0, 100],
			["AveragePulseTime"] = [100, 1000],
			["PulseTimeVariation"] = [0, 100],
			["AverageCoverage"] = [10, 90, 100],
			["DepthOfEffect"] = [1, 2],
			["ColorGradient"] = [V.Red1, V.RedBlue, V.Rainbow],
		};
		foreach (var target in BasicTargets)
		{
			s.Add($"{target}/defaults", target, 1500);
			s.Add($"{target}/static", target, 1500, ("ColorHandling", "StaticColor"), ("StaticColor", V.Red));
			s.Add($"{target}/whole", target, 1500, ("ColorHandling", "GradientThroughWholeEffect"), ("ColorGradient", V.RgbStops));
			s.Add($"{target}/each", target, 1500, ("ColorHandling", "GradientForEachPulse"), ("ColorGradient", V.RgbStops));
			s.Add($"{target}/across", target, 1500, ("ColorHandling", "ColorAcrossItems"), ("ColorGradient", V.RgbStops));
			s.Add($"{target}/min-static", target, 1500, ("ColorHandling", "StaticColor"), ("StaticColor", V.Red), ("MinimumLevel", 0.25));
			s.Add($"{target}/min-whole", target, 1500, ("ColorHandling", "GradientThroughWholeEffect"), ("ColorGradient", V.RgbStops), ("MinimumLevel", 0.25));
			s.Add($"{target}/min-each", target, 1500, ("ColorHandling", "GradientForEachPulse"), ("ColorGradient", V.RgbStops), ("MinimumLevel", 0.25));
			s.Add($"{target}/min-across", target, 1500, ("ColorHandling", "ColorAcrossItems"), ("ColorGradient", V.RgbStops), ("MinimumLevel", 0.25));
		}
		Ofat(s, "string30", 1500, P(), overrides);
		Ofat(s, "grid", 1500, P(("IndividualElements", false)), overrides, ["IndividualElements"], ["ColorHandling", "AverageCoverage", "DepthOfEffect"]);
		Ofat(s, "grid", 1500, P(("DepthOfEffect", 1)), overrides, ["DepthOfEffect"], ["ColorHandling", "IndividualElements"]);
		var fixedPulse = P(("LevelVariation", 0), ("PulseTimeVariation", 0), ("AverageCoverage", 100));
		s.Add("fixed/string30", "string30", 1500, fixedPulse);
		s.Add("fixed/grid", "grid", 1500, fixedPulse);
		s.Add("fixed/static", "string30", 1500, Merge(fixedPulse, ("ColorHandling", "StaticColor"), ("StaticColor", V.Orange), ("MinimumLevel", 0.2), ("MaximumLevel", 0.7)));
		s.Add("fixed/gradient", "string30", 1500, Merge(fixedPulse, ("ColorHandling", "GradientThroughWholeEffect"), ("ColorGradient", V.Rainbow)));
		s.Add("fixed/discrete", "discrete", 1500, Merge(fixedPulse, ("ColorGradient", V.Red1)));
		s.Add("two-targets", "string30+string30b", 1500);
		s.Add("1230ms", "string30", 1230);
		s.Add("40ms", "string30", 40);
		s.Add("40ms/min-level", "string30", 40, ("MinimumLevel", 0.3));
		s.Add("40ms/discrete", "discrete", 40, ("ColorHandling", "StaticColor"), ("StaticColor", V.Red), ("MinimumLevel", 0.3));
		s.Add("zero-coverage", "string30", 1500, ("AverageCoverage", 0));
		s.Add("zero-pulse-time", "string30", 1500, ("AveragePulseTime", 0));
		s.Add("group-zero-pulse-time", "string30", 1500, ("AveragePulseTime", 0), ("IndividualElements", false));
		s.Add("depth-too-deep", "tree4", 1500, ("DepthOfEffect", 9));
		s.FrameMs = 10;
		foreach (var h in new[] { "StaticColor", "GradientThroughWholeEffect", "GradientForEachPulse", "ColorAcrossItems" })
		{
			s.Add($"string30/{h}", "string30", 700, ("ColorHandling", h), ("ColorGradient", V.RgbStops), ("StaticColor", V.Red));
			s.Add($"discrete_rgb/{h}", "discrete_rgb", 700, ("ColorHandling", h), ("ColorGradient", V.RgbStops), ("StaticColor", V.Red));
		}
		s.Add("3ms", "string30", 3);
		s.FrameMs = 50;
		RealTwinkle(s);
		return s;
	}

	private static EffectSpec ChaseCases()
	{
		var s = new EffectSpec("Chase", new ChaseDescriptor().TypeId);
		var overrides = new Dictionary<string, object[]>
		{
			["DefaultLevel"] = [0.25, 0.75],
			["PulseOverlap"] = [50, 200, 1000, 5000],
			["DepthOfEffect"] = [1, 2],
			["ColorGradient"] = [V.Red1, V.RedBlue, V.Rainbow, V.Hard, V.Focused],
			["PulseCurve"] = [V.RampDown, V.Flat100, V.Tri, V.Multi, .. ExtraCurves],
			["ChaseMovement"] = [V.RampDown, V.Tri, V.Multi, V.Flat50, .. ExtraCurves],
		};
		var handlings = new[] { "StaticColor", "GradientThroughWholeEffect", "GradientForEachPulse", "ColorAcrossItems" };
		foreach (var target in BasicTargets)
		{
			s.Add($"{target}/defaults", target, 1000);
			foreach (var h in handlings)
			{
				s.Add($"{target}/{h}", target, 1000, ("ColorHandling", h), ("ColorGradient", V.RgbStops), ("StaticColor", V.Red));
				s.Add($"{target}/{h}/default-level", target, 1000, ("ColorHandling", h), ("ColorGradient", V.RgbStops), ("StaticColor", V.Red), ("EnableDefaultLevel", true), ("DefaultLevel", 0.3));
				s.Add($"{target}/{h}/extend", target, 1000, ("ColorHandling", h), ("ColorGradient", V.RgbStops), ("StaticColor", V.Red), ("ExtendPulseToStart", true), ("ExtendPulseToEnd", true), ("PulseCurve", V.Flat100));
				s.Add($"{target}/{h}/extend-tri", target, 1000, ("ColorHandling", h), ("ColorGradient", V.RgbStops), ("StaticColor", V.Red), ("ExtendPulseToStart", true), ("ExtendPulseToEnd", true), ("PulseCurve", V.Multi));
				s.Add($"{target}/{h}/overlap", target, 1000, ("ColorHandling", h), ("ColorGradient", V.RgbStops), ("StaticColor", V.Red), ("PulseOverlap", 120));
			}
		}
		Ofat(s, "string30", 1000, P(), overrides);
		foreach (var ch in handlings[1..])
			Ofat(s, "string30", 1000, P(("ColorHandling", ch), ("ColorGradient", V.Rainbow)), overrides, null, ["PulseOverlap", "ChaseMovement", "PulseCurve", "ExtendPulseToStart", "ExtendPulseToEnd", "EnableDefaultLevel"]);
		foreach (var ch in handlings)
			Cross(s, "grid", 1000, P(("ColorHandling", ch), ("ColorGradient", V.Rainbow), ("DepthOfEffect", 1)), 16, null, null);
		s.Add("grid/group-depth1", "grid", 1000, ("DepthOfEffect", 1));
		s.Add("grid/individual", "grid", 1000, ("TargetNodeHandling", "Individual"));
		s.Add("grid/individual-depth1", "grid", 1000, ("TargetNodeHandling", "Individual"), ("DepthOfEffect", 1));
		s.Add("tree4/depth1", "tree4", 1000, ("DepthOfEffect", 1));
		s.Add("tree4/depth2", "tree4", 1000, ("DepthOfEffect", 2));
		s.Add("tree4/depth-too-deep", "tree4", 1000, ("DepthOfEffect", 9));
		s.Add("tree4/individual", "tree4", 1000, ("TargetNodeHandling", "Individual"));
		s.Add("tree4/individual-depth1", "tree4", 1000, ("TargetNodeHandling", "Individual"), ("DepthOfEffect", 1));
		s.Add("tree4/individual-depth2", "tree4", 1000, ("TargetNodeHandling", "Individual"), ("DepthOfEffect", 2));
		s.Add("string30/extend-move", "string30", 1000, ("ExtendPulseToStart", true), ("ExtendPulseToEnd", true), ("PulseCurve", V.Flat100), ("ChaseMovement", V.Flat50));
		s.Add("string30/inner-move", "string30", 1000, ("ChaseMovement", Inner), ("PulseCurve", Inner));
		s.Add("two-targets", "string30+string30b", 1000);
		s.Add("two-targets/individual", "string30+string30b", 1000, ("TargetNodeHandling", "Individual"));
		s.Add("two-targets/depth", "string30+string30b", 1000, ("DepthOfEffect", 1));
		s.Add("two-targets/default-level", "string30+string30b", 1000, ("EnableDefaultLevel", true), ("DefaultLevel", 0.3));
		foreach (var ms in new[] { 1230, 30, 3000, 1, 0 })
			s.Add($"string30/{ms}ms", "string30", ms);
		s.Add("string30/overlap-longer-than-effect", "string30", 100, ("PulseOverlap", 500));
		s.Add("grid+tree4/defaults", "grid+tree4", 1000);
		s.Add("grid+tree4/individual", "grid+tree4", 1000, ("TargetNodeHandling", "Individual"));
		s.Add("grid+tree4/individual-depth1", "grid+tree4", 1000, ("TargetNodeHandling", "Individual"), ("DepthOfEffect", 1));
		s.Add("grid+tree4/depth1", "grid+tree4", 1000, ("DepthOfEffect", 1));
		s.Add("tree4+discrete_rgb/gradient", "tree4+discrete_rgb", 1000, ("ColorHandling", "GradientThroughWholeEffect"), ("ColorGradient", V.RgbStops));
		foreach (var h in handlings)
		{
			s.Add($"partial-gradient/{h}", "string30", 1000, ("ColorHandling", h), ("ColorGradient", V.Inset), ("PulseCurve", Inner), ("ChaseMovement", Inner));
			s.Add($"movement-reversed/{h}", "string30", 1000, ("ColorHandling", h), ("ColorGradient", V.Rainbow), ("ChaseMovement", Reversed), ("ExtendPulseToStart", true), ("ExtendPulseToEnd", true));
		}
		s.FrameMs = 10;
		s.Add("string30/defaults", "string30", 500);
		s.Add("grid/defaults", "grid", 500);
		s.Add("discrete_rgb/gradient", "discrete_rgb", 500, ("ColorHandling", "GradientThroughWholeEffect"), ("ColorGradient", V.RgbStops));
		s.FrameMs = 50;
		return s;
	}

	private static EffectSpec SpinCases()
	{
		var s = new EffectSpec("Spin", new SpinDescriptor().TypeId);
		var overrides = new Dictionary<string, object[]>
		{
			["RevolutionCount"] = [1.0, 2.5, 7.0, 0.3],
			["RevolutionFrequency"] = [1.0, 3.3],
			["RevolutionTime"] = [200, 1000],
			["PulseTime"] = [40, 300],
			["PulseOverlap"] = [50, 300],
			["PulsePercentage"] = [25, 100],
			["DefaultLevel"] = [0.25, 0.75],
			["DepthOfEffect"] = [1, 2],
			["ColorGradient"] = [V.Red1, V.RedBlue, V.Rainbow, V.Hard, V.Focused],
			["PulseCurve"] = [V.RampDown, V.Flat100, V.Tri, V.Multi, .. ExtraCurves],
		};
		var handlings = new[] { "StaticColor", "GradientThroughWholeEffect", "GradientForEachPulse", "ColorAcrossItems" };
		foreach (var target in BasicTargets)
		{
			s.Add($"{target}/defaults", target, 1000);
			foreach (var h in handlings)
			{
				s.Add($"{target}/{h}", target, 1000, ("ColorHandling", h), ("ColorGradient", V.RgbStops), ("StaticColor", V.Red));
				s.Add($"{target}/{h}/default-level", target, 1000, ("ColorHandling", h), ("ColorGradient", V.RgbStops), ("StaticColor", V.Red), ("EnableDefaultLevel", true), ("DefaultLevel", 0.3));
				s.Add($"{target}/{h}/reverse", target, 1000, ("ColorHandling", h), ("ColorGradient", V.RgbStops), ("StaticColor", V.Red), ("ReverseSpin", true));
				s.Add($"{target}/{h}/fixed-pulse", target, 1000, ("ColorHandling", h), ("ColorGradient", V.RgbStops), ("StaticColor", V.Red), ("PulseLengthFormat", "FixedTime"), ("PulseTime", 80));
				s.Add($"{target}/{h}/pct-pulse", target, 1000, ("ColorHandling", h), ("ColorGradient", V.RgbStops), ("StaticColor", V.Red), ("PulseLengthFormat", "PercentageOfRevolution"), ("PulsePercentage", 40));
			}
		}
		Ofat(s, "string30", 1000, P(), overrides);
		foreach (var sf in new[] { "RevolutionFrequency", "FixedTime" })
			Ofat(s, "string30", 1000, P(("SpeedFormat", sf)), overrides, null, ["PulseLengthFormat", "RevolutionFrequency", "RevolutionTime", "ReverseSpin"]);
		foreach (var pf in new[] { "FixedTime", "PercentageOfRevolution" })
			Ofat(s, "string30", 1000, P(("PulseLengthFormat", pf)), overrides, null, ["PulseTime", "PulsePercentage", "PulseOverlap", "SpeedFormat"]);
		Cross(s, "string30", 1000, P(), 150);
		s.Add("grid/depth1", "grid", 1000, ("DepthOfEffect", 1));
		s.Add("grid/individual", "grid", 1000, ("TargetNodeHandling", "Individual"));
		s.Add("grid/individual-depth1", "grid", 1000, ("TargetNodeHandling", "Individual"), ("DepthOfEffect", 1));
		s.Add("tree4/depth1", "tree4", 1000, ("DepthOfEffect", 1));
		s.Add("tree4/depth2", "tree4", 1000, ("DepthOfEffect", 2));
		s.Add("tree4/depth-too-deep", "tree4", 1000, ("DepthOfEffect", 9));
		s.Add("tree4/individual", "tree4", 1000, ("TargetNodeHandling", "Individual"));
		s.Add("tree4/individual-depth1", "tree4", 1000, ("TargetNodeHandling", "Individual"), ("DepthOfEffect", 1));
		s.Add("two-targets", "string30+string30b", 1000);
		s.Add("two-targets/individual", "string30+string30b", 1000, ("TargetNodeHandling", "Individual"));
		s.Add("two-targets/default-level", "string30+string30b", 1000, ("EnableDefaultLevel", true), ("DefaultLevel", 0.3));
		foreach (var ms in new[] { 1230, 30, 3000, 1, 0 })
			s.Add($"string30/{ms}ms", "string30", ms);
		s.Add("grid+tree4/defaults", "grid+tree4", 1000);
		s.Add("grid+tree4/individual", "grid+tree4", 1000, ("TargetNodeHandling", "Individual"));
		s.Add("grid+tree4/individual-depth1", "grid+tree4", 1000, ("TargetNodeHandling", "Individual"), ("DepthOfEffect", 1));
		s.Add("zero-revolutions", "string30", 1000, ("RevolutionCount", 0.0));
		s.Add("tiny-revolutions", "string30", 1000, ("RevolutionCount", 0.01));
		s.Add("tiny-revolutions/percentage", "string30", 1000, ("RevolutionCount", 0.01), ("PulseLengthFormat", "PercentageOfRevolution"), ("PulsePercentage", 1));
		s.Add("overlap-past-the-end", "string30", 1000, ("PulseLengthFormat", "FixedTime"), ("PulseOverlap", 5000));
		s.Add("fixed-overlap", "string30", 1000, ("PulseLengthFormat", "FixedTime"), ("PulseOverlap", 200));
		s.FrameMs = 10;
		s.Add("string30/defaults", "string30", 500);
		s.Add("grid/defaults", "grid", 500);
		s.Add("discrete_rgb/gradient", "discrete_rgb", 500, ("ColorHandling", "GradientThroughWholeEffect"), ("ColorGradient", V.RgbStops));
		s.FrameMs = 50;
		return s;
	}

	private static EffectSpec BarsCases()
	{
		// The extras copy: upstream's code with WPF's rotation arithmetic. See WpfShim.cs.
		var s = new EffectSpec("Bars", new Guid("24decea6-78ab-4f42-acee-79afe4ffc016"), new Guid("24decea6-78ab-4f42-acee-79afe4ffc015"));
		PixelCommon(s, true);
		Ofat(s, "grid", 1000, P(), null, ["RotationAngle"]);
		Ofat(s, "string30", 1000, P(), null, null, ["Direction"]);
		var zig = P(("BarType", "ZigZag"));
		Ofat(s, "grid", 1000, zig, null, null, ["Direction", "ZigZagAmplitude", "ZigZagPeriod", "ZigZagBarThickness", "ZigZagSpacing", "Highlight", "Show3D", "Colors", "Repeat", "SpeedCurve"]);
		Ofat(s, "grid", 1000, Merge(zig, ("StringOrientation", "Horizontal")), null, null, ["Direction", "ZigZagAmplitude", "ZigZagPeriod"]);
		Ofat(s, "grid", 1000, Merge(zig, ("TargetPositioning", "Locations")), null, null, ["Direction", "ZigZagAmplitude", "ZigZagPeriod", "ZigZagBarThickness", "ZigZagSpacing"]);
		var speed = P(("MovementType", "Speed"));
		Ofat(s, "grid", 1000, speed, null, null, ["Speed", "SpeedCurve", "Direction"]);
		Ofat(s, "grid", 1000, Merge(speed, ("TargetPositioning", "Locations")), null, null, ["Speed", "SpeedCurve", "Direction"]);
		Ofat(s, "grid", 1000, P(("TargetPositioning", "Locations")), null, ["RotationAngle"]);
		Cross(s, "grid", 1000, P(), 300);
		Cross(s, "grid", 1000, P(("TargetPositioning", "Locations")), 150);
		Cross(s, "uneven", 1000, P(), 60);
		foreach (var d in new[] { "Up", "Down", "Expand", "Compress", "Left", "Right", "HExpand", "HCompress", "AlternateUp", "AlternateDown", "AlternateLeft", "AlternateRight" })
		{
			s.Add($"horizontal/{d}", "grid", 1000, ("StringOrientation", "Horizontal"), ("Direction", d));
			s.Add($"highlight/{d}", "grid", 1000, ("Direction", d), ("Highlight", true));
			s.Add($"3d/{d}", "grid", 1000, ("Direction", d), ("Show3D", true));
			s.Add($"gradient/{d}", "grid", 1000, ("Direction", d), ("Colors", V.Four));
			s.Add($"location/{d}", "grid", 1000, ("TargetPositioning", "Locations"), ("Direction", d));
			foreach (var angle in new[] { 12.5, 33.0, 75.0 })
				s.Add($"rotation/{d}/{angle}", "grid", 1000, ("Direction", d), ("RotationAngle", V.Flat(angle)));
			s.Add($"rotation/{d}/location", "grid", 1000, ("Direction", d), ("RotationAngle", V.Flat(70)), ("TargetPositioning", "Locations"));
			s.Add($"rotation/{d}/zigzag", "grid", 1000, ("Direction", d), ("RotationAngle", V.Flat(70)), ("BarType", "ZigZag"));
		}
		foreach (var angle in new[] { 0.0, 12.5, 25.0, 37.5, 50.0, 62.5, 75.0, 87.5, 90.0, 100.0 })
		{
			s.Add($"rotation/{angle}", "grid", 1000, ("RotationAngle", V.Flat(angle)));
			s.Add($"rotation/{angle}/zigzag", "grid", 1000, ("RotationAngle", V.Flat(angle)), ("BarType", "ZigZag"));
			s.Add($"rotation/{angle}/horizontal", "grid", 1000, ("RotationAngle", V.Flat(angle)), ("StringOrientation", "Horizontal"));
			s.Add($"rotation/{angle}/location", "grid", 1000, ("RotationAngle", V.Flat(angle)), ("TargetPositioning", "Locations"));
		}
		s.Add("rotation/ramp", "grid", 1000, ("RotationAngle", V.RampUp));
		s.Add("rotation/multi", "grid", 1000, ("RotationAngle", V.Multi));
		foreach (var d in new[] { "Up", "Down", "Expand", "Compress", "Left", "Right", "HExpand", "HCompress", "AlternateUp", "AlternateDown", "AlternateLeft", "AlternateRight" })
		{
			foreach (var rate in new[] { 20.0, 45.0, 90.0 })
			{
				var b = P(("Direction", d), ("MovementType", "Speed"), ("SpeedCurve", V.Flat(rate)));
				s.Add($"speed/{rate}/{d}", "grid", 1000, b);
				s.Add($"speed/{rate}/{d}/location", "grid", 1000, Merge(b, ("TargetPositioning", "Locations")));
				s.Add($"speed/{rate}/{d}/horizontal", "grid", 1000, Merge(b, ("StringOrientation", "Horizontal")));
				s.Add($"speed/{rate}/{d}/rotated", "grid", 1000, Merge(b, ("RotationAngle", V.Flat(70))));
				s.Add($"speed/{rate}/{d}/rotated-location", "grid", 1000, Merge(b, ("RotationAngle", V.Flat(70)), ("TargetPositioning", "Locations")));
			}
			// Many bars on few pixels: the bar height falls below one.
			s.Add($"crowded/{d}", "string30", 500, ("Direction", d), ("Repeat", 10), ("Colors", V.Four));
			s.Add($"crowded/{d}/location", "tree4", 500, ("Direction", d), ("Repeat", 10), ("Colors", V.Four), ("TargetPositioning", "Locations"));
			s.Add($"crowded/{d}/mixed", "mixed_depth", 500, ("Direction", d), ("Repeat", 10));
			foreach (var thickness in new[] { 1, 40 })
				foreach (var spacing in new[] { 0, 1, 60 })
				{
					s.Add($"zigzag/{d}/{thickness}/{spacing}", "grid", 500, ("BarType", "ZigZag"), ("Direction", d), ("ZigZagBarThickness", thickness), ("ZigZagSpacing", spacing));
					s.Add($"zigzag/{d}/{thickness}/{spacing}/location", "grid", 500, ("BarType", "ZigZag"), ("Direction", d), ("ZigZagBarThickness", thickness), ("ZigZagSpacing", spacing), ("TargetPositioning", "Locations"));
				}
		}
		Frame10(s, true, 40);
		return s;
	}

	private static EffectSpec CandleCases()
	{
		// The extras copy, identical but for a random source that can be scripted.
		var s = new EffectSpec("Candle", new Guid("504582d2-43ac-472e-8885-ec4bcbf2b1f8"), new Guid("504582d2-43ac-472e-8885-ec4bcbf2b1f7"));
		foreach (var target in BasicTargets)
		{
			s.Add($"{target}/defaults", target, 1500);
			s.Add($"{target}/red", target, 1500, ("Color", V.Red));
		}
		var overrides = new Dictionary<string, object[]>
		{
			["GroupLevel"] = [1, 2, 5, 25],
			["Color"] = [V.Orange, V.Red],
			["FlickerFrequency"] = [1, 5, 50, 500],
			["ChangePercentage"] = [0.0, 0.05, 0.8],
			["MinLevel"] = [0.0, 0.5, 0.9],
			["MaxLevel"] = [0.2, 0.5, 0.9],
			["FlickerFrequencyDeviationCap"] = [0.0, 0.9],
			["ChangePercentageDeviationCap"] = [0.0, 0.9],
		};
		Ofat(s, "string30", 1500, P(), overrides);
		var flat = P(("MinLevel", 0.5), ("MaxLevel", 0.5), ("FlickerFrequencyDeviationCap", 0.0), ("ChangePercentageDeviationCap", 0.0), ("ChangePercentage", 0.0));
		s.Add("flat/string30", "string30", 1500, flat);
		s.Add("flat/grid", "grid", 1500, flat);
		s.Add("flat/freq", "string30", 1500, Merge(flat, ("FlickerFrequency", 7)));
		s.Add("flat/discrete", "discrete", 1500, Merge(flat, ("Color", V.Red)));
		s.Add("flat/group", "grid", 1500, Merge(flat, ("GroupLevel", 3)));
		s.Add("two-targets", "string30+string30b", 1500);
		s.Add("two-targets/group", "string30+string30b", 1500, ("GroupLevel", 7));
		s.Add("1230ms", "string30", 1230);
		s.Add("3ms", "string30", 3);
		s.Add("zero-length-states", "string30", 3000, ("FlickerFrequencyDeviationCap", 1.0), ("FlickerFrequency", 200));
		s.FrameMs = 10;
		s.Add("string30/defaults", "string30", 500);
		s.FrameMs = 50;
		RealCandle(s);
		return s;
	}

	private static EffectSpec StrobeCases()
	{
		var s = new EffectSpec("Strobe", new StrobeDescriptor().TypeId);
		foreach (var target in BasicTargets)
		{
			s.Add($"{target}/defaults", target, 1000);
			s.Add($"{target}/advanced", target, 1000, ("StrobeMode", "Advanced"));
			s.Add($"{target}/rgb", target, 1000, ("Colors", V.RgbStops));
			s.Add($"{target}/advanced-rgb", target, 1000, ("StrobeMode", "Advanced"), ("Colors", V.RgbStops));
		}
		var overrides = new Dictionary<string, object[]>
		{
			["CycleTime"] = [50, 75, 333, 2000],
			["CycleVariationCurve"] = [V.RampUp, V.RampDown, V.Tri, V.Flat50, .. ExtraCurves],
			["OnTimeCurve"] = [V.RampUp, V.RampDown, V.Tri, V.Multi, .. ExtraCurves],
			["IntensityCurve"] = [V.RampUp, V.RampDown, V.Flat100, V.Multi, .. ExtraCurves],
			["Colors"] = [V.Red1, V.RedBlue, V.Rainbow, V.Hard],
			["StrobeSource"] = ["TimeInterval"],
		};
		Ofat(s, "string30", 1000, P(), overrides);
		Ofat(s, "string30", 1000, P(("StrobeMode", "Advanced")), overrides, ["StrobeMode"]);
		s.Add("two-targets", "string30+string30b", 1000);
		foreach (var ms in new[] { 1230, 30, 3000, 1 })
			s.Add($"string30/{ms}ms", "string30", ms);
		foreach (var source in new[] { "MarkCollection", "MarkCollectionLabelDuration" })
		{
			foreach (var collection in new[] { "Beats", "Late", "Odd" })
			{
				AddMarked(s, $"marks/{source}/{collection}", "string30", 1000, collection, ("StrobeSource", source));
				AddMarked(s, $"marks/{source}/{collection}/advanced", "string30", 1000, collection, ("StrobeSource", source), ("StrobeMode", "Advanced"));
				AddMarked(s, $"marks/{source}/{collection}/grid", "grid", 1000, collection, ("StrobeSource", source), ("Colors", V.RedBlue));
			}
			s.Add($"marks/{source}/none", "string30", 1000, ("StrobeSource", source)).Marks = SampleMarks().ToList();
			s.Add($"marks/{source}/sequence-has-none", "string30", 1000, ("StrobeSource", source)).Marks = new List<MarkDef>();
		}
		s.FrameMs = 10;
		s.Add("string30/defaults", "string30", 500);
		s.Add("string30/advanced", "string30", 500, ("StrobeMode", "Advanced"));
		s.FrameMs = 50;
		return s;
	}

	private static EffectSpec AlternatingCases()
	{
		var s = new EffectSpec("Alternating", new AlternatingDescriptor().TypeId);
		var overrides = new Dictionary<string, object[]>
		{
			["AlternatingMode"] = ["TimeInterval"],
			["GroupLevel"] = [1, 2, 5, 40],
			["Interval"] = [0, 100, 333, 2000],
			["IntervalSkipCount"] = [1, 2, 3],
			["DepthOfEffect"] = [1, 2],
			["Colors"] = [V.Glp1, V.Glp2, V.Glp3],
		};
		object[] rgb = [new GlpV(V.Red1, V.Flat100), new GlpV(V.Lime1, V.Tri), new GlpV(V.Blue1, V.RampDown)];
		foreach (var target in BasicTargets)
		{
			s.Add($"{target}/defaults", target, 1000);
			s.Add($"{target}/static", target, 1000, ("EnableStatic", true));
			s.Add($"{target}/rgb", target, 1000, ("Colors", rgb));
			s.Add($"{target}/rgb-static", target, 1000, ("Colors", rgb), ("EnableStatic", true));
			s.Add($"{target}/rgb-stops", target, 1000, ("Colors", new object[] { new GlpV(V.RgbStops, V.Multi) }));
		}
		Ofat(s, "string30", 1000, P(), overrides);
		Ofat(s, "string30", 1000, P(("EnableStatic", true)), overrides, ["EnableStatic"], ["GroupLevel", "Colors"]);
		Ofat(s, "grid", 1000, P(("Interval", 200)), overrides, ["Interval"], ["GroupLevel", "IntervalSkipCount", "Colors"]);
		Ofat(s, "string30", 1000, P(("Colors", rgb), ("Interval", 150)), overrides, ["Colors", "Interval"], ["GroupLevel", "IntervalSkipCount", "EnableStatic"]);
		s.Add("empty-colors", "string30", 1000, ("Colors", Array.Empty<object>()));
		s.Add("depth/grid", "grid", 1000, ("EnableDepth", true), ("DepthOfEffect", 1));
		s.Add("depth/tree4", "tree4", 1000, ("EnableDepth", true), ("DepthOfEffect", 2));
		s.Add("depth/too-deep", "tree4", 1000, ("EnableDepth", true), ("DepthOfEffect", 9));
		s.Add("two-targets", "string30+string30b", 1000);
		foreach (var ms in new[] { 1230, 30, 3000, 1 })
			s.Add($"string30/{ms}ms", "string30", ms);
		foreach (var collection in new[] { "Beats", "Late", "Odd" })
		{
			AddMarked(s, $"marks/{collection}", "string30", 1000, collection, ("AlternatingMode", "MarkCollection"));
			AddMarked(s, $"marks/{collection}/grid", "grid", 1000, collection, ("AlternatingMode", "MarkCollection"), ("GroupLevel", 2));
			AddMarked(s, $"marks/{collection}/rgb", "string30", 1000, collection, ("AlternatingMode", "MarkCollection"), ("Colors", rgb));
			AddMarked(s, $"marks/{collection}/skip", "string30", 1000, collection, ("AlternatingMode", "MarkCollection"), ("Colors", rgb), ("IntervalSkipCount", 2));
		}
		s.Add("marks/none", "string30", 1000, ("AlternatingMode", "MarkCollection")).Marks = SampleMarks().ToList();
		s.Add("marks/sequence-has-none", "string30", 1000, ("AlternatingMode", "MarkCollection")).Marks = new List<MarkDef>();
		s.FrameMs = 10;
		s.Add("string30/defaults", "string30", 500);
		s.Add("grid/rgb", "grid", 500, ("Colors", rgb), ("Interval", 120));
		s.FrameMs = 50;
		return s;
	}
}
