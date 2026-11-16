// Diagnostic host only. Vixen/NAudio perform all decoding; no playback runtime starts.
using System.ComponentModel;
using System.Diagnostics;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.Loader;
using System.Security.Cryptography;
using System.Text.Json;

internal static class Program
{
    private static readonly JsonSerializerOptions Json = new() { WriteIndented = true };
    private const string Usage = "WindowsPcmCapture --vixen-bin <matching Windows Vixen folder> --input <MP3> --out <NEW folder>\n" +
        "Requires Windows and .NET 10; invokes only original AudioFileReader/CachedSampleSource.\n" +
        "No audio device, Vixen application, controller, profile, or network is opened.";

    public static int Main(string[] args)
    {
        try
        {
            if (args.Length == 1 && args[0] is "--help" or "-h") { Console.WriteLine(Usage); return 0; }
            var options = Parse(args);
            if (!OperatingSystem.IsWindows()) throw new PlatformNotSupportedException("Actual capture requires Windows Media Foundation. No decoder was opened.");
            Capture(options);
            return 0;
        }
        catch (Exception exception)
        {
            while (exception is TargetInvocationException { InnerException: not null } invoke) exception = invoke.InnerException!;
            Console.Error.WriteLine($"ERROR: {exception.GetType().Name}: {exception.Message}");
            return 1;
        }
    }

    private sealed record Options(string VixenBin, string Input, string Output);
    private static Options Parse(string[] args)
    {
        var values = new Dictionary<string, string>(StringComparer.Ordinal);
        for (var i = 0; i < args.Length; i += 2)
        {
            if (args[i] is not ("--vixen-bin" or "--input" or "--out") || i + 1 == args.Length || !values.TryAdd(args[i], args[i + 1]))
                throw new ArgumentException(Usage);
        }
        if (values.Count != 3) throw new ArgumentException(Usage);
        var result = new Options(Path.GetFullPath(values["--vixen-bin"]), Path.GetFullPath(values["--input"]), Path.GetFullPath(values["--out"]));
        foreach (var path in new[] { result.Input, result.VixenBin, result.Output })
        {
            if (path.StartsWith("\\\\", StringComparison.Ordinal) ||
                (OperatingSystem.IsWindows() && new DriveInfo(Path.GetPathRoot(path)!).DriveType == DriveType.Network))
                throw new ArgumentException("Use local paths, not UNC/network paths.");
            if (OperatingSystem.IsWindows()) RejectReparseAncestors(path);
        }
        if (!Directory.Exists(result.VixenBin)) throw new DirectoryNotFoundException("The supplied Vixen binary directory does not exist.");
        if (!File.Exists(result.Input) || !string.Equals(Path.GetExtension(result.Input), ".mp3", StringComparison.OrdinalIgnoreCase))
            throw new ArgumentException("Input must be an existing local MP3 file.");
        if (File.Exists(result.Output) || Directory.Exists(result.Output)) throw new IOException("Output already exists; choose a fresh directory.");
        var parent = Path.GetDirectoryName(result.Output);
        if (parent is null || !Directory.Exists(parent)) throw new DirectoryNotFoundException("Create the output's parent directory first.");
        if (Within(result.Output, result.VixenBin)) throw new ArgumentException("Output must be outside the Vixen installation.");
        return result;
    }

    private static void RejectReparseAncestors(string path)
    {
        var root = Path.GetPathRoot(path)!;
        var current = root;
        foreach (var segment in path[root.Length..].Split(Path.DirectorySeparatorChar, StringSplitOptions.RemoveEmptyEntries))
        {
            current = Path.Combine(current, segment);
            FileAttributes attributes;
            try { attributes = File.GetAttributes(current); }
            catch (FileNotFoundException) { break; }
            catch (DirectoryNotFoundException) { break; }
            if ((attributes & FileAttributes.ReparsePoint) != 0)
                throw new ArgumentException("Use ordinary local copies, not symlinks, junctions, or cloud placeholders.");
        }
    }

    private static void ValidateAssemblies(Options options)
    {
        foreach (var name in new[] { "AudioPlayer.dll", "NAudio.Core.dll", "NAudio.Wasapi.dll", "NAudio.WinMM.dll", "NAudio.Vorbis.dll", "NVorbis.dll", "NLog.dll" })
        {
            var path = Path.Combine(options.VixenBin, name);
            if (!File.Exists(path)) throw new FileNotFoundException($"Missing matching Vixen dependency: {name}");
            var identity = AssemblyName.GetAssemblyName(path);
            if (identity.Name != Path.GetFileNameWithoutExtension(name)) throw new InvalidDataException($"Incorrect assembly identity for {name}: {identity.Name}");
            if (name is "NAudio.Core.dll" or "NAudio.Wasapi.dll" or "NAudio.WinMM.dll" && identity.Version != new Version(2, 3, 0, 0))
                throw new InvalidDataException($"{name} must have assembly version 2.3.0.0; found {identity.Version}.");
        }
    }

