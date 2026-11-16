from pathlib import Path
import os,subprocess,xml.sax.saxutils as xml,json,sys,hashlib,argparse
parser=argparse.ArgumentParser();parser.add_argument('--out',type=Path,required=True);args=parser.parse_args();R=Path(__file__).resolve().parents[2];S=Path(__file__).resolve().parent;O=args.out.resolve();assert O.is_relative_to(R/'target') and not O.exists();O.mkdir(parents=True);dotnet=R/'target/golden/dotnet-root/dotnet';env=os.environ.copy();env.update(DOTNET_ROOT=str(dotnet.parent),DOTNET_CLI_HOME=str(O/'cli-home'),DOTNET_GENERATE_ASPNET_CERTIFICATE='false',DOTNET_CLI_TELEMETRY_OPTOUT='1',DOTNET_SKIP_FIRST_TIME_EXPERIENCE='1',DOTNET_CLI_WORKLOAD_UPDATE_NOTIFY_DISABLE='1',DOTNET_NOLOGO='1',MSBuildEnableWorkloadResolver='false')
outputs={}
for name,files,entry,drawing in [('Capture',['Shared.cs.in','OriginalPictureMethods.cs.in','RasterCapture.cs.in'],'RasterCapture',True),('Supervisor',['Shared.cs.in','SupervisorEngine.cs.in','RasterWorkflow.cs.in'],'VixenCapture.RasterEntry',False)]:
 folder=O/name;folder.mkdir();text='<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net10.0</TargetFramework><OutputType>Exe</OutputType><LangVersion>5</LangVersion><EnableDefaultCompileItems>false</EnableDefaultCompileItems><ImplicitUsings>disable</ImplicitUsings><Nullable>disable</Nullable><UseAppHost>false</UseAppHost><NoWarn>CA1416;CS0414</NoWarn><StartupObject>'+entry+'</StartupObject></PropertyGroup><ItemGroup>'
 for n in files:text+='<Compile Include="'+xml.escape(str(S/n),{'"':'&quot;'})+'"/>'
 if drawing:
  for dll in ['System.Drawing.Common','System.Private.Windows.Core','System.Private.Windows.GdiPlus']:text+='<Reference Include="'+dll+'"><HintPath>'+xml.escape(str(R/'target/golden/media/bin'/f'{dll}.dll'))+'</HintPath></Reference>'
 text+='</ItemGroup></Project>';project=folder/(name+'.csproj');project.write_text(text)
 with (folder/'build.log').open('w') as log:subprocess.run([str(dotnet),'build',str(project),'-c','Release','--configfile',str(R/'tools/windows-pcm/nuget.config'),'-p:UseSharedCompilation=false','-v:q'],env=env,check=True,timeout=90,stdout=log,stderr=subprocess.STDOUT)
 outputs[name]=str(folder/'bin/Release/net10.0'/f'{name}.dll')
run=subprocess.run([str(dotnet),outputs['Capture'],'--emit-cases'],env=env,check=True,capture_output=True,text=True,timeout=10);assert json.loads(run.stdout)==json.loads((S/'cases.json').read_text())
subprocess.run([str(dotnet),outputs['Capture'],'--validate-cases',str(S/'cases.json')],env=env,check=True,timeout=10)
sys.path.insert(0,str(S));from contract import cases,requested
readback=subprocess.run([str(dotnet),outputs['Capture'],'--emit-input-digests'],env=env,check=True,capture_output=True,text=True,timeout=10)
assert readback.stdout.splitlines()==[c['id']+' '+hashlib.sha256(requested(c)).hexdigest() for c in cases()['cases']]
(O/'build-result.json').write_text(json.dumps(dict(passed=True,outputs=outputs,windowsDrawingExecuted=False,windowsFrameworkCompilerExecuted=False,windowsJobObjectsExecuted=False,certificateGenerationDisabled=True,scope='Offline cached net10 references and C#5 syntax; emitted/validated exact case contract only.'),indent=2)+'\n');print(json.dumps(dict(passed=True,outputs=outputs)))
