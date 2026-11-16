#!/usr/bin/env python3
"""Accept retained editor v4 evidence with an exact pre-save backup proof.

No input/application launch; preserve the original false runner result.
"""
import argparse,copy,importlib.util,json,re,sys
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[1]

def accept_backup(actual,prepared):
    extras=set(actual)-set(prepared)
    backups=extras-{'.lock','logs/Application.Info.log'}
    assert len(backups)==1,'Expected one exact pre-save ModuleStore backup'
    backup=next(iter(backups))
    assert re.fullmatch(r'SystemData/auto_backup/ModuleStore_[0-9]{8}_[0-9]{6}\.xml',backup),'Unexpected backup path'
    assert actual[backup]==prepared['SystemData/ModuleStore.xml'],'Backup is not exact original ModuleStore'
    return backup

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--capture',type=Path,required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args()
    out=a.capture.resolve();dest=a.out.resolve()
    assert out.is_relative_to(ROOT/'target') and dest.is_relative_to(ROOT/'target') and not dest.exists()
    retained=out/'used-sources/test_editor_library_foreground.py'
    spec=importlib.util.spec_from_file_location('retained_editor_runner',retained);r=importlib.util.module_from_spec(spec);spec.loader.exec_module(r)
    raw_path=out/'result.json';before=r.sha(raw_path);raw=json.loads(raw_path.read_text());m=json.loads((out/'prepared.json').read_text())
    assert raw['passed'] is False and raw['error']=="AssertionError('Unexpected private profile output')"
    assert raw['exitCode']==raw['driverExitCode']==0
    for k in ['processExited','driverExited','cleanupCompleted','noInputHeld','restoredPrevious','ownerPreferencesUnchanged','inputsUnchanged','withinDeadline']:assert raw[k] is True,k
    pins={}
    for path,digest in m['frozenHashes'].items():
        saved=out/'used-sources'/Path(path).name if Path(path).parent==ROOT/'scripts' and (out/'used-sources'/Path(path).name).exists() else Path(path)
        assert r.sha(saved)==digest,'Retained input drift: '+path
        pins[path]=dict(sha256=digest,retainedAt=str(saved))
    assert r.tree(Path(m['source']))==m['sourceHashes']
    actual=r.tree(out/'profile');backup=accept_backup(actual,m['preparedProfileHashes'])
    checks=['all retained capture pins','raw failure exactly unexpected profile output','normal app/helper exit0','source fixture unchanged','single backup exact initial ModuleStore']
    for mode in ['wrong hash','wrong path','extra backup','missing backup']:
        bad=copy.deepcopy(actual)
        if mode=='wrong hash':bad[backup]='0'*64
        elif mode=='wrong path':bad['SystemData/auto_backup/unrelated.xml']=bad.pop(backup)
        elif mode=='extra backup':bad['SystemData/auto_backup/ModuleStore_00000000_000000.xml']=bad[backup]
        else:bad.pop(backup)
        try:accept_backup(bad,m['preparedProfileHashes'])
        except AssertionError:checks.append(mode+' refused')
        else:raise AssertionError('Invalid backup accepted: '+mode)
    # Execute every original assertion, removing only the separately proven
    # single backup from the strict extra-file inventory presented to it.
    original_tree=r.tree
    def verified_tree(path):
        files=original_tree(path)
        if Path(path)==out/'profile':
            assert files==actual
            assert accept_backup(files,m['preparedProfileHashes'])==backup
            files={name:digest for name,digest in files.items() if name!=backup}
        return files
    accepted=copy.deepcopy(raw)
    with patch.object(r,'tree',verified_tree):
        r.verify(out,m,accepted,SimpleNamespace(returncode=0),SimpleNamespace(returncode=0))
    checks.append('all original frozen verifier assertions including final Rust inspector')
    for name in ['ready','copied']:
        disk=json.loads((out/f'checkpoint-{name}.json').read_text());assert disk==accepted['checkpoints'][name]
        r.accept_checkpoint(name,disk,m,accepted['checkpoints'])
    checks.append('both live saved checkpoints rechecked against disk')
    fg=accepted['foreground'];e=fg['rasterEvidence'][0]
    assert e['toolbarSamples']==9480 and e['maximumRGBDifference']==0 and e['sourceImageSha256']==r.sha(out/'ui/ready.png')
    assert fg['lastActualHit']['status']==0 and fg['lastActualHit']['ownerPID']==fg['targetPID'] and fg['lastActualHit']['reachedExactRoot'] is True
    owned=json.loads((out/'owned-processes.json').read_text());assert owned['appPID']==fg['targetPID'] and owned['appExecutable']==m['app']
    checks+=['exact9480observed raster samples','actual AX hit owner and root','owned process identity matches capture']
    assert r.sha(raw_path)==before
    report=dict(passed=True,acceptance=True,scope=m['scope'],capture=str(out),rawResultRemainsPassed=False,rawResultSha256=before,rawFailure=raw['error'],reconciliation='Only one exact initial ModuleStore timestamped pre-save backup is excluded from the original extra-file inventory; every original verifier assertion runs unchanged.',backup=dict(path=backup,sha256=actual[backup],initialModuleStoreSha256=m['preparedProfileHashes']['SystemData/ModuleStore.xml']),backupProductionSource=dict(path=str(ROOT/'crates/vixen-io/src/file.rs'),sha256=r.sha(ROOT/'crates/vixen-io/src/file.rs'),contract='save moves previous system file to auto_backup using its last-write timestamp'),checks=checks,checkCount=len(checks),steps=accepted['steps'],actualDragSessions=1,mousePairs=1,keyPairs=3,nameOKPressStatus=fg['nameOKPressStatus'],inspection=accepted['inspection'],normalProcessExitCodes=[0,0],noInputHeld=True,restoredPointer=True,restoredPrevious=True,ownerPreferencesUnchanged=True,captureInputs=pins,consumerSha256=r.sha(Path(__file__)),elapsedSeconds=raw['elapsedSeconds'],newInputPosted=False,newApplicationLaunched=False)
    dest.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(passed=True,checks=len(checks),report=str(dest),newInputPosted=False)))

if __name__=='__main__':main()
