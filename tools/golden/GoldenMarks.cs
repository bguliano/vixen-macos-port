// Minimal IMark and IMarkCollection for the cases that use marks. VixenModules.App.Marks is not in the compiled Vixen
// assemblies; MarksInclusiveOfTime and EnsureOrder follow its MarkCollection.cs.

using System.Collections.ObjectModel;
using System.ComponentModel;
using Vixen.Marks;

namespace VixenGolden;

public sealed class GoldenMark(TimeSpan start, TimeSpan duration, string text) : IMark
{
	public TimeSpan StartTime { get; set; } = start;
	public TimeSpan EndTime => StartTime + Duration;
	public TimeSpan Duration { get; set; } = duration;
	public string Text { get; set; } = text;
	public IMarkCollection Parent { get; set; }
	public int CompareTo(IMark other) => StartTime.CompareTo(other.StartTime);
	public object Clone() => new GoldenMark(StartTime, Duration, Text);
	public event PropertyChangedEventHandler PropertyChanged { add { } remove { } }
}

public sealed class GoldenMarkCollection : IMarkCollection
{
	private readonly ObservableCollection<IMark> _marks = new();

	public GoldenMarkCollection() => Marks = new ReadOnlyObservableCollection<IMark>(_marks);

	public Guid Id { get; set; }
	public string Name { get; set; }
	public bool IsDefault { get; set; }
	public bool IsVisible => true;
	public bool ShowGridLines { get; set; }
	public bool ShowTailGridLines { get; set; }
	public bool ShowMarkBar { get; set; }
	public bool Locked { get; set; }
	public int Level { get; set; } = 1;
	public ReadOnlyObservableCollection<IMark> Marks { get; }
	public IMarkDecorator Decorator { get; set; }
	public MarkCollectionType CollectionType { get; set; } = MarkCollectionType.Generic;
	public Guid LinkedMarkCollectionId { get; set; }

	public void RemoveMark(IMark mark) => _marks.Remove(mark);

	public void RemoveAll(Func<IMark, bool> condition)
	{
		foreach (var m in _marks.Where(condition).ToArray()) _marks.Remove(m);
	}

	public void AddMark(IMark mark)
	{
		mark.Parent = this;
		_marks.Add(mark);
	}

	public void AddMarks(IEnumerable<IMark> marks)
	{
		foreach (var m in marks) AddMark(m);
	}

	public void SwapPlaces(IMark lhs, IMark rhs) => throw new NotSupportedException();

	public void EnsureOrder()
	{
		var sorted = _marks.OrderBy(m => m.StartTime).ToList();
		_marks.Clear();
		foreach (var m in sorted) _marks.Add(m);
	}

	public List<IMark> MarksInclusiveOfTime(TimeSpan start, TimeSpan end)
	{
		var marks = new List<IMark>();
		foreach (var mark in _marks.ToArray())
		{
			if (mark.EndTime < start) continue;
			if (mark.StartTime > end) break;
			marks.Add(mark);
		}
		return marks;
	}

	public void FillGapTimes(IMark mark) => throw new NotSupportedException();
	public object Clone() => throw new NotSupportedException();
	public event PropertyChangedEventHandler PropertyChanged { add { } remove { } }
}
