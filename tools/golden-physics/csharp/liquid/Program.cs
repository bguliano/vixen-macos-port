using System.Globalization;
using System.Text.Json;
using NLog;
using NLog.Config;
using NLog.Targets;
using Vixen.Native;
using Vixen.Sys;

internal static class Program
{
	private static MemoryTarget _log;

	private static async Task<int> Main(string[] args)
	{
		CultureInfo.DefaultThreadCurrentCulture = CultureInfo.InvariantCulture;
		var profile = Path.Combine(Path.GetTempPath(), "liquid-diff-" + Guid.NewGuid().ToString("N"));
		Directory.CreateDirectory(profile);
		var configuration = new LoggingConfiguration();
		_log = new MemoryTarget("mem") { Layout = "${message} ${exception:format=tostring}" };
		configuration.AddRule(NLog.LogLevel.Warn, NLog.LogLevel.Fatal, _log);
		LogManager.Configuration = configuration;
		VixenSystem.UIContext = new SynchronizationContext();
		await using var runtime = await NativeVixenRuntime.StartAsync(profile);
		_log.Logs.Clear();

		using var doc = JsonDocument.Parse(File.ReadAllText(args[0]));
		using var w = new StreamWriter(args[1]);
		foreach (var c in doc.RootElement.EnumerateArray())
		{
			var id = c.GetProperty("id").GetString();
			try
			{
				w.WriteLine(id + "\t" + LiquidDiff.RenderCase(c, _log));
			}
			catch (Exception e)
			{
				w.WriteLine(id + "\tTHROWN " + e.GetType().Name + " " + e.Message);
			}
		}
		try { Directory.Delete(profile, true); } catch { }
		return 0;
	}
}
