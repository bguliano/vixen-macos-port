#!/usr/bin/env python3
"""Offline C#5 compile and exact comparator controls. Never runs Windows drawing."""
import argparse,copy,hashlib,importlib.util,json,os,subprocess
from pathlib import Path
from xml.sax.saxutils import escape
ROOT=Path(__file__).resolve().parents[2];SRC=Path(__file__).resolve().parent

def main():
    p=argparse.ArgumentParser();p.add_argument('--out',type=Path,required=True);p.add_argument('--fixtures',type=Path,required=True);p.add_argument('--mac',type=Path,required=True);a=p.parse_args();out=a.out.resolve();out.mkdir()
    text='<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net10.0</TargetFramework><OutputType>Exe</OutputType><LangVersion>5</LangVersion><EnableDefaultCompileItems>false</EnableDefaultCompileItems><ImplicitUsings>disable</ImplicitUsings><Nullable>disable</Nullable><UseAppHost>false</UseAppHost><NoWarn>CA1416</NoWarn></PropertyGroup><ItemGroup>'
    for file in ['GifCapture.cs','OriginalPictureScale.cs']:text+='<Compile Include="'+escape(str(SRC/file))+'"/>'
    for dll in ['System.Drawing.Common','System.Private.Windows.Core','System.Private.Windows.GdiPlus']:text+='<Reference Include="'+dll+'"><HintPath>'+escape(str(ROOT/'target/golden/media/bin'/f'{dll}.dll'))+'</HintPath></Reference>'
    text+='</ItemGroup></Project>';project=out/'GifCapture.csproj';project.write_text(text)
    dotnet=ROOT/'target/golden/dotnet-root/dotnet';env=os.environ.copy();env.update(DOTNET_ROOT=str(dotnet.parent),DOTNET_CLI_HOME=str(out/'dotnet-home'),DOTNET_CLI_TELEMETRY_OPTOUT='1',DOTNET_SKIP_FIRST_TIME_EXPERIENCE='1',MSBuildEnableWorkloadResolver='false')
    with (out/'build.log').open('w') as log:subprocess.run([str(dotnet),'build',str(project),'-c','Release','--configfile',str(ROOT/'tools/windows-pcm/nuget.config'),'-p:UseSharedCompilation=false','-v:q'],env=env,check=True,timeout=90,stdout=log,stderr=subprocess.STDOUT)
    cmd=[str(dotnet),str(out/'bin/Release/net10.0/GifCapture.dll')]
    run=subprocess.run(cmd+['--schedules'],env=env,check=True,capture_output=True,text=True,timeout=5);(out/'schedules.json').write_text(run.stdout)
    contract=json.loads((a.fixtures/'cases.json').read_text());mac=json.loads(a.mac.read_text());checks=[]
    assert json.loads(run.stdout)==[{'id':c['id'],'schedule':c['expectedSourceSchedule']} for c in contract['cases']];checks.append('C# source schedule matches independent Python for all eight cases')
    refused=subprocess.run(cmd+[str(a.fixtures),'full-speed1',str(out/'must-not-exist.json')],env=env,capture_output=True,text=True,timeout=5);assert refused.returncode!=0 and not (out/'must-not-exist.json').exists();checks.append('non-Windows capture rejected without output')
    spec=importlib.util.spec_from_file_location('compare',SRC/'compare.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    baseline=[]
    for c,m in zip(contract['cases'],mac['cases']):baseline.append(dict(m,schema=1,id=c['id'],inputSha256=c['inputSha256'],installedEffectExecuted=False,delaysCentiseconds=[1,7,13,2],sourceSchedule=c['expectedSourceSchedule']))
    assert module.compare(contract,mac,baseline)['status']=='exact-within-scope';checks.append('synthetic comparator positive, explicitly not Windows evidence')
    mutated=copy.deepcopy(baseline);mutated[0]['decodedFrames'][0]['argb'][0]-=1
    report=module.compare(contract,mac,mutated);assert report['status']=='mismatch' and report['cases'][0]['decodedFrames']['channelMismatches']==1;checks.append('one alpha byte mismatch retained')
    for name in ['missing-frame','frame-index','wrong-input','schedule','effect-claim']:
        value=copy.deepcopy(baseline)
        if name=='missing-frame':value[0]['cache'].pop()
        elif name=='frame-index':value[0]['cache'][0]['effectFrame']=1
        elif name=='wrong-input':value[0]['inputSha256']='0'*64
        elif name=='schedule':value[0]['sourceSchedule'][0]['sourceFrame']=3
        elif name=='effect-claim':value[0]['installedEffectExecuted']=True
        try:module.compare(contract,mac,value)
        except AssertionError:checks.append(name+' rejected')
        else:raise AssertionError(name+' wrongly accepted')
    assert (SRC/'Run-Gif.ps1').read_bytes().isascii();checks.append('PowerShell source ASCII')
    original=(ROOT/'target/windows-gif-prepare-01/source/src__Vixen.Modules__Effect__Picture__Picture.cs').read_text(encoding='utf-8-sig');body=original[original.index('\t\tpublic static Bitmap ScaleImage('):original.index('\n\t\tpublic static Image ConvertToGrayScale')]
    assert body in (SRC/'OriginalPictureScale.cs').read_text();checks.append('ScaleImage method byte-for-byte after newline normalization')
    result={'passed':True,'checks':checks,'scope':'Offline compile/control proof only; Windows Framework/PowerShell/GDI execution pending','sourcePins':{f.name:hashlib.sha256(f.read_bytes()).hexdigest() for f in SRC.iterdir() if f.is_file()}}
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'passed':True,'checks':len(checks)}))
if __name__=='__main__':main()
