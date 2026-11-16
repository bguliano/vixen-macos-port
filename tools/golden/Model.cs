// Parameter values and golden-file writing. The JSON stores values by what they are, and each runtime converts
// them to its own types: C# through the property's type, Rust through the ParamDesc kind.

using System.Globalization;
using System.Text;
using System.Text.Json;
using Common.Controls.ColorManagement.ColorModels;
using Common.ValueTypes;
using VixenModules.App.ColorGradients;
using VixenModules.App.Curves;
using ZedGraph;

namespace VixenGolden;

public sealed class ColorV(int r, int g, int b)
{
	public int R = r, G = g, B = b;
	public Color ToColor() => Color.FromArgb(R, G, B);
	public string Hex => $"{R:X2}{G:X2}{B:X2}";
}

public sealed class CurveV(params (double x, double y)[] points)
{
	public (double x, double y)[] Points = points;

	public Curve ToCurve()
	{
		var xs = Points.Select(p => p.x).ToArray();
		var ys = Points.Select(p => p.y).ToArray();
		return new Curve(new PointPairList(xs, ys));
	}
}

public sealed class Stop(double pos, ColorV color, double focus = 0.5)
{
	public double Pos = pos, Focus = focus;
	public ColorV Color = color;
}

public sealed class AlphaStop(double pos, double alpha, double focus = 0.5)
{
	public double Pos = pos, Alpha = alpha, Focus = focus;
}

public sealed class GradV(Stop[] stops, AlphaStop[] alphas = null)
{
	public Stop[] Stops = stops;
	public AlphaStop[] Alphas = alphas;

	public ColorGradient ToGradient()
	{
		var g = new ColorGradient();
		g.Colors.Clear();
		foreach (var s in Stops)
			g.Colors.Add(new ColorPoint(XYZ.FromRGB(new RGB(s.Color.ToColor())), s.Focus, s.Pos));
		if (Alphas != null)
		{
			g.Alphas.Clear();
			foreach (var a in Alphas)
				g.Alphas.Add(new AlphaPoint(a.Alpha, a.Focus, a.Pos));
		}
		return g;
	}
}

public sealed class GlpV(GradV gradient, CurveV curve)
{
	public GradV Gradient = gradient;
	public CurveV Curve = curve;
	public GradientLevelPair ToPair() => new(Gradient.ToGradient(), Curve.ToCurve());
}

/// Individual list items are applied through the original child property setters.
public sealed record RecordV(params (string name, object value)[] Values);
public sealed record RecordListV(params RecordV[] Items);

/// <summary>A .NET float property value, kept as a float so the exact float widens into the JSON.</summary>
public static class Convert2
{
	public static object To(object v, Type t)
	{
		if (t == typeof(bool)) return (bool)v;
		if (t == typeof(int)) return v is int i ? i : (int)(double)v;
		if (t == typeof(long)) return v is int l ? (long)l : (long)(double)v;
		if (t == typeof(double)) return v is int d ? (double)d : (double)v;
		if (t == typeof(float)) return v is int f ? (float)f : (float)(double)v;
		if (t == typeof(string)) return (string)v;
		if (t == typeof(Guid)) return Guid.Parse((string)v);
		if (t == typeof(byte)) return (byte)(int)v;
		if (t == typeof(ushort)) return (ushort)(int)v;
		if (t == typeof(uint)) return v is int u ? (uint)u : (uint)(double)v;
		if (t == typeof(ulong)) return ulong.Parse((string)v);
		if (t.IsEnum) return Enum.Parse(t, (string)v);
		if (t == typeof(Color)) return ((ColorV)v).ToColor();
		if (t == typeof(RGB)) return new RGB(((ColorV)v).ToColor());
		if (t == typeof(Curve)) return ((CurveV)v).ToCurve();
		if (t == typeof(ColorGradient)) return ((GradV)v).ToGradient();
		if (t == typeof(Percentage)) return new Percentage((float)(double)v);
		if (t == typeof(List<ColorGradient>)) return ((object[])v).Cast<GradV>().Select(g => g.ToGradient()).ToList();
		if (t == typeof(List<GradientLevelPair>)) return ((object[])v).Cast<GlpV>().Select(g => g.ToPair()).ToList();
		throw new NotSupportedException("Unhandled property type " + t);
	}
}

