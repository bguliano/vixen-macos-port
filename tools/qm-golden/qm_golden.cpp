// Golden data generator for crates/vixen-analysis.
//
// Built by build.sh together with the unmodified upstream QM sources, this drives BarBeatTracker the way
// BeatsAndBars.cs does and writes every feature to JSON, plus the intermediate values a Rust test can compare when a
// feature differs. Audio samples never go into the output: only their count and a hash.
//
//   qm-golden --synthetic OUT_DIR
//       writes every case in the table below
//   qm-golden --raw FILE.f32 --rate N --name NAME --out OUT_DIR [--driver D] [--bpb N] [--alpha X] [--tempo X]
//             [--constrain X] [--note TEXT] [--full DIR] [--time]
//       runs little-endian f32 samples from a file. Real music comes in this way, decoded by the Rust side.

#include "BarBeatTrack.h"
#include "dsp/onsets/DetectionFunction.h"
#include "dsp/tempotracking/DownBeat.h"
#include "dsp/tempotracking/TempoTrackV2.h"
#include "maths/MathUtilities.h"

#include <algorithm>
#include <chrono>
#include <cinttypes>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <utility>
#include <vector>

using std::string;
using std::vector;

typedef Vamp::Plugin::FeatureSet FeatureSet;

struct Spec {
    string name;
    string kind = "click"; // click, bursts, noise, silence, dc, impulse, tone, raw
    int rate = 44100;
    int beats_per_bar = 4;
    vector<std::pair<int, double>> sections;
    uint32_t seed = 1;
    double amplitude = 1.0;
    int length = 0;
    int position = 0;
    double frequency = 0;
    int lead = 0;
    int tail = 0;
    string note;

    // beatsandbars: GenerateFeatures in BeatsAndBars.cs
    // preview: the same, over the first ten seconds zero padded to ten seconds, as DoBeatBarDetection prepares them
    // upstream_test: ProcessSamples in QmBarBeatTrackTests.cs
    // reset_rerun: half the blocks, reset(), then all of them
    // remaining_twice: getRemainingFeatures called twice, the second result written
    string driver = "beatsandbars";

    bool has_bpb = true;
    float bpb = 4;
    bool has_alpha = false;
    float alpha = 0;
    bool has_tempo = false;
    float tempo = 0;
    bool has_constrain = false;
    float constrain = 0;
};

// ---- input generators; crates/vixen-analysis/tests/common/mod.rs has the same ones ----

static uint32_t xorshift32(uint32_t &x)
{
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    return x;
}

// CreateClickTrack and AddClick from QmBarBeatTrackTests.cs. rint rounds ties to even like Math.Round.
static vector<int> beat_starts(const Spec &spec, int &total)
{
    vector<int> starts;
    int cursor = 0;
    for (const auto &section : spec.sections) {
        double samples_per_beat = spec.rate * 60.0 / section.second;
        for (int beat = 0; beat < section.first * spec.beats_per_bar; beat++) {
            starts.push_back(cursor + (int)rint(beat * samples_per_beat));
        }
        cursor += (int)rint(section.first * spec.beats_per_bar * samples_per_beat);
    }
    total = cursor + 257;
    return starts;
}

static void add_click(vector<float> &samples, int start, int rate, float amplitude, double frequency)
{
    int length = (int)(rate * 0.025);
    for (int offset = 0; offset < length && start + offset < (int)samples.size(); offset++) {
        double envelope = exp(-7.0 * offset / length);
        double phase = 2.0 * M_PI * frequency * offset / rate;
        samples[start + offset] += (float)(amplitude * envelope * sin(phase));
    }
}

