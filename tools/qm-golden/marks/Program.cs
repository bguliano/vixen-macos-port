// Runs upstream's BeatsAndBars.cs and MarkCollection.cs over scripted features and writes what comes out, for
// crates/vixen-analysis/tests/golden_marks.rs.
//
//   QmMarksGolden GOLDEN_DIR
//
// GOLDEN_DIR is tests/golden/analysis. Features come from its JSON files; output goes to its marks folder.

using System.Reflection;
using System.Text;
using System.Text.Json;
using QMLibrary;
using Vixen.Marks;
using VixenModules.Analysis.BeatsAndBars;
using VixenModules.App.Marks;
using VixenModules.Media.Audio;

namespace QmMarksGolden;

internal sealed class Spec
{
	public string Name;
	public string Main;
	public string Preview = "preview_long_track";
	public Action<BeatBarSettingsData> Configure = _ => { };
	public int BeatsPerBar = 4;
	public int NoteSize = 4;
	public (string Name, bool IsDefault)[] Existing = Array.Empty<(string, bool)>();
	public int RawBytes = 4096 * 8;
	public int BytesPerSample = 4;
	public int Channels = 2;
	public float Frequency = 44100f;
	public List<object[]> CraftedBeats;
	public bool NoBars;
}

internal static class Program
{
	private static string _goldenDir;

	private static readonly Color BarsColor = Color.FromArgb(255, 255, 128);
	private static readonly Color BeatCountsColor = Color.FromArgb(255, 128, 0);

	private static void Defaults(BeatBarSettingsData s)
	{
		s.BarsEnabled = true;
		s.AllFeaturesEnabled = true;
		s.BeatCollectionsEnabled = true;
		s.BeatSplitsEnabled = false;
		s.Divisions = 1;
		s.AllFeaturesColor = Color.White;
		s.BarsColor = BarsColor;
		s.BeatCountsColor = BeatCountsColor;
		s.BeatSplitsColor = Color.Lime;
	}

	private static Action<BeatBarSettingsData> With(Action<BeatBarSettingsData> change) => s =>
	{
		Defaults(s);
		change(s);
	};

