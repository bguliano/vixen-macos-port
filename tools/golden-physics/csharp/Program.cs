// Renders the shared cases through the real Morph code and writes every pixel of every frame.
using System.Drawing;
using System.Globalization;
using System.Text;
using System.Text.Json;
using Common.Controls.ColorManagement.ColorModels;
using NLog;
using NLog.Config;
using NLog.Targets;
using Vixen.Data.Value;
using Vixen.Native;
using Vixen.Services;
using Vixen.Sys;
using VixenModules.App.ColorGradients;
using VixenModules.App.Curves;
using VixenModules.App.Polygon;
using VixenModules.Effect.Effect;
using VixenModules.Effect.Morph;
using VixenModules.Property.Location;
using ZedGraph;

internal static class Program
{
	private static int _nodeNames;
	private static MemoryTarget _log;

	private static async Task<int> Main(string[] args)
	{
		CultureInfo.DefaultThreadCurrentCulture = CultureInfo.InvariantCulture;
		var casesPath = args[0];
		var outPath = args[1];
		var profile = Path.Combine(Path.GetTempPath(), "physics-diff-" + Guid.NewGuid().ToString("N"));
		Directory.CreateDirectory(profile);
		var configuration = new LoggingConfiguration();
		_log = new MemoryTarget("mem") { Layout = "${message} ${exception:format=tostring}" };
		configuration.AddRule(NLog.LogLevel.Warn, NLog.LogLevel.Fatal, _log);
		LogManager.Configuration = configuration;
		VixenSystem.UIContext = new SynchronizationContext();
		await using var runtime = await NativeVixenRuntime.StartAsync(profile);
		_log.Logs.Clear();

		using var doc = JsonDocument.Parse(File.ReadAllText(casesPath));
		using var w = new StreamWriter(outPath);
		foreach (var c in doc.RootElement.EnumerateArray())
		{
			var id = c.GetProperty("id").GetString();
			try
			{
				w.WriteLine(id + "\t" + RenderCase(c));
			}
			catch (Exception e)
			{
				w.WriteLine(id + "\tTHROWN " + e.GetType().Name);
			}
		}
		try { Directory.Delete(profile, true); } catch { }
		return 0;
	}

	private static ElementNode Grid(int strings, int pixels, bool located, out List<ElementNode> leaves)
	{
		var name = "G" + (++_nodeNames);
		var root = ElementNodeService.Instance.CreateSingle(null, name, false, false);
		leaves = new List<ElementNode>();
		var i = 0;
		for (var s = 0; s < strings; s++)
		{
			var str = ElementNodeService.Instance.CreateSingle(root, $"{name} S{s}", false, false);
			for (var p = 0; p < pixels; p++)
			{
				var leaf = ElementNodeService.Instance.CreateSingle(str, $"{name} S{s}-{p}", true, false);
				if (located)
				{
					var property = (LocationModule)leaf.Properties.Add(LocationDescriptor.ModuleId);
					property.ModuleData = new LocationData { X = (i % 10) * 4, Y = (i / 10) * 4, Z = 0 };
				}
				leaves.Add(leaf);
				i++;
			}
		}
		return root;
	}

	private static double D(JsonElement e) => e.GetDouble();

	private static ColorGradient Gradient(JsonElement stops)
	{
		var g = new ColorGradient();
		g.Colors.Clear();
		foreach (var s in stops.EnumerateArray())
		{
			var rgb = s[1];
			var c = Color.FromArgb(rgb[0].GetInt32(), rgb[1].GetInt32(), rgb[2].GetInt32());
			g.Colors.Add(new ColorPoint(XYZ.FromRGB(new RGB(c)), 0.5, s[0].GetDouble()));
		}
		return g;
	}

	private static Curve CurveOf(JsonElement points)
	{
		var xs = points.EnumerateArray().Select(p => p[0].GetDouble()).ToArray();
		var ys = points.EnumerateArray().Select(p => p[1].GetDouble()).ToArray();
		return new Curve(new PointPairList(xs, ys));
	}

