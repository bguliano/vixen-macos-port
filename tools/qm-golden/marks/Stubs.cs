// Stand-ins for what BeatsAndBars.cs needs and this Mac can't load: the C++/CLI plugin wrapper, the audio module and the
// WinForms dialogs. The plugin hands back scripted features, because the features are compared separately, and records
// what it was given. Nothing else changes: the C# under test is upstream's own source file.

using System.Collections.Generic;

public class ManagedFeature
{
	public bool hasTimestamp;
	public ManagedRealtime timestamp;
	public bool hasDuration;
	public ManagedRealtime duration;
	public ICollection<float> values;
	public string label;
}

// WrapperFiles/ManagedRealtime.cpp and RealTime.cpp, for the two members BeatsAndBars.cs calls.
public class ManagedRealtime
{
	public int Sec;
	public int Nsec;

	public ManagedRealtime(int sec, int nsec)
	{
		Sec = sec;
		Nsec = nsec;
	}

	public double totalMilliseconds()
	{
		return (Sec * 1000) + (Nsec / 1000000);
	}

	public static ManagedRealtime frame2RealTime(long frame, uint sampleRate)
	{
		int sec = (int)(frame / sampleRate);
		frame -= sec * (long)sampleRate;
		int nsec = (int)(((double)frame / sampleRate) * 1000000000 + 0.5);
		return new ManagedRealtime(sec, nsec);
	}
}

namespace QMLibrary
{
	public abstract class ManagedPlugin : IDisposable
	{
		public abstract int GetPreferredStepSize();
		public abstract int GetPreferredBlockSize();
		public abstract void SetParameter(string name, float value);
		public abstract bool Initialise(uint channels, uint stepSize, uint blockSize);
		public abstract IDictionary<int, ICollection<ManagedFeature>> Process(float[] inputBuffer, ManagedRealtime timestamp);
		public abstract IDictionary<int, ICollection<ManagedFeature>> GetRemainingFeatures();
		public void Dispose() { }
	}

	public sealed class QMBarBeatTrack : ManagedPlugin
	{
		public sealed class Recording
		{
			public int Blocks;
			public ulong Hash = 0xcbf29ce484222325UL;
			public readonly Dictionary<string, float> Parameters = new();
		}

		public static readonly Queue<IDictionary<int, ICollection<ManagedFeature>>> ScriptedFeatures = new();
		public static readonly List<Recording> Recordings = new();

		private readonly float _rate;
		private readonly Recording _recording = new();

		public QMBarBeatTrack(float inputSampleRate)
		{
			_rate = inputSampleRate;
			Recordings.Add(_recording);
		}

		public override int GetPreferredStepSize()
		{
			int step = (int)(_rate * 0.01161f + 0.0001);
			return step < 1 ? 1 : step;
		}

		public override int GetPreferredBlockSize() => GetPreferredStepSize() * 2;

		public override void SetParameter(string name, float value) => _recording.Parameters[name] = value;

		public override bool Initialise(uint channels, uint stepSize, uint blockSize) => true;

		public override IDictionary<int, ICollection<ManagedFeature>> Process(float[] inputBuffer, ManagedRealtime timestamp)
		{
			_recording.Blocks++;
			foreach (var value in inputBuffer)
			{
				Mix(BitConverter.GetBytes(value));
			}
			Mix(BitConverter.GetBytes(timestamp.Sec));
			Mix(BitConverter.GetBytes(timestamp.Nsec));
			return new Dictionary<int, ICollection<ManagedFeature>>();
		}

		public override IDictionary<int, ICollection<ManagedFeature>> GetRemainingFeatures() => ScriptedFeatures.Dequeue();

		private void Mix(byte[] bytes)
		{
			foreach (var b in bytes)
			{
				_recording.Hash ^= b;
				_recording.Hash *= 0x100000001b3UL;
			}
		}
	}
}

namespace VixenModules.Media.Audio
{
	public class Audio
	{
		public float Frequency;
		public int Channels;
		public int BytesPerSample;
		public byte[] Raw;

		public byte[] GetRawAudioSamples() => Raw;
	}
}

namespace VixenModules.Analysis.BeatsAndBars
{
	public enum DialogResult
	{
		OK,
		Cancel
	}

	public sealed class BeatsAndBarsDialog
	{
		public static BeatBarSettingsData NextSettings;
		public static BeatBarPreviewData LastPreview;

		public BeatsAndBarsDialog(VixenModules.Media.Audio.Audio audio) { }

		public BeatBarSettingsData Settings => NextSettings;

		public BeatBarPreviewData PreviewData
		{
			get => LastPreview;
			set => LastPreview = value;
		}

		public DialogResult ShowDialog() => DialogResult.OK;
	}

	public sealed class BeatsAndBarsProgress : IDisposable
	{
		private readonly ManualResetEventSlim _closed = new(false);

		public event EventHandler Shown;

		public void ShowDialog()
		{
			Shown?.Invoke(this, EventArgs.Empty);
			_closed.Wait();
		}

		public void UpdateProgress(int value) { }
		public void SetFinalizing() { }
		public void Close() => _closed.Set();
		public void Dispose() => _closed.Dispose();
	}
}