	private static IEnumerable<Spec> Specs()
	{
		yield return new Spec { Name = "defaults_bpb4", Main = "click_120_bpb4", Configure = Defaults };
		yield return new Spec
		{
			Name = "splits_bpb4",
			Main = "click_120_bpb4",
			Configure = With(s => { s.BeatSplitsEnabled = true; s.Divisions = 2; })
		};
		yield return new Spec
		{
			Name = "splits_bpb3",
			Main = "click_120_bpb3",
			BeatsPerBar = 3,
			Configure = With(s => { s.BeatSplitsEnabled = true; s.Divisions = 2; })
		};
		yield return new Spec
		{
			Name = "splits_bpb5_note8",
			Main = "click_120_bpb5",
			BeatsPerBar = 5,
			NoteSize = 8,
			Configure = With(s => { s.BeatSplitsEnabled = true; s.Divisions = 2; })
		};
		yield return new Spec
		{
			// The dialog allows ticking Beat Splits without Split Beats.
			Name = "splits_with_one_division",
			Main = "click_120_bpb4",
			Configure = With(s => s.BeatSplitsEnabled = true)
		};
		yield return new Spec
		{
			Name = "three_divisions",
			Main = "click_tempo_changes",
			Configure = With(s => { s.BeatSplitsEnabled = true; s.Divisions = 3; })
		};
		yield return new Spec
		{
			Name = "seven_divisions_all_only",
			Main = "bursts_130_bpb4",
			Configure = s => { s.AllFeaturesEnabled = true; s.Divisions = 7; }
		};
		yield return new Spec { Name = "only_bars", Main = "click_120_bpb4", Configure = s => s.BarsEnabled = true };
		yield return new Spec { Name = "only_counts", Main = "click_120_bpb4", Configure = s => s.BeatCollectionsEnabled = true };
		yield return new Spec
		{
			Name = "only_splits",
			Main = "click_120_bpb4",
			Configure = s => { s.BeatSplitsEnabled = true; s.Divisions = 2; }
		};
		yield return new Spec { Name = "only_all", Main = "click_120_bpb4", Configure = s => s.AllFeaturesEnabled = true };
		yield return new Spec { Name = "nothing_enabled", Main = "click_120_bpb4", Existing = new[] { ("Keep", false), ("Also", false) } };
		yield return new Spec
		{
			Name = "custom_name_and_colours",
			Main = "click_120_bpb4",
			Configure = With(s =>
			{
				s.CollectionBaseName = "My Song (v2)";
				s.BarsColor = Color.FromArgb(1, 2, 3);
				s.BeatCountsColor = Color.FromArgb(4, 5, 6);
				s.BeatSplitsColor = Color.FromArgb(7, 8, 9);
				s.AllFeaturesColor = Color.FromArgb(10, 11, 12);
				s.BeatSplitsEnabled = true;
				s.Divisions = 2;
			})
		};
		yield return new Spec
		{
			Name = "replaces_same_names",
			Main = "click_120_bpb4",
			Configure = Defaults,
			Existing = new[]
			{
				("Beats - All", false), ("Keep Me", false), ("Beats 1/4 Beat #1 (Bar)", false), ("Beats Beat #2", false),
				("beats - all", false), ("Beats Beat #3 ", false),
			}
		};
		yield return new Spec
		{
			Name = "keeps_existing_default",
			Main = "click_120_bpb4",
			Configure = Defaults,
			Existing = new[] { ("First", false), ("The Default", true) }
		};
		yield return new Spec
		{
			Name = "replacing_the_default_needs_a_new_one",
			Main = "click_120_bpb4",
			Configure = Defaults,
			Existing = new[] { ("Beats - All", true) }
		};
		yield return new Spec { Name = "long_bars_note16", Main = "click_bpb16", BeatsPerBar = 16, NoteSize = 16, Configure = With(s => { s.BeatSplitsEnabled = true; s.Divisions = 2; }) };
		yield return new Spec { Name = "tracker_without_downbeats_labels", Main = "click_bpb1", BeatsPerBar = 1, NoteSize = 4, Configure = With(s => { s.BeatSplitsEnabled = true; s.Divisions = 2; }) };
		yield return new Spec { Name = "real_sandstorm", Main = "real_sandstorm", Preview = "real_sandstorm_preview", Configure = With(s => { s.BeatSplitsEnabled = true; s.Divisions = 2; }) };
		yield return new Spec { Name = "real_wizards_defaults", Main = "real_wizards", Preview = "real_sandstorm_preview", Configure = Defaults };

		// What the two dictionary lookups throw on.
		yield return new Spec { Name = "no_features_at_all", Main = "short_noise_0", Configure = Defaults };
		yield return new Spec { Name = "no_preview_features", Main = "click_120_bpb4", Preview = "short_noise_0", Configure = Defaults };
		yield return new Spec
		{
			Name = "bars_missing",
			Main = "crafted",
			Preview = "crafted",
			NoBars = true,
			Configure = Defaults,
			Existing = new[] { ("Beats 1/4 Beat #1 (Bar)", false), ("Keep", false) },
			CraftedBeats = new List<object[]>
			{
				new object[] { 0, 500_000_000, "1" }, new object[] { 1, 0, "2" }, new object[] { 1, 500_000_000, "1" },
				new object[] { 2, 0, "2" },
			}
		};
		yield return new Spec
		{
			Name = "bars_missing_but_not_wanted",
			Main = "crafted",
			Preview = "crafted",
			NoBars = true,
			Configure = s => { s.BeatCollectionsEnabled = true; s.AllFeaturesEnabled = true; s.BeatSplitsEnabled = true; s.Divisions = 2; },
			CraftedBeats = new List<object[]>
			{
				new object[] { 0, 500_000_000, "1" }, new object[] { 1, 0, "2" }, new object[] { 1, 500_000_000, "1" },
				new object[] { 2, 0, "2" },
			}
		};

		// Features no tracker run happens to give: a first beat at time zero, equal times, labels out of range.
		yield return new Spec
		{
			Name = "crafted_edges",
			Main = "crafted",
			Preview = "crafted",
			Configure = With(s => { s.BeatSplitsEnabled = true; s.Divisions = 2; }),
			CraftedBeats = new List<object[]>
			{
				new object[] { 0, 0, "1" }, new object[] { 0, 333_333_333, "2" }, new object[] { 0, 333_333_333, "3" },
				new object[] { 1, 1_000, "4" }, new object[] { 1, 999_999, "1" }, new object[] { 1, 1_000_000, "9" },
				new object[] { 2, 500_000_000, "0" }, new object[] { 3, 0, "-1" }, new object[] { 3, 777_777_777, "2" },
				new object[] { 4, 1, "3" },
			}
		};
		yield return new Spec
		{
			Name = "crafted_period_from_a_later_downbeat",
			Main = "crafted",
			Preview = "crafted",
			Configure = Defaults,
			CraftedBeats = new List<object[]>
			{
				new object[] { 0, 100_000_000, "3" }, new object[] { 0, 600_000_000, "4" }, new object[] { 1, 100_000_000, "1" },
				new object[] { 1, 700_000_000, "2" }, new object[] { 2, 100_000_000, "3" }, new object[] { 2, 800_000_000, "4" },
				new object[] { 3, 100_000_000, "1" },
			}
		};

		// The sample conversion: both reads, and a length that doesn't divide evenly.
		yield return new Spec { Name = "raw_two_byte_path", Main = "click_120_bpb4", Configure = Defaults, BytesPerSample = 2 };
		yield return new Spec { Name = "raw_partial_sample", Main = "click_120_bpb4", Configure = Defaults, RawBytes = 4096 * 8 + 2 };
		yield return new Spec { Name = "raw_shorter_than_preview", Main = "click_120_bpb4", Configure = Defaults, RawBytes = 1500 };
		yield return new Spec { Name = "raw_empty", Main = "click_120_bpb4", Configure = Defaults, RawBytes = 0 };
		yield return new Spec { Name = "raw_longer_than_preview", Main = "click_120_bpb4", Configure = Defaults, RawBytes = 22050 * 4 * 12, Frequency = 22050f };
		yield return new Spec { Name = "raw_48000", Main = "click_120_bpb4", Configure = Defaults, RawBytes = 48000 * 4 * 2, Frequency = 48000f };
		yield return new Spec { Name = "no_channels", Main = "click_120_bpb4", Configure = Defaults, Channels = 0, Existing = new[] { ("Keep", false) } };
	}

