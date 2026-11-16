// C# 5 / .NET Framework 4.x. Uses only Windows system APIs and framework assemblies.
// The scheduler is shared with local fault tests; Windows jobs require Windows validation.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Runtime.Serialization;
using System.Runtime.Serialization.Json;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading;

namespace VixenCapture {
    public sealed class StageSpec {
        public string Name, Executable;
        public string[] Arguments = new string[0];
        public int Seconds;
    }
    [DataContract] public sealed class StageResult {
        [DataMember] public string name;
        [DataMember] public string status;
        [DataMember] public string error;
        [DataMember] public int? pid;
        [DataMember] public int? exitCode;
        [DataMember] public double elapsedSeconds;
        [DataMember] public double budgetSeconds;
        [DataMember] public bool timedOut;
        [DataMember] public bool overallExpired;
        [DataMember] public bool outputLimit;
    }
    public interface IStageProcess : IDisposable {
        int Id { get; }
        bool HasExited { get; }
        int ExitCode { get; }
        bool Wait(int milliseconds);
        void Terminate();
    }
    public interface IStageLauncher {
        IStageProcess Start(StageSpec spec, string stdoutPath, string stderrPath);
    }
    [DataContract] public sealed class InventoryChoice {
        [DataMember] public string selectedBin;
        [DataMember] public bool assemblyReady;
        [DataMember] public string[] hosts;
        [DataMember] public string reason;
    }
    [DataContract] public sealed class Summary {
        [DataMember] public int runnerVersion = 3;
        [DataMember] public string patchVersion = "3.2";
        [DataMember] public string status = "diagnostics-only";
        [DataMember] public string results;
        [DataMember] public string selectedBin;
        [DataMember] public string pcm = "not-run";
        [DataMember] public string captureContract = "not-selected";
        [DataMember] public string gdiNet10 = "not-run";
        [DataMember] public string inventory = "not-run";
        [DataMember] public string package = "not-run";
        [DataMember] public List<StageResult> stages = new List<StageResult>();
        [DataMember] public List<string> notes = new List<string>();
    }
    // Console selection/slow readers must not suspend stage deadlines. The file
    // checkpoint is authoritative; console output is best-effort and bounded.
    internal static class ConsoleProgress {
        static readonly object gate = new object();
        static readonly Queue<string> pending = new Queue<string>();
        static readonly AutoResetEvent ready = new AutoResetEvent(false);
        static ConsoleProgress() {
            var thread = new Thread(WriteLoop); thread.IsBackground = true; thread.Name = "diagnostic-console"; thread.Start();
        }
        internal static void Send(string line) {
            lock (gate) { if (pending.Count >= 128) pending.Dequeue(); pending.Enqueue(line); }
            ready.Set();
        }
        static void WriteLoop() {
            for (;;) {
                ready.WaitOne();
                for (;;) {
                    string line;
                    lock (gate) { if (pending.Count == 0) break; line = pending.Dequeue(); }
                    try { Console.WriteLine(line); } catch { }
                }
            }
        }
    }
    public sealed class CaptureSupervisor {
        readonly string output;
        readonly IStageLauncher launcher;
        readonly Stopwatch overall = Stopwatch.StartNew();
        readonly double overallSeconds;
        double reserveSeconds;
        public readonly Summary Report = new Summary();
        public const long LogLimit = 32L * 1024 * 1024;
        public CaptureSupervisor(string outputDirectory, IStageLauncher processLauncher, double totalSeconds) {
            output = outputDirectory; launcher = processLauncher; overallSeconds = totalSeconds;
            Report.results = outputDirectory;
        }
        public static string Json(object value) {
            using (var stream = new MemoryStream()) {
                new DataContractJsonSerializer(value.GetType()).WriteObject(stream, value);
                return Encoding.UTF8.GetString(stream.ToArray());
            }
        }
        public static void Save(object value, string path) {
            // Keep the previous complete checkpoint if serialization/write is interrupted.
            string temporary = path + ".tmp";
            File.WriteAllText(temporary, Json(value), new UTF8Encoding(false));
            if (File.Exists(path)) File.Replace(temporary, path, null);
            else File.Move(temporary, path);
        }
        public void Note(string message) {
            string line = DateTime.UtcNow.ToString("o") + " " + message;
            try { File.AppendAllText(Path.Combine(output, "progress.log"), line + Environment.NewLine); }
            catch (Exception e) { ConsoleProgress.Send("Progress file unavailable: " + e.Message); }
            ConsoleProgress.Send(line);
        }
        public void Checkpoint() {
            try { Save(Report, Path.Combine(output, "diagnostics.json")); }
            catch (Exception e) { Note("Could not update diagnostics; prior stage files remain: " + e.Message); }
        }
        bool OverLimit(string path) { return File.Exists(path) && new FileInfo(path).Length > LogLimit; }
        public StageResult RunStage(StageSpec spec) {
            if (!Regex.IsMatch(spec.Name, "^[a-z0-9-]{1,48}$")) throw new ArgumentException("Invalid stage name");
            var record = new StageResult { name = spec.Name, status = "starting" };
            record.budgetSeconds = Math.Max(0, Math.Min(spec.Seconds, overallSeconds - reserveSeconds - overall.Elapsed.TotalSeconds));
            var watch = Stopwatch.StartNew(); IStageProcess child = null;
            string stdout = Path.Combine(output, spec.Name + ".stdout.txt");
            string stderr = Path.Combine(output, spec.Name + ".stderr.txt");
            try {
                if (record.budgetSeconds <= 0) {
                    record.overallExpired = true; record.status = "skipped-overall-deadline";
                    return record;
                }
                Note("START " + spec.Name + " (limit " + record.budgetSeconds.ToString("F1") + "s)");
                child = launcher.Start(spec, stdout, stderr); record.pid = child.Id;
                record.status = "running";
                Save(record, Path.Combine(output, spec.Name + ".stage.json"));
                double heartbeat = 0;
                while (!child.Wait(200)) {
                    if (watch.Elapsed.TotalSeconds >= record.budgetSeconds) {
                        record.timedOut = true;
                        record.overallExpired = overall.Elapsed.TotalSeconds >= overallSeconds;
                        record.status = "timed-out"; child.Terminate(); child.Wait(2000); break;
                    }
                    if (OverLimit(stdout) || OverLimit(stderr)) {
                        record.outputLimit = true; record.status = "output-limit";
                        child.Terminate(); child.Wait(2000); break;
                    }
                    if (watch.Elapsed.TotalSeconds >= heartbeat) {
                        Note("RUNNING " + spec.Name + " PID " + child.Id + " " + watch.Elapsed.TotalSeconds.ToString("F0") + "s; partial files are safe to copy");
                        heartbeat = watch.Elapsed.TotalSeconds + 5;
                    }
                }
                if (child.HasExited) record.exitCode = child.ExitCode;
                if (record.status == "running") record.status = record.exitCode == 0 ? "completed" : "failed";
                if (OverLimit(stdout) || OverLimit(stderr)) { record.outputLimit = true; record.status = "output-limit"; }
            } catch (Exception e) { record.status = "failed"; record.error = e.GetType().Name + ": " + e.Message; }
            finally {
                // Closing a job kills descendants too, including a child that inherited log handles.
                if (child != null) {
                    try { child.Terminate(); } catch (Exception e) { record.error = (record.error ?? "") + " cleanup: " + e.Message; }
                    try { child.Dispose(); } catch (Exception e) { record.error = (record.error ?? "") + " dispose: " + e.Message; }
                }
                record.elapsedSeconds = Math.Round(watch.Elapsed.TotalSeconds, 3);
                Report.stages.Add(record);
                try { Save(record, Path.Combine(output, spec.Name + ".stage.json")); }
                catch (Exception e) { Note("Stage checkpoint unavailable: " + e.Message); }
                Checkpoint();
                Note("END " + spec.Name + ": " + record.status + "; exit=" + record.exitCode + "; " + record.elapsedSeconds + "s");
            }
            return record;
        }
        public static string OptionalProbeSkipReason(bool inventoryReady, bool packTrusted, int hostCandidates, int successfulRuntimeListings) {
            if (!inventoryReady) return "skipped-inventory-unavailable";
            if (!packTrusted) return "skipped-pack-integrity";
            if (hostCandidates == 0) return "skipped-no-runtime-host";
            if (successfulRuntimeListings == 0) return "skipped-runtime-probe-unavailable";
            return "skipped-no-verified-net10";
        }
        void SkipDependency(string name, string reason) {
            var record = new StageResult { name = name, status = "skipped-dependency", error = reason };
            Report.stages.Add(record);
            try { Save(record, Path.Combine(output, name + ".stage.json")); }
            catch (Exception e) { Note("Skipped-stage checkpoint unavailable: " + e.Message); }
            Checkpoint(); Note("SKIP " + name + ": " + reason);
        }
        public static T ReadJson<T>(string path, int limit) {
            using (var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite)) {
                if (stream.Length > limit) throw new InvalidDataException("Report exceeds size limit");
                // PowerShell 5.1 UTF8 includes a BOM; remove it before the serializer.
                byte[] bytes = new byte[(int)stream.Length]; int count = 0;
                while (count < bytes.Length) { int n = stream.Read(bytes, count, bytes.Length - count); if (n == 0) throw new EndOfStreamException(); count += n; }
                int start = bytes.Length >= 3 && bytes[0] == 239 && bytes[1] == 187 && bytes[2] == 191 ? 3 : 0;
                using (var input = new MemoryStream(bytes, start, bytes.Length - start))
                    return (T)new DataContractJsonSerializer(typeof(T)).ReadObject(input);
            }
        }
        public static string ReadText(string path, int limit) {
            if (new FileInfo(path).Length > limit) throw new InvalidDataException("Text exceeds size limit");
            return File.ReadAllText(path);
        }
        StageSpec Worker(string pack, string stage, int seconds, string selected) {
            var arguments = new List<string> { "-NoLogo", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", Path.Combine(pack, "Capture-Worker.ps1"), "-Stage", stage, "-Out", output };
            if (!String.IsNullOrEmpty(selected)) { arguments.Add("-VixenRoot"); arguments.Add(selected); }
            return new StageSpec { Name = stage, Seconds = seconds,
                Executable = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Windows), @"System32\WindowsPowerShell\v1.0\powershell.exe"),
                Arguments = arguments.ToArray() };
        }
        string RunLegacyCapture(string pack, string dotnet, string selected) {
            Report.captureContract = "vixen3.12.4-net8-naudio2.2.1";
            // Only hash-pinned, privately copied dependencies reach this host. The
            // original installation directory is never an assembly search path.
            string copies = Path.Combine(output, "reference-assemblies");
            string hostDirectory = Path.Combine(output, "legacy-host");
            string executable = Path.Combine(hostDirectory, "LegacyCapture.exe");
            var prepared = RunStage(Worker(pack, "legacy-prepare", 20, selected));
            if (prepared.status != "completed") return "skipped-legacy-prepare-" + prepared.status;
            var compiled = RunStage(new StageSpec { Name = "legacy-compile", Seconds = 30,
                Executable = Path.Combine(RuntimeEnvironment.GetRuntimeDirectory(), "csc.exe"),
                Arguments = new[] { "/nologo", "/noconfig", "/target:exe", "/platform:x64", "/langversion:5", "/optimize+",
                    "/reference:System.dll", "/reference:System.Core.dll", "/out:" + executable,
                    Path.Combine(hostDirectory, "LegacyCapture.cs.in") } });
            if (compiled.status != "completed") return "skipped-legacy-compile-" + compiled.status;
            if (!File.Exists(executable) || new FileInfo(executable).Length == 0 ||
                !File.Exists(Path.Combine(hostDirectory, "LegacyCapture.runtimeconfig.json"))) {
                Report.notes.Add("Legacy compiler exited successfully but its executable or runtime configuration is missing/empty; optional capture was not launched.");
                return "failed-legacy-compiler-output";
            }
            var contract = RunStage(new StageSpec { Name = "legacy-contract", Seconds = 20, Executable = dotnet,
                Arguments = new[] { executable, "--validate-contract", "--vixen-bin", copies } });
            if (contract.status != "completed") return "skipped-legacy-contract-" + contract.status;
            string preflightDirectory = Path.Combine(output, "legacy-preflight");
            var preflight = RunStage(new StageSpec { Name = "legacy-preflight", Seconds = 30, Executable = dotnet,
                Arguments = new[] { executable, "--preflight", "--vixen-bin", copies, "--out", preflightDirectory } });
            if (preflight.status != "completed") return "skipped-legacy-preflight-" + preflight.status;
            if (!File.Exists(Path.Combine(preflightDirectory, "preflight.json"))) {
                Report.notes.Add("Legacy preflight exited successfully but did not publish its report.");
                return "failed-legacy-preflight-missing-report";
            }
            var capture = RunStage(new StageSpec { Name = "pcm", Seconds = 300, Executable = dotnet,
                Arguments = new[] { executable, "--vixen-bin", copies, "--input",
                    Path.Combine(pack, @"input\Coldest Winter Pentatonix.mp3"), "--out", Path.Combine(output, "capture") } });
            if (capture.status == "completed" && !File.Exists(Path.Combine(output, "capture", "manifest.json"))) {
                Report.notes.Add("Legacy capture exited successfully without a completed manifest; do not compare partial PCM.");
                return "failed-missing-manifest";
            }
            return capture.status;
        }
        public int Run(string pack, string selected) {
            reserveSeconds = 70; // Retain a packaging/cleanup budget even after optional probes time out.
            Checkpoint();
            // Inventory runs even when optional managed hosts cannot start.
            var inventory = RunStage(Worker(pack, "inventory", 60, selected));
            Report.inventory = inventory.status;
            InventoryChoice choice = null;
            if (inventory.status == "completed") {
                try {
                    choice = ReadJson<InventoryChoice>(Path.Combine(output, "inventory-choice.json"), 65536);
                    if (choice == null || choice.selectedBin == null || choice.reason == null || choice.hosts == null || choice.hosts.Length > 8)
                        throw new InvalidDataException("Inventory choice is missing required fields or exceeds the host cap");
                    foreach (string host in choice.hosts) if (String.IsNullOrWhiteSpace(host)) throw new InvalidDataException("Inventory contains an empty runtime host path");
                    if (choice.assemblyReady && String.IsNullOrWhiteSpace(choice.selectedBin)) throw new InvalidDataException("Compatible assembly selection has no directory");
                }
                catch (Exception e) { choice = null; Report.notes.Add("Inventory choice unavailable/malformed: " + e.Message); }
            }
            if (choice != null) RunStage(Worker(pack, "reference-copy", 30, selected));
            else SkipDependency("reference-copy", "Completed inventory choice is unavailable; no installed DLLs were copied.");
            var integrity = RunStage(Worker(pack, "integrity", 30, selected));
            bool trusted = integrity.status == "completed";
            string dotnet = null, dotnet8 = null;
            int hostCandidates = 0, successfulRuntimeListings = 0;
            if (choice != null) {
                Report.selectedBin = choice.selectedBin;
                if (!String.IsNullOrEmpty(choice.reason)) Report.notes.Add(choice.reason);
                int index = 0;
                foreach (string host in choice.hosts ?? new string[0]) {
                    if (index >= 8) { Report.notes.Add("Additional runtime hosts omitted at cap8"); break; }
                    string name = "runtimes-" + index++; hostCandidates++;
                    try {
                        Paths.Local(host, true);
                        var runtime = RunStage(new StageSpec { Name = name, Executable = host, Arguments = new[] { "--list-runtimes" }, Seconds = 20 });
                        if (runtime.status == "completed") {
                            string text = ReadText(Path.Combine(output, name + ".stdout.txt"), 65536);
                            successfulRuntimeListings++;
                            if (dotnet == null && Regex.IsMatch(text, @"(?m)^Microsoft\.NETCore\.App 10\.\d+\.\d+ ")) dotnet = host;
                            if (dotnet8 == null && Regex.IsMatch(text, @"(?m)^Microsoft\.NETCore\.App 8\.0\.\d+ ")) dotnet8 = host;
                        }
                    } catch (Exception e) { Report.notes.Add("Runtime inspection: " + e.Message); Checkpoint(); }
                }
            }
            RunStage(Worker(pack, "framework-gdi", 30, selected));
            if (trusted && dotnet != null) {
                Report.gdiNet10 = RunStage(new StageSpec { Name = "net10-gdi", Executable = dotnet,
                    Arguments = new[] { Path.Combine(pack, @"gdi\GdiAvailability.dll") }, Seconds = 30 }).status;
            } else Report.gdiNet10 = OptionalProbeSkipReason(choice != null, trusted, hostCandidates, successfulRuntimeListings);
            if (trusted && dotnet != null && choice != null && choice.assemblyReady && !String.IsNullOrEmpty(choice.selectedBin)) {
                    try {
                        Report.captureContract = "strict-net10-naudio2.3";
                        Paths.Local(choice.selectedBin, true);
                        if (!Directory.Exists(choice.selectedBin)) throw new DirectoryNotFoundException("Selected audio directory is no longer present");
                        if (Paths.Within(pack, choice.selectedBin)) throw new InvalidOperationException("Pack must be outside selected install");
                        Report.pcm = RunStage(new StageSpec { Name = "pcm", Executable = dotnet,
                            Arguments = new[] { Path.Combine(pack, @"capture\WindowsPcmCapture.dll"), "--vixen-bin", choice.selectedBin.TrimEnd('\\'),
                                "--input", Path.Combine(pack, @"input\Coldest Winter Pentatonix.mp3"), "--out", Path.Combine(output, "capture") }, Seconds = 300 }).status;
                        if (Report.pcm == "completed" && File.Exists(Path.Combine(output, @"capture\manifest.json"))) Report.status = "pcm-captured";
                    } catch (Exception e) {
                        Report.pcm = "failed-preflight";
                        Report.notes.Add("Optional PCM setup failed; remaining diagnostics continue: " + e.Message);
                        Checkpoint();
                    }
            } else if (trusted && dotnet8 != null && choice != null && !String.IsNullOrEmpty(choice.selectedBin)) {
                try {
                    Report.pcm = RunLegacyCapture(pack, dotnet8, selected);
                    if (Report.pcm == "completed") Report.status = "pcm-captured";
                } catch (Exception e) {
                    Report.pcm = "failed-legacy-preflight";
                    Report.notes.Add("Optional legacy capture failed; remaining diagnostics continue: " + e.Message);
                    Checkpoint();
                }
            } else {
                Report.pcm = trusted && (dotnet != null || dotnet8 != null) ? "skipped-incompatible-or-ambiguous-install" :
                    OptionalProbeSkipReason(choice != null, trusted, hostCandidates, successfulRuntimeListings);
                if (Report.pcm == "skipped-no-verified-net10") Report.pcm = "skipped-no-verified-net8-or-net10";
                Report.notes.Add("Optional probe reason: " + Report.pcm + ". No runtime was installed/downloaded. Missing or failed inventory/runtime probes do not establish which runtimes are installed.");
            }
            var inputCheck = RunStage(Worker(pack, "input-recheck", 15, selected));
            if (inputCheck.status != "completed") { Report.notes.Add("Final input integrity was not verified; do not compare a PCM capture until the input hashes are reviewed."); if (Report.status == "pcm-captured") Report.status = "pcm-captured-input-unverified"; }
            reserveSeconds = 0;
            Report.package = "pending";
            Report.notes.Add("The ZIP contains the pre-archive diagnostics checkpoint; package.stage.json and the final supervisor status remain alongside it in the results folder.");
            Checkpoint();
            var package = RunStage(Worker(pack, "package", 60, selected));
            Report.package = package.status;
            Checkpoint();
            string zip = output + ".zip";
            bool zipReady = package.status == "completed" && File.Exists(zip);
            Note("FINISHED " + Report.status);
            Note(zipReady ? "COPY BACK: " + zip : "ZIP unavailable. COPY THIS ENTIRE FOLDER BACK: " + output);
            Note("The folder also retains the final supervisor status. No Vixen application was launched.");
            return zipReady ? 0 : 2;
        }
    }
    public static class Paths {
        public static bool Within(string path, string root) {
            string p = Path.GetFullPath(path).TrimEnd(Path.DirectorySeparatorChar);
            string r = Path.GetFullPath(root).TrimEnd(Path.DirectorySeparatorChar);
            return String.Equals(p, r, StringComparison.OrdinalIgnoreCase) || p.StartsWith(r + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase);
        }
        public static string Local(string path, bool exists) {
            string full = Path.GetFullPath(path);
            if (full.StartsWith(@"\\") || full.IndexOf('"') >= 0 || full.IndexOf('\n') >= 0 || full.IndexOf('\r') >= 0)
                throw new IOException("Only ordinary local paths are supported");
            if (new DriveInfo(Path.GetPathRoot(full)).DriveType == DriveType.Network) throw new IOException("Network drives are excluded");
            if (exists && !File.Exists(full) && !Directory.Exists(full)) throw new FileNotFoundException("Path not found", full);
            string cursor = full;
            while (!String.IsNullOrEmpty(cursor)) {
                if ((File.Exists(cursor) || Directory.Exists(cursor)) && (File.GetAttributes(cursor) & FileAttributes.ReparsePoint) != 0)
                    throw new IOException("Linked/cloud paths are excluded: " + cursor);
                cursor = Path.GetDirectoryName(cursor);
            }
            return full;
        }
    }
    public sealed class WindowsLauncher : IStageLauncher {
        public IStageProcess Start(StageSpec spec, string stdoutPath, string stderrPath) { return new WindowsChild(spec, stdoutPath, stderrPath); }
    }
    public sealed class WindowsChild : IStageProcess {
        IntPtr job, process;
        int id;
        public int Id { get { return id; } }
        public bool HasExited { get { return Native.WaitForSingleObject(process, 0) == 0; } }
        public int ExitCode { get { uint code; if (!Native.GetExitCodeProcess(process, out code)) throw Native.Error("GetExitCodeProcess"); return unchecked((int)code); } }
        public bool Wait(int milliseconds) {
            uint result = Native.WaitForSingleObject(process, (uint)milliseconds);
            if (result == 0xffffffff) throw Native.Error("WaitForSingleObject");
            return result == 0;
        }
        public void Terminate() { if (job != IntPtr.Zero && !Native.TerminateJobObject(job, 1460)) throw Native.Error("TerminateJobObject"); }
        public void Dispose() {
            if (job != IntPtr.Zero) { Native.CloseHandle(job); job = IntPtr.Zero; }
            if (process != IntPtr.Zero) { Native.WaitForSingleObject(process, 2000); Native.CloseHandle(process); process = IntPtr.Zero; }
        }
        public static string Quote(string value) {
            if (value.IndexOf('\0') >= 0 || value.IndexOf('\r') >= 0 || value.IndexOf('\n') >= 0) throw new ArgumentException("Invalid command argument");
            var b = new StringBuilder("\""); int slashes = 0;
            foreach (char c in value) {
                if (c == '\\') { slashes++; continue; }
                if (c == '"') { b.Append('\\', slashes * 2 + 1); b.Append(c); }
                else { b.Append('\\', slashes); b.Append(c); }
                slashes = 0;
            }
            b.Append('\\', slashes * 2); b.Append('"'); return b.ToString();
        }
        public WindowsChild(StageSpec spec, string stdout, string stderr) {
            Paths.Local(spec.Executable, true);
            IntPtr outHandle = IntPtr.Zero, errHandle = IntPtr.Zero, inHandle = IntPtr.Zero, attrs = IntPtr.Zero, handles = IntPtr.Zero;
            bool attrsInitialized = false;
            Native.PROCESS_INFORMATION pi = new Native.PROCESS_INFORMATION();
            try {
                job = Native.CreateJobObject(IntPtr.Zero, null);
                if (job == IntPtr.Zero) throw Native.Error("CreateJobObject");
                var limits = new Native.JOBOBJECT_EXTENDED_LIMIT_INFORMATION(); limits.BasicLimitInformation.LimitFlags = 0x2000; // KILL_ON_JOB_CLOSE
                if (!Native.SetInformationJobObject(job, 9, ref limits, (uint)Marshal.SizeOf(limits))) throw Native.Error("SetInformationJobObject");
                var security = new Native.SECURITY_ATTRIBUTES(); security.nLength = Marshal.SizeOf(security); security.bInheritHandle = true;
                outHandle = Native.CreateFile(stdout, 0x40000000, 3, ref security, 1, 0x80, IntPtr.Zero);
                errHandle = Native.CreateFile(stderr, 0x40000000, 3, ref security, 1, 0x80, IntPtr.Zero);
                inHandle = Native.CreateFile("NUL", 0x80000000, 3, ref security, 3, 0x80, IntPtr.Zero);
                if (outHandle == new IntPtr(-1) || errHandle == new IntPtr(-1) || inHandle == new IntPtr(-1)) throw Native.Error("CreateFile logs");
                IntPtr bytes = IntPtr.Zero;
                Native.InitializeProcThreadAttributeList(IntPtr.Zero, 1, 0, ref bytes);
                attrs = Marshal.AllocHGlobal(bytes);
                if (!Native.InitializeProcThreadAttributeList(attrs, 1, 0, ref bytes)) throw Native.Error("InitializeProcThreadAttributeList");
                attrsInitialized = true;
                handles = Marshal.AllocHGlobal(IntPtr.Size * 3);
                Marshal.WriteIntPtr(handles, 0, outHandle); Marshal.WriteIntPtr(handles, IntPtr.Size, errHandle); Marshal.WriteIntPtr(handles, IntPtr.Size * 2, inHandle);
                if (!Native.UpdateProcThreadAttribute(attrs, 0, new IntPtr(0x20002), handles, new IntPtr(IntPtr.Size * 3), IntPtr.Zero, IntPtr.Zero)) throw Native.Error("UpdateProcThreadAttribute");
                var startup = new Native.STARTUPINFOEX(); startup.StartupInfo.cb = Marshal.SizeOf(startup); startup.StartupInfo.dwFlags = 0x100;
                startup.StartupInfo.hStdOutput = outHandle; startup.StartupInfo.hStdError = errHandle; startup.StartupInfo.hStdInput = inHandle; startup.lpAttributeList = attrs;
                var command = new StringBuilder(Quote(spec.Executable)); foreach (var arg in spec.Arguments) command.Append(" ").Append(Quote(arg));
                if (!Native.CreateProcess(spec.Executable, command, IntPtr.Zero, IntPtr.Zero, true, 0x08080004, IntPtr.Zero, Path.GetDirectoryName(stdout), ref startup, out pi)) throw Native.Error("CreateProcess");
                process = pi.hProcess; id = (int)pi.dwProcessId;
                if (!Native.AssignProcessToJobObject(job, process)) throw Native.Error("AssignProcessToJobObject");
                if (Native.ResumeThread(pi.hThread) == 0xffffffff) throw Native.Error("ResumeThread");
            } catch {
                if (pi.hProcess != IntPtr.Zero) { Native.TerminateProcess(pi.hProcess, 1460); Native.WaitForSingleObject(pi.hProcess, 2000); }
                Dispose(); throw;
            } finally {
                if (pi.hThread != IntPtr.Zero) Native.CloseHandle(pi.hThread);
                foreach (var handle in new[] { outHandle, errHandle, inHandle }) if (handle != IntPtr.Zero && handle != new IntPtr(-1)) Native.CloseHandle(handle);
                if (attrs != IntPtr.Zero) { if (attrsInitialized) Native.DeleteProcThreadAttributeList(attrs); Marshal.FreeHGlobal(attrs); }
                if (handles != IntPtr.Zero) Marshal.FreeHGlobal(handles);
            }
        }
    }
    internal static class Native {
        internal static Exception Error(string call) { return new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error(), call); }
        [StructLayout(LayoutKind.Sequential)] internal struct SECURITY_ATTRIBUTES { public int nLength; public IntPtr lpSecurityDescriptor; [MarshalAs(UnmanagedType.Bool)] public bool bInheritHandle; }
        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] internal struct STARTUPINFO { public int cb; public string lpReserved, lpDesktop, lpTitle; public uint dwX, dwY, dwXSize, dwYSize, dwXCountChars, dwYCountChars, dwFillAttribute, dwFlags; public short wShowWindow, cbReserved2; public IntPtr lpReserved2, hStdInput, hStdOutput, hStdError; }
        [StructLayout(LayoutKind.Sequential)] internal struct STARTUPINFOEX { public STARTUPINFO StartupInfo; public IntPtr lpAttributeList; }
        [StructLayout(LayoutKind.Sequential)] internal struct PROCESS_INFORMATION { public IntPtr hProcess, hThread; public uint dwProcessId, dwThreadId; }
        [StructLayout(LayoutKind.Sequential)] internal struct JOBOBJECT_BASIC_LIMIT_INFORMATION { public long PerProcessUserTimeLimit, PerJobUserTimeLimit; public uint LimitFlags; public UIntPtr MinimumWorkingSetSize, MaximumWorkingSetSize; public uint ActiveProcessLimit; public UIntPtr Affinity; public uint PriorityClass, SchedulingClass; }
        [StructLayout(LayoutKind.Sequential)] internal struct IO_COUNTERS { public ulong ReadOperationCount, WriteOperationCount, OtherOperationCount, ReadTransferCount, WriteTransferCount, OtherTransferCount; }
        [StructLayout(LayoutKind.Sequential)] internal struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION { public JOBOBJECT_BASIC_LIMIT_INFORMATION BasicLimitInformation; public IO_COUNTERS IoInfo; public UIntPtr ProcessMemoryLimit, JobMemoryLimit, PeakProcessMemoryUsed, PeakJobMemoryUsed; }
        [DllImport("kernel32.dll", SetLastError=true, CharSet=CharSet.Unicode)] internal static extern IntPtr CreateJobObject(IntPtr attributes, string name);
        [DllImport("kernel32.dll", SetLastError=true)] [return:MarshalAs(UnmanagedType.Bool)] internal static extern bool SetInformationJobObject(IntPtr job, int info, ref JOBOBJECT_EXTENDED_LIMIT_INFORMATION data, uint length);
        [DllImport("kernel32.dll", SetLastError=true)] [return:MarshalAs(UnmanagedType.Bool)] internal static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
        [DllImport("kernel32.dll", SetLastError=true)] [return:MarshalAs(UnmanagedType.Bool)] internal static extern bool TerminateJobObject(IntPtr job, uint exitCode);
        [DllImport("kernel32.dll", SetLastError=true)] [return:MarshalAs(UnmanagedType.Bool)] internal static extern bool TerminateProcess(IntPtr process, uint exitCode);
        [DllImport("kernel32.dll", SetLastError=true)] internal static extern uint WaitForSingleObject(IntPtr handle, uint milliseconds);
        [DllImport("kernel32.dll", SetLastError=true)] internal static extern uint ResumeThread(IntPtr thread);
        [DllImport("kernel32.dll", SetLastError=true)] [return:MarshalAs(UnmanagedType.Bool)] internal static extern bool GetExitCodeProcess(IntPtr process, out uint exitCode);
        [DllImport("kernel32.dll", SetLastError=true)] [return:MarshalAs(UnmanagedType.Bool)] internal static extern bool CloseHandle(IntPtr handle);
        [DllImport("kernel32.dll", SetLastError=true, CharSet=CharSet.Unicode)] internal static extern IntPtr CreateFile(string name, uint access, uint share, ref SECURITY_ATTRIBUTES attributes, uint creation, uint flags, IntPtr template);
        [DllImport("kernel32.dll", SetLastError=true)] [return:MarshalAs(UnmanagedType.Bool)] internal static extern bool InitializeProcThreadAttributeList(IntPtr list, int count, uint flags, ref IntPtr size);
        [DllImport("kernel32.dll", SetLastError=true)] [return:MarshalAs(UnmanagedType.Bool)] internal static extern bool UpdateProcThreadAttribute(IntPtr list, uint flags, IntPtr attribute, IntPtr value, IntPtr size, IntPtr previous, IntPtr returnSize);
        [DllImport("kernel32.dll")] internal static extern void DeleteProcThreadAttributeList(IntPtr list);
        [DllImport("kernel32.dll", SetLastError=true, CharSet=CharSet.Unicode)] [return:MarshalAs(UnmanagedType.Bool)] internal static extern bool CreateProcess(string app, StringBuilder command, IntPtr processAttributes, IntPtr threadAttributes, bool inherit, uint flags, IntPtr environment, string currentDirectory, ref STARTUPINFOEX startup, out PROCESS_INFORMATION process);
    }
    public static class EntryPoint {
        public static int Main(string[] args) {
            if (args.Length != 3) { ConsoleProgress.Send("Expected pack, private results, optional install root"); return 2; }
            string pack = Paths.Local(args[0], true), output = Paths.Local(args[1], true);
            if (Path.GetDirectoryName(output) != pack || !Path.GetFileName(output).StartsWith("results-")) throw new IOException("Results must be a direct private child of this pack");
            try { return new CaptureSupervisor(output, new WindowsLauncher(), 660).Run(pack, args[2]); }
            catch (Exception e) {
                ConsoleProgress.Send("Diagnostic failed. COPY THIS FOLDER BACK: " + output + "\n" + e);
                try { File.WriteAllText(Path.Combine(output, "supervisor-error.txt"), e.ToString()); } catch { }
                return 2;
            }
        }
    }
}