static vector<float> generate(const Spec &spec)
{
    vector<float> samples;
    uint32_t state = spec.seed;

    if (spec.kind == "click") {
        int total = 0;
        vector<int> starts = beat_starts(spec, total);
        samples.assign(total, 0.f);
        for (int i = 0; i < (int)starts.size(); i++) {
            bool downbeat = i % spec.beats_per_bar == 0;
            add_click(samples, starts[i], spec.rate, downbeat ? 28000.f : 18000.f, downbeat ? 880.0 : 1760.0);
        }
    } else if (spec.kind == "bursts") {
        int total = 0;
        vector<int> starts = beat_starts(spec, total);
        samples.assign(total, 0.f);
        int length = (int)(spec.rate * 0.05);
        for (int i = 0; i < (int)starts.size(); i++) {
            double amplitude = i % spec.beats_per_bar == 0 ? spec.amplitude : spec.amplitude * 0.6;
            for (int offset = 0; offset < length && starts[i] + offset < total; offset++) {
                double envelope = exp(-5.0 * offset / length);
                double noise = (double)(int32_t)xorshift32(state) / 2147483648.0;
                samples[starts[i] + offset] += (float)(amplitude * envelope * noise);
            }
        }
    } else if (spec.kind == "noise") {
        samples.resize(spec.length);
        for (int i = 0; i < spec.length; i++) {
            samples[i] = (float)((double)(int32_t)xorshift32(state) * spec.amplitude);
        }
    } else if (spec.kind == "silence") {
        samples.assign(spec.length, 0.f);
    } else if (spec.kind == "dc") {
        samples.assign(spec.length, (float)spec.amplitude);
    } else if (spec.kind == "impulse") {
        samples.assign(spec.length, 0.f);
        samples[spec.position] = (float)spec.amplitude;
    } else if (spec.kind == "tone") {
        samples.resize(spec.length);
        for (int i = 0; i < spec.length; i++) {
            samples[i] = (float)(spec.amplitude * sin(2.0 * M_PI * spec.frequency * i / spec.rate));
        }
    }

    if (spec.lead > 0) {
        samples.insert(samples.begin(), spec.lead, 0.f);
    }
    samples.insert(samples.end(), spec.tail, 0.f);
    return samples;
}

// ---- drivers ----

static uint64_t fnv1a64(const void *data, size_t bytes)
{
    const unsigned char *p = (const unsigned char *)data;
    uint64_t hash = 0xcbf29ce484222325ULL;
    for (size_t i = 0; i < bytes; i++) {
        hash ^= p[i];
        hash *= 0x100000001b3ULL;
    }
    return hash;
}

static void set_parameters(BarBeatTracker &plugin, const Spec &spec)
{
    if (spec.has_bpb) plugin.setParameter("bpb", spec.bpb);
    if (spec.has_alpha) plugin.setParameter("alpha", spec.alpha);
    if (spec.has_tempo) plugin.setParameter("inputtempo", spec.tempo);
    if (spec.has_constrain) plugin.setParameter("constraintempo", spec.constrain);
}

// The block and timestamp sequence of one driver loop, without any plugin.
struct Block {
    vector<float> samples;
    Vamp::RealTime timestamp;
};

template <typename F>
static int for_each_block_generate_features(const vector<float> &data, int step_size, int block_size,
                                            unsigned frequency, F process)
{
    // GenerateFeatures in BeatsAndBars.cs, statement for statement.
    int processed = 0;
    vector<float> samples(block_size);
    int j;
    for (j = 0; ((int)data.size() - j) >= block_size; j += step_size) {
        std::copy(data.begin() + j, data.begin() + j + block_size, samples.begin());
        process(samples.data(), Vamp::RealTime::frame2RealTime(j, frequency));
        processed++;
    }

    std::fill(samples.begin(), samples.end(), 0.f);
    std::copy(data.begin() + j, data.end(), samples.begin());
    process(samples.data(), Vamp::RealTime::frame2RealTime(j, frequency));
    processed++;
    return processed;
}

template <typename F>
static int for_each_block_upstream_test(const vector<float> &data, int step_size, int block_size, unsigned frequency,
                                        F process)
{
    // ProcessSamples in QmBarBeatTrackTests.cs.
    int processed = 0;
    for (int offset = 0; offset < (int)data.size(); offset += step_size) {
        vector<float> block(block_size, 0.f);
        int count = std::min(block_size, (int)data.size() - offset);
        std::copy(data.begin() + offset, data.begin() + offset + count, block.begin());
        process(block.data(), Vamp::RealTime::frame2RealTime(offset, frequency));
        processed++;
    }
    return processed;
}

static vector<float> preview_samples(const vector<float> &all, float frequency)
{
    // DoBeatBarDetection: new float[(int)(Frequency * PreviewTime)], then Array.Copy of the shorter length.
    const int preview_time = 10;
    vector<float> preview((int)(frequency * preview_time), 0.f);
    int count = (int)std::min(frequency * preview_time, (float)all.size());
    std::copy(all.begin(), all.begin() + count, preview.begin());
    return preview;
}

struct Result {
    int step_size = 0;
    int block_size = 0;
    int blocks = 0;
    FeatureSet features;
    double seconds = 0;
};