	private static byte[] RawBytes(int count)
	{
		var bytes = new byte[count];
		uint state = 2463534242;
		for (int i = 0; i < count; i++)
		{
			state ^= state << 13;
			state ^= state >> 17;
			state ^= state << 5;
			bytes[i] = (byte)(state >> 24);
		}
		return bytes;
	}

	private static IDictionary<int, ICollection<ManagedFeature>> LoadFeatures(string name)
	{
		var result = new Dictionary<int, ICollection<ManagedFeature>>();
		using var doc = JsonDocument.Parse(File.ReadAllText(Path.Combine(_goldenDir, name + ".json")));
		foreach (var output in doc.RootElement.GetProperty("features").EnumerateObject())
		{
			var list = new List<ManagedFeature>();
			foreach (var entry in output.Value.EnumerateArray())
			{
				var values = new List<float>();
				foreach (var v in entry[3].EnumerateArray())
				{
					values.Add((float)v.GetDouble());
				}
				list.Add(new ManagedFeature
				{
					hasTimestamp = true,
					timestamp = new ManagedRealtime(entry[0].GetInt32(), entry[1].GetInt32()),
					label = entry[2].GetString(),
					values = values
				});
			}
			result[int.Parse(output.Name)] = list;
		}
		return result;
	}

	private static IDictionary<int, ICollection<ManagedFeature>> Crafted(List<object[]> beats, bool noBars)
	{
		ICollection<ManagedFeature> Feature(IEnumerable<object[]> entries) => entries
			.Select(e => new ManagedFeature
			{
				hasTimestamp = true,
				timestamp = new ManagedRealtime((int)e[0], (int)e[1]),
				label = (string)e[2],
				values = new List<float>()
			})
			.ToList();

		var result = new Dictionary<int, ICollection<ManagedFeature>>
		{
			[0] = Feature(beats),
			[1] = Feature(beats.Where(b => (string)b[2] == "1")),
			[2] = Feature(beats),
		};
		if (noBars)
		{
			result.Remove(1);
		}
		return result;
	}

