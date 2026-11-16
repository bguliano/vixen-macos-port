#!/usr/bin/env python3
"""Local C#5 syntax/helper/hash-refusal tests; never claims Windows/.NET8 decode."""
import argparse, hashlib, json, os, pathlib, shutil, subprocess


def digest(path):
    return hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--dotnet',type=pathlib.Path,required=True)
    p.add_argument('--host',type=pathlib.Path,required=True)
    p.add_argument('--assemblies',type=pathlib.Path,required=True)
    p.add_argument('--out',type=pathlib.Path,required=True)
    a=p.parse_args(); a.out=a.out.resolve();a.out.mkdir()
    host=a.host.resolve();dotnet=a.dotnet.resolve();assemblies=a.assemblies.resolve()
    before={x.name:digest(x) for x in assemblies.iterdir()}
    source=pathlib.Path(__file__).with_name('LegacyCapture.cs.in')
    results=[]
    env=dict(os.environ,DOTNET_ROOT=str(dotnet.parent),DOTNET_CLI_TELEMETRY_OPTOUT='1')
    def run(name,args,code,contains=None,report=True):
        r=subprocess.run([str(dotnet),str(host),*map(str,args)],env=env,capture_output=True,text=True,timeout=20)
        (a.out/(name+'.stdout.txt')).write_text(r.stdout);(a.out/(name+'.stderr.txt')).write_text(r.stderr)
        assert r.returncode==code,(name,r.returncode,r.stderr)
        if contains: assert contains in r.stdout+r.stderr,(name,r.stdout,r.stderr)
        if report: results.append({'name':name,'exitCode':r.returncode,'passed':True})
        return r
    run('help',['--help'],0,'Capture requires Windows x64/.NET 8')
    helper=json.loads(run('helpers',['--self-test'],0,'host-self-test-passed').stdout)
    contract=json.loads(run('contract',['--validate-contract','--vixen-bin',assemblies],0,'contract-matched').stdout)
    expected=json.loads(source.with_name('contract.json').read_text())
    assert [{k:f[k] for k in ('name','bytes','sha256')} for f in contract['contract']['files']]==expected['files']
    assert contract['assembliesLoaded'] is False
    run('malformed-cli',[],1,'ArgumentException')
    run('duplicate-cli',['--validate-contract','--vixen-bin',assemblies,'--vixen-bin',assemblies],1,'ArgumentException')
    run('missing-value',['--validate-contract','--vixen-bin'],1,'ArgumentException')
    run('wrong-mode-fields',['--preflight','--vixen-bin',assemblies,'--input','x'],1,'ArgumentException')
    private=a.out/'private';shutil.copytree(assemblies,private)
    for i,name in enumerate(before):
        file=private/name;content=file.read_bytes();changed=bytearray(content);changed[-1]^=1;file.write_bytes(changed)
        run('changed-'+name,['--validate-contract','--vixen-bin',private],1,'contract mismatch: '+name)
        file.write_bytes(content)
    removed=private/'Preferences.dll';saved=removed.read_bytes();removed.unlink()
    run('missing-dll',['--validate-contract','--vixen-bin',private],1,'exactly the nine audited DLLs')
    removed.write_bytes(saved)
    extra=private/'unexpected.dll';extra.write_bytes(b'not executable')
    run('extra-dll',['--validate-contract','--vixen-bin',private],1,'exactly the nine audited DLLs');extra.unlink()
    sub=private/'subdirectory';sub.mkdir()
    run('extra-directory',['--validate-contract','--vixen-bin',private],1,'exactly the nine audited DLLs');sub.rmdir()
    run('output-existing',['--preflight','--vixen-bin',assemblies,'--out',a.out],1,'Output already exists')
    run('output-inside-binaries',['--preflight','--vixen-bin',assemblies,'--out',assemblies/'forbidden'],1,'outside the supplied binary folder')
    run('output-parent-missing',['--preflight','--vixen-bin',assemblies,'--out',a.out/'missing'/'output'],1,'parent must already exist')
    link=a.out/'linked-binaries';link.symlink_to(assemblies,target_is_directory=True)
    run('symlink-input',['--validate-contract','--vixen-bin',link],1,'not links/junctions/cloud placeholders')
    run('nonwindows-capture',['--vixen-bin',assemblies,'--input',a.out/'unused.mp3','--out',a.out/'capture-refused'],1,'Actual capture requires Windows x64 running .NET 8')
    assert not (a.out/'capture-refused').exists()
    # The supplied Vixen PE binaries are x64. This Mac runs arm64, so metadata/IL
    # provides the API audit; loading the x64 AudioPlayer must be refused locally.
    preflight=run('preflight-platform-limit',['--preflight','--vixen-bin',assemblies,'--out',a.out/'preflight'],1,'assembly architecture is not compatible',report=False)
    assert not (a.out/'preflight').exists()
    after={x.name:digest(x) for x in assemblies.iterdir()};assert before==after
    report={'schema':1,'status':'passed-with-explicit-platform-limit','host':str(host),'hostSha256':digest(host),'hostSourceSha256':digest(source),'runtime':helper['runtime'],'checks':results,'hostHelperChecks':helper['checks'],'actualDllContractsMatched':len(contract['contract']['files']),'actualInputDllsUnchanged':True,'preflightLimit':{'exitCode':preflight.returncode,'reason':'Actual returned AudioPlayer PE architecture x64 cannot load on this arm64 Mac. No decoder was constructed. Static metadata/IL audit supplies local API evidence.'},'notVerified':['Windows Framework csc compilation','Windows .NET8 API preflight','Windows Media Foundation PCM capture']}
    (a.out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'status':report['status'],'subprocessChecks':len(results),'helperChecks':helper['checks'],'result':str(a.out/'result.json')}))

if __name__=='__main__':main()
