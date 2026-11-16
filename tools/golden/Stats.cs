// Statistics for cases that use random numbers. crates/vixen-effect-registry/tests/golden.rs computes the same numbers
// from its own renders, so anything changed here must change there too.

using System.Globalization;
using System.Text.Json;

namespace VixenGolden;

public static class Stats
{
	public sealed record Run(double[][] Brightness, int[][] ColorKey, List<(int kind, long start, long end)>[] Intents);

	private static double Round(double v) => Math.Round(v, 6, MidpointRounding.ToEven);

	private sealed class Series
	{
		private readonly List<double> _values = new();
		public void Add(double v) => _values.Add(v);
		public double Mean => _values.Count == 0 ? 0 : _values.Average();
		public double Std
		{
			get
			{
				if (_values.Count < 2) return 0;
				var m = Mean;
				return Math.Sqrt(_values.Sum(v => (v - m) * (v - m)) / (_values.Count - 1));
			}
		}
		public double Min => _values.Count == 0 ? 0 : _values.Min();
		public double Max => _values.Count == 0 ? 0 : _values.Max();
	}

	/// <summary>The colours a set of renders shows. A rare colour can be missing from a few renders, so palettes get more.</summary>
	public static SortedSet<int> ColorsOf(IEnumerable<Run> runs)
	{
		var colors = new SortedSet<int>();
		foreach (var run in runs)
			for (var e = 0; e < run.Brightness.Length; e++)
				for (var t = 0; t < run.Brightness[e].Length; t++)
					if (run.Brightness[e][t] > 0)
						colors.Add(run.ColorKey[e][t]);
		return colors;
	}

	public static void Write(Utf8JsonWriter w, List<Run> runs, long[] times, IEnumerable<int> extraColors)
	{
		var names = new[] { "elements_with_intents", "intent_count", "span_min_ms", "span_max_ms", "span_mean_ms", "start_min_ms", "end_max_ms", "lit_fraction", "mean_brightness", "max_brightness", "min_lit_brightness", "distinct_colors" };
		var scalars = names.ToDictionary(n => n, _ => new Series());
		var hist = Enumerable.Range(0, 10).Select(_ => new Series()).ToArray();
		var elementCount = runs[0].Brightness.Length;
		var litByTime = times.Select(_ => new Series()).ToArray();
		var levelByTime = times.Select(_ => new Series()).ToArray();
		var litByElement = Enumerable.Range(0, elementCount).Select(_ => new Series()).ToArray();
		var colorUnion = new SortedSet<int>();

		foreach (var run in runs)
		{
			var withIntents = 0;
			var count = 0;
			double spanMin = double.MaxValue, spanMax = 0, spanSum = 0, startMin = double.MaxValue, endMax = 0;
			foreach (var list in run.Intents)
			{
				if (list.Count > 0) withIntents++;
				foreach (var (_, start, end) in list)
				{
					count++;
					var span = (end - start) / 10000.0;
					spanMin = Math.Min(spanMin, span);
					spanMax = Math.Max(spanMax, span);
					spanSum += span;
					startMin = Math.Min(startMin, start / 10000.0);
					endMax = Math.Max(endMax, end / 10000.0);
				}
			}

			long samples = 0, lit = 0;
			double sum = 0, max = 0, minLit = double.MaxValue;
			var histogram = new long[10];
			var colors = new HashSet<int>();
			var perTimeLit = new long[times.Length];
			var perTimeSum = new double[times.Length];
			for (var e = 0; e < elementCount; e++)
			{
				var elementLit = 0;
				var b = run.Brightness[e];
				for (var t = 0; t < times.Length; t++)
				{
					samples++;
					sum += b[t];
					perTimeSum[t] += b[t];
					if (b[t] > 0)
					{
						lit++;
						elementLit++;
						perTimeLit[t]++;
						max = Math.Max(max, b[t]);
						minLit = Math.Min(minLit, b[t]);
						histogram[Math.Min(9, (int)(b[t] * 10))]++;
						colors.Add(run.ColorKey[e][t]);
					}
				}
				litByElement[e].Add(elementLit / (double)times.Length);
			}

			scalars["elements_with_intents"].Add(withIntents);
			scalars["intent_count"].Add(count);
			scalars["span_min_ms"].Add(count == 0 ? 0 : spanMin);
			scalars["span_max_ms"].Add(spanMax);
			scalars["span_mean_ms"].Add(count == 0 ? 0 : spanSum / count);
			scalars["start_min_ms"].Add(count == 0 ? 0 : startMin);
			scalars["end_max_ms"].Add(endMax);
			scalars["lit_fraction"].Add(samples == 0 ? 0 : lit / (double)samples);
			scalars["mean_brightness"].Add(samples == 0 ? 0 : sum / samples);
			scalars["max_brightness"].Add(max);
			scalars["min_lit_brightness"].Add(lit == 0 ? 0 : minLit);
			scalars["distinct_colors"].Add(colors.Count);
			for (var h = 0; h < 10; h++) hist[h].Add(lit == 0 ? 0 : histogram[h] / (double)lit);
			for (var t = 0; t < times.Length; t++)
			{
				litByTime[t].Add(elementCount == 0 ? 0 : perTimeLit[t] / (double)elementCount);
				levelByTime[t].Add(elementCount == 0 ? 0 : perTimeSum[t] / elementCount);
			}
			colorUnion.UnionWith(colors);
		}

		w.WriteNumber("renders", runs.Count);
		w.WriteStartObject("scalars");
		foreach (var n in names)
		{
			w.WriteStartArray(n);
			var s = scalars[n];
			w.WriteNumberValue(Round(s.Mean));
			w.WriteNumberValue(Round(s.Std));
			w.WriteNumberValue(Round(s.Min));
			w.WriteNumberValue(Round(s.Max));
			w.WriteEndArray();
		}
		w.WriteEndObject();
		WriteSeries(w, "hist", hist);
		WriteSeries(w, "lit_by_time", litByTime);
		WriteSeries(w, "level_by_time", levelByTime);
		WriteSeries(w, "lit_by_element", litByElement);
		colorUnion.UnionWith(extraColors);
		// Colours are only comparable when the effect draws from a small palette.
		if (colorUnion.Count <= 64)
		{
			w.WriteStartArray("colors");
			foreach (var c in colorUnion) w.WriteNumberValue(c);
			w.WriteEndArray();
		}
		w.WriteNumber("color_union_size", colorUnion.Count);
	}

	private static void WriteSeries(Utf8JsonWriter w, string name, Series[] series)
	{
		w.WriteStartArray(name);
		foreach (var s in series)
		{
			w.WriteStartArray();
			w.WriteNumberValue(Math.Round(s.Mean, 4, MidpointRounding.ToEven));
			w.WriteNumberValue(Math.Round(s.Std, 4, MidpointRounding.ToEven));
			w.WriteEndArray();
		}
		w.WriteEndArray();
	}
}
