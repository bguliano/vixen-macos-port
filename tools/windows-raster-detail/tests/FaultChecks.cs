// Local net10 test driver. No Windows process API, PowerShell or drawing executes.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Text.Json;
using VixenCapture;
using VixenRaster;

public sealed class Immediate : IStageProcess {
    readonly int code;
    public Immediate(int code){this.code=code;}
    public int Id {get{return 0;}}
    public bool HasExited {get{return true;}}
    public int ExitCode {get{return code;}}
    public bool Wait(int milliseconds){return true;}
    public void Terminate(){}
    public void Dispose(){}
}
public sealed class FakeWorkflow : IStageLauncher {
    readonly string pack,output,scenario;
    public readonly List<StageSpec> requested=new List<StageSpec>();
    public FakeWorkflow(string pack,string output,string scenario){this.pack=pack;this.output=output;this.scenario=scenario;}
    public IStageProcess Start(StageSpec stage,string stdout,string stderr) {
        requested.Add(stage);File.WriteAllText(stdout,"");File.WriteAllText(stderr,"");
        string host=Path.Combine(output,"RasterCapture.exe");
        if(stage.Name=="integrity" && scenario=="integrity-fail") return new Immediate(2);
        if(stage.Name=="integrity-final" && scenario=="integrity-final-fail") return new Immediate(2);
        if(stage.Name=="compile-raster") {
            if(scenario=="compiler-missing")throw new FileNotFoundException("test missing compiler");
            if(scenario=="compiler-fail")return new Immediate(2);
            if(scenario!="compiler-missing-output")File.WriteAllText(host,scenario=="compiler-empty-output"?"":"synthetic host; never executable");
        }
        if(stage.Name=="preflight") {
            if(scenario=="preflight-fail")return new Immediate(2);
            if(scenario=="preflight-corrupt")File.WriteAllText(Path.Combine(output,"preflight.json"),"{");
            else if(scenario!="preflight-missing")Data.SaveNew(Path.Combine(output,"preflight.json"),new RuntimeReport{status="available",hostSha256=Data.HashFile(host),gdiplusSha256=new string('a',64),drawingSha256=new string('b',64)});
            if(scenario=="source-vanished")File.Delete(Path.Combine(pack,"OriginalPictureMethods.cs.in"));
            if(scenario=="cases-directory-blocked")File.WriteAllText(Path.Combine(output,"cases"),"blocked directory fixture");
        }
        if(stage.Name.StartsWith("case-")) {
            string id=stage.Name.Substring(5);CaseSpec spec=null;foreach(var c in Data.Cases().cases)if(c.id==id)spec=c;
            bool first=id==Data.Cases().cases[0].id;string directory=Path.Combine(output,"cases",id);
            if(first && scenario=="case-fail")return new Immediate(23);
            if(first && scenario=="case-missing")return new Immediate(0);
            if(first && scenario=="case-corrupt") {File.WriteAllText(Path.Combine(directory,"manifest.json"),"{");return new Immediate(0);}
            byte[] pixels=Data.Requested(spec);var files=new List<FileRecord>();
            int width=spec.operation=="resize-exact"?spec.maxWidth:spec.width,height=spec.operation=="resize-exact"?spec.maxHeight:spec.height;
            Action<string,byte[]> write=delegate(string name,byte[] bytes){Data.WriteNew(Path.Combine(directory,name),bytes);files.Add(Data.Record(directory,name));};
            foreach(string name in new[]{"input-requested.argb","input-getpixel.argb","input-fastpixel.argb"})write(name,pixels);
            foreach(string name in new[]{"output-getpixel.argb","output-fastpixel.argb"})write(name,new byte[width*height*4]);
            bool ellipse=spec.operation=="ellipse";
            if(ellipse) {
                string[] labels={"original","flat025","flat050","flat100"};float[] flatness={0,0.25f,0.5f,1};var traces=new List<PathObservation>();
                for(int i=0;i<4;i++) {
                    write("path-"+labels[i]+"-getpixel.argb",pixels);write("path-"+labels[i]+"-fastpixel.argb",pixels);
                    traces.Add(new PathObservation{label=labels[i],flatness=flatness[i],points=new float[8],pointBits=new int[8],types=new[]{0,1,1,129},drawingDefaults=new DrawingDefaults{transform=new float[]{1,0,0,1,0,0}},penColor="FFFFFFFF",penWidth=1,penStartCap="Flat",penEndCap="Flat",penLineJoin="Miter",penAlignment="Center",penMiterLimit=10,fillMode="Alternate"});
                }
                if(id=="detail-ellipse-1") {
                    if(scenario=="case-path-bits")traces[1].pointBits[2]=1;
                    if(scenario=="case-path-state")traces[1].drawingDefaults=null;
                    if(scenario=="case-path-types")traces[1].types[1]=99;
                    if(scenario=="case-path-label")traces[1].label="unknown";
                    if(scenario=="case-path-missing")files.RemoveAt(files.Count-1);
                    if(scenario=="case-path-readback") {string name="path-original-fastpixel.argb";var changed=new byte[pixels.Length];changed[0]=1;File.WriteAllBytes(Path.Combine(directory,name),changed);for(int n=0;n<files.Count;n++)if(files[n].path==name)files[n]=Data.Record(directory,name);}
                }
                write("path-trace.json",Data.Json(new PathTraceReport{id=id,paths=traces.ToArray()}));
            }
            var report=new RasterReport{status="complete",id=id,input=spec,outputWidth=width,outputHeight=height,inputStride=spec.width*4,outputStride=width*4,inputPixelFormat="Format32bppArgb",outputPixelFormat="Format32bppArgb",inputUnchanged=true,inputReadbacksEqual=true,outputReadbacksEqual=true,files=files.ToArray(),hostSha256=Data.HashFile(host),originalMethodsSha256=Data.HashFile(Path.Combine(pack,"OriginalPictureMethods.cs.in")),countDownMethodsSha256=Data.HashFile(Path.Combine(pack,"OriginalCountDownMethods.cs.in")),fastPixelSha256=Data.HashFile(Path.Combine(pack,"OriginalFastPixel.cs.in")),freshGraphicsDefaults=new DrawingDefaults(),actualDrawingDefaults=spec.operation=="grayscale"?null:new DrawingDefaults{transform=new float[]{1,0,0,1,0,0},clipBounds=new float[]{0,0,40,40},visibleClipBounds=new float[]{0,0,40,40}},drawingStateScope=spec.operation=="resize-exact"?"actual-before-original-picture-drawimage-overload":spec.family=="picture"?"fresh-defaults-only-original-picture-body-uninstrumented":"actual-before-identity-transform-fixture"};
            if(first) {
                if(scenario=="case-hash")report.files[0].sha256=new string('f',64);
                if(scenario=="case-traversal")report.files[0].path="../unrelated";
                if(scenario=="case-duplicate")report.files[1]=report.files[0];
                if(scenario=="case-mutation")report.inputUnchanged=false;
                if(scenario=="case-readback")report.outputReadbacksEqual=false;
                if(scenario=="case-state-scope")report.drawingStateScope="unverified-state";
                if(scenario=="case-countdown-source")report.countDownMethodsSha256=new string('f',64);
                if(scenario=="case-host")report.hostSha256=new string('f',64);
                if(scenario=="case-size")report.outputWidth=65;
                if(scenario=="case-input") {File.WriteAllBytes(Path.Combine(directory,"input-requested.argb"),new byte[pixels.Length]);report.files[0]=Data.Record(directory,"input-requested.argb");}
            }
            if(scenario=="case-drawing-state" && id=="detail-ellipse-1")report.actualDrawingDefaults=null;
            Data.SaveNew(Path.Combine(directory,"manifest.json"),report);
        }
        if(stage.Name=="package") {
            if(scenario=="package-fail")return new Immediate(2);
            File.WriteAllText(output+".zip","synthetic archive; no Windows packaging executed");
        }
        return new Immediate(0);
    }
}
public static class FaultChecks {
    public static int Main(string[] args) {
        string root=args[0],python=args[1],fixture=args[2],source=args[3];
        var result=new Dictionary<string,object>();var launcher=new PortableLauncher(python,fixture);
        foreach(string mode in new[]{"success","nonzero","hang","large","flood","descendant"}) {
            string dir=Path.Combine(root,"stage-"+mode);Directory.CreateDirectory(dir);string worker=Path.Combine(dir,"worker");Directory.CreateDirectory(worker);
            var supervisor=new CaptureSupervisor(dir,launcher,10);
            result[mode]=supervisor.RunStage(new StageSpec{Name=mode,Executable=python,Arguments=new[]{fixture,mode,worker},Seconds=mode=="hang"?1:6});
        }
        string totalDir=Path.Combine(root,"overall");Directory.CreateDirectory(totalDir);
        var total=new CaptureSupervisor(totalDir,launcher,0.3);
        result["overall"]=total.RunStage(new StageSpec{Name="overall",Executable=python,Arguments=new[]{fixture,"hang",totalDir},Seconds=10});
        result["after-overall"]=total.RunStage(new StageSpec{Name="after-overall",Executable=python,Seconds=1});
        foreach(string scenario in new[]{"success","integrity-fail","compiler-missing","compiler-fail","compiler-missing-output","compiler-empty-output","preflight-fail","preflight-corrupt","preflight-missing","source-vanished","cases-directory-blocked","case-fail","case-missing","case-corrupt","case-hash","case-traversal","case-duplicate","case-mutation","case-readback","case-host","case-countdown-source","case-state-scope","case-drawing-state","case-path-bits","case-path-state","case-path-types","case-path-label","case-path-missing","case-path-readback","case-size","case-input","integrity-final-fail","package-fail","reserve"}) {
            string pack=Path.Combine(root,"workflow-"+scenario);Directory.CreateDirectory(pack);
            foreach(string name in new[]{"OriginalPictureMethods.cs.in","OriginalCountDownMethods.cs.in","OriginalFastPixel.cs.in","cases.json"})File.Copy(Path.Combine(source,name),Path.Combine(pack,name));
            string output=Path.Combine(pack,"results-test");Directory.CreateDirectory(output);
            var fake=new FakeWorkflow(pack,output,scenario);
            var workflow=new RasterWorkflow(pack,output,Process.GetCurrentProcess().MainModule.FileName,"unused-powershell",fake,scenario=="reserve"?69:90);
            int code=workflow.Run();result["workflow-"+scenario]=new{exitCode=code,summary=workflow.Report,requested=fake.requested,folderExists=Directory.Exists(output)};
        }
        result["windowsDrawingExecuted"]=false;result["windowsNativeLauncherExecuted"]=false;result["powershellExecuted"]=false;
        File.WriteAllText(Path.Combine(root,"fault-results.json"),JsonSerializer.Serialize(result,new JsonSerializerOptions{IncludeFields=true,WriteIndented=true}));
        return 0;
    }
}
