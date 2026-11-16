// The synthetic audio audio-driven effects read. crates/vixen-effect-registry/tests/golden.rs computes exactly the same
// samples: every step is integer or single precision arithmetic, which both runtimes do to the same bit.
// This file is compiled into both the generator and the extras module.

namespace VixenGolden;

public static class AudioSignal
{
	public static (int rate, int channels, int seconds) Parameters(string name) => name switch
	{
		"mono22" => (22050, 1, 6),
		"stereo44" => (44100, 2, 6),
		_ => throw new ArgumentException(name),
	};

	/// <summary>
	/// Noise under a hit every 300 ms, a quiet passage in the second second, and digital silence for the first half of the
	/// third. Each channel draws its own noise.
	/// </summary>
	public static float[] Make(int rate, int channels, int seconds)
	{
		var samples = new float[rate * seconds * channels];
		var beat = rate * 3 / 10;
		var state = 0x2468ace1u;
		for (var frame = 0; frame < rate * seconds; frame++)
		{
			var envelope = 1f - frame % beat / (float)beat;
			var second = frame / rate;
			var level = second == 1 ? 0.125f : second == 2 && frame % rate < rate / 2 ? 0f : 1f;
			for (var channel = 0; channel < channels; channel++)
			{
				state = unchecked(state * 1664525u + 1013904223u);
				var noise = ((state >> 16) & 0xffff) / 32768f - 1f;
				samples[frame * channels + channel] = noise * envelope * level;
			}
		}
		return samples;
	}
}
