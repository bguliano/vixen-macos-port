// Class-level golden data for crates/vixen-analysis: calls the upstream classes directly, on inputs chosen to reach
// the decisions a whole-track run rarely depends on, and writes a hash of each result.
//
//   qm-golden-units OUT.json
//
// The whole-track goldens can't see some details, because a beat position hardly ever depends on them: the C round()
// in the beat search, say, or the truncating integer division behind the tempo weighting. Mutating those made no
// golden test fail, so these checks exist. crates/vixen-analysis/tests/golden_units.rs runs the same list.
//
// Private members are opened up here, in this file only, so the upstream sources stay exactly as they are. Access
// control doesn't affect the compiled symbols.

#include <algorithm>
#include <cinttypes>
#include <cmath>
#include <complex>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <iostream>
#include <map>
#include <string>
#include <vector>

#define private public
#define protected public
#include "dsp/tempotracking/DownBeat.h"
#include "dsp/tempotracking/TempoTrackV2.h"
#undef private
#undef protected

#include "base/Window.h"
#include "dsp/onsets/DetectionFunction.h"
#include "dsp/phasevocoder/PhaseVocoder.h"
#include "dsp/rateconversion/Decimator.h"
#include "dsp/transforms/FFT.h"
#include "maths/MathUtilities.h"
#include "vamp-sdk/RealTime.h"

using std::string;
using std::vector;

static FILE *out_file;
static bool first_check = true;

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

static void record(const string &name, const vector<double> &values)
{
    fprintf(out_file, "%s\n    \"%s\": {\"len\": %zu, \"fnv1a64\": \"0x%016" PRIx64 "\"}", first_check ? "" : ",",
            name.c_str(), values.size(), fnv1a64(values.data(), values.size() * sizeof(double)));
    first_check = false;
}

static uint32_t xorshift32(uint32_t &x)
{
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    return x;
}

// Values in [-scale, scale).
static vector<double> noise(uint32_t seed, int n, double scale)
{
    vector<double> out(n);
    uint32_t state = seed;
    for (int i = 0; i < n; i++) {
        out[i] = (double)(int32_t)xorshift32(state) / 2147483648.0 * scale;
    }
    return out;
}

static vector<double> magnitudes(uint32_t seed, int n, double scale)
{
    vector<double> out = noise(seed, n, scale);
    for (double &v : out) v = fabs(v);
    return out;
}

// Most values are small and a few are enormous, so a transition weight near zero can still win the beat search.
static vector<double> heavy_tailed(uint32_t seed, int n)
{
    vector<double> out = noise(seed, n, 12.0);
    for (double &v : out) v = exp(v);
    return out;
}

static vector<float> to_float(const vector<double> &in)
{
    return vector<float>(in.begin(), in.end());
}

static void append(vector<double> &to, const vector<double> &from)
{
    to.insert(to.end(), from.begin(), from.end());
}

static void math_checks()
{
    vector<double> xs = noise(101, 2000, 100.0);
    vector<double> ys = noise(102, 2000, 10.0);
    vector<double> out;
    for (size_t i = 0; i < xs.size(); i++) out.push_back(MathUtilities::mod(xs[i], ys[i]));
    record("math_mod", out);

    out.clear();
    for (double x : xs) out.push_back(MathUtilities::princarg(x));
    record("math_princarg", out);

    for (int n : {1, 2, 3, 8, 15, 16, 17, 64, 128, 512, 2048}) {
        vector<double> data = noise(200 + n, n, 1000.0);
        MathUtilities::adaptiveThreshold(data);
        record("adaptive_threshold_" + std::to_string(n), data);
    }

    out.clear();
    for (int n : {1, 2, 3, 10, 100, 1000}) {
        out.push_back(MathUtilities::getMax(noise(300 + n, n, 1.0)));
        out.push_back(MathUtilities::getMax(vector<double>(n, 0.0)));
    }
    record("math_get_max", out);

    out.clear();
    for (int x = -4; x < 5000; x++) out.push_back(MathUtilities::nextPowerOfTwo(x));
    record("math_next_power_of_two", out);
}

static void window_checks()
{
    for (int n : {1, 2, 3, 512, 1024, 1114}) {
        Window<double> window(HanningWindow, n);
        vector<double> in = noise(400 + n, n, 1000.0);
        vector<double> dst(n);
        window.cut(in.data(), dst.data());
        record("window_hanning_" + std::to_string(n), dst);
    }
}