static Result run_plugin(const Spec &spec, const vector<float> &data)
{
    Result result;
    float frequency = (float)spec.rate;
    BarBeatTracker plugin(frequency);
    set_parameters(plugin, spec);

    result.step_size = (int)plugin.getPreferredStepSize();
    result.block_size = (int)plugin.getPreferredBlockSize();
    if (!plugin.initialise(1, result.step_size, result.block_size)) {
        fprintf(stderr, "initialise failed for %s\n", spec.name.c_str());
        exit(2);
    }

    auto process = [&plugin](const float *block, Vamp::RealTime timestamp) {
        const float *const buffers[1] = {block};
        plugin.process(buffers, timestamp);
    };
    unsigned uf = (unsigned)frequency;

    auto start = std::chrono::steady_clock::now();
    if (spec.driver == "upstream_test") {
        result.blocks = for_each_block_upstream_test(data, result.step_size, result.block_size, uf, process);
        result.features = plugin.getRemainingFeatures();
    } else if (spec.driver == "preview") {
        vector<float> preview = preview_samples(data, frequency);
        result.blocks = for_each_block_generate_features(preview, result.step_size, result.block_size, uf, process);
        result.features = plugin.getRemainingFeatures();
    } else if (spec.driver == "reset_rerun") {
        vector<float> half(data.begin(), data.begin() + data.size() / 2);
        for_each_block_generate_features(half, result.step_size, result.block_size, uf, process);
        plugin.reset();
        result.blocks = for_each_block_generate_features(data, result.step_size, result.block_size, uf, process);
        result.features = plugin.getRemainingFeatures();
    } else if (spec.driver == "remaining_twice") {
        result.blocks = for_each_block_generate_features(data, result.step_size, result.block_size, uf, process);
        plugin.getRemainingFeatures();
        result.features = plugin.getRemainingFeatures();
    } else {
        result.blocks = for_each_block_generate_features(data, result.step_size, result.block_size, uf, process);
        result.features = plugin.getRemainingFeatures();
    }
    result.seconds = std::chrono::duration<double>(std::chrono::steady_clock::now() - start).count();
    return result;
}

// The tracker hides its intermediate values, so build the same pipeline from the same upstream classes and check below
// that it reaches the plugin's beats.
struct Intermediate {
    vector<double> df;
    vector<double> beat_period;
    vector<double> tempi;
    vector<double> beats;
    vector<int> downbeats;
    vector<double> beatsd;
    vector<float> downsampled;
    bool empty_df = false;
};

static Intermediate run_shadow(const Spec &spec, const vector<float> &data, int step_size, int block_size)
{
    Intermediate out;
    float rate = (float)spec.rate;

    DFConfig config;
    config.DFType = DF_COMPLEXSD;
    config.stepSize = step_size;
    config.frameLength = block_size;
    config.dbRise = 3;
    config.adaptiveWhitening = false;
    config.whiteningRelaxCoeff = -1;
    config.whiteningFloor = -1;

    DetectionFunction *df = new DetectionFunction(config);
    int factor = MathUtilities::nextPowerOfTwo(rate / 3000);
    DownBeat downbeat(rate, factor, step_size);

    int bpb = spec.has_bpb ? (int)lrintf(spec.bpb) : 4;
    downbeat.setBeatsPerBar(bpb);

    vector<double> df_output;
    vector<double> dfinput(block_size);
    auto process = [&](const float *block, Vamp::RealTime) {
        for (int i = 0; i < block_size; i++) dfinput[i] = block[i];
        df_output.push_back(df->processTimeDomain(dfinput.data()));
        downbeat.pushAudioBlock(block);
    };
    unsigned uf = (unsigned)rate;

    if (spec.driver == "upstream_test") {
        for_each_block_upstream_test(data, step_size, block_size, uf, process);
    } else if (spec.driver == "preview") {
        for_each_block_generate_features(preview_samples(data, rate), step_size, block_size, uf, process);
    } else if (spec.driver == "reset_rerun") {
        vector<float> half(data.begin(), data.begin() + data.size() / 2);
        for_each_block_generate_features(half, step_size, block_size, uf, process);
        delete df;
        df = new DetectionFunction(config);
        df_output.clear();
        downbeat.resetAudioBuffer();
        for_each_block_generate_features(data, step_size, block_size, uf, process);
    } else {
        for_each_block_generate_features(data, step_size, block_size, uf, process);
    }
    delete df;

    for (size_t i = 2; i < df_output.size(); i++) {
        out.df.push_back(df_output[i]);
        out.beat_period.push_back(0.0);
    }
    if (out.df.empty()) {
        out.empty_df = true;
        return out;
    }

    double alpha = spec.has_alpha ? (double)spec.alpha : 0.9;
    double tempo = spec.has_tempo ? (double)spec.tempo : 120.0;
    bool constrain = spec.has_constrain ? spec.constrain > 0.5 : false;

    TempoTrackV2 tt(rate, step_size);
    tt.calculateBeatPeriod(out.df, out.beat_period, out.tempi, tempo, constrain);
    tt.calculateBeats(out.df, out.beat_period, out.beats, alpha, 4.0);

    size_t length = 0;
    const float *downsampled = downbeat.getBufferedAudio(length);
    out.downsampled.assign(downsampled, downsampled + length);
    downbeat.findDownBeats(downsampled, length, out.beats, out.downbeats);
    downbeat.getBeatSD(out.beatsd);
    return out;
}

