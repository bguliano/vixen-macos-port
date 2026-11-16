// Layer mixing: the original LayeredStateCombinator and mixing filter modules over stacks of layers, for
// crates/vixen-effect-registry/tests/golden.rs to compare with vixen_core::layer. The inputs come from a fixed generator,
// so the data is the same every run.

using System.Globalization;
using System.Text.Json;
using Vixen.Data.StateCombinator;
using Vixen.Data.Value;
using Vixen.Intent;
using Vixen.Module.MixingFilter;
using Vixen.Services;
using Vixen.Sys;
using Vixen.Sys.LayerMixing;
using VixenModules.LayerMixingFilter.ChromaKey;
using VixenModules.LayerMixingFilter.LumaKey;
using VixenModules.LayerMixingFilter.MaskFill;

namespace VixenGolden;

public static class Layers
{
	private sealed record FilterDef(string Name, Guid Id, Action<ILayerMixingFilterInstance> Setup, Action<Utf8JsonWriter> Write);

	private static readonly Guid ProportionalMix = new("1f16d67a-1fff-485c-8e76-27ff7e7d072b");
	private static readonly Guid HighestValue = new("01d6d526-9586-44bb-82f4-10abacf58b61");
	private static readonly Guid Mask = new("ace0f097-7982-4edc-b5dd-8989dce60e1d");
	private static readonly Guid MaskFill = new("24878b18-14f8-458e-abaa-3a9c9a34931c");
	private static readonly Guid MultiplyColor = new("0c9dfc0b-8e39-4167-8036-79408c6b0b35");
	private static readonly Guid IntensityOverlay = new("43f48bf5-4088-403f-ad7a-d0ff2114ca31");
	private static readonly Guid ColorChange = new("93d4169e-5138-403a-b71c-a5a1ab089801");
	private static readonly Guid LumaKey = new("7583f7d4-df99-4492-8d10-257c2677ff7b");
	private static readonly Guid ChromaKey = new("2d467733-1a64-45ad-bcdc-84bf074747e6");

	private static FilterDef Plain(string name, Guid id) => new(name, id, _ => { }, w => { });

	private static FilterDef Luma(double lower, double upper) => new($"LumaKey {lower}-{upper}", LumaKey,
		f => { var d = (LumaKeyData)f.ModuleData; d.LowerLimit = lower; d.UpperLimit = upper; },
		w => { w.WriteNumber("lower", lower); w.WriteNumber("upper", upper); });

	private static FilterDef Fill(bool exclude, bool partner) => new($"MaskFill exclude={exclude} partner={partner}", MaskFill,
		f => { var d = (MaskAndFillData)f.ModuleData; d.ExcludeZeroValues = exclude; d.RequiresMixingPartner = partner; },
		w => { w.WriteBoolean("exclude", exclude); w.WriteBoolean("partner", partner); });

	/// <summary>A chroma key on a colour, as its setup dialog stores one: saturation and hue taken from the key colour.</summary>
	private static FilterDef Chroma(string name, Color key, float hueTolerance, float saturationTolerance, double lower, double upper, bool transparent) => new(name, ChromaKey,
		f =>
		{
			var d = (ChromaKeyData)f.ModuleData;
			d.KeyColor = key;
			d.KeySaturation = Common.Controls.ColorManagement.ColorModels.HSV.FromRGB(key).S;
			d.KeyHue = key.GetHue();
			d.HueTolerance = hueTolerance;
			d.SaturationTolerance = saturationTolerance;
			d.LowerLimit = lower;
			d.UpperLimit = upper;
			d.TransparentOnZeroBrightness = transparent;
		},
		w =>
		{
			w.WriteString("key", $"{key.R:X2}{key.G:X2}{key.B:X2}");
			w.WriteNumber("hue_tolerance", hueTolerance);
			w.WriteNumber("saturation_tolerance", saturationTolerance);
			w.WriteNumber("lower", lower);
			w.WriteNumber("upper", upper);
			w.WriteBoolean("transparent", transparent);
		});