static void fft_checks()
{
    // Sizes that reach every radix and the generic butterfly at different depths.
    for (int n : {2,   4,   6,   8,   10,  12,  14,  16,  18,  20,   22,   24,   26,   28,   30,  32, 34, 36, 40, 46,
                  50,  56,  64,  70,  74,  90,  92,  96,  128, 184,  256,  348,  370,  512,  556, 742, 1024, 1114, 2048,
                  4096, 8192}) {
        FFTReal fft(n);
        vector<double> in = noise(500 + n, n, 30000.0);
        vector<double> re(n), im(n);
        fft.forward(in.data(), re.data(), im.data());
        append(re, im);
        record("fft_real_" + std::to_string(n), re);
    }
    {
        // Signs of zero decide atan2 on silence.
        FFTReal fft(1024);
        vector<double> in(1024, 0.0), re(1024), im(1024);
        fft.forward(in.data(), re.data(), im.data());
        append(re, im);
        record("fft_real_silence", re);
    }
}

static void phase_vocoder_checks()
{
    for (int n : {16, 512, 1024, 1114}) {
        PhaseVocoder pv(n, n / 2);
        vector<double> mag(n / 2 + 1), theta(n / 2 + 1), unwrapped(n / 2 + 1), out;
        for (int frame = 0; frame < 3; frame++) {
            vector<double> in = noise(600 + n + frame, n, 30000.0);
            pv.processTimeDomain(in.data(), mag.data(), theta.data(), unwrapped.data());
            append(out, mag);
            append(out, theta);
        }
        record("phase_vocoder_" + std::to_string(n), out);
    }
}

static void detection_function_checks()
{
    for (int n : {16, 512, 1024, 1114}) {
        DFConfig config;
        config.DFType = DF_COMPLEXSD;
        config.stepSize = n / 2;
        config.frameLength = n;
        config.dbRise = 3;
        config.adaptiveWhitening = false;
        config.whiteningRelaxCoeff = -1;
        config.whiteningFloor = -1;
        DetectionFunction df(config);
        vector<double> out;
        for (int frame = 0; frame < 8; frame++) {
            vector<double> in = noise(700 + n + frame, n, 2147483648.0);
            out.push_back(df.processTimeDomain(in.data()));
        }
        vector<double> silence(n, 0.0);
        out.push_back(df.processTimeDomain(silence.data()));
        out.push_back(df.processTimeDomain(silence.data()));
        out.push_back(df.processTimeDomain(silence.data()));
        record("detection_function_" + std::to_string(n), out);
    }
}

static void decimator_checks()
{
    for (int factor : {1, 2, 3, 4, 8, 16}) {
        const int in_length = 512;
        Decimator decimator(in_length, factor);
        vector<double> out;
        for (int block = 0; block < 4; block++) {
            vector<float> in = to_float(noise(800 + factor + block, in_length, 2147483648.0));
            vector<float> dst(in_length / factor);
            decimator.process(in.data(), dst.data());
            out.insert(out.end(), dst.begin(), dst.end());
        }
        record("decimator_" + std::to_string(factor), out);
    }
}

static vector<double> test_weights()
{
    vector<double> wv(128);
    double r = 43.0;
    for (int i = 0; i < 128; i++) {
        double d = i;
        wv[i] = (d / (r * r)) * exp((-1.0 * (d * d)) / (2.0 * (r * r)));
    }
    return wv;
}