static void check_shadow(const Spec &spec, const Result &result, const Intermediate &shadow)
{
    auto beats = result.features.find(0);
    size_t expected = beats == result.features.end() ? 0 : beats->second.size();
    bool same = expected == shadow.beats.size();
    for (size_t i = 0; same && i < expected; i++) {
        size_t frame = size_t(shadow.beats[i]) * result.step_size;
        Vamp::RealTime t = Vamp::RealTime::frame2RealTime(frame, lrintf((float)spec.rate));
        same = t.sec == beats->second[i].timestamp.sec && t.nsec == beats->second[i].timestamp.nsec;
    }
    if (!same) {
        fprintf(stderr, "%s: the shadow pipeline disagrees with the plugin\n", spec.name.c_str());
        exit(3);
    }
}

// ---- JSON ----

static void write_double_array(FILE *f, const vector<double> &values)
{
    fputc('[', f);
    for (size_t i = 0; i < values.size(); i++) {
        if (i) fputc(',', f);
        fprintf(f, "%.17g", values[i]);
    }
    fputc(']', f);
}

static void write_hash(FILE *f, const char *key, const void *data, size_t bytes, bool comma = true)
{
    fprintf(f, "    \"%s\": \"0x%016" PRIx64 "\"%s\n", key, fnv1a64(data, bytes), comma ? "," : "");
}

