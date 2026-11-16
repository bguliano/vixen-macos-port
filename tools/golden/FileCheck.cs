// Puts sequences through the original Vixen's own loader and saver. Comparing what it writes for a file the Rust port saved with what
// it writes for the original shows whether the original still reads the port's files the same way.

using System.Reflection;
using Vixen.Services;

namespace VixenGolden;

public static class FileCheck
{
	/// <summary>
	/// The module store names each data class's assembly as upstream built it, such as Module.App.Curves. The compiled original keeps
	/// them together in assemblies like Module.App.Native, so without this it can't find them, and loads those modules with no data.
	/// </summary>
	public static Assembly FindMovedAssembly(object sender, ResolveEventArgs args)
	{
		var name = new AssemblyName(args.Name).Name;
		if (!name.StartsWith("Module.", StringComparison.Ordinal)) return null;
		var moved = "VixenModules." + name["Module.".Length..];
		foreach (var assembly in AppDomain.CurrentDomain.GetAssemblies().Where(a => a.GetName().Name.StartsWith("Module.", StringComparison.Ordinal)))
		{
			try
			{
				if (assembly.GetTypes().Any(t => t.Namespace == moved))
					return assembly;
			}
			catch (ReflectionTypeLoadException)
			{
			}
		}
		Unresolved.Add(name);
		return null;
	}

	/// <summary>Assembly names no compiled module stands in for. Their modules' data can't be checked this way.</summary>
	public static readonly HashSet<string> Unresolved = new();

	/// <summary>Loads each sequence in <paramref name="from"/> and saves it to <paramref name="to"/> through the real serializer.</summary>
	/// <returns>How many could not be loaded or saved.</returns>
	public static int Resave(string from, string to)
	{
		Directory.CreateDirectory(to);
		var failures = 0;
		foreach (var file in Directory.GetFiles(from, "*.tim").OrderBy(f => f, StringComparer.Ordinal))
		{
			var name = Path.GetFileName(file);
			try
			{
				var sequence = SequenceService.Instance.Load(file);
				var effects = sequence.SequenceData.EffectData.Count();
				SequenceService.Instance.Save(sequence, Path.Combine(to, name));
				(sequence as IDisposable)?.Dispose();
				Console.WriteLine($"{name}: {effects} effects");
			}
			catch (Exception e)
			{
				failures++;
				Console.WriteLine($"{name}: FAILED {Text.Trunc(e.ToString(), 600)}");
			}
		}
		return failures;
	}
}
