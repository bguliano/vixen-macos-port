using VixenModules.Media.Audio;
using Common.AudioPlayer.SampleProvider;
using NAudio.Wave;
var bytes = File.ReadAllBytes(args[0]);
var decoded = new float[bytes.Length / 4];
Buffer.BlockCopy(bytes, 0, decoded, 0, bytes.Length);
var expectedBytes = File.ReadAllBytes(args[1]);
var expected = new double[expectedBytes.Length / 8];
Buffer.BlockCopy(expectedBytes, 0, expected, 0, expectedBytes.Length);
var audio = new Audio(decoded);
var actual = audio.GetMonoSamples((int)(44100.0 * 116.807 * 2), (int)(44100.0 * 3.106 * 2));
if (!actual.SequenceEqual(expected)) throw new Exception("Original NAudio mono input differs from Rust");
Console.Error.WriteLine($"PASS: {actual.Length} original NAudio / Rust mono samples identical");
var helper = new AudioUtilities {
    StartTime = TimeSpan.FromMilliseconds(116807), TimeSpan = TimeSpan.FromMilliseconds(3106),
    AttackTime = 50, DecayTime = 1500, Normalize = false, Gain = 0,
    LowPass = false, HighPass = true, HighPassFreq = 500,
};
helper.ReloadAudio(audio);
for (var ms = 0; ms < 3106; ms += 30)
    Console.WriteLine($"{ms},{helper.ClosestSample(ms)},{helper.VolumeAtTime(ms):R}");
namespace NLog {
    public class Logger { }
    public static class LogManager { public static Logger GetCurrentClassLogger() => new(); }
}
namespace VixenModules.Media.Audio {
    // Only the decoded stereo host is synthetic. The original GetMonoSamples,
    // cached/mono providers and actual pinned NAudio DLL perform the mix.
    public sealed partial class Audio(float[] decoded) {
        private readonly CachedSampleSource _cachedAudioData = new(44100, 2, decoded);
        public int Channels => 2;
        public float Frequency => 44100;
        public TimeSpan Position => TimeSpan.Zero;
        public void LoadMedia(TimeSpan start) { }
        public void Start() { }
        public void Stop() { }
    }
}
namespace Common.AudioPlayer.SampleProvider {
    public sealed class CachedSampleSource(int rate, int channels, float[] decoded) {
        public float[] AudioData { get; } = decoded;
        public WaveFormat WaveFormat { get; } = WaveFormat.CreateIeeeFloatWaveFormat(rate, channels);
    }
}