static void write_json(const string &dir, const Spec &spec, const vector<float> &input, const Result &result,
                       const Intermediate &shadow)
{
    string path = dir + "/" + spec.name + ".json";
    FILE *f = fopen(path.c_str(), "w");
    if (!f) {
        fprintf(stderr, "cannot write %s\n", path.c_str());
        exit(2);
    }

    fprintf(f, "{\n  \"name\": \"%s\",\n", spec.name.c_str());

    fprintf(f, "  \"input\": {\"kind\": \"%s\", \"rate\": %d", spec.kind.c_str(), spec.rate);
    if (spec.kind == "click" || spec.kind == "bursts") {
        fprintf(f, ", \"beats_per_bar\": %d, \"sections\": [", spec.beats_per_bar);
        for (size_t i = 0; i < spec.sections.size(); i++) {
            fprintf(f, "%s[%d, %.17g]", i ? ", " : "", spec.sections[i].first, spec.sections[i].second);
        }
        fprintf(f, "]");
    }
    if (spec.kind != "click" && spec.kind != "raw") {
        fprintf(f, ", \"seed\": %u, \"amplitude\": %.17g, \"length\": %d, \"position\": %d, \"frequency\": %.17g",
                spec.seed, spec.amplitude, spec.length, spec.position, spec.frequency);
    }
    if (spec.kind != "raw") {
        fprintf(f, ", \"lead\": %d, \"tail\": %d", spec.lead, spec.tail);
    }
    if (!spec.note.empty()) {
        fprintf(f, ", \"note\": \"%s\"", spec.note.c_str());
    }
    fprintf(f, "},\n");

    fprintf(f, "  \"input_len\": %zu,\n", input.size());
    fprintf(f, "  \"input_fnv1a64\": \"0x%016" PRIx64 "\",\n", fnv1a64(input.data(), input.size() * sizeof(float)));
    fprintf(f, "  \"driver\": \"%s\",\n", spec.driver.c_str());

    fprintf(f, "  \"params\": {");
    const char *sep = "";
    if (spec.has_bpb) {
        fprintf(f, "%s\"bpb\": %.9g", sep, spec.bpb);
        sep = ", ";
    }
    if (spec.has_alpha) {
        fprintf(f, "%s\"alpha\": %.9g", sep, spec.alpha);
        sep = ", ";
    }
    if (spec.has_tempo) {
        fprintf(f, "%s\"inputtempo\": %.9g", sep, spec.tempo);
        sep = ", ";
    }
    if (spec.has_constrain) {
        fprintf(f, "%s\"constraintempo\": %.9g", sep, spec.constrain);
    }
    fprintf(f, "},\n");

    fprintf(f, "  \"step_size\": %d,\n  \"block_size\": %d,\n  \"blocks\": %d,\n", result.step_size, result.block_size,
            result.blocks);

    // [sec, nsec, label, [values]]; a non-finite value is written as null.
    fprintf(f, "  \"features\": {");
    bool first_output = true;
    for (const auto &output : result.features) {
        fprintf(f, "%s\n    \"%d\": [", first_output ? "" : ",", output.first);
        first_output = false;
        bool first_feature = true;
        for (const auto &feature : output.second) {
            fprintf(f, "%s\n      [%d, %d, \"%s\", [", first_feature ? "" : ",", feature.timestamp.sec,
                    feature.timestamp.nsec, feature.label.c_str());
            first_feature = false;
            for (size_t i = 0; i < feature.values.size(); i++) {
                if (i) fputc(',', f);
                if (std::isfinite(feature.values[i])) {
                    fprintf(f, "%.9g", feature.values[i]);
                } else {
                    fputs("null", f);
                }
            }
            fputs("]]", f);
        }
        fputs("\n    ]", f);
    }
    fprintf(f, "\n  },\n");

    fprintf(f, "  \"intermediate\": {\n");
    fprintf(f, "    \"df_len\": %zu,\n", shadow.df.size());
    write_hash(f, "df_fnv1a64", shadow.df.data(), shadow.df.size() * sizeof(double));

    fprintf(f, "    \"beat_period_rle\": [");
    for (size_t i = 0; i < shadow.beat_period.size();) {
        size_t run = i;
        while (run < shadow.beat_period.size() && shadow.beat_period[run] == shadow.beat_period[i]) run++;
        fprintf(f, "%s[%.17g, %zu]", i ? ", " : "", shadow.beat_period[i], run - i);
        i = run;
    }
    fprintf(f, "],\n");

    fprintf(f, "    \"tempi_len\": %zu,\n", shadow.tempi.size());
    write_hash(f, "tempi_fnv1a64", shadow.tempi.data(), shadow.tempi.size() * sizeof(double));

    fprintf(f, "    \"beats\": ");
    write_double_array(f, shadow.beats);
    fprintf(f, ",\n    \"downbeats\": [");
    for (size_t i = 0; i < shadow.downbeats.size(); i++) {
        fprintf(f, "%s%d", i ? "," : "", shadow.downbeats[i]);
    }
    fprintf(f, "],\n");

    fprintf(f, "    \"beatsd_len\": %zu,\n", shadow.beatsd.size());
    write_hash(f, "beatsd_fnv1a64", shadow.beatsd.data(), shadow.beatsd.size() * sizeof(double));
    fprintf(f, "    \"downsampled_len\": %zu,\n", shadow.downsampled.size());
    write_hash(f, "downsampled_fnv1a64", shadow.downsampled.data(), shadow.downsampled.size() * sizeof(float), false);
    fprintf(f, "  }\n}\n");

    fclose(f);
}

static void write_binary(const string &path, const void *data, size_t bytes)
{
    FILE *f = fopen(path.c_str(), "wb");
    if (!f) {
        fprintf(stderr, "cannot write %s\n", path.c_str());
        exit(2);
    }
    fwrite(data, 1, bytes, f);
    fclose(f);
}

static void write_full(const string &dir, const Spec &spec, const Intermediate &shadow)
{
    string base = dir + "/" + spec.name;
    write_binary(base + ".df.f64", shadow.df.data(), shadow.df.size() * sizeof(double));
    write_binary(base + ".beat_period.f64", shadow.beat_period.data(), shadow.beat_period.size() * sizeof(double));
    write_binary(base + ".beatsd.f64", shadow.beatsd.data(), shadow.beatsd.size() * sizeof(double));
    write_binary(base + ".downsampled.f32", shadow.downsampled.data(), shadow.downsampled.size() * sizeof(float));
}

