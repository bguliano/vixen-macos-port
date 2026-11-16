// Synthetic decoded-data/media hosts only. prepare.py supplies the original
// Audio.GetMonoSamples; unchanged Vixen cached/mono providers call the actual
// pinned NAudio.Core 2.3.0 DLL. No decoder or playback device is constructed.

using Common.AudioPlayer.SampleProvider;
using NAudio.Wave;
using Vixen.Module;
using Vixen.Module.Media;
using Vixen.Module.Timing;
using VixenGolden;

namespace VixenModules.Media.Audio
{
	public sealed partial class Audio : MediaModuleInstanceBase
	{
		private readonly CachedSampleSource _cachedAudioData;

		public Audio(float frequency, int channels, float[] samples)
		{
			Frequency = frequency;
			Channels = channels;
			_cachedAudioData = new CachedSampleSource((int)frequency, channels, samples);
		}

		public float Frequency { get; }

		public int Channels { get; }

		public TimeSpan Position => TimeSpan.Zero;

		public override string MediaFilePath { get; set; }
		public override void LoadMedia(TimeSpan startTime) { }
		public override ITiming TimingSource => null;
		public override void Start() { }
		public override void Stop() { }
		public override void Pause() { }
		public override void Resume() { }
		public override IModuleDataModel ModuleData { get; set; }
	}
}

namespace VixenGolden
{
	/// <summary>The signals cases can name, as the generator finds them by reflection.</summary>
	public static class GoldenAudio
	{
		private static readonly Dictionary<string, VixenModules.Media.Audio.Audio> Known = new();

		public static IMediaModuleInstance Named(string name)
		{
			lock (Known)
			{
				if (Known.TryGetValue(name, out var audio)) return audio;
				var (rate, channels, seconds) = AudioSignal.Parameters(name);
				audio = new VixenModules.Media.Audio.Audio(rate, channels, AudioSignal.Make(rate, channels, seconds));
				Known[name] = audio;
				return audio;
			}
		}
	}
}

namespace Common.AudioPlayer.SampleProvider {
    // Supplies already-decoded inputs; the original CachedSampleSource constructor
    // opens an audio decoder, which is outside the effect helper oracle.
    public sealed class CachedSampleSource {
        public float[] AudioData { get; }
        public WaveFormat WaveFormat { get; }
        public CachedSampleSource(int rate, int channels, float[] samples) {
            AudioData = samples;
            WaveFormat = WaveFormat.CreateIeeeFloatWaveFormat(rate, channels);
        }
    }
}