static void tempo_checks()
{
    TempoTrackV2 tt(44100.f, 512);
    vector<double> wv = test_weights();

    for (int seed = 1; seed <= 5; seed++) {
        vector<double> frame = magnitudes(900 + seed, 512, seed * seed * 100.0);
        vector<double> rcf(128);
        tt.get_rcf(frame, wv, rcf);
        record("rcf_" + std::to_string(seed), rcf);
    }
    {
        vector<double> frame(512, 0.0), rcf(128);
        tt.get_rcf(frame, wv, rcf);
        record("rcf_silence", rcf);
    }

    for (int t_len : {0, 1, 2, 3, 8, 32}) {
        vector<vector<double>> rcfmat;
        for (int t = 0; t < t_len; t++) {
            vector<double> column = magnitudes(1000 + t_len * 100 + t, 128, 1.0);
            double sum = 0;
            for (double v : column) sum += v;
            for (double &v : column) v /= sum;
            rcfmat.push_back(column);
        }
        vector<double> period(t_len * 128 + 37, 0.0), tempi;
        tt.viterbi_decode(rcfmat, wv, period, tempi);
        append(period, tempi);
        record("viterbi_" + std::to_string(t_len), period);
    }

    // On silence the chosen period follows the weighting alone, and it steps from one integer to the next at a precise
    // tempo. Sweeping tempo finely across a step pins the weighting formula, truncations included.
    vector<double> silent_df(512 + 128 * 3 + 1, 0.0);
    for (int constrain = 0; constrain <= 1; constrain++) {
        vector<double> out;
        for (int center : {30, 43, 60}) {
            for (int k = -60; k <= 60; k++) {
                double tempo = (5167.0 / (center + 0.5)) * (1.0 + k * 0.0001);
                vector<double> period(silent_df.size(), 0.0), tempi;
                tt.calculateBeatPeriod(silent_df, period, tempi, tempo, constrain != 0);
                out.push_back(period[0]);
                out.push_back(period[period.size() - 1]);
            }
        }
        record(constrain ? "beat_period_sweep_gaussian" : "beat_period_sweep_rayleigh", out);
    }

    // The default TempoTrackV2 rate doesn't matter for beats but does for tempi; check another.
    {
        TempoTrackV2 other(48000.f, 557);
        vector<vector<double>> rcfmat;
        for (int t = 0; t < 3; t++) {
            rcfmat.push_back(magnitudes(1200 + t, 128, 1.0));
        }
        vector<double> period(3 * 128, 0.0), tempi;
        other.viterbi_decode(rcfmat, wv, period, tempi);
        record("tempi_48000", tempi);
    }

    const double alphas[] = {0.1, 0.5, 0.9, 0.99};
    const double tightnesses[] = {1.0, 4.0, 10.0};
    const double periods[] = {0, 1, 2, 3, 4, 5, 43, 44, 45, 86, 127};
    for (int seed = 1; seed <= 12; seed++) {
        const int df_len = 2000;
        vector<double> df = heavy_tailed(1300 + seed, df_len);
        vector<double> period(df_len);
        uint32_t state = 1400 + seed;
        double current = 0;
        for (int i = 0; i < df_len; i++) {
            if (i % 128 == 0) current = periods[xorshift32(state) % 11];
            period[i] = current;
        }
        vector<double> beats;
        tt.calculateBeats(df, period, beats, alphas[seed % 4], tightnesses[seed % 3]);
        record("calculate_beats_" + std::to_string(seed), beats);
    }
    for (int p : {1, 2, 3, 5, 7, 43, 44, 45, 127}) {
        vector<double> df = heavy_tailed(1500 + p, 3000);
        vector<double> period(df.size(), (double)p), beats;
        tt.calculateBeats(df, period, beats, 0.9, 4.0);
        record("calculate_beats_constant_" + std::to_string(p), beats);
    }

    // Two equal spikes, then the beat before the last is whichever earlier frame the search range can reach. Stepping
    // the distance across the whole range pins both ends of it, and so the rounding of each end.
    for (int p : {1, 2, 3, 5, 9, 45}) {
        vector<double> out;
        for (int distance = 1; distance <= 2 * p + 1; distance++) {
            const int df_len = 300;
            vector<double> df(df_len, 0.0);
            df[df_len - 1] = 1.0;
            df[df_len - 1 - distance] = 1.0;
            vector<double> period(df_len, (double)p), beats;
            tt.calculateBeats(df, period, beats, 0.9, 4.0);
            out.push_back(-1.0);
            append(out, beats);
        }
        record("calculate_beats_spikes_" + std::to_string(p), out);
    }
}

