#!/usr/bin/env python3
"""Offline C#5 build/common-code and bounded fault checks; never executes GDI.

Uses cached net10 reference assemblies. Actual Windows Framework compilation,
PowerShell and Job Objects remain Windows verification requirements.
"""
import argparse,copy,hashlib,json,os,subprocess,sys
from pathlib import Path
import xml.sax.saxutils as xml
ROOT=Path(__file__).resolve().parents[2];SOURCE=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);a=p.parse_args();out=a.out.resolve()
    assert out.is_relative_to(ROOT/'target') and not out.exists();out.mkdir(parents=True)
    inputs={str(f.relative_to(ROOT)):sha(f) for f in SOURCE.rglob('*') if f.is_file() and '__pycache__' not in f.parts}
    subprocess.run([sys.executable,str(SOURCE/'extract_sources.py'),'--check'],check=True,timeout=10,capture_output=True)
    subprocess.run([sys.executable,str(SOURCE/'generate_cases.py'),'--check'],check=True,timeout=10,capture_output=True)
    dotnet=ROOT/'target/golden/dotnet-root/dotnet';env=os.environ.copy()
    env.update(DOTNET_ROOT=str(dotnet.parent),DOTNET_CLI_HOME=str(ROOT/'target/golden/cli-home'),DOTNET_CLI_TELEMETRY_OPTOUT='1',DOTNET_SKIP_FIRST_TIME_EXPERIENCE='1',MSBuildEnableWorkloadResolver='false')
    def build(name,files,entry,drawing=False):
        folder=out/name;folder.mkdir()
        text='<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net10.0</TargetFramework><OutputType>Exe</OutputType><LangVersion>5</LangVersion><EnableDefaultCompileItems>false</EnableDefaultCompileItems><ImplicitUsings>disable</ImplicitUsings><Nullable>disable</Nullable><UseAppHost>false</UseAppHost><NoWarn>CA1416;CS0414</NoWarn><StartupObject>'+entry+'</StartupObject></PropertyGroup><ItemGroup>'
        for file in files:text+='<Compile Include="'+xml.escape(str(SOURCE/file),{'"':'&quot;'})+'"/>'
        if drawing:
            for dll in ['System.Drawing.Common','System.Private.Windows.Core','System.Private.Windows.GdiPlus']:text+='<Reference Include="'+dll+'"><HintPath>'+xml.escape(str(ROOT/'target/golden/media/bin'/f'{dll}.dll'))+'</HintPath></Reference>'
        text+='</ItemGroup></Project>'
        project=folder/f'{name}.csproj';project.write_text(text)
        with (folder/'build.log').open('w') as log:subprocess.run([str(dotnet),'build',str(project),'-c','Release','--configfile',str(ROOT/'tools/windows-pcm/nuget.config'),'-p:UseSharedCompilation=false','-v:q'],env=env,check=True,timeout=90,stdout=log,stderr=subprocess.STDOUT)
        return folder/'bin/Release/net10.0'/f'{name}.dll'
    capture=build('Capture',['Shared.cs.in','OriginalPictureMethods.cs.in','OriginalCountDownMethods.cs.in','OriginalFastPixel.cs.in','RasterCapture.cs.in'],'RasterCapture',True)
    supervisor=build('Supervisor',['Shared.cs.in','SupervisorEngine.cs.in','RasterWorkflow.cs.in'],'VixenCapture.RasterEntry')
    faults=build('Faults',['Shared.cs.in','SupervisorEngine.cs.in','RasterWorkflow.cs.in','tests/PortableLauncher.cs','tests/FaultChecks.cs'],'FaultChecks')
    checks=[]
    def check(name,condition):checks.append(name);assert condition,name
    def run_contract(*args):return subprocess.run([str(dotnet),str(capture),*args],env=env,capture_output=True,text=True,timeout=10)
    reference=json.loads((SOURCE/'cases.json').read_text());emitted=run_contract('--emit-cases')
    check('C# grid agrees with independent JSON',emitted.returncode==0 and json.loads(emitted.stdout)==reference)
    check('valid case grid accepted without drawing',run_contract('--validate-cases',str(SOURCE/'cases.json')).returncode==0)
    for mode in ['missing','duplicate','dimension','scale','operation','extra','family','rect','line','points','fillmode','angle','length','schema','alpha','swatch','pattern']:
        negative=copy.deepcopy(reference)
        if mode=='missing':negative['cases'].pop()
        elif mode=='duplicate':negative['cases'][1]=negative['cases'][0]
        elif mode=='dimension':negative['cases'][0]['width']=64
        elif mode=='scale':negative['cases'][0]['scale']=.25
        elif mode=='operation':negative['cases'][0]['operation']='external-file'
        elif mode=='extra':negative['cases'].append(negative['cases'][0])
        elif mode=='family':negative['cases'][64]['family']='picture'
        elif mode=='rect':negative['cases'][64]['rect'][0]=99999
        elif mode=='line':negative['cases'][72]['line'].pop()
        elif mode=='points':negative['cases'][-1]['points']=[0,1]
        elif mode=='fillmode':negative['cases'][-1]['fillMode']='Unknown'
        elif mode=='angle':negative['cases'][40]['angle']+=1
        elif mode=='length':negative['cases'][40]['lineLength']=-1
        elif mode=='schema':negative['schema']=1
        elif mode=='alpha':negative['cases'][60]['constantAlpha']=256
        elif mode=='swatch':negative['cases'][60]['swatches'][0]=1
        elif mode=='pattern':negative['cases'][60]['inputPattern']='unknown'
        file=out/f'negative-{mode}.json';file.write_text(json.dumps(negative));check('invalid grid '+mode+' rejected',run_contract('--validate-cases',str(file)).returncode!=0)
    work=out/'fault cases é';work.mkdir();fixture=ROOT/'tools/windows-pcm/v3-tests/fault_child.py';fixture_hash=sha(fixture)
    with (out/'faults.log').open('w') as log:subprocess.run([str(dotnet),str(faults),str(work),sys.executable,str(fixture),str(SOURCE)],env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=60)
    results=json.loads((work/'fault-results.json').read_text())
    check('real subprocess normal',results['success']['status']=='completed' and results['success']['exitCode']==0)
    check('real subprocess nonzero partial retained',results['nonzero']['exitCode']==23 and (work/'stage-nonzero/worker/partial.txt').exists())
    check('real subprocess hang bounded',results['hang']['timedOut'] and results['hang']['elapsedSeconds']<4)
    for name in ['large','flood']:check('real '+name+' stream cap',results[name]['status']=='output-limit' and results[name]['elapsedSeconds']<8)
    check('descendant handles cannot stall',results['descendant']['status']=='completed' and results['descendant']['elapsedSeconds']<4)
    check('overall bound',results['overall']['timedOut'] and results['overall']['overallExpired'] and results['after-overall']['status']=='skipped-overall-deadline')
    for name,value in results.items():
        if not name.startswith('workflow-'):continue
        scenario=name.removeprefix('workflow-');summary=value['summary'];stages=summary['stages']
        check(scenario+' preserves folder and cases',value['folderExists'] and len(summary['cases'])==80)
        check(scenario+' attempts final integrity/package',[s['name'] for s in stages][-2:]==['integrity-final','package'])
        if scenario.startswith(('compiler-','preflight-')) or scenario in ['integrity-fail','reserve','source-vanished','cases-directory-blocked']:
            check(scenario+' blocks dependent capture',summary['completedCases']==0 and all(c['status']=='skipped-dependency' for c in summary['cases']))
        elif scenario.startswith('case-'):
            failed=64 if scenario=='case-drawing-state' or scenario.startswith('case-path-') else 0
            check(scenario+' fails only affected case',summary['completedCases']==79 and [i for i,c in enumerate(summary['cases']) if c['status']!='captured']==[failed])
        else:check(scenario+' complete capture count',summary['completedCases']==80)
        expected_complete=scenario in ['success','package-fail']
        check(scenario+' honest capture status',summary['status']==('captured-comparison-pending' if expected_complete else 'partial-or-diagnostics-only'))
        check(scenario+' exit status',value['exitCode']==(0 if scenario=='success' else 2))
        check(scenario+' bounded compiler/cases',all(s['Seconds']==(30 if s['Name']=='compile-raster' else 10) for s in value['requested'] if s['Name']=='compile-raster' or s['Name'].startswith('case-')))
        for stage in value['requested']:
            if stage['Name']=='compile-raster':
                check(scenario+' explicit Framework references',set(arg for arg in stage['Arguments'] if arg.startswith('/reference:'))=={'/reference:'+name+'.dll' for name in ['System','System.Core','System.Xml','System.Drawing','System.Runtime.Serialization']})
        check(scenario+' flag describes validated captures',summary['validWindowsRasterCaptured']==(summary['completedCases']>0) and not summary['comparisonPerformed'])
    check('exact source inputs unchanged',inputs=={str(f.relative_to(ROOT)):sha(f) for f in SOURCE.rglob('*') if f.is_file() and '__pycache__' not in f.parts})
    check('external fault subprocess source unchanged',fixture_hash==sha(fixture))
    report={'passed':True,'checks':checks,'sourceHashes':inputs,'externalTestInputs':{str(fixture.relative_to(ROOT)):fixture_hash},'captureDllSha256':sha(capture),'supervisorDllSha256':sha(supervisor),'faultDllSha256':sha(faults),'windowsDrawingExecuted':False,'frameworkCompilerExecuted':False,'powershellExecuted':False,'windowsJobObjectsExecuted':False,'scope':'C#5 syntax/common-code using cached net10 references; real local fault subprocesses plus injected workflow outcomes. No Windows raster/Framework/PowerShell execution claim.'}
    (out/'result.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'checks':len(checks),'result':str(out/'result.json')}))
if __name__=='__main__':main()
