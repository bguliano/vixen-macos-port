// A System.Random that produces exactly what the Rust port's `Rand` produces for the same seed, so effects that use random
// numbers can still be compared frame by frame when both runtimes consume the stream in the same order.
//
// `Rand::default()` is `fastrand::Rng::new()`, which forks from the thread-local generator. The Rust test calls
// `fastrand::seed(seed)` first, so `forks` here says how many `Rand::default()` calls a render makes (the last one wins).
// This file is compiled into both the generator and the extras module.

namespace VixenGolden;

public sealed class GoldenRandom : Random
{
	private const ulong WyConst0 = 0x2d35_8dcc_aa6c_78a5;
	private const ulong WyConst1 = 0x8bb8_4b93_962e_acc9;
	private ulong _state;

	/// <summary>How many numbers have been drawn, which tells an effect that uses random numbers from one that does not.</summary>
	public long Draws { get; private set; }

	public GoldenRandom(ulong seed, int forks)
	{
		var threadLocal = seed;
		for (var i = 0; i < forks; i++) _state = GenU64(ref threadLocal);
	}

	private static ulong GenU64(ref ulong s)
	{
		s = unchecked(s + WyConst0);
		var t = (UInt128)s * (s ^ WyConst1);
		return unchecked((ulong)t ^ (ulong)(t >> 64));
	}

	private uint GenModU32(uint n)
	{
		Draws++;
		var r = (uint)GenU64(ref _state);
		var hi = (uint)(((ulong)r * n) >> 32);
		var lo = unchecked(r * n);
		if (lo < n)
		{
			var t = unchecked(0u - n) % n;
			while (lo < t)
			{
				r = (uint)GenU64(ref _state);
				hi = (uint)(((ulong)r * n) >> 32);
				lo = unchecked(r * n);
			}
		}
		return hi;
	}

	/// <summary><c>Rng::i32(low..high)</c>.</summary>
	private int Range(int low, int high) => unchecked(low + (int)GenModU32((uint)(high - 1 - low) + 1));

	/// <summary><c>Rand::next</c>.</summary>
	public override int Next() => Range(0, int.MaxValue);

	/// <summary><c>Rand::next_max</c>.</summary>
	public override int Next(int max) => max <= 0 ? 0 : Range(0, max);

	/// <summary><c>Rand::next_range</c>.</summary>
	public override int Next(int min, int max) => max <= min ? min : Range(min, max);

	/// <summary><c>Rand::next_double</c>.</summary>
	public override double NextDouble()
	{
		const double mul = 1.0 / (1UL << 63);
		Draws++;
		while (true)
		{
			var x = (GenU64(ref _state) >> 1) * mul;
			if (x < 1.0) return x;
		}
	}

	protected override double Sample() => NextDouble();
}

/// <summary>Where the copies of effects compiled with their random source replaced find it.</summary>
public static class GoldenRandomHook
{
	public static Random Current;
}