public static class Json
{
	public static void WriteValue(Utf8JsonWriter w, object v)
	{
		switch (v)
		{
			case bool b: w.WriteBooleanValue(b); break;
			case int i: w.WriteNumberValue(i); break;
			case double d: w.WriteNumberValue(d); break;
			case string s: w.WriteStringValue(s); break;
			case ColorV c: w.WriteStringValue(c.Hex); break;
			case CurveV c:
				w.WriteStartArray();
				foreach (var p in c.Points)
				{
					w.WriteStartArray();
					w.WriteNumberValue(p.x);
					w.WriteNumberValue(p.y);
					w.WriteEndArray();
				}
				w.WriteEndArray();
				break;
			case GradV g:
				w.WriteStartObject();
				w.WritePropertyName("c");
				w.WriteStartArray();
				foreach (var s in g.Stops)
				{
					w.WriteStartArray();
					w.WriteNumberValue(s.Pos);
					w.WriteStringValue(s.Color.Hex);
					w.WriteNumberValue(s.Focus);
					w.WriteEndArray();
				}
				w.WriteEndArray();
				if (g.Alphas != null)
				{
					w.WritePropertyName("a");
					w.WriteStartArray();
					foreach (var a in g.Alphas)
					{
						w.WriteStartArray();
						w.WriteNumberValue(a.Pos);
						w.WriteNumberValue(a.Alpha);
						w.WriteNumberValue(a.Focus);
						w.WriteEndArray();
					}
					w.WriteEndArray();
				}
				w.WriteEndObject();
				break;
			case GlpV p:
				w.WriteStartObject();
				w.WritePropertyName("g");
				WriteValue(w, p.Gradient);
				w.WritePropertyName("l");
				WriteValue(w, p.Curve);
				w.WriteEndObject();
				break;
			case RecordV record:
				// Ordered setter calls, just like the outer effect parameters. A dictionary loses
				// ordering and cannot represent setters that change another child property.
				w.WriteStartArray();
				foreach (var (name, value) in record.Values) {
					w.WriteStartArray(); w.WriteStringValue(name); WriteValue(w, value); w.WriteEndArray();
				}
				w.WriteEndArray();
				break;
			case RecordListV records:
				w.WriteStartArray();
				foreach (var record in records.Items) WriteValue(w, record);
				w.WriteEndArray();
				break;
			case object[] list:
				w.WriteStartArray();
				foreach (var o in list) WriteValue(w, o);
				w.WriteEndArray();
				break;
			default:
				throw new NotSupportedException("Unhandled JSON value " + v.GetType());
		}
	}

	public static string Num(double d) => d.ToString("R", CultureInfo.InvariantCulture);
}

/// <summary>Values reused across effect case lists.</summary>
public static class V
{
	public static readonly ColorV Red = new(255, 0, 0), Lime = new(0, 255, 0), Blue = new(0, 0, 255), White = new(255, 255, 255),
		Yellow = new(255, 255, 0), Cyan = new(0, 255, 255), Magenta = new(255, 0, 255), Orange = new(255, 128, 64), Teal = new(16, 200, 128);

	public static readonly CurveV RampUp = new((0, 0), (100, 100));
	public static readonly CurveV RampDown = new((0, 100), (100, 0));
	public static readonly CurveV Flat100 = new((0, 100), (100, 100));
	public static readonly CurveV Flat50 = new((0, 50), (100, 50));
	public static readonly CurveV Flat25 = new((0, 25), (100, 25));
	public static readonly CurveV Flat0 = new((0, 0), (100, 0));
	public static readonly CurveV Tri = new((0, 0), (50, 100), (100, 0));
	public static readonly CurveV Multi = new((0, 10), (20, 90), (35, 35), (70, 80), (100, 25));
	public static CurveV Flat(double y) => new((0, y), (100, y));

	public static readonly GradV White1 = new([new Stop(0, White)]);
	public static readonly GradV Red1 = new([new Stop(0, Red)]);
	public static readonly GradV Lime1 = new([new Stop(0, Lime)]);
	public static readonly GradV Blue1 = new([new Stop(0, Blue)]);
	public static readonly GradV RedBlue = new([new Stop(0, Red), new Stop(1, Blue)]);
	public static readonly GradV Rainbow = new([new Stop(0, Red), new Stop(0.2, Yellow), new Stop(0.4, Lime), new Stop(0.6, Cyan), new Stop(0.8, Blue), new Stop(1, Magenta)]);
	public static readonly GradV Focused = new([new Stop(0, Orange), new Stop(0.37, Teal, 0.2), new Stop(1, Blue, 0.8)]);
	public static readonly GradV Hard = new([new Stop(0, Red), new Stop(0.5, Red), new Stop(0.5, Blue), new Stop(1, Blue)]);
	public static readonly GradV Faded = new([new Stop(0, Red), new Stop(1, Lime)], [new AlphaStop(0, 0.25), new AlphaStop(1, 1)]);
	public static readonly GradV Inset = new([new Stop(0.25, Lime), new Stop(0.75, Magenta)]);
	public static readonly GradV RgbStops = new([new Stop(0, Red), new Stop(0.5, Lime), new Stop(1, Blue)]);

	public static readonly object[] Rgb3 = [Red1, Lime1, Blue1];
	public static readonly object[] Two = [RedBlue, Lime1];
	public static readonly object[] Four = [Rainbow, Focused, White1, Hard];
	public static readonly object[] One = [White1];

	public static readonly object[] Glp2 = [new GlpV(Red1, Flat100), new GlpV(Blue1, RampDown)];
	public static readonly object[] Glp3 = [new GlpV(Rainbow, Tri), new GlpV(Lime1, Flat50), new GlpV(RedBlue, RampUp)];
	public static readonly object[] Glp1 = [new GlpV(Focused, Multi)];
}

/// <summary>Builds compact per-case dictionaries of distinct values.</summary>
public sealed class Interner
{
	private readonly Dictionary<string, int> _ids = new(StringComparer.Ordinal);
	public readonly List<string> Items = new();

	public int Id(string s)
	{
		if (!_ids.TryGetValue(s, out var id))
		{
			id = Items.Count;
			Items.Add(s);
			_ids[s] = id;
		}
		return id;
	}
}

public static class Text
{
	public static string Trunc(string s, int max) => s.Length <= max ? s : s[..max];

	public static string Join(IEnumerable<int> items)
	{
		var sb = new StringBuilder();
		foreach (var i in items)
		{
			if (sb.Length > 0) sb.Append(',');
			sb.Append(i);
		}
		return sb.ToString();
	}
}