static void run_case(const Spec &spec, const vector<float> &input, const string &out_dir, const string &full_dir,
                     bool time)
{
    Result result = run_plugin(spec, input);
    Intermediate shadow = run_shadow(spec, input, result.step_size, result.block_size);
    if (spec.driver != "remaining_twice") {
        check_shadow(spec, result, shadow);
    }
    write_json(out_dir, spec, input, result, shadow);
    if (!full_dir.empty()) {
        write_full(full_dir, spec, shadow);
    }
    if (time) {
        printf("%s: %d blocks, %.3f s in the plugin\n", spec.name.c_str(), result.blocks, result.seconds);
    }
}

// ---- the case table ----

static Spec click(const string &name, int rate, int beats_per_bar, vector<std::pair<int, double>> sections, float bpb)
{
    Spec spec;
    spec.name = name;
    spec.kind = "click";
    spec.rate = rate;
    spec.beats_per_bar = beats_per_bar;
    spec.sections = std::move(sections);
    spec.bpb = bpb;
    return spec;
}

static Spec of_kind(const string &name, const string &kind, int length, double amplitude, uint32_t seed = 1)
{
    Spec spec;
    spec.name = name;
    spec.kind = kind;
    spec.length = length;
    spec.amplitude = amplitude;
    spec.seed = seed;
    return spec;
}