	private static void WriteCollection(Utf8JsonWriter w, IMarkCollection c)
	{
		w.WriteStartObject();
		w.WriteString("name", c.Name);
		var color = c.Decorator.Color;
		w.WriteStartArray("color");
		w.WriteNumberValue(color.A);
		w.WriteNumberValue(color.R);
		w.WriteNumberValue(color.G);
		w.WriteNumberValue(color.B);
		w.WriteEndArray();
		w.WriteBoolean("is_bold", c.Decorator.IsBold);
		w.WriteBoolean("is_solid_line", c.Decorator.IsSolidLine);
		w.WriteBoolean("is_default", c.IsDefault);
		w.WriteNumber("level", c.Level);
		w.WriteBoolean("show_grid_lines", c.ShowGridLines);
		w.WriteBoolean("show_mark_bar", c.ShowMarkBar);
		w.WriteBoolean("show_tail_grid_lines", c.ShowTailGridLines);
		w.WriteBoolean("locked", c.Locked);
		w.WriteString("collection_type", c.CollectionType.ToString());
		w.WriteBoolean("linked", c.LinkedMarkCollectionId != Guid.Empty);
		w.WriteStartArray("marks");
		foreach (var mark in c.Marks)
		{
			w.WriteStartArray();
			w.WriteNumberValue(mark.StartTime.Ticks);
			w.WriteNumberValue(mark.Duration.Ticks);
			w.WriteStringValue(mark.Text);
			w.WriteEndArray();
		}
		w.WriteEndArray();
		w.WriteEndObject();
	}

	private static string HashOf(float[] values)
	{
		ulong hash = 0xcbf29ce484222325UL;
		foreach (var value in values)
		{
			foreach (var b in BitConverter.GetBytes(value))
			{
				hash ^= b;
				hash *= 0x100000001b3UL;
			}
		}
		return "0x" + hash.ToString("x16");
	}