	private static string RenderCase(JsonElement c)
	{
		var strings = c.GetProperty("strings").GetInt32();
		var pixels = c.GetProperty("pixels").GetInt32();
		var ms = c.GetProperty("ms").GetInt32();
		var located = c.TryGetProperty("located", out var l) && l.GetBoolean();
		var root = Grid(strings, pixels, located, out var leaves);

		// `new MorphPolygon()` looks at statics the last render left behind.
		MorphPolygon.BufferWidth = 0;
		MorphPolygon.BufferHeight = 0;

		var data = new MorphData();
		var s = c.GetProperty("settings");
		int I(string n, int d) => s.TryGetProperty(n, out var v) ? v.GetInt32() : d;
		var hasShapes = c.GetProperty("shapes").GetArrayLength() > 0;
		// A file holds shapes already placed for its margin. With no shapes the user sets the margin on the default one.
		if (hasShapes) data.Margin = I("Margin", data.Margin);
		data.RepeatCount = I("RepeatCount", data.RepeatCount);
		data.RepeatSkip = I("RepeatSkip", data.RepeatSkip);
		data.Stagger = I("Stagger", data.Stagger);
		data.HeadLength = I("HeadLength", data.HeadLength);
		data.HeadDuration = I("HeadDuration", data.HeadDuration);
		data.Acceleration = I("Acceleration", data.Acceleration);
		data.DisplayElementWidth = I("DisplayElementWidth", 0);
		data.DisplayElementHeight = I("DisplayElementHeight", 0);
		if (s.TryGetProperty("RepeatDirection", out var rd)) data.RepeatDirection = Enum.Parse<WipeRepeatDirection>(rd.GetString());
		if (s.TryGetProperty("PolygonType", out var pt)) data.PolygonType = Enum.Parse<PolygonType>(pt.GetString());
		if (s.TryGetProperty("FillType", out var ft)) data.PolygonFillType = Enum.Parse<PolygonFillType>(ft.GetString());
		if (c.TryGetProperty("orientation", out var o)) data.Orientation = Enum.Parse<StringOrientation>(o.GetString());
		if (located)
		{
			data.TargetPositioning = TargetPositioningType.Locations;
			data.Orientation = StringOrientation.Vertical;
		}
		if (c.TryGetProperty("head", out var head)) data.HeadColor = Gradient(head);
		if (c.TryGetProperty("tail", out var tail)) data.TailColor = Gradient(tail);
		if (c.TryGetProperty("fillColor", out var fc)) data.FillColor = Gradient(fc);
		if (s.TryGetProperty("FillPolygon", out var fp)) data.FillPolygon = fp.GetBoolean();
		if (c.TryGetProperty("fillBrightness", out var fb)) data.FillBrightness = CurveOf(fb);

		foreach (var shape in c.GetProperty("shapes").EnumerateArray())
		{
			var r = new MorphPolygonData
			{
				HeadLength = shape.TryGetProperty("headLength", out var a) ? a.GetInt32() : 4,
				HeadDuration = shape.TryGetProperty("headDuration", out var b) ? b.GetInt32() : 20,
				Acceleration = shape.TryGetProperty("acceleration", out var d) ? d.GetInt32() : 0,
				HeadColor = new ColorGradient(Color.White),
				TailColor = new ColorGradient(Color.Red),
				FillColor = shape.TryGetProperty("color", out var col) ? Gradient(col) : new ColorGradient(Color.Red),
				Time = shape.TryGetProperty("time", out var t) ? t.GetDouble() : 0.0,
				StartOffset = shape.TryGetProperty("start", out var st) ? st.GetInt32() : 0,
				FillType = shape.TryGetProperty("fill", out var f) ? Enum.Parse<PolygonFillType>(f.GetString()) : PolygonFillType.Wipe,
				TailBrightness = new Curve(CurveType.Flat100),
				HeadBrightness = new Curve(CurveType.Flat100),
				FillBrightness = shape.TryGetProperty("fillBrightness", out var sfb) ? CurveOf(sfb) : new Curve(CurveType.Flat100),
			};
			var points = shape.GetProperty("points").EnumerateArray()
				.Select(p => new PolygonPoint { X = p[0].GetDouble(), Y = p[1].GetDouble() }).ToList();
			var kind = shape.GetProperty("kind").GetString();
			if (kind == "Line")
			{
				r.Line = new Line();
				r.Line.Points.AddRange(points);
			}
			else if (kind == "Ellipse")
			{
				r.Ellipse = new Ellipse();
				r.Ellipse.Points.AddRange(points);
				var center = shape.GetProperty("center");
				r.Ellipse.Center = new PolygonPoint { X = center[0].GetDouble(), Y = center[1].GetDouble() };
				r.Ellipse.Width = shape.GetProperty("w").GetDouble();
				r.Ellipse.Height = shape.GetProperty("h").GetDouble();
				r.Ellipse.Angle = shape.GetProperty("angle").GetDouble();
			}
			else
			{
				r.Polygon = new Polygon();
				r.Polygon.Points.AddRange(points);
			}
			data.MorphPolygonData.Add(r);
		}

		var effect = new Morph();
		effect.ModuleData = data;
		effect.TargetNodes = new IElementNode[] { root };
		effect.TimeSpan = TimeSpan.FromMilliseconds(ms);
		if (!hasShapes) effect.Margin = I("Margin", 0);
		if (c.TryGetProperty("headBrightness", out var hb)) effect.HeadBrightness = CurveOf(hb);
		if (c.TryGetProperty("tailBrightness", out var tb)) effect.TailBrightness = CurveOf(tb);

		_log.Logs.Clear();
		var ok = effect.PreRender();
		if (!ok)
		{
			var msg = _log.Logs.Count > 0 ? _log.Logs[0] : "";
			var type = msg.Split(' ', ':').FirstOrDefault(x => x.EndsWith("Exception")) ?? "?";
			return "FAILED " + type;
		}
		var intents = effect.Render();
		var sb = new StringBuilder();
		var frames = ms / 50;
		sb.Append(frames).Append('\t');
		foreach (var leaf in leaves)
		{
			for (var frame = 0; frame < frames; frame++)
			{
				var at = TimeSpan.FromMilliseconds(frame * 50);
				var active = intents.GetElementIntentsAtTime(leaf.Element.Id, at);
				if (active == null || active.Length == 0)
				{
					sb.Append("------");
					continue;
				}
				object state = active[0].Intent.GetStateAt(at - active[0].StartTime);
				var v = (RGBValue)state;
				sb.Append($"{v.R:x2}{v.G:x2}{v.B:x2}");
			}
		}
		return sb.ToString();
	}
}
