// Twinkle, Candle and Wave draw from random sources the runtime cannot redirect: a static generator and a `new Random()`.
// These shims let upstream's unmodified sources be compiled against a redirectable source. Without a hook installed they
// behave as the originals do, apart from Twinkle's uniform source.

using VixenGolden;
using VixenModules.Effect.Effect;

namespace VixenModules.Effect.Twinkle
{
	/// <summary>Replaces upstream's RandomGenerator. GetExponential keeps its formula; only the uniform source changes.</summary>
	public static class RandomGenerator
	{
		public static double GetExponential(double mean)
		{
			if (mean <= 0.0) throw new ArgumentOutOfRangeException(nameof(mean));
			var source = GoldenRandomHook.Current ?? ThreadSafeRandom.Instance;
			return mean * -Math.Log(1.0 - source.NextDouble());
		}
	}
}

namespace VixenModules.Effect.Wave
{
	/// <summary>Takes the place of System.Random inside this namespace, so `new Random()` in the effect lands here.</summary>
	public sealed class Random
	{
		private readonly System.Random _r = GoldenRandomHook.Current ?? new System.Random();
		public double NextDouble() => _r.NextDouble();
		public int Next() => _r.Next();
		public int Next(int max) => _r.Next(max);
		public int Next(int min, int max) => _r.Next(min, max);
	}
}

namespace VixenModules.Effect.Candle
{
	/// <summary>Takes the place of System.Random inside this namespace, so `new Random()` in _PreRender lands here.</summary>
	public sealed class Random
	{
		private readonly System.Random _r = GoldenRandomHook.Current ?? new System.Random();
		public double NextDouble() => _r.NextDouble();
		public int Next(int min, int max) => _r.Next(min, max);
	}
}