    private static bool Within(string path, string directory) =>
        string.Equals(path, directory, StringComparison.OrdinalIgnoreCase) ||
        path.StartsWith(directory.TrimEnd(Path.DirectorySeparatorChar) + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase);

    private sealed class VixenAssemblies(string directory) : AssemblyLoadContext("OriginalVixenAudio", isCollectible: false)
    {
        private readonly HashSet<string> framework = ((string?)AppContext.GetData("TRUSTED_PLATFORM_ASSEMBLIES") ?? "")
            .Split(Path.PathSeparator).Select(Path.GetFileNameWithoutExtension).OfType<string>().ToHashSet(StringComparer.OrdinalIgnoreCase);
        protected override Assembly? Load(AssemblyName identity)
        {
            // Only the supplied installation supplies non-framework dependencies.
            // Keep System.* bound to the installed .NET runtime when available.
            if (identity.Name is null || framework.Contains(identity.Name)) return null;
            if (Path.GetFileName(identity.Name) != identity.Name) throw new InvalidDataException("Invalid assembly name.");
            var path = Path.Combine(directory, identity.Name + ".dll");
            if (!File.Exists(path)) throw new FileNotFoundException($"Missing supplied-install dependency: {identity.Name}", path);
            return LoadFromAssemblyPath(path);
        }
    }

    private sealed record Format(string Encoding, int SampleRate, int Channels, int BitsPerSample, int BlockAlign, int AverageBytesPerSecond);
    private static object Property(object value, string name) => value.GetType().GetProperty(name)?.GetValue(value)
        ?? throw new MissingMemberException(value.GetType().FullName, name);
    private static Format WaveFormat(object value) => new(
        Property(value, "Encoding").ToString()!, (int)Property(value, "SampleRate"), (int)Property(value, "Channels"),
        (int)Property(value, "BitsPerSample"), (int)Property(value, "BlockAlign"), (int)Property(value, "AverageBytesPerSecond"));
    private static string Hash(string path) { using var file = File.OpenRead(path); return Convert.ToHexStringLower(SHA256.HashData(file)); }
    private static object FileIdentity(string path) => new {
        path, bytes = new FileInfo(path).Length, sha256 = Hash(path),
        fileVersion = FileVersionInfo.GetVersionInfo(path).FileVersion,
    };

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CreateDirectoryW(string path, IntPtr securityAttributes);