static vector<Spec> synthetic_cases()
{
    vector<Spec> cases;
    const vector<std::pair<int, double>> bars32 = {{32, 120.0}};

    // QmBarBeatTrackTests.cs, with its own driving loop, then the same track driven as BeatsAndBars.cs drives it.
    {
        Spec spec = click("upstream_test_click", 44100, 4, bars32, 4);
        spec.driver = "upstream_test";
        cases.push_back(spec);
    }
    cases.push_back(click("click_120_bpb4", 44100, 4, bars32, 4));
    cases.push_back(click("click_120_bpb3", 44100, 3, bars32, 3));
    cases.push_back(click("click_120_bpb5", 44100, 5, bars32, 5));

    // Non-default parameters.
    {
        Spec spec = click("click_120_bpb4_alpha05", 44100, 4, bars32, 4);
        spec.has_alpha = true;
        spec.alpha = 0.5f;
        cases.push_back(spec);
    }
    {
        Spec spec = click("click_120_bpb3_alpha099", 44100, 3, bars32, 3);
        spec.has_alpha = true;
        spec.alpha = 0.99f;
        cases.push_back(spec);
    }
    {
        Spec spec = click("click_90_tempo60", 44100, 4, {{24, 90.0}}, 4);
        spec.has_tempo = true;
        spec.tempo = 60;
        cases.push_back(spec);
    }
    {
        Spec spec = click("click_90_tempo180", 44100, 5, {{24, 90.0}}, 5);
        spec.has_tempo = true;
        spec.tempo = 180;
        cases.push_back(spec);
    }
    {
        Spec spec = click("click_120_constrain120", 44100, 4, bars32, 4);
        spec.has_tempo = true;
        spec.tempo = 120;
        spec.has_constrain = true;
        spec.constrain = 1;
        cases.push_back(spec);
    }
    {
        Spec spec = click("click_90_constrain90", 44100, 3, {{24, 90.0}}, 3);
        spec.has_tempo = true;
        spec.tempo = 90;
        spec.has_constrain = true;
        spec.constrain = 1;
        cases.push_back(spec);
    }
    {
        Spec spec = click("click_150_constrain75_alpha08", 44100, 4, {{24, 150.0}}, 4);
        spec.has_tempo = true;
        spec.tempo = 75;
        spec.has_constrain = true;
        spec.constrain = 1;
        spec.has_alpha = true;
        spec.alpha = 0.8f;
        cases.push_back(spec);
    }
    {
        Spec spec = click("click_constrain_off_at_half", 44100, 4, {{16, 120.0}}, 4);
        spec.has_constrain = true;
        spec.constrain = 0.5f;
        cases.push_back(spec);
    }

    // Tempo changes, rhythm from noise, and silence around the music.
    cases.push_back(click("click_tempo_changes", 44100, 4, {{16, 120.0}, {16, 90.0}, {16, 140.0}}, 4));
    {
        Spec spec = click("bursts_130_bpb4", 44100, 4, {{32, 130.0}}, 4);
        spec.kind = "bursts";
        spec.seed = 7;
        spec.amplitude = 1.5e9;
        cases.push_back(spec);
    }
    {
        Spec spec = click("click_after_silence", 44100, 4, {{16, 120.0}}, 4);
        spec.lead = 44100 * 7;
        cases.push_back(spec);
    }
    {
        Spec spec = click("click_before_silence", 44100, 4, {{16, 120.0}}, 4);
        spec.tail = 44100 * 11;
        cases.push_back(spec);
    }

    // Every beats-per-bar shape, including the out-of-range values the plugin still accepts.
    for (int bpb : {2, 6, 7, 16, 1, 0}) {
        cases.push_back(click("click_bpb" + std::to_string(bpb), 44100, 4, {{16, 120.0}}, (float)bpb));
    }
    {
        Spec spec = click("click_bpb_rounded_2_5", 44100, 4, {{16, 120.0}}, 2.5f);
        cases.push_back(spec);
    }
    {
        Spec spec = click("click_no_bpb_set", 44100, 4, {{16, 120.0}}, 4);
        spec.has_bpb = false;
        cases.push_back(spec);
    }

    // Noise at the scales audio really reaches this plugin at, and below.
    const int eight_seconds = 44100 * 8;
    cases.push_back(of_kind("noise_int32_scale", "noise", eight_seconds, 1.0, 11));
    cases.push_back(of_kind("noise_int16_scale", "noise", eight_seconds, 1.0 / 65536.0, 12));
    cases.push_back(of_kind("noise_unit_scale", "noise", eight_seconds, 1.0 / 2147483648.0, 13));
    cases.push_back(of_kind("noise_quiet", "noise", eight_seconds, 1e-6 / 2147483648.0, 14));
    {
        Spec spec = of_kind("noise_alpha_tempo_constrain", "noise", eight_seconds, 1.0 / 65536.0, 15);
        spec.has_alpha = true;
        spec.alpha = 0.3f;
        spec.has_tempo = true;
        spec.tempo = 200;
        spec.has_constrain = true;
        spec.constrain = 1;
        cases.push_back(spec);
    }
    cases.push_back(of_kind("silence_8s", "silence", eight_seconds, 0));
    cases.push_back(of_kind("silence_30s", "silence", 44100 * 30, 0));
    cases.push_back(of_kind("dc_10s", "dc", 44100 * 10, 1000));
    {
        Spec spec = of_kind("impulse_10s", "impulse", 44100 * 10, 30000);
        spec.position = 44100 * 3;
        cases.push_back(spec);
    }
    {
        Spec spec = of_kind("tone_440_10s", "tone", 44100 * 10, 20000);
        spec.frequency = 440;
        cases.push_back(spec);
    }

    // Very short inputs, then the lengths on each side of "no tempo frames", "one", and "two".
    for (int length : {0, 1, 100, 511, 512, 513, 1023, 1024, 1025, 1535, 1536, 1537, 2048, 2560, 3072, 3584, 5120, 10240}) {
        cases.push_back(of_kind("short_noise_" + std::to_string(length), "noise", length, 1.0 / 65536.0, 21));
    }
    for (int df_len : {512, 513, 640, 641, 642, 768, 769}) {
        cases.push_back(of_kind("short_noise_df" + std::to_string(df_len), "noise", 1024 + df_len * 512, 1.0 / 65536.0, 22));
    }
    for (int length : {0, 1024, 3072, 1024 + 641 * 512}) {
        cases.push_back(of_kind("short_silence_" + std::to_string(length), "silence", length, 0));
    }

    // Other sample rates. These reach the radix 3, 5 and generic butterflies and every decimator arrangement.
    for (int rate : {22050, 48000, 8000, 11025, 12000, 16000, 24000, 30000, 32000, 5000}) {
        cases.push_back(click("rate_" + std::to_string(rate) + "_click", rate, 4, {{16, 120.0}}, 4));
    }
    cases.push_back(click("rate_96000_click", 96000, 4, {{8, 120.0}}, 4));
    cases.push_back(click("rate_192000_click", 192000, 4, {{4, 120.0}}, 4));
    cases.push_back(click("rate_384000_click", 384000, 4, {{2, 120.0}}, 4));
    {
        Spec spec = of_kind("rate_48000_noise", "noise", 48000 * 8, 1.0 / 65536.0, 31);
        spec.rate = 48000;
        cases.push_back(spec);
    }
    {
        Spec spec = of_kind("rate_22050_noise", "noise", 22050 * 8, 1.0 / 65536.0, 32);
        spec.rate = 22050;
        cases.push_back(spec);
    }
    {
        Spec spec = of_kind("rate_22050_silence", "silence", 22050 * 8, 0);
        spec.rate = 22050;
        cases.push_back(spec);
    }
    {
        Spec spec = click("rate_48000_bpb3_constrain", 48000, 3, {{8, 100.0}}, 3);
        spec.has_tempo = true;
        spec.tempo = 100;
        spec.has_constrain = true;
        spec.constrain = 1;
        cases.push_back(spec);
    }

    // The other driver paths.
    {
        Spec spec = click("preview_long_track", 44100, 4, {{16, 120.0}}, 4);
        spec.driver = "preview";
        cases.push_back(spec);
    }
    {
        Spec spec = click("preview_short_track", 44100, 4, {{2, 120.0}}, 4);
        spec.driver = "preview";
        cases.push_back(spec);
    }
    {
        Spec spec = click("reset_rerun", 44100, 4, {{16, 120.0}}, 4);
        spec.driver = "reset_rerun";
        cases.push_back(spec);
    }
    {
        Spec spec = click("remaining_twice", 44100, 4, {{16, 120.0}}, 4);
        spec.driver = "remaining_twice";
        cases.push_back(spec);
    }

    return cases;
}