static void down_beat_checks()
{
    for (int size : {0, 2, 4, 8, 16, 512, 2048, 4096, 8192}) {
        DownBeat downbeat(44100.f, 16, 512);
        vector<double> out;
        for (int seed = 1; seed <= 4; seed++) {
            vector<double> a = magnitudes(1600 + size + seed, size, 100.0);
            vector<double> b = magnitudes(1700 + size + seed, size, 100.0);
            out.push_back(downbeat.measureSpecDiff(a, b));
        }
        out.push_back(downbeat.measureSpecDiff(vector<double>(size, 0.0), vector<double>(size, 0.0)));
        record("spec_diff_" + std::to_string(size), out);
    }

    struct Arrangement {
        float rate;
        int factor;
        int increment;
    };
    const Arrangement arrangements[] = {
        {44100.f, 16, 512}, {48000.f, 16, 557}, {22050.f, 8, 256}, {8000.f, 2, 92}, {5000.f, 1, 58}, {1.f, 1, 1},
    };
    int index = 0;
    for (const Arrangement &a : arrangements) {
        for (int bpb : {0, 1, 3, 4, 5, 16}) {
            DownBeat downbeat(a.rate, a.factor, a.increment);
            downbeat.setBeatsPerBar(bpb);

            vector<float> audio = to_float(noise(1800 + index, 20000, 2147483648.0));

            // Gaps from one df step up to well past the beat frame, then beats past the end of the audio and a step
            // backwards, which the extent checks have to absorb.
            vector<double> beats;
            uint32_t state = 1900 + index;
            double position = 0;
            int count = 40 + (int)(xorshift32(state) % 40);
            for (int i = 0; i < count; i++) {
                position += 1 + (xorshift32(state) % 300);
                beats.push_back(position);
            }
            beats.push_back(position + 100000);
            beats.push_back(position + 100001);
            beats.push_back(position);

            vector<int> downbeats;
            downbeat.findDownBeats(audio.data(), audio.size(), beats, downbeats);

            vector<double> beatsd;
            downbeat.getBeatSD(beatsd);
            vector<double> out(downbeats.begin(), downbeats.end());
            append(out, beatsd);
            record("find_down_beats_" + std::to_string(index), out);
            index++;
        }
    }
    {
        // Few beats, and none of the spectral differences to choose by.
        for (int beat_count : {0, 1, 2, 3}) {
            DownBeat downbeat(44100.f, 16, 512);
            downbeat.setBeatsPerBar(4);
            vector<float> audio = to_float(noise(2000, 4000, 1.0));
            vector<double> beats;
            for (int i = 0; i < beat_count; i++) beats.push_back(i * 40.0);
            vector<int> downbeats;
            downbeat.findDownBeats(audio.data(), audio.size(), beats, downbeats);
            vector<double> beatsd;
            downbeat.getBeatSD(beatsd);
            vector<double> out(downbeats.begin(), downbeats.end());
            append(out, beatsd);
            record("find_down_beats_few_" + std::to_string(beat_count), out);
        }
    }
    {
        vector<double> out;
        for (auto rate_factor : vector<std::pair<float, int>>{{44100.f, 16}, {48000.f, 16}, {22050.f, 8}, {8000.f, 2}, {96000.f, 32}, {5000.f, 1}, {1.f, 1}}) {
            DownBeat downbeat(rate_factor.first, rate_factor.second, 64);
            out.push_back((double)downbeat.m_beatframesize);
        }
        record("beat_frame_sizes", out);
    }
}

static void real_time_checks()
{
    vector<double> out;
    for (unsigned rate : {1u, 8000u, 22050u, 44100u, 48000u, 96000u, 192000u}) {
        for (long frame : {0L, 1L, 2L, 511L, 512L, 1024L, 22050L, 44099L, 44100L, 44101L, 1000000L, 123456789L, -1L, -44100L, -66150L}) {
            Vamp::RealTime t = Vamp::RealTime::frame2RealTime(frame, rate);
            out.push_back(t.sec);
            out.push_back(t.nsec);
        }
    }
    record("real_time_frames", out);

    out.clear();
    for (int s : {-3, -1, 0, 1, 2}) {
        for (int n : {-1500000000, -1000000000, -999999999, -1, 0, 1, 999999999, 1000000000, 1500000000}) {
            Vamp::RealTime t(s, n);
            out.push_back(t.sec);
            out.push_back(t.nsec);
            Vamp::RealTime sum = t + Vamp::RealTime(1, 600000000);
            out.push_back(sum.sec);
            out.push_back(sum.nsec);
        }
    }
    record("real_time_normalise_and_add", out);
}

int main(int argc, char **argv)
{
    if (argc != 2) {
        fprintf(stderr, "usage: qm-golden-units OUT.json\n");
        return 1;
    }
    out_file = fopen(argv[1], "w");
    if (!out_file) {
        fprintf(stderr, "cannot write %s\n", argv[1]);
        return 1;
    }

    fprintf(out_file, "{\n  \"checks\": {");
    math_checks();
    window_checks();
    fft_checks();
    phase_vocoder_checks();
    detection_function_checks();
    decimator_checks();
    tempo_checks();
    down_beat_checks();
    real_time_checks();
    fprintf(out_file, "\n  }\n}\n");
    fclose(out_file);
    return 0;
}
