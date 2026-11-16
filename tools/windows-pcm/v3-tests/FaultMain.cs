// Local adapter only: runs the production scheduler against real private
// subprocess failures. Windows process/job APIs are not exercised by this file.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text.Json;
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
public sealed class WorkflowLauncher : IStageLauncher {
    readonly PortableLauncher inner;
    readonly string python, fixture, root, inventoryMode;
    public WorkflowLauncher(string python,string fixture,string root,string inventoryMode) { inner=new PortableLauncher(python,fixture);this.python=python;this.fixture=fixture;this.root=root;this.inventoryMode=inventoryMode; }
    public IStageProcess Start(StageSpec requested,string stdout,string stderr) {
        string mode=requested.Name=="inventory"?(inventoryMode=="inventory-failed"?"nonzero":inventoryMode):(requested.Name=="package"?"nonzero":"success");
        if(requested.Name.StartsWith("runtimes-")) mode="runtime-net10";
        string worker=Path.Combine(root,"worker");Directory.CreateDirectory(worker);
        return inner.Start(new StageSpec{Name=requested.Name,Executable=python,Arguments=new[]{fixture,mode,worker},Seconds=requested.Seconds},stdout,stderr);
    }
}
public sealed class LegacyWorkflowLauncher : IStageLauncher {
    readonly PortableLauncher inner;
    readonly string python, fixture, root, scenario;
    public readonly List<StageSpec> Requested = new List<StageSpec>();
    public LegacyWorkflowLauncher(string python,string fixture,string root,string scenario) {
        inner=new PortableLauncher(python,fixture);this.python=python;this.fixture=fixture;this.root=root;this.scenario=scenario;
    }
    public IStageProcess Start(StageSpec requested,string stdout,string stderr) {
        Requested.Add(requested);
        if(requested.Name=="legacy-compile" && scenario=="compiler-missing") throw new FileNotFoundException("fixture missing Framework compiler",requested.Executable);
        string worker=Path.Combine(root,"worker");Directory.CreateDirectory(worker);
        return inner.Start(new StageSpec{Name=requested.Name,Executable=python,
            Arguments=new[]{fixture,"legacy-workflow",worker,scenario,requested.Name},Seconds=requested.Seconds},stdout,stderr);
    }
}
public sealed class PausedConsoleWriter : System.IO.TextWriter {
    public readonly System.Threading.ManualResetEvent Entered=new System.Threading.ManualResetEvent(false);
    public readonly System.Threading.ManualResetEvent Release=new System.Threading.ManualResetEvent(false);
    public override System.Text.Encoding Encoding { get { return System.Text.Encoding.UTF8; } }
    public override void WriteLine(string value) { Entered.Set();Release.WaitOne(); }
}
public static class FaultMain {
    public static int Main(string[] args) {
        string root=args[0], python=args[1], fixture=args[2];
        var launcher=new PortableLauncher(python,fixture);
        var reports=new Dictionary<string,object>();
        Func<string,double,CaptureSupervisor> supervisor=(name,budget)=>{
            string output=Path.Combine(root,name);Directory.CreateDirectory(output);
            return new CaptureSupervisor(output,launcher,budget);
        };
        Func<string,string,int,StageSpec> stage=(name,mode,seconds)=>{
            string output=Path.Combine(root,name,"worker");Directory.CreateDirectory(output);
            return new StageSpec{Name=name,Executable=python,Arguments=new[]{fixture,mode,output},Seconds=seconds};
        };
        foreach (var mode in new[]{"success","nonzero","hang","large","flood","malformed","descendant"}) {
            var s=supervisor(mode,15);
            var result=s.RunStage(stage(mode,mode,mode=="hang"?1:10));
            reports[mode]=result;
        }
        var missing=supervisor("missing",5);
        reports["missing"]=missing.RunStage(new StageSpec{Name="missing",Executable=Path.Combine(root,"absent.exe"),Seconds=2});
        var total=supervisor("overall",.35);
        reports["overall"]=total.RunStage(stage("overall","hang",10));
        reports["after-overall"]=total.RunStage(new StageSpec{Name="after-overall",Executable=python,Seconds=2});
        var recovery=supervisor("recovery",5);
        var fail=recovery.RunStage(new StageSpec{Name="initial-failure",Executable=Path.Combine(root,"absent.exe"),Seconds=2});
        reports["recovery"]=recovery.RunStage(stage("recovery","success",2));
        var checkpointFailure=supervisor("checkpoint-failure",5);
        Directory.CreateDirectory(Path.Combine(root,"checkpoint-failure","checkpoint-failure.stage.json.tmp"));
        reports["checkpoint-failure"]=checkpointFailure.RunStage(stage("checkpoint-failure","hang",3));
        string saved=Path.Combine(root,"preserved.json");
        CaptureSupervisor.Save(new InventoryChoice{reason="prior complete report"},saved);
        string prior=File.ReadAllText(saved);Directory.CreateDirectory(saved+".tmp");
        try{CaptureSupervisor.Save(new InventoryChoice{reason="replacement"},saved);}catch(IOException){}catch(UnauthorizedAccessException){}
        reports["priorCheckpointPreserved"]=File.ReadAllText(saved)==prior;
        foreach(var mode in new[]{"inventory","inventory-malformed","inventory-failed","inventory-empty-object","inventory-pcm-missing"}) {
            string workflow=Path.Combine(root,"workflow-"+mode);Directory.CreateDirectory(workflow);
            var running=new CaptureSupervisor(workflow,new WorkflowLauncher(python,fixture,workflow,mode),90);
            int code=running.Run(root,"");
            reports["workflow-"+mode]=new{exitCode=code,summary=running.Report,folderExists=Directory.Exists(workflow)};
        }
        foreach(var scenario in new[]{"success","runtime-missing","runtime-failed","no-host","prepare-failed","compiler-missing","compiler-failed","compiler-missing-output","compiler-empty-output","compiler-missing-runtimeconfig","contract-mismatch","preflight-failed","preflight-missing-report","pcm-failed","pcm-missing-manifest","integrity-failed","copy-failed","package-failed","input-failed","both-runtimes","strict-preferred","deadline-exhausted"}) {
            string workflow=Path.Combine(root,"legacy-"+scenario);Directory.CreateDirectory(workflow);
            var adapter=new LegacyWorkflowLauncher(python,fixture,workflow,scenario);
            var running=new CaptureSupervisor(workflow,adapter,scenario=="deadline-exhausted"?71:90);
            int code=running.Run(root,"");
            reports["legacy-"+scenario]=new{exitCode=code,summary=running.Report,requested=adapter.Requested,folderExists=Directory.Exists(workflow)};
        }
        var skipCases=new List<object>();
        foreach(var inventoryReady in new[]{false,true}) {
            foreach(var trusted in new[]{false,true}) {
                foreach(var counts in new[]{new[]{0,0},new[]{2,0},new[]{2,1},new[]{2,2}}) {
                    skipCases.Add(new{inventoryReady=inventoryReady,packTrusted=trusted,
                        hostCandidates=counts[0],successfulRuntimeListings=counts[1],
                        reason=CaptureSupervisor.OptionalProbeSkipReason(inventoryReady,trusted,counts[0],counts[1])});
                }
            }
        }
        reports["runtimeSkipCases"]=skipCases;
        bool rejected=false;
        try { CaptureSupervisor.ReadJson<InventoryChoice>(Path.Combine(root,"malformed","worker","worker.json"),65536); }
        catch { rejected=true; }
        reports["malformedRejected"]=rejected;
        string bom=Path.Combine(root,"choice-bom.json");
        File.WriteAllText(bom,"{\"selectedBin\":\"C:\\\\A é\\\\Vixen\",\"assemblyReady\":false,\"hosts\":[],\"reason\":\"ok\"}",new System.Text.UTF8Encoding(true));
        reports["bomChoice"]=CaptureSupervisor.ReadJson<InventoryChoice>(bom,65536);
        string tooBig=Path.Combine(root,"oversized.txt");File.WriteAllText(tooBig,new string('x',300));
        bool oversized=false;try{CaptureSupervisor.ReadText(tooBig,10);}catch(InvalidDataException){oversized=true;}
        reports["oversizedRejected"]=oversized;
        string[] values={"","plain","a b","C:\\folder\\","a\"b","a\\\"b","é path","line\nfeed"};
        var quotes=new List<object>();
        foreach(var value in values){try{quotes.Add(new{value=value,quoted=WindowsChild.Quote(value)});}catch(ArgumentException){quotes.Add(new{value=value,rejected=true});}}
        reports["quotes"]=quotes;
        var paused=supervisor("paused-console",5);
        var previousConsole=Console.Out;
        var blockedConsole=new PausedConsoleWriter();
        Console.SetOut(blockedConsole);
        var consoleWatch=Stopwatch.StartNew();
        using(var timer=new System.Threading.Timer(delegate(object ignored){blockedConsole.Release.Set();},null,2500,System.Threading.Timeout.Infinite)) {
            try {
                paused.Note("Testing a blocked console without blocking stage deadlines");
                bool entered=blockedConsole.Entered.WaitOne(1000);
                var completed=paused.RunStage(stage("paused-console","success",3));
                reports["paused-console"]=new{entered=entered,elapsed=consoleWatch.Elapsed.TotalSeconds,stage=completed};
            } finally { blockedConsole.Release.Set();Console.SetOut(previousConsole); }
        }
        reports["windowsNativeAdapterExecuted"]=false;
        File.WriteAllText(Path.Combine(root,"fault-results.json"),JsonSerializer.Serialize(reports,new JsonSerializerOptions{IncludeFields=true,WriteIndented=true}));
        return 0;
    }
}
