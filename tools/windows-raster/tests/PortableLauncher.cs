// Local test adapter only; never shipped as a Windows launcher.
using System;
using System.IO;
using System.Diagnostics;
using System.Runtime.InteropServices;
using VixenCapture;
public sealed class PortableLauncher : IStageLauncher {
    readonly string python, fixture;
    public PortableLauncher(string python, string fixture) { this.python=python; this.fixture=fixture; }
    public IStageProcess Start(StageSpec spec, string stdout, string stderr) {
        if (!File.Exists(spec.Executable)) throw new FileNotFoundException("fixture executable absent",spec.Executable);
        var start=new ProcessStartInfo(python) { UseShellExecute=false };
        foreach (var arg in new[]{fixture,"--exec",stdout,stderr,spec.Executable}) start.ArgumentList.Add(arg);
        foreach (var arg in spec.Arguments) start.ArgumentList.Add(arg);
        return new PortableChild(Process.Start(start));
    }
}
public sealed class PortableChild : IStageProcess {
    readonly Process process;
    readonly int pid;
    [DllImport("libSystem.B.dylib",SetLastError=true)] static extern int kill(int pid,int signal);
    public PortableChild(Process p) { process=p;pid=p.Id; }
    public int Id { get { return pid; } }
    public bool HasExited { get { return process.HasExited; } }
    public int ExitCode { get { return process.ExitCode; } }
    public bool Wait(int milliseconds) { return process.WaitForExit(milliseconds); }
    public void Terminate() {
        // Only the new session/process group owned by this private stage.
        if (pid<=1) throw new InvalidOperationException("invalid private pid");
        kill(-pid,9);
        if (!process.HasExited) process.Kill();
    }
    public void Dispose() { process.WaitForExit(2000);process.Dispose(); }
}
