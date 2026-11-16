// Nested editor records use original child setters and the original data-contract reload.
// Keep these after the historical generic cases so existing random seeds remain stable.
namespace VixenGolden;
public static partial class Cases
{
    private static RecordV WaveRecord(params (string, object)[] values) => new(values);
    // Whirl's editor constructor uses 1 while its serialized data defaults use 2.
    // State both values explicitly so each runtime receives the same authored record.
    private static RecordV WhirlRecord(params (string, object)[] values) => new(Merge(P(("Spacing", 1), ("Thickness", 1)), values));
    private static EffectSpec WaveListCases(EffectSpec s)
    {
        var types = new[] { "Sine", "Triangle", "Square", "DecayingSine", "FractalIvey", "Sawtooth" };
        var dirs = new[] { "LeftToRight", "RightToLeft", "TopToBottom", "BottomToTop" };
        var moves = new[] { "Continuous", "Snake", "GrowAndShrink" };
        var n = 0;
        foreach (var type in types) foreach (var dir in dirs) foreach (var move in moves)
        {
            var record = WaveRecord(("WaveType", type), ("Direction", dir), ("MovementType", move),
                ("Mirror", n % 2 == 0), ("PrimeWave", move == "Snake" || n % 3 == 0),
                ("ColorHandling", n % 2 == 0 ? "Across" : "Along"), ("Color", V.Rainbow));
            s.Add($"lists/cross/{type}/{dir}/{move}", "grid", 2000, ("WaveformData", new RecordListV(record)));
            s.Add($"lists/cross-location/{type}/{dir}/{move}", "grid", 1500, ("TargetPositioning", "Locations"), ("WaveformData", new RecordListV(record)));
            n++;
        }
        foreach (var field in new[] { "Speed", "Thickness", "Frequency", "Height", "YOffset" })
            foreach (var curve in new[] { new CurveV((0, 0), (100, 0)), new CurveV((0, 100), (100, 100)), V.RampUp, V.Multi })
                s.Add($"lists/curve/{field}/{NameOf(curve)}", "grid", 2000, ("WaveformData", new RecordListV(WaveRecord((field, curve), ("PrimeWave", true)))));
        foreach (var type in types)
        {
            foreach (var phase in new[] { 0, 45, 90, 180 })
                s.Add($"lists/phase/{type}/{phase}", "grid", 2000, ("WaveformData", new RecordListV(WaveRecord(("WaveType", type), ("PhaseShift", phase), ("Color", V.RedBlue)))));
            foreach (var length in new[] { 1, 25, 50 })
                s.Add($"lists/snake/{type}/{length}", "grid", 2000, ("WaveformData", new RecordListV(WaveRecord(("WaveType", type), ("MovementType", "Snake"), ("WindowPercentage", length)))));
        }
        foreach (var color in new[] { V.Red1, V.RedBlue, V.Focused, V.Faded, V.Hard })
            foreach (var handling in new[] { "Across", "Along" })
                s.Add($"lists/gradient/{NameOf(color)}/{handling}", "grid", 1500, ("WaveformData", new RecordListV(WaveRecord(("Color", color), ("ColorHandling", handling)))));
        foreach (var count in new[] { 0, 1, 2, 3, 6 })
            foreach (var target in new[] { "single", "grid", "uneven", "discrete_rgb", "grid+string30" })
            {
                var records = Enumerable.Range(0, count).Select(i => WaveRecord(("WaveType", types[i]), ("Color", i % 2 == 0 ? V.RedBlue : V.Rainbow),
                    ("Direction", dirs[i % 4]), ("PhaseShift", i * 30), ("Mirror", i % 2 == 0))).ToArray();
                s.Add($"lists/count/{count}/{target}", target, 2000, ("WaveformData", new RecordListV(records)));
            }
        foreach (var marks in new[] { true, false }) foreach (var id in new[] { SampleMarks()[0].Id, SampleMarks()[1].Id, new Guid("00000000-0000-0000-0000-00000000dead"), Guid.Empty })
        {
            var c = s.Add($"lists/marks/{marks}/{id}", "grid", 2000, ("WaveformData", new RecordListV(WaveRecord(("WaveType", "DecayingSine"), ("UseMarks", marks), ("MarkCollectionId", id.ToString())))));
            c.Marks = SampleMarks().ToList(); c.StartMs = 300;
        }
        foreach (var id in new[] { new Guid("00000000-0000-0000-0000-00000000dead"), Guid.Empty })
        {
            var missing = WaveRecord(("WaveType", "DecayingSine"), ("UseMarks", true), ("MarkCollectionId", id.ToString()));
            foreach (var trigger in new[] { "WaveType", "UseMarks", "Color" })
            {
                object value = trigger == "WaveType" ? "Triangle" : trigger == "UseMarks" ? true : V.RedBlue;
                var c = s.Add($"lists/marks/other-row/{trigger}/{id}", "grid", 2000,
                    ("WaveformData", new RecordListV(missing, WaveRecord((trigger, value)))));
                c.Marks = SampleMarks().ToList(); c.StartMs = 300;
            }
            var local = s.Add($"lists/marks/unrelated-color/{id}", "grid", 2000,
                ("WaveformData", new RecordListV(WaveRecord(missing.Values.Concat(P(("Color", V.RedBlue))).ToArray()))));
            local.Marks = SampleMarks().ToList(); local.StartMs = 300;
        }
        return s;
    }
    private static EffectSpec WhirlListCases(EffectSpec s)
    {
        void Add(string id, int slot, RecordV record, string target = "grid", params (string, object)[] extra)
        {
            var records = Enumerable.Range(0, 4).Select(i => WhirlRecord(("SingleColor", i % 2 == 0 ? V.Red1 : V.RedBlue))).ToArray();
            records[slot] = record;
            s.Add($"lists/{slot}/{id}/{target}", target, 2000, Merge(P(("Rows", 2), ("Columns", 2), ("IndividualConfiguration", true), ("WhirlData", new RecordListV(records))), extra));
        }
        var choices = new Dictionary<string, object[]> {
            ["WhirlMode"] = ["RecurrentWhirls", "SymmetricalWhirls", "Meteor"],
            ["ColorMode"] = ["GradientOverTime", "LegColors", "RectangularRings", "Bands"],
            ["WhirlDirection"] = ["In", "Out", "InAndOut"],
            ["Rotation"] = ["Clockwise", "CounterClockwise"],
            ["StartLocation"] = ["TopLeft", "TopRight", "BottomLeft", "BottomRight"],
            ["Enabled"] = [false, true], ["ReverseDraw"] = [true, false], ["Show3D"] = [true, false],
            ["Spacing"] = [0, 25, 100], ["Thickness"] = [1, 25, 100],
            ["TailLength"] = [1, 25, 100], ["BandLength"] = [1, 25, 100],
            ["SingleColor"] = [V.RedBlue, V.Focused, V.Faded],
            ["LeftColor"] = [V.RedBlue], ["RightColor"] = [V.Focused], ["TopColor"] = [V.Faded], ["BottomColor"] = [V.Rainbow],
            ["Colors"] = [V.One, V.Two, V.Four],
        };
        for (var slot = 0; slot < 4; slot++)
            foreach (var (field, values) in choices) foreach (var value in values)
            {
                var mode = field is "LeftColor" or "RightColor" or "TopColor" or "BottomColor" ? "LegColors" : field is "BandLength" or "Colors" ? "Bands" : "GradientOverTime";
                Add($"ofat/{field}/{NameOf(value)}", slot, WhirlRecord(Merge(P(("ColorMode", mode), ("WhirlMode", field == "TailLength" ? "Meteor" : "RecurrentWhirls"), ("Thickness", field == "Show3D" ? 40 : 1)), (field, value))));
            }
        var n = 0;
        foreach (var mode in choices["WhirlMode"]) foreach (var color in choices["ColorMode"]) foreach (var dir in choices["WhirlDirection"])
            foreach (var rotation in choices["Rotation"])
            {
                Add($"cross/{mode}/{color}/{dir}/{rotation}", n % 4, WhirlRecord(("WhirlMode", mode), ("ColorMode", color), ("WhirlDirection", dir), ("Rotation", rotation),
                    ("StartLocation", choices["StartLocation"][n % 4]), ("ReverseDraw", n % 2 == 0), ("Show3D", n % 3 == 0), ("Thickness", 25), ("Colors", V.Four)),
                    "grid", ("TargetPositioning", n % 2 == 0 ? "Locations" : "Strings"), ("PauseAtEnd", n % 3 == 0 ? 25 : 0), ("StringOrientation", n % 2 == 0 ? "Vertical" : "Horizontal")); n++;
            }
        foreach (var count in new[] { 0, 1, 2, 4, 6 })
            s.Add($"lists/count/{count}", "grid", 1500, ("Rows", 2), ("Columns", 2), ("IndividualConfiguration", true),
                ("WhirlData", new RecordListV(Enumerable.Range(0, count).Select(i => WhirlRecord(("Enabled", i % 2 == 0), ("ColorMode", "Bands"), ("Colors", V.Four))).ToArray())));
        return s;
    }
}
