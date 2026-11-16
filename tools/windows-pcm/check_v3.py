#!/usr/bin/env python3
"""Run actual local fault subprocesses against the v3 production supervisor core.

This does not execute Windows Job Objects, Windows PowerShell or Media Foundation.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import xml.sax.saxutils as xml

ROOT=Path(__file__).resolve().parents[2]


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dotnet',type=Path,default=ROOT/'target/golden/dotnet-root/dotnet')
    parser.add_argument('--out',type=Path,required=True)
    args=parser.parse_args()
    out=args.out.resolve()
    assert out.is_relative_to(ROOT/'target') and not out.exists()
    out.mkdir(parents=True)
    source=ROOT/'tools/windows-pcm/Supervisor.cs'
    driver=ROOT/'tools/windows-pcm/v3-tests/FaultMain.cs'
    fixture=ROOT/'tools/windows-pcm/v3-tests/fault_child.py'
    worker=ROOT/'tools/windows-pcm/Capture-Worker.ps1'
    sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
    hashes={str(p.relative_to(ROOT)):sha(p) for p in (source,driver,fixture,worker)}
    (out/'inputs.json').write_text(json.dumps(hashes,indent=2)+'\n')
    project=out/'FaultChecks.csproj'
    project.write_text('''<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup>
<OutputType>Exe</OutputType><TargetFramework>net10.0</TargetFramework><LangVersion>5</LangVersion><UseAppHost>false</UseAppHost>
<EnableDefaultCompileItems>false</EnableDefaultCompileItems><StartupObject>FaultMain</StartupObject>
<OutputPath>bin/</OutputPath><BaseIntermediateOutputPath>obj/</BaseIntermediateOutputPath>
</PropertyGroup><ItemGroup>'''+''.join('<Compile Include="'+xml.escape(str(p),{'"':'&quot;'})+'" />' for p in (source,driver))+'</ItemGroup></Project>')
    env=os.environ.copy()
    dotnet=args.dotnet.resolve()
    env.update(DOTNET_ROOT=str(dotnet.parent),DOTNET_CLI_HOME=str(ROOT/'target/golden/cli-home'),
               DOTNET_CLI_TELEMETRY_OPTOUT='1',MSBuildEnableWorkloadResolver='false')
    with (out/'build.log').open('w') as log:
        subprocess.run([str(dotnet),'build',str(project),'-c','Release','--configfile',str(ROOT/'tools/windows-pcm/nuget.config'),'-p:UseSharedCompilation=false'],env=env,stdout=log,stderr=subprocess.STDOUT,timeout=90,check=True)
    work=out/'fault cases é';work.mkdir()
    with (out/'run.log').open('w') as log:
        subprocess.run([str(dotnet),str(out/'bin/net10.0/FaultChecks.dll'),str(work),sys.executable,str(fixture)],env=env,stdout=log,stderr=subprocess.STDOUT,timeout=60,check=True)
    data=json.loads((work/'fault-results.json').read_text())
    checks=[]
    def check(name,condition):
        assert condition,name
        checks.append(name)
    worker_source=worker.read_text()
    legacy_prepare=worker_source[worker_source.index("if ($Stage -eq 'legacy-prepare')"):worker_source.index("if ($Stage -eq 'integrity'")]
    check('legacy-worker-config-schema-source-guard',all(x in legacy_prepare for x in ("-ne 'net8.0'","-ne 'LatestPatch'","-ne 'Microsoft.NETCore.App'","-ne '8.0.0'",'Properties.Name).Count -ne 1','Properties.Name).Count -ne 3','Properties.Name).Count -ne 2')))
    check('legacy-worker-config-bounded-before-parse',legacy_prepare.index('.Length -gt 65536')<legacy_prepare.index('ConvertFrom-Json'))
    check('legacy-worker-creates-only-fresh-host',legacy_prepare.index('Legacy host directory already exists')<legacy_prepare.index('CreateDirectory($hostDirectory)'))
    check('legacy-worker-copies-source-with-size-and-hash-checks','.Length -gt 1048576' in legacy_prepare and '[IO.File]::Copy($source, $destination, $false)' in legacy_prepare and 'Legacy host copy hash mismatch' in legacy_prepare)
    check('ordinary-success',data['success']['status']=='completed' and data['success']['exitCode']==0)
    check('separate-stdout-stderr',(work/'success/success.stdout.txt').read_text()=='stdout-ready\n' and (work/'success/success.stderr.txt').read_text()=='stderr-ready\n')
    check('nonzero-retains-partial',data['nonzero']['status']=='failed' and data['nonzero']['exitCode']==23 and (work/'nonzero/worker/partial.txt').read_text()=='retained before failure')
    check('hang-timeout',data['hang']['status']=='timed-out' and data['hang']['timedOut'] and data['hang']['elapsedSeconds']<4)
    check('hang-partial-retained',(work/'hang/worker/worker.json').read_text()=='{"status":"running"')
    check('large-two-stream-output',data['large']['status']=='completed' and all((work/f'large/large.{stream}.txt').stat().st_size==2*1024*1024 for stream in ('stdout','stderr')))
    check('output-flood-stopped',data['flood']['status']=='output-limit' and data['flood']['outputLimit'] and data['flood']['elapsedSeconds']<8)
    check('checkpoint-write-failure-bounded',data['checkpoint-failure']['status']=='failed' and data['checkpoint-failure']['elapsedSeconds']<4)
    check('prior-checkpoint-survives-write-error',data['priorCheckpointPreserved'])
    for mode in ('inventory','inventory-malformed','inventory-failed','inventory-empty-object'):
        workflow=data['workflow-'+mode]
        check(mode+'-zip-failure-folder-preserved',workflow['exitCode']==2 and workflow['folderExists'] and workflow['summary']['package']=='failed')
        stages=workflow['summary']['stages']
        check(mode+'-remaining-stages-ran',[s['name'] for s in stages]==['inventory','reference-copy','integrity','framework-gdi','input-recheck','package'] and all(s['status']=='completed' for s in stages[2:-1]) and stages[0]['status']==('failed' if mode=='inventory-failed' else 'completed'))
        check(mode+'-copy-dependency-guard',stages[1]['status']==('completed' if mode=='inventory' else 'skipped-dependency'))
        expected='skipped-no-runtime-host' if mode=='inventory' else 'skipped-inventory-unavailable'
        check(mode+'-optional-skip-keeps-unknown-runtime',workflow['summary']['pcm']==expected and workflow['summary']['gdiNet10']==expected)
    vanished=data['workflow-inventory-pcm-missing']
    vanished_stages=vanished['summary']['stages']
    check('vanished-install-preflight-reported',vanished['summary']['pcm']=='failed-preflight')
    check('vanished-install-still-checks-input-and-packages',[s['name'] for s in vanished_stages]==['inventory','reference-copy','integrity','runtimes-0','framework-gdi','net10-gdi','input-recheck','package'] and all(s['status']=='completed' for s in vanished_stages[:-1]) and vanished_stages[-1]['status']=='failed')
    check('vanished-install-folder-preserved',vanished['folderExists'] and vanished['exitCode']==2)
    legacy_prefix=['inventory','reference-copy','integrity','runtimes-0','framework-gdi']
    legacy_stages=['legacy-prepare','legacy-compile','legacy-contract','legacy-preflight','pcm']
    legacy_failures={'prepare-failed':('legacy-prepare','skipped-legacy-prepare-failed'),
                     'compiler-missing':('legacy-compile','skipped-legacy-compile-failed'),
                     'compiler-failed':('legacy-compile','skipped-legacy-compile-failed'),
                     'compiler-missing-output':('legacy-compile','failed-legacy-compiler-output'),
                     'compiler-empty-output':('legacy-compile','failed-legacy-compiler-output'),
                     'compiler-missing-runtimeconfig':('legacy-compile','failed-legacy-compiler-output'),
                     'contract-mismatch':('legacy-contract','skipped-legacy-contract-failed'),
                     'preflight-failed':('legacy-preflight','skipped-legacy-preflight-failed'),
                     'preflight-missing-report':('legacy-preflight','failed-legacy-preflight-missing-report'),
                     'pcm-failed':('pcm','failed'),
                     'pcm-missing-manifest':('pcm','failed-missing-manifest'),
                     'copy-failed':('legacy-contract','skipped-legacy-contract-failed')}
    for scenario in ('success','runtime-missing','runtime-failed','no-host','prepare-failed','compiler-missing','compiler-failed','compiler-missing-output','compiler-empty-output','compiler-missing-runtimeconfig','contract-mismatch','preflight-failed','preflight-missing-report','pcm-failed','pcm-missing-manifest','integrity-failed','copy-failed','package-failed','input-failed','both-runtimes','strict-preferred','deadline-exhausted'):
        workflow=data['legacy-'+scenario]; summary=workflow['summary']; stages=summary['stages']; names=[s['name'] for s in stages]
        check('legacy-'+scenario+'-final-integrity-and-package-attempted',names[-2:]==['input-recheck','package'])
        check('legacy-'+scenario+'-folder-preserved',workflow['folderExists'])
        check('legacy-'+scenario+'-package-outcome',summary['package']==('failed' if scenario=='package-failed' else 'completed') and workflow['exitCode']==(2 if scenario=='package-failed' else 0))
        if scenario in legacy_failures:
            stop,status=legacy_failures[scenario]
            check('legacy-'+scenario+'-precise-failure',summary['pcm']==status and summary['status']=='diagnostics-only')
            check('legacy-'+scenario+'-dependent-stages-not-run',names==legacy_prefix+legacy_stages[:legacy_stages.index(stop)+1]+['input-recheck','package'])
        if scenario in ('runtime-missing','runtime-failed','no-host','integrity-failed','deadline-exhausted'):
            check('legacy-'+scenario+'-no-optional-capture',not any(s in names for s in legacy_stages))
        if scenario in ('success','both-runtimes','package-failed'):
            check('legacy-'+scenario+'-capture-complete',summary['pcm']=='completed' and summary['status']=='pcm-captured' and summary['captureContract']=='vixen3.12.4-net8-naudio2.2.1')
        if scenario=='input-failed':
            check('legacy-failed-input-qualified',summary['pcm']=='completed' and summary['status']=='pcm-captured-input-unverified')
        if scenario=='strict-preferred':
            check('strict-net10-preferred-contract',summary['captureContract']=='strict-net10-naudio2.3' and summary['status']=='pcm-captured' and not any(n.startswith('legacy-') for n in names))
        if scenario=='both-runtimes':
            check('net10-gdi-preserved-with-legacy-capture','net10-gdi' in names and summary['gdiNet10']=='completed')
        for requested in workflow['requested']:
            if requested['Name']=='legacy-compile':
                check('legacy-'+scenario+'-bounded-compiler',requested['Seconds']==30 and requested['Executable'].endswith('csc.exe') and '/platform:x64' in requested['Arguments'] and any(a.endswith('LegacyCapture.cs.in') for a in requested['Arguments']))
            if requested['Name'] in ('legacy-contract','legacy-preflight') or (requested['Name']=='pcm' and scenario!='strict-preferred'):
                arguments=requested['Arguments']
                check('legacy-'+scenario+'-'+requested['Name']+'-private-dependencies',arguments[arguments.index('--vixen-bin')+1]==str(work/('legacy-'+scenario)/'reference-assemblies') and arguments[0].endswith('/legacy-host/LegacyCapture.exe'))
    check('legacy-no-net8-runtime-reported',data['legacy-runtime-missing']['summary']['pcm']=='skipped-no-verified-net8-or-net10')
    check('legacy-failed-runtime-not-absence',data['legacy-runtime-failed']['summary']['pcm']=='skipped-runtime-probe-unavailable')
    check('legacy-pack-integrity-required',data['legacy-integrity-failed']['summary']['pcm']=='skipped-pack-integrity')
    check('legacy-missing-compiler-diagnosed',any(s['name']=='legacy-compile' and 'FileNotFoundException' in s['error'] for s in data['legacy-compiler-missing']['summary']['stages']))
    deadline=data['legacy-deadline-exhausted']['summary']['stages']
    check('overall-reserves-packaging-after-inventory-timeout',deadline[0]['timedOut'] and deadline[-2]['status']=='skipped-overall-deadline' and deadline[-1]['status']=='completed')
    for i,case in enumerate(data['runtimeSkipCases']):
        expected=('skipped-inventory-unavailable' if not case['inventoryReady'] else
                  'skipped-pack-integrity' if not case['packTrusted'] else
                  'skipped-no-runtime-host' if not case['hostCandidates'] else
                  'skipped-runtime-probe-unavailable' if not case['successfulRuntimeListings'] else
                  'skipped-no-verified-net10')
        check('runtime-skip-cause-'+str(i),case['reason']==expected)
    check('malformed-inventory-recorded',any('unavailable/malformed' in x for x in data['workflow-inventory-malformed']['summary']['notes']))
    check('paused-console-does-not-block',data['paused-console']['entered'] and data['paused-console']['elapsed']<2 and data['paused-console']['stage']['status']=='completed')
    check('malformed-report-rejected',data['malformedRejected'])
    check('missing-executable-recorded',data['missing']['status']=='failed' and data['missing']['pid'] is None and 'FileNotFoundException' in data['missing']['error'])
    check('overall-clips-stage',data['overall']['status']=='timed-out' and data['overall']['overallExpired'] and data['overall']['elapsedSeconds']<3)
    check('overall-skips-later-stage',data['after-overall']['status']=='skipped-overall-deadline' and data['after-overall']['pid'] is None)
    check('later-stage-recovers',data['recovery']['status']=='completed')
    check('utf8-bom-read',data['bomChoice']['selectedBin']=='C:\\A é\\Vixen' and data['bomChoice']['reason']=='ok')
    check('oversized-output-rejected',data['oversizedRejected'])
    expected_quotes=['""','"plain"','"a b"','"C:\\folder\\\\"','"a\\\"b"','"a\\\\\\\"b"','"é path"']
    check('windows-argument-escaping',[q['quoted'] for q in data['quotes'][:-1]]==expected_quotes)
    check('quote-newline-refused',data['quotes'][-1].get('rejected') is True)
    for name in ('success','nonzero','hang','large','flood','malformed','descendant','missing','overall','recovery'):
        saved=json.loads((work/name/'diagnostics.json').read_text())
        check(name+'-checkpoint',bool(saved['stages']) and saved['runnerVersion']==3)
    # No child/descendant may be left alive. POSIX zombies are already terminal.
    pids=[v['pid'] for v in data.values() if isinstance(v,dict) and isinstance(v.get('pid'),int)]
    pids.append(int((work/'descendant/worker/descendant.pid').read_text()))
    pids.append(data['paused-console']['stage']['pid'])
    for key,value in data.items():
        if isinstance(value,dict) and 'summary' in value:
            pids.extend(s['pid'] for s in value['summary']['stages'] if isinstance(s.get('pid'),int))
    process_list=subprocess.run(['ps','-axo','pid=,stat='],capture_output=True,text=True,check=True)
    states={int(line.split()[0]):line.split()[1] for line in process_list.stdout.splitlines() if line.split()}
    for pid in sorted(set(pids)):
        state=states.get(pid,'')
        check('owned-process-exited-'+str(pid),not state or state.startswith('Z'))
    check('descendant-does-not-block-output',data['descendant']['status']=='completed' and data['descendant']['elapsedSeconds']<4)
    check('source-unchanged',all(sha(ROOT/p)==value for p,value in hashes.items()))
    report={'passed':True,'checks':checks,'sourceHashes':hashes,'report':str(work/'fault-results.json'),
            'windowsNativeAdapterExecuted':False,'powershellExecuted':False,'scope':'Production scheduler and checkpoint/parser/quoting code with actual local fault subprocesses; worker runtime-config checks are source invariants only. Native Windows launcher, Framework compiler and PowerShell workers require Windows execution.'}
    (out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'passed':True,'checks':len(checks),'report':str(out/'result.json')}))


if __name__=='__main__':main()