    private static void Capture(Options options)
    {
        ValidateAssemblies(options);
        if (!BitConverter.IsLittleEndian) throw new PlatformNotSupportedException("The capture format requires a little-endian runtime.");
        var inputHash = Hash(options.Input);
        var loader = new VixenAssemblies(options.VixenBin);
        // The diagnostic must not inherit application log targets that write a profile.
        // This changes only logging destinations, before any original reader is constructed.
        var logging = loader.LoadFromAssemblyPath(Path.Combine(options.VixenBin, "NLog.dll"));
        var configuration = Activator.CreateInstance(logging.GetType("NLog.Config.LoggingConfiguration", throwOnError: true)!);
        logging.GetType("NLog.LogManager", throwOnError: true)!.GetProperty("Configuration")!.SetValue(null, configuration);
        var assembly = loader.LoadFromAssemblyPath(Path.Combine(options.VixenBin, "AudioPlayer.dll"));
        var readerType = assembly.GetType("Common.AudioPlayer.FileReader.AudioFileReader", throwOnError: true)!;
        var cachedType = assembly.GetType("Common.AudioPlayer.SampleProvider.CachedSampleSource", throwOnError: true)!;
        Format negotiated, converted;
        string decoderType;
        long reportedStreamBytes;
        long readerSamples = 0;
        string readerPcmHash;
        // The first original reader exposes the negotiated underlying format.
        // The second decode below is the unchanged Vixen cache constructor.
        using (var reader = (IDisposable)Activator.CreateInstance(readerType, options.Input)!)
        {
            var field = readerType.GetField("_readerStream", BindingFlags.NonPublic | BindingFlags.Instance)
                ?? throw new MissingFieldException(readerType.FullName, "_readerStream");
            var stream = field.GetValue(reader) ?? throw new InvalidDataException("Original reader has no stream.");
            decoderType = stream.GetType().AssemblyQualifiedName!;
            if (stream.GetType().FullName != "NAudio.Wave.MediaFoundationReader") throw new InvalidDataException("The supplied MP3 reader does not use the expected MediaFoundationReader.");
            negotiated = WaveFormat(Property(stream, "WaveFormat"));
            converted = WaveFormat(Property(reader, "WaveFormat"));
            reportedStreamBytes = (long)Property(reader, "StreamLength");
            var read = readerType.GetMethod("Read", [typeof(float[]), typeof(int), typeof(int)])
                ?? throw new MissingMethodException(readerType.FullName, "Read(float[], int, int)");
            var buffer = new float[checked(converted.SampleRate * converted.Channels)];
            using var checksum = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
            while (true)
            {
                var count = (int)read.Invoke(reader, [buffer, 0, buffer.Length])!;
                if (count == 0) break;
                if (count < 0 || count > buffer.Length) throw new InvalidDataException("Invalid original reader sample count.");
                readerSamples = checked(readerSamples + count);
                checksum.AppendData(MemoryMarshal.AsBytes(buffer.AsSpan(0, count)));
            }
            readerPcmHash = Convert.ToHexStringLower(checksum.GetHashAndReset());
        }
        var cache = Activator.CreateInstance(cachedType, options.Input)!;
        var samples = (float[])Property(cache, "AudioData");
        var format = WaveFormat(Property(cache, "WaveFormat"));
        if (format != converted || format.BitsPerSample != 32 || format.Encoding != "IeeeFloat" ||
            format.SampleRate <= 0 || format.Channels <= 0 || samples.Length == 0 || samples.Length % format.Channels != 0)
            throw new InvalidDataException("Unexpected or inconsistent decoded PCM format.");
        if (Hash(options.Input) != inputHash) throw new IOException("Input changed while decoding; capture discarded.");
        var cacheHash = Convert.ToHexStringLower(SHA256.HashData(MemoryMarshal.AsBytes(samples.AsSpan())));
        if (readerSamples != samples.LongLength || readerPcmHash != cacheHash)
            throw new InvalidDataException("The separate format-probe decode differs from CachedSampleSource; capture discarded.");

        // Win32 CreateDirectory is exclusive: an existing/racing path is never owned or removed.
        if (!CreateDirectoryW(options.Output, IntPtr.Zero)) throw new Win32Exception(Marshal.GetLastWin32Error(), "Cannot create a fresh output directory.");
        var pcmPath = Path.Combine(options.Output, "cached.f32le");
        using (var file = new FileStream(pcmPath, FileMode.CreateNew, FileAccess.Write, FileShare.None))
            file.Write(MemoryMarshal.AsBytes(samples.AsSpan()));

        var managed = AppDomain.CurrentDomain.GetAssemblies().Where(a => !a.IsDynamic && a.Location.Length > 0)
            .OrderBy(a => a.FullName, StringComparer.Ordinal)
            .Select(a => new { identity = a.FullName, file = FileIdentity(a.Location) }).ToArray();
        using var process = Process.GetCurrentProcess();
        var native = process.Modules.Cast<ProcessModule>()
            .Where(m => m.ModuleName.StartsWith("mf", StringComparison.OrdinalIgnoreCase) ||
                m.ModuleName.Equals("mp3dmod.dll", StringComparison.OrdinalIgnoreCase) ||
                m.ModuleName.Equals("coreclr.dll", StringComparison.OrdinalIgnoreCase))
            .OrderBy(m => m.FileName, StringComparer.OrdinalIgnoreCase).Select(m => FileIdentity(m.FileName)).ToArray();
        using var referenceStream = Assembly.GetExecutingAssembly().GetManifestResourceStream("WindowsPcmCapture.reference.json")!;
        var reference = JsonSerializer.Deserialize<JsonElement>(referenceStream);
        var manifest = new {
            schema = 1, status = "captured-on-windows", capturedUtc = DateTimeOffset.UtcNow,
            input = new { path = options.Input, bytes = new FileInfo(options.Input).Length, sha256 = inputHash },
            runtime = new { os = RuntimeInformation.OSDescription, framework = RuntimeInformation.FrameworkDescription,
                processArchitecture = RuntimeInformation.ProcessArchitecture.ToString(), osArchitecture = RuntimeInformation.OSArchitecture.ToString() },
            decoder = new { type = decoderType, negotiatedFormat = negotiated, reportedStreamBytes,
                separateReaderSamples = readerSamples, separateReaderPcmSha256 = readerPcmHash,
                note = "Format observed from a separate original AudioFileReader decode; its complete float sample count/hash exactly match the unchanged CachedSampleSource decode." },
            pcm = new { file = "cached.f32le", sha256 = Hash(pcmPath), bytes = new FileInfo(pcmPath).Length,
                format, samples = samples.LongLength, frames = samples.LongLength / format.Channels,
                nonFiniteSamples = samples.LongCount(s => !float.IsFinite(s)),
                timeOrigin = "Original CachedSampleSource.AudioData[0]; no trim, shift, resample or quantization added." },
            referenceSourceContract = reference,
            managedAssemblies = managed, nativeDecoderRuntimeModules = native,
        };
        // A manifest is published only after a complete PCM file. Partial failures remain explicit.
        using (var file = new FileStream(Path.Combine(options.Output, "manifest.json"), FileMode.CreateNew, FileAccess.Write, FileShare.None))
            JsonSerializer.Serialize(file, manifest, Json);
        Console.WriteLine($"Captured {samples.LongLength / format.Channels} frames at {format.SampleRate} Hz / {format.Channels} channels.");
        Console.WriteLine($"Underlying decoder: {negotiated.Encoding}, {negotiated.BitsPerSample} bits; output: {options.Output}");
    }
}