	private static void Run(Spec spec, string outDir)
	{
		QMBarBeatTrack.ScriptedFeatures.Clear();
		QMBarBeatTrack.Recordings.Clear();
		BeatsAndBarsDialog.LastPreview = null;

		var preview = spec.CraftedBeats != null ? Crafted(spec.CraftedBeats, false) : LoadFeatures(spec.Preview);
		var main = spec.CraftedBeats != null ? Crafted(spec.CraftedBeats, spec.NoBars) : LoadFeatures(spec.Main);
		QMBarBeatTrack.ScriptedFeatures.Enqueue(preview);
		QMBarBeatTrack.ScriptedFeatures.Enqueue(main);

		var settings = new BeatBarSettingsData("Beats") { BeatsPerBar = spec.BeatsPerBar, NoteSize = spec.NoteSize };
		spec.Configure(settings);
		BeatsAndBarsDialog.NextSettings = settings;

		var collections = new List<IMarkCollection>();
		foreach (var (name, isDefault) in spec.Existing)
		{
			var existing = new MarkCollection { Name = name, IsDefault = isDefault };
			existing.AddMark(new Mark(TimeSpan.FromMilliseconds(123)));
			collections.Add(existing);
		}

		var audio = new Audio
		{
			Frequency = spec.Frequency,
			Channels = spec.Channels,
			BytesPerSample = spec.BytesPerSample,
			Raw = RawBytes(spec.RawBytes)
		};
		var module = new BeatsAndBars(audio);

		string exception = null;
		try
		{
			module.DoBeatBarDetection(collections);
		}
		catch (Exception e)
		{
			exception = e.GetType().Name;
		}

		var flags = BindingFlags.NonPublic | BindingFlags.Instance;
		var all = (float[])typeof(BeatsAndBars).GetField("_fSamplesAll", flags).GetValue(module);
		var previewSamples = (float[])typeof(BeatsAndBars).GetField("_fSamplesPreview", flags).GetValue(module);

		var path = Path.Combine(outDir, spec.Name + ".json");
		using (var stream = File.Create(path))
		using (var w = new Utf8JsonWriter(stream, new JsonWriterOptions { Indented = true }))
		{
			w.WriteStartObject();
			w.WriteString("name", spec.Name);
			w.WriteString("main_features", spec.Main);
			w.WriteString("preview_features", spec.Preview);

			if (spec.CraftedBeats != null)
			{
				w.WriteStartArray("crafted_beats");
				foreach (var beat in spec.CraftedBeats)
				{
					w.WriteStartArray();
					w.WriteNumberValue((int)beat[0]);
					w.WriteNumberValue((int)beat[1]);
					w.WriteStringValue((string)beat[2]);
					w.WriteEndArray();
				}
				w.WriteEndArray();
				w.WriteBoolean("no_bars", spec.NoBars);
			}

			w.WriteStartObject("settings");
			w.WriteBoolean("bars_enabled", settings.BarsEnabled);
			w.WriteBoolean("beat_collections_enabled", settings.BeatCollectionsEnabled);
			w.WriteBoolean("beat_splits_enabled", settings.BeatSplitsEnabled);
			w.WriteBoolean("all_features_enabled", settings.AllFeaturesEnabled);
			w.WriteString("collection_base_name", settings.CollectionBaseName);
			w.WriteNumber("divisions", settings.Divisions);
			foreach (var (key, color) in new[]
			{
				("all_features_color", settings.AllFeaturesColor), ("bars_color", settings.BarsColor),
				("beat_counts_color", settings.BeatCountsColor), ("beat_splits_color", settings.BeatSplitsColor)
			})
			{
				w.WriteStartArray(key);
				w.WriteNumberValue(color.A);
				w.WriteNumberValue(color.R);
				w.WriteNumberValue(color.G);
				w.WriteNumberValue(color.B);
				w.WriteEndArray();
			}
			w.WriteNumber("beats_per_bar", settings.BeatsPerBar);
			w.WriteNumber("note_size", settings.NoteSize);
			w.WriteString("all_collection_name", settings.AllCollectionName);
			w.WriteString("bars_collection_name", settings.BarsCollectionName);
			w.WriteStartArray("beat_collection_names");
			foreach (var n in settings.BeatCollectionNames(false)) w.WriteStringValue(n);
			w.WriteEndArray();
			w.WriteStartArray("split_collection_names");
			foreach (var n in settings.BeatCollectionNames(true)) w.WriteStringValue(n);
			w.WriteEndArray();
			w.WriteEndObject();

			w.WriteStartArray("existing");
			foreach (var (name, isDefault) in spec.Existing)
			{
				w.WriteStartObject();
				w.WriteString("name", name);
				w.WriteBoolean("is_default", isDefault);
				w.WriteEndObject();
			}
			w.WriteEndArray();

			if (exception != null) w.WriteString("exception", exception); else w.WriteNull("exception");

			w.WriteStartObject("audio");
			w.WriteNumber("raw_bytes", spec.RawBytes);
			w.WriteNumber("bytes_per_sample", spec.BytesPerSample);
			w.WriteNumber("channels", spec.Channels);
			w.WriteNumber("frequency", spec.Frequency);
			if (all != null)
			{
				w.WriteNumber("samples_len", all.Length);
				w.WriteString("samples_fnv1a64", HashOf(all));
			}
			if (previewSamples != null)
			{
				w.WriteNumber("preview_len", previewSamples.Length);
				w.WriteString("preview_fnv1a64", HashOf(previewSamples));
			}
			w.WriteStartArray("plugins");
			foreach (var r in QMBarBeatTrack.Recordings)
			{
				w.WriteStartObject();
				w.WriteNumber("blocks", r.Blocks);
				w.WriteString("blocks_fnv1a64", "0x" + r.Hash.ToString("x16"));
				w.WriteNumber("bpb", r.Parameters.TryGetValue("bpb", out var bpb) ? bpb : -1);
				w.WriteEndObject();
			}
			w.WriteEndArray();
			w.WriteEndObject();

			if (BeatsAndBarsDialog.LastPreview is { } p)
			{
				w.WriteStartObject("preview");
				w.WriteNumber("beat_period", p.BeatPeriod);
				w.WritePropertyName("collection");
				WriteCollection(w, p.PreviewCollection);
				w.WritePropertyName("split_collection");
				WriteCollection(w, p.PreviewSplitCollection);
				w.WriteEndObject();
			}
			else
			{
				w.WriteNull("preview");
			}

			w.WriteStartArray("collections");
			foreach (var c in collections) WriteCollection(w, c);
			w.WriteEndArray();
			w.WriteEndObject();
		}
	}

	private static int Main(string[] args)
	{
		if (args.Length != 1)
		{
			Console.Error.WriteLine("usage: QmMarksGolden GOLDEN_DIR");
			return 1;
		}
		_goldenDir = args[0];
		var outDir = Path.Combine(_goldenDir, "marks");
		Directory.CreateDirectory(outDir);

		int count = 0;
		foreach (var spec in Specs())
		{
			Run(spec, outDir);
			count++;
		}
		Console.WriteLine($"{count} mark cases written to {outDir}");
		return 0;
	}
}