static const char *value_after(int argc, char **argv, int &i)
{
    if (i + 1 >= argc) {
        fprintf(stderr, "%s needs a value\n", argv[i]);
        exit(1);
    }
    return argv[++i];
}

int main(int argc, char **argv)
{
    if (argc >= 3 && strcmp(argv[1], "--synthetic") == 0) {
        string full_dir = argc >= 5 && strcmp(argv[3], "--full") == 0 ? argv[4] : "";
        vector<Spec> cases = synthetic_cases();
        for (const Spec &spec : cases) {
            run_case(spec, generate(spec), argv[2], full_dir, false);
        }
        printf("%zu synthetic cases written to %s\n", cases.size(), argv[2]);
        return 0;
    }

    Spec spec;
    spec.kind = "raw";
    string raw_path;
    string out_dir;
    string full_dir;
    bool time = false;
    for (int i = 1; i < argc; i++) {
        string arg = argv[i];
        if (arg == "--raw") raw_path = value_after(argc, argv, i);
        else if (arg == "--rate") spec.rate = atoi(value_after(argc, argv, i));
        else if (arg == "--name") spec.name = value_after(argc, argv, i);
        else if (arg == "--out") out_dir = value_after(argc, argv, i);
        else if (arg == "--driver") spec.driver = value_after(argc, argv, i);
        else if (arg == "--bpb") spec.bpb = (float)atof(value_after(argc, argv, i));
        else if (arg == "--alpha") { spec.has_alpha = true; spec.alpha = (float)atof(value_after(argc, argv, i)); }
        else if (arg == "--tempo") { spec.has_tempo = true; spec.tempo = (float)atof(value_after(argc, argv, i)); }
        else if (arg == "--constrain") { spec.has_constrain = true; spec.constrain = (float)atof(value_after(argc, argv, i)); }
        else if (arg == "--note") spec.note = value_after(argc, argv, i);
        else if (arg == "--full") full_dir = value_after(argc, argv, i);
        else if (arg == "--time") time = true;
        else {
            fprintf(stderr, "unknown argument %s\n", arg.c_str());
            return 1;
        }
    }
    if (raw_path.empty() || out_dir.empty() || spec.name.empty()) {
        fprintf(stderr, "usage: see the top of qm_golden.cpp\n");
        return 1;
    }

    FILE *f = fopen(raw_path.c_str(), "rb");
    if (!f) {
        fprintf(stderr, "cannot read %s\n", raw_path.c_str());
        return 1;
    }
    fseek(f, 0, SEEK_END);
    long bytes = ftell(f);
    fseek(f, 0, SEEK_SET);
    vector<float> input(bytes / sizeof(float));
    if (fread(input.data(), sizeof(float), input.size(), f) != input.size()) {
        fprintf(stderr, "short read from %s\n", raw_path.c_str());
        return 1;
    }
    fclose(f);

    run_case(spec, input, out_dir, full_dir, time);
    return 0;
}