	/// <summary>Writes tests/golden/layers.json.gz.</summary>
	public static void Write(string path)
	{
		var filters = new[]
		{
			Plain("ProportionalMix", ProportionalMix),
			Plain("HighestValue", HighestValue),
			Plain("Mask", Mask),
			Fill(true, false),
			Fill(false, false),
			Fill(true, true),
			Plain("MultiplyColor", MultiplyColor),
			Plain("IntensityOverlay", IntensityOverlay),
			Plain("ColorChange", ColorChange),
			Luma(0, 0),
			Luma(0, 0.1),
			Luma(0.25, 0.75),
			Luma(0.9, 1),
			Luma(0, 1),
			new FilterDef("ChromaKey defaults", ChromaKey, _ => { }, w => w.WriteBoolean("defaults", true)),
			Chroma("ChromaKey lime", Color.FromArgb(0, 255, 0), 5, 0.05f, 0, 1, true),
			Chroma("ChromaKey lime, dim only", Color.FromArgb(0, 255, 0), 5, 0.05f, 0.1, 0.6, false),
			Chroma("ChromaKey red wraps low", Color.FromArgb(255, 0, 0), 5, 0.05f, 0, 1, false),
			Chroma("ChromaKey rose wraps high", Color.FromArgb(255, 0, 8), 5, 0.05f, 0, 1, true),
			Chroma("ChromaKey teal wide", Color.FromArgb(16, 200, 128), 40, 0.5f, 0, 1, false),
		};

		var rng = new Lcg(0x1357);
		using var fs = File.Create(path);
		using var gz = new System.IO.Compression.GZipStream(fs, System.IO.Compression.CompressionLevel.SmallestSize);
		using var w = new Utf8JsonWriter(gz, new JsonWriterOptions { SkipValidation = true });
		w.WriteStartObject();
		w.WriteStartArray("stacks");
		var combinator = new LayeredStateCombinator();
		foreach (var top in filters)
		{
			// One stack of top filter over the default layer, and one with a second filter between them.
			foreach (var middle in new[] { null, filters[1], filters[2], filters[8], filters[11] })
			{
				var defs = middle == null ? new[] { top } : new[] { top, middle };
				var layers = new List<ILayer>();
				for (var i = 0; i < defs.Length; i++)
				{
					var filter = LayerMixingFilterService.Instance.GetInstance(defs[i].Id);
					defs[i].Setup(filter);
					layers.Add(new StandardLayer(defs[i].Name) { LayerLevel = defs.Length - i, LayerMixingFilter = filter });
				}
				layers.Add(new DefaultLayer());

				w.WriteStartObject();
				w.WriteStartArray("filters");
				foreach (var d in defs)
				{
					w.WriteStartObject();
					w.WriteString("id", d.Id.ToString());
					w.WriteString("name", d.Name);
					d.Write(w);
					w.WriteEndObject();
				}
				w.WriteEndArray();
				w.WriteStartArray("cases");
				var count = middle == null ? 400 : 120;
				for (var n = 0; n < count; n++)
				{
					var states = new List<IIntentState>();
					var stateCount = rng.Next(1, 7);
					w.WriteStartArray();
					w.WriteStartArray();
					for (var s = 0; s < stateCount; s++)
					{
						var layerIndex = rng.Next(0, layers.Count);
						var (kind, color, intensity) = rng.State();
						w.WriteStartArray();
						w.WriteNumberValue(layerIndex);
						w.WriteStringValue(kind);
						w.WriteStringValue($"{color.R:X2}{color.G:X2}{color.B:X2}");
						w.WriteNumberValue(intensity);
						w.WriteEndArray();
						IIntentState state = kind switch
						{
							"L" => new StaticIntentState<LightingValue>(new LightingValue(color, intensity), layers[layerIndex]),
							"R" => new StaticIntentState<RGBValue>(new RGBValue(color), layers[layerIndex]),
							_ => new StaticIntentState<DiscreteValue>(new DiscreteValue(color, intensity), layers[layerIndex]),
						};
						states.Add(state);
					}
					w.WriteEndArray();
					var result = combinator.Combine(states);
					w.WriteStartArray();
					foreach (var line in result.Select(Describe).OrderBy(x => x, StringComparer.Ordinal))
						w.WriteStringValue(line);
					w.WriteEndArray();
					w.WriteEndArray();
				}
				w.WriteEndArray();
				w.WriteEndObject();
			}
		}
		w.WriteEndArray();
		w.WriteEndObject();
	}

	private static string Describe(IIntentState state)
	{
		switch (state.GetValue())
		{
			case RGBValue v:
				return $"R{v.R:x2}{v.G:x2}{v.B:x2}";
			case LightingValue v:
				return $"L{v.Color.R:x2}{v.Color.G:x2}{v.Color.B:x2},{v.Intensity.ToString("G10", CultureInfo.InvariantCulture)}";
			case DiscreteValue v:
				return $"D{v.Color.R:x2}{v.Color.G:x2}{v.Color.B:x2},{v.Intensity.ToString("G10", CultureInfo.InvariantCulture)}";
			default:
				throw new NotSupportedException();
		}
	}

	/// <summary>A small fixed generator: the file must not change between runs.</summary>
	private sealed class Lcg(uint state)
	{
		private uint _s = state;

		private uint Step()
		{
			_s = unchecked(_s * 1664525u + 1013904223u);
			return _s >> 8;
		}

		public int Next(int min, int max) => min + (int)(Step() % (uint)(max - min));

		private static readonly Color[] Palette =
		[
			Color.FromArgb(255, 0, 0), Color.FromArgb(0, 255, 0), Color.FromArgb(0, 0, 255), Color.FromArgb(255, 255, 255),
			Color.FromArgb(0, 0, 0), Color.FromArgb(128, 128, 128), Color.FromArgb(255, 128, 64), Color.FromArgb(16, 200, 128),
			Color.FromArgb(0, 128, 0), Color.FromArgb(0, 200, 0), Color.FromArgb(10, 255, 10), Color.FromArgb(255, 0, 8),
			Color.FromArgb(255, 8, 0), Color.FromArgb(1, 2, 3),
		];

		// The colours discrete values commonly hold, kept few so that values on different layers meet.
		private static readonly Color[] Discrete =
		[
			Color.FromArgb(255, 0, 0), Color.FromArgb(0, 255, 0), Color.FromArgb(0, 0, 255), Color.FromArgb(255, 255, 255),
		];

		private static readonly double[] Levels = [0, 1, 0.5, 0.25, 0.1, 0.75, 0.9, 0.333333333, 0.05];

		public (string kind, Color color, double intensity) State()
		{
			var kind = Next(0, 3) switch { 0 => "L", 1 => "R", _ => "D" };
			Color color;
			if (kind == "D") color = Discrete[Next(0, Discrete.Length)];
			else if (Next(0, 4) == 0) color = Color.FromArgb(Next(0, 256), Next(0, 256), Next(0, 256));
			else color = Palette[Next(0, Palette.Length)];
			var intensity = Next(0, 4) == 0 ? Next(0, 1001) / 1000.0 : Levels[Next(0, Levels.Length)];
			return (kind, color, intensity);
		}
	}
}
